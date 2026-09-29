//! Showcase 框架：courtyard 庭院展台 + 10 条捕获规格
//! （docs/showcase/05；release gate §Showcase）。
//!
//! 每个用例 = 固定场景 + 可选 headless 捕获。职责边界：showcase 是
//! 完整 feature demo / manual QA / debug / benchmark host；内容构建器归
//! `labs/scenes`（返回相机配方），执行推进归 `labs/scene-runtime`
//! （CaseDriver），本 crate 只做宿主装配与输出绑定（插件、tier、资产根、
//! 视图基线、事件循环）；不充当 fixture（自动测试的最小资产在 fixtures/）。
//!
//! 捕获 profile（exact 合同，不因重构放宽）：旧流程"构建 → 24 次 update
//! → 发请求 → 下一帧提取"映射为驱动器动作图 `fixed_step{22} → observe`
//! 并关闭隐式就绪等待——Build 泵 + 调度泵 + 22 个实验 tick 恰好 24 帧
//! 暖场，观测复制落在第 25 帧，与旧流程逐帧一致。
//!
//! CLI：`cargo run -p showcase -- <name|all> [--capture]`
//! - 无 `--capture`：开窗交互模式；
//! - `--capture`：headless 离屏，PNG 写入 `target/stylized/showcase/`
//!   （gitignored）。

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    color::LinearRgba,
    math::Vec3,
    render::{
        RenderPlugin,
        settings::{RenderCreation, RenderResources},
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::*;
use golden_harness::{CapturedImage, capture};
use scenario_model::action::{ActionGraph, ActionKind, ActionNode, Step};
use scenario_model::canon::CanonicalU64;
use scenario_model::ident::{ActionId, SessionId};
use scene_runtime::{CaseDriver, DriverLimits, run_to_completion};
use scenes::registry::{self, SceneBuildFn, ViewPresetFn};

/// 捕获分辨率。
pub const CAPTURE_SIZE: u32 = 512;

/// 一个 showcase 用例：名称 + 场景构建器。
pub struct ShowcaseCase {
    /// 用例名（PNG 文件名 stem）。
    pub name: &'static str,
    /// 场景构建（`labs/scenes` 内容构建器；返回相机配方）。
    pub build: SceneBuildFn,
    /// 构建后、捕获前的额外 view 设置覆盖。
    pub view: Option<ViewPresetFn>,
    /// 资产根（None = "assets" 占位；由 catalog 资产依赖的 source 派生）。
    pub asset_root: Option<String>,
    /// 请求的执行档位（screen-space 描边等 Standard+ 特性用）。
    pub tier: Option<StylizedExecutionTier>,
}

pub(crate) fn headless_app(
    resources: RenderResources,
    asset_root: &str,
    tier: Option<StylizedExecutionTier>,
) -> App {
    configured_headless_app(resources, asset_root, tier, |_| {})
}

fn configured_headless_app(
    resources: RenderResources,
    asset_root: &str,
    tier: Option<StylizedExecutionTier>,
    configure: impl FnOnce(&mut App),
) -> App {
    let mut app = App::new();
    if let Some(tier) = tier {
        // 必须在 StylizedPlugin 之前（Stage B 在 finish 时读该配置）。
        app.insert_resource(StylizedCapabilityConfig {
            requested_tier: tier,
            ..StylizedCapabilityConfig::default()
        });
    }
    let plugins = DefaultPlugins
        .build()
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
        });
    #[cfg(feature = "interactive")]
    let plugins = plugins.disable::<bevy::winit::WinitPlugin>();
    app.add_plugins(plugins);
    // 角色台 GLB 的 NPR 材质 adoption（05 §4；旧 vanilla StandardMaterial
    // 路径在 Dawn/Metal 下渲染纯黑——host 限定上游差异）。
    app.add_plugins((
        StylizedPlugin::default(),
        bevy_stylized::StylizedAssetIntegrationPlugin,
    ));
    capture::install(&mut app);
    configure(&mut app);
    app.finish();
    app.cleanup();
    app
}

