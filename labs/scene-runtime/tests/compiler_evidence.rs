//! 配方编译器 → CaseDriver 的端到端证据（GPU-gated，无 GPU graceful skip）。
//!
//! - 编译的 outline 配方与 showcase 既有捕获逐字节一致（declare→execute
//!   闭环的等价证据；相机/AA/视图/暖场语义见 compiler 模块文档）；
//! - apply_params 按声明作用域写入实例参数：不同声明阈值产出可测差异，
//!   同一配方产出逐字节一致；
//! - deform 经稳定逻辑 ID 绑定 strip root：不同声明旋转产出差异，
//!   回执实验 tick 正确；
//! - 未声明参数键在执行路径上产生结构化失败（不是 panic）。

use bevy::{
    DefaultPlugins,
    app::PluginGroup,
    asset::{AssetMetaCheck, AssetPlugin},
    color::LinearRgba,
    math::Vec3,
    prelude::App,
    render::{
        RenderPlugin,
        settings::{RenderCreation, RenderResources},
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::{StylizedPlugin, StylizedViewDescriptor, StylizedViewSettings};
use golden_harness::{CapturedImage, capture, readiness};
use scenario_model::ident::SessionId;
use scenario_model::recipe::{RecipeReference, RunMode, RunSpec, SceneRecipe};
use scene_runtime::{CaseDriver, DriverStatus, ScenePlan, compile, run_to_completion};

mod support_recipes;

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
    // showcase 宿主视图基线（byte-match 前提）。
    *app.world_mut().resource_mut::<StylizedViewSettings>() =
        StylizedViewSettings(StylizedViewDescriptor {
            key_light_direction: Vec3::new(0.5, 0.8, 0.6).normalize(),
            key_light_color: LinearRgba::WHITE,
            exposure: 1.0,
            day_night_factor: 1.0,
            ..StylizedViewDescriptor::default()
        });
    app.finish();
    app.cleanup();
    app
}

fn run_plan(app: &mut App, plan: ScenePlan, budget: usize) -> scene_runtime::ObservationReceipt {
    let mut driver = CaseDriver::from_plan(
        app.world(),
        SessionId::new("compiled-evidence").expect("session id"),
        plan,
    )
    .expect("plan");
    run_to_completion(app, &mut driver, budget).expect("driver completes");
    driver.drain_observations().pop().expect("one observation")
}

fn capture_run(recipe: &SceneRecipe) -> RunSpec {
    RunSpec {
        schema: scenario_model::recipe::RUN_SPEC_SCHEMA,
        recipe: RecipeReference::Inline(Box::new(recipe.clone())),
        mode: RunMode::Capture,
        sampling: None,
        evaluation: None,
    }
}

/// 编译的 outline 配方（相机/AA/暖场与 showcase 捕获 profile 等价）→
/// 与 /tmp/showcase-pre-driver/outline.png 逐字节一致。
#[test]
#[ignore = "requires explicit GPU execution"]
fn compiled_outline_recipe_matches_showcase_capture() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("compiler evidence: no GPU adapter available; skipping");
        return;
    };
    let recipe = support_recipes::outline_recipe();
    let run = capture_run(&recipe);
    let plan = compile(&recipe, &run, None).expect("compile");
    let mut app = evidence_app(resources);
    let receipt = run_plan(&mut app, plan, 700);
    assert_eq!(receipt.experiment_tick, 22);

    let baseline_path = std::path::Path::new("/tmp/showcase-pre-driver/outline.png");
    let baseline_bytes = std::fs::read(baseline_path)
        .unwrap_or_else(|error| panic!("baseline missing ({error}); regenerate showcase captures"));
    let baseline = golden_harness::decode_png(&baseline_bytes).expect("decode baseline");
    assert_eq!(
        receipt.image.data, baseline.data,
        "compiled outline recipe must match the showcase capture byte-for-byte"
    );
}

/// apply_params（entity 作用域）：face 中间实例的 face_threshold 声明扫描。
#[test]
#[ignore = "requires explicit GPU execution"]
fn apply_params_sweeps_face_threshold_deterministically() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("compiler evidence: no GPU adapter available; skipping");
        return;
    };
    let recipe_for = |threshold: f64| support_recipes::face_threshold_recipe(threshold);
    let capture_with = |resources: &RenderResources, recipe: &SceneRecipe| {
        let run = capture_run(recipe);
        let plan = compile(recipe, &run, None).expect("compile");
        let mut app = evidence_app(resources.clone());
        run_plan(&mut app, plan, 700).image
    };
    let low = capture_with(&resources, &recipe_for(0.2));
    let high = capture_with(&resources, &recipe_for(0.9));
    assert_ne!(
        low.data, high.data,
        "different declared thresholds must produce measurably different pixels"
    );
    let low_again = capture_with(&resources, &recipe_for(0.2));
    assert_eq!(
        low.data, low_again.data,
        "same recipe must produce byte-identical pixels"
    );
    // 差异定位在中间实例区域（instances.1），不是全局漂移。
    let middle = |image: &CapturedImage, x: usize, y: usize| {
        let offset = (y * image.width as usize + x) * 4;
        image.data[offset]
    };
    let differs_in_middle = (200usize..300).step_by(8).any(|x| {
        (200usize..300)
            .step_by(8)
            .any(|y| middle(&low, x, y) != middle(&high, x, y))
    });
    assert!(
        differs_in_middle,
        "threshold sweep must change the middle instance"
    );
}

