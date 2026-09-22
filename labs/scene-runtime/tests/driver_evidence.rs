//! CaseDriver 证据测试（GPU-gated，无 GPU graceful skip；动作调度与计划
//! 级拒绝另有 CPU 覆盖见同文件末尾与 crate 单测）。
//!
//! 对应 docs/showcase/01 R1/R12/R13 与验收矩阵生命周期行的本地证据：
//! 单步捕获的帧身份、等待泵送节奏不改变实验状态、A→B→A 场景切换无残留、
//! 动作稳定次序与结构化计划拒绝。内容一律复用 scenes 注册表构建器。

use bevy::{
    DefaultPlugins,
    app::PluginGroup,
    asset::{AssetMetaCheck, AssetPlugin},
    camera::ClearColorConfig,
    color::Color,
    prelude::{App, Camera, Camera3d, With, World},
    render::{
        RenderPlugin,
        settings::{RenderCreation, RenderResources},
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::StylizedPlugin;
use golden_harness::{CapturedImage, capture, readiness};
use scenario_model::action::{ActionGraph, ActionKind, ActionNode, Step};
use scenario_model::canon::CanonicalU64;
use scenario_model::ident::{ActionId, SessionId};
use scene_runtime::{
    CaseDriver, DriverLimits, DriverStatus, RuntimeError, SceneCounts, run_to_completion,
};
use scenes::registry::{SceneBinding, bindings};

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

fn binding(id: &str) -> SceneBinding {
    bindings()
        .into_iter()
        .find(|binding| binding.id == id)
        .expect("registered scene")
}

fn action(id: &str, tick: u64, depends_on: &[&str], kind: ActionKind) -> ActionNode {
    ActionNode {
        id: ActionId::new(id).expect("action id"),
        tick: CanonicalU64(tick),
        depends_on: depends_on
            .iter()
            .map(|dep| ActionId::new(*dep).expect("dep id"))
            .collect(),
        writes: Vec::new(),
        kind,
    }
}

fn fixed_step(id: &str, ticks: u64) -> ActionNode {
    action(
        id,
        0,
        &[],
        ActionKind::FixedStep {
            ticks: CanonicalU64(ticks),
            step: Step::new(1, 60),
        },
    )
}

fn observe(id: &str, observation: &str, depends_on: &[&str]) -> ActionNode {
    action(
        id,
        0,
        depends_on,
        ActionKind::Observe {
            observation: observation.to_string(),
            view: None,
        },
    )
}

fn set_clear(world: &mut World, clear: Color) {
    let mut cameras = world.query_filtered::<&mut Camera, With<Camera3d>>();
    for mut camera in cameras.iter_mut(world) {
        camera.clear_color = ClearColorConfig::Custom(clear);
    }
}

fn mean_rgb(image: &CapturedImage) -> [f32; 3] {
    let mut sum = [0_f32; 3];
    for px in image.data.as_chunks::<4>().0 {
        for c in 0..3 {
            sum[c] += f32::from(px[c]);
        }
    }
    let n = (image.width * image.height) as f32;
    [sum[0] / n, sum[1] / n, sum[2] / n]
}

const RED: Color = Color::srgb(1.0, 0.0, 0.0);
const GREEN: Color = Color::srgb(0.0, 1.0, 0.0);
const BLUE: Color = Color::srgb(0.0, 0.0, 1.0);

/// 单步捕获：fixed_step(2) 后 observe；回执的实验 tick 与 stamp 证明帧
/// 身份，像素反映步进后的状态（tick 1 绿、tick 2 蓝 → 观测为蓝）。
#[test]
#[ignore = "requires explicit GPU execution"]
fn single_step_capture_binds_experiment_tick_and_stamp() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("scene-runtime evidence: no GPU adapter available; skipping");
        return;
    };
    let mut app = evidence_app(resources);
    let mut driver = CaseDriver::new(
        app.world(),
        SessionId::new("s1").expect("session"),
        binding("outline"),
        ActionGraph {
            nodes: vec![fixed_step("step", 2), observe("shot", "rgba8", &["step"])],
        },
        DriverLimits::default(),
    )
    .expect("plan");
    driver.set_tick_hook(Box::new(|tick, world| {
        set_clear(world, if tick == 1 { GREEN } else { BLUE });
    }));
    run_to_completion(&mut app, &mut driver, 1200).expect("driver completes");

    assert_eq!(driver.clock().tick(), 2);
    assert_eq!(driver.clock().elapsed(), (1, 30));
    let receipts = driver.observations();
    assert_eq!(receipts.len(), 1);
    let receipt = &receipts[0];
    assert_eq!(
        receipt.experiment_tick, 2,
        "observe issues after both ticks"
    );
    assert_eq!(receipt.session.as_str(), "s1");
    assert_eq!(receipt.generation.get(), 0);
    assert_eq!(receipt.view.as_str(), "main");
    assert!(
        receipt.stamp.completed_update_index >= receipt.stamp.issued_update_index,
        "completion never precedes issue: {:?}",
        receipt.stamp
    );
    let [r, g, b] = mean_rgb(&receipt.image);
    assert!(
        b > 100.0 && b > r && b > g,
        "pixels must reflect the stepped (BLUE) state: {:?}",
        receipt.stamp
    );
    eprintln!(
        "[driver] tick={} receipt tick={} stamp issued={} completed={} render_frame={:?}",
        driver.clock().tick(),
        receipt.experiment_tick,
        receipt.stamp.issued_update_index,
        receipt.stamp.completed_update_index,
        receipt.stamp.completed_render_frame
    );
}