/// 通用 view 默认值（昼间 + 斜上 key light；交互会话的视图基线）。
///
/// 公共契约：showcase PC 捕获与 browser host 共享同一视图基线。
pub fn default_view_descriptor() -> StylizedViewDescriptor {
    StylizedViewDescriptor {
        key_light_direction: Vec3::new(0.5, 0.8, 0.6).normalize(),
        key_light_color: LinearRgba::WHITE,
        exposure: 1.0,
        day_night_factor: 1.0,
        ..StylizedViewDescriptor::default()
    }
}

pub(crate) fn default_view(world: &mut bevy::prelude::World) {
    *world.resource_mut::<StylizedViewSettings>() = StylizedViewSettings(default_view_descriptor());
}

/// 捕获 profile 的动作图：`fixed_step{22} → observe`（帧语义见 crate 文档）。
///
/// 公共契约：showcase PC 捕获与 browser host（apps/browser-showcase）共享
/// 同一动作图；改动它即改动两侧捕获的帧语义。
pub fn capture_profile_graph() -> ActionGraph {
    ActionGraph {
        nodes: vec![
            ActionNode {
                id: ActionId::new("settle").expect("action id"),
                tick: CanonicalU64(0),
                depends_on: vec![],
                writes: vec![],
                kind: ActionKind::FixedStep {
                    ticks: CanonicalU64(22),
                    step: Step::new(1, 60),
                },
            },
            ActionNode {
                id: ActionId::new("capture").expect("action id"),
                tick: CanonicalU64(0),
                depends_on: vec![ActionId::new("settle").expect("action id")],
                writes: vec![],
                kind: ActionKind::Observe {
                    observation: "capture".to_string(),
                    view: None,
                },
            },
        ],
    }
}

/// headless 跑一个用例并捕获；返回捕获图。
///
/// 与旧流程的逐帧等价性由 `capture_all`（17/17 字节比较）持续证明；
/// 观测回执的身份（session/generation/tick/stamp）随 driver 保留，
/// 本 API 只返回不可变像素（既有消费者签名不变）。
pub fn capture_case(case: &ShowcaseCase, resources: RenderResources) -> CapturedImage {
    let asset_root = case.asset_root.as_deref().unwrap_or("assets");
    let mut app = headless_app(resources, asset_root, case.tier);
    default_view(app.world_mut());
    if let Some(view) = case.view {
        let mut settings = app.world_mut().resource_mut::<StylizedViewSettings>();
        view(&mut settings.0);
    }
    let binding = registry::bindings()
        .into_iter()
        .find(|binding| binding.id == case.name)
        .expect("showcase case must be registered in scenes");
    let mut driver = CaseDriver::new(
        app.world(),
        SessionId::new("showcase-capture").expect("session id"),
        binding,
        capture_profile_graph(),
        DriverLimits {
            implicit_readiness: false,
            ..DriverLimits::default()
        },
    )
    .expect("capture profile must be a valid plan");
    if let Err(error) = run_to_completion(&mut app, &mut driver, 700) {
        panic!("showcase capture failed for {}: {error}", case.name);
    }
    let mut receipts = driver.drain_observations();
    assert_eq!(
        receipts.len(),
        1,
        "capture profile must produce exactly one observation"
    );
    receipts.pop().expect("one observation").image
}

/// 全部用例（顺序固定，与 `labs/scenes` 绑定表/catalog 一致；05 §10
/// 退役后只有 courtyard）。
///
/// 构建器与视图 preset 来自 `labs/scenes` 注册表；宿主侧元数据（资产根、
/// tier 需求）从 catalog 条目派生，无硬编码特例表。
pub fn all_cases() -> Vec<ShowcaseCase> {
    registry::bindings()
        .into_iter()
        .map(|binding| {
            // 宿主侧元数据读 catalog 条目（无硬编码特例表）：资产依赖 →
            // 资产根（source 的仓库相对路径转绝对）；tier 需求直通。
            let entry = scenario_model::catalog_entry(binding.id)
                .expect("binding must resolve to a catalog entry");
            let asset_root = entry.assets.first().map(|asset| {
                let relative = asset
                    .source
                    .strip_prefix("fixtures/")
                    .expect("asset source is repo-relative under fixtures/");
                // 注意：该路径在编译期展开为绝对路径（fixtures 目录位置固定）。
                concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/").to_string() + relative
            });
            ShowcaseCase {
                name: binding.id,
                build: binding.build,
                view: binding.view.map(|(_, preset)| preset),
                asset_root,
                tier: entry.tier,
            }
        })
        .collect()
}

