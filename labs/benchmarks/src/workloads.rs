//! 固定 workload 配方与 runner（docs/showcase/01：测量协议、采样器、统计与
//! 性能报告归 `labs/benchmarks`；不依赖 egui/Inspector/桌面窗口）。
//!
//! 自 `apps/showcase` 迁入，数值身份逐字保留：9 个 workload 名、512²、
//! 600 帧、3 repeats、诊断路径 `render/inspector_frame/elapsed_gpu`、
//! 报告 schema `bevy_stylized.workloads/v1` 及全部 JSON 键均不改变。
//! 单视图程序化负载，GPU 时间戳 + CPU 墙钟分列记录。

use crate::measurement::{install_measurement, stats};
use bevy::{
    DefaultPlugins,
    app::PluginGroup,
    asset::{AssetMetaCheck, AssetPlugin},
    core_pipeline::tonemapping::Tonemapping,
    diagnostic::DiagnosticsStore,
    mesh::{
        VertexAttributeValues,
        morph::{MeshMorphWeights, MorphAttributes},
        skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    },
    prelude::*,
    render::{
        RenderPlugin,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        renderer::RenderDevice,
        settings::{RenderCreation, RenderResources},
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::*;
use golden_harness::{capture, readiness};
use serde_json::{Value, json};
use std::{
    path::Path,
    time::{Duration, Instant},
};

/// 固定 workload 名（数值身份；顺序与既有报告一致）。
pub const CASES: [&str; 9] = [
    "hero-1",
    "hero-3",
    "character-32",
    "portable-100",
    "portable-500",
    "portable-1000",
    "hero-skin-morph",
    "hero-outline",
    "hero-post",
];
const SIZE: u32 = 512;
const FRAMES: usize = 600;

fn tex(app: &mut App, rgba: [u8; 4], srgb: bool) -> Handle<Image> {
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_fill(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &rgba,
            if srgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            },
            bevy::asset::RenderAssetUsages::default(),
        ))
}
fn full_hero(app: &mut App) -> Handle<StylizedHeroMaterial> {
    let mut m = StylizedHeroMaterial::default();
    m.base_color_texture = Some(tex(app, [170, 190, 230, 128], true));
    m.ilm_texture = Some(tex(app, [255, 255, 128, 0], false));
    m.ramp_texture = Some(tex(app, [190, 200, 220, 255], true));
    m.packed_normal_texture = Some(tex(app, [145, 140, 255, 255], false));
    m.matcap_texture = Some(tex(app, [160, 160, 160, 255], true));
    m.spec_ramp_texture = Some(tex(app, [32, 48, 64, 255], false));
    m.params = m.params.with_genshin_ramp(0.55, 1.);
    m.params.genshin.controls = Vec4::new(1., 1., 1., 0.);
    m.params.metal.controls = Vec4::new(1., 1., 1., 1.);
    m.params.source_conventions.x = 1.;
    m.params.source_emission = Vec4::new(1.5, 1.5, 1.5, 1.);
    m.sync_flags();
    app.world_mut()
        .resource_mut::<Assets<StylizedHeroMaterial>>()
        .add(m)
}
struct SceneState {
    joint: Option<Entity>,
    morph: Option<Entity>,
}
fn build(app: &mut App, name: &str) -> (SceneState, Vec3, f32) {
    let mut state = SceneState {
        joint: None,
        morph: None,
    };
    if let Some(n) = name.strip_prefix("portable-") {
        let count: u32 = n.parse().expect("registered count");
        crate::build_portable_instances(app, count);
        let cols = (count as f32).sqrt().ceil();
        let rows = (count as f32 / cols).ceil();
        return (
            state,
            Vec3::new(-0.425, (rows - 1.) * 0.65 * 0.5, 0.),
            (cols * 0.85).max(rows * 0.65) * 1.12,
        );
    }
    if name == "character-32" {
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Sphere::new(0.25));
        let mut m = StylizedCharacterMaterial::default();
        m.params = m
            .params
            .with_hair_specular(24., 0.4)
            .with_emission(LinearRgba::new(0.1, 0.2, 0.4, 1.), 0.2);
        let mat = app
            .world_mut()
            .resource_mut::<Assets<StylizedCharacterMaterial>>()
            .add(m);
        for i in 0..32 {
            app.world_mut().spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz((i % 8) as f32 * 0.7 - 2.45, (i / 8) as f32 * 0.7 - 1.05, 0.),
            ));
        }
        return (state, Vec3::ZERO, 6.2);
    }
    let mat = full_hero(app);
    let mut mesh: Mesh = Sphere::new(0.65).into();
    if name == "hero-skin-morph" {
        let n = mesh.count_vertices();
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            VertexAttributeValues::Uint16x4(vec![[0, 0, 0, 0]; n]),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![[1., 0., 0., 0.]; n]);
        mesh.set_morph_targets(vec![
            MorphAttributes::new(
                Vec3::X * 0.1,
                Vec3::ZERO,
                Vec3::ZERO
            );
            n
        ]);
    }
    let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
    let count = if name == "hero-3" { 3 } else { 1 };
    for i in 0..count {
        let entity = app
            .world_mut()
            .spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz((i as f32 - (count - 1) as f32 * 0.5) * 1.5, 0., 0.),
            ))
            .id();
        if name == "hero-outline" {
            let outline = app
                .world_mut()
                .resource_mut::<Assets<StylizedOutlineMaterial>>()
                .add(StylizedOutlineMaterial::default());
            app.world_mut()
                .entity_mut(entity)
                .insert(StylizedOutline::new(outline));
        }
        if name == "hero-skin-morph" {
            let joint = app.world_mut().spawn(Transform::IDENTITY).id();
            let poses = app
                .world_mut()
                .resource_mut::<Assets<SkinnedMeshInverseBindposes>>()
                .add(SkinnedMeshInverseBindposes::from(vec![Mat4::IDENTITY]));
            app.world_mut().entity_mut(entity).insert((
                SkinnedMesh {
                    inverse_bindposes: poses,
                    joints: vec![joint],
                },
                MeshMorphWeights::Value { weights: vec![0.5] },
            ));
            state.joint = Some(joint);
            state.morph = Some(entity);
        }
    }
    (state, Vec3::ZERO, if count == 3 { 5.0 } else { 2.4 })
}
fn step(app: &mut App, state: &SceneState, frame: usize) {
    if let Some(joint) = state.joint {
        app.world_mut()
            .get_mut::<Transform>(joint)
            .unwrap()
            .rotation = Quat::from_rotation_y((frame as f32 / 60.).sin() * 0.3);
    }
    if let Some(entity) = state.morph
        && let Some(MeshMorphWeights::Value { weights }) = app
            .world_mut()
            .get_mut::<MeshMorphWeights>(entity)
            .as_deref_mut()
    {
        weights[0] = 0.5 + 0.4 * (frame as f32 / 60.).sin();
    }
    app.update();
}

