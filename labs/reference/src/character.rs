//! Static character comparison using the actual product material integration.
use bevy::{
    asset::{AssetMetaCheck, LoadState},
    camera::{Exposure, ScalingMode},
    core_pipeline::tonemapping::Tonemapping,
    gltf::GltfAssetLabel,
    prelude::*,
    render::{RenderApp, RenderPlugin, render_resource::PipelineCache, settings::RenderCreation},
    window::ExitCondition,
    world_serialization::WorldAssetRoot,
};
use bevy_stylized::{
    StylizedAssetIntegrationPlugin, StylizedMaterialAdoption, StylizedMaterialSet, prelude::*,
};
use golden_harness::{CapturedImage, Tier2Thresholds, capture, evaluate_tier2};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Deserialize)]
struct CameraInput {
    position: [f32; 3],
    target: [f32; 3],
    up: [f32; 3],
    projection: String,
    scale: f32,
    near: f32,
    far: f32,
}
#[derive(Deserialize)]
struct Case {
    id: String,
    camera: CameraInput,
    direction_to_light: [f32; 3],
    reference: Vec<String>,
    actual: String,
    silhouette: String,
    #[serde(default)]
    factor_reference: Option<String>,
    #[serde(default)]
    center_reference: Option<String>,
}
#[derive(Deserialize)]
struct Package {
    schema: String,
    coordinates: String,
    resolution: [u32; 2],
    background_linear: [f32; 4],
    required_cases: Vec<String>,
    cases: Vec<Case>,
    files: BTreeMap<String, String>,
    #[serde(default)]
    diagnostics: Value,
    #[serde(default)]
    supersample: Option<u32>,
    #[serde(default)]
    sampling: Value,
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: impl AsRef<Path>) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| e.to_string())
}
fn write(path: impl AsRef<Path>, data: &Value) -> Result<(), String> {
    std::fs::write(
        path,
        serde_json::to_vec_pretty(data).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
fn vec(v: [f32; 3]) -> Vec3 {
    Vec3::from_array(v)
}
fn package(path: &Path) -> Result<Package, String> {
    let p: Package = serde_json::from_slice(&read(path)?).map_err(|e| e.to_string())?;
    if !matches!(
        p.schema.as_str(),
        "bevy_stylized.character-capture/v1"
            | "bevy_stylized.character-capture/v2"
            | "bevy_stylized.character-capture/v3"
    ) || p.coordinates != "gltf-y-up-metres"
        || p.resolution != [512, 512]
    {
        return Err("unsupported capture domain".into());
    }
    if p.schema.ends_with("/v3") && !p.supersample.is_some_and(|v| [2, 4, 8].contains(&v)) {
        return Err("matched case requires a supported sampling grid".into());
    }
    if p.schema.ends_with("/v3")
        && (p.diagnostics["policy"] != "lighting-factors/v1"
            || p.sampling["matched"]["policy"] != "matched-box/v1"
            || p.sampling["matched"]["grid"].as_u64() != p.supersample.map(u64::from))
    {
        return Err(
            "matched qualification requires lighting factors and matching sampling policy".into(),
        );
    }
    let ids: Vec<_> = ["front", "left", "right"]
        .into_iter()
        .flat_map(|l| [format!("full_body-{l}"), format!("face_closeup-{l}")])
        .collect();
    if p.required_cases != ids || p.cases.iter().map(|c| c.id.clone()).collect::<Vec<_>>() != ids {
        return Err("required cases missing/duplicated".into());
    }
    let root = path
        .parent()
        .ok_or("case parent")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    for (name, digest) in &p.files {
        if Path::new(name).is_absolute() {
            return Err("absolute input".into());
        }
        let file = root.join(name).canonicalize().map_err(|e| e.to_string())?;
        if !file.starts_with(&root) || sha(&read(file)?) != *digest {
            return Err(format!("input identity mismatch: {name}"));
        }
    }
    for name in [
        "runtime/character.glb",
        "runtime/character.npr.json",
        "runtime/character.view.json",
        "authoring.json",
        "audit.json",
        "calibration.json",
        "calibration-blender.png",
        "runtime/calibration.glb",
    ] {
        if !p.files.contains_key(name) {
            return Err(format!("input not inventoried: {name}"));
        }
    }
    for c in &p.cases {
        if (p.schema.ends_with("/v2") || p.schema.ends_with("/v3"))
            && (c.factor_reference.is_none() || c.center_reference.is_none())
        {
            return Err("missing diagnostic reference".into());
        }
        let a = &c.camera;
        if a.projection != "orthographic"
            || !a.scale.is_finite()
            || a.scale <= 0.
            || !(0. < a.near && a.near < a.far)
            || !a.far.is_finite()
            || !vec(a.position).is_finite()
            || !vec(a.target).is_finite()
            || !vec(a.up).is_finite()
            || (vec(a.target) - vec(a.position))
                .cross(vec(a.up))
                .length_squared()
                < 1e-12
            || !vec(c.direction_to_light).is_finite()
            || (vec(c.direction_to_light).length_squared() - 1.).abs() > 1e-5
        {
            return Err("invalid camera/light".into());
        }
        if c.reference.len() != 2
            || c.reference
                .iter()
                .chain([&c.actual, &c.silhouette])
                .chain(c.factor_reference.iter())
                .chain(c.center_reference.iter())
                .any(|f| !p.files.contains_key(f))
        {
            return Err("reference not inventoried".into());
        }
    }
    for c in &p.cases {
        let actual: Value =
            serde_json::from_slice(&read(root.join(&c.actual))?).map_err(|e| e.to_string())?;
        let matrix: [[f32; 4]; 4] =
            serde_json::from_value(actual["camera"]["matrix_world"].clone())
                .map_err(|e| e.to_string())?;
        let position = Vec3::new(matrix[0][3], matrix[2][3], -matrix[1][3]);
        let forward = Vec3::new(-matrix[0][2], -matrix[2][2], matrix[1][2]);
        let body: [f32; 3] = serde_json::from_value(actual["body_direction_blender"].clone())
            .map_err(|e| e.to_string())?;
        let light = Vec3::new(body[0], body[2], -body[1]).normalize();
        if position.distance(vec(c.camera.position)) > 1e-5
            || forward.distance((vec(c.camera.target) - position).normalize()) > 1e-5
            || light.distance(vec(c.direction_to_light)) > 1e-5
            || actual["exposure_ev"] != 0.
            || actual["gamma"] != 1.
            || actual["view_transform"] != "Standard"
            || (actual["camera"]["ortho_scale"]
                .as_f64()
                .ok_or("actual scale")? as f32
                - c.camera.scale)
                .abs()
                > 1e-5
        {
            return Err("upstream actual settings mismatch".into());
        }
    }
    Ok(p)
}
fn pipelines_ready(app: &App) -> Result<bool, String> {
    golden_harness::readiness::pipeline_status(
        app.sub_app(RenderApp).world().resource::<PipelineCache>(),
    )
}
fn frame(app: &mut App, target: &capture::CaptureTarget) -> Result<CapturedImage, String> {
    let req = capture::request_capture(app.world_mut(), target.clone());
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(30) {
        app.update();
        pipelines_ready(app)?;
        if let Some(image) = capture::try_take(app.world_mut(), req.clone()) {
            return Ok(image);
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Err("capture timeout".into())
}
fn repeat(a: &CapturedImage, b: &CapturedImage) -> Result<Value, String> {
    if a.width != b.width || a.height != b.height {
        return Err("capture dimensions mismatch".into());
    }
    let d = golden_harness::diff_rgba8(&a.data, &b.data).map_err(|e| e.to_string())?;
    Ok(
        json!({"passed":d.max_channel_delta<=2&&d.mean_absolute_delta<=0.5&&d.psnr_db>=48.,"stats":d}),
    )
}
/// Run a declared six-case capture package, retaining truthful failed parity.
pub fn run(path: PathBuf, out: PathBuf) -> Result<Value, String> {
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    let p = package(&path)?;
    let root = path.parent().ok_or("case parent")?;
    if out.exists() {
        return Err("candidate output already exists".into());
    }
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let sidecar =
        bevy_stylized::stylized_gltf::load_sidecar(root.join("runtime/character.npr.json"))
            .map_err(|e| e.to_string())?;
    let view: StylizedViewDescriptor =
        serde_json::from_slice(&read(root.join("runtime/character.view.json"))?)
            .map_err(|e| e.to_string())?;
    let resources = std::panic::catch_unwind(capture::probe_render_resources)
        .map_err(|_| "required GPU unavailable")?
        .ok_or("required GPU unavailable")?;
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .build()
            .set(bevy_stylized::stylized_gltf_plugin())
            .set(AssetPlugin {
                file_path: root.join("runtime").to_string_lossy().into_owned(),
                meta_check: AssetMetaCheck::Never,
                ..default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Manual(resources),
                synchronous_pipeline_compilation: true,
                ..default()
            })
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            }),
    );
    app.add_plugins((StylizedPlugin::default(), StylizedAssetIntegrationPlugin));
    capture::install(&mut app);
    app.insert_resource(ClearColor(Color::linear_rgba(
        p.background_linear[0],
        p.background_linear[1],
        p.background_linear[2],
        p.background_linear[3],
    )));
    let target = capture::create_capture_target(
        &mut app.world_mut().resource_mut::<Assets<Image>>(),
        512,
        512,
    );
    let mut camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            capture::capture_target_bundle(&target),
            Tonemapping::None,
            Msaa::Sample4,
            Transform::default(),
        ))
        .id();
    let diagnostic_mode = if p.diagnostics["policy"] == "beauty-center/v1" {
        StylizedHeroDebugView::Beauty
    } else if p.diagnostics["policy"] == "ndoth/v1" {
        StylizedHeroDebugView::NormalHalfDot
    } else if p.diagnostics["policy"] == "pre-palette/v1" {
        StylizedHeroDebugView::PrePalette
    } else if p.diagnostics["policy"] == "base-color/v1" {
        StylizedHeroDebugView::BaseColor
    } else {
        StylizedHeroDebugView::LightingFactors
    };
    app.insert_resource(DiagnosticMode(diagnostic_mode));
    app.finish();
    app.cleanup();
    if p.supersample.is_some_and(|s| ![2, 4, 8].contains(&s)) {
        return Err("unsupported matched sampling grid".into());
    }
    let linear_target = p.supersample.map(|scale| {
        capture::create_linear_capture_target(
            &mut app.world_mut().resource_mut::<Assets<Image>>(),
            512 * scale,
            512 * scale,
        )
    });
    let calibration = calibrate(&mut app, root, &out, &target, camera)?;
    let scene: Handle<bevy::world_serialization::WorldAsset> = app
        .world()
        .resource::<AssetServer>()
        .load(GltfAssetLabel::Scene(0).from_asset("character.glb"));
    app.world_mut().spawn((
        WorldAssetRoot(scene.clone()),
        StylizedMaterialSet {
            materials: sidecar.materials.clone(),
            texture_prefix: String::new(),
        },
        Transform::default(),
    ));
    let glb = read(root.join("runtime/character.glb"))?;
    let len = u32::from_le_bytes(glb[12..16].try_into().map_err(|_| "GLB header")?) as usize;
    let metadata: Value = serde_json::from_slice(glb.get(20..20 + len).ok_or("GLB bounds")?)
        .map_err(|e| e.to_string())?;
    let expected = metadata["meshes"]
        .as_array()
        .ok_or("GLB meshes")?
        .iter()
        .map(|m| m["primitives"].as_array().map_or(0, Vec::len))
        .sum::<usize>();
    if expected == 0 {
        return Err("empty character".into());
    }
    let textures: Vec<_> = sidecar
        .materials
        .values()
        .flat_map(|m| m.textures.iter().map(|t| t.uri.clone()))
        .collect();
    let mut reports = Vec::new();
    for case in p.cases {
        let c = &case.camera;
        if let Some(linear) = &linear_target {
            app.world_mut().despawn(camera);
            camera = app
                .world_mut()
                .spawn((
                    Camera3d::default(),
                    Camera::default(),
                    bevy::camera::Hdr,
                    bevy::camera::RenderTarget::Image(linear.image.clone().into()),
                    Msaa::Off,
                    Tonemapping::None,
                    Transform::default(),
                ))
                .id();
        }

        app.world_mut().entity_mut(camera).insert((
            Transform::from_translation(vec(c.position)).looking_at(vec(c.target), vec(c.up)),
            Projection::Orthographic(OrthographicProjection {
                near: c.near,
                far: c.far,
                scaling_mode: ScalingMode::Fixed {
                    width: c.scale,
                    height: c.scale,
                },
                ..OrthographicProjection::default_3d()
            }),
        ));
        let mut settings = view.clone();
        settings.key_light_direction = vec(case.direction_to_light);
        settings.exposure = 1.;
        *app.world_mut().resource_mut::<StylizedViewSettings>() = StylizedViewSettings(settings);
        let start = Instant::now();
        let mut stable = 0;
        loop {
            app.update();
            let mut q = app.world_mut().query::<&StylizedMaterialAdoption>();
            let adopted: Vec<_> = q.iter(app.world()).collect();
            if let Some(e) = adopted.iter().find_map(|a| a.error.as_ref()) {
                return Err(e.clone());
            }
            let server = app.world().resource::<AssetServer>();
            if let Some(LoadState::Failed(e)) = server.get_load_state(scene.id()) {
                return Err(format!("scene load failed: {e}"));
            }
            let mut loaded = server.is_loaded_with_dependencies(scene.id());
            for uri in &textures {
                if let Some(handle) = server.get_handle::<Image>(uri) {
                    if let Some(LoadState::Failed(e)) = server.get_load_state(handle.id()) {
                        return Err(format!("texture load failed: {e}"));
                    }
                    loaded &= server.is_loaded_with_dependencies(handle.id());
                } else {
                    loaded = false;
                }
            }
            if loaded && adopted.len() == expected && pipelines_ready(&app)? {
                stable += 1
            } else {
                stable = 0
            }
            if stable >= 24 {
                break;
            }
            if start.elapsed() > readiness_timeout() {
                return Err(format!("asset/pipeline readiness timeout: {}", case.id));
            }
            std::thread::sleep(Duration::from_millis(3));
        }
        let a = if let (Some(linear), Some(scale)) = (&linear_target, p.supersample) {
            frame_linear(&mut app, linear, scale)?
        } else {
            frame(&mut app, &target)?
        };
        let b = if let (Some(linear), Some(scale)) = (&linear_target, p.supersample) {
            frame_linear(&mut app, linear, scale)?
        } else {
            frame(&mut app, &target)?
        };
        if linear_target.is_some() {
            let transform = *app
                .world()
                .get::<Transform>(camera)
                .ok_or("camera transform")?;
            let projection = app
                .world()
                .get::<Projection>(camera)
                .ok_or("camera projection")?
                .clone();
            app.world_mut().despawn(camera);
            camera = app
                .world_mut()
                .spawn((
                    Camera3d::default(),
                    capture::capture_target_bundle(&target),
                    Tonemapping::None,
                    Msaa::Sample4,
                    transform,
                    projection,
                ))
                .id();
            for _ in 0..24 {
                app.update();
            }
        }
        let ra = golden_harness::decode_png(&read(root.join(&case.reference[0]))?)
            .map_err(|e| e.to_string())?;
        let rb = golden_harness::decode_png(&read(root.join(&case.reference[1]))?)
            .map_err(|e| e.to_string())?;
        let repeat_up = repeat(&ra, &rb)?;
        let repeat_bevy = repeat(&a, &b)?;
        if a.width != ra.width || a.height != ra.height {
            return Err("cross-renderer dimensions mismatch".into());
        }
        let color = Color::linear_rgb(
            p.background_linear[0],
            p.background_linear[1],
            p.background_linear[2],
        )
        .to_srgba()
        .to_u8_array();
        let tier = evaluate_tier2(
            &case.id,
            &ra,
            &a,
            [color[0], color[1], color[2]],
            Tier2Thresholds::default(),
        )
        .map_err(|e| e.to_string())?;
        let mut difference = a.clone();
        for ((pixel, left), right) in difference
            .data
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(ra.data.as_chunks::<4>().0.iter())
            .zip(a.data.as_chunks::<4>().0.iter())
        {
            for c in 0..3 {
                pixel[c] = left[c].abs_diff(right[c]).saturating_mul(4);
            }
            pixel[3] = 255;
        }
        std::fs::write(
            out.join(format!("{}-diff.png", case.id)),
            golden_harness::encode_png(&difference).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::write(
            out.join(format!("{}-bevy-repeat.png", case.id)),
            golden_harness::encode_png(&b).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let name = format!("{}-bevy.png", case.id);
        let png = golden_harness::encode_png(&a).map_err(|e| e.to_string())?;
        std::fs::write(out.join(&name), &png).map_err(|e| e.to_string())?;
        std::fs::copy(
            root.join(&case.reference[0]),
            out.join(format!("{}-blender.png", case.id)),
        )
        .map_err(|e| e.to_string())?;
        let mask = coverage(&mut app, &target)?;
        let reference_mask = golden_harness::decode_png(&read(root.join(&case.silhouette))?)
            .map_err(|e| e.to_string())?;
        let silhouette = coverage_iou(&reference_mask, &mask)?;
        std::fs::write(
            out.join(format!("{}-coverage-bevy.png", case.id)),
            golden_harness::encode_png(&mask).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::copy(
            root.join(&case.silhouette),
            out.join(format!("{}-coverage-blender.png", case.id)),
        )
        .map_err(|e| e.to_string())?;
        let mut light_query = app
            .world_mut()
            .query_filtered::<(&Transform, &DirectionalLight), With<StylizedKeyLight>>();
        let (light_transform, light_component) =
            light_query.single(app.world()).map_err(|e| e.to_string())?;
        let actual_light = -light_transform.forward().as_vec3();
        if actual_light.distance(vec(case.direction_to_light)) > 1e-5 {
            return Err("actual key light direction mismatch".into());
        }
        let actual_illuminance = light_component.illuminance;
        let entity = app.world().entity(camera);
        let transform = entity.get::<Transform>().ok_or("camera transform")?;
        let projection = entity.get::<Projection>().ok_or("camera projection")?;
        let exposure = entity
            .get::<Exposure>()
            .ok_or("camera exposure")?
            .exposure();
        let forward = transform.forward().as_vec3();
        let expected_forward = (vec(c.target) - vec(c.position)).normalize();
        if transform.translation.distance(vec(c.position)) > 1e-5
            || forward.distance(expected_forward) > 1e-5
            || (exposure - 1.).abs() > 1e-5
        {
            return Err("runtime camera/exposure mismatch".into());
        }
        let Projection::Orthographic(ortho) = projection else {
            return Err("runtime projection mismatch".into());
        };
        if (ortho.area.width() - c.scale).abs() > 1e-5
            || (ortho.area.height() - c.scale).abs() > 1e-5
        {
            return Err("runtime orthographic dimensions mismatch".into());
        }
        let actual = json!({"key_light_direction":actual_light.to_array(),"key_light_illuminance":actual_illuminance,"position":transform.translation.to_array(),"rotation":transform.rotation.to_array(),"projection":format!("{projection:?}"),"orthographic_width":ortho.area.width(),"orthographic_height":ortho.area.height(),"exposure_multiplier":exposure,
            "view":app.world().resource::<StylizedViewSettings>().0,"background":format!("{:?}",app.world().resource::<ClearColor>().0),"adopted_primitives":expected});
        write(out.join(format!("{}-bevy-actual.json", case.id)), &actual)?;
        let semantic_shadow = if let (Some(factor), Some(center)) =
            (&case.factor_reference, &case.center_reference)
        {
            lighting_diagnostics(
                &mut app, &target, camera, root, &out, &case.id, factor, center,
            )?
        } else {
            json!({"status":"UNQUALIFIED","reason":"No lighting-factor references in this v1 case"})
        };
        let color_passed = tier.max_abs_delta_nonedge <= 16.0 / 255.0
            && tier.mean_abs_delta <= 4.0 / 255.0
            && tier.psnr_db >= 30.0;
        let passed = if p.schema.ends_with("/v3") {
            color_passed
                && repeat_up["passed"] == true
                && repeat_bevy["passed"] == true
                && semantic_shadow["status"] == "PASS"
        } else {
            tier.passed
                && repeat_up["passed"] == true
                && repeat_bevy["passed"] == true
                && silhouette["passed"] == true
                && semantic_shadow["status"] == "PASS"
        };
        reports.push(json!({"case":case.id,"image_sha256":sha(&png),"color_criteria_passed":color_passed,"sampling_grid":p.supersample,"silhouette":silhouette,"repeat_blender":repeat_up,"repeat_bevy":repeat_bevy,"legacy_tier2":tier,
            "input_alignment":{"status":"PASS","scope":"calibrated camera/color and declared NPR direction; not shader formula equivalence"},"semantic_shadow":semantic_shadow,"parity_status":if passed {"PASS"}else{"FAIL"}}));
    }
    let gpu = app
        .world()
        .resource::<bevy::render::renderer::RenderAdapterInfo>();
    let device = json!({"name":gpu.name,"backend":format!("{:?}",gpu.backend),"driver":gpu.driver,"driver_info":gpu.driver_info});
    let capabilities = format!(
        "{:?}",
        app.world().get_resource::<StylizedRenderCapabilities>()
    );
    let report = json!({"schema":"bevy_stylized.character-comparison/v1","input_sha256":sha(&read(&path)?),"device":device,"capabilities":capabilities,"material_classes":sidecar.materials.values().map(|m|format!("{:?}",m.surface_class)).collect::<Vec<_>>(),"capture_status":"PASS","parity_status":if reports.iter().all(|r|r["parity_status"]=="PASS") {"PASS"}else{"FAIL"},"threshold_policy":"RGB thresholds from docs/reference/08 unchanged; matched v3 uses center coverage IoU>=0.99 and sharp lighting-factor IoU>=0.95; legacy AA and luminance proxies remain diagnostic","calibration":calibration,"reports":reports});
    write(out.join("report.json"), &report)?;
    Ok(report)
}

fn calibrate(
    app: &mut App,
    root: &Path,
    out: &Path,
    target: &capture::CaptureTarget,
    camera: Entity,
) -> Result<Value, String> {
    let record: Value =
        serde_json::from_slice(&read(root.join("calibration.json"))?).map_err(|e| e.to_string())?;
    let input: CameraInput =
        serde_json::from_value(record["camera"].clone()).map_err(|e| e.to_string())?;
    app.world_mut().entity_mut(camera).insert((
        Transform::from_translation(vec(input.position))
            .looking_at(vec(input.target), vec(input.up)),
        Projection::Orthographic(OrthographicProjection {
            near: input.near,
            far: input.far,
            scaling_mode: ScalingMode::Fixed {
                width: input.scale,
                height: input.scale,
            },
            ..OrthographicProjection::default_3d()
        }),
    ));
    let scene: Handle<bevy::world_serialization::WorldAsset> = app
        .world()
        .resource::<AssetServer>()
        .load(GltfAssetLabel::Scene(0).from_asset("calibration.glb"));
    let entity = app
        .world_mut()
        .spawn((WorldAssetRoot(scene.clone()), Transform::default()))
        .id();
    let start = Instant::now();
    let mut stable = 0;
    loop {
        app.update();
        if app
            .world()
            .resource::<AssetServer>()
            .is_loaded_with_dependencies(scene.id())
            && pipelines_ready(app)?
        {
            stable += 1
        } else {
            stable = 0
        }
        if stable >= 24 {
            break;
        }
        if start.elapsed() > readiness_timeout() {
            return Err("calibration readiness timeout".into());
        }
        std::thread::sleep(Duration::from_millis(3));
    }
    let image = frame(app, target)?;
    let reference = golden_harness::decode_png(&read(root.join("calibration-blender.png"))?)
        .map_err(|e| e.to_string())?;
    let mut checks = Vec::new();
    for marker in record["markers"].as_array().ok_or("calibration markers")? {
        let x = marker["pixel"][0].as_u64().ok_or("pixel x")? as usize;
        let y = marker["pixel"][1].as_u64().ok_or("pixel y")? as usize;
        if x >= 512 || y >= 512 {
            return Err("calibration pixel out of bounds".into());
        }
        let color: [f32; 3] =
            serde_json::from_value(marker["linear_color"].clone()).map_err(|e| e.to_string())?;
        let expected = Color::linear_rgb(color[0], color[1], color[2])
            .to_srgba()
            .to_u8_array();
        let offset = (y * 512 + x) * 4;
        let delta = (0..3)
            .map(|c| {
                image.data[offset + c]
                    .abs_diff(expected[c])
                    .max(reference.data[offset + c].abs_diff(expected[c]))
            })
            .max()
            .unwrap_or(255);
        let centroid = |im: &CapturedImage| -> Result<[f64; 2], String> {
            let mut n = 0.;
            let mut sum = [0., 0.];
            for (i, p) in im.data.as_chunks::<4>().0.iter().enumerate() {
                if (0..3).all(|c| p[c].abs_diff(expected[c]) <= 2) {
                    n += 1.;
                    sum[0] += (i % 512) as f64 + 0.5;
                    sum[1] += (i / 512) as f64 + 0.5;
                }
            }
            if n < 100. {
                return Err("calibration marker missing".into());
            }
            Ok([sum[0] / n, sum[1] / n])
        };
        let a = centroid(&image)?;
        let b = centroid(&reference)?;
        let position_error = (a[0] - b[0])
            .abs()
            .max((a[1] - b[1]).abs())
            .max((a[0] - x as f64).abs())
            .max((a[1] - y as f64).abs());
        checks.push(json!({"marker":marker["name"],"max_channel_delta":delta,"position_error_pixels":position_error,"passed":delta<=2&&position_error<=1.}));
    }
    let passed = checks.iter().all(|c| c["passed"] == true);
    std::fs::write(
        out.join("calibration-bevy.png"),
        golden_harness::encode_png(&image).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::copy(
        root.join("calibration-blender.png"),
        out.join("calibration-blender.png"),
    )
    .map_err(|e| e.to_string())?;
    app.world_mut().despawn(entity);
    for _ in 0..8 {
        app.update();
    }
    let result = json!({"passed":passed,"checks":checks,"scope":"geometry, projected axes, color and unit exposure; excludes NPR shading"});
    write(out.join("calibration-report.json"), &result)?;
    if !passed {
        return Err("geometry/color calibration failed; inspect calibration-report.json".into());
    }
    Ok(result)
}

fn coverage(app: &mut App, target: &capture::CaptureTarget) -> Result<CapturedImage, String> {
    let mut q = app.world_mut().query::<(
        Entity,
        Option<&MeshMaterial3d<StylizedCelMaterial>>,
        Option<&MeshMaterial3d<StylizedCharacterMaterial>>,
        Option<&MeshMaterial3d<StylizedHeroMaterial>>,
    )>();
    let originals: Vec<_> = q
        .iter(app.world())
        .filter(|(_, a, b, c)| a.is_some() || b.is_some() || c.is_some())
        .map(|(e, a, b, c)| (e, a.cloned(), b.cloned(), c.cloned()))
        .collect();
    // Vertex colors carry NPR controls, not albedo. A white coverage pass
    // must ignore them while preserving positions/indices and all deformation.
    let mut meshes = Vec::new();
    for (e, _, _, _) in &originals {
        let original = app
            .world()
            .get::<Mesh3d>(*e)
            .ok_or("coverage mesh")?
            .0
            .clone();
        let mut copy = app
            .world()
            .resource::<Assets<Mesh>>()
            .get(&original)
            .ok_or("coverage mesh asset")?
            .clone();
        copy.remove_attribute(Mesh::ATTRIBUTE_COLOR);
        let diagnostic = app.world_mut().resource_mut::<Assets<Mesh>>().add(copy);
        app.world_mut().entity_mut(*e).insert(Mesh3d(diagnostic));
        meshes.push((*e, original));
    }
    let white = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            double_sided: true,
            cull_mode: None,
            ..default()
        });
    for (e, _, _, _) in &originals {
        app.world_mut()
            .entity_mut(*e)
            .remove::<(
                MeshMaterial3d<StylizedCelMaterial>,
                MeshMaterial3d<StylizedCharacterMaterial>,
                MeshMaterial3d<StylizedHeroMaterial>,
            )>()
            .insert(MeshMaterial3d(white.clone()));
    }
    let start = Instant::now();
    let mut stable = 0;
    loop {
        app.update();
        if pipelines_ready(app)? {
            stable += 1
        } else {
            stable = 0
        }
        if stable >= 24 {
            break;
        }
        if start.elapsed() > Duration::from_secs(30) {
            return Err("coverage pipeline timeout".into());
        }
    }
    let image = frame(app, target)?;
    for (e, a, b, c) in originals {
        let mut entity = app.world_mut().entity_mut(e);
        entity.remove::<MeshMaterial3d<StandardMaterial>>();
        if let Some(m) = a {
            entity.insert(m);
        }
        if let Some(m) = b {
            entity.insert(m);
        }
        if let Some(m) = c {
            entity.insert(m);
        }
    }
    for (entity, mesh) in meshes {
        app.world_mut().entity_mut(entity).insert(Mesh3d(mesh));
    }
    Ok(image)
}
fn coverage_iou(a: &CapturedImage, b: &CapturedImage) -> Result<Value, String> {
    if a.width != b.width || a.height != b.height {
        return Err("coverage dimensions mismatch".into());
    }
    let mut intersection = 0;
    let mut union = 0;
    let mut count_a = 0;
    let mut count_b = 0;
    // Geometry rendered in white over linear 0.12 background: fixed midpoint
    // threshold in linear coverage (0.56 linear -> about 197 sRGB).
    for (a, b) in a
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .zip(b.data.as_chunks::<4>().0)
    {
        let x = a[0] > 197;
        let y = b[0] > 197;
        count_a += usize::from(x);
        count_b += usize::from(y);
        intersection += usize::from(x && y);
        union += usize::from(x || y);
    }
    if count_a < 100 || count_b < 100 {
        return Err("empty/insufficient geometry mask".into());
    }
    let iou = intersection as f64 / union as f64;
    Ok(
        json!({"algorithm":"white-geometry-coverage/v1","iou":iou,"foreground_blender":count_a,"foreground_bevy":count_b,"threshold_srgb":197,"passed":iou>=0.99,"scope":"opaque static geometry; does not qualify alpha masks or cast shadows"}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_geometry_masks_cannot_pass() {
        let a = CapturedImage::new(16, 16, vec![0; 1024]).unwrap();
        assert!(coverage_iou(&a, &a).is_err());
    }
}

fn readiness_timeout() -> Duration {
    Duration::from_millis(
        std::env::var("STYLIZED_REFERENCE_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60_000),
    )
}

#[allow(clippy::too_many_arguments)]
fn lighting_diagnostics(
    app: &mut App,
    target: &capture::CaptureTarget,
    camera: Entity,
    root: &Path,
    out: &Path,
    id: &str,
    factor: &str,
    center: &str,
) -> Result<Value, String> {
    let background = app.world().resource::<ClearColor>().clone();
    let msaa = *app.world().get::<Msaa>(camera).ok_or("camera MSAA")?;
    app.world_mut().insert_resource(ClearColor(Color::BLACK));
    app.world_mut().entity_mut(camera).insert(Msaa::Off);
    let mask = coverage(app, target)?;
    let reference_mask =
        golden_harness::decode_png(&read(root.join(center))?).map_err(|e| e.to_string())?;
    let center_report = coverage_iou(&reference_mask, &mask)?;
    std::fs::write(
        out.join(format!("{id}-center-coverage-bevy.png")),
        golden_harness::encode_png(&mask).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::copy(
        root.join(center),
        out.join(format!("{id}-center-coverage-blender.png")),
    )
    .map_err(|e| e.to_string())?;
    let debug_mode = app.world().resource::<DiagnosticMode>().0;
    let modes: Vec<_> = app
        .world_mut()
        .resource_mut::<Assets<StylizedHeroMaterial>>()
        .iter_mut()
        .map(|(id, m)| {
            let old = m.debug_view;
            m.debug_view = debug_mode;
            (id, old)
        })
        .collect();
    let start = Instant::now();
    let mut stable = 0;
    loop {
        app.update();
        if pipelines_ready(app)? {
            stable += 1
        } else {
            stable = 0
        }
        if stable >= 24 {
            break;
        }
        if start.elapsed() > Duration::from_secs(30) {
            return Err("lighting diagnostic pipeline timeout".into());
        }
    }
    let rendered = frame(app, target)?;
    let reference =
        golden_harness::decode_png(&read(root.join(factor))?).map_err(|e| e.to_string())?;
    std::fs::write(
        out.join(format!("{id}-factors-bevy.png")),
        golden_harness::encode_png(&rendered).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::copy(
        root.join(factor),
        out.join(format!("{id}-factors-blender.png")),
    )
    .map_err(|e| e.to_string())?;
    for (id, mode) in modes {
        if let Some(mut m) = app
            .world_mut()
            .resource_mut::<Assets<StylizedHeroMaterial>>()
            .get_mut(id)
        {
            m.debug_view = mode
        }
    }
    app.world_mut().entity_mut(camera).insert(msaa);
    app.world_mut().insert_resource(background);
    if rendered.width != reference.width || rendered.height != reference.height {
        return Err("factor dimensions mismatch".into());
    }
    if debug_mode != StylizedHeroDebugView::LightingFactors {
        return Ok(
            json!({"status":"UNQUALIFIED","reason":"base-color diagnostic, not lighting factors","center_coverage":center_report}),
        );
    }
    let mut intersection = 0usize;
    let mut union = 0usize;
    let mut count_a = 0usize;
    let mut count_b = 0usize;
    for (a, b) in reference
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .zip(rendered.data.as_chunks::<4>().0)
    {
        let x = a[2] > 200 && a[1] < 188;
        let y = b[2] > 200 && b[1] < 188;
        count_a += usize::from(x);
        count_b += usize::from(y);
        intersection += usize::from(x && y);
        union += usize::from(x || y);
    }
    if count_a == 0 || count_b == 0 {
        return Ok(
            json!({"status":"UNQUALIFIED","reason":"empty lighting-factor shadow mask","center_coverage":center_report}),
        );
    }
    let iou = intersection as f64 / union as f64;
    Ok(
        json!({"status":if iou>=0.95&&center_report["passed"]==true{"PASS"}else{"FAIL"},"algorithm":"sharp-light-factor/v1","iou":iou,"shadow_pixels_blender":count_a,"shadow_pixels_bevy":count_b,"center_coverage":center_report,"scope":"NPR sharp lighting classification; not physical cast shadows"}),
    )
}

#[derive(Resource)]
struct DiagnosticMode(StylizedHeroDebugView);

fn frame_linear(
    app: &mut App,
    target: &capture::CaptureTarget,
    scale: u32,
) -> Result<CapturedImage, String> {
    let request = capture::request_capture(app.world_mut(), target.clone());
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(60) {
        app.update();
        pipelines_ready(app)?;
        if let Some(image) =
            capture::try_take_linear_resolved(app.world_mut(), request.clone(), scale)?
        {
            return Ok(image);
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Err("linear capture timeout".into())
}