/// 等待/泵送节奏不改变实验状态：同一计划跑两遍，一遍在每个 driver 步
/// 之间插入额外渲染泵；实验 tick、采样/请求身份与像素必须逐位一致
/// （stamp 的主世界 update 序号随节奏变化——它是宿主节奏的证据元数据，
/// 不是实验身份）。
#[test]
#[ignore = "requires explicit GPU execution"]
fn wait_pumping_cadence_does_not_change_experiment_state() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("scene-runtime evidence: no GPU adapter available; skipping");
        return;
    };
    let plan = || ActionGraph {
        nodes: vec![
            action(
                "wait",
                0,
                &[],
                ActionKind::WaitReady {
                    condition: scenario_model::action::ReadyCondition::PipelinesReady,
                },
            ),
            fixed_step("s1", 1),
            observe("a", "first", &["s1"]),
            fixed_step("s2", 1),
            observe("b", "second", &["s2"]),
        ],
    };
    let make_driver = |app: &App| {
        let mut driver = CaseDriver::new(
            app.world(),
            SessionId::new("s1").expect("session"),
            binding("outline"),
            plan(),
            DriverLimits::default(),
        )
        .expect("plan");
        driver.set_tick_hook(Box::new(|tick, world| {
            set_clear(world, if tick == 1 { RED } else { GREEN });
        }));
        driver
    };

    // Run A：最小泵送。
    let mut app_a = evidence_app(resources.clone());
    let mut driver_a = make_driver(&app_a);
    run_to_completion(&mut app_a, &mut driver_a, 1200).expect("run A");

    // Run B：每个 driver 步之间插入 3 次额外渲染泵（宿主节奏不同）。
    let mut app_b = evidence_app(resources);
    let mut driver_b = make_driver(&app_b);
    for _ in 0..4800 {
        match driver_b.update(app_b.world_mut()) {
            DriverStatus::Running => {
                app_b.update();
                app_b.update();
                app_b.update();
                app_b.update();
            }
            DriverStatus::Done => break,
            DriverStatus::Failed(error) => panic!("run B failed: {error}"),
        }
    }

    assert_eq!(driver_a.clock().tick(), driver_b.clock().tick());
    assert_eq!(driver_a.clock().elapsed(), driver_b.clock().elapsed());
    let a = driver_a.observations();
    let b = driver_b.observations();
    assert_eq!(a.len(), 2);
    assert_eq!(b.len(), 2);
    for (ra, rb) in a.iter().zip(b) {
        assert_eq!(
            ra.experiment_tick, rb.experiment_tick,
            "experiment identity"
        );
        assert_eq!(ra.request, rb.request);
        assert_eq!(ra.sample, rb.sample);
        assert_eq!(ra.observation, rb.observation);
        assert_eq!(
            ra.image.data, rb.image.data,
            "pixels must not depend on pump cadence (tick {})",
            ra.experiment_tick
        );
    }
    eprintln!(
        "[driver] cadence experiment identity equal; stamp issued {} vs {} (host-cadence metadata)",
        a[0].stamp.issued_update_index, b[0].stamp.issued_update_index
    );
}

