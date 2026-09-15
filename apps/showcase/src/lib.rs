//! Showcase 框架（It-9a）：17 个固定用例（docs/showcase/01 关卡表 ×
//! release gate §Showcase）。
//!
//! 每个用例 = 固定场景构建器 + 可选 headless 捕获。职责边界：
//! showcase 是完整 feature demo / manual QA / debug / benchmark host；
//! 不充当 fixture（自动测试的最小资产在 fixtures/）。
//!
//! CLI：`cargo run -p showcase -- <name|all> [--capture]`
//! - 无 `--capture`：开窗交互模式；
//! - `--capture`：headless 离屏，PNG 写入 `target/stylized/showcase/`
//!   （gitignored）。

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    color::LinearRgba,
    core_pipeline::tonemapping::Tonemapping,
    image::Image,
    math::Vec3,
    prelude::{Camera3d, Transform},
    render::{
        RenderPlugin,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        settings::{RenderCreation, RenderResources},
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::*;
use golden_harness::{CapturedImage, capture};

/// 捕获分辨率。
pub const CAPTURE_SIZE: u32 = 512;

/// 一个 showcase 用例：名称 + 场景构建器。
pub struct ShowcaseCase {
    /// 用例名（PNG 文件名 stem）。
    pub name: &'static str,
    /// 场景构建（往已装配好的 App 里放内容）。
    pub build: fn(&mut App),
    /// 构建后、捕获前的额外 view 设置覆盖。
    pub view: Option<fn(&mut StylizedViewDescriptor)>,
    /// 资产根（None = "assets" 占位；reference 用例指向 fixtures/synthetic/reference-scene）。
    pub asset_root: Option<&'static str>,
    /// 请求的执行档位（screen-space 描边等 Standard+ 特性用）。
    pub tier: Option<StylizedExecutionTier>,
}

fn headless_app(
    resources: RenderResources,
    asset_root: &str,
    tier: Option<StylizedExecutionTier>,
) -> App {
    let mut app = App::new();
    if let Some(tier) = tier {
        // 必须在 StylizedPlugin 之前（Stage B 在 finish 时读该配置）。
        app.insert_resource(StylizedCapabilityConfig {
            requested_tier: tier,
            ..StylizedCapabilityConfig::default()
        });
    }
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
    app.finish();
    app.cleanup();
    app
}

/// 相机离屏 target 句柄（spawn_camera 时登记）。
#[derive(bevy::prelude::Resource, Clone)]
struct ShowcaseTarget(capture::CaptureTarget);

fn spawn_camera(app: &mut App, eye: Vec3, look: Vec3) {
    let target = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        capture::create_capture_target(&mut images, CAPTURE_SIZE, CAPTURE_SIZE)
    };
    app.world_mut().spawn((
        Camera3d::default(),
        capture::capture_target_bundle(&target),
        Tonemapping::None,
        Transform::from_translation(eye).looking_at(look, Vec3::Y),
    ));
    app.insert_resource(ShowcaseTarget(target));
}

/// 通用 view 默认值（昼间 + 斜上 key light）。
fn default_view(app: &mut App) {
    *app.world_mut().resource_mut::<StylizedViewSettings>() =
        StylizedViewSettings(StylizedViewDescriptor {
            key_light_direction: Vec3::new(0.5, 0.8, 0.6).normalize(),
            key_light_color: LinearRgba::WHITE,
            exposure: 1.0,
            day_night_factor: 1.0,
            ..StylizedViewDescriptor::default()
        });
}

/// headless 跑一个用例并捕获；返回捕获图。
pub fn capture_case(case: &ShowcaseCase, resources: RenderResources) -> CapturedImage {
    let asset_root = case.asset_root.unwrap_or("assets");
    let mut app = headless_app(resources, asset_root, case.tier);
    default_view(&mut app);
    if let Some(view) = case.view {
        let mut settings = app.world_mut().resource_mut::<StylizedViewSettings>();
        view(&mut settings.0);
    }
    (case.build)(&mut app);
    for _ in 0..24 {
        app.update();
    }
    let target = app
        .world()
        .get_resource::<ShowcaseTarget>()
        .expect("showcase case must spawn its camera via case_camera")
        .0
        .clone();
    let request = capture::request_capture(app.world_mut(), target);
    for _ in 0..600 {
        app.update();
        if let Some(frame) = capture::try_take(app.world_mut(), request.clone()) {
            return frame;
        }
    }
    panic!("showcase capture timed out for {}", case.name);
}

