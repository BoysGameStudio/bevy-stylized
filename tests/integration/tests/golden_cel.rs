//! Cel@Portable 合成场景的 Golden 测试（GPU-gated，无 GPU 时 graceful skip）。
//!
//! 三个用例：解析 ramp 单球、纹理 ramp + MeshTag 行选择三球、
//! hull outline 球。首次运行写 baseline 到
//! `fixtures/golden/cel-portable/`（local-only，gitignored），
//! 之后按 Tolerance 做 diff gate。

use std::path::{Path, PathBuf};

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    color::LinearRgba,
    image::{Image, ImageSampler},
    math::primitives::Sphere,
    mesh::MeshTag,
    pbr::MeshMaterial3d,
    prelude::{Mesh, Mesh3d, Transform, Vec3},
    render::{
        RenderPlugin,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        renderer::RenderAdapterInfo,
        settings::{RenderCreation, RenderResources},
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::*;
use bevy_stylized::stylized_render::{StylizedOutlineParams, StylizedRenderCapabilities};
use golden_harness::{CapturedImage, GoldenProvenance, Tolerance, capture, diff_rgba8};
use scenario_model::ident::SessionId;
use scene_runtime::{Settle, capture_in_app};
use scenes::CameraRecipe;

const WIDTH: u32 = 512;
const HEIGHT: u32 = 512;
const SETTLE_FRAMES: u32 = 12;
const MAX_WAIT_FRAMES: u32 = 600;

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden/cel-portable")
}

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
    app.finish();
    app.cleanup();
    app
}

fn set_key_light(app: &mut App, direction: Vec3, color: LinearRgba) {
    let mut descriptor = StylizedViewDescriptor {
        key_light_direction: direction,
        key_light_color: color,
        exposure: 1.0,
        ..StylizedViewDescriptor::default()
    };
    descriptor.quality.validation_mode = StylizedValidationMode::Production;
    *app.world_mut().resource_mut::<StylizedViewSettings>() = StylizedViewSettings(descriptor);
}

/// 经 CaseDriver 观测周期捕获（settle→observe；帧语义与旧手写
/// "SETTLE_FRAMES 次 update 后请求、下一帧提取"逐帧一致）。
fn capture_frame(app: &mut App, eye: Vec3, look: Vec3) -> CapturedImage {
    capture_in_app(
        app,
        &SessionId::new("golden-cel").expect("session id"),
        &CameraRecipe::new(eye, look),
        (WIDTH, HEIGHT),
        Settle::Frames(SETTLE_FRAMES as u64),
        false,
        MAX_WAIT_FRAMES as usize + 40,
    )
    .expect("capture readback timed out")
}

fn run_case(name: &str, eye: Vec3, look: Vec3, build: impl FnOnce(&mut App)) -> bool {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("golden-cel: no GPU adapter available; skipping");
        return true;
    };
    let mut app = headless_app(resources);
    build(&mut app);
    let captured = capture_frame(&mut app, eye, look);
    let (gpu, driver) = app
        .world()
        .get_resource::<RenderAdapterInfo>()
        .map(|info| {
            (
                format!("{} ({:?})", info.name, info.backend),
                format!("{} {}", info.driver, info.driver_info),
            )
        })
        .unwrap_or_else(|| ("unknown".to_owned(), "unknown".to_owned()));

    // Stage B 断言（ADR-0022）：真实设备上解析必须成功。
    let capabilities = app
        .world()
        .get_resource::<StylizedRenderCapabilities>()
        .expect("facade must insert StylizedRenderCapabilities");
    assert!(
        capabilities.cel_portable_satisfied(),
        "capability resolution failed on a real device: {:?}",
        capabilities.resolve_error
    );

    let backend = app
        .world()
        .get_resource::<RenderAdapterInfo>()
        .map(|info| format!("{:?}", info.backend))
        .unwrap_or_else(|| "unknown".to_owned());
    let platform = golden_harness::current_golden_platform(&backend);
    let paths = golden_harness::resolve_baseline_paths(&golden_dir(), name, &platform);
    let mut provenance = GoldenProvenance::for_capture(
        "engine-golden-v1",
        None,
        "synthetic-fixtures",
        None,
        "blender-gltf-npr-v1",
        "bevy_stylized.material/v1",
        &format!("cel-portable/{name}"),
        "bevy-stylized-render-0.1.0",
        &gpu,
        &driver,
        &captured,
    );
    if !paths.baseline_exists() {
        assert!(
            std::env::var("GOLDEN_REBASELINE").as_deref() == Ok("1"),
            "accepted baseline missing; capture/accept explicitly"
        );
        let target = golden_harness::resolve_capture_paths(&golden_dir(), name, &platform);
        golden_harness::write_baseline(&target, &captured, &mut provenance)
            .expect("write baseline");
        eprintln!("[baselined] {name} -> {}", target.png.display());
        return true;
    }
    let (baseline, _) = golden_harness::read_baseline(&paths).expect("read baseline");
    let stats = diff_rgba8(&baseline.data, &captured.data).expect("same size");
    let report = golden_harness::DiffReport::evaluate(
        name.to_owned(),
        paths.png.to_string_lossy().into_owned(),
        stats,
        Tolerance::default(),
    );
    eprintln!(
        "[{}] {name} max_delta={} mean_delta={:.4} psnr={:.2}dB",
        if report.passed { "matched" } else { "exceeded" },
        report.stats.max_channel_delta,
        report.stats.mean_absolute_delta,
        report.stats.psnr_db,
    );
    if !report.passed
        && std::env::var("GOLDEN_REBASELINE").as_deref() == Ok("1")
        && platform != golden_harness::CANONICAL_GOLDEN_PLATFORM
    {
        // Explicit platform-overlay capture: the canonical baseline stays
        // untouched; the overlay image is a candidate until reviewed and
        // accepted (fixtures/README.md).
        let overlay = golden_harness::resolve_capture_paths(&golden_dir(), name, &platform);
        golden_harness::write_baseline(&overlay, &captured, &mut provenance)
            .expect("write overlay candidate");
        eprintln!("[overlay-candidate] {platform}/{name} (canonical baseline unchanged)");
        return true;
    }
    report.passed
}

