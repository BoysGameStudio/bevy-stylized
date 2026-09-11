//! Character@Portable 的 Golden 用例（GPU-gated，无 GPU/资产 graceful skip）。
//!
//! - 合成用例：球体 + 程序化 face SDF 纹理（验证 face SDF 方向性阴影、
//!   MeshTag 逐实例阈值、hair 高光、emission）；
//! - Odette 角色用例（TENACITY_ROOT env-gated）：face 用 NPR sidecar 的
//!   face SDF，body/hair 用 ILM AO；对 legacy engine golden 输出 diff
//!   **报告**（非门禁，provisional——shading model 不同）。

use std::path::{Path, PathBuf};

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets, RenderAssetUsages},
    camera::{primitives::Aabb, visibility::Visibility},
    color::{Color, LinearRgba},
    core_pipeline::tonemapping::Tonemapping,
    gltf::{GltfAssetLabel, GltfMaterialName},
    image::{Image, ImageSampler},
    math::{Quat, Vec3},
    pbr::{MeshMaterial3d, StandardMaterial},
    prelude::{
        Camera3d, Entity, GlobalTransform, Handle, Mesh, Mesh3d, Name, Query, Res, ResMut,
        Resource, Transform, With, Without,
    },
    render::{
        RenderPlugin,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        renderer::RenderAdapterInfo,
        settings::{RenderCreation, RenderResources},
    },
    window::{ExitCondition, WindowPlugin},
    world_serialization::WorldAssetRoot,
};
use bevy_stylized::prelude::*;
use bevy_stylized::stylized_render::{StylizedOutlineMaterial, StylizedOutlineParams};
use golden_harness::{
    CapturedImage, GoldenPaths, GoldenProvenance, Tolerance, capture, diff_rgba8,
};

const WIDTH: u32 = 512;
const HEIGHT: u32 = 512;
const SETTLE_FRAMES: u32 = 24;
const MAX_WAIT_FRAMES: u32 = 900;

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden-local/character-portable")
}

fn headless_app(resources: RenderResources, asset_root: &str) -> (App, capture::CaptureTarget) {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: asset_root.to_string(),
                meta_check: bevy::asset::AssetMetaCheck::Never,
                ..Default::default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Manual(resources),
                synchronous_pipeline_compilation: true,
                ..Default::default()
            })
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..Default::default()
            }),
    );
    app.add_plugins(StylizedPlugin::default());
    capture::install(&mut app);
    let target = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        capture::create_capture_target(&mut images, WIDTH, HEIGHT)
    };
    app.insert_resource(bevy::prelude::ClearColor(Color::srgb(0.105, 0.12, 0.13)));
    *app.world_mut().resource_mut::<StylizedViewSettings>() =
        StylizedViewSettings(StylizedViewDescriptor {
            key_light_direction: Vec3::new(0.8, 0.2, 0.35).normalize(),
            key_light_color: LinearRgba::WHITE,
            exposure: 1.0,
            day_night_factor: 1.0,
            ..StylizedViewDescriptor::default()
        });
    app.finish();
    app.cleanup();
    (app, target)
}

fn gpu_info(app: &App) -> (String, String) {
    app.world()
        .get_resource::<RenderAdapterInfo>()
        .map(|info| {
            (
                format!("{} ({:?})", info.name, info.backend),
                format!("{} {}", info.driver, info.driver_info),
            )
        })
        .unwrap_or_else(|| ("unknown".to_owned(), "unknown".to_owned()))
}

fn capture_frame(app: &mut App, target: &capture::CaptureTarget) -> CapturedImage {
    let request = capture::request_capture(app.world_mut(), target.clone());
    for _ in 0..MAX_WAIT_FRAMES {
        app.update();
        if let Some(frame) = capture::try_take(app.world_mut(), request.clone()) {
            return frame;
        }
    }
    panic!("capture readback timed out");
}

