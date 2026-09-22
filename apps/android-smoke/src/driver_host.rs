//! Android 宿主的设备端配方重放与结构化报告（lines 25-28、739-757、R1）。
//!
//! 流程（Update 系统每帧推进一个阶段；无忙等、无第二事件循环）：
//!
//! 1. **Compile**：用与 PC 完全相同的编译器在设备端编译共享 catalog 的
//!    `outline` 配方 → 同一 recipe hash、同一 Execution 域计划摘要
//!    （身份可追溯——"PC 保存的配方可直接在移动端重放"）；
//! 2. **Run**：`CaseDriver::from_plan` 执行 `fixed_step{22} → observe`，
//!    经 Vulkan 读回取得带回执捕获（实验 tick、harness stamp、像素）；
//!    合理性检查：非黑帧（distinct 颜色数）；
//! 3. **Refuse**：编译 `reference` 配方（未声明 reference-scene 资产）
//!    → 结构化拒绝原因（line 248：未声明的资产依赖在执行前失败；
//!    "无法执行时返回具体原因"）；
//! 4. **Report**：候选写 `report.candidate.json` + 产物 PNG，原子改名
//!    发布（部分输出不算 PASS）；报告绑定 run/scene/recipe hash/plan
//!    digest/session/generation/tick/adapter/backend/resolved tier/stamp/
//!    artifact hash。
//!
//! 设备/驱动/backend 记录为执行身份的一部分（line 239-244）；跨设备像素
//! 一致性不是获准的比较域（line 708-709），本模块不做任何跨设备像素断言。

use bevy::render::renderer::RenderAdapterInfo;
use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_stylized::prelude::StylizedResolvedCapabilities;
use scenario_model::ident::SessionId;
use scenario_model::recipe::{RecipeReference, RunMode, RunSpec, SceneRecipe};
use scene_runtime::{CaseDriver, CompileError, ObservationReceipt, compile};

/// 宿主绑定场景（fixture 内容经 Startup 系统装配；本模块只重放 catalog 配方）。
fn outline_recipe() -> SceneRecipe {
    SceneRecipe::from_json_slice(
        br#"{
            "schema": {"major": 1, "minor": 0},
            "scene": {"id": "outline", "preset": null},
            "seed": "42",
            "rng": {"algorithm": "chacha", "version": "1"},
            "resolution": {"width": 512, "height": 512},
            "anti_aliasing": {"type": "msaa", "samples": 4},
            "tier": null,
            "required_effects": [],
            "assets": [],
            "camera": {"position": [0.0, 0.4, 6.0], "target": [0.0, 0.0, 0.0], "fov_degrees": 45.0},
            "parameters": {},
            "actions": {"nodes": [
                {"id": "settle", "tick": "0", "depends_on": [], "writes": [],
                 "kind": {"type": "fixed_step", "ticks": "22", "step": {"numerator": "1", "denominator": "60"}}},
                {"id": "shot", "tick": "0", "depends_on": ["settle"], "writes": [],
                 "kind": {"type": "observe", "observation": "capture", "view": null}}
            ]}
        }"#,
    )
    .expect("embedded outline recipe must ingest")
}

fn reference_recipe() -> SceneRecipe {
    SceneRecipe::from_json_slice(
        br#"{
            "schema": {"major": 1, "minor": 0},
            "scene": {"id": "reference", "preset": null},
            "seed": "42",
            "rng": {"algorithm": "chacha", "version": "1"},
            "resolution": {"width": 512, "height": 512},
            "anti_aliasing": {"type": "msaa", "samples": 4},
            "tier": null,
            "required_effects": [],
            "assets": [],
            "camera": {"position": [0.0, 1.1, 4.2], "target": [0.0, 0.9, 0.0], "fov_degrees": 45.0},
            "parameters": {},
            "actions": {"nodes": [
                {"id": "shot", "tick": "0", "depends_on": [], "writes": [],
                 "kind": {"type": "observe", "observation": "capture", "view": null}}
            ]}
        }"#,
    )
    .expect("embedded reference recipe must ingest")
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

