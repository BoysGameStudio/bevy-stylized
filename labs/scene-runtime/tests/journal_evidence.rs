//! 命令日志 GPU 证据（Metal，GPU-gated）：机位/参数命令录制 → 新鲜会话
//! 重放 → 末次捕获逐字节一致（逻辑身份 ⇒ 同设备像素身份）。
//! （05 §10 退役后目录单场景：录制/回放覆盖 V1 机位与参数命令。）

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
    StylizedExecutionTier, StylizedPlugin, StylizedViewDescriptor, StylizedViewSettings,
};
use golden_harness::{CapturedImage, capture, readiness};
use scenario_model::action::{ActionGraph, ActionKind, ActionNode, Step};
use scenario_model::canon::CanonicalU64;
use scenario_model::ident::{ActionId, SessionId};
use scene_runtime::{
    CaseDriver, CommandJournal, CommandOutcome, DriverLimits, SessionCommand, SessionController,
    run_to_completion,
};

mod support_recipes;

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
    *app.world_mut().resource_mut::<StylizedViewSettings>() =
        StylizedViewSettings(StylizedViewDescriptor {
            key_light_direction: bevy::math::Vec3::new(0.5, 0.8, 0.6).normalize(),
            key_light_color: bevy::color::LinearRgba::WHITE,
            exposure: 1.0,
            day_night_factor: 1.0,
            ..StylizedViewDescriptor::default()
        });
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

fn controller_at_courtyard(world: &bevy::prelude::World) -> SessionController {
    let driver = CaseDriver::new(
        world,
        SessionId::new("journal-gpu").expect("session"),
        binding("courtyard"),
        ActionGraph::default(),
        DriverLimits::default(),
    )
    .expect("plan");
    let mut controller =
        SessionController::from_catalog(driver, 0, StylizedViewDescriptor::default())
            .expect("controller");
    controller.set_tier_ceiling(StylizedExecutionTier::Standard);
    controller
}

fn final_capture(app: &mut App, controller: &mut SessionController) -> CapturedImage {
    controller
        .driver_mut()
        .run_actions(capture_profile_graph())
        .expect("capture plan");
    run_to_completion(app, controller.driver_mut(), 700).expect("capture");
    controller
        .driver_mut()
        .drain_observations()
        .pop()
        .expect("one observation")
        .image
}

#[test]
#[ignore = "requires explicit GPU execution"]
fn journal_replay_preserves_logical_and_pixel_identity() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("journal evidence: no GPU adapter available; skipping");
        return;
    };
    // 录制：A→B→A 经 command_recorded。
    let mut app = evidence_app(resources.clone());
    let mut controller = controller_at_courtyard(app.world());
    // 初始构建推进到 Done（命令只在 Done/AwaitObservation 可切换）。
    run_to_completion(&mut app, controller.driver_mut(), 700).expect("initial build");
    let mut journal = CommandJournal::new(SessionId::new("journal-gpu").expect("session"), 0);
    for command in [
        SessionCommand::SetCameraPreset("lod-side".to_string()),
        SessionCommand::SetParameter {
            key: "crowd".to_string(),
            value: serde_json::json!(0),
        },
        SessionCommand::SetParameter {
            key: "crowd".to_string(),
            value: serde_json::json!(100),
        },
        SessionCommand::SetCameraPreset("lamps".to_string()),
    ] {
        assert!(matches!(
            controller.command_recorded(app.world_mut(), &command, &mut journal),
            CommandOutcome::Switched { .. } | CommandOutcome::Applied
        ));
        run_to_completion(&mut app, controller.driver_mut(), 700).expect("build completes");
    }
    assert_eq!(journal.entries.len(), 4);
    assert!(
        journal
            .entries
            .iter()
            .all(|entry| matches!(entry.state, scene_runtime::JournalEntryState::Confirmed(_)))
    );
    let recorded = final_capture(&mut app, &mut controller);

    // 重放：新鲜会话 + 同一日志 → 逻辑结果逐条一致 + 末次捕获逐字节一致。
    let mut replay_app = evidence_app(resources);
    let mut replay_controller = controller_at_courtyard(replay_app.world());
    run_to_completion(&mut replay_app, replay_controller.driver_mut(), 700).expect("initial build");
    let report = replay_controller.replay(&mut replay_app, &journal);
    assert!(
        report.mismatches.is_empty(),
        "replay mismatches: {:?}",
        report.mismatches
    );
    assert_eq!(report.applied, 4);
    assert!(report.indeterminate.is_empty());
    let replayed = final_capture(&mut replay_app, &mut replay_controller);
    assert_eq!(
        recorded.data, replayed.data,
        "journal replay must reproduce the recorded session byte-for-byte"
    );

    // 持久化：journal JSON round-trip 后重放仍然一致。
    let bytes = journal.to_json().expect("encode");
    let loaded = CommandJournal::from_json(&bytes).expect("decode");
    assert_eq!(journal, loaded);
    let mut second_app = evidence_app(capture::probe_render_resources().expect("GPU"));
    let mut second = controller_at_courtyard(second_app.world());
    run_to_completion(&mut second_app, second.driver_mut(), 700).expect("initial build");
    let report = second.replay(&mut second_app, &loaded);
    assert!(
        report.is_exact(&loaded),
        "round-tripped journal must replay exactly"
    );
}

/// 恢复水位（CPU 语义经 GPU 路径再确认）：尾部 Tentative 只报告不重跑。
#[test]
#[ignore = "requires explicit GPU execution"]
fn trailing_tentative_entry_is_indeterminate_not_reexecuted() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("journal evidence: no GPU adapter available; skipping");
        return;
    };
    let mut app = evidence_app(resources);
    let mut controller = controller_at_courtyard(app.world());
    run_to_completion(&mut app, controller.driver_mut(), 700).expect("initial build");
    let mut journal = CommandJournal::new(SessionId::new("journal-gpu").expect("session"), 6);
    // 模拟"接受后崩溃"：手工塞入一条 Tentative（不经 command_recorded）。
    journal.entries.push(scene_runtime::JournalEntry {
        sequence: 0,
        command: SessionCommand::SetCameraPreset("lod-side".to_string()),
        generation_before: 0,
        state: scene_runtime::JournalEntryState::Tentative,
    });
    let report = controller.replay(&mut app, &journal);
    assert_eq!(report.applied, 0, "tentative entry must not be reexecuted");
    assert_eq!(report.indeterminate, vec![0]);
    assert_eq!(
        controller.current().id.as_str(),
        "courtyard",
        "controller state must be unchanged"
    );
}