fn evaluate_case(
    dir: &Path,
    name: &str,
    captured: &CapturedImage,
    gpu: &str,
    driver: &str,
    fixture_prefix: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let paths = GoldenPaths::new(dir, name);
    let mut provenance = GoldenProvenance::for_capture(
        "engine-golden-v1",
        None,
        "synthetic-or-embedded",
        None,
        "blender-gltf-npr-v1",
        "bevy_stylized.material/v1",
        &format!("{fixture_prefix}/{name}"),
        "bevy-stylized-render-0.1.0",
        gpu,
        driver,
        captured,
    );
    if !paths.baseline_exists() {
        golden_harness::write_baseline(&paths, captured, &mut provenance)?;
        eprintln!("[baselined] {fixture_prefix}/{name}");
        return Ok(true);
    }
    let (baseline, _) = golden_harness::read_baseline(&paths)?;
    let stats = diff_rgba8(&baseline.data, &captured.data)?;
    let report = golden_harness::DiffReport::evaluate(
        name.to_owned(),
        paths.png.to_string_lossy().into_owned(),
        stats,
        Tolerance::default(),
    );
    eprintln!(
        "[{}] {fixture_prefix}/{name} max_delta={} mean_delta={:.4} psnr={:.2}dB",
        if report.passed { "matched" } else { "exceeded" },
        report.stats.max_channel_delta,
        report.stats.mean_absolute_delta,
        report.stats.psnr_db,
    );
    Ok(report.passed)
}

/// 程序化 face SDF：水平方向的"受光角度"编码（左亮右暗的 SDF 渐变，
/// 中心为鼻梁）。这不是 profile 资产，只是合成测试纹理。
fn procedural_face_sdf() -> Image {
    let (w, h) = (256usize, 256usize);
    let mut data = Vec::with_capacity(w * h * 4);
    for _y in 0..h {
        for x in 0..w {
            // sdf 值 = 该纹素被照亮的阈值（0 = 总是受光, 1 = 总是阴影）。
            // 左半张脸亮（阈值低），向右渐变到暗。
            let u = x as f32 / (w - 1) as f32;
            let sdf = u; // 线性：x 越大越晚被照亮
            let v = (sdf * 255.0) as u8;
            data.extend_from_slice(&[v, v, v, 255]);
        }
    }
    let mut image = Image::new_fill(
        Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    image.data = Some(data);
    image.sampler = ImageSampler::linear();
    image
}

/// ILM：R=AO=1（不遮挡），G=hair mask 横向条纹，A=emission 中心块。
fn procedural_ilm() -> Image {
    let (w, h) = (64usize, 64usize);
    let mut data = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let hair = if y < h / 2 { 255u8 } else { 0 };
            let emission = if x > w / 4 && x < w * 3 / 4 && y > h / 4 && y < h * 3 / 4 {
                255u8
            } else {
                0
            };
            data.extend_from_slice(&[255, hair, 0, emission]);
        }
    }
    let mut image = Image::new_fill(
        Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    image.data = Some(data);
    image.sampler = ImageSampler::linear();
    image
}

/// 合成用例：三个球共享一个 Character 材质，MeshTag 携带不同 face 阈值
/// （中间球用材质默认；左右球逐实例阈值），外加 hair/emission ILM。
fn synthetic_character_scene(app: &mut App) {
    let face_sdf = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(procedural_face_sdf());
    let ilm = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(procedural_ilm());
    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedCharacterMaterial>>()
        .add(StylizedCharacterMaterial {
            params: StylizedCharacterParams::default()
                .with_face_forward(Vec3::Z, 0.05)
                .with_face_threshold_bias(0.5)
                .with_hair_specular(24.0, 0.6)
                .with_emission(LinearRgba::new(1.0, 0.6, 0.2, 1.0), 2.0),
            base_color_texture: None,
            ilm_texture: Some(ilm),
            face_sdf_texture: Some(face_sdf),
            ..StylizedCharacterMaterial::default()
        });
    let sphere = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(bevy::math::primitives::Sphere::new(0.9));
    // 左：实例阈值低（全亮）；中：材质默认；右：实例阈值高（更暗）。
    for (i, x) in [-2.2, 0.0, 2.2].into_iter().enumerate() {
        let threshold = [Some(0.15), None, Some(0.9)][i];
        app.world_mut().spawn((
            Mesh3d(sphere.clone()),
            MeshMaterial3d(material.clone()),
            StylizedInstanceData {
                face_threshold: threshold,
                ..StylizedInstanceData::default()
            },
            Transform::from_xyz(x, 0.0, 0.0),
        ));
    }
}