/// 交互模式（开窗）。仅在 `interactive` feature（winit）下可用；
/// 默认构建为 headless-only（winit 不进入测试线程）。
///
/// 同一 App/Device 会话内经 CaseDriver 切换用例：内容/相机配方/view 可
/// 合法在会话内更换；tier 上调（已解析能力不能原地增加，R2）与资产根
/// 变更（启动时固定）以结构化信息拒绝并保留当前场景。严格 GPU gate 仍
/// 走每用例独立进程（R4），本模式只服务交互与明确的生命周期测试。
///
/// 交互 V1（docs/showcase/05 §11.3/§11.6）：窗口相机每帧从
/// `SessionController::camera_view()` 同步（与捕获相机同源，修
/// 01:416-417 的写死机位遗留）；键位表见 [`crate::keymap`]（courtyard
/// 参数面 + 机位循环，过渡期 ←/→ 语义见 keymap 文档）。
#[cfg(feature = "interactive")]
pub fn run_interactive(case: &ShowcaseCase) {
    interactive::run(case);
}

/// 无 `interactive` feature 时的占位实现。
#[cfg(not(feature = "interactive"))]
pub fn run_interactive(case: &ShowcaseCase) {
    eprintln!(
        "showcase case {:?}: interactive mode requires `--features interactive`",
        case.name
    );
}

#[cfg(feature = "interactive")]
mod interactive {
    use bevy::input::ButtonInput;
    use bevy::prelude::{Camera3d, KeyCode, Mut, Transform, World};
    use golden_harness::readiness;
    use scenario_model::action::ActionGraph;
    use scenario_model::ident::SessionId;
    use scene_runtime::{
        CaseDriver, CommandOutcome, DriverLimits, DriverStatus, RefusalReason, SessionCommand,
        SessionController,
    };

    use super::*;

    /// 桌面适配层状态：控制器（平台无关）+ 窗口相机 + 失败记录（宿主报告）
    /// + 最近相机状态缓存（egui 叠加层的只读数据源；camera_view() 需要
    /// &mut controller，非独占 UI 系统不能直接调用）。
    #[derive(bevy::prelude::Resource)]
    struct DesktopSession {
        controller: SessionController,
        window_camera: bevy::prelude::Entity,
        failed: bool,
        last_camera_view: Option<scene_runtime::CameraViewInfo>,
    }

    /// winit KeyCode → V1 按键集合（输入语义的唯一采集点；翻译归
    /// [`crate::keymap::translate_keys`]）。
    fn collect_key_presses(input: &ButtonInput<KeyCode>) -> crate::keymap::KeyPresses {
        crate::keymap::KeyPresses {
            prev: input.just_pressed(KeyCode::ArrowLeft),
            next: input.just_pressed(KeyCode::ArrowRight),
            prev_case: input.just_pressed(KeyCode::Comma),
            next_case: input.just_pressed(KeyCode::Period),
            time_next: input.just_pressed(KeyCode::ArrowUp),
            time_prev: input.just_pressed(KeyCode::ArrowDown),
            outline: input.just_pressed(KeyCode::KeyO),
            post: input.just_pressed(KeyCode::KeyP),
            crowd: if input.just_pressed(KeyCode::Digit1) {
                Some(0)
            } else if input.just_pressed(KeyCode::Digit2) {
                Some(100)
            } else if input.just_pressed(KeyCode::Digit3) {
                Some(1000)
            } else {
                None
            },
            flags: input.just_pressed(KeyCode::Space),
            reset: input.just_pressed(KeyCode::KeyR),
        }
    }

