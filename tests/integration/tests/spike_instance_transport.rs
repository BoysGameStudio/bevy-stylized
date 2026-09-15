//! ADR-0004 spike（证据采集，不固化 ABI，全部 provisional）。
//!
//! 在 Cel@Portable 材质之上对比两种 per-instance 数据通道在
//! automatic instancing 下的表现：
//!
//! - **A. MeshTag bits**：`MeshTag(i)` 直接携带行索引（shader 内
//!   `mesh_functions::get_tag` 读取，映射 ramp 行）；
//! - **B. storage buffer 表**：`#[storage(0, read_only)] ShaderBuffer` 表 +
//!   `MeshTag` 作索引（对应 GpuArrayBuffer 家族；WebGL2/GLES3.0 无 storage
//!   buffer，Portable 不可用的结论作为证据记录）。
//!
//! 测量（100 / 1000 实例，同 mesh+material）：bin/draw 数（批是否被破坏）
//! 与主线程 `app.update()` 平均耗时（粗略 CPU prepare 成本）。
//!
//! 结果写入 `target/tmp/adr-0004-spike/`（gitignored）。无 GPU 时 skip。

use std::time::Instant;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    core_pipeline::core_3d::Opaque3d,
    core_pipeline::tonemapping::Tonemapping,
    mesh::MeshTag,
    pbr::{Material, MeshMaterial3d},
    prelude::{Camera3d, Handle, Mesh, Mesh3d, Resource, Transform, Vec3},
    render::{
        RenderApp, RenderPlugin,
        render_phase::ViewBinnedRenderPhases,
        render_resource::AsBindGroup,
        settings::{RenderCreation, RenderResources},
        storage::ShaderBuffer,
    },
    shader::{Shader, ShaderRef},
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::*;

const SETTLE_FRAMES: u32 = 20;
const TIMED_FRAMES: u32 = 40;

/// Spike 用 WGSL（方案 B 专用；内联避免依赖资产管线路径）。
const SPIKE_WGSL: &str = r"
#import bevy_pbr::{
    forward_io::{Vertex, VertexOutput, FragmentOutput},
    mesh_functions,
    view_transformations::position_world_to_clip,
}

struct SpikeTable {
    colors: array<vec4<f32>>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<storage, read> spike_colors: array<vec4<f32>>;

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex_in.position, 1.0),
    );
    out.position = position_world_to_clip(out.world_position.xyz);
#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex_in.normal,
        vertex_in.instance_index,
    );
#endif
#ifdef VERTEX_UVS_A
    out.uv = vertex_in.uv;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex_in.instance_index;
#endif
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    let index = mesh_functions::get_tag(in.instance_index);
    var out: FragmentOutput;
    out.color = spike_colors[index];
    return out;
}
";

/// 方案 B 的 spike 材质（仅测试用；不进公共 API）。
#[derive(bevy::prelude::Asset, bevy::reflect::TypePath, AsBindGroup, Clone, Debug)]
struct SpikeStorageMaterial {
    #[storage(0, read_only)]
    colors: Handle<ShaderBuffer>,
}

static SPIKE_SHADER: std::sync::OnceLock<Handle<Shader>> = std::sync::OnceLock::new();

impl Material for SpikeStorageMaterial {
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Handle(SPIKE_SHADER.get().expect("shader installed").clone())
    }

    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(SPIKE_SHADER.get().expect("shader installed").clone())
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }
}

#[derive(Resource, Clone)]
struct SpikeTarget(golden_harness::CaptureTarget);

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
    app.add_plugins(StylizedPlugin::default());
    app.add_plugins(bevy::pbr::MaterialPlugin::<SpikeStorageMaterial>::default());
    let shader = app
        .world_mut()
        .resource_mut::<Assets<Shader>>()
        .add(Shader::from_wgsl(SPIKE_WGSL, "spike_instance.wgsl"));
    let _ = SPIKE_SHADER.set(shader);
    let target = {
        let mut images = app.world_mut().resource_mut::<Assets<bevy::image::Image>>();
        golden_harness::capture::create_capture_target(&mut images, 256, 256)
    };
    app.insert_resource(SpikeTarget(target));
    app.finish();
    app.cleanup();
    app
}

