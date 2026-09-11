//! Odette GLB × StylizedCelMaterial 的 Golden 测试（TENACITY_ROOT 环境变量
//! 与 GPU 双重 gate；缺失即 graceful skip）。
//!
//! Cel class 只消费嵌入纹理（base color），不需要 NPR sidecar。捕获
//! `full_body` / `face_closeup` 到 `fixtures/golden-local/odette-cel/`；
//! 若 legacy engine golden（`fixtures/golden-local/odette/`）存在，额外产出
//! 一份 `*.vs_legacy.diff.json` **报告**（不同 shading model，不是硬门禁，
//! provisional）。

use std::path::{Path, PathBuf};

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    camera::{primitives::Aabb, visibility::Visibility},
    color::{Color, LinearRgba},
    core_pipeline::tonemapping::Tonemapping,
    gltf::GltfAssetLabel,
    image::Image,
    math::{Quat, Vec3},
    pbr::{MeshMaterial3d, StandardMaterial},
    prelude::{
        Camera3d, Entity, GlobalTransform, Name, Query, Res, ResMut, Resource, Transform, With,
        Without,
    },
    render::{
        RenderPlugin,
        renderer::RenderAdapterInfo,
        settings::{RenderCreation, RenderResources},
    },
    window::{ExitCondition, WindowPlugin},
    world_serialization::WorldAssetRoot,
};
use bevy_stylized::prelude::*;
use bevy_stylized::stylized_render::StylizedOutlineParams;
use golden_harness::{
    CapturedImage, GoldenPaths, GoldenProvenance, Tolerance, capture, diff_rgba8,
};

/// Tenacity checkout 位置（与 odette-reference-lab 同约定）。
const TENACITY_ROOT_ENV: &str = "TENACITY_ROOT";
const DEFAULT_TENACITY_ROOT: &str = "/home/x/Projects/Tenacity";
const ODETTE_GLB: &str = "models/odette.glb";
const ODETTE_HEAD_BONE: &str = "DEF-spine.006";

const WIDTH: u32 = 512;
const HEIGHT: u32 = 512;
const SETTLE_FRAMES: u32 = 24;
const MAX_WAIT_FRAMES: u32 = 900;

// Tenacity view-ink Odette 标定（models.rs；仅为取景/摆放常数，不含任何
// profile 着色数值）。
const ODETTE_SCALE: f32 = 31.5;
const ODETTE_FEET_OFFSET_Y: f32 = 24.5;
const ODETTE_FORWARD_YAW_RADIANS: f32 = core::f32::consts::PI;

fn tenacity_asset_root() -> PathBuf {
    std::env::var(TENACITY_ROOT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_TENACITY_ROOT))
        .join("crates/client-bevy/assets")
}

fn assets_available() -> bool {
    tenacity_asset_root().join(ODETTE_GLB).is_file()
}

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden-local/odette-cel")
}

fn legacy_golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden-local/odette")
}

#[derive(Resource, Default)]
struct AdoptState {
    done: bool,
}

fn headless_app(resources: RenderResources) -> (App, capture::CaptureTarget) {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: tenacity_asset_root().to_string_lossy().into_owned(),
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
    app.insert_resource(bevy::prelude::ClearColor(Color::srgb(0.105, 0.12, 0.13)))
        .init_resource::<AdoptState>();
    app.finish();
    app.cleanup();

    // View 层：key light（Tenacity showcase 的 Day 方向）+ 曝光 1.0。
    *app.world_mut().resource_mut::<StylizedViewSettings>() =
        StylizedViewSettings(StylizedViewDescriptor {
            key_light_direction: Vec3::new(0.422_618_33, 0.906_307_8, 0.0),
            key_light_color: LinearRgba::WHITE,
            exposure: 1.0,
            ..StylizedViewDescriptor::default()
        });
    (app, target)
}

