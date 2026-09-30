//! Showcase 框架：courtyard 庭院展台 + 12 条捕获规格
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
/// 参数面 + 机位循环，过渡期 ←/→ 语义见 keymap 文档）。U1（06 §13）：
/// 鼠标手势（[`crate::mousemap`]）、toast 反馈、演示模式（H）、机位补间
/// （`camera_transition` = 补间开关；冒烟路径传 false 钉关，06 §8）。
/// U2：分区 dock、schema 驱动参数抽屉（[`crate::param_panel`]）。
/// （v3：minimap 与场景内点击聚焦随展亭模式退役——同一位置无空间导航。）
/// U3：导览模式（G，[`crate::tour`]
/// 脚本数据）、会话级 journal 录制（命令循环经 `command_recorded`，
/// 键位/鼠标/UI/BRP 四驱动源同通道）、C=Capture（[`SessionCommand::Capture`]
/// → driver ad-hoc observe，浏览快照落盘 target/stylized/showcase/
/// interactive/，不参与 golden 判定）。
#[cfg(feature = "interactive")]
pub fn run_interactive(case: &ShowcaseCase, camera_transition: bool) {
    interactive::run(case, camera_transition);
}

/// 无 `interactive` feature 时的占位实现。
#[cfg(not(feature = "interactive"))]
pub fn run_interactive(case: &ShowcaseCase, camera_transition: bool) {
    let _ = camera_transition;
    eprintln!(
        "showcase case {:?}: interactive mode requires `--features interactive`",
        case.name
    );
}

