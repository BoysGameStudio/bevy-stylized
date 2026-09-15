//! Independent production/upstream comparison on a declared reference fixture.
//! Historical legacy-engine captures are not part of this active renderer graph.
//! The CLI can report diagnostics or require visual criteria; those are distinct outcomes.

pub mod character;

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    color::{Color, LinearRgba},
    core_pipeline::tonemapping::Tonemapping,
    gltf::GltfAssetLabel,
    image::Image,
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

fn case_path(key: &str) -> PathBuf {
    let path = PathBuf::from(
        std::env::var("STYLIZED_REFERENCE_CASE")
            .expect("explicit STYLIZED_REFERENCE_CASE required"),
    );
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).expect("reference case missing"))
            .expect("reference case JSON");
    assert_eq!(record["schema"], "bevy_stylized.reference-case/v1");
    let value = PathBuf::from(record[key].as_str().expect("reference case path missing"));
    if value.is_absolute() {
        value
    } else {
        path.parent().expect("case parent").join(value)
    }
}
fn fixture_dir() -> PathBuf {
    case_path("fixture")
        .parent()
        .expect("fixture parent")
        .to_path_buf()
}
/// Explicit diagnostic output directory.
pub fn output_dir() -> PathBuf {
    case_path("output")
}
fn upstream_golden_path() -> PathBuf {
    case_path("upstream")
}
fn upstream_sidecar_path() -> PathBuf {
    case_path("sidecar")
}

/// 从桥 sidecar 提取的 upstream palette ground truth。
///
/// 来源（docs/reference/07 §3.7，2026-09-12 核实）：palette socket 由
/// `ShaderNodeAttribute` 驱动（Lighting Panel rig / wizard 运行时注入）；
/// blend 内可提取的 ground truth 是 `Global Material Properties` 的
/// interface 默认值 + Lighting Panel picker 默认（均为白色），材质级阴影
/// 色在各材质的 group 实例默认值里。
struct UpstreamPalette {
    // 全局四色接口默认（全白）不再消费——golden 用的是 rig 注入值
    //（未导出），白色会整体欠曝；lit 能量经太阳常数归一（见
    // adopt_production）。此处只保留实际消费的字段。
    /// 全局 `Shadow Position` 默认值（阴影阈值基准）。
    shadow_position: f32,
    /// 材质级 ground truth：Face Shader 的 Daytime Shadow Color
    ///（暖粉阴影色，docs/reference/07 §3.7 已核实为材质实例默认）。
    daytime_shadow: LinearRgba,
}

fn sidecar_color(defaults: &serde_json::Value, name: &str) -> Option<LinearRgba> {
    let arr = defaults.get(name)?.as_array()?;
    if arr.len() < 3 {
        return None;
    }
    Some(LinearRgba::new(
        arr[0].as_f64()? as f32,
        arr[1].as_f64()? as f32,
        arr[2].as_f64()? as f32,
        1.0,
    ))
}

fn load_upstream_palette() -> UpstreamPalette {
    let path = upstream_sidecar_path();
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("upstream sidecar missing ({e}): {path:?} — run the export bridge on the vendor main blend (role=main)"));
    let json: serde_json::Value = serde_json::from_str(&text).expect("parse upstream sidecar");
    let defaults = json
        .get("bevy_stylized.genshin.ground_truth")
        .and_then(|gt| gt.get("palette"))
        .and_then(|p| p.get("interface_defaults"))
        .expect("sidecar missing bevy_stylized.genshin.ground_truth.palette.interface_defaults");
    let color = |name: &str| {
        sidecar_color(defaults, name).unwrap_or_else(|| panic!("palette socket {name} missing"))
    };
    let _ = color; // 四色接口默认已知为全白（见 struct 注释），不再读取
    UpstreamPalette {
        shadow_position: defaults
            .get("Shadow Position")
            .and_then(serde_json::Value::as_f64)
            .expect("palette socket Shadow Position missing") as f32,
        daytime_shadow: json
            .get("bevy_stylized.genshin.ground_truth")
            .and_then(|gt| gt.get("materials"))
            .and_then(|m| m.get("HoYoverse - Genshin Face"))
            .and_then(|m| m.get("HoYoverse - GI Face"))
            .and_then(|g| g.get("inputs"))
            .and_then(|i| sidecar_color(i, "Daytime Shadow Color"))
            .expect("face Daytime Shadow Color missing"),
    }
}

#[derive(Resource, Clone)]
struct TargetHolder(capture::CaptureTarget);

