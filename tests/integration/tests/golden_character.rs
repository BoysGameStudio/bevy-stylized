//! First-party Character golden: face SDF instances and emission/hair semantics.

use std::path::{Path, PathBuf};

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets, RenderAssetUsages},
    color::{Color, LinearRgba},
    core_pipeline::tonemapping::Tonemapping,
    image::{Image, ImageSampler},
    math::Vec3,
    pbr::MeshMaterial3d,
    prelude::{Camera3d, Mesh, Mesh3d, Transform},
    render::{
        RenderPlugin,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        renderer::RenderAdapterInfo,
        settings::{RenderCreation, RenderResources},
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::*;
use golden_harness::{
    CapturedImage, GoldenPaths, GoldenProvenance, Tolerance, capture, diff_rgba8,
};

const WIDTH: u32 = 512;
const HEIGHT: u32 = 512;
const SETTLE_FRAMES: u32 = 24;
const MAX_WAIT_FRAMES: u32 = 900;

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden/character-portable")
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
            key_light_direction: Vec3::new(0.95, 0.25, 0.1).normalize(),
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
        assert!(
            std::env::var("GOLDEN_REBASELINE").as_deref() == Ok("1"),
            "accepted baseline missing; capture/accept explicitly"
        );
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

/// ILM（上游布局，docs/reference/07 §3.2）：R=metal(0)，G=AO（上亮下半
/// 暗条纹），B=高光抑制(0)，A=材质分带(0=band1)。emission 演示经 base
/// alpha（见 emission_mask 注记）。
fn procedural_ilm() -> Image {
    let (w, h) = (64usize, 64usize);
    let mut data = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for _x in 0..w {
            let ao = if y < h / 2 { 255u8 } else { 128 };
            data.extend_from_slice(&[0, ao, 0, 0]);
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
/// base 纹理：灰底 + 中心 alpha 块（emission mask，docs/character.rs 的
/// emission 通道约定）。
fn procedural_base_with_emission_mask() -> Image {
    let (w, h) = (64usize, 64usize);
    let mut data = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            // 小球 UV 把纹理中心块铺满可见半球——用小块才能看出是"块"。
            let emission = if x > w * 9 / 20 && x < w * 11 / 20 && y > h * 9 / 20 && y < h * 11 / 20
            {
                255u8
            } else {
                0
            };
            data.extend_from_slice(&[200, 200, 200, emission]);
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
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.data = Some(data);
    image
}

fn synthetic_character_scene(app: &mut App) {
    let face_sdf = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(procedural_face_sdf());
    let ilm = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(procedural_ilm());
    let base = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(procedural_base_with_emission_mask());
    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedCharacterMaterial>>()
        .add(StylizedCharacterMaterial {
            params: StylizedCharacterParams::default()
                .with_face_forward(Vec3::Z, 0.05)
                .with_face_threshold_bias(0.5)
                .with_palette(
                    LinearRgba::WHITE,
                    LinearRgba::new(0.25, 0.25, 0.35, 1.0),
                    LinearRgba::WHITE,
                    LinearRgba::new(0.45, 0.45, 0.55, 1.0),
                )
                .with_hair_specular(24.0, 0.6)
                .with_emission(LinearRgba::new(1.0, 0.6, 0.2, 1.0), 2.0),
            base_color_texture: Some(base),
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
#[ignore = "requires GPU; run the explicit GPU lane"]
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