    pub fn run(case: &ShowcaseCase) {
        let mut app = App::new();
        app.add_plugins(
            DefaultPlugins.build().set(AssetPlugin {
                // 与 all_cases 的特例一致：courtyard/reference 的资产根指向
                // fixtures（会话资产根启动时固定，R2）。
                file_path: case
                    .asset_root
                    .clone()
                    .unwrap_or_else(|| "assets".to_string()),
                meta_check: bevy::asset::AssetMetaCheck::Never,
                ..Default::default()
            }),
        );
        app.add_plugins((
            StylizedPlugin::default(),
            bevy_stylized::StylizedAssetIntegrationPlugin,
        ));
        capture::install(&mut app);
        readiness::install(&mut app);
        default_view(app.world_mut());
        if let Some(view) = case.view {
            let mut settings = app.world_mut().resource_mut::<StylizedViewSettings>();
            view(&mut settings.0);
        }
        // 主窗口相机（宿主所有，在账本基线之前创建，不随场景切换退役）。
        // 初始 Transform 只是占位——desktop_tick 每帧从控制器的相机状态
        // 同步（§11.3：窗口与捕获画面同源）。
        let window_camera = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                Transform::from_xyz(0.0, 2.0, 8.0).looking_at(Vec3::Y, Vec3::Y),
            ))
            .id();
        let cases = all_cases();
        let current = cases
            .iter()
            .position(|entry| entry.name == case.name)
            .unwrap_or(0);
        let driver = CaseDriver::new(
            app.world(),
            SessionId::new("showcase-interactive").expect("session id"),
            registry::bindings()
                .into_iter()
                .find(|binding| binding.id == case.name)
                .expect("showcase case must be registered in scenes"),
            ActionGraph::default(),
            DriverLimits::default(),
        )
        .expect("interactive plan");
        let controller =
            SessionController::from_catalog(driver, current, default_view_descriptor())
                .expect("controller");
        app.insert_resource(DesktopSession {
            controller,
            window_camera,
            failed: false,
            last_camera_view: None,
        });
        // egui 叠加层（05 §11.4/§11.5 PC 行最小切片）：状态条 + 分区图例 +
        // 键位帮助。装配参照 Inspector 的模式（主上下文归专用 Camera2d，
        // 避免与场景/离屏相机的自动上下文竞争）。
        app.insert_resource(bevy_egui::EguiGlobalSettings {
            auto_create_primary_context: false,
            ..Default::default()
        });
        app.add_plugins(bevy_egui::EguiPlugin::default());
        app.world_mut().spawn((
            bevy::prelude::Camera2d,
            bevy::prelude::Camera {
                order: 10,
                ..Default::default()
            },
        ));
        app.add_systems(bevy::prelude::Update, desktop_tick);
        app.add_systems(bevy_egui::EguiPrimaryContextPass, overlay_ui);
        app.run();
    }

    /// 桌面输入适配：winit KeyCode → 控制器命令（唯一的输入翻译点；
    /// 业务逻辑在控制器，键位表归 [`crate::keymap`]）。
    fn desktop_tick(world: &mut World) {
        // EguiWantsInput 模式（05 §11.5 调研结论）：egui 面板消费输入的帧
        // 不翻译场景键位。
        let egui_wants_input = world
            .get_resource::<bevy_egui::input::EguiWantsInput>()
            .map(|wants| wants.wants_keyboard_input() || wants.wants_pointer_input())
            .unwrap_or(false);
        let keys = if egui_wants_input {
            crate::keymap::KeyPresses::default()
        } else {
            let input = world.resource::<ButtonInput<KeyCode>>();
            collect_key_presses(input)
        };
        world.resource_scope(|world, mut session: Mut<DesktopSession>| {
            // 能力解析（插件 finish 后）作为会话上限提供给控制器（R2；幂等）。
            if let Some(ceiling) = world
                .get_resource::<StylizedResolvedCapabilities>()
                .and_then(|resolved| resolved.resolved.clone().ok().map(|v| v.tier))
            {
                session.controller.set_tier_ceiling(ceiling);
            }
            // 翻译需要会话状态：当前用例、当前机位 preset、当前参数值。
            let commands = {
                let current_case = session.controller.current().id.as_str().to_string();
                let preset = session
                    .controller
                    .camera_view()
                    .and_then(|view| view.preset);
                let settings = world
                    .get_resource::<scenes::CourtyardSettings>()
                    .copied()
                    .unwrap_or_default();
                crate::keymap::translate_keys(&keys, &current_case, preset.as_deref(), settings)
            };
            for command in commands {
                match session.controller.command(world, &command) {
                    CommandOutcome::Switched { case, generation } => {
                        eprintln!("[interactive] switched to {case} (generation {generation})");
                    }
                    CommandOutcome::Applied => {
                        eprintln!("[interactive] applied");
                    }
                    CommandOutcome::Refused(reason) => {
                        eprintln!("[interactive] refused: {}", describe(&reason));
                    }
                }
            }
            if !session.failed
                && let DriverStatus::Failed(error) = session.controller.update(world)
            {
                session.failed = true;
                eprintln!("[interactive] driver failed: {error}");
            }
            // 窗口相机统一（§11.3 第一条）：每帧从控制器相机状态同步
            // Transform；相机尚未构建（首帧 Build 前）时保持占位机位。
            session.last_camera_view = session.controller.camera_view();
            if let Some(view) = session.last_camera_view.clone()
                && let Some(mut transform) = world.get_mut::<Transform>(session.window_camera)
            {
                *transform = Transform::from_translation(view.eye).looking_at(view.look, Vec3::Y);
            }
        });
    }

    /// egui 叠加层（05 §11.4/§11.5 PC 行最小切片）：左下常驻状态条
    /// （机位/时刻/描边/post/crowd/flags）+ 分区图例（可折叠，默认展开）
    /// + 键位帮助（默认折叠）。数据源：DesktopSession.last_camera_view
    /// （desktop_tick 每帧缓存）、CourtyardSettings 资源、
    /// StylizedResolvedCapabilities（tier 门控呈现——hybrid 不足 Standard
    /// 时置灰并带原因，§11.4）。UI 语言与 Inspector 面板一致（英文）。
    fn overlay_ui(
        mut contexts: bevy_egui::EguiContexts,
        session: bevy::prelude::Res<DesktopSession>,
        settings: Option<bevy::prelude::Res<scenes::CourtyardSettings>>,
        caps: Option<bevy::prelude::Res<StylizedResolvedCapabilities>>,
    ) -> bevy::prelude::Result {
        use bevy_egui::egui;
        let Ok(ctx) = contexts.ctx_mut() else {
            return Ok(());
        };
        ctx.set_visuals(egui::Visuals::dark());
        let settings = settings.as_deref().copied().unwrap_or_default();
        let preset = session
            .last_camera_view
            .as_ref()
            .and_then(|view| view.preset.clone())
            .unwrap_or_else(|| "panorama".to_string());
        let time = match settings.time_of_day {
            scenes::CourtyardTimeOfDay::Day => "day",
            scenes::CourtyardTimeOfDay::Dusk => "dusk",
            scenes::CourtyardTimeOfDay::Night => "night",
        };
        let flags = match settings.flags {
            scenes::CourtyardFlagsPose::Rest => "rest",
            scenes::CourtyardFlagsPose::Posed => "posed",
        };
        let standard = caps
            .and_then(|caps| caps.resolved.clone().ok())
            .is_some_and(|resolved| resolved.tier >= StylizedExecutionTier::Standard);

        // 左下常驻状态条。
        egui::Window::new("Status")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::LEFT_BOTTOM, [8.0, -8.0])
            .show(ctx, |ui| {
                ui.monospace(format!(
                    "preset {preset}  ·  {time}  ·  crowd {}  ·  flags {flags}  ·  post {}",
                    settings.crowd,
                    if settings.post { "on" } else { "off" },
                ));
                ui.horizontal(|ui| {
                    ui.monospace("outline:");
                    for (mode, label) in [
                        (scenes::CourtyardOutlineMode::Off, "off"),
                        (scenes::CourtyardOutlineMode::Hull, "hull"),
                        (scenes::CourtyardOutlineMode::Hybrid, "hybrid"),
                    ] {
                        let current = settings.outline_mode == mode;
                        let enabled = mode != scenes::CourtyardOutlineMode::Hybrid || standard;
                        let text = if current {
                            format!("[{label}]")
                        } else {
                            label.to_string()
                        };
                        if !enabled {
                            ui.weak(format!("{text} (requires Standard)"));
                        } else if current {
                            ui.strong(text);
                        } else {
                            ui.monospace(text);
                        }
                    }
                });
            });

        // 分区图例（可折叠，默认展开）。
        egui::Window::new("Legend")
            .anchor(egui::Align2::RIGHT_TOP, [-8.0, 8.0])
            .default_open(true)
            .collapsible(true)
            .show(ctx, |ui| {
                ui.heading("Courtyard zones");
                ui.separator();
                for (zone, desc) in [
                    (
                        "Center trio",
                        "face SDF threshold 0.2 / default / 0.8 · hair specular off/on · metal matcap off/on",
                    ),
                    ("P1 ramp", "analytic ramp vs texture ramp rows 0/1/2"),
                    ("P2 outline", "hull width 0 / 2 / 8 px"),
                    ("P3 hybrid", "hull + screen-space internal edges (Standard+)"),
                    ("P4 lighting", "key light override: none / +Z / -Z"),
                    ("P5 flags", "skinned flag strips · rest/posed"),
                    ("LOD axis", "near -> mid -> far -> very far (4 bands)"),
                    ("Lamp ring", "instanced emissive lamps · crowd 0/100/1000"),
                    ("Stage", "NPR character (reference GLB)"),
                ] {
                    ui.label(egui::RichText::new(zone).strong());
                    ui.small(desc);
                }
            });

        // 键位帮助（默认折叠）。
        egui::Window::new("Keys")
            .anchor(egui::Align2::LEFT_TOP, [8.0, 8.0])
            .default_open(false)
            .collapsible(true)
            .show(ctx, |ui| {
                for line in [
                    "Left/Right   camera preset cycle (courtyard) / case cycle (others)",
                    ", / .      prev / next case",
                    "Up/Down    time of day: day / dusk / night",
                    "O          outline mode: off / hull / hybrid",
                    "P          post process on/off",
                    "1 / 2 / 3  crowd: 0 / 100 / 1000",
                    "Space      flags rest/posed",
                    "R          reset view to preset",
                ] {
                    ui.monospace(line);
                }
            });
        Ok(())
    }

    fn describe(reason: &RefusalReason) -> String {
        match reason {
            RefusalReason::TierRequired {
                required,
                resolved,
                reconfiguration_attempted,
            } => {
                if *reconfiguration_attempted {
                    format!(
                        "case requires {required:?}; in-session reconfiguration was refused by the device (session resolved {resolved:?}); a new device session is required"
                    )
                } else {
                    format!(
                        "case requires {required:?} but the session resolved {resolved:?} and no capability snapshot is available to attempt reconfiguration"
                    )
                }
            }
            RefusalReason::AssetRootFixed(id) => format!(
                "{id} needs a fixed asset root; the session asset root is fixed at startup (a new session is required)"
            ),
            RefusalReason::UnknownScene(id) => format!("unknown case {id}"),
            RefusalReason::UnknownCameraPreset(preset) => {
                format!("unknown camera preset {preset:?} for the current case")
            }
            RefusalReason::UnknownParameter(key) => {
                format!("unknown parameter {key:?} for the current case")
            }
            RefusalReason::InvalidParameter(message) => format!("invalid parameter: {message}"),
            RefusalReason::InvalidCamera(message) => format!("invalid camera command: {message}"),
            RefusalReason::DriverBusy(message) => format!("driver rejected the switch: {message}"),
        }
    }
}