fn spawn_grid<B: bevy::prelude::Bundle>(
    app: &mut App,
    mesh: Handle<Mesh>,
    n: u32,
    spawn: impl Fn(u32) -> B,
) {
    let cols = (n as f32).sqrt().ceil() as u32;
    let world = app.world_mut();
    for i in 0..n {
        let x = (i % cols) as f32 - cols as f32 * 0.5;
        let y = (i / cols) as f32 * 0.5;
        world.spawn((
            Mesh3d(mesh.clone()),
            MeshTag(i),
            Transform::from_xyz(x, y, 0.0),
            spawn(i),
        ));
    }
    let half = cols as f32 * 0.5;
    let target = app.world().resource::<SpikeTarget>().0.clone();
    app.world_mut().spawn((
        Camera3d::default(),
        golden_harness::capture::capture_target_bundle(&target),
        Tonemapping::None,
        Transform::from_xyz(0.0, half * 0.25, half * 2.5)
            .looking_at(Vec3::new(0.0, half * 0.25, 0.0), Vec3::Y),
    ));
}

/// 数 Opaque3d binned phase 的 draw 数（bin + multidraw batch set 计数）。
///
/// 注：multidrawable batch set 的实例数是 pub(crate) 不可见；draw 数本身
/// 即为合批证据（未合批时 draw 数 ≈ 实例数）。
fn count_draws(app: &mut App) -> usize {
    let render_app = app.get_sub_app_mut(RenderApp).expect("render app");
    let Some(phases) = render_app
        .world()
        .get_resource::<ViewBinnedRenderPhases<Opaque3d>>()
    else {
        return 0;
    };
    let mut draws = 0;
    for phase in phases.0.values() {
        draws += phase.multidrawable_meshes.len();
        draws += phase.batchable_meshes.len();
    }
    draws
}

#[derive(serde::Serialize)]
struct SpikeMeasurement {
    scenario: &'static str,
    instances: u32,
    opaque_draw_calls: usize,
    avg_update_ms: f64,
}