#[test]
fn character_synthetic_golden() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("golden-character: no GPU adapter available; skipping");
        return;
    };
    let (mut app, target) = headless_app(resources, "assets");
    synthetic_character_scene(&mut app);
    app.world_mut().spawn((
        Camera3d::default(),
        capture::capture_target_bundle(&target),
        Tonemapping::None,
        Transform::from_xyz(0.0, 0.0, 5.2).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    for _ in 0..SETTLE_FRAMES {
        app.update();
    }
    let captured = capture_frame(&mut app, &target);
    let (gpu, driver) = gpu_info(&app);
    let ok = evaluate_case(
        &golden_dir(),
        "face_sdf_instances",
        &captured,
        &gpu,
        &driver,
        "character-portable",
    )
    .expect("golden evaluation");
    assert!(ok, "character synthetic case exceeded tolerance");
}

// ---------------------------------------------------------------------------
// Odette character case（env-gated，REPORT-only 对比 legacy）
// ---------------------------------------------------------------------------

const TENACITY_ROOT_ENV: &str = "TENACITY_ROOT";
const DEFAULT_TENACITY_ROOT: &str = "/home/x/Projects/Tenacity";
const ODETTE_GLB: &str = "models/odette.glb";
const ODETTE_HEAD_BONE: &str = "DEF-spine.006";
// Tenacity view-ink 摆放标定（取景常数，非着色语义）。
const ODETTE_SCALE: f32 = 31.5;
const ODETTE_FEET_OFFSET_Y: f32 = 24.5;
const ODETTE_FORWARD_YAW_RADIANS: f32 = core::f32::consts::PI;

fn tenacity_asset_root() -> PathBuf {
    std::env::var(TENACITY_ROOT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_TENACITY_ROOT))
        .join("crates/client-bevy/assets")
}

fn odette_assets_available() -> bool {
    let root = tenacity_asset_root();
    root.join(ODETTE_GLB).is_file()
        && root.join("models/odette-npr/face_lightmap.png").is_file()
        && root.join("models/odette-npr/body_lightmap.png").is_file()
        && root.join("models/odette-npr/hair_lightmap.png").is_file()
}

#[derive(Resource, Default)]
struct AdoptState {
    done: bool,
}

#[derive(Resource, Clone)]
struct OdetteAssets {
    face_sdf: Handle<Image>,
    hair_ilm: Handle<Image>,
    body_ilm: Handle<Image>,
}

