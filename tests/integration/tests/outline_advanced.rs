//! Outline-advanced golden（GPU-gated）：hull-only / screen-only / hybrid。
//!
//! 场景：叠放方块（内部接触褶皱）+ 球。hull 只能画剪影；screen 描边应
//! 在 screen-only 捕获中抓到方块接触边与球-方块交线。
//!
//! 捕获到 `fixtures/golden/outline-advanced/`（gitignored）。

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    color::LinearRgba,
    math::{Vec3, primitives::Cuboid, primitives::Sphere},
    mesh::Mesh,
    pbr::MeshMaterial3d,
    prelude::{Mesh3d, Transform},
    render::{
        RenderPlugin,
        settings::{RenderCreation, RenderResources},
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::*;
use bevy_stylized::stylized_render::screen_outline::StylizedScreenOutlineOverlay;
use golden_harness::{
    CapturedImage, GoldenPaths, GoldenProvenance, Tolerance, capture, diff_rgba8,
};

const SIZE: u32 = 512;

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
    // Screen-space 描边是 Standard+ 特性：请求 Standard 档（5090 满足）。
    app.insert_resource(StylizedCapabilityConfig {
        requested_tier: StylizedExecutionTier::Standard,
        ..StylizedCapabilityConfig::default()
    });
    app.add_plugins(StylizedPlugin::default());
    capture::install(&mut app);
    golden_harness::readiness::install(&mut app);
    *app.world_mut().resource_mut::<StylizedViewSettings>() =
        StylizedViewSettings(StylizedViewDescriptor {
            key_light_direction: Vec3::new(0.4, 0.8, 0.5).normalize(),
            key_light_color: LinearRgba::WHITE,
            exposure: 1.0,
            day_night_factor: 1.0,
            ..StylizedViewDescriptor::default()
        });
    app.finish();
    app.cleanup();
    app
}

/// 场景：两个叠放方块 + 一个球（接触褶皱 + 剪影）。
fn build_scene(app: &mut App, hull: bool) {
    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial::default());
    let outline = app
        .world_mut()
        .resource_mut::<Assets<StylizedOutlineMaterial>>()
        .add(StylizedOutlineMaterial {
            params: StylizedOutlineParams::default()
                .with_color(LinearRgba::new(0.02, 0.02, 0.04, 1.0))
                .with_width(3.0),
            ..Default::default()
        });
    let big_box = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Cuboid::from_size(Vec3::new(2.0, 0.8, 1.6)));
    let small_box = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Cuboid::from_size(Vec3::new(0.9, 0.9, 0.9)));
    let ball = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Sphere::new(0.55));
    for (mesh, translation) in [
        (big_box, Vec3::new(0.0, -0.4, 0.0)),
        (small_box.clone(), Vec3::new(0.0, 0.45, 0.0)),
        (small_box, Vec3::new(0.35, 0.45, 0.2)),
        (ball, Vec3::new(0.7, 0.55, 0.1)),
    ] {
        let mut entity = app.world_mut().spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(translation),
        ));
        if hull {
            entity.insert(StylizedOutline::new(outline.clone()));
        }
    }
}