#[cfg(test)]
mod driver_adoption_tests {
    //! CaseDriver/SessionController 采用证据（GPU-gated）：会话内内容重建
    //! （crowd 100→0→100，退役后单场景的 rebuild 循环）的 headless 验证——
    //! 经共享控制器的命令接口执行，资源计数回到首次构建基线，重建回来的
    //! 捕获与单次运行捕获逐字节一致。
    use super::*;
    use scene_runtime::{CommandOutcome, SceneCounts, SessionCommand, SessionController};

    fn binding_of(name: &str) -> scenes::registry::SceneBinding {
        registry::bindings()
            .into_iter()
            .find(|binding| binding.id == name)
            .expect("registered scene")
    }

    #[test]
    #[ignore = "requires explicit GPU execution"]
    fn scene_rebuild_cycle_preserves_capture_identity() {
        let Some(resources) = capture::probe_render_resources() else {
            eprintln!("showcase driver adoption: no GPU adapter available; skipping");
            return;
        };
        let cases = all_cases();
        let courtyard = cases
            .iter()
            .find(|case| case.name == "courtyard")
            .expect("case");
        let single_run = capture_case(courtyard, resources.clone());
        let asset_root = courtyard
            .asset_root
            .clone()
            .unwrap_or_else(|| "assets".to_string());

        let mut app = headless_app(resources, &asset_root, None);
        default_view(app.world_mut());
        let limits = DriverLimits {
            implicit_readiness: false,
            ..DriverLimits::default()
        };
        let driver = CaseDriver::new(
            app.world(),
            SessionId::new("showcase-interactive").expect("session id"),
            binding_of("courtyard"),
            capture_profile_graph(),
            limits,
        )
        .expect("plan");
        let mut controller = SessionController::from_catalog(driver, 0, default_view_descriptor())
            .expect("controller");
        controller.set_tier_ceiling(StylizedExecutionTier::Standard);
        run_to_completion(&mut app, controller.driver_mut(), 700).expect("build completes");
        // reference-scene GLB 异步实例化：先排空到实体计数稳定再记基线
        // （计数漂移不是泄漏——旧 outline 场景无 GLB，无此窗口）。
        for _ in 0..8 {
            app.update();
        }
        let first_build = SceneCounts::capture(app.world());
        assert_eq!(controller.driver().generation().get(), 0);

        // 重建循环（内容类参数，同场景 retire→build；退役后单场景目录下
        // SelectCase 被资产根规则拒绝，重建循环覆盖同一路径）。
        for (index, crowd) in [0u32, 100].into_iter().enumerate() {
            let command = SessionCommand::SetParameter {
                key: "crowd".to_string(),
                value: serde_json::json!(crowd),
            };
            assert!(matches!(
                controller.command(app.world_mut(), &command),
                CommandOutcome::Switched { .. }
            ));
            run_to_completion(&mut app, controller.driver_mut(), 700).expect("rebuild completes");
        }
        assert_eq!(controller.driver().generation().get(), 2);
        // 排空若干帧（资产句柄释放/场景实例清理的处理窗口）后逐项对账。
        for _ in 0..32 {
            app.update();
        }
        let after = SceneCounts::capture(app.world());
        assert_eq!(
            after, first_build,
            "rebuild cycle must return to the first-build counts (no leaked entities/cameras/assets)"
        );

        let receipts = controller.driver().observations();
        assert_eq!(receipts.len(), 3, "each generation observes once");
        let post_rebuild = &receipts[2];
        assert_eq!(post_rebuild.generation.get(), 2);
        assert_eq!(
            post_rebuild.image.data, single_run.data,
            "post-rebuild capture must equal the single-run capture byte-for-byte"
        );
    }
}

#[cfg(feature = "inspector")]
pub mod inspector;

pub mod keymap;

pub mod capture_spec;