/// Character adopt：face 用 SDF 共享材质 + MeshTag 实例数据；body/hair 用
/// ILM AO + hair 高光。
fn adopt_odette_character(
    mut commands: bevy::prelude::Commands,
    mut state: ResMut<AdoptState>,
    sidecars: Option<Res<OdetteAssets>>,
    standard: Res<Assets<StandardMaterial>>,
    images: Res<Assets<Image>>,
    mut materials: ResMut<Assets<StylizedCharacterMaterial>>,
    mut outlines: ResMut<Assets<StylizedOutlineMaterial>>,
    meshes: Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&GltfMaterialName>,
    )>,
) {
    if state.done || meshes.is_empty() {
        return;
    }
    let Some(sidecars) = sidecars else { return };
    let ready = meshes
        .iter()
        .all(|(_, handle, _)| standard.contains(&handle.0))
        && [&sidecars.face_sdf, &sidecars.hair_ilm, &sidecars.body_ilm]
            .into_iter()
            .all(|h| images.contains(h));
    if !ready {
        return;
    }
    let outline = outlines.add(StylizedOutlineMaterial {
        params: StylizedOutlineParams::default()
            .with_color(LinearRgba::new(0.03, 0.035, 0.05, 1.0))
            .with_width(2.0),
    });
    // 共享材质（face 一张、body 一张、hair 一张）；逐实例差异经
    // StylizedInstanceData/MeshTag。
    let face_material = materials.add(StylizedCharacterMaterial {
        params: StylizedCharacterParams::default()
            .with_face_forward(Vec3::Z, 0.02)
            .with_face_threshold_bias(0.5),
        base_color_texture: None, // 由源 StandardMaterial 填入（见下）
        ilm_texture: None,
        face_sdf_texture: Some(sidecars.face_sdf.clone()),
        double_sided: false,
    });
    let mut face_material_handle_used = false;
    for (entity, handle, material_name) in &meshes {
        let source = standard.get(&handle.0).expect("checked above");
        let factor = LinearRgba::from(source.base_color);
        let name = material_name.map(|n| n.0.as_str()).unwrap_or("");
        let is_face = name.contains("Face") || name.contains("Brow");
        let is_hair = name.contains("Hair");
        let base = StylizedCharacterParams {
            cel: StylizedCelParams {
                base_color_factor: bevy::math::Vec4::new(
                    factor.red,
                    factor.green,
                    factor.blue,
                    1.0,
                ),
                ..StylizedCelParams::default()
            }
            .with_shadow(0.5, 0.12, 0.35)
            .with_rim(3.0, 0.3, LinearRgba::new(0.5, 0.6, 0.8, 1.0)),
            ..StylizedCharacterParams::default()
        };
        let handle = if is_face {
            // 共享 face 材质：克隆参数、换入 base 纹理（face/brow 共享
            // face 贴图族；差异经实例通道）。
            if !face_material_handle_used {
                if let Some(mut material) = materials.get_mut(&face_material) {
                    material.base_color_texture = source.base_color_texture.clone();
                    material.sync_flags();
                }
                face_material_handle_used = true;
                face_material.clone()
            } else {
                face_material.clone()
            }
        } else {
            let params = if is_hair {
                base.with_hair_specular(24.0, 0.5)
            } else {
                base
            };
            materials.add(StylizedCharacterMaterial {
                params,
                base_color_texture: source.base_color_texture.clone(),
                ilm_texture: Some(if is_hair {
                    sidecars.hair_ilm.clone()
                } else {
                    sidecars.body_ilm.clone()
                }),
                face_sdf_texture: None,
                double_sided: false,
            })
        };
        let mut entity_commands = commands.entity(entity);
        entity_commands
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert(MeshMaterial3d(handle))
            .insert(StylizedOutline::new(outline.clone()));
        if is_face {
            entity_commands.insert(StylizedInstanceData {
                face_threshold: None, // 用材质基准
                ..StylizedInstanceData::default()
            });
        }
    }
    state.done = true;
}

fn scene_world_bounds(app: &mut App) -> Option<(Vec3, f32)> {
    let mut query = app
        .world_mut()
        .query::<(&Aabb, &GlobalTransform, &Visibility)>();
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    let mut any = false;
    for (aabb, transform, visibility) in query.iter(app.world()) {
        if visibility == Visibility::Hidden {
            continue;
        }
        let center = aabb.center;
        let half = aabb.half_extents;
        for corner in 0..8 {
            let offset = bevy::math::Vec3A::new(
                if corner & 1 == 0 { -half.x } else { half.x },
                if corner & 2 == 0 { -half.y } else { half.y },
                if corner & 4 == 0 { -half.z } else { half.z },
            );
            let world = transform.transform_point(Vec3::from(center + offset));
            min = min.min(world);
            max = max.max(world);
            any = true;
        }
    }
    any.then(|| {
        let center = (min + max) * 0.5;
        let radius = (max - min).length() * 0.5;
        (center, radius)
    })
}

fn set_camera(app: &mut App, case: &str) {
    let (center, radius) = scene_world_bounds(app).expect("adopted scene has bounds");
    let (eye, look) = match case {
        "full_body" => {
            let direction = Vec3::new(0.25, 0.12, -1.0).normalize();
            let distance = radius * 1.05 / (std::f32::consts::FRAC_PI_4 * 0.5).tan();
            (center + direction * distance, center)
        }
        "face_closeup" => {
            let head = {
                let mut query = app
                    .world_mut()
                    .query_filtered::<(&Name, &GlobalTransform), Without<Camera3d>>();
                query
                    .iter(app.world())
                    .find(|(name, _)| name.as_str() == ODETTE_HEAD_BONE)
                    .map(|(_, transform)| transform.translation())
                    .expect("head bone transform must exist")
            };
            let direction = Vec3::new(0.3, 0.1, -1.0).normalize();
            (head + direction * radius * 0.26, head)
        }
        other => panic!("unknown case {other}"),
    };
    let mut cameras = app
        .world_mut()
        .query_filtered::<&mut Transform, With<Camera3d>>();
    for mut transform in cameras.iter_mut(app.world_mut()) {
        *transform = Transform::from_translation(eye).looking_at(look, Vec3::Y);
    }
}

