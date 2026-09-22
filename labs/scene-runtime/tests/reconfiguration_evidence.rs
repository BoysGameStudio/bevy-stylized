//! 运行时重配置 GPU 证据（ADR-0030；Metal，GPU-gated）：
//!
//! 1. Portable 会话捕获 `outline` → 经控制器切换到声明 Standard 的
//!    `outline-advanced`：控制器先尝试会话内重配置（R2），设备已具备能力
//!    则直接切换；捕获与同一内容在"新鲜 Standard 会话"中的捕获逐字节
//!    一致，并与 Portable 会话的 hull-only 回落渲染显著不同（screen
//!    outline 内部边缘出现）；
//! 2. 断言未启用设备特性（Stage A 未请求 Bindless）的重配置请求被结构化
//!    拒绝，会话资源与像素逐字节不变。

use bevy::{
    DefaultPlugins,
    app::PluginGroup,
    asset::{AssetMetaCheck, AssetPlugin},
    prelude::App,
    render::{
        RenderPlugin,
        settings::{RenderCreation, RenderResources},
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::{
    StylizedCapabilityConfig, StylizedExecutionTier, StylizedGpuFeature, StylizedPlugin,
    StylizedReconfigError, StylizedReconfigGeneration, StylizedReconfigurationExt,
    StylizedResolvedCapabilities, StylizedViewDescriptor, StylizedViewSettings,
};
use golden_harness::{CapturedImage, capture, readiness};
use scenario_model::action::{ActionGraph, ActionKind, ActionNode, Step};
use scenario_model::canon::CanonicalU64;
use scenario_model::ident::{ActionId, SceneId, SessionId};
use scene_runtime::{
    CaseDriver, CommandOutcome, DriverLimits, SessionCommand, SessionController, run_to_completion,
};

fn evidence_app(resources: RenderResources) -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .build()
            .set(AssetPlugin {
                file_path: "assets".to_string(),
                meta_check: AssetMetaCheck::Never,
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
    readiness::install(&mut app);
    app.finish();
    app.cleanup();
    app
}

fn binding(id: &str) -> scenes::registry::SceneBinding {
    scenes::registry::bindings()
        .into_iter()
        .find(|binding| binding.id == id)
        .expect("registered scene")
}

fn capture_profile_graph() -> ActionGraph {
    ActionGraph {
        nodes: vec![
            ActionNode {
                id: ActionId::new("settle").expect("id"),
                tick: CanonicalU64(0),
                depends_on: vec![],
                writes: vec![],
                kind: ActionKind::FixedStep {
                    ticks: CanonicalU64(22),
                    step: Step::new(1, 60),
                },
            },
            ActionNode {
                id: ActionId::new("shot").expect("id"),
                tick: CanonicalU64(0),
                depends_on: vec![ActionId::new("settle").expect("id")],
                writes: vec![],
                kind: ActionKind::Observe {
                    observation: "capture".to_string(),
                    view: None,
                },
            },
        ],
    }
}

fn capture_once(app: &mut App, driver: &mut CaseDriver) -> CapturedImage {
    driver
        .run_actions(capture_profile_graph())
        .expect("capture plan");
    run_to_completion(app, driver, 700).expect("capture completes");
    driver
        .drain_observations()
        .pop()
        .expect("one observation")
        .image
}

fn resolved_tier(app: &App) -> StylizedExecutionTier {
    app.world()
        .resource::<StylizedResolvedCapabilities>()
        .resolved
        .as_ref()
        .expect("resolved")
        .tier
}

/// 控制器切换后的会话视图状态 = 基线重置 + 用例 preset；直接驱动路径
/// （无控制器）须复现同一状态才能做像素比较。
fn reset_view_to_session_baseline(app: &mut App) {
    *app.world_mut().resource_mut::<StylizedViewSettings>() =
        StylizedViewSettings(StylizedViewDescriptor::default());
}

fn differing_pixels(a: &CapturedImage, b: &CapturedImage) -> usize {
    assert_eq!(a.data.len(), b.data.len(), "same capture extent");
    a.data
        .as_chunks::<4>()
        .0
        .iter()
        .zip(b.data.as_chunks::<4>().0)
        .filter(|(pa, pb)| pa != pb)
        .count()
}

#[test]
#[ignore = "requires explicit GPU execution"]
fn portable_session_upgrades_to_standard_in_session() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("reconfiguration evidence: no GPU adapter available; skipping");
        return;
    };

    // A：Portable 会话（StylizedPlugin 默认 Stage A 配置）。
    let mut app = evidence_app(resources.clone());
    let driver = CaseDriver::new(
        app.world(),
        SessionId::new("reconfig-gpu-a").expect("session"),
        binding("outline"),
        ActionGraph::default(),
        DriverLimits::default(),
    )
    .expect("plan");
    let mut controller =
        SessionController::from_catalog(driver, 6, StylizedViewDescriptor::default())
            .expect("controller");
    controller.set_tier_ceiling(resolved_tier(&app));
    assert_eq!(resolved_tier(&app), StylizedExecutionTier::Portable);
    run_to_completion(&mut app, controller.driver_mut(), 700).expect("initial build");
    let outline = capture_once(&mut app, controller.driver_mut());
    assert!(
        outline
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[..3] != [0, 0, 0])
            .count()
            > 0,
        "outline capture must be non-black"
    );

    // 会话内升档：控制器先尝试重配置，设备已具备 Standard 能力 → 直接切换。
    let outcome = controller.command(
        app.world_mut(),
        &SessionCommand::SelectCase(SceneId::new("outline-advanced").expect("id")),
    );
    assert!(
        matches!(outcome, CommandOutcome::Switched { .. }),
        "in-session upgrade must reach the Standard case: {outcome:?}"
    );
    assert_eq!(
        resolved_tier(&app),
        StylizedExecutionTier::Standard,
        "reconfiguration applied in-session"
    );
    assert_eq!(app.world().resource::<StylizedReconfigGeneration>().0, 1);
    run_to_completion(&mut app, controller.driver_mut(), 700).expect("switch build");
    let upgraded = capture_once(&mut app, controller.driver_mut());

    // P：新鲜 Portable 会话直接驱动 outline-advanced（无控制器 → 无升档）
    // → hull-only 回落渲染。
    let mut portable_app = evidence_app(resources.clone());
    reset_view_to_session_baseline(&mut portable_app);
    let mut portable_driver = CaseDriver::new(
        portable_app.world(),
        SessionId::new("reconfig-gpu-p").expect("session"),
        binding("outline-advanced"),
        ActionGraph::default(),
        DriverLimits::default(),
    )
    .expect("plan");
    run_to_completion(&mut portable_app, &mut portable_driver, 700).expect("build");
    let portable_capture = capture_once(&mut portable_app, &mut portable_driver);
    let diff = differing_pixels(&upgraded, &portable_capture);
    let total = upgraded.data.len() / 4;
    assert!(
        diff * 100 > total,
        "screen-outline render must differ from the hull-only fallback on >1% of pixels (got {diff}/{total})"
    );

    // S2：新鲜会话在构建内容前重配置到 Standard → 与会话内升档的渲染
    // 逐字节一致（重配置不留下影响像素的状态）。
    let mut standard_app = evidence_app(resources);
    let applied = standard_app
        .world_mut()
        .apply_stylized_reconfiguration(StylizedCapabilityConfig {
            requested_tier: StylizedExecutionTier::Standard,
            ..StylizedCapabilityConfig::default()
        })
        .expect("standard resolves on this device");
    assert!(applied.changed);
    reset_view_to_session_baseline(&mut standard_app);
    let mut standard_driver = CaseDriver::new(
        standard_app.world(),
        SessionId::new("reconfig-gpu-s2").expect("session"),
        binding("outline-advanced"),
        ActionGraph::default(),
        DriverLimits::default(),
    )
    .expect("plan");
    run_to_completion(&mut standard_app, &mut standard_driver, 700).expect("build");
    let fresh_standard = capture_once(&mut standard_app, &mut standard_driver);
    assert_eq!(
        upgraded.data, fresh_standard.data,
        "in-session upgrade must be pixel-identical to a fresh Standard session"
    );
}

