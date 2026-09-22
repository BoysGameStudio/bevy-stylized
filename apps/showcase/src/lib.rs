//! Showcase 框架（It-9a）：17 个固定用例（docs/showcase/01 关卡表 ×
//! release gate §Showcase）。
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
    app.add_plugins(StylizedPlugin::default());
    capture::install(&mut app);
    configure(&mut app);
    app.finish();
    app.cleanup();
    app
}

/// 通用 view 默认值（昼间 + 斜上 key light；交互会话的视图基线）。
fn default_view_descriptor() -> StylizedViewDescriptor {
    StylizedViewDescriptor {
        key_light_direction: Vec3::new(0.5, 0.8, 0.6).normalize(),
        key_light_color: LinearRgba::WHITE,
        exposure: 1.0,
        day_night_factor: 1.0,
        ..StylizedViewDescriptor::default()
    }
}

fn default_view(world: &mut bevy::prelude::World) {
    *world.resource_mut::<StylizedViewSettings>() = StylizedViewSettings(default_view_descriptor());
}

/// 捕获 profile 的动作图：`fixed_step{22} → observe`（帧语义见 crate 文档）。
fn capture_profile_graph() -> ActionGraph {
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
    let asset_root = case.asset_root.unwrap_or("assets");
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

/// 全部 17 个用例（顺序固定，与 `labs/scenes` 绑定表/catalog 一致）。
///
/// 构建器与视图 preset 来自 `labs/scenes` 注册表；宿主侧只补充输出绑定
/// 元数据（reference 的资产根、outline-advanced 的 tier 请求）。
pub fn all_cases() -> Vec<ShowcaseCase> {
    registry::bindings()
        .into_iter()
        .map(|binding| {
            let (asset_root, tier) = match binding.id {
                "reference" => (
                    Some(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../fixtures/synthetic/reference-scene"
                    )),
                    None,
                ),
                // screen-space 描边是 Standard+ 特性。
                "outline-advanced" => (None, Some(StylizedExecutionTier::Standard)),
                _ => (None, None),
            };
            ShowcaseCase {
                name: binding.id,
                build: binding.build,
                view: binding.view.map(|(_, preset)| preset),
                asset_root,
                tier,
            }
        })
        .collect()
}