/// 经 CaseDriver 观测周期捕获（readiness 就绪 + 24 帧暖场后观测；
/// 与旧 `readiness::wait(24)` 后请求逐帧一致）。screen 变体的
/// screen-space 附件经相机配方声明（内容级 per-camera opt-in）。
/// 返回（图像, 第 3 帧时点的 overlay 数）——overlay 断言留在测试。
fn capture_frame(app: &mut App, screen: bool) -> (CapturedImage, usize) {
    if !app
        .world()
        .contains_resource::<scene_runtime::CycleResource>()
    {
        let attachment = if screen {
            scenes::CameraAttachment::ScreenOutline
        } else {
            scenes::CameraAttachment::None
        };
        app.insert_resource(scene_runtime::CycleResource(
            scene_runtime::CaptureCycle::new(
                scenario_model::ident::SessionId::new("outline-advanced").expect("session id"),
                scenes::CameraRecipe::with_attachment(
                    Vec3::new(2.2, 1.6, 3.4),
                    Vec3::new(0.0, 0.3, 0.0),
                    attachment,
                ),
                (SIZE, SIZE),
                true,
                660,
            ),
        ));
    }
    app.world_mut()
        .resource_scope(
            |world, mut cycle: bevy::prelude::Mut<scene_runtime::CycleResource>| {
                cycle.begin_capture(world, scene_runtime::Settle::ReadinessThenFrames(24))
            },
        )
        .expect("capture profile must be a valid plan");
    let mut overlays = 0;
    let mut done = false;
    for i in 0..660 {
        let status = app.world_mut().resource_scope(
            |world, mut cycle: bevy::prelude::Mut<scene_runtime::CycleResource>| cycle.pump(world),
        );
        match status {
            scene_runtime::DriverStatus::Running => {
                app.update();
                if i == 2 {
                    // 与旧流程同一点：相机生成后的第 3 次 update 结束。
                    overlays = overlay_count(app);
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
    let receipt = app
        .world_mut()
        .resource_scope(
            |_world, mut cycle: bevy::prelude::Mut<scene_runtime::CycleResource>| {
                cycle.take_receipt()
            },
        )
        .expect("capture must produce an observation");
    (receipt.image, overlays)
}

fn overlay_count(app: &mut App) -> usize {
    let mut query = app.world_mut().query::<&StylizedScreenOutlineOverlay>();
    query.iter(app.world()).count()
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn outline_advanced_hull_screen_hybrid() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("outline-advanced: no GPU adapter; skipping");
        return;
    };
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden/outline-advanced");
    let mut captures = Vec::new();
    for (name, hull, screen) in [
        ("hull_only", true, false),
        ("screen_only", false, true),
        ("hybrid", true, true),
    ] {
        let mut app = headless_app(resources.clone());
        build_scene(&mut app, hull);
        let (frame, overlays) = capture_frame(&mut app, screen);
        // screen 开启时必须有 overlay；关闭时必须没有。
        assert_eq!(overlays > 0, screen, "{name}: overlay presence");
        let diagnostics = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/stylized/outline-advanced")
            .join(std::process::id().to_string());
        std::fs::create_dir_all(&diagnostics).unwrap();
        std::fs::write(
            diagnostics.join(format!("{name}.png")),
            golden_harness::encode_png(&frame).unwrap(),
        )
        .unwrap();
        eprintln!("OUTLINE_CAPTURE {}", diagnostics.display());
        captures.push((name, frame));
    }

    // 语义断言（在 baseline/diff 之前）：screen-only 必须抓到内部边缘。
    // 只在物体区域窗口内数暗边像素（排除暗色背景的污染）。
    let edge_count = |image: &CapturedImage| {
        let (x0, y0, x1, y1) = (100usize, 80usize, 420usize, 420usize);
        let mut count = 0;
        for y in y0..y1 {
            for x in x0..x1 {
                let o = (y * image.width as usize + x) * 4;
                let px = &image.data[o..o + 4];
                // 边缘像素 = 暗色且位于前景（邻近有亮像素）。
                if px[0] < 60 && px[1] < 60 && px[2] < 70 {
                    let right = &image.data[o + 4..o + 8];
                    if right[0] > 150 {
                        count += 1;
                    }
                }
            }
        }
        count
    };
    let hull_edges = edge_count(&captures[0].1);
    let screen_edges = edge_count(&captures[1].1);
    let hybrid_edges = edge_count(&captures[2].1);
    eprintln!("edge pixels: hull={hull_edges} screen={screen_edges} hybrid={hybrid_edges}");
    assert!(
        screen_edges > hull_edges,
        "screen-only must catch MORE interior edges than hull-only          (hull={hull_edges} screen={screen_edges})"
    );
    assert!(
        hybrid_edges >= screen_edges.max(hull_edges),
        "hybrid must be the union"
    );

    // baseline / diff
    for (name, frame) in captures {
        let paths = GoldenPaths::new(&dir, name);
        let mut provenance = GoldenProvenance::for_capture(
            "engine-golden-v1",
            None,
            "synthetic-fixtures",
            None,
            "blender-gltf-npr-v1",
            "bevy_stylized.material/v1",
            &format!("outline-advanced/{name}"),
            "bevy-stylized-render-0.1.0",
            "local",
            "local",
            &frame,
        );
        if !paths.baseline_exists() {
            assert!(
                std::env::var("GOLDEN_REBASELINE").as_deref() == Ok("1"),
                "accepted baseline missing; capture/accept explicitly"
            );
            golden_harness::write_baseline(&paths, &frame, &mut provenance)
                .expect("write baseline");
            eprintln!("[baselined] outline-advanced/{name}");
            continue;
        }
        let (baseline, _) = golden_harness::read_baseline(&paths).expect("read baseline");
        let stats = diff_rgba8(&baseline.data, &frame.data).expect("same size");
        let report = golden_harness::DiffReport::evaluate(
            name.to_owned(),
            paths.png.to_string_lossy().into_owned(),
            stats,
            Tolerance::default(),
        );
        assert!(
            report.passed,
            "outline-advanced/{name} exceeded: {:?}",
            report.violations
        );
    }
}