/// workload 的 headless App 装配（Portable tier；与既有 runner 同一装配语义：
/// 同步管线编译、无窗口、capture 就绪探针与测量插件经 `configure` 注入）。
fn headless_app(resources: RenderResources, configure: impl FnOnce(&mut App)) -> App {
    let mut app = App::new();
    // 必须在 StylizedPlugin 之前（Stage B 在 finish 时读该配置）。
    app.insert_resource(StylizedCapabilityConfig {
        requested_tier: StylizedExecutionTier::Portable,
        ..StylizedCapabilityConfig::default()
    });
    app.add_plugins(
        DefaultPlugins
            .build()
            .set(AssetPlugin {
                file_path: "assets".to_string(),
                meta_check: AssetMetaCheck::Never,
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
    configure(&mut app);
    app.finish();
    app.cleanup();
    app
}

/// workload 的 view 默认值（昼间 + 斜上 key light；数值身份沿用既有 runner）。
fn default_view(app: &mut App) {
    *app.world_mut().resource_mut::<StylizedViewSettings>() =
        StylizedViewSettings(StylizedViewDescriptor {
            key_light_direction: Vec3::new(0.5, 0.8, 0.6).normalize(),
            key_light_color: LinearRgba::WHITE,
            exposure: 1.0,
            day_night_factor: 1.0,
            ..StylizedViewDescriptor::default()
        });
}

fn measure(resources: RenderResources, name: &str, out: &Path) -> Result<Value, String> {
    let mut app = headless_app(resources, |app| {
        install_measurement(app);
        readiness::install(app);
    });
    if !app
        .world()
        .resource::<RenderDevice>()
        .features()
        .contains(bevy::render::render_resource::WgpuFeatures::TIMESTAMP_QUERY)
    {
        return Err("required GPU timestamps unavailable".into());
    }
    default_view(&mut app);
    let (state, target_point, height) = build(&mut app, name);
    let target = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        capture::create_capture_target(&mut images, SIZE, SIZE)
    };
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            capture::capture_target_bundle(&target),
            Tonemapping::None,
            Msaa::Sample4,
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: bevy::camera::ScalingMode::FixedVertical {
                    viewport_height: height,
                },
                ..OrthographicProjection::default_3d()
            }),
            Transform::from_translation(target_point + Vec3::Z * 30.)
                .looking_at(target_point, Vec3::Y),
        ))
        .id();
    if name == "hero-post" {
        app.world_mut()
            .entity_mut(camera)
            .insert(StylizedPostProcess {
                vignette_strength: 0.5,
                chromatic_aberration: 0.004,
                display_transform: StylizedDisplayTransform::SimpleCompress,
                ..Default::default()
            });
    }
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut stable = 0;
    let mut last = 0;
    while stable < 24 {
        if Instant::now() > deadline {
            return Err(format!("{name}: pipeline readiness timeout"));
        }
        step(&mut app, &state, 0);
        let p = app.world().resource::<readiness::Probe>().snapshot();
        if let Some(e) = p.error {
            return Err(e);
        }
        if p.frame != last {
            last = p.frame;
            stable = if p.ready { stable + 1 } else { 0 };
        }
    }
    let outlines = app
        .world_mut()
        .query::<&StylizedOutlineReplica>()
        .iter(app.world())
        .count();
    let post_active = app
        .world()
        .get::<StylizedFullscreenEffect>(camera)
        .is_some();
    if (name == "hero-outline" && outlines != 1)
        || (name == "hero-post") != post_active
        || (name == "hero-skin-morph" && (state.joint.is_none() || state.morph.is_none()))
    {
        return Err(format!("{name}: requested workload feature is inactive"));
    }
    for i in 0..60 {
        step(&mut app, &state, i);
    }
    let mut seen = None;
    let mut gpu = Vec::new();
    let mut cpu = Vec::new();
    let path = bevy::diagnostic::DiagnosticPath::new("render/inspector_frame/elapsed_gpu");
    if let Some(m) = app
        .world()
        .resource::<DiagnosticsStore>()
        .get(&path)
        .and_then(|d| d.measurement())
    {
        seen = Some(m.time);
    }
    for i in 0..FRAMES {
        let start = Instant::now();
        step(&mut app, &state, i);
        cpu.push(start.elapsed().as_secs_f64() * 1000.);
        if let Some(m) = app
            .world()
            .resource::<DiagnosticsStore>()
            .get(&path)
            .and_then(|d| d.measurement())
            && seen != Some(m.time)
        {
            seen = Some(m.time);
            gpu.push(m.value);
        }
    }
    if gpu.len() < FRAMES * 9 / 10 || gpu.iter().any(|x| !x.is_finite() || *x <= 0.) {
        return Err(format!("{name}: invalid/insufficient GPU samples"));
    }
    let memory=app.world().resource::<RenderDevice>().wgpu_device().generate_allocator_report().map(|r|json!({"allocated_bytes":r.total_allocated_bytes,"reserved_bytes":r.total_reserved_bytes}));
    let counts = json!({"cel":app.world().resource::<Assets<StylizedCelMaterial>>().len(),"character":app.world().resource::<Assets<StylizedCharacterMaterial>>().len(),"hero":app.world().resource::<Assets<StylizedHeroMaterial>>().len(),"outline":app.world().resource::<Assets<StylizedOutlineMaterial>>().len()});
    let request = capture::request_capture(app.world_mut(), target);
    let mut image = None;
    for _ in 0..600 {
        app.update();
        if let Some(p) = capture::try_take(app.world_mut(), request.clone()) {
            image = Some(p);
            break;
        }
    }
    let image = image.ok_or("workload readback timeout")?;
    std::fs::write(
        out.join(format!("{name}.png")),
        golden_harness::encode_png(&image).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let distinct: std::collections::BTreeSet<_> = image
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| [p[0], p[1], p[2]])
        .collect();
    if distinct.len() < 8 {
        return Err(format!("{name}: empty/unshaded capture"));
    }
    Ok(
        json!({"case":name,"status":"PASS","resolution":[SIZE,SIZE],"views":1,"msaa":4,"requested_tier":"Portable","resolved":format!("{:?}",app.world().resource::<StylizedResolvedCapabilities>()),"warmup":60,"requested_samples":FRAMES,"cpu_update_ms":stats(&cpu),"gpu_render_graph_ms":stats(&gpu),"raw_gpu_ms":gpu,"raw_cpu_ms":cpu,"allocator":memory,"material_assets":counts,"active_features":{"outline_replicas":outlines,"post":post_active,"skin_morph":state.morph.is_some()},"capture":format!("{name}.png"),"image_fnv1a64":golden_harness::fnv1a64_hex(&image.data),"draw_calls":"UNAVAILABLE","scope":"procedural single-view workload; render-graph timestamps, not presented FPS; capture performed after timing"}),
    )
}

