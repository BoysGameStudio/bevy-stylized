//! 合批 gate（It-4 退出条件之一；100/1000 实例 + 可见的逐实例差异）。
//!
//! 同 mesh + 同材质的实例携带不同 `StylizedInstanceData`（ramp 行 /
//! face 阈值），验证：
//!
//! 1. `Opaque3d` binned phase 的 draw 数恒为 1（自动合批未被破坏）；
//! 2. 捕获图里实例颜色确实不同（golden diff-gate）；
//! 3. 共享材质资产零 per-frame 修改（semantics_matrix 里有独立的
//!    架构回归测试）。
//!
//! 无 GPU 时 graceful skip。

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    core_pipeline::core_3d::Opaque3d,
    image::Image,
    math::primitives::Sphere,
    pbr::MeshMaterial3d,
    prelude::{Mesh, Mesh3d, Transform, Vec3},
    render::{
        RenderApp, RenderPlugin,
        render_phase::ViewBinnedRenderPhases,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        settings::{RenderCreation, RenderResources},
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::*;
use golden_harness::{GoldenPaths, GoldenProvenance, Tolerance, capture, diff_rgba8};

const WIDTH: u32 = 512;
const HEIGHT: u32 = 512;

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
    capture::install(&mut app);
    *app.world_mut().resource_mut::<StylizedViewSettings>() =
        StylizedViewSettings(StylizedViewDescriptor {
            key_light_direction: Vec3::new(0.0, 1.0, 0.2).normalize(),
            key_light_color: bevy::color::LinearRgba::WHITE,
            exposure: 1.0,
            day_night_factor: 1.0,
            ..StylizedViewDescriptor::default()
        });
    app.finish();
    app.cleanup();
    app
}

/// 4 行 ramp：每行一种阴影色（行 = profile_index）。
fn ramp_texture_4rows() -> Image {
    let rows: [[u8; 4]; 4] = [
        [80, 80, 220, 255],
        [220, 80, 80, 255],
        [80, 200, 80, 255],
        [220, 200, 80, 255],
    ];
    let mut data = Vec::new();
    for row in rows {
        for col in 0..64 {
            let px = if col >= 32 { [245, 245, 245, 255] } else { row };
            data.extend_from_slice(&px);
        }
    }
    let mut image = Image::new_fill(
        Extent3d {
            width: 64,
            height: 4,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    );
    image.data = Some(data);
    image
}

fn spawn_instances(app: &mut App, n: u32) -> f32 {
    let ramp = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(ramp_texture_4rows());
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Sphere::new(0.35));
    // 一个共享材质句柄（关键：合批前提是同 mesh+material）。
    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial {
            params: StylizedCelParams::default().with_ramp_rows(4.0),
            base_color_texture: None,
            ramp_texture: Some(ramp),
            ilm_texture: None,
            ..StylizedCelMaterial::default()
        });
    let cols = (n as f32).sqrt().ceil() as u32;
    for i in 0..n {
        let x = (i % cols) as f32 - cols as f32 * 0.5;
        let y = (i / cols) as f32 - cols as f32 * 0.25;
        app.world_mut().spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            StylizedInstanceData {
                profile_index: i % 4,
                ..StylizedInstanceData::default()
            },
            Transform::from_xyz(x * 0.9, y * 0.9, 0.0),
        ));
    }
    cols as f32 * 0.45
}

