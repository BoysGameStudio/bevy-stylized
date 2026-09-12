//! V2-e GPU Instance Database 的集成证据（GPU-gated，无 GPU graceful
//! skip）。
//!
//! 三个测试：
//!
//! 1. `table_golden_full_precision_face_thresholds`：8 个 character 实例
//!    经 StorageTable 携带全精度 face 阈值（MeshTag 量化网格外的值），
//!    捕获并断言逐实例亮度单调（索引正确性）；PNG 写入
//!    `target/tmp/instance-db/`（gitignored，不进 golden 基线——表通道
//!    是 provisional 证据，不是冻结外观）。
//! 2. `instance_db_measurement`：100/1000/5000 实例 × 两通道（MeshTag
//!    vs StorageTable）的 draw 数与主线程 update 耗时对比，JSON +
//!    summary 写入 `target/tmp/instance-db/`。
//! 3. `portable_materials_have_zero_storage_bindings`：钉死三个主材质
//!    类型零 storage binding；表变体恰好 1 个（fragment 可见）。

use std::time::Instant;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    camera::{OrthographicProjection, Projection, ScalingMode},
    core_pipeline::{core_3d::Opaque3d, tonemapping::Tonemapping},
    image::Image,
    math::Vec3,
    pbr::MeshMaterial3d,
    prelude::{Camera3d, Handle, Mesh, Mesh3d, Query, Transform},
    render::{
        RenderApp, RenderPlugin,
        render_phase::ViewBinnedRenderPhases,
        render_resource::ShaderStages,
        settings::{RenderCreation, RenderResources},
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::*;
use bevy_stylized::stylized_render::{
    budget, character::StylizedCharacterTableMaterial, instance_table::StylizedInstanceTableEntry,
};

const WIDTH: u32 = 512;
const HEIGHT: u32 = 512;

/// 梯度用例的实例数。
const N: u32 = 8;

fn headless_app(resources: RenderResources) -> (App, golden_harness::capture::CaptureTarget) {
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
    golden_harness::capture::install(&mut app);
    let target = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        golden_harness::capture::create_capture_target(&mut images, WIDTH, HEIGHT)
    };
    app.finish();
    app.cleanup();
    (app, target)
}

fn out_dir() -> std::path::PathBuf {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp/instance-db");
    std::fs::create_dir_all(&dir).expect("create instance-db dir");
    dir
}

// ---------------------------------------------------------------------------
// 1. 全精度 face 阈值 golden（索引正确性）
// ---------------------------------------------------------------------------

/// 与 golden_character 相同的程序化 face SDF（左亮右暗线性渐变）。
fn procedural_face_sdf() -> Image {
    let (w, h) = (256usize, 256usize);
    let mut data = Vec::with_capacity(w * h * 4);
    for _y in 0..h {
        for x in 0..w {
            let v = ((x as f32 / (w - 1) as f32) * 255.0) as u8;
            data.extend_from_slice(&[v, v, v, 255]);
        }
    }
    let mut image = Image::new_fill(
        bevy::render::render_resource::Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        &[0, 0, 0, 255],
        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
        bevy::asset::RenderAssetUsages::default(),
    );
    image.data = Some(data);
    image.sampler = bevy::image::ImageSampler::linear();
    image
}