/// 生命周期（A→B→A）：切换只退役场景账本实体；完成 A→B→A 后场景资源
/// 计数与首次构建后逐项一致（无重复相机/系统/增长的资源）；切换时作废的
/// 在途观测无可取结果（退役代次的晚到回执被忽略）。
#[test]
#[ignore = "requires explicit GPU execution"]
fn scene_switch_a_b_a_returns_to_baseline() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("scene-runtime evidence: no GPU adapter available; skipping");
        return;
    };
    let mut app = evidence_app(resources);
    let plan = || ActionGraph {
        nodes: vec![observe("shot", "rgba8", &[])],
    };
    let mut driver = CaseDriver::new(
        app.world(),
        SessionId::new("s1").expect("session"),
        binding("outline"),
        plan(),
        DriverLimits::default(),
    )
    .expect("plan");

    run_to_completion(&mut app, &mut driver, 1200).expect("A completes");
    let after_first_build = SceneCounts::capture(app.world());
    let a_observations = driver.observations().len();
    assert_eq!(a_observations, 1);
    assert_eq!(driver.generation().get(), 0);

    driver
        .switch_scene(app.world_mut(), binding("animation"))
        .into_completed()
        .expect("switch A→B accepted");
    run_to_completion(&mut app, &mut driver, 1200).expect("B completes");
    assert_eq!(driver.generation().get(), 1);

    driver
        .switch_scene(app.world_mut(), binding("outline"))
        .into_completed()
        .expect("switch B→A accepted");
    run_to_completion(&mut app, &mut driver, 1200).expect("A completes again");
    assert_eq!(driver.generation().get(), 2);
    // 排空若干帧（资产句柄释放的处理窗口），再逐项对账。
    for _ in 0..8 {
        app.update();
    }
    let after_cycle = SceneCounts::capture(app.world());
    assert_eq!(
        after_cycle, after_first_build,
        "A→B→A must return to the first-build counts (no leaked entities/cameras/assets)"
    );
}

/// 在途观测在切换时被作废：观测发出后未完成即切换，旧请求无可取结果。
#[test]
#[ignore = "requires explicit GPU execution"]
fn in_flight_observation_is_voided_on_scene_switch() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("scene-runtime evidence: no GPU adapter available; skipping");
        return;
    };
    let mut app = evidence_app(resources);
    let mut driver = CaseDriver::new(
        app.world(),
        SessionId::new("s1").expect("session"),
        binding("outline"),
        ActionGraph {
            nodes: vec![observe("shot", "rgba8", &[])],
        },
        DriverLimits::default(),
    )
    .expect("plan");
    // 推进到观测发出（AwaitObservation），但不给它完成的泵送窗口。
    for _ in 0..1200 {
        if driver.state_name() == "AwaitObservation" {
            break;
        }
        match driver.update(app.world_mut()) {
            DriverStatus::Running => app.update(),
            DriverStatus::Done => panic!("plan should not finish before the observation"),
            DriverStatus::Failed(error) => panic!("driver failed: {error}"),
        }
    }
    assert_eq!(driver.state_name(), "AwaitObservation");
    driver
        .switch_scene(app.world_mut(), binding("animation"))
        .into_completed()
        .expect("switch during pending observation");
    run_to_completion(&mut app, &mut driver, 1200).expect("B completes");
    // 退役代次没有发布任何观测；B 代只产生自己的回执。
    assert_eq!(driver.observations().len(), 1);
    assert_eq!(driver.observations()[0].generation.get(), 1);
}