fn headless_app(resources: RenderResources) -> (App, capture::CaptureTarget) {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(bevy_stylized::stylized_gltf_plugin())
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
// 侧车声明的纹理接线（export-sidecar-v1.1 ground_truth.textures +
// gacha_upstream.golden.json 的本地 provenance）
// ---------------------------------------------------------------------------

struct SidecarTextures {
    /// ILM：R=metal / G=AO / B=spec 抑制 / A=材质分带（Non-Color）。
    body_lightmap: Option<bevy::prelude::Handle<Image>>,
    /// 同上（hair 用）。
    hair_lightmap: Option<bevy::prelude::Handle<Image>>,
    /// Face SDF 阈值图（Non-Color）。
    face_lightmap: Option<bevy::prelude::Handle<Image>>,
    /// 阴影 ramp atlas（sRGB）。
    shadow_ramp: Option<bevy::prelude::Handle<Image>>,
    // specular_ramp / metal_map：侧车有声明，但 Use Specular Ramp=0 且
    // Metallic 块公式与我方 matcap 语义不同源——有意不加载（见
    // docs/reference/08 第四轮记录）。
}

fn load_png_image(
    images: &mut Assets<Image>,
    path: &PathBuf,
    srgb: bool,
) -> bevy::prelude::Handle<Image> {
    let decoded = golden_harness::decode_png(&std::fs::read(path).expect("read texture png"))
        .expect("decode texture png");
    let format = if srgb {
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb
    } else {
        bevy::render::render_resource::TextureFormat::Rgba8Unorm
    };
    let mut image = Image::new_fill(
        bevy::render::render_resource::Extent3d {
            width: decoded.width,
            height: decoded.height,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        &[0, 0, 0, 255],
        format,
        bevy::asset::RenderAssetUsages::default(),
    );
    image.data = Some(decoded.data);
    image.sampler = bevy::image::ImageSampler::linear();
    images.add(image)
}

/// Load the explicitly declared maps; missing input is a failure, never a new case.
fn load_sidecar_textures(images: &mut Assets<Image>) -> SidecarTextures {
    let load = |images: &mut Assets<Image>, name: &str, srgb: bool| {
        Some(load_png_image(
            images,
            &case_path("textures").join(name),
            srgb,
        ))
    };
    SidecarTextures {
        body_lightmap: load(images, "body_lightmap.png", false),
        hair_lightmap: load(images, "hair_lightmap.png", false),
        face_lightmap: load(images, "face_lightmap.png", false),
        shadow_ramp: load(images, "shadow_ramp.png", true),
    }
}

// ---------------------------------------------------------------------------
// (a) production：StylizedHeroMaterial
// ---------------------------------------------------------------------------

fn adopt_production(
    mut commands: bevy::prelude::Commands,
    ready: Res<SceneReady>,
    standard: Res<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<StylizedHeroMaterial>>,
    mut images: ResMut<Assets<Image>>,
    meshes: Query<(
        bevy::prelude::Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&bevy::gltf::GltfMaterialName>,
    )>,
    mut done: bevy::prelude::Local<bool>,
) {
    if !ready.0 || *done {
        return;
    }
    let palette = load_upstream_palette();
    // 侧车纹理（export-sidecar-v1.1 声明 + golden provenance 的本地路径；
    // Missing input fails instead of silently changing the compared case.
    let textures = load_sidecar_textures(&mut images);
    let textures_present = textures.body_lightmap.is_some();
    eprintln!("[adopt] sidecar textures present: {textures_present}");

    for (entity, handle, material_name) in &meshes {
        let source = standard.get(&handle.0).expect("ready");
        let factor = LinearRgba::from(source.base_color);
        let mut params = StylizedHeroParams::default();
        params.character.cel.base_color_factor =
            bevy::math::Vec4::new(factor.red, factor.green, factor.blue, 1.0);
        // palette 取自桥提取的 upstream ground truth（export-main
        // sidecar）。**lit 色 = fixture 太阳能量色（(1,0.956,0.898)×3.0，
        // 线性）**：已核实上游结构中 palette lit 携带光能（shader 的
        // diffuse 项不乘 key light 颜色/能量——energy 走 palette 通道），
        // golden 的 rig 实际值未入任何导出物，最忠实的可溯源近似是
        // fixture 的同名太阳常数（与 SUN_LINEAR 同源，去掉 1/π——半兰伯特
        // 路径无 Lambert 归一）。shadow 色 = 侧车材质级 ground truth
        //（Daytime Shadow Color (0.9,0.7,0.75)）；接口默认的白色四色
        // 证明与 golden 不符（golden 用 rig 注入值），白色会导致整体
        // 欠曝——这是已记录的导出物缺口（rig palette 未导出）。
        let sun_energy = LinearRgba::new(1.0 * 3.0, 0.956 * 3.0, 0.898 * 3.0, 1.0);
        params.character = params.character.with_palette(
            sun_energy,
            palette.daytime_shadow,
            LinearRgba::WHITE, // sharp lit = 白（能量只进 soft 层，避免双层相乘过曝）
            palette.daytime_shadow,
        );
        params.character.cel = params
            .character
            .cel
            .with_shadow(palette.shadow_position, 0.1, 0.3)
            .with_rim(3.0, 0.2, LinearRgba::new(0.5, 0.6, 0.8, 1.0));

        // 按材质名接线侧车纹理（已核实 ILM 布局 R=metal/G=AO/B=spec
        // 抑制/A=分带；docs/reference/07 §3.2）：
        //
        // - Head：face SDF = face_lightmap（Non-Color），face basis +Z
        //   （fixture 头朝相机）；face_shadow（blush overlay）**生产材质无
        //   此槽位** —— 记录在案的 wiring gap（见 docs/reference/08）；
        // - Body/Hair：ILM lightmap + Genshin 寻址的 shadow ramp +
        //   specular ramp + metal matcap（MetalMap 语义 = matcap）；
        // - Ground：无纹理（场景地板）。
        let name = material_name.map(|n| n.0.as_str()).unwrap_or("");
        let material = StylizedHeroMaterial {
            params: match name {
                "FixtureMatHead" => {
                    // 侧车 ground truth：Face Shadow Softness = 0.001
                    //（近硬边界）。阈值不覆盖——用 head-basis 光照角导出值
                    //（face_threshold_from_dots；bias 属 authoring 微调，
                    // 上游 Face 组无此输入）。
                    params.character = params.character.with_face_forward(Vec3::Z, 0.001);
                    params
                }
                "FixtureMatBody" | "FixtureMatHair" => {
                    // 侧车材质级 ground truth：Use Shadow Ramp=1（ramp width
                    // 1.0）、Use Specular Ramp=**0**（不接线）。
                    params = params.with_genshin_ramp(palette.shadow_position, 1.0);
                    // Enable Metallics=1 在上游有效，但其 matcap 块公式
                    // （dark/light tint 混合 + brightness 缩放）与我方
                    // matcap 语义不同源——绑定 metal_map 反引入误差
                    // （实测：不绑 17.6dB > 绑 16.7dB），**不接线**并在
                    // docs/reference/08 记录该语义缺口。
                    params
                }
                _ => params,
            },
            base_color_texture: source.base_color_texture.clone(),
            ilm_texture: match name {
                "FixtureMatBody" => textures.body_lightmap.clone(),
                "FixtureMatHair" => textures.hair_lightmap.clone(),
                _ => None,
            },
            face_sdf_texture: match name {
                "FixtureMatHead" => textures.face_lightmap.clone(),
                _ => None,
            },
            ramp_texture: match name {
                "FixtureMatBody" | "FixtureMatHair" => textures.shadow_ramp.clone(),
                _ => None,
            },
            // Use Specular Ramp = 0（侧车 ground truth）→ 不绑定。
            spec_ramp_texture: None,
            // matcap 语义缺口（见上）：不绑定 metal_map。
            matcap_texture: None,
            // 发卡面片等 double-sided 资产（ADR-0023 矩阵的 cull 维度）。
            double_sided: source.double_sided,
            ..StylizedHeroMaterial::default()
        };
        let material = materials.add(material);
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
            // ambient 为白色昼间亮度；背景色由 ClearColor 承担（world 色）。
            palette: vec![LinearRgba::WHITE],
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
/// 一对层的对比报告。
#[derive(Clone, Debug, serde::Serialize)]
pub struct LayerPairReport {
    /// 对比双方。
    pub pair: String,
    /// Tier-2 报告。
    pub tier2: Tier2Report,
}

/// Compare production with upstream and write the unmodified visual result.
pub fn run_reference_comparison() -> Result<Vec<LayerPairReport>, String> {
    let resources = capture::probe_render_resources().ok_or("required GPU unavailable")?;
    let production = capture_production(resources);
    let upstream = golden_harness::decode_png(
        &std::fs::read(upstream_golden_path()).expect("upstream golden png"),
    )
    .expect("decode upstream golden");

    let dir = output_dir();
    std::fs::create_dir_all(&dir).expect("create output dir");
    let png = golden_harness::encode_png(&production).expect("encode png");
    std::fs::write(dir.join("production.png"), png).expect("write png");

    let pairs = [("production_vs_upstream", &production, &upstream)];
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
    Ok(reports)
}
