//! 三层 Golden 对比（It-5）：同一 fixture（`fixtures/reference/
//! reference_scene.glb`，自制、可入库）经三条路径渲染后按
//! docs/reference/08 §3 的 Tier-2 草案判据对比：
//!
//! - (a) production：`bevy-stylized` 的 StylizedHeroMaterial；
//! - (b) engine reference：legacy `bevy_cel` CharacterMaterial（neutral
//!   ramp 路径；**dev-only 依赖，production crate 永不依赖 legacy**）；
//! - (c) upstream golden：Blender EEVEE 预捕获
//!   （`fixtures/golden-local/upstream/gacha_upstream.png`）。
//!
//! 归一化（doc-08 §2）：512×512、Tonemapping::None、固定相机/灯光（常量
//! 来自 tools/blender-reference-export/make_reference_scene.py，坐标经
//! Blender Z-up → glTF Y-up 转换）、同一 GLB。两侧描边均关闭（边缘排除带
//! 之外避免 hull 语义差异）。
//!
//! 全部为 **REPORT**（provisional）：输出 JSON 报告与 PNG 捕获到
//! `fixtures/golden-local/reference-scene/`，不硬门禁——未过判据时如实
//! 记录，不放宽容差。

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    color::{Color, LinearRgba},
    core_pipeline::tonemapping::Tonemapping,
    gltf::GltfAssetLabel,
    image::Image,
    light::DirectionalLight,
    pbr::{MeshMaterial3d, StandardMaterial},
    prelude::{
        Camera3d, PerspectiveProjection, Projection, Query, Res, ResMut, Resource, Transform, Vec3,
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
use golden_harness::{CapturedImage, Tier2Report, Tier2Thresholds, capture, evaluate_tier2};

/// 视口尺寸（与 upstream golden 一致，禁止重采样）。
pub const SIZE: u32 = 512;

// ---------------------------------------------------------------------------
// fixture 锁定常数（make_reference_scene.py；Blender Z-up → glTF/Bevy Y-up
// 转换：(x, y, z)_blender → (x, z, -y)）
// ---------------------------------------------------------------------------

/// 相机位置（Blender (0, -4.2, 1.1) → Bevy）。
pub const CAMERA_POSITION: Vec3 = Vec3::new(0.0, 1.1, 4.2);
/// 相机目标（Blender (0, 0, 0.9) → Bevy）。
pub const CAMERA_TARGET: Vec3 = Vec3::new(0.0, 0.9, 0.0);
/// 50mm 镜头（36mm 传感器，方形画幅垂直 FOV）。
pub const CAMERA_FOV_RADIANS: f32 = 0.691_121_16; // 2*atan(18/50)
/// 太阳方向（指向光源）：Blender 光源位于 (2,-3,4) 指向原点 →
/// direction_to_light = normalize((2,4,3))。
pub const SUN_DIRECTION_TO_LIGHT: Vec3 = Vec3::new(0.371_390_7, 0.742_781_4, 0.557_086);
/// 太阳色 × 能量 × 1/π（(1.0, 0.956, 0.898) × 3.0 / π，线性）。
///
/// **对比域归一化（只在本 crate 生效，生产渲染器不变）**：Blender
/// "Standard" view transform 是纯 sRGB 传递（无 roll-off），EEVEE 的漫反射
/// 遵循 Lambert `1/π` 能量归一；我们的 NPR 路径的 half-lambert 无该项。
/// 跨层归一化（docs/reference/08 §2）因此把 key light 能量乘 1/π。
pub const SUN_LINEAR: LinearRgba = LinearRgba::new(0.95493, 0.91308, 0.85758, 1.0);
/// 环境光（Blender world (0.03, 0.03, 0.045)，sRGB → 线性）。
pub const WORLD_LINEAR: LinearRgba = LinearRgba::new(0.0028, 0.0028, 0.0052, 1.0);
/// 背景色（world 色经 sRGB 传递后的实测值；用于 Tier-2 剪影遮罩）。
pub const BACKGROUND_SRGB: [u8; 3] = [48, 48, 60];
/// 清屏色（= Blender world 线性色；对齐背景，见 docs/reference/08 §2）。
pub const CLEAR_LINEAR: LinearRgba = LinearRgba::new(0.03, 0.03, 0.045, 1.0);

const GLB: &str = "reference_scene.glb";
const SETTLE_FRAMES: u32 = 24;
const MAX_WAIT_FRAMES: u32 = 900;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/reference")
}

/// 输出目录（local-only）。
pub fn output_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/golden-local/reference-scene")
}

fn upstream_golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/golden-local/upstream/gacha_upstream.png")
}

#[derive(Resource, Clone)]
struct TargetHolder(capture::CaptureTarget);

fn headless_app(resources: RenderResources) -> (App, capture::CaptureTarget) {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: fixture_dir().to_string_lossy().into_owned(),
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
    capture::install(&mut app);
    let target = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        capture::create_capture_target(&mut images, SIZE, SIZE)
    };
    app.insert_resource(TargetHolder(target.clone()));
    // 背景对齐（docs/reference/08 §2）：清屏色 = Blender world 色。
    app.insert_resource(bevy::prelude::ClearColor(Color::LinearRgba(CLEAR_LINEAR)));
    (app, target)
}