/// 交互模式（开窗）。仅在 `interactive` feature（winit）下可用；
/// 默认构建为 headless-only（winit 不进入测试线程）。
///
/// 同一 App/Device 会话内经 CaseDriver 切换用例（←/→ 方向键循环）：
/// 内容/相机配方/view 可合法在会话内更换；tier 上调（已解析能力不能
/// 原地增加，R2）与资产根变更（启动时固定）以结构化信息拒绝并保留
/// 当前场景。严格 GPU gate 仍走每用例独立进程（R4），本模式只服务
/// 交互与明确的生命周期测试。
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

    /// 桌面适配层状态：控制器（平台无关）+ 失败记录（宿主报告）。
    #[derive(bevy::prelude::Resource)]
    struct DesktopSession {
        controller: SessionController,
        failed: bool,
    }

    pub fn run(case: &ShowcaseCase) {
        let mut app = App::new();
        app.add_plugins(DefaultPlugins);
        app.add_plugins(StylizedPlugin::default());
        capture::install(&mut app);
        readiness::install(&mut app);
        default_view(app.world_mut());
        if let Some(view) = case.view {
            let mut settings = app.world_mut().resource_mut::<StylizedViewSettings>();
            view(&mut settings.0);
        }
        // 主窗口相机（宿主所有，在账本基线之前创建，不随场景切换退役）。
        app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 2.0, 8.0).looking_at(Vec3::Y, Vec3::Y),
        ));
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
            failed: false,
        });
        app.add_systems(bevy::prelude::Update, desktop_tick);
        app.run();
    }

    /// 桌面输入适配：winit KeyCode → 控制器命令（唯一的输入翻译点；
    /// 业务逻辑在控制器）。
    fn desktop_tick(world: &mut World) {
        let commands = {
            let input = world.resource::<ButtonInput<KeyCode>>();
            let mut commands = Vec::new();
            if input.just_pressed(KeyCode::ArrowRight) {
                commands.push(SessionCommand::NextCase);
            }
            if input.just_pressed(KeyCode::ArrowLeft) {
                commands.push(SessionCommand::PrevCase);
            }
            commands
        };
        world.resource_scope(|world, mut session: Mut<DesktopSession>| {
            // 能力解析（插件 finish 后）作为会话上限提供给控制器（R2；幂等）。
            if let Some(ceiling) = world
                .get_resource::<StylizedResolvedCapabilities>()
                .and_then(|resolved| resolved.resolved.clone().ok().map(|v| v.tier))
            {
                session.controller.set_tier_ceiling(ceiling);
            }
            for command in commands {
                match session.controller.command(world, &command) {
                    CommandOutcome::Switched { case, generation } => {
                        eprintln!("[interactive] switched to {case} (generation {generation})");
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
        });
    }

    fn describe(reason: &RefusalReason) -> String {
        match reason {
            RefusalReason::TierRequired { required, resolved } => format!(
                "case requires {required:?} but the session resolved {resolved:?}; a new session is required"
            ),
            RefusalReason::AssetRootFixed(id) => format!(
                "{id} needs a fixed asset root; the session asset root is fixed at startup (a new session is required)"
            ),
            RefusalReason::UnknownScene(id) => format!("unknown case {id}"),
            RefusalReason::DriverBusy(message) => format!("driver rejected the switch: {message}"),
        }
    }
}

#[cfg(test)]
mod driver_adoption_tests {
    //! CaseDriver/SessionController 采用证据（GPU-gated）：交互切换机制
    //! （A→B→A 同会话）的 headless 验证——经共享控制器的命令接口切换，
    //! 资源计数回到首次构建基线，切换回来的 A 捕获与单次运行捕获逐字节一致。
    use super::*;
    use scenario_model::ident::SceneId;
    use scene_runtime::{CommandOutcome, SceneCounts, SessionCommand, SessionController};

    fn binding_of(name: &str) -> scenes::registry::SceneBinding {
        registry::bindings()
            .into_iter()
            .find(|binding| binding.id == name)
            .expect("registered scene")
    }

    #[test]
    #[ignore = "requires explicit GPU execution"]
    fn scene_switch_a_b_a_preserves_capture_identity() {
        let Some(resources) = capture::probe_render_resources() else {
            eprintln!("showcase driver adoption: no GPU adapter available; skipping");
            return;
        };
        let cases = all_cases();
        let outline = cases
            .iter()
            .find(|case| case.name == "outline")
            .expect("case");
        let single_run = capture_case(outline, resources.clone());

        let mut app = headless_app(resources, "assets", None);
        default_view(app.world_mut());
        let limits = DriverLimits {
            implicit_readiness: false,
            ..DriverLimits::default()
        };
        let driver = CaseDriver::new(
            app.world(),
            SessionId::new("showcase-interactive").expect("session id"),
            binding_of("outline"),
            capture_profile_graph(),
            limits,
        )
        .expect("plan");
        let mut controller = SessionController::from_catalog(
            driver,
            6, // outline
            default_view_descriptor(),
        )
        .expect("controller");
        controller.set_tier_ceiling(StylizedExecutionTier::Standard);
        run_to_completion(&mut app, controller.driver_mut(), 700).expect("A completes");
        let first_build = SceneCounts::capture(app.world());
        assert_eq!(controller.driver().generation().get(), 0);

        // A→B：经控制器命令接口（与桌面 ←/→ 同一业务逻辑）。
        let select_b = SessionCommand::SelectCase(SceneId::new("animation").expect("id"));
        assert!(matches!(
            controller.command(app.world_mut(), &select_b),
            CommandOutcome::Switched { .. }
        ));
        run_to_completion(&mut app, controller.driver_mut(), 700).expect("B completes");
        // B→A。
        let select_a = SessionCommand::SelectCase(SceneId::new("outline").expect("id"));
        assert!(matches!(
            controller.command(app.world_mut(), &select_a),
            CommandOutcome::Switched { .. }
        ));
        run_to_completion(&mut app, controller.driver_mut(), 700).expect("A completes again");
        assert_eq!(controller.driver().generation().get(), 2);
        // 排空若干帧（资产句柄释放的处理窗口）后逐项对账。
        for _ in 0..8 {
            app.update();
        }
        assert_eq!(
            SceneCounts::capture(app.world()),
            first_build,
            "A→B→A must return to the first-build counts (no leaked entities/cameras/assets)"
        );

        let receipts = controller.driver().observations();
        assert_eq!(receipts.len(), 3, "each generation observes once");
        let post_switch = &receipts[2];
        assert_eq!(post_switch.generation.get(), 2);
        assert_eq!(
            post_switch.image.data, single_run.data,
            "post-switch A capture must equal the single-run A capture byte-for-byte"
        );
    }
}

#[cfg(feature = "inspector")]
pub mod inspector;