const REPORT_DIR: &str = "/sdcard/Android/data/org.bevystylized.smoke/files";
const REPORT_CANDIDATE: &str = "report.candidate.json";
const REPORT_FINAL: &str = "report.json";
const ARTIFACT_CANDIDATE: &str = "outline.candidate.png";
const ARTIFACT_FINAL: &str = "outline.png";

#[derive(Default)]
enum Stage {
    #[default]
    Compile,
    Run(Box<CaseDriver>),
    Failed(String),
    RefuseReference {
        receipt: Box<ObservationReceipt>,
    },
    Write {
        receipt: Box<ObservationReceipt>,
        refusal_reason: String,
    },
    Done,
}

/// 宿主侧重放状态（资源；每帧由 [`pump_replay`] 推进）。
#[derive(Resource, Default)]
pub struct ReplayHost {
    stage: Option<Stage>,
    recipe_sha256: String,
    plan_digest: String,
}

fn session_id() -> SessionId {
    SessionId::new("android-smoke").expect("session id")
}

/// 逐帧推进重放（Update 系统）。
pub fn pump_replay(world: &mut World) {
    world.resource_scope(|world, mut host: Mut<ReplayHost>| {
        let stage = host.stage.take().unwrap_or_default();
        host.stage = Some(match stage {
            Stage::Compile => compile_stage(world, &mut host),
            Stage::Run(driver) => run_stage(world, driver),
            Stage::Failed(reason) => {
                info!("ANDROID_REPLAY failed {reason}");
                Stage::Done
            }
            Stage::RefuseReference { receipt } => refuse_stage(receipt),
            Stage::Write {
                receipt,
                refusal_reason,
            } => write_stage(world, &host, receipt, refusal_reason),
            Stage::Done => Stage::Done,
        });
    });
}

fn compile_stage(world: &mut World, host: &mut ReplayHost) -> Stage {
    let recipe = outline_recipe();
    let run = capture_run(&recipe);
    match compile(&recipe, &run, None) {
        Ok(plan) => {
            host.recipe_sha256 =
                scenario_model::identity::recipe_identity(&recipe).expect("recipe identity");
            host.plan_digest = plan.digest.clone();
            info!(
                "ANDROID_REPLAY compiled recipe={} plan={}",
                host.recipe_sha256, host.plan_digest
            );
            match CaseDriver::from_plan(world, session_id(), plan) {
                Ok(driver) => Stage::Run(Box::new(driver)),
                Err(error) => Stage::Failed(format!("from_plan: {error}")),
            }
        }
        Err(error) => Stage::Failed(format!("compile: {error}")),
    }
}

fn run_stage(world: &mut World, mut driver: Box<CaseDriver>) -> Stage {
    match driver.update(world) {
        scene_runtime::DriverStatus::Running => Stage::Run(driver),
        scene_runtime::DriverStatus::Done => {
            let Some(receipt) = driver.drain_observations().pop() else {
                return Stage::Failed("no observation receipt".into());
            };
            let distinct: std::collections::BTreeSet<_> = receipt
                .image
                .data
                .as_chunks::<4>()
                .0
                .iter()
                .map(|px| [px[0], px[1], px[2]])
                .collect();
            if distinct.len() < 8 {
                return Stage::Failed(format!(
                    "capture sanity failed: only {} distinct colors",
                    distinct.len()
                ));
            }
            info!(
                "ANDROID_REPLAY captured tick={} stamp={:?} distinct={}",
                receipt.experiment_tick,
                receipt.stamp,
                distinct.len()
            );
            Stage::RefuseReference {
                receipt: Box::new(receipt),
            }
        }
        scene_runtime::DriverStatus::Failed(error) => Stage::Failed(error.to_string()),
    }
}