/// 固定相机/灯光的场景装载（两路径共用）。
fn spawn_fixture_scene(app: &mut App) {
    let target = app.world().resource::<TargetHolder>().0.clone();
    let scene = app
        .world()
        .resource::<bevy::asset::AssetServer>()
        .load(GltfAssetLabel::Scene(0).from_asset(GLB));
    app.world_mut().spawn(WorldAssetRoot(scene));
    app.world_mut().spawn((
        Camera3d::default(),
        capture::capture_target_bundle(&target),
        Tonemapping::None,
        Projection::Perspective(PerspectiveProjection {
            fov: CAMERA_FOV_RADIANS,
            ..PerspectiveProjection::default()
        }),
        Transform::from_translation(CAMERA_POSITION).looking_at(CAMERA_TARGET, Vec3::Y),
    ));
}

/// 等场景所有 mesh 实体的 StandardMaterial 就绪。
#[derive(Resource, Default)]
struct SceneReady(bool);

fn mark_ready(
    mut ready: ResMut<SceneReady>,
    standard: Res<Assets<StandardMaterial>>,
    meshes: Query<&MeshMaterial3d<StandardMaterial>>,
) {
    if ready.0 {
        return;
    }
    if !meshes.is_empty() && meshes.iter().all(|h| standard.contains(&h.0)) {
        ready.0 = true;
    }
}

fn wait_ready(app: &mut App) {
    for _ in 0..MAX_WAIT_FRAMES {
        app.update();
        if app.world().resource::<SceneReady>().0 {
            return;
        }
    }
    panic!("reference scene did not become ready");
}