#[test]
fn table_golden_full_precision_face_thresholds() {
    let Some(resources) = golden_harness::probe_render_resources() else {
        eprintln!("instance-table golden: no GPU adapter available; skipping");
        return;
    };
    // 两通道同场景捕获：StorageTable（全精度）vs MeshTag（量化子集）。
    let (table_image, table_lumas) = capture_threshold_gradient(&resources, true);
    let (tag_image, tag_lumas) = capture_threshold_gradient(&resources, false);
    eprintln!("[instance-table golden] table lumas: {table_lumas:?}");
    eprintln!("[instance-table golden] meshtag lumas: {tag_lumas:?}");

    // 索引正确性：盘内均值严格单调不增 + 至少 5 个可区分档位 +
    // 首末落差足够（可见梯度）。
    for pair in table_lumas.windows(2) {
        assert!(
            pair[0] >= pair[1],
            "阈值递增 ⇒ 受光面积不增：{table_lumas:?}"
        );
    }
    let distinct = {
        let mut levels = table_lumas.clone();
        levels.dedup_by(|a, b| (*a as i32 - *b as i32).abs() < 8);
        levels.len()
    };
    assert!(distinct >= 5, "至少 5 个可区分亮度档位：{table_lumas:?}");
    assert!(
        table_lumas[0] > table_lumas[N as usize - 1] * 3 / 2,
        "首末球应有显著落差：{table_lumas:?}"
    );

    // 两通道在此精度下视觉一致（量化差异微妙，逐字记录）；保真差异由
    // 单测 `record_pack_roundtrip_full_precision` 钉死。
    let max_divergence = table_lumas
        .iter()
        .zip(&tag_lumas)
        .map(|(a, b)| (*a as i32 - *b as i32).unsigned_abs())
        .max()
        .unwrap_or(0);
    eprintln!(
        "[instance-table golden] max per-sphere luma divergence (table vs tag): {max_divergence}"
    );

    let dir = out_dir();
    let png = golden_harness::encode_png(&table_image).expect("encode png");
    std::fs::write(dir.join("table_face_thresholds.png"), png).expect("write png");
    let png = golden_harness::encode_png(&tag_image).expect("encode png");
    std::fs::write(dir.join("meshtag_face_thresholds.png"), png).expect("write png");
}