fn run_scenario(
    resources: &RenderResources,
    scenario: &'static str,
    instances: u32,
    build: impl FnOnce(&mut App),
) -> SpikeMeasurement {
    let mut app = headless_app(resources.clone());
    build(&mut app);
    for _ in 0..SETTLE_FRAMES {
        app.update();
    }
    let draws = count_draws(&mut app);
    let start = Instant::now();
    for _ in 0..TIMED_FRAMES {
        app.update();
    }
    let avg_update_ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(TIMED_FRAMES);
    SpikeMeasurement {
        scenario,
        instances,
        opaque_draw_calls: draws,
        avg_update_ms,
    }
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn adr_0004_instance_transport_spike() {
    let Some(resources) = golden_harness::probe_render_resources() else {
        eprintln!("adr-0004 spike: no GPU adapter available; skipping");
        return;
    };

    let mut measurements = Vec::new();
    for instances in [100_u32, 1000] {
        // 方案 A：StylizedCelMaterial + MeshTag（ramp 行索引）。
        let m = run_scenario(&resources, "a_mesh_tag", instances, |app| {
            let ramp = {
                let mut data = Vec::new();
                for row in 0..4u8 {
                    let color = match row {
                        0 => [80, 80, 220, 255],
                        1 => [220, 80, 80, 255],
                        2 => [80, 220, 80, 255],
                        _ => [220, 220, 80, 255],
                    };
                    for _ in 0..64 {
                        data.extend_from_slice(&color);
                    }
                }
                let mut image = bevy::image::Image::new_fill(
                    bevy::render::render_resource::Extent3d {
                        width: 64,
                        height: 4,
                        depth_or_array_layers: 1,
                    },
                    bevy::render::render_resource::TextureDimension::D2,
                    &[0, 0, 0, 255],
                    bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                    bevy::asset::RenderAssetUsages::default(),
                );
                image.data = Some(data);
                app.world_mut()
                    .resource_mut::<Assets<bevy::image::Image>>()
                    .add(image)
            };
            let mesh = app
                .world_mut()
                .resource_mut::<Assets<Mesh>>()
                .add(bevy::math::primitives::Sphere::new(0.3));
            let material = app
                .world_mut()
                .resource_mut::<Assets<StylizedCelMaterial>>()
                .add(StylizedCelMaterial {
                    params: StylizedCelParams::default().with_ramp_rows(4.0),
                    ramp_texture: Some(ramp),
                    ..StylizedCelMaterial::default()
                });
            spawn_grid(app, mesh, instances, |_| MeshMaterial3d(material.clone()));
        });
        measurements.push(m);

        // 方案 B：storage buffer 表 + MeshTag 索引。
        let m = run_scenario(&resources, "b_storage_table", instances, |app| {
            let colors: Vec<[f32; 4]> = (0..instances)
                .map(|i| {
                    let t = i as f32 / instances as f32;
                    [t, 1.0 - t, 0.5, 1.0]
                })
                .collect();
            let buffer = app
                .world_mut()
                .resource_mut::<Assets<ShaderBuffer>>()
                .add(ShaderBuffer::from(colors));
            let mesh = app
                .world_mut()
                .resource_mut::<Assets<Mesh>>()
                .add(bevy::math::primitives::Sphere::new(0.3));
            let material = app
                .world_mut()
                .resource_mut::<Assets<SpikeStorageMaterial>>()
                .add(SpikeStorageMaterial { colors: buffer });
            spawn_grid(app, mesh, instances, |_| MeshMaterial3d(material.clone()));
        });
        measurements.push(m);
    }

    let (gpu, features, storage_limit) = {
        let (device, info) = (&resources.0, &resources.2);
        (
            format!("{} ({:?})", info.name, info.backend),
            format!("{:?}", device.features()),
            device.limits().max_storage_buffers_per_shader_stage,
        )
    };

    #[derive(serde::Serialize)]
    struct SpikeReport {
        provisional: bool,
        adr: &'static str,
        gpu: String,
        device_features: String,
        max_storage_buffers_per_shader_stage: u32,
        portable_notes: &'static str,
        measurements: Vec<SpikeMeasurement>,
    }
    let report = SpikeReport {
        provisional: true,
        adr: "ADR-0004",
        gpu,
        device_features: features,
        max_storage_buffers_per_shader_stage: storage_limit,
        portable_notes: "方案 B 依赖 fragment/vertex stage 的 storage buffer：\
            WebGL2/GLES3.0 baseline 不可用（downlevel_webgl2 limits 中 \
            max_storage_buffers_per_shader_stage = 0），Portable 只能走 \
            方案 A（MeshTag）或 GpuArrayBuffer 的 dynamic uniform fallback。",
        measurements,
    };
    let out_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp/adr-0004-spike");
    std::fs::create_dir_all(&out_dir).expect("create spike dir");
    std::fs::write(
        out_dir.join("spike_results.json"),
        serde_json::to_string_pretty(&report).expect("serialize"),
    )
    .expect("write spike json");

    let mut summary = String::from(
        "# ADR-0004 instance transport spike (provisional)\n\n\
         draw call 数来自 Opaque3d binned phase 的 bin/batch-set 计数；\
         同 mesh+material 实例全部合批 ⇒ draw 数 ≈ 1-2 且与实例数无关。\n\n\
         | scenario | instances | draw calls | avg update ms |\n\
         |---|---|---|---|\n",
    );
    for m in &report.measurements {
        summary.push_str(&format!(
            "| {} | {} | {} | {:.3} |\n",
            m.scenario, m.instances, m.opaque_draw_calls, m.avg_update_ms
        ));
    }
    summary.push_str(&format!(
        "\nGPU: {}\nstorage buffers/stage: {}\n\n{}\n",
        report.gpu, report.max_storage_buffers_per_shader_stage, report.portable_notes
    ));
    std::fs::write(out_dir.join("summary.md"), &summary).expect("write summary");
    eprintln!("{summary}");

    // 合批 gate（spike 级断言）：两方案在 1000 实例下 draw 数都必须远小于实例数。
    for m in &report.measurements {
        assert!(
            m.opaque_draw_calls * 50 <= m.instances as usize,
            "{} @ {} instances: batching looks broken ({} draws)",
            m.scenario,
            m.instances,
            m.opaque_draw_calls,
        );
    }
}