fn adopt_odette(
    mut commands: bevy::prelude::Commands,
    mut state: ResMut<AdoptState>,
    standard: Res<Assets<StandardMaterial>>,
    mut cel: ResMut<Assets<StylizedCelMaterial>>,
    mut outlines: ResMut<Assets<StylizedOutlineMaterial>>,
    meshes: Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&bevy::gltf::GltfMaterialName>,
    )>,
) {
    if state.done || meshes.is_empty() {
        return;
    }
    // 等所有引用到的 StandardMaterial 就绪。
    let ready = meshes
        .iter()
        .all(|(_, handle, _)| standard.contains(&handle.0));
    if !ready {
        return;
    }
    let outline = outlines.add(StylizedOutlineMaterial {
        params: StylizedOutlineParams::default()
            .with_color(LinearRgba::new(0.03, 0.035, 0.05, 1.0))
            .with_width(2.0),
    });
    for (entity, handle, _material_name) in &meshes {
        let source = standard.get(&handle.0).expect("checked above");
        let factor = LinearRgba::from(source.base_color);
        let params = StylizedCelParams {
            base_color_factor: bevy::math::Vec4::new(factor.red, factor.green, factor.blue, 1.0),
            ..StylizedCelParams::default()
        }
        .with_shadow(0.5, 0.12, 0.35);
        // Gacha 资产的 diffuse alpha 是控制数据而非透明度（与 legacy 一致
        // 强制 Opaque）；发卡等半透明语义属 Character class（It-4），
        // alpha 语义矩阵 pending ADR-0023。材质本身支持
        // `with_alpha_cutoff`，此处不启用。
        let material = cel.add(StylizedCelMaterial {
            params,
            double_sided: false,
            base_color_texture: source.base_color_texture.clone(),
            ramp_texture: None,
            ilm_texture: None,
        });
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert(MeshMaterial3d(material))
            .insert(StylizedOutline::new(outline.clone()));
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

fn head_position(app: &mut App) -> Vec3 {
    let mut query = app
        .world_mut()
        .query_filtered::<(&Name, &GlobalTransform), Without<Camera3d>>();
    query
        .iter(app.world())
        .find(|(name, _)| name.as_str() == ODETTE_HEAD_BONE)
        .map(|(_, transform)| transform.translation())
        .expect("Odette GLB must expose its DEF-spine.006 head bone")
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
            let head = head_position(app);
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

/// baseline + （可选的）legacy 对比报告。legacy 对比是 REPORT，不是门禁。
fn evaluate_case(
    name: &str,
    captured: &CapturedImage,
    gpu: &str,
    driver: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let paths = GoldenPaths::new(&golden_dir(), name);
    let mut provenance = GoldenProvenance::for_capture(
        "engine-golden-v1",
        None,
        "tenacity-odette-embedded",
        None,
        "blender-gltf-npr-v1",
        "bevy_stylized.material/v1",
        &format!("odette-cel/{name}"),
        "bevy-stylized-render-0.1.0",
        gpu,
        driver,
        captured,
    );
    if !paths.baseline_exists() {
        golden_harness::write_baseline(&paths, captured, &mut provenance)?;
        eprintln!("[baselined] odette-cel/{name}");
        return Ok(());
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
        "[{}] odette-cel/{name} max_delta={} mean_delta={:.4} psnr={:.2}dB",
        if report.passed { "matched" } else { "exceeded" },
        report.stats.max_channel_delta,
        report.stats.mean_absolute_delta,
        report.stats.psnr_db,
    );
    if !report.passed {
        return Err(format!(
            "odette-cel/{name} exceeded tolerance: {:?}",
            report.violations
        )
        .into());
    }

    // legacy engine golden 对比（不同 shading model ⇒ 报告而非门禁，provisional）。
    let legacy_paths = GoldenPaths::new(&legacy_golden_dir(), name);
    if legacy_paths.png.is_file() {
        let legacy_pixels = golden_harness::decode_png(&std::fs::read(&legacy_paths.png)?)?;
        let stats = diff_rgba8(&legacy_pixels.data, &captured.data)?;
        let report = golden_harness::DiffReport::evaluate(
            format!("{name}.vs_legacy"),
            legacy_paths.png.to_string_lossy().into_owned(),
            stats,
            Tolerance::default(),
        );
        let report_path = golden_dir().join(format!("{name}.vs_legacy.diff.json"));
        std::fs::create_dir_all(golden_dir())?;
        std::fs::write(&report_path, serde_json::to_string_pretty(&report)?)?;
        eprintln!(
            "[report] odette-cel/{name} vs legacy: mean_delta={:.4} psnr={:.2}dB differing={}/{} → {}",
            report.stats.mean_absolute_delta,
            report.stats.psnr_db,
            report.stats.differing_pixels,
            report.stats.total_pixels,
            report_path.display(),
        );
    }
    Ok(())
}

#[test]
fn odette_cel_portable_golden() {
    if !assets_available() {
        eprintln!(
            "odette-cel: {} not found (set {TENACITY_ROOT_ENV}); skipping",
            tenacity_asset_root().join(ODETTE_GLB).display()
        );
        return;
    }
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("odette-cel: no GPU adapter available; skipping");
        return;
    };
    let (mut app, target) = headless_app(resources);

    // 加载场景（只依赖嵌入纹理）。
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
    app.add_systems(bevy::prelude::Update, adopt_odette);

    // 等待场景 adopt 完成。
    let mut waited = 0;
    loop {
        app.update();
        waited += 1;
        if app.world().resource::<AdoptState>().done {
            break;
        }
        if waited > MAX_WAIT_FRAMES {
            panic!("Odette scene adoption timed out");
        }
    }
    // 等轮廓副本 + 纹理 + pipeline 就绪。
    for _ in 0..SETTLE_FRAMES {
        app.update();
    }

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

    for case in ["full_body", "face_closeup"] {
        set_camera(&mut app, case);
        for _ in 0..SETTLE_FRAMES {
            app.update();
        }
        let captured = capture_frame(&mut app, &target);
        evaluate_case(case, &captured, &gpu, &driver).expect("golden evaluation");
    }
}