/// 数 Opaque3d binned phase 的 draw 数（spike 同款口径）。
fn count_draws(app: &mut App) -> usize {
    let render_app = app.get_sub_app_mut(RenderApp).expect("render app");
    let Some(phases) = render_app
        .world()
        .get_resource::<ViewBinnedRenderPhases<Opaque3d>>()
    else {
        return 0;
    };
    phases
        .0
        .values()
        .map(|phase| phase.multidrawable_meshes.len() + phase.batchable_meshes.len())
        .sum()
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn instances_batch_with_per_instance_variation() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("instance-batching: no GPU adapter available; skipping");
        return;
    };
    let mut first_capture = None;
    for n in [100u32, 1000] {
        let mut app = headless_app(resources.clone());
        let half = spawn_instances(&mut app, n);
        // CaseDriver 观测周期：Build 泵 + 调度泵 + 22 tick 恰好 24 帧暖场，
        // 第 24 帧时点（与旧流程相同）做 draw 计数内省，随后完成观测。
        app.insert_resource(scene_runtime::CycleResource(
            scene_runtime::CaptureCycle::new(
                scenario_model::ident::SessionId::new("instance-batching").expect("session id"),
                scenes::CameraRecipe::new(Vec3::new(0.0, 0.0, half * 2.6), Vec3::ZERO),
                (WIDTH, HEIGHT),
                false,
                660,
            ),
        ));
        app.world_mut()
            .resource_scope(
                |world, mut cycle: bevy::prelude::Mut<scene_runtime::CycleResource>| {
                    cycle.begin_capture(world, scene_runtime::Settle::Frames(24))
                },
            )
            .expect("capture profile must be a valid plan");
        let mut draws = 0;
        let mut done = false;
        for i in 0..660 {
            let status = app.world_mut().resource_scope(
                |world, mut cycle: bevy::prelude::Mut<scene_runtime::CycleResource>| {
                    cycle.pump(world)
                },
            );
            match status {
                scene_runtime::DriverStatus::Running => {
                    app.update();
                    if i == 23 {
                        draws = count_draws(&mut app);
                    }
                }
                scene_runtime::DriverStatus::Done => {
                    done = true;
                    break;
                }
                scene_runtime::DriverStatus::Failed(error) => panic!("capture timed out: {error}"),
            }
        }
        assert!(done, "capture timed out");
        assert_eq!(
            draws, 1,
            "{n} instances of same mesh+material must stay at 1 draw call"
        );

        if n == 100 {
            // 捕获：ramp 行可见差异（4 色条纹）。
            first_capture = Some(
                app.world_mut()
                    .resource_scope(
                        |_world, mut cycle: bevy::prelude::Mut<scene_runtime::CycleResource>| {
                            cycle.take_receipt()
                        },
                    )
                    .expect("100-instance capture")
                    .image,
            );
        }
    }
    let captured = first_capture.expect("100-instance capture");

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden/cel-portable");
    let paths = GoldenPaths::new(&dir, "instances_batch");
    let mut provenance = GoldenProvenance::for_capture(
        "engine-golden-v1",
        None,
        "synthetic-fixtures",
        None,
        "blender-gltf-npr-v1",
        "bevy_stylized.material/v1",
        "cel-portable/instances_batch",
        "bevy-stylized-render-0.1.0",
        "unknown",
        "unknown",
        &captured,
    );
    if !paths.baseline_exists() {
        assert!(
            std::env::var("GOLDEN_REBASELINE").as_deref() == Ok("1"),
            "accepted baseline missing; capture/accept explicitly"
        );
        golden_harness::write_baseline(&paths, &captured, &mut provenance).expect("write baseline");
        eprintln!("[baselined] cel-portable/instances_batch");
        return;
    }
    let (baseline, _) = golden_harness::read_baseline(&paths).expect("read baseline");
    let stats = diff_rgba8(&baseline.data, &captured.data).expect("same size");
    let report = golden_harness::DiffReport::evaluate(
        "instances_batch".to_owned(),
        paths.png.to_string_lossy().into_owned(),
        stats,
        Tolerance::default(),
    );
    eprintln!(
        "[{}] instances_batch max_delta={} psnr={:.2}dB",
        if report.passed { "matched" } else { "exceeded" },
        report.stats.max_channel_delta,
        report.stats.psnr_db,
    );
    assert!(report.passed, "instances_batch exceeded tolerance");
}