fn analytic_ramp_scene(app: &mut App) {
    set_key_light(
        app,
        Vec3::new(0.9, 0.15, 0.15).normalize(),
        LinearRgba::WHITE,
    );
    let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
    let sphere = meshes.add(Sphere::new(1.0));
    let mut materials = app
        .world_mut()
        .resource_mut::<Assets<StylizedCelMaterial>>();
    let material = materials.add(StylizedCelMaterial {
        params: StylizedCelParams::default()
            .with_shadow(0.45, 0.08, 0.25)
            .with_rim(3.0, 0.35, LinearRgba::new(0.6, 0.8, 1.0, 1.0)),
        base_color_texture: None,
        ramp_texture: None,
        ilm_texture: None,
        double_sided: false,
    });
    app.world_mut().spawn((
        Mesh3d(sphere),
        MeshMaterial3d(material),
        Transform::IDENTITY,
    ));
    // 同步资源（无启动系统场景）：params.flags 已由 sync_cel_material_flags 处理。
}

/// 3 行 × 256 列 ramp 纹理（行色不同，验证 MeshTag 行选择）。
fn make_ramp_texture() -> Image {
    let rows: [[u8; 4]; 3] = [
        [64, 64, 255, 255], // row 0: 蓝阴影
        [255, 64, 64, 255], // row 1: 红阴影
        [64, 255, 64, 255], // row 2: 绿阴影
    ];
    let mut data = Vec::new();
    for row in rows {
        for col in 0..256 {
            let lit = if col >= 128 {
                [255, 255, 255, 255]
            } else {
                row
            };
            data.extend_from_slice(&lit);
        }
    }
    let mut image = Image::new_fill(
        Extent3d {
            width: 256,
            height: 3,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    );
    image.data = Some(data);
    image.sampler = ImageSampler::linear();
    image
}

fn textured_ramp_scene(app: &mut App) {
    set_key_light(app, Vec3::new(0.0, 1.0, 0.0), LinearRgba::WHITE);
    let ramp = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(make_ramp_texture());
    let sphere = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Sphere::new(0.9));
    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial {
            params: StylizedCelParams::default().with_ramp_rows(3.0),
            ramp_texture: Some(ramp),
            ..StylizedCelMaterial::default()
        });
    for (index, x) in [-2.2, 0.0, 2.2].into_iter().enumerate() {
        app.world_mut().spawn((
            Mesh3d(sphere.clone()),
            MeshMaterial3d(material.clone()),
            MeshTag(index as u32),
            Transform::from_xyz(x, 0.0, 0.0),
        ));
    }
}

fn outline_scene(app: &mut App) {
    set_key_light(app, Vec3::new(0.5, 0.8, 0.6).normalize(), LinearRgba::WHITE);
    let sphere = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Sphere::new(1.0));
    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial::default());
    let outline = app
        .world_mut()
        .resource_mut::<Assets<StylizedOutlineMaterial>>()
        .add(StylizedOutlineMaterial {
            params: StylizedOutlineParams::default()
                .with_color(LinearRgba::new(0.01, 0.01, 0.02, 1.0))
                .with_width(6.0),
            ..Default::default()
        });
    app.world_mut().spawn((
        Mesh3d(sphere),
        MeshMaterial3d(material),
        StylizedOutline::new(outline),
        Transform::IDENTITY,
    ));
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn cel_portable_golden_cases() {
    let cases: [(&str, Vec3, Vec3, fn(&mut App)); 3] = [
        (
            "sphere_analytic_ramp",
            Vec3::new(0.0, 0.4, 3.2),
            Vec3::ZERO,
            analytic_ramp_scene,
        ),
        (
            "sphere_textured_ramp",
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::ZERO,
            textured_ramp_scene,
        ),
        (
            "sphere_outline",
            Vec3::new(0.0, 0.4, 3.2),
            Vec3::ZERO,
            outline_scene,
        ),
    ];
    let mut failures = Vec::new();
    for (name, eye, look, build) in cases {
        if !run_case(name, eye, look, build) {
            failures.push(name);
        }
    }
    assert!(failures.is_empty(), "golden cases exceeded: {failures:?}");
}