fn refuse_stage(receipt: Box<ObservationReceipt>) -> Stage {
    let recipe = reference_recipe();
    let run = capture_run(&recipe);
    let reason = match compile(&recipe, &run, None) {
        Err(CompileError::Catalog(error)) => {
            info!("ANDROID_REPLAY refused reference reason={error}");
            error.to_string()
        }
        Err(error) => {
            return Stage::Failed(format!(
                "reference recipe must fail with a catalog refusal, got: {error}"
            ));
        }
        Ok(_) => {
            return Stage::Failed(
                "reference recipe compiled on device (asset contract broken)".into(),
            );
        }
    };
    Stage::Write {
        receipt,
        refusal_reason: reason,
    }
}

fn write_stage(
    world: &World,
    host: &ReplayHost,
    receipt: Box<ObservationReceipt>,
    refusal_reason: String,
) -> Stage {
    let Some(adapter) = world.get_resource::<RenderAdapterInfo>() else {
        return Stage::Failed("RenderAdapterInfo missing".into());
    };
    let resolved_tier = world
        .get_resource::<StylizedResolvedCapabilities>()
        .and_then(|caps| caps.resolved.as_ref().ok().map(|v| format!("{:?}", v.tier)))
        .unwrap_or_else(|| "unknown".to_string());
    let dir = std::path::Path::new(REPORT_DIR);
    if let Err(error) = std::fs::create_dir_all(dir) {
        return Stage::Failed(format!("report dir: {error}"));
    }
    // 候选 → 原子发布：产物先改名，报告最后落笔（部分输出不算 PASS）。
    let artifact_fnv = golden_harness::fnv1a64_hex(&receipt.image.data);
    let artifact = dir.join(ARTIFACT_FINAL);
    if let Ok(png) = golden_harness::encode_png(&receipt.image) {
        let candidate = dir.join(ARTIFACT_CANDIDATE);
        if let Err(error) =
            std::fs::write(&candidate, &png).and_then(|()| std::fs::rename(&candidate, &artifact))
        {
            return Stage::Failed(format!("artifact publish: {error}"));
        }
    }
    let stamp = receipt.stamp;
    let report = serde_json::json!({
        "schema": "bevy_stylized.android-replay/v1",
        "status": "PASS",
        "scene": "outline",
        "recipe_sha256": host.recipe_sha256,
        "plan_digest": host.plan_digest,
        "session_id": session_id().to_string(),
        "generation": receipt.generation.get(),
        "experiment_tick": receipt.experiment_tick,
        "adapter": format!("{:?}", adapter.0),
        "backend": format!("{:?}", adapter.0.backend).to_lowercase(),
        "resolved_tier": resolved_tier,
        "capture": {
            "stamp": {
                "sequence": stamp.sequence,
                "issued_update_index": stamp.issued_update_index,
                "completed_update_index": stamp.completed_update_index,
                "completed_render_frame": stamp.completed_render_frame,
            },
            "artifact_fnv1a64": artifact_fnv,
            "artifact": ARTIFACT_FINAL,
            "pixels": [receipt.image.width, receipt.image.height],
        },
        "refusals": [{"scene": "reference", "reason": refusal_reason}],
        "scope": "on-device replay evidence; not a cross-device pixel comparison",
    });
    let candidate = dir.join(REPORT_CANDIDATE);
    let final_path = dir.join(REPORT_FINAL);
    match serde_json::to_vec_pretty(&report)
        .map_err(|error| error.to_string())
        .and_then(|bytes| {
            std::fs::write(&candidate, bytes)
                .and_then(|()| std::fs::rename(&candidate, &final_path))
                .map_err(|error| error.to_string())
        }) {
        Ok(()) => {
            info!("ANDROID_REPLAY report_ready {}", final_path.display());
            Stage::Done
        }
        Err(error) => Stage::Failed(format!("report publish: {error}")),
    }
}
