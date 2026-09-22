//! ADR-0001 spike（证据采集，provisional——不关闭 ADR）。
//!
//! Runtime material backend 三方对比（docs/architecture/02）：
//!
//! - **A. ExtendedMaterial**：`ExtendedMaterial<StandardMaterial,
//!   SpikeCelExtension>`（本测试内的最小原型；获得 StandardMaterial 的
//!   shadow/fog/prepass 集成，代价是 base 绑定成本）；
//! - **B. Direct Material**：生产路径 `StylizedCelMaterial`（当前 shipping
//!   形态）；
//! - **C. Custom RenderPipeline**：不在本 spike 实现——集成面（自建
//!   phase/draw/queue/绑定管理，失去 Bevy 材质管线复用）记录在
//!   summary.md。
//!
//! 测量（500 实例同 mesh+material）：管线就绪时间（首帧可捕获）与
//! steady-state 平均 `app.update()` 耗时。结果写
//! `target/tmp/adr-0001-spike/`（gitignored）。

use std::time::Instant;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    core_pipeline::tonemapping::Tonemapping,
    light::DirectionalLight,
    math::primitives::Sphere,
    pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin, MeshMaterial3d, StandardMaterial},
    prelude::Handle,
    prelude::{Camera3d, Mesh, Mesh3d, Reflect, Transform, Vec3},
    render::{
        RenderPlugin,
        render_resource::{AsBindGroup, ShaderType},
        settings::{RenderCreation, RenderResources},
    },
    shader::{Shader, ShaderRef},
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::*;

const INSTANCES: u32 = 500;
const SETTLE_FRAMES: u32 = 12;
const TIMED_FRAMES: u32 = 40;

// ---------------------------------------------------------------------------
// 方案 A 原型：ExtendedMaterial + 最小 cel 扩展
// ---------------------------------------------------------------------------

static EXTENSION_SHADER: std::sync::OnceLock<Handle<Shader>> = std::sync::OnceLock::new();

/// 扩展 fragment：PBR 输入 → half-lambert toon remap（物理绑定全保留）。
const EXTENSION_WGSL: &str = r"
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    mesh_view_bindings::{lights, view},
}

struct SpikeCelExt {
    // x: threshold, y: softness, z: floor
    shading: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> ext: SpikeCelExt;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr = pbr_input_from_standard_material(in, is_front);
    var light_direction = vec3<f32>(0.0, 1.0, 0.0);
    var light_color = vec3<f32>(1.0);
    if lights.n_directional_lights > 0u {
        light_direction = lights.directional_lights[0].direction_to_light;
        light_color = lights.directional_lights[0].color.rgb;
    }
    let half_lambert = clamp(dot(pbr.N, light_direction) * 0.5 + 0.5, 0.0, 1.0);
    let t = smoothstep(ext.shading.x - ext.shading.y, ext.shading.x + ext.shading.y, half_lambert);
    let factor = mix(ext.shading.z, 1.0, t);
    var out: FragmentOutput;
    out.color = vec4<f32>(
        pbr.material.base_color.rgb * (lights.ambient_color.rgb + light_color * factor)
            * view.exposure,
        1.0,
    );
    return out;
}
";

#[derive(Clone, Copy, Debug, PartialEq, Reflect, ShaderType)]
struct SpikeCelExtParams {
    shading: Vec3,
    _pad: f32,
}

/// ADR-0001 方案 A 原型扩展（仅 spike；不进公共 API）。
#[derive(bevy::prelude::Asset, AsBindGroup, Clone, Debug, Reflect)]
struct SpikeCelExtension {
    #[uniform(100)]
    params: SpikeCelExtParams,
}

impl MaterialExtension for SpikeCelExtension {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(EXTENSION_SHADER.get().expect("shader installed").clone())
    }
}

type ExtendedCel = ExtendedMaterial<StandardMaterial, SpikeCelExtension>;

// ---------------------------------------------------------------------------
// 场景与测量
// ---------------------------------------------------------------------------

fn headless_app(resources: RenderResources) -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: "assets".to_string(),
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
    app
}