/// 动作调度：依赖 + 声明序决胜的稳定次序经动作接缝可观测；
/// 同 tick 写冲突在构造期返回结构化错误（不是 panic）。
#[test]
#[ignore = "requires explicit GPU execution"]
fn action_graph_executes_in_stable_order() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("scene-runtime evidence: no GPU adapter available; skipping");
        return;
    };
    let mut app = evidence_app(resources);
    let apply = |id: &str, depends_on: &[&str]| ActionNode {
        kind: ActionKind::ApplyParams {
            target: scenario_model::action::TargetRef {
                scope: scenario_model::action::TargetScope::Material,
                id: scenario_model::ident::TargetId::new("hero_face").expect("target"),
            },
            params: Default::default(),
        },
        ..action(
            id,
            0,
            depends_on,
            ActionKind::Checkpoint {
                label: String::new(),
            },
        )
    };
    // 声明序 [c, b, a]；c 依赖 a/b → 稳定次序 b, a, c。
    let mut driver = CaseDriver::new(
        app.world(),
        SessionId::new("s1").expect("session"),
        binding("outline"),
        ActionGraph {
            nodes: vec![apply("c", &["a", "b"]), apply("b", &[]), apply("a", &[])],
        },
        DriverLimits::default(),
    )
    .expect("plan");
    let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let record = order.clone();
    driver.set_action_hook(Box::new(move |node, _world| {
        record
            .lock()
            .expect("order lock")
            .push(node.id.as_str().to_string());
    }));
    run_to_completion(&mut app, &mut driver, 1200).expect("driver completes");
    assert_eq!(*order.lock().expect("order lock"), vec!["b", "a", "c"]);
}

/// 计划级拒绝（CPU）：同 tick 写冲突 → 结构化 Plan 错误；未注册接缝的
/// apply_params → 结构化 UnsupportedAction（GPU 覆盖在执行路径上）。
#[test]
fn same_tick_write_conflict_is_a_structured_plan_error() {
    let app = App::new();
    let write = |id: &str| ActionNode {
        writes: vec![scenario_model::action::TargetRef {
            scope: scenario_model::action::TargetScope::Material,
            id: scenario_model::ident::TargetId::new("hero_face").expect("target"),
        }],
        ..action(
            id,
            5,
            &[],
            ActionKind::Checkpoint {
                label: id.to_string(),
            },
        )
    };
    let result = CaseDriver::new(
        app.world(),
        SessionId::new("s1").expect("session"),
        binding("outline"),
        ActionGraph {
            nodes: vec![write("a"), write("b")],
        },
        DriverLimits::default(),
    );
    match result {
        Err(RuntimeError::Plan(error)) => {
            assert!(
                error.contains("write conflict"),
                "unexpected plan error: {error}"
            );
        }
        Err(error) => panic!("wrong error variant: {error}"),
        Ok(_) => panic!("conflicting plan must be rejected at construction"),
    }
}

/// 测试辅助：把 DriverStatus::Running 映射为可 expect 的结果。
trait IntoCompleted {
    fn into_completed(self) -> Result<(), RuntimeError>;
}

impl IntoCompleted for DriverStatus {
    fn into_completed(self) -> Result<(), RuntimeError> {
        match self {
            DriverStatus::Running => Ok(()),
            DriverStatus::Done => Ok(()),
            DriverStatus::Failed(error) => Err(error),
        }
    }
}