/// 全部 17 个用例（顺序固定）。
pub fn all_cases() -> Vec<ShowcaseCase> {
    vec![
        ShowcaseCase {
            name: "overview",
            build: cases::overview,
            view: None,
            asset_root: None,
            tier: None,
        },
        ShowcaseCase {
            name: "ramp",
            build: cases::ramp,
            view: None,
            asset_root: None,
            tier: None,
        },
        ShowcaseCase {
            name: "lighting",
            build: cases::lighting,
            view: None,
            asset_root: None,
            tier: None,
        },
        ShowcaseCase {
            name: "face",
            build: cases::face,
            view: None,
            asset_root: None,
            tier: None,
        },
        ShowcaseCase {
            name: "hair",
            build: cases::hair,
            view: None,
            asset_root: None,
            tier: None,
        },
        ShowcaseCase {
            name: "metal",
            build: cases::metal,
            view: None,
            asset_root: None,
            tier: None,
        },
        ShowcaseCase {
            name: "outline",
            build: cases::outline,
            view: None,
            asset_root: None,
            tier: None,
        },
        ShowcaseCase {
            name: "animation",
            build: cases::animation,
            view: None,
            asset_root: None,
            tier: None,
        },
        ShowcaseCase {
            name: "post",
            build: cases::post,
            view: None,
            asset_root: None,
            tier: None,
        },
        ShowcaseCase {
            name: "stress",
            build: cases::stress,
            view: None,
            asset_root: None,
            tier: None,
        },
        ShowcaseCase {
            name: "reference",
            build: cases::reference,
            view: None,
            asset_root: Some(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../fixtures/synthetic/reference-scene"
            )),
            tier: None,
        },
        ShowcaseCase {
            name: "performance",
            build: cases::performance,
            view: None,
            asset_root: None,
            tier: None,
        },
        ShowcaseCase {
            name: "outline-advanced",
            build: cases::outline_advanced,
            view: None,
            asset_root: None,
            // screen-space 描边是 Standard+ 特性。
            tier: Some(StylizedExecutionTier::Standard),
        },
        ShowcaseCase {
            name: "lod",
            build: cases::lod,
            view: None,
            asset_root: None,
            tier: None,
        },
        // environment（V2-d）：同场景三套环境调色板（昼雾/黄昏/夜）。
        ShowcaseCase {
            name: "environment-day",
            build: cases::environment,
            view: Some(cases::environment_day),
            asset_root: None,
            tier: None,
        },
        ShowcaseCase {
            name: "environment-dusk",
            build: cases::environment,
            view: Some(cases::environment_dusk),
            asset_root: None,
            tier: None,
        },
        ShowcaseCase {
            name: "environment-night",
            build: cases::environment,
            view: Some(cases::environment_night),
            asset_root: None,
            tier: None,
        },
    ]
}

pub mod cases;

/// 交互模式（开窗）。仅在 `interactive` feature（winit）下可用；
/// 默认构建为 headless-only（winit 不进入测试线程）。
#[cfg(feature = "interactive")]
pub fn run_interactive(case: &ShowcaseCase) {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins(StylizedPlugin::default());
    default_view(&mut app);
    if let Some(view) = case.view {
        let mut settings = app.world_mut().resource_mut::<StylizedViewSettings>();
        view(&mut settings.0);
    }
    (case.build)(&mut app);
    // 交互模式补一个主窗口相机
    app.world_mut().spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.0, 8.0).looking_at(Vec3::Y, Vec3::Y),
    ));
    app.run();
}

/// 无 `interactive` feature 时的占位实现。
#[cfg(not(feature = "interactive"))]
pub fn run_interactive(case: &ShowcaseCase) {
    eprintln!(
        "showcase case {:?}: interactive mode requires `--features interactive`",
        case.name
    );
}

/// 生成 ramp 测试纹理（rows × 64，左半行色右半白）。
pub fn ramp_image(rows: &[[u8; 4]]) -> Image {
    let mut data = Vec::new();
    for row in rows {
        for col in 0..64 {
            let px = if col >= 32 {
                [245, 245, 245, 255]
            } else {
                *row
            };
            data.extend_from_slice(&px);
        }
    }
    let mut image = Image::new_fill(
        Extent3d {
            width: 64,
            height: rows.len() as u32,
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

/// 供 cases 模块复用的相机辅助。
pub fn case_camera(app: &mut App, eye: Vec3, look: Vec3) {
    spawn_camera(app, eye, look);
}

#[cfg(feature = "inspector")]
pub mod inspector;