fn spawn_grid(app: &mut App, mesh: Handle<Mesh>, material: Handle<impl bevy::pbr::Material>) {
    let cols = (INSTANCES as f32).sqrt().ceil() as u32;
    let world = app.world_mut();
    for i in 0..INSTANCES {
        let x = (i % cols) as f32 - cols as f32 * 0.5;
        let y = (i / cols) as f32 * 0.5;
        world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(x, y, 0.0),
        ));
    }
    world.spawn((
        Camera3d::default(),
        Tonemapping::None,
        Transform::from_xyz(0.0, 5.0, 25.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    world.spawn((
        DirectionalLight {
            color: bevy::prelude::Color::WHITE,
            illuminance: 1.0,
            shadow_maps_enabled: false,
            ..DirectionalLight::default()
        },
        Transform::default().looking_to(-Vec3::new(0.4, 0.8, 0.6).normalize(), Vec3::Y),
    ));
}

struct SpikeMeasurement {
    backend: &'static str,
    time_to_first_frame_ms: f64,
    avg_update_ms: f64,
}

fn measure(backend: &'static str, build: impl FnOnce(&mut App)) -> SpikeMeasurement {
    // 方案 A 每次跑新 app（OnceLock shader 句柄需可重置——用 asset 重载）。
    let Some(resources) = golden_harness::probe_render_resources() else {
        return SpikeMeasurement {
            backend,
            time_to_first_frame_ms: f64::NAN,
            avg_update_ms: f64::NAN,
        };
    };
    let mut app = headless_app(resources);
    build(&mut app);
    app.finish();
    app.cleanup();
    let start = Instant::now();
    for _ in 0..SETTLE_FRAMES {
        app.update();
    }
    let time_to_first_frame_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = Instant::now();
    for _ in 0..TIMED_FRAMES {
        app.update();
    }
    let avg_update_ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(TIMED_FRAMES);
    SpikeMeasurement {
        backend,
        time_to_first_frame_ms,
        avg_update_ms,
    }
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn adr_0001_backend_comparison_spike() {
    if golden_harness::probe_render_resources().is_none() {
        eprintln!("adr-0001 spike: no GPU adapter available; skipping");
        return;
    }

    // 方案 B：StylizedCelMaterial（当前 shipping 形态）。
    let direct = measure("B_direct_material", |app| {
        app.add_plugins(StylizedPlugin::default());
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Sphere::new(0.3));
        let material = app
            .world_mut()
            .resource_mut::<Assets<StylizedCelMaterial>>()
            .add(StylizedCelMaterial::default());
        spawn_grid(app, mesh, material);
    });

    // 方案 A：ExtendedMaterial 原型。
    let extended = measure("A_extended_material", |app| {
        app.add_plugins(MaterialPlugin::<ExtendedCel>::default());
        let shader = app
            .world_mut()
            .resource_mut::<Assets<Shader>>()
            .add(Shader::from_wgsl(EXTENSION_WGSL, "spike_cel_ext.wgsl"));
        let _ = EXTENSION_SHADER.set(shader);
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Sphere::new(0.3));
        let material =
            app.world_mut()
                .resource_mut::<Assets<ExtendedCel>>()
                .add(ExtendedMaterial {
                    base: StandardMaterial::default(),
                    extension: SpikeCelExtension {
                        params: SpikeCelExtParams {
                            shading: Vec3::new(0.5, 0.1, 0.3),
                            _pad: 0.0,
                        },
                    },
                });
        spawn_grid(app, mesh, material);
    });

    #[derive(serde::Serialize)]
    struct Report {
        provisional: bool,
        adr: &'static str,
        instances: u32,
        note: &'static str,
        measurements: Vec<MeasurementJson>,
    }
    #[derive(serde::Serialize)]
    struct MeasurementJson {
        backend: String,
        time_to_first_frame_ms: f64,
        avg_update_ms: f64,
    }
    let measurements = [&direct, &extended]
        .into_iter()
        .map(|m| MeasurementJson {
            backend: m.backend.to_string(),
            time_to_first_frame_ms: m.time_to_first_frame_ms,
            avg_update_ms: m.avg_update_ms,
        })
        .collect::<Vec<_>>();
    let report = Report {
        provisional: true,
        adr: "ADR-0001",
        instances: INSTANCES,
        note: "time_to_first_frame 含管线编译（synchronous_pipeline_compilation）；\
               avg_update 为 steady-state 主线程帧耗时。方案 C（custom render \
               pipeline）未实现——集成面见 summary.md。",
        measurements,
    };
    let out_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp/adr-0001-spike");
    std::fs::create_dir_all(&out_dir).expect("create spike dir");
    std::fs::write(
        out_dir.join("spike_results.json"),
        serde_json::to_string_pretty(&report).expect("serialize"),
    )
    .expect("write spike json");

    let summary = format!(
        "# ADR-0001 backend comparison spike (provisional)\n\n\
         场景：{INSTANCES} 实例，同 mesh+material，RTX 5090。\n\n\
         | backend | time-to-first-frame ms | avg update ms |\n\
         |---|---|---|\n\
         | A ExtendedMaterial | {:.1} | {:.3} |\n\
         | B Direct Material（shipping） | {:.1} | {:.3} |\n\n\
         ## 定性对比\n\n\
         - A 获得 StandardMaterial 的 shadow/fog/prepass/deferred 集成与全部\n\
           base 纹理槽位，代价是 base 绑定成本与 per-material 更大的\n\
           bind group；扩展数学挂在 PBR 输入之后，组合自由度受 pbr_input 约束。\n\
         - B 完全控制绑定布局与 shader（budget 纪律、NPR 语义纯净），代价是\n\
           shadow/fog 等需要自接（当前切片显式关闭，矩阵见 ADR-0023）。\n\
         - C（custom render pipeline）未原型化：需要自建 render phase、draw\n\
           function、queue/bin 逻辑与绑定管理，失去 Bevy Material 管线的全部\n\
           复用；仅在 A/B 都被测量证明不足时才值得投入。\n\n\
         ## Bindless（ADR-0003 证据条款，不实施）\n\n\
         Bindless 留在 Portable 之外（不变量）。重访阈值（测量方法）：当\n\
         单帧 draw 数 × 材质数导致 bind group 创建/切换成本在 profile 中\n\
         超过 5% 帧时间，或材质变体数超过固定绑定可维护范围（经验阈值：\n\
         >64 活跃材质变体）时，用同一场景的 binding-array 原型对比\n\
         CPU prepare 时间。当前 B 路径 500 实例 1 draw、绑定为固定布局，\n\
         无证据支持提前引入。\n",
        extended.time_to_first_frame_ms,
        extended.avg_update_ms,
        direct.time_to_first_frame_ms,
        direct.avg_update_ms,
    );
    std::fs::write(out_dir.join("summary.md"), &summary).expect("write summary");
    eprintln!("{summary}");
}