#[cfg(feature = "interactive")]
mod interactive {
    use bevy::ecs::schedule::IntoScheduleConfigs;
    use bevy::input::ButtonInput;
    use bevy::input::mouse::{MouseButton, MouseMotion, MouseScrollUnit, MouseWheel};
    use bevy::prelude::{
        Camera3d, KeyCode, MessageReader, Mut, Query, Res, ResMut, Transform, Vec2, Window, With,
        World,
    };
    use bevy::window::PrimaryWindow;
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
        /// 命令结果 toast 堆叠（06 §7；Refused 驻留至下次命令或点击关闭）。
        toasts: std::collections::VecDeque<Toast>,
        /// 演示模式（06 §8；H 开关 / Esc 退出）：隐藏全部 egui UI。
        presentation: bool,
        /// driver 状态快照（状态条 building 指示，06 §7）。
        driver_running: bool,
        /// 主键按下期间的拖拽累积（点击/拖拽阈值判定，06 §5）。
        drag: DragState,
        /// 双击判定（合成时间戳可测，mousemap）。
        click_tracker: crate::mousemap::ClickTracker,
        /// UI 队列命令（分区 dock / 参数抽屉；desktop_tick 次帧经
        /// 同一 journal 录制通道执行——UI 只是 SessionCommand 驱动源，06 §2）。
        pending_commands: Vec<SessionCommand>,
        /// 会话级命令日志（06 §8，U3：命令循环经 `command_recorded` 录制；
        /// 键位/鼠标/UI/BRP 四驱动源同通道）。
        journal: scene_runtime::CommandJournal,
        /// 导览进度（06 §8，U3；None = 关闭；任意导航输入退出）。
        tour: Option<crate::tour::TourProgress>,
        /// 已消费的观测回执数（U3 Capture 落盘：只处理新增回执）。
        seen_observations: usize,
    }

    /// 命令结果 toast（右下堆叠，06 §7）。
    struct Toast {
        text: String,
        /// Refused = 驻留（琥珀色、点击关闭或下次命令关闭）；其余按 ttl 消退。
        sticky: bool,
        /// 非驻留 toast 的显示时长（秒；快照路径 toast 用 6s，其余 2s）。
        ttl_secs: f32,
        created: std::time::Instant,
    }

    /// 主键按下期间的拖拽累积：位移阈值内是候选点击（积蓄位移不发射），
    /// 越过阈值转拖拽并补发积蓄位移（06 §5 判定口径）。
    #[derive(Default)]
    struct DragState {
        /// 按下时的光标位置。
        origin: Option<Vec2>,
        /// 按下后累计位移（px）。
        accumulated: Vec2,
        /// 是否已越过点击阈值转为拖拽。
        dragging: bool,
    }

    /// 一帧的原始鼠标输入（collect_mouse_frame 采集；desktop_tick 消费）。
    /// 与 collect_key_presses 同一分层：winit/bevy 输入语义只在此出现一次。
    #[derive(bevy::prelude::Resource, Default, Clone, Copy)]
    struct MouseFrame {
        /// 本帧累计光标位移（px）。
        motion: Vec2,
        /// 本帧滚轮 y（行；Pixel 单位按 16px/行折算）。
        scroll: f32,
        primary_pressed: bool,
        secondary_pressed: bool,
        primary_just_pressed: bool,
        primary_just_released: bool,
        /// 主窗口光标位置（px）。
        cursor: Option<Vec2>,
        /// 主窗口高度（px；pan 折算分母）。
        viewport_height: f32,
    }

    /// winit 鼠标事件 → MouseFrame（唯一采集点；翻译归
    /// [`crate::mousemap::translate_mouse`]）。
    fn collect_mouse_frame(
        mut motion: MessageReader<MouseMotion>,
        mut wheel: MessageReader<MouseWheel>,
        buttons: Res<ButtonInput<MouseButton>>,
        windows: Query<&Window, With<PrimaryWindow>>,
        mut frame: ResMut<MouseFrame>,
    ) {
        frame.motion = motion.read().map(|event| event.delta).sum();
        frame.scroll = wheel
            .read()
            .map(|event| match event.unit {
                MouseScrollUnit::Line => event.y,
                // 触控板/高精度滚轮：像素折算行（约 16px/行）。
                MouseScrollUnit::Pixel => event.y / 16.0,
            })
            .sum();
        frame.primary_pressed = buttons.pressed(MouseButton::Left);
        frame.secondary_pressed = buttons.pressed(MouseButton::Right);
        frame.primary_just_pressed = buttons.just_pressed(MouseButton::Left);
        frame.primary_just_released = buttons.just_released(MouseButton::Left);
        if let Ok(window) = windows.single() {
            frame.cursor = window.cursor_position();
            frame.viewport_height = window.height();
        }
    }

    /// toast 落栈（06 §7）：新 toast 清掉驻留的 Refused（驻留至下次命令）。
    fn push_toast(session: &mut DesktopSession, text: String, sticky: bool, ttl_secs: f32) {
        session.toasts.retain(|toast| !toast.sticky);
        session.toasts.push_back(Toast {
            text,
            sticky,
            ttl_secs,
            created: std::time::Instant::now(),
        });
        while session.toasts.len() > 5 {
            session.toasts.pop_front();
        }
    }

    /// 命令结果 → toast（06 §7；键位/手势/UI/BRP 四驱动源共用）。BRP 路径
    /// 的机位名可能滞后一帧（last_camera_view 由 desktop_tick 每帧同步）。
    fn push_outcome_toast(world: &World, session: &mut DesktopSession, outcome: &CommandOutcome) {
        let (text, sticky) = match outcome {
            CommandOutcome::Applied => ("applied".to_string(), false),
            CommandOutcome::Switched { generation, .. } => {
                let zone = world
                    .get_resource::<scenes::CourtyardSettings>()
                    .map(|settings| settings.zone.id().to_string())
                    .unwrap_or_else(|| "?".to_string());
                // 重建跨帧推进：toast 落笔时新机位可能尚未就绪
                // （camera_view 为 None），此时省略机位名。
                let preset = session
                    .last_camera_view
                    .as_ref()
                    .and_then(|view| view.preset.clone());
                let preset = preset.map(|preset| format!(" · {preset}")).unwrap_or_default();
                (
                    format!("switched: zone {zone}{preset} · gen {}", generation.get()),
                    false,
                )
            }
            CommandOutcome::Refused(reason) => (describe(reason), true),
        };
        push_toast(session, text, sticky, 2.0);
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
            character: input.just_pressed(KeyCode::KeyN),
            reset: input.just_pressed(KeyCode::KeyR),
            capture: input.just_pressed(KeyCode::KeyC),
        }
    }

    /// BRP 会话命令入口（bevy-copilot 实时检视/驱动；AGENTS.md「optional
    /// live BRP inspection when exposed」）。与键位同一通道——载荷即
    /// `SessionCommand` 的 serde 形态，经同一个 `controller.command` 执行
    /// （用户路径同源：键位 → translate_keys → 同一入口）。
    fn brp_session_command(
        bevy::prelude::In(params): bevy::prelude::In<Option<serde_json::Value>>,
        world: &mut World,
    ) -> bevy::remote::BrpResult {
        let invalid = |message: String| bevy::remote::BrpError {
            code: bevy::remote::error_codes::INVALID_PARAMS,
            message,
            data: None,
        };
        let value = params.ok_or_else(|| invalid("missing SessionCommand payload".to_string()))?;
        let command: SessionCommand =
            serde_json::from_value(value).map_err(|error| invalid(format!("{error}")))?;
        let outcome = world.resource_scope(|world, mut session: Mut<DesktopSession>| {
            // BRP 与键位/手势/UI 同通道（06 §7/§8）：命令经 journal 录制，
            // 结果同样落 toast。
            let session = &mut *session;
            let outcome =
                session
                    .controller
                    .command_recorded(world, &command, &mut session.journal);
            push_outcome_toast(world, session, &outcome);
            outcome
        });
        serde_json::to_value(format!("{outcome:?}")).map_err(|error| bevy::remote::BrpError {
            code: bevy::remote::error_codes::INTERNAL_ERROR,
            message: format!("{error}"),
            data: None,
        })
    }

    pub fn run(case: &ShowcaseCase, camera_transition: bool) {
        let mut app = App::new();
        // odette 角色台展品档（stage_character=odette）的资产源：指向本地
        // compiled static-v3 目录（gitignored 第三方美术，ADR-0017 政策）。
        // 须在 AssetPlugin（DefaultPlugins）之前注册；目录缺席不注册——
        // 届时切 odette 档加载失败并在 stderr 可见（不做空台降级）。
        const ODETTE_SOURCE_DIR: &str = "assets/local/genshin-odette/compiled/static-v3";
        let odette_dir = std::path::Path::new(ODETTE_SOURCE_DIR)
            .canonicalize()
            .ok()
            .filter(|path| path.is_dir());
        if let Some(odette_dir) = odette_dir {
            use bevy::asset::AssetApp;
            app.register_asset_source(
                "odette",
                bevy::asset::io::AssetSourceBuilder::new(move || {
                    Box::new(bevy::asset::io::file::FileAssetReader::new(odette_dir.clone()))
                }),
            );
        } else {
            eprintln!("[interactive] odette asset source dir missing ({ODETTE_SOURCE_DIR}); stage_character=odette will fail to load");
        }
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
        // §0 默认状态显式落资源（v2：trio/day/hull/100/false/rest/None）——
        // builder/控制器虽能缺省回退，但状态条与 BRP 检视需要资源在场。
        app.insert_resource(scenes::CourtyardSettings::default());
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
                // 与捕获/证据宿主一致（scene-runtime driver 同为 None）：默认
                // TonyMcMapFace 在本 workspace 未开 `tonemapping_luts`，LUT 缺席
                // 时窗口输出为空白；画面语义必须与捕获路径同源（§11.7）。
                bevy::core_pipeline::tonemapping::Tonemapping::None,
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
        let mut controller = controller;
        // 机位补间（06 §8）：交互窗口默认开（0.4s）；冒烟/捕获经
        // `--no-camera-transition` 钉关。命令语义不变——补间是纯视觉插值。
        controller.set_camera_transition(camera_transition.then_some(0.4));
        app.insert_resource(DesktopSession {
            controller,
            window_camera,
            failed: false,
            last_camera_view: None,
            toasts: std::collections::VecDeque::new(),
            presentation: false,
            driver_running: false,
            drag: DragState::default(),
            click_tracker: crate::mousemap::ClickTracker::default(),
            pending_commands: Vec::new(),
            journal: scene_runtime::CommandJournal::new(
                SessionId::new("showcase-interactive").expect("session id"),
                current,
            ),
            tour: None,
            seen_observations: 0,
        });
        // egui 叠加层（05 §11.4/§11.5 + 06 U1–U3）：状态条 + 内容 dock +
        // 参数抽屉 + 键位帮助 + toast。装配参照 Inspector 的模式（主上下文
        // 归专用 Camera2d，避免与场景/离屏相机的自动上下文竞争）。
        app.insert_resource(bevy_egui::EguiGlobalSettings {
            auto_create_primary_context: false,
            ..Default::default()
        });
        app.add_plugins(bevy_egui::EguiPlugin::default());
        app.world_mut().spawn((
            bevy::prelude::Camera2d,
            bevy::prelude::Camera {
                order: 10,
                // 叠加相机不清屏（Default 会清成全局 ClearColor，盖住 order 0
                // 的 3D 窗口相机——整窗只剩暗灰清屏色）。
                clear_color: bevy::prelude::ClearColorConfig::None,
                ..Default::default()
            },
            // 主上下文挂载点（Inspector 同款）：auto_create_primary_context=false
            // 时缺了它 egui 拿不到上下文，叠加层静默不画。
            bevy_egui::PrimaryEguiContext,
        ));
        app.init_resource::<MouseFrame>();
        app.add_systems(
            bevy::prelude::Update,
            (collect_mouse_frame, desktop_tick).chain(),
        );
        app.add_systems(bevy_egui::EguiPrimaryContextPass, overlay_ui);
        // BRP 实时检视（bevy-copilot）：默认 127.0.0.1:15702；反射注册使
        // CourtyardSettings 等资源可经 get-resources 读取；自定义方法
        // showcase/session_command 与键位同一 SessionCommand 通道。
        app.register_type::<scenes::CourtyardSettings>()
            .register_type::<scenes::CourtyardZone>()
            .register_type::<scenes::CourtyardTimeOfDay>()
            .register_type::<scenes::CourtyardOutlineMode>()
            .register_type::<scenes::CourtyardFlagsPose>()
            .register_type::<scenes::CourtyardStageCharacter>();
        app.add_plugins(
            bevy::remote::RemotePlugin::default()
                .with_method_main("showcase/session_command", brp_session_command),
        );
        app.add_plugins(bevy::remote::http::RemoteHttpPlugin::default());
        app.run();
    }

    /// 桌面输入适配：winit KeyCode/鼠标手势 → 控制器命令（唯一的输入翻译
    /// 消费点；业务逻辑在控制器，键位表归 [`crate::keymap`]，手势表归
    /// [`crate::mousemap`]）。
    #[allow(clippy::too_many_lines)]
    fn desktop_tick(world: &mut World) {
        // EguiWantsInput 模式（05 §11.5 调研结论）：egui 面板消费输入的帧
        // 不翻译场景键位/手势（键盘与指针门控分开：指针在面板上时场景手势
        // 不触发，06 §5）。
        let (egui_wants_keyboard, egui_wants_pointer) = world
            .get_resource::<bevy_egui::input::EguiWantsInput>()
            .map(|wants| (wants.wants_keyboard_input(), wants.wants_pointer_input()))
            .unwrap_or((false, false));
        let keys = if egui_wants_keyboard {
            crate::keymap::KeyPresses::default()
        } else {
            let input = world.resource::<ButtonInput<KeyCode>>();
            collect_key_presses(input)
        };
        // 演示模式开关（H 翻转 / Esc 退出；宿主本地 UI 状态，不产生命令、
        // 不进配方、不影响捕获，06 §8）。导览开关（G）同为宿主本地状态。
        let (presentation_toggle, presentation_exit, tour_toggle) = if egui_wants_keyboard {
            (false, false, false)
        } else {
            let input = world.resource::<ButtonInput<KeyCode>>();
            (
                input.just_pressed(KeyCode::KeyH),
                input.just_pressed(KeyCode::Escape),
                input.just_pressed(KeyCode::KeyG),
            )
        };
        let frame = *world.resource::<MouseFrame>();
        let (now, dt) = {
            let time = world.resource::<bevy::prelude::Time>();
            (time.elapsed_secs(), time.delta_secs())
        };
        world.resource_scope(|world, mut session: Mut<DesktopSession>| {
            if presentation_toggle {
                session.presentation = !session.presentation;
            }
            if presentation_exit {
                session.presentation = false;
            }
            // 能力解析（插件 finish 后）作为会话上限提供给控制器（R2；幂等）。
            if let Some(ceiling) = world
                .get_resource::<StylizedResolvedCapabilities>()
                .and_then(|resolved| resolved.resolved.clone().ok().map(|v| v.tier))
            {
                session.controller.set_tier_ceiling(ceiling);
            }
            // 翻译需要会话状态：当前用例、当前机位 preset、当前参数值、
            // 当前相机姿态（手势预夹）。
            let commands = {
                let current_case = session.controller.current().id.as_str().to_string();
                let view = session.controller.camera_view();
                let settings = world
                    .get_resource::<scenes::CourtyardSettings>()
                    .copied()
                    .unwrap_or_default();
                let mut commands = crate::keymap::translate_keys(
                    &keys,
                    &current_case,
                    view.as_ref().and_then(|view| view.preset.as_deref()),
                    settings,
                );
                // 鼠标手势装配（06 §5）：点击/拖拽阈值判定 → mousemap 翻译。
                let mut gestures = crate::mousemap::MouseGestures::default();
                if !egui_wants_pointer {
                    if frame.primary_just_pressed {
                        session.drag.origin = frame.cursor;
                        session.drag.accumulated = Vec2::ZERO;
                        session.drag.dragging = false;
                    }
                    if frame.primary_pressed && session.drag.origin.is_some() {
                        session.drag.accumulated += frame.motion;
                        if session.drag.dragging {
                            if frame.motion != Vec2::ZERO {
                                gestures.orbit_delta = Some((frame.motion.x, frame.motion.y));
                            }
                        } else if session.drag.accumulated.length()
                            > crate::mousemap::CLICK_MAX_DISTANCE_PX
                        {
                            // 越过阈值转拖拽：补发积蓄位移（无死区跳变）。
                            session.drag.dragging = true;
                            gestures.orbit_delta =
                                Some((session.drag.accumulated.x, session.drag.accumulated.y));
                        }
                    }
                    if frame.primary_just_released {
                        if session.drag.origin.is_some()
                            && !session.drag.dragging
                            && let Some(position) = frame.cursor
                        {
                            if session.click_tracker.click(now, position.x, position.y) {
                                gestures.double_click = true;
                            }
                        }
                        session.drag = DragState::default();
                    }
                    if frame.secondary_pressed && frame.motion != Vec2::ZERO {
                        gestures.pan_delta = Some((frame.motion.x, frame.motion.y));
                    }
                    gestures.scroll = frame.scroll;
                }
                if let Some(view) = &view {
                    commands.extend(crate::mousemap::translate_mouse(
                        &gestures,
                        current_case == "courtyard",
                        view.pitch,
                        view.distance,
                        frame.viewport_height,
                    ));
                }
                commands
            };
            // UI 队列命令（dock/参数抽屉，06 §2：UI 只是驱动源）。
            let mut commands = commands;
            let ui_commands = std::mem::take(&mut session.pending_commands);
            // 任意导航输入退出导览（06 §8）：本帧有非导览来源的命令即退出。
            // 导览自身发出的命令不在此列（下方单独通道）。
            if session.tour.is_some() && (!commands.is_empty() || !ui_commands.is_empty()) {
                session.tour = None;
                push_toast(&mut session, "tour off (manual input)".to_string(), false, 2.0);
            }
            // 导览开关（G）：开启 = 立即发首站命令；关闭 = 提示。
            let mut tour_commands = Vec::new();
            if tour_toggle {
                if session.tour.take().is_some() {
                    push_toast(&mut session, "tour off".to_string(), false, 2.0);
                } else if let Some((progress, command)) =
                    crate::tour::tour_start(crate::tour::COURTYARD_TOUR)
                {
                    session.tour = Some(progress);
                    tour_commands.push(command);
                }
            } else if session.tour.is_some() && !session.driver_running {
                // 导览推进（06 §8）：driver 落定时驻留计时；到期发下一站。
                let tour = session.tour.as_mut().expect("tour active");
                match crate::tour::tour_advance(tour, crate::tour::COURTYARD_TOUR, dt) {
                    Some(crate::tour::TourStep::Focus(zone)) => {
                        tour_commands.push(crate::tour::zone_command(zone));
                    }
                    Some(crate::tour::TourStep::Finished) => {
                        session.tour = None;
                        push_toast(
                            &mut session,
                            "tour finished (resting on trio)".to_string(),
                            false,
                            2.0,
                        );
                    }
                    None => {}
                }
            }
            commands.extend(ui_commands);
            commands.extend(tour_commands);
            let mut outcomes = Vec::new();
            for command in commands.drain(..) {
                // 会话级 journal 录制（06 §8，U3）：接受先记 Tentative、结果
                // 落笔转 Confirmed；四驱动源同通道。
                let outcome = {
                    let session = &mut *session;
                    session
                        .controller
                        .command_recorded(world, &command, &mut session.journal)
                };
                match &outcome {
                    CommandOutcome::Switched { case, generation } => {
                        eprintln!("[interactive] switched to {case} (generation {generation})");
                    }
                    CommandOutcome::Applied => {
                        eprintln!("[interactive] applied");
                    }
                    CommandOutcome::Refused(reason) => {
                        eprintln!("[interactive] refused: {}", describe(reason));
                    }
                }
                outcomes.push(outcome);
            }
            let status = session.controller.update(world);
            session.driver_running = matches!(status, DriverStatus::Running);
            if !session.failed
                && let DriverStatus::Failed(error) = status
            {
                session.failed = true;
                eprintln!("[interactive] driver failed: {error}");
            }
            // 机位补间推进（06 §8；补间关闭时为空操作）。
            session.controller.advance_camera_transition(world, dt);
            // 窗口相机统一（§11.3 第一条）：每帧从控制器相机状态同步
            // Transform；相机尚未构建（首帧 Build 前）时保持占位机位。
            session.last_camera_view = session.controller.camera_view();
            if let Some(view) = session.last_camera_view.clone()
                && let Some(mut transform) = world.get_mut::<Transform>(session.window_camera)
            {
                *transform = Transform::from_translation(view.eye).looking_at(view.look, Vec3::Y);
            }
            // 命令结果 → toast（06 §7；含切换后的机位名/分区，数据源 =
            // 刚同步的 last_camera_view 与 CourtyardSettings，与状态条同源）。
            for outcome in outcomes {
                push_outcome_toast(world, &mut session, &outcome);
            }
            // U3 Capture 落盘（06 §9）：新增观测回执 → PNG 写会话输出目录
            // + toast 显示路径（浏览快照，非判定证据）。
            let receipts = session.controller.driver().observations();
            if receipts.len() > session.seen_observations {
                let new: Vec<scene_runtime::ObservationReceipt> =
                    receipts[session.seen_observations..].to_vec();
                session.seen_observations = receipts.len();
                for receipt in &new {
                    match write_capture_receipt(receipt) {
                        Some(path) => {
                            eprintln!("[interactive] snapshot saved: {path}");
                            push_toast(
                                &mut session,
                                format!("snapshot saved: {path} (browsing snapshot, not golden evidence)"),
                                false,
                                6.0,
                            );
                        }
                        None => {
                            push_toast(
                                &mut session,
                                "snapshot failed: cannot write output".to_string(),
                                true,
                                2.0,
                            );
                        }
                    }
                }
            }
        });
    }

    /// 观测回执 → PNG（会话输出目录，gitignored；文件名带观测名/代次/
    /// 实验 tick/时刻戳，06 §9）。
    fn write_capture_receipt(receipt: &scene_runtime::ObservationReceipt) -> Option<String> {
        let dir = std::path::Path::new("target/stylized/showcase/interactive");
        std::fs::create_dir_all(dir).ok()?;
        let epoch_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_millis();
        let path = dir.join(format!(
            "{}-g{}-t{}-{epoch_ms}.png",
            receipt.observation,
            receipt.generation.get(),
            receipt.experiment_tick
        ));
        let png = golden_harness::encode_png(&receipt.image).ok()?;
        std::fs::write(&path, png).ok()?;
        Some(path.display().to_string())
    }

    /// egui 叠加层（05 §11.4/§11.5 PC 行 + 06 U1/U2/U3）：左下常驻状态条
    /// （机位/时刻/描边/post/crowd/flags/char + building 指示）+ 底部分区
    /// dock（§4.1）+ 右侧参数抽屉（§6，schema 驱动）
    /// + 键位帮助（默认折叠）+ 右下 toast 堆叠（§7）。
    /// 演示模式（session.presentation，H/Esc）下全部不画（06 §8）。
    /// 数据源：DesktopSession.last_camera_view（desktop_tick 每帧缓存）、
    /// CourtyardSettings 资源、StylizedResolvedCapabilities（tier 门控呈现——
    /// hybrid 不足 Standard 时置灰并带原因，§11.4）。UI 语言与 Inspector
    /// 面板一致（英文）。
    fn overlay_ui(
        mut contexts: bevy_egui::EguiContexts,
        mut session: ResMut<DesktopSession>,
        settings: Option<bevy::prelude::Res<scenes::CourtyardSettings>>,
        caps: Option<bevy::prelude::Res<StylizedResolvedCapabilities>>,
    ) -> bevy::prelude::Result {
        use bevy_egui::egui;
        let Ok(ctx) = contexts.ctx_mut() else {
            return Ok(());
        };
        if session.presentation {
            return Ok(());
        }
        ctx.set_visuals(egui::Visuals::dark());
        let settings = settings.as_deref().copied().unwrap_or_default();
        let zone = settings.zone.id();
        let preset = session
            .last_camera_view
            .as_ref()
            .and_then(|view| view.preset.clone())
            .unwrap_or_else(|| "—".to_string());
        let time = match settings.time_of_day {
            scenes::CourtyardTimeOfDay::Day => "day",
            scenes::CourtyardTimeOfDay::Dusk => "dusk",
            scenes::CourtyardTimeOfDay::Night => "night",
        };
        let flags = match settings.flags {
            scenes::CourtyardFlagsPose::Rest => "rest",
            scenes::CourtyardFlagsPose::Posed => "posed",
        };
        let character = match settings.stage_character {
            scenes::CourtyardStageCharacter::Fixture => "fixture",
            scenes::CourtyardStageCharacter::Odette => "odette",
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
                    "zone {zone}  ·  {preset}  ·  {time}  ·  crowd {}  ·  flags {flags}  ·  char {character}  ·  post {}",
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
                // 构建指示（06 §7）：DriverStatus::Running 期间如实呈现，
                // 回到稳态即消失。
                if session.driver_running {
                    ui.monospace(format!(
                        "building · gen {}",
                        session.controller.driver().generation().get()
                    ));
                }
            });

        // 内容 dock（06 §4.1，底部常驻）：9 内容芯片；点击
        // = SetParameter(zone)，与 ←/→ 完全同命令（经 pending 队列进
        // desktop_tick 的 journal 录制通道）。Legend 退役（06 §3）——分区
        // 说明下移为芯片 tooltip。
        egui::Window::new("Zones")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -8.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (zone_id, desc) in ZONE_DESCRIPTIONS {
                        let active = zone == zone_id;
                        if ui
                            .selectable_label(active, zone_id)
                            .on_hover_text(desc)
                            .clicked()
                            && !active
                        {
                            session
                                .pending_commands
                                .push(crate::tour::zone_command(zone_id));
                        }
                    }
                });
            });

        // 参数抽屉（06 §6，右侧）：schema 驱动——控件从 catalog 参数 schema
        // 生成（选型 = param_panel::widget_kind），分组/预设芯片读 catalog
        // ui 元数据；tier 置灰经控制器 evaluate 同一判定（结构化原因进
        // tooltip）。控件操作只发 SetParameter（pending 队列；校验失败走
        // §7 toast）。
        egui::Window::new("Parameters")
            .anchor(egui::Align2::RIGHT_TOP, [-8.0, 8.0])
            .default_open(true)
            .collapsible(true)
            .show(ctx, |ui| {
                ui.monospace(format!(
                    "tier: {}",
                    if standard { "Standard" } else { "Portable" }
                ));
                ui.separator();
                let entry = scenario_model::catalog_entry("courtyard").expect("catalog entry");
                for group in ["Scene", "Rendering", "Content"] {
                    ui.label(egui::RichText::new(group).strong());
                    for spec in entry
                        .parameters
                        .iter()
                        .filter(|spec| spec.ui.as_ref().is_some_and(|ui| ui.group == group))
                    {
                        parameter_widget(ui, &mut session, spec, &settings);
                    }
                    ui.separator();
                }
                // Capture 按钮（06 §9；同 C 键命令）。
                if ui
                    .button("Capture snapshot (C)")
                    .on_hover_text("browsing snapshot, not golden evidence")
                    .clicked()
                {
                    session.pending_commands.push(SessionCommand::Capture);
                }
            });

        // 键位帮助（默认折叠）。
        egui::Window::new("Keys")
            .anchor(egui::Align2::LEFT_TOP, [8.0, 8.0])
            .default_open(false)
            .collapsible(true)
            .show(ctx, |ui| {
                for line in [
                    "Left/Right   zone cycle (courtyard) / case cycle (others)",
                    ", / .      prev / next case",
                    "Up/Down    time of day: day / dusk / night",
                    "O          outline mode: off / hull / hybrid",
                    "P          post process on/off",
                    "1 / 2 / 3  crowd: 0 / 100 / 1000",
                    "Space      flags rest/posed",
                    "N          stage character: fixture / odette (interactive display)",
                    "R          reset view to preset",
                    "C          capture snapshot (browsing snapshot, not golden evidence)",
                    "G          guided tour (scripted walk; any navigation input exits)",
                    "Drag (L)   orbit  ·  drag (R) pan  ·  wheel zoom",
                    "Double-click (L)  reset view",
                    "H          presentation mode (hide UI; Esc or H exits)",
                ] {
                    ui.monospace(line);
                }
            });

        // 右下 toast 堆叠（06 §7）：确认按 ttl 消退（快照路径 6s，其余 2s）；
        // Refused 琥珀色驻留（点击 × 或下次命令关闭）。
        let now = std::time::Instant::now();
        session.toasts.retain(|toast| {
            toast.sticky || now.duration_since(toast.created).as_secs_f32() < toast.ttl_secs
        });
        let mut close = None;
        egui::Area::new(egui::Id::new("command-toasts"))
            .anchor(egui::Align2::RIGHT_BOTTOM, [-8.0, -8.0])
            .show(ctx, |ui| {
                for (index, toast) in session.toasts.iter().enumerate() {
                    let color = if toast.sticky {
                        egui::Color32::from_rgb(0x6b, 0x54, 0x10)
                    } else {
                        egui::Color32::from_rgb(0x14, 0x46, 0x1e)
                    };
                    egui::Frame::window(&ui.style())
                        .fill(color)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(&toast.text);
                                if toast.sticky && ui.small_button("×").clicked() {
                                    close = Some(index);
                                }
                            });
                        });
                }
            });
        if let Some(index) = close {
            session.toasts.remove(index);
        }
        Ok(())
    }

    /// 分区说明（06 §3：原 Legend 文案，退役后下移为 dock 芯片 tooltip）。
    const ZONE_DESCRIPTIONS: [(&str, &str); 14] = [
        (
            "trio",
            "center trio: face SDF threshold 0.2/default/0.8 · hair specular off/on · metal matcap off/on",
        ),
        ("p1", "P1 ramp: analytic ramp vs texture ramp rows 0/1/2"),
        ("p2", "P2 outline: hull width 0 / 2 / 8 px"),
        ("p3", "P3 hybrid: hull + screen-space internal edges (Standard+)"),
        ("p4", "P4 lighting: key light override none / +Z / -Z"),
        ("p5", "P5 flags: skinned flag strips · rest/posed"),
        ("lod", "LOD axis: near -> mid -> far -> very far (4 bands)"),
        ("stage", "stage: NPR character (reference GLB / odette)"),
        ("lamps", "lamp ring: instanced emissive lamps · crowd 0/100/1000"),
        ("rim", "rim light: off / strong white / soft warm"),
        ("shadow", "shadow shape: default / hard high / soft low-floor"),
        ("palette", "character palette: default / warm shadow / cool shadow"),
        ("metal", "hero metal: matcap base / 3-tone / + spec ramp"),
        ("cutout", "alpha cutout: checkerboard cards (mask + double-sided)"),
    ];

    /// 单个参数控件（06 §6 表）：选型 = [`crate::param_panel::widget_kind`]
    /// （catalog `ParameterKind` 单一源）；当前值 =
    /// [`crate::param_panel::courtyard_value`]；tier/合法性门控 = 控制器
    /// `evaluate` 同一判定（拒绝 → 置灰 + 结构化原因 tooltip）。控件操作
    /// 只产出 SetParameter 命令（汇进 pending 队列，次帧经 journal 录制
    /// 通道执行；校验失败走 §7 toast，控件下帧回弹到当前值）。
    fn parameter_widget(
        ui: &mut bevy_egui::egui::Ui,
        session: &mut DesktopSession,
        spec: &scenario_model::catalog::ParameterSpec,
        settings: &scenes::CourtyardSettings,
    ) {
        use bevy_egui::egui;
        use scenario_model::catalog::ParameterKind;
        let key = spec.name.as_str();
        let current = crate::param_panel::courtyard_value(settings, key);
        let mut issued: Vec<SessionCommand> = Vec::new();
        // tier/合法性门控（06 §6）：候选值经控制器 evaluate 判定。
        let gate = |value: &serde_json::Value| -> Result<(), String> {
            match session.controller.evaluate(&SessionCommand::SetParameter {
                key: key.to_string(),
                value: value.clone(),
            }) {
                scene_runtime::CommandPlan::Refused(reason) => Err(describe(&reason)),
                _ => Ok(()),
            }
        };
        ui.horizontal(|ui| {
            ui.monospace(key).on_hover_text(&spec.summary);
            match crate::param_panel::widget_kind(&spec.kind) {
                crate::param_panel::WidgetKind::Segmented => {
                    let ParameterKind::Enum { options } = &spec.kind else {
                        return;
                    };
                    for option in options {
                        let value = serde_json::json!(option);
                        let active = current == value;
                        match gate(&value) {
                            Ok(()) => {
                                if ui.selectable_label(active, option).clicked() && !active {
                                    issued.push(SessionCommand::SetParameter {
                                        key: key.to_string(),
                                        value,
                                    });
                                }
                            }
                            Err(reason) => {
                                ui.add_enabled(
                                    false,
                                    egui::Button::new(option).selected(active),
                                )
                                .on_hover_text(reason);
                            }
                        }
                    }
                }
                crate::param_panel::WidgetKind::Combo => {
                    let ParameterKind::Enum { options } = &spec.kind else {
                        return;
                    };
                    let selected = current.as_str().unwrap_or("?").to_string();
                    egui::ComboBox::from_id_salt(key)
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            for option in options {
                                let value = serde_json::json!(option);
                                if ui.selectable_label(current == value, option).clicked()
                                    && current != value
                                {
                                    issued.push(SessionCommand::SetParameter {
                                        key: key.to_string(),
                                        value,
                                    });
                                }
                            }
                        });
                }
                crate::param_panel::WidgetKind::StepSlider => {
                    let ParameterKind::U32 { max } = &spec.kind else {
                        return;
                    };
                    let mut value = current.as_u64().unwrap_or(0) as u32;
                    // 步进滑杆：提交在拖拽结束时（crowd 是内容重建语义，
                    // 逐帧提交 = 重建风暴）。
                    if ui
                        .add(egui::Slider::new(&mut value, 0..=*max))
                        .drag_stopped()
                        && value as u64 != current.as_u64().unwrap_or(0)
                    {
                        issued.push(SessionCommand::SetParameter {
                            key: key.to_string(),
                            value: serde_json::json!(value),
                        });
                    }
                    // 快捷预设芯片（catalog `ui.presets`，06 §16 D5）。
                    if let Some(presets) = spec.ui.as_ref().and_then(|ui| ui.presets.as_ref()) {
                        for preset in presets {
                            let active = current == *preset;
                            let label = preset
                                .as_u64()
                                .map(|value| value.to_string())
                                .unwrap_or_default();
                            if ui.selectable_label(active, label).clicked() && !active {
                                issued.push(SessionCommand::SetParameter {
                                    key: key.to_string(),
                                    value: preset.clone(),
                                });
                            }
                        }
                    }
                }
                crate::param_panel::WidgetKind::Toggle => {
                    let mut value = current.as_bool().unwrap_or(false);
                    if ui.checkbox(&mut value, "").changed() {
                        issued.push(SessionCommand::SetParameter {
                            key: key.to_string(),
                            value: serde_json::json!(value),
                        });
                    }
                }
                crate::param_panel::WidgetKind::Slider => {
                    let ParameterKind::F32 { min, max } = &spec.kind else {
                        return;
                    };
                    let mut value = current.as_f64().unwrap_or(0.0) as f32;
                    if ui
                        .add(egui::Slider::new(&mut value, *min..=*max))
                        .drag_stopped()
                    {
                        issued.push(SessionCommand::SetParameter {
                            key: key.to_string(),
                            value: serde_json::json!(value),
                        });
                    }
                }
                crate::param_panel::WidgetKind::OptionalSlider => {
                    let mut auto = current.is_null();
                    // 「自动」勾选（null = 回退默认，06 §6）。
                    if ui.checkbox(&mut auto, "auto").changed() {
                        issued.push(SessionCommand::SetParameter {
                            key: key.to_string(),
                            value: if auto {
                                serde_json::Value::Null
                            } else {
                                serde_json::json!(0.5)
                            },
                        });
                    }
                    let mut value = current.as_f64().unwrap_or(0.5) as f32;
                    if ui
                        .add_enabled(!auto, egui::Slider::new(&mut value, 0.0..=1.0))
                        .drag_stopped()
                    {
                        issued.push(SessionCommand::SetParameter {
                            key: key.to_string(),
                            value: serde_json::json!(value),
                        });
                    }
                }
            }
        });
        session.pending_commands.extend(issued);
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

        // W3：增量更新停留在 Done、不重跑动作图——观测只在初始构建产生；
        // 最终捕获经显式重跑捕获 profile（Done 稳态的既有入口）。
        let receipts = controller.driver().observations();
        assert_eq!(
            receipts.len(),
            1,
            "incremental updates do not re-run the action graph"
        );
        controller
            .driver_mut()
            .run_actions(capture_profile_graph())
            .expect("re-run capture profile from Done");
        run_to_completion(&mut app, controller.driver_mut(), 700).expect("final capture");
        let receipts = controller.driver().observations();
        let post_rebuild = receipts.last().expect("final observation");
        assert_eq!(post_rebuild.generation.get(), 2);
        assert_eq!(
            post_rebuild.image.data, single_run.data,
            "post-rebuild capture must equal the single-run capture byte-for-byte"
        );
    }

    /// W3 等价性证据（GPU-gated）：交互式增量装卸（zone 巡场 + live 参数
    /// + lamps 桶重装）回到默认状态后，捕获与单次运行捕获逐字节一致，
    /// 实体/资产计数回到首建基线——增量路径与全量构建的世界状态不分叉。
    #[test]
    #[ignore = "requires explicit GPU execution"]
    fn incremental_zone_walk_preserves_capture_identity() {
        let Some(resources) = capture::probe_render_resources() else {
            eprintln!("incremental zone walk: no GPU adapter available; skipping");
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
            SessionId::new("showcase-incremental").expect("session id"),
            binding_of("courtyard"),
            capture_profile_graph(),
            limits,
        )
        .expect("plan");
        let mut controller = SessionController::from_catalog(driver, 0, default_view_descriptor())
            .expect("controller");
        controller.set_tier_ceiling(StylizedExecutionTier::Standard);
        run_to_completion(&mut app, controller.driver_mut(), 700).expect("initial build");
        for _ in 0..8 {
            app.update();
        }
        let first_build = SceneCounts::capture(app.world());
        let observation_camera = controller.driver().observation_entity();

        // 巡场 + live 参数混合命令序列（zone 增量装卸、lamps 桶重装、
        // flags/face_threshold live、post/outline 附件 live）；全部经命令
        // 通道（与交互宿主同路径），每条后泵送数帧排空异步工作。
        let set = |key: &str, value: serde_json::Value| SessionCommand::SetParameter {
            key: key.to_string(),
            value,
        };
        let generation_before = controller.driver().generation().get();
        let commands = [
            set("zone", serde_json::json!("p1")),
            set("zone", serde_json::json!("p5")),
            set("flags", serde_json::json!("posed")), // live（p5 在场）
            set("flags", serde_json::json!("rest")),
            set("zone", serde_json::json!("lamps")),
            set("crowd", serde_json::json!(1000)), // lamps 桶重装
            set("crowd", serde_json::json!(100)),
            set("zone", serde_json::json!("p3")),
            set("zone", serde_json::json!("trio")),
            set("post", serde_json::json!(true)), // 附件 live
            set("post", serde_json::json!(false)),
            set("outline_mode", serde_json::json!("off")),
            set("outline_mode", serde_json::json!("hull")),
            set("trio.face.0.face_threshold", serde_json::json!(0.5)), // live
            set("trio.face.0.face_threshold", serde_json::Value::Null),
        ];
        for command in &commands {
            let outcome = controller.command(app.world_mut(), command);
            assert!(
                !matches!(outcome, CommandOutcome::Refused(_)),
                "{command:?} refused: {outcome:?}"
            );
            for _ in 0..4 {
                app.update();
            }
        }
        // 代次语义：内容命令（zone/crowd）代次 +1，live 命令不换代次。
        assert_eq!(
            controller.driver().generation().get(),
            generation_before + 7,
            "5 zone switches + 2 crowd reloads bump generation; live params do not"
        );
        // 观测相机实体全程不重建（W3：相机常驻）。
        assert_eq!(
            controller.driver().observation_entity(),
            observation_camera,
            "observation camera persists across incremental updates"
        );
        // 排空后逐项对账：计数回到首建基线（无实体/资产漂移）。
        for _ in 0..32 {
            app.update();
        }
        let after = SceneCounts::capture(app.world());
        assert_eq!(
            after, first_build,
            "incremental zone walk must return to the first-build counts"
        );
        // 最终捕获（同一捕获 profile：22 settle + observe）与单次运行逐字节一致。
        controller
            .driver_mut()
            .run_actions(capture_profile_graph())
            .expect("re-run capture profile from Done");
        run_to_completion(&mut app, controller.driver_mut(), 700).expect("final capture");
        let receipts = controller.driver().observations();
        let final_capture = receipts.last().expect("final observation");
        assert_eq!(
            final_capture.image.data, single_run.data,
            "capture after the incremental zone walk must equal the fresh-build capture byte-for-byte"
        );
    }
}

#[cfg(feature = "inspector")]
pub mod inspector;

pub mod keymap;

pub mod mousemap;

pub mod param_panel;

pub mod tour;

pub mod capture_spec;