/// deform（entity 作用域，稳定逻辑 ID）：strip root 的 rotation_z 绑定。
#[test]
#[ignore = "requires explicit GPU execution"]
fn deform_strip_root_binds_logical_target() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("compiler evidence: no GPU adapter available; skipping");
        return;
    };
    let recipe_for = |rotation: f64| support_recipes::strip_deform_recipe(rotation);
    let capture_with = |resources: &RenderResources, recipe: &SceneRecipe| {
        let run = capture_run(recipe);
        let plan = compile(recipe, &run, None).expect("compile");
        let mut app = evidence_app(resources.clone());
        run_plan(&mut app, plan, 700)
    };
    let straight = capture_with(&resources, &recipe_for(0.0));
    let bent = capture_with(&resources, &recipe_for(1.2));
    assert_eq!(
        straight.experiment_tick, 2,
        "observe issues after both ticks"
    );
    assert_eq!(bent.experiment_tick, 2);
    assert_ne!(
        straight.image.data, bent.image.data,
        "declared rotation must change the strip pose"
    );
    let straight_again = capture_with(&resources, &recipe_for(0.0));
    assert_eq!(straight.image.data, straight_again.image.data);
}

/// 配方 `parameters` 下降与 apply_params 动作路径像素级等价：
/// parameters 声明的阈值与 H2 动作声明的阈值产出逐字节一致的图像。
#[test]
#[ignore = "requires explicit GPU execution"]
fn recipe_parameters_bind_face_threshold_like_apply_params() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("compiler evidence: no GPU adapter available; skipping");
        return;
    };
    let capture_with = |resources: &RenderResources, recipe: &SceneRecipe| {
        let run = capture_run(recipe);
        let plan = compile(recipe, &run, None).expect("compile");
        let mut app = evidence_app(resources.clone());
        run_plan(&mut app, plan, 700).image
    };
    // A：parameters 声明（catalog schema 校验 + 下降）。
    let with_parameters = support_recipes::face_parameters_recipe(0.2);
    // B：H2 的 apply_params 动作声明（同阈值同目标）。
    let with_action = support_recipes::face_threshold_recipe(0.2);
    let a = capture_with(&resources, &with_parameters);
    let b = capture_with(&resources, &with_action);
    assert_eq!(
        a.data, b.data,
        "parameters lowering must be pixel-equivalent to the apply_params path"
    );
    let a_again = capture_with(&resources, &support_recipes::face_parameters_recipe(0.2));
    assert_eq!(a.data, a_again.data, "same recipe must be byte-identical");
    let different = capture_with(&resources, &support_recipes::face_parameters_recipe(0.9));
    assert_ne!(
        a.data, different.data,
        "declared parameter values must change pixels"
    );
}

/// 执行路径上的结构化失败：未声明的参数键（entity 作用域）。
#[test]
#[ignore = "requires explicit GPU execution"]
fn undeclared_param_key_fails_structurally() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("compiler evidence: no GPU adapter available; skipping");
        return;
    };
    let recipe = support_recipes::face_threshold_recipe(0.5);
    let bad = SceneRecipe::from_json_slice(
        support_recipes::face_threshold_json(0.5)
            .replace("\"face_threshold\": 0.5", "\"not_a_declared_param\": 1.0")
            .as_bytes(),
    )
    .expect("recipe");
    let _ = recipe;
    let run = capture_run(&bad);
    let plan = compile(&bad, &run, None).expect("compile");
    let mut app = evidence_app(resources);
    let mut driver = CaseDriver::from_plan(
        app.world(),
        SessionId::new("compiled-evidence").expect("session id"),
        plan,
    )
    .expect("plan");
    for _ in 0..700 {
        match driver.update(app.world_mut()) {
            DriverStatus::Running => app.update(),
            DriverStatus::Done => panic!("undeclared parameter must not succeed"),
            DriverStatus::Failed(error) => {
                assert!(
                    error.to_string().contains("not_a_declared_param"),
                    "failure must name the parameter: {error}"
                );
                return;
            }
        }
    }
    panic!("driver did not reach a terminal state");
}