fn capture_frame(app: &mut App, target: &capture::CaptureTarget) -> CapturedImage {
    for _ in 0..SETTLE_FRAMES {
        app.update();
    }
    let request = capture::request_capture(app.world_mut(), target.clone());
    for _ in 0..MAX_WAIT_FRAMES {
        app.update();
        if let Some(frame) = capture::try_take(app.world_mut(), request.clone()) {
            return frame;
        }
    }
    panic!("capture readback timed out");
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

// ---------------------------------------------------------------------------
// (a) production：StylizedHeroMaterial
// ---------------------------------------------------------------------------

fn adopt_production(
    mut commands: bevy::prelude::Commands,
    ready: Res<SceneReady>,
    standard: Res<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<StylizedHeroMaterial>>,
    meshes: Query<(bevy::prelude::Entity, &MeshMaterial3d<StandardMaterial>)>,
    mut done: bevy::prelude::Local<bool>,
) {
    if !ready.0 || *done {
        return;
    }
    for (entity, handle) in &meshes {
        let source = standard.get(&handle.0).expect("ready");
        let factor = LinearRgba::from(source.base_color);
        let mut params = StylizedHeroParams::default();
        params.character.cel.base_color_factor =
            bevy::math::Vec4::new(factor.red, factor.green, factor.blue, 1.0);
        params.character.cel = params.character.cel.with_shadow(0.5, 0.1, 0.3);
        params.character.cel =
            params
                .character
                .cel
                .with_rim(3.0, 0.2, LinearRgba::new(0.5, 0.6, 0.8, 1.0));
        let material = materials.add(StylizedHeroMaterial {
            params,
            base_color_texture: source.base_color_texture.clone(),
            // 发卡面片等 double-sided 资产（ADR-0023 矩阵的 cull 维度）。
            double_sided: source.double_sided,
            ..StylizedHeroMaterial::default()
        });
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert(MeshMaterial3d(material));
    }
    *done = true;
}

/// (a) production 路径捕获。
pub fn capture_production(resources: RenderResources) -> CapturedImage {
    let (mut app, target) = headless_app(resources);
    app.add_plugins(StylizedPlugin::default());
    *app.world_mut().resource_mut::<StylizedViewSettings>() =
        StylizedViewSettings(StylizedViewDescriptor {
            key_light_direction: SUN_DIRECTION_TO_LIGHT,
            key_light_color: SUN_LINEAR,
            exposure: 1.0,
            day_night_factor: 1.0,
            // palette[0] → ambient 颜色；补偿 day 亮度基准得到 world 色。
            palette: vec![LinearRgba::new(
                WORLD_LINEAR.red / 0.35,
                WORLD_LINEAR.green / 0.35,
                WORLD_LINEAR.blue / 0.35,
                1.0,
            )],
            ..StylizedViewDescriptor::default()
        });
    app.finish();
    app.cleanup();
    app.init_resource::<SceneReady>();
    spawn_fixture_scene(&mut app);
    app.add_systems(bevy::prelude::Update, (mark_ready, adopt_production));
    wait_ready(&mut app);
    let captured = capture_frame(&mut app, &target);
    let (gpu, driver) = gpu_info(&app);
    eprintln!("[capture] production on {gpu} ({driver})");
    captured
}

// ---------------------------------------------------------------------------
// (b) engine reference：legacy bevy_cel CharacterMaterial（neutral ramp）
// ---------------------------------------------------------------------------

fn neutral_ramp_image() -> Image {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    use bevy::{asset::RenderAssetUsages, image::ImageSampler};
    let mut image = Image::new_fill(
        Extent3d {
            width: 3,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255; 12],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::linear();
    image
}

fn adopt_legacy(
    mut commands: bevy::prelude::Commands,
    ready: Res<SceneReady>,
    standard: Res<Assets<StandardMaterial>>,
    mut characters: ResMut<Assets<bevy_cel::CharacterMaterial>>,
    mut images: ResMut<Assets<Image>>,
    meshes: Query<(bevy::prelude::Entity, &MeshMaterial3d<StandardMaterial>)>,
    mut done: bevy::prelude::Local<bool>,
) {
    if !ready.0 || *done {
        return;
    }
    let ramp = images.add(neutral_ramp_image());
    for (entity, handle) in &meshes {
        let source = standard.get(&handle.0).expect("ready").clone();
        let mut material = bevy_cel::character_material(source);
        material.extension = material
            .extension
            .clone()
            .with_ramp_texture(ramp.clone())
            .with_ramp_settings(bevy_cel::ToonRampSettings::default().with_bands(1, 0.0))
            .with_cel_lighting_settings(
                bevy_cel::CelLightingSettings::default().with_readability_floor(0.22),
            );
        let handle = characters.add(material);
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert(MeshMaterial3d(handle));
    }
    *done = true;
}

/// (b) legacy engine reference 路径捕获。
pub fn capture_engine_reference(resources: RenderResources) -> CapturedImage {
    let (mut app, target) = headless_app(resources);
    app.add_plugins(bevy_cel::CharacterMaterialPlugin);
    app.finish();
    app.cleanup();
    app.init_resource::<SceneReady>();
    spawn_fixture_scene(&mut app);
    // legacy 路径的 key light：显式 DirectionalLight（production 路径的
    // StylizedKeyLight 等价物）。
    app.world_mut().spawn((
        DirectionalLight {
            color: Color::LinearRgba(SUN_LINEAR),
            illuminance: 1.0,
            shadow_maps_enabled: false,
            ..DirectionalLight::default()
        },
        Transform::default().looking_to(-SUN_DIRECTION_TO_LIGHT, Vec3::Y),
    ));
    app.add_systems(bevy::prelude::Update, (mark_ready, adopt_legacy));
    wait_ready(&mut app);
    let captured = capture_frame(&mut app, &target);
    let (gpu, driver) = gpu_info(&app);
    eprintln!("[capture] engine_reference on {gpu} ({driver})");
    captured
}

// ---------------------------------------------------------------------------
// 三层对比报告
// ---------------------------------------------------------------------------

/// 一对层的对比报告。
#[derive(Clone, Debug, serde::Serialize)]
pub struct LayerPairReport {
    /// 对比双方。
    pub pair: String,
    /// Tier-2 报告。
    pub tier2: Tier2Report,
}

/// 运行三层对比，写 PNG + JSON 报告到 `output_dir()`。
/// 返回 `None` 表示无 GPU（graceful skip）。
pub fn run_three_layer() -> Option<Vec<LayerPairReport>> {
    let resources = capture::probe_render_resources()?;
    let production = capture_production(resources.clone());
    let engine = capture_engine_reference(resources);
    let upstream = golden_harness::decode_png(
        &std::fs::read(upstream_golden_path()).expect("upstream golden png"),
    )
    .expect("decode upstream golden");

    let dir = output_dir();
    std::fs::create_dir_all(&dir).expect("create output dir");
    for (name, image) in [("production", &production), ("engine_reference", &engine)] {
        let png = golden_harness::encode_png(image).expect("encode png");
        std::fs::write(dir.join(format!("{name}.png")), png).expect("write png");
    }

    let pairs = [
        ("engine_reference_vs_upstream", &engine, &upstream),
        ("production_vs_engine_reference", &production, &engine),
        ("production_vs_upstream", &production, &upstream),
    ];
    let mut reports = Vec::new();
    for (name, a, b) in pairs {
        let tier2 = evaluate_tier2(name, a, b, BACKGROUND_SRGB, Tier2Thresholds::default())
            .expect("tier2 evaluation");
        std::fs::write(
            dir.join(format!("{name}.tier2.json")),
            serde_json::to_string_pretty(&tier2).expect("serialize report"),
        )
        .expect("write report");
        eprintln!(
            "[tier2] {name}: max_nonedge={:.4} mean={:.4} psnr={:.2}dB shadow_iou={:.4} silhouette_iou={:.4} passed={}",
            tier2.max_abs_delta_nonedge,
            tier2.mean_abs_delta,
            tier2.psnr_db,
            tier2.shadow_iou,
            tier2.silhouette_iou,
            tier2.passed,
        );
        reports.push(LayerPairReport {
            pair: name.to_owned(),
            tier2,
        });
    }
    Some(reports)
}