fn legacy_golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden-local/odette")
}

#[test]
fn odette_character_golden_with_legacy_report() {
    if !odette_assets_available() {
        eprintln!(
            "odette-character: assets not found under {} (set {TENACITY_ROOT_ENV}); skipping",
            tenacity_asset_root().display()
        );
        return;
    }
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("odette-character: no GPU adapter available; skipping");
        return;
    };
    let asset_root = tenacity_asset_root().to_string_lossy().into_owned();
    let (mut app, target) = headless_app(resources, &asset_root);

    let sidecars = {
        let asset_server = app.world().resource::<bevy::asset::AssetServer>();
        OdetteAssets {
            face_sdf: asset_server.load("models/odette-npr/face_lightmap.png"),
            hair_ilm: asset_server.load("models/odette-npr/hair_lightmap.png"),
            body_ilm: asset_server.load("models/odette-npr/body_lightmap.png"),
        }
    };
    app.insert_resource(sidecars).init_resource::<AdoptState>();
    let scene = app
        .world()
        .resource::<bevy::asset::AssetServer>()
        .load(GltfAssetLabel::Scene(0).from_asset(ODETTE_GLB));
    app.world_mut().spawn((
        WorldAssetRoot(scene),
        Transform {
            translation: Vec3::new(0.0, ODETTE_FEET_OFFSET_Y, 0.0),
            rotation: Quat::from_rotation_y(ODETTE_FORWARD_YAW_RADIANS),
            scale: Vec3::splat(ODETTE_SCALE),
        },
    ));
    app.world_mut().spawn((
        Camera3d::default(),
        capture::capture_target_bundle(&target),
        Tonemapping::None,
    ));
    app.add_systems(bevy::prelude::Update, adopt_odette_character);

    let mut waited = 0;
    loop {
        app.update();
        waited += 1;
        if app.world().resource::<AdoptState>().done {
            break;
        }
        if waited > MAX_WAIT_FRAMES {
            panic!("Odette character adoption timed out");
        }
    }
    for _ in 0..SETTLE_FRAMES {
        app.update();
    }

    let (gpu, driver) = gpu_info(&app);
    let dir = golden_dir();
    for case in ["full_body", "face_closeup"] {
        set_camera(&mut app, case);
        for _ in 0..SETTLE_FRAMES {
            app.update();
        }
        let captured = capture_frame(&mut app, &target);
        let ok = evaluate_case(
            &dir,
            &format!("odette_{case}"),
            &captured,
            &gpu,
            &driver,
            "character-portable",
        )
        .expect("golden evaluation");
        assert!(ok, "odette character case {case} exceeded tolerance");

        // legacy engine golden 对比：REPORT，不是门禁（provisional）。
        let legacy = legacy_golden_dir().join(format!("{case}.png"));
        if legacy.is_file() {
            let legacy_pixels =
                golden_harness::decode_png(&std::fs::read(&legacy).expect("legacy png"))
                    .expect("decode legacy");
            let stats = diff_rgba8(&legacy_pixels.data, &captured.data).expect("same size");
            let report = golden_harness::DiffReport::evaluate(
                format!("odette_{case}.vs_legacy"),
                legacy.to_string_lossy().into_owned(),
                stats,
                Tolerance::default(),
            );
            let report_path = dir.join(format!("odette_{case}.vs_legacy.diff.json"));
            std::fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap())
                .expect("write legacy diff report");
            eprintln!(
                "[report] character odette_{case} vs legacy: mean_delta={:.4} psnr={:.2}dB differing={}/{}",
                report.stats.mean_absolute_delta,
                report.stats.psnr_db,
                report.stats.differing_pixels,
                report.stats.total_pixels,
            );
        }
    }
}
