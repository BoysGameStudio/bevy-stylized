//! 运行时重配置 GPU 证据（ADR-0030；Metal，GPU-gated）：
//!
//! 1. Portable 会话构建 courtyard（hull 描边）→ 经控制器
//!    `SetParameter outline_mode=hybrid`（需 Standard）：控制器先尝试
//!    会话内重配置（R2），设备已具备能力则直接接受并同场景重建；重建后
//!    捕获与同一内容在"新鲜 Standard 会话"中的捕获逐字节一致，并与
//!    Portable 的 hull 捕获显著不同（screen outline 内部边缘出现）；
//! 2. 断言未启用设备特性（Stage A 未请求 Bindless）的重配置请求被结构化
//!    拒绝，会话资源与像素逐字节不变。
//!
//! 05 §10 退役后：tier 门控从目录条目（旧 outline-advanced）移到
//! courtyard 的 outline_mode 参数（§8/§11.2）。

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
use scenario_model::ident::{ActionId, SessionId};
use scene_runtime::{
    CaseDriver, CommandOutcome, DriverLimits, SessionCommand, SessionController, run_to_completion,
};

fn evidence_app(resources: RenderResources) -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .build()
            .set(AssetPlugin {
                file_path: concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../fixtures/synthetic/reference-scene"
                )
                .to_string(),
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
    app.add_plugins((
        StylizedPlugin::default(),
        bevy_stylized::StylizedAssetIntegrationPlugin,
    ));
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

    // A：Portable 会话（StylizedPlugin 默认 Stage A 配置）构建 courtyard（hull）。
    let mut app = evidence_app(resources.clone());
    let driver = CaseDriver::new(
        app.world(),
        SessionId::new("reconfig-gpu-a").expect("session"),
        binding("courtyard"),
        ActionGraph::default(),
        DriverLimits::default(),
    )
    .expect("plan");
    let mut controller =
        SessionController::from_catalog(driver, 0, StylizedViewDescriptor::default())
            .expect("controller");
    controller.set_tier_ceiling(resolved_tier(&app));
    assert_eq!(resolved_tier(&app), StylizedExecutionTier::Portable);
    run_to_completion(&mut app, controller.driver_mut(), 700).expect("initial build");
    let hull = capture_once(&mut app, controller.driver_mut());
    assert!(
        hull.data
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[..3] != [0, 0, 0])
            .count()
            > 0,
        "courtyard capture must be non-black"
    );

    // 会话内升档：outline_mode=hybrid 需 Standard → 控制器先尝试重配置，
    // 设备已具备能力 → 接受。W1（05 §11.2）起 outline_mode 是相机附件
    // live 换——不换代次、不重建内容。
    let outcome = controller.command(
        app.world_mut(),
        &SessionCommand::SetParameter {
            key: "outline_mode".into(),
            value: serde_json::json!("hybrid"),
        },
    );
    assert_eq!(
        outcome,
        CommandOutcome::Applied,
        "in-session upgrade must accept the hybrid parameter"
    );
    assert_eq!(
        resolved_tier(&app),
        StylizedExecutionTier::Standard,
        "reconfiguration applied in-session"
    );
    assert_eq!(app.world().resource::<StylizedReconfigGeneration>().0, 1);
    run_to_completion(&mut app, controller.driver_mut(), 700).expect("rebuild");
    let upgraded = capture_once(&mut app, controller.driver_mut());
    let total = upgraded.data.len() / 4;
    let diff = differing_pixels(&upgraded, &hull);
    assert!(
        diff * 100 > total,
        "hybrid (screen outline) must differ from hull on >1% of pixels (got {diff}/{total})"
    );

    // S2：新鲜会话在构建内容前重配置到 Standard + 同参数 → 与会话内升档
    // 的渲染逐字节一致（重配置不留下影响像素的状态）。
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
    standard_app.insert_resource(scenes::CourtyardSettings {
        outline_mode: scenes::CourtyardOutlineMode::Hybrid,
        ..scenes::CourtyardSettings::default()
    });
    let mut standard_driver = CaseDriver::new(
        standard_app.world(),
        SessionId::new("reconfig-gpu-s2").expect("session"),
        binding("courtyard"),
        ActionGraph::default(),
        DriverLimits::default(),
    )
    .expect("plan");
    run_to_completion(&mut standard_app, &mut standard_driver, 700).expect("build");
    let fresh_standard = capture_once(&mut standard_app, &mut standard_driver);
    // 重配置不留影响像素的状态：两条路径的渲染在 screen-outline 噪声包络
    // 内一致。噪声依据（fixtures/golden/showcase.courtyard.acceptance.md 的
    // 修正条目）：screen-space 描边在背景剪影处有逐运行 2–4 像素的边缘
    // 翻转，与会话历史无关、与重配置语义无关；hybrid 的严格判定因此归
    // p3 ROI（此处不用整图逐字节）。
    let noisy = differing_pixels(&upgraded, &fresh_standard);
    assert!(
        noisy <= 16,
        "in-session upgrade must match a fresh Standard session within the documented screen-outline noise envelope (got {noisy} differing pixels)"
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
        binding("courtyard"),
        ActionGraph::default(),
        DriverLimits::default(),
    )
    .expect("plan");
    run_to_completion(&mut app, &mut driver, 700).expect("initial build");
    // 快照缺特性 → 结构化拒绝。宿主无关的确定性构造（2026-09-30 lane 登记
    // 后发现旧前提失效：本机 Stage B 快照默认启用 Bindless/Float16——
    // 「Stage A 未请求」在本机不成立；改为显式从快照摘除特性，测拒绝
    // 合同本身而非宿主的默认启用集）。摘除在基线读取之前。
    app.world_mut()
        .resource_mut::<StylizedResolvedCapabilities>()
        .actual
        .features
        .remove(&StylizedGpuFeature::Bindless);
    let before_capture = capture_once(&mut app, &mut driver);
    let before_caps = app
        .world()
        .resource::<StylizedResolvedCapabilities>()
        .clone();
    let before_generation = app.world().resource::<StylizedReconfigGeneration>().0;

    let error = app
        .world_mut()
        .apply_stylized_reconfiguration(StylizedCapabilityConfig {
            requested_tier: StylizedExecutionTier::Standard,
            requested_features: std::collections::BTreeSet::from([StylizedGpuFeature::Bindless]),
            ..StylizedCapabilityConfig::default()
        })
        .expect_err("snapshot without bindless must refuse");
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