/// 运行全部 9 个 workload × 3 repeats，写出 `report.json`（schema 与键不变）。
pub fn run(args: &[String]) -> Result<(), String> {
    let out = args
        .windows(2)
        .find(|w| w[0] == "--out")
        .map(|w| std::path::PathBuf::from(&w[1]))
        .ok_or("workloads requires --out NEW_DIRECTORY")?;
    if out.exists() {
        return Err("workload output exists".into());
    }
    let resources = capture::probe_render_resources().ok_or("required GPU unavailable")?;
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let device = format!("{:?}", resources.2.0);
    let mut cells = Vec::new();
    for repeat in 0..3 {
        let dir = out.join(format!("repeat-{repeat}"));
        std::fs::create_dir(&dir).map_err(|e| e.to_string())?;
        for name in CASES {
            println!("WORKLOAD {repeat} {name}");
            let mut cell = measure(resources.clone(), name, &dir)?;
            cell["repeat"] = json!(repeat);
            cell["capture"] = json!(format!("repeat-{repeat}/{name}.png"));
            cells.push(cell);
        }
    }
    let report = json!({"schema":"bevy_stylized.workloads/v1","status":"PASS","collection_status":"PASS","regression_status":"NOT_EXECUTED","absolute_budget_status":"NOT_DEFINED","optimized":!cfg!(debug_assertions),"device":device,"expected_cases":CASES,"repeats":3,"cells":cells,"scope":"timestamp collection plus nonempty captures; candidate reference data, no accepted new Golden or absolute performance budget"});
    std::fs::write(
        out.join("report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