#[test]
#[ignore = "requires explicit GPU execution"]
fn unenabled_device_feature_is_refused_without_state_change() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("reconfiguration evidence: no GPU adapter available; skipping");
        return;
    };
    let mut app = evidence_app(resources);
    let mut driver = CaseDriver::new(
        app.world(),
        SessionId::new("reconfig-gpu-refusal").expect("session"),
        binding("outline"),
        ActionGraph::default(),
        DriverLimits::default(),
    )
    .expect("plan");
    run_to_completion(&mut app, &mut driver, 700).expect("initial build");
    let before_capture = capture_once(&mut app, &mut driver);
    let before_caps = app
        .world()
        .resource::<StylizedResolvedCapabilities>()
        .clone();
    let before_generation = app.world().resource::<StylizedReconfigGeneration>().0;

    // Stage A 未请求 Bindless → 设备快照不含 binding-array 特性 → 结构化拒绝。
    let error = app
        .world_mut()
        .apply_stylized_reconfiguration(StylizedCapabilityConfig {
            requested_tier: StylizedExecutionTier::Standard,
            requested_features: std::collections::BTreeSet::from([StylizedGpuFeature::Bindless]),
            ..StylizedCapabilityConfig::default()
        })
        .expect_err("bindless was never enabled on this device");
    assert_eq!(
        error,
        StylizedReconfigError::FeaturesNotEnabled {
            missing: vec![StylizedGpuFeature::Bindless],
        }
    );

    let after_caps = app.world().resource::<StylizedResolvedCapabilities>();
    assert_eq!(after_caps.actual, before_caps.actual);
    assert_eq!(after_caps.resolved, before_caps.resolved);
    assert_eq!(
        app.world().resource::<StylizedReconfigGeneration>().0,
        before_generation,
        "refused requests never bump the generation"
    );
    let after_capture = capture_once(&mut app, &mut driver);
    assert_eq!(
        before_capture.data, after_capture.data,
        "a refused request must not perturb session pixels"
    );
}