/// 8 个 face 球（全过渡区阈值梯度）在指定通道下的捕获 + 盘内亮度序列。
fn capture_threshold_gradient(
    resources: &RenderResources,
    use_table: bool,
) -> (golden_harness::CapturedImage, Vec<u32>) {
    let (mut app, target) = headless_app(resources.clone());
    app.insert_resource(bevy::prelude::ClearColor(bevy::prelude::Color::srgb(
        0.105, 0.12, 0.13,
    )));
    *app.world_mut().resource_mut::<StylizedViewSettings>() =
        StylizedViewSettings(StylizedViewDescriptor {
            key_light_direction: Vec3::new(0.95, 0.25, 0.1).normalize(),
            key_light_color: bevy::color::LinearRgba::WHITE,
            exposure: 1.0,
            day_night_factor: 1.0,
            ..StylizedViewDescriptor::default()
        });
    let face_sdf = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(procedural_face_sdf());
    let params = StylizedCharacterParams::default()
        .with_face_forward(Vec3::Z, 0.05)
        .with_face_threshold_bias(0.5)
        .with_palette(
            bevy::color::LinearRgba::WHITE,
            bevy::color::LinearRgba::new(0.25, 0.25, 0.35, 1.0),
            bevy::color::LinearRgba::WHITE,
            bevy::color::LinearRgba::new(0.45, 0.45, 0.55, 1.0),
        );
    let sphere = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(bevy::math::primitives::Sphere::new(0.45));

    // 8 实例：face 阈值覆盖可见盘的响应过渡区（≈0.45..0.94；低于 ~0.5
    // 时正面盘全亮），取值偏离 MeshTag 的 1/254 量化网格（+0.0013）。
    for i in 0..N {
        let threshold = 0.45 + 0.07 * i as f32 + 0.0013;
        let x = (i as f32 - (N - 1) as f32 / 2.0) * 1.05;
        if use_table {
            // 表通道：预建 buffer + 预置资源，材质绑定同一句柄。
            if i == 0 {
                let buffer = app
                    .world_mut()
                    .resource_mut::<Assets<bevy::render::storage::ShaderBuffer>>()
                    .add(bevy::render::storage::ShaderBuffer::with_size(
                        64 * bevy_stylized::stylized_render::INSTANCE_RECORD_STRIDE,
                        bevy::asset::RenderAssetUsages::default(),
                    ));
                app.insert_resource(
                    bevy_stylized::stylized_render::instance_table::StylizedInstanceTable {
                        buffer: buffer.clone(),
                        capacity: 64,
                        next_slot: 0,
                    },
                );
                let material = app
                    .world_mut()
                    .resource_mut::<Assets<StylizedCharacterTableMaterial>>()
                    .add(StylizedCharacterTableMaterial {
                        params,
                        face_sdf_texture: Some(face_sdf.clone()),
                        instance_table: buffer,
                        ..StylizedCharacterTableMaterial::default()
                    });
                app.insert_resource(GoldenTableMaterial(material));
            }
            let material = app.world().resource::<GoldenTableMaterial>().0.clone();
            app.world_mut().spawn((
                Mesh3d(sphere.clone()),
                MeshMaterial3d(material),
                StylizedInstanceData::default(),
                StylizedInstanceTableEntry(StylizedInstanceGpuRecord {
                    face_threshold: threshold,
                    ..StylizedInstanceGpuRecord::default()
                }),
                Transform::from_xyz(x, 0.0, 0.0),
            ));
        } else {
            if i == 0 {
                let material = app
                    .world_mut()
                    .resource_mut::<Assets<StylizedCharacterMaterial>>()
                    .add(StylizedCharacterMaterial {
                        params,
                        face_sdf_texture: Some(face_sdf.clone()),
                        ..StylizedCharacterMaterial::default()
                    });
                app.insert_resource(GoldenTagMaterial(material));
            }
            let material = app.world().resource::<GoldenTagMaterial>().0.clone();
            app.world_mut().spawn((
                Mesh3d(sphere.clone()),
                MeshMaterial3d(material),
                StylizedInstanceData {
                    face_threshold: Some(threshold),
                    ..StylizedInstanceData::default()
                },
                Transform::from_xyz(x, 0.0, 0.0),
            ));
        }
    }
    // 正交相机：屏幕 x 与世界 x 线性（逐球采样定位不依赖透视投影）。
    app.world_mut().spawn((
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedHorizontal {
                viewport_width: 12.0,
            },
            ..OrthographicProjection::default_3d()
        }),
        golden_harness::capture::capture_target_bundle(&target),
        Tonemapping::None,
        Transform::from_xyz(0.0, 0.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    for _ in 0..24 {
        app.update();
    }
    let request = golden_harness::capture::request_capture(app.world_mut(), target);
    let mut captured = None;
    for _ in 0..900 {
        app.update();
        if let Some(frame) = golden_harness::capture::try_take(app.world_mut(), request.clone()) {
            captured = Some(frame);
            break;
        }
    }
    let captured = captured.expect("capture readback timed out");

    // 盘内平均亮度（阈值 ↑ ⇒ 受光面积 ↓ ⇒ 均值 ↓）。
    let mut luminas = Vec::new();
    for i in 0..N {
        let world_x = (i as f32 - (N - 1) as f32 / 2.0) * 1.05;
        let center_px = ((0.5 + world_x / 12.0) * WIDTH as f32) as usize;
        let py = (HEIGHT / 2) as usize;
        let mut sum = 0u32;
        let mut count = 0u32;
        for dy in 0..21usize {
            for dx in 0..21usize {
                let idx = ((py + dy - 10) * WIDTH as usize + (center_px + dx - 10)) * 4;
                sum += captured.data[idx] as u32
                    + captured.data[idx + 1] as u32
                    + captured.data[idx + 2] as u32;
                count += 1;
            }
        }
        luminas.push(sum / count);
    }
    (captured, luminas)
}

#[derive(bevy::prelude::Resource, Clone)]
struct GoldenTableMaterial(Handle<StylizedCharacterTableMaterial>);

#[derive(bevy::prelude::Resource, Clone)]
struct GoldenTagMaterial(Handle<StylizedCharacterMaterial>);

// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// 2. 测量对比（100/1000/5000 × 两通道）
// ---------------------------------------------------------------------------

const SETTLE_FRAMES: u32 = 20;
const TIMED_FRAMES: u32 = 40;

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
struct Measurement {
    transport: &'static str,
    instances: u32,
    opaque_draw_calls: usize,
    avg_update_ms: f64,
    bytes_per_instance: u32,
    richness: &'static str,
}

fn spawn_field<B: bevy::prelude::Bundle>(app: &mut App, n: u32, spawn: impl Fn(u32) -> B) {
    let cols = (n as f32).sqrt().ceil() as u32;
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(bevy::math::primitives::Sphere::new(0.3));
    let world = app.world_mut();
    for i in 0..n {
        let x = (i % cols) as f32 - cols as f32 * 0.5;
        let y = (i / cols) as f32 * 0.5;
        world.spawn((
            Mesh3d(mesh.clone()),
            Transform::from_xyz(x, y, 0.0),
            spawn(i),
        ));
    }
    let half = cols as f32 * 0.5;
    let target = app.world().resource::<MeasureTarget>().0.clone();
    app.world_mut().spawn((
        Camera3d::default(),
        golden_harness::capture::capture_target_bundle(&target),
        Tonemapping::None,
        Transform::from_xyz(0.0, half * 0.25, half * 2.5)
            .looking_at(Vec3::new(0.0, half * 0.25, 0.0), Vec3::Y),
    ));
}

#[derive(bevy::prelude::Resource, Clone)]
struct MeasureTarget(golden_harness::capture::CaptureTarget);

/// 每帧改动全部实例记录（churn 路径测量：sync_instance_table 全量重写
/// + 重上传 vs sync_instance_tags 重新打包）。
fn churn_table(mut entries: Query<&mut StylizedInstanceTableEntry>) {
    for mut entry in &mut entries {
        entry.0.face_threshold = entry.0.face_threshold.fract() + 0.001;
    }
}

fn churn_tags(mut instances: Query<&mut StylizedInstanceData>) {
    for mut data in &mut instances {
        let next = data.face_threshold.unwrap_or(0.5).fract() + 0.001;
        data.face_threshold = Some(next);
    }
}

fn measure_scenario(
    resources: &RenderResources,
    transport: &'static str,
    instances: u32,
    use_table: bool,
    churn: bool,
) -> Measurement {
    let (mut app, target) = headless_app(resources.clone());
    app.insert_resource(MeasureTarget(target));
    let material = if use_table {
        let buffer = app
            .world_mut()
            .resource_mut::<Assets<bevy::render::storage::ShaderBuffer>>()
            .add(bevy::render::storage::ShaderBuffer::with_size(
                instances as usize * bevy_stylized::stylized_render::INSTANCE_RECORD_STRIDE,
                bevy::asset::RenderAssetUsages::default(),
            ));
        app.world_mut()
            .resource_mut::<Assets<StylizedCharacterTableMaterial>>()
            .add(StylizedCharacterTableMaterial {
                instance_table: buffer,
                ..StylizedCharacterTableMaterial::default()
            })
    } else {
        // MeshTag 通道：同参数语义的 StylizedCharacterMaterial。
        return measure_mesh_tag(&mut app, transport, instances, churn);
    };
    if churn {
        app.add_systems(bevy::prelude::Update, churn_table);
    }
    spawn_field(&mut app, instances, move |i| {
        (
            MeshMaterial3d(material.clone()),
            StylizedInstanceData::default(),
            StylizedInstanceTableEntry(StylizedInstanceGpuRecord {
                face_threshold: i as f32 / instances as f32,
                ..StylizedInstanceGpuRecord::default()
            }),
        )
    });
    for _ in 0..SETTLE_FRAMES {
        app.update();
    }
    let draws = count_draws(&mut app);
    let start = Instant::now();
    for _ in 0..TIMED_FRAMES {
        app.update();
    }
    Measurement {
        transport,
        instances,
        opaque_draw_calls: draws,
        avg_update_ms: start.elapsed().as_secs_f64() * 1000.0 / f64::from(TIMED_FRAMES),
        bytes_per_instance: 48,
        richness: "full record (face f32 / outline f32 / palette / quality / light_tint)",
    }
}

fn measure_mesh_tag(
    app: &mut App,
    transport: &'static str,
    instances: u32,
    churn: bool,
) -> Measurement {
    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedCharacterMaterial>>()
        .add(StylizedCharacterMaterial::default());
    if churn {
        app.add_systems(bevy::prelude::Update, churn_tags);
    }
    spawn_field(app, instances, move |i| {
        (
            MeshMaterial3d(material.clone()),
            StylizedInstanceData {
                face_threshold: Some(i as f32 / instances as f32),
                ..StylizedInstanceData::default()
            },
        )
    });
    for _ in 0..SETTLE_FRAMES {
        app.update();
    }
    let draws = count_draws(app);
    let start = Instant::now();
    for _ in 0..TIMED_FRAMES {
        app.update();
    }
    Measurement {
        transport,
        instances,
        opaque_draw_calls: draws,
        avg_update_ms: start.elapsed().as_secs_f64() * 1000.0 / f64::from(TIMED_FRAMES),
        bytes_per_instance: 4, // MeshTag u32
        richness: "coarse subset (face u8 1/254, outline u7, profile u8, flags u8)",
    }
}

#[test]
fn instance_db_measurement() {
    let Some(resources) = golden_harness::probe_render_resources() else {
        eprintln!("instance-db measurement: no GPU adapter available; skipping");
        return;
    };
    let mut measurements = Vec::new();
    for instances in [100_u32, 1000, 5000] {
        // 稳态（无 per-frame 变更）与 churn（每帧全量变更）分开测：
        // 两条通道的 CPU 成本差异只在 churn 路径上可见。
        measurements.push(measure_scenario(
            &resources,
            "mesh_tag_bits",
            instances,
            false,
            false,
        ));
        measurements.push(measure_scenario(
            &resources,
            "mesh_tag_bits_churn",
            instances,
            false,
            true,
        ));
        measurements.push(measure_scenario(
            &resources,
            "storage_table",
            instances,
            true,
            false,
        ));
        measurements.push(measure_scenario(
            &resources,
            "storage_table_churn",
            instances,
            true,
            true,
        ));
    }

    let (gpu, storage_limit) = {
        let (device, info) = (&resources.0, &resources.2);
        (
            format!("{} ({:?})", info.name, info.backend),
            device.limits().max_storage_buffers_per_shader_stage,
        )
    };
    #[derive(serde::Serialize)]
    struct Report {
        provisional: bool,
        iteration: &'static str,
        gpu: String,
        max_storage_buffers_per_shader_stage: u32,
        fidelity_contract: &'static str,
        measurements: Vec<Measurement>,
    }
    let report = Report {
        provisional: true,
        iteration: "V2-e",
        gpu,
        max_storage_buffers_per_shader_stage: storage_limit,
        fidelity_contract: "MeshTag = 粗粒度子集（face_threshold u8/254 步进、outline_scale u7、profile_index u8）；StorageTable = 48B 全精度记录（+palette_index/quality_flags/light_tint）。",
        measurements,
    };
    let dir = out_dir();
    std::fs::write(
        dir.join("instance_db_results.json"),
        serde_json::to_string_pretty(&report).expect("serialize"),
    )
    .expect("write json");

    let mut summary = String::from(
        "# V2-e instance transport 对比（provisional）\n\n\
         | transport | instances | draw calls | avg update ms | bytes/instance |\n\
         |---|---|---|---|---|\n",
    );
    for m in &report.measurements {
        summary.push_str(&format!(
            "| {} | {} | {} | {:.3} | {} |\n",
            m.transport, m.instances, m.opaque_draw_calls, m.avg_update_ms, m.bytes_per_instance
        ));
    }
    summary.push_str(&format!(
        "\nGPU: {}\nstorage buffers/stage: {}\n契约：{}\n",
        report.gpu, report.max_storage_buffers_per_shader_stage, report.fidelity_contract
    ));
    std::fs::write(dir.join("summary.md"), &summary).expect("write summary");
    eprintln!("{summary}");

    // 合批 gate：两通道在全部规模下 draw 数都必须远小于实例数。
    for m in &report.measurements {
        assert!(
            m.opaque_draw_calls * 50 <= m.instances as usize,
            "{} @ {} instances: batching looks broken ({} draws)",
            m.transport,
            m.instances,
            m.opaque_draw_calls,
        );
    }
}

// ---------------------------------------------------------------------------
// 3. Portable 钉死：主材质零 storage；表变体恰好 1
// ---------------------------------------------------------------------------

#[test]
fn portable_materials_have_zero_storage_bindings() {
    let Some(resources) = golden_harness::probe_render_resources() else {
        eprintln!("instance-db budget pin: no GPU adapter available; skipping");
        return;
    };
    let device = &resources.0;
    for (name, usage) in [
        (
            "StylizedCelMaterial",
            budget::count_material_usage::<StylizedCelMaterial>(device, ShaderStages::FRAGMENT),
        ),
        (
            "StylizedCharacterMaterial",
            budget::count_material_usage::<StylizedCharacterMaterial>(
                device,
                ShaderStages::FRAGMENT,
            ),
        ),
        (
            "StylizedHeroMaterial",
            budget::count_material_usage::<StylizedHeroMaterial>(device, ShaderStages::FRAGMENT),
        ),
    ] {
        assert_eq!(
            usage.storage_buffers, 0,
            "{name}：Portable 路径永不触碰 storage buffer"
        );
    }
    let table = budget::count_material_usage::<StylizedCharacterTableMaterial>(
        device,
        ShaderStages::FRAGMENT,
    );
    assert_eq!(
        table.storage_buffers, 1,
        "表变体恰好 1 个 storage binding（instance_table）"
    );
}
