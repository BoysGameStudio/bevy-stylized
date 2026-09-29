//! 配方编译器（docs/showcase/01 lines 222-227）：把 `SceneRecipe` +
//! `RunSpec`（+ `SessionPlan` 执行封套）展开成不可变 [`ScenePlan`]，
//! 关闭"声明 → 执行"回路。
//!
//! 展开规则：
//!
//! - 默认值只在编译阶段展开（运行时无隐式默认）；preset 保持显式——
//!   catalog 当前没有声明任何 preset，声明了 preset 的配方在编译期拒绝
//!   （line 140：相似外观不等于相同负载，不按名字合并 preset）；
//! - 场景引用经 `scenario-model` catalog 解析：未知场景、catalog 交叉校验
//!   （未声明的资产依赖、tier 不足）在编译期失败，绝不在执行期失败；
//! - 相机 `CameraSpec`（声明 f64）映射到运行时 `CameraRecipe`（f32）：
//!   编译期精度映射（f64→f32 最近值）；`fov_degrees == 45.0` 映射为
//!   引擎默认投影（π/4，文档化的默认等价），其余值映射为显式
//!   Perspective 投影；
//! - 抗锯齿声明映射：`None → Msaa::Off`（显式声明无 AA），
//!   `Msaa{2|4|8} → Msaa::SampleN`；
//! - 配方 `parameters` 按 catalog 参数 schema 校验（未声明键/类型或范围
//!   非法在编译期失败，line 248），并下降为动作图最前面的
//!   `apply_params` 节点（tick 0、声明序在配方动作之前）；参数与动作写
//!   同一目标时必须声明依赖，否则按未声明写冲突拒绝；
//! - 动作图沿用 scenario-model 的校验（循环/悬空/同 tick 写冲突）；目标
//!   作用域 `Entity`/`View` 在本阶段可绑定，`Material`/`Light` 在编译期
//!   以结构化错误拒绝（明确缺口，不静默降级）；
//! - 配方不携带任意代码：执行句柄只有 catalog 绑定的构建器函数指针，
//!   不进入身份载荷。
//!
//! 身份（R9 + lines 258-273）：计划摘要是 `Execution` 域的
//! `SHA-256(编译器版本 ‖ JCS{recipe hash, run 字段, session hash})`——
//! 同一配方 → 同一计划 → 同一摘要；RunSpec 绑定的 EvaluationSpec 身份
//! 被记录（不参与判定）。

use serde::Serialize;

use bevy::render::view::Msaa;
use bevy_stylized::prelude::StylizedExecutionTier;
use scenario_model::action::{ActionKind, TargetScope};
use scenario_model::canon::{self, CANON_COMPILER_VERSION};
use scenario_model::catalog::{self, CatalogError};
use scenario_model::identity::{self, ContractKind};
use scenario_model::recipe::{AntiAliasing, RecipeReference, RunSpec, SceneRecipe, SessionPlan};
use scenes::CameraRecipe;
use scenes::registry::{SceneBinding, bindings};

use crate::error::RuntimeError;

/// 编译失败（结构化；执行前失败）。
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum CompileError {
    /// 配方/动作图模型校验失败。
    #[error("invalid recipe: {0}")]
    Model(String),
    /// 未知场景 ID。
    #[error("unknown scene: {0}")]
    UnknownScene(String),
    /// catalog 未声明 preset，配方却声明了 preset。
    #[error("unknown preset {0:?} (catalog declares none; presets stay explicit)")]
    UnknownPreset(String),
    /// catalog 交叉校验失败（未声明资产依赖 / tier 不足）。
    #[error(transparent)]
    Catalog(#[from] CatalogError),
    /// 参数 schema 未绑定（H3）但配方声明了非空参数。
    #[error("recipe declares parameters but parameter-schema binding is H3 work: {0}")]
    Parameters(String),
    /// RunSpec 的配方引用与给定配方身份不符。
    #[error("run spec recipe reference does not match the provided recipe identity")]
    RecipeReferenceMismatch,
    /// 动作目标作用域本阶段不可绑定。
    #[error("target scope {0:?} is not bindable in this phase")]
    UnsupportedScope(TargetScope),
}

/// 不可变执行计划：内容绑定 + 相机 + 执行参数 + 有序动作 + 摘要。
///
/// 可审查：除 `binding`（构建器函数指针）外全部为数据；摘要只覆盖数据
/// 载荷（见模块文档）。
#[derive(Clone, Debug)]
pub struct ScenePlan {
    /// 内容构建绑定（catalog 1:1）。
    pub binding: SceneBinding,
    /// 运行时相机配方（编译期 f64→f32 映射）。
    pub camera: CameraRecipe,
    /// 捕获分辨率。
    pub resolution: (u32, u32),
    /// 声明的抗锯齿（显式插入相机；Off 或 SampleN）。
    pub msaa: Msaa,
    /// 请求执行 tier（封套/身份数据；宿主在 App 装配时应用）。
    pub tier: Option<StylizedExecutionTier>,
    /// 必需效果集合（编译期展开、排序去重后的声明数据）。
    pub required_effects: Vec<String>,
    /// 已校验动作图（声明序；driver 内部按稳定执行次序展开）。
    pub actions: scenario_model::action::ActionGraph,
    /// Execution 域计划摘要（小写十六进制）。
    pub digest: String,
}

#[derive(Serialize)]
struct PlanRunPayload<'a> {
    mode: serde_json::Value,
    sampling: &'a Option<scenario_model::recipe::SamplingRule>,
    evaluation: &'a Option<String>,
}

#[derive(Serialize)]
struct PlanDigestPayload<'a> {
    compiler_schema: bevy_stylized::prelude::StylizedSchemaVersion,
    recipe: String,
    run: PlanRunPayload<'a>,
    session: Option<String>,
}

/// 编译：`SceneRecipe + RunSpec (+ SessionPlan)` → 不可变 [`ScenePlan`]。
pub fn compile(
    recipe: &SceneRecipe,
    run: &RunSpec,
    session: Option<&SessionPlan>,
) -> Result<ScenePlan, CompileError> {
    recipe
        .validate()
        .map_err(|error| CompileError::Model(error.to_string()))?;
    recipe
        .actions
        .validate()
        .map_err(|error| CompileError::Model(error.to_string()))?;

    // 场景引用解析 + catalog 交叉校验（资产依赖 / tier）。
    let entry = scenario_model::catalog::catalog_entry(recipe.scene.id.as_str())
        .ok_or_else(|| CompileError::UnknownScene(recipe.scene.id.to_string()))?;
    if let Some(preset) = &recipe.scene.preset {
        return Err(CompileError::UnknownPreset(preset.to_string()));
    }
    catalog::require_declared_assets(recipe, &entry)?;
    let binding = bindings()
        .into_iter()
        .find(|binding| binding.id == entry.id.as_str())
        .ok_or_else(|| CompileError::UnknownScene(entry.id.to_string()))?;

    // 参数 schema 校验 + 下降（见模块文档）。
    let parameter_nodes = lower_parameters(&entry, recipe)?;

    // 动作目标作用域检查（Material/Light 本阶段不可绑定）。
    for node in &recipe.actions.nodes {
        let scope = match &node.kind {
            ActionKind::ApplyParams { target, .. }
            | ActionKind::Deform { target, .. }
            | ActionKind::Move { target, .. } => target.scope,
            _ => continue,
        };
        match scope {
            TargetScope::Entity | TargetScope::View => {}
            other => return Err(CompileError::UnsupportedScope(other)),
        }
    }

    // RunSpec 引用一致性。
    let recipe_hash = identity::recipe_identity(recipe)
        .map_err(|error| CompileError::Model(error.to_string()))?;
    match &run.recipe {
        RecipeReference::ByHash { hash } if *hash != recipe_hash => {
            return Err(CompileError::RecipeReferenceMismatch);
        }
        RecipeReference::Inline(inline) => {
            let inline_hash = identity::recipe_identity(inline)
                .map_err(|error| CompileError::Model(error.to_string()))?;
            if inline_hash != recipe_hash {
                return Err(CompileError::RecipeReferenceMismatch);
            }
        }
        _ => {}
    }

    // 相机映射（声明 f64 → 运行时 f32；见模块文档）。
    let position = &recipe.camera.position;
    let target = &recipe.camera.target;
    let mut camera = CameraRecipe::new(
        bevy::math::Vec3::new(
            position[0].get() as f32,
            position[1].get() as f32,
            position[2].get() as f32,
        ),
        bevy::math::Vec3::new(
            target[0].get() as f32,
            target[1].get() as f32,
            target[2].get() as f32,
        ),
    );
    let fov_degrees = recipe.camera.fov_degrees.get();
    if (fov_degrees - 45.0).abs() > f64::EPSILON {
        camera = camera.with_projection(bevy::camera::Projection::Perspective(
            bevy::camera::PerspectiveProjection {
                fov: (fov_degrees as f32).to_radians(),
                ..Default::default()
            },
        ));
    }

    let msaa = match recipe.anti_aliasing {
        AntiAliasing::None => Msaa::Off,
        AntiAliasing::Msaa { samples } => match samples {
            2 => Msaa::Sample2,
            4 => Msaa::Sample4,
            8 => Msaa::Sample8,
            other => {
                return Err(CompileError::Model(format!(
                    "illegal MSAA sample count {other}"
                )));
            }
        },
    };

    // 下降的参数节点在配方动作之前（声明序决胜）；合并后整体校验
    // （未声明的同 tick 写冲突在此失败）。
    let mut actions = scenario_model::action::ActionGraph {
        nodes: parameter_nodes,
    };
    actions.nodes.extend(recipe.actions.nodes.iter().cloned());
    actions
        .validate()
        .map_err(|error| CompileError::Model(format!("merged action graph: {error}")))?;

    let session_hash = match session {
        Some(plan) => Some(
            identity::session_identity(plan)
                .map_err(|error| CompileError::Model(error.to_string()))?,
        ),
        None => None,
    };
    let payload = PlanDigestPayload {
        compiler_schema: scenario_model::recipe::SCENE_RECIPE_SCHEMA,
        recipe: recipe_hash,
        run: PlanRunPayload {
            mode: serde_json::to_value(run.mode)
                .map_err(|error| CompileError::Model(error.to_string()))?,
            sampling: &run.sampling,
            evaluation: &run.evaluation,
        },
        session: session_hash,
    };
    let canonical =
        canon::canonical_bytes(&payload).map_err(|error| CompileError::Model(error.to_string()))?;
    let digest = identity::digest_hex(&identity::contract_digest(
        ContractKind::Execution,
        CANON_COMPILER_VERSION,
        &canonical,
    ));

    Ok(ScenePlan {
        binding,
        camera,
        resolution: (recipe.resolution.width, recipe.resolution.height),
        msaa,
        tier: recipe.tier,
        required_effects: recipe.required_effects.as_slice().to_vec(),
        actions,
        digest,
    })
}

/// 参数下降：每个声明参数变成一个 tick 0 的 `apply_params` 节点
/// （`param.<key>`，落点来自 catalog schema；与配方动作 ID 冲突即拒绝）。
fn lower_parameters(
    entry: &scenario_model::catalog::CatalogEntry,
    recipe: &SceneRecipe,
) -> Result<Vec<scenario_model::action::ActionNode>, CompileError> {
    use scenario_model::action::{ActionKind, ActionNode, TargetRef};
    use scenario_model::canon::{CanonicalU64, ParameterMap};
    use scenario_model::ident::{ActionId, TargetId};

    catalog::validate_parameters(entry, &recipe.parameters)
        .map_err(|error| CompileError::Parameters(error.to_string()))?;
    let mut nodes = Vec::with_capacity(recipe.parameters.len());
    for (key, value) in &recipe.parameters {
        let spec = entry
            .parameters
            .iter()
            .find(|spec| spec.name == *key)
            .expect("validated parameter key");
        let id = ActionId::new(format!("param.{key}"))
            .map_err(|error| CompileError::Parameters(error.to_string()))?;
        if recipe.actions.nodes.iter().any(|node| node.id == id) {
            return Err(CompileError::Parameters(format!(
                "parameter node id collides with a recipe action: param.{key}"
            )));
        }
        let target = TargetRef {
            scope: spec.target.scope,
            id: TargetId::new(spec.target.id.clone())
                .map_err(|error| CompileError::Parameters(error.to_string()))?,
        };
        let mut params = ParameterMap::new();
        params.insert(spec.target.field.clone(), value.clone());
        nodes.push(ActionNode {
            id,
            tick: CanonicalU64(0),
            depends_on: vec![],
            writes: vec![target.clone()],
            kind: ActionKind::ApplyParams { target, params },
        });
    }
    Ok(nodes)
}

/// 把编译错误映射为运行时错误（driver 入口用）。
#[must_use]
pub fn into_runtime(error: CompileError) -> RuntimeError {
    RuntimeError::Plan(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use scenario_model::canon::CanonicalSet;
    use scenario_model::recipe::{AssetSource, Backend, RunnerKind};
    use scenario_model::recipe::{RunMode, SESSION_PLAN_SCHEMA, SessionPlan};

    pub fn recipe_json(scene: &str, camera: &str, msaa: &str, actions: &str) -> String {
        format!(
            r#"{{
            "schema": {{"major": 1, "minor": 0}},
            "scene": {{"id": "{scene}", "preset": null}},
            "seed": "42",
            "rng": {{"algorithm": "chacha", "version": "1"}},
            "resolution": {{"width": 512, "height": 512}},
            "anti_aliasing": {{"type": "msaa", "samples": {msaa}}},
            "tier": null,
            "required_effects": [],
            "assets": [],
            "camera": {camera},
            "parameters": {{}},
            "actions": {{"nodes": {actions}}}
        }}"#
        )
    }

    /// courtyard 配方（声明 reference-scene 资产依赖；相机 = panorama 机位）。
    pub fn courtyard_json(camera: &str, msaa: &str, actions: &str) -> String {
        recipe_json("courtyard", camera, msaa, actions).replace(
            r#""assets": []"#,
            r#""assets": [{"id": "reference-scene", "source": "fixtures/synthetic/reference-scene", "sha256": null}]"#,
        )
    }

    pub const OUTLINE_CAMERA: &str = r#"{
        "position": [0.0, 0.4, 6.0],
        "target": [0.0, 0.0, 0.0],
        "fov_degrees": 45.0
    }"#;

    /// panorama 机位相机（与 scenes::metadata::COURTYARD_CAMERA_PRESETS[0] 一致）。
    pub const PANORAMA_CAMERA: &str = r#"{
        "position": [0.0, 3.4, 16.0],
        "target": [0.0, 1.3, -2.0],
        "fov_degrees": 45.0
    }"#;

    pub const SETTLE_OBSERVE: &str = r#"[
        {"id": "settle", "tick": "0", "depends_on": [], "writes": [],
         "kind": {"type": "fixed_step", "ticks": "22", "step": {"numerator": "1", "denominator": "60"}}},
        {"id": "shot", "tick": "0", "depends_on": ["settle"], "writes": [],
         "kind": {"type": "observe", "observation": "capture", "view": null}}
    ]"#;

    pub fn courtyard_recipe() -> SceneRecipe {
        SceneRecipe::from_json_slice(
            courtyard_json(PANORAMA_CAMERA, "4", SETTLE_OBSERVE).as_bytes(),
        )
        .expect("recipe")
    }

    pub fn capture_run(recipe: &SceneRecipe) -> RunSpec {
        RunSpec {
            schema: scenario_model::recipe::RUN_SPEC_SCHEMA,
            recipe: RecipeReference::Inline(Box::new(recipe.clone())),
            mode: RunMode::Capture,
            sampling: None,
            evaluation: None,
        }
    }

    fn session_plan() -> SessionPlan {
        SessionPlan {
            schema: SESSION_PLAN_SCHEMA,
            backend: Backend::Metal,
            device_features: CanonicalSet::new(vec![]),
            limits_profile: "portable-baseline".to_string(),
            tier_ceiling: StylizedExecutionTier::Standard,
            plugins: CanonicalSet::new(vec![]),
            asset_source: AssetSource::Filesystem,
            instrumentation: CanonicalSet::new(vec![]),
            runner: RunnerKind::Headless,
        }
    }

    #[test]
    fn compiles_courtyard_recipe_to_executable_plan() {
        let recipe = courtyard_recipe();
        let run = capture_run(&recipe);
        let plan = compile(&recipe, &run, None).expect("compile");
        assert_eq!(plan.binding.id, "courtyard");
        assert_eq!(plan.resolution, (512, 512));
        assert_eq!(plan.msaa, Msaa::Sample4);
        assert!(
            plan.camera.projection.is_none(),
            "45° maps to engine default"
        );
        let eye = plan.camera.eye;
        assert_eq!((eye.x, eye.y, eye.z), (0.0, 3.4, 16.0));
        assert_eq!(plan.actions.nodes.len(), 2);
        assert_eq!(plan.digest.len(), 64);
    }

    #[test]
    fn digest_is_stable_and_envelope_sensitive() {
        let recipe = courtyard_recipe();
        let run = capture_run(&recipe);
        let a = compile(&recipe, &run, None).expect("compile");
        let b = compile(&recipe, &run, None).expect("compile");
        assert_eq!(a.digest, b.digest, "same recipe → same plan → same digest");
        let with_session = compile(&recipe, &run, Some(&session_plan())).expect("compile");
        assert_ne!(
            a.digest, with_session.digest,
            "session envelope changes identity"
        );
        let with_eval = compile(
            &recipe,
            &RunSpec {
                evaluation: Some("evaluation-hash".to_string()),
                ..capture_run(&recipe)
            },
            None,
        )
        .expect("compile");
        assert_ne!(
            a.digest, with_eval.digest,
            "evaluation inputs change identity"
        );
        // 域分离：计划摘要（Execution）不同于配方身份（Recipe 域）。
        assert_ne!(
            a.digest,
            identity::recipe_identity(&recipe).expect("recipe hash")
        );
    }

    #[test]
    fn rejects_unknown_scene_preset_parameters_and_scope() {
        let run_for = |recipe: &SceneRecipe| capture_run(recipe);
        // 未知场景
        let unknown = SceneRecipe::from_json_slice(
            recipe_json("no-such-scene", OUTLINE_CAMERA, "4", SETTLE_OBSERVE).as_bytes(),
        )
        .expect("recipe");
        assert!(matches!(
            compile(&unknown, &run_for(&unknown), None),
            Err(CompileError::UnknownScene(_))
        ));
        // 声明 preset（catalog 未声明任何 preset）
        let preset = SceneRecipe::from_json_slice(
            courtyard_json(OUTLINE_CAMERA, "4", SETTLE_OBSERVE)
                .replace("\"preset\": null", "\"preset\": \"hybrid\"")
                .as_bytes(),
        )
        .expect("recipe");
        assert!(matches!(
            compile(&preset, &run_for(&preset), None),
            Err(CompileError::UnknownPreset(_))
        ));
        // 非空 parameters（schema 绑定属 H3）
        let params = SceneRecipe::from_json_slice(
            courtyard_json(OUTLINE_CAMERA, "4", SETTLE_OBSERVE)
                .replace(
                    "\"parameters\": {}",
                    "\"parameters\": {\"rim_strength\": 0.5}",
                )
                .as_bytes(),
        )
        .expect("recipe");
        assert!(matches!(
            compile(&params, &run_for(&params), None),
            Err(CompileError::Parameters(_))
        ));
        // Material 作用域动作
        let material_scope = SceneRecipe::from_json_slice(
            courtyard_json(
                OUTLINE_CAMERA,
                "4",
                r#"[{"id": "a", "tick": "0", "depends_on": [], "writes": [],
                    "kind": {"type": "apply_params",
                             "target": {"scope": "material", "id": "hero_face"},
                             "params": {}}}]"#,
            )
            .as_bytes(),
        )
        .expect("recipe");
        assert!(matches!(
            compile(&material_scope, &run_for(&material_scope), None),
            Err(CompileError::UnsupportedScope(TargetScope::Material))
        ));
    }

    #[test]
    fn rejects_catalog_contract_violations_and_reference_mismatch() {
        // 未声明资产依赖：courtyard 配方不带 reference-scene 资产声明。
        let undeclared = SceneRecipe::from_json_slice(
            recipe_json("courtyard", OUTLINE_CAMERA, "4", SETTLE_OBSERVE).as_bytes(),
        )
        .expect("recipe");
        let run = capture_run(&undeclared);
        assert!(matches!(
            compile(&undeclared, &run, None),
            Err(CompileError::Catalog(
                CatalogError::UndeclaredAssetDependency { .. }
            ))
        ));
        // tier 交叉校验机制的单元覆盖在 scenario-model recipe 测试（合成条目）；
        // 目录当前无 tier 门控场景（05 §10 退役后），此处不再重复。
        // ByHash 引用与配方身份不符。
        let recipe = courtyard_recipe();
        let mismatched = RunSpec {
            recipe: RecipeReference::ByHash {
                hash: "0".repeat(64),
            },
            ..capture_run(&recipe)
        };
        assert!(matches!(
            compile(&recipe, &mismatched, None),
            Err(CompileError::RecipeReferenceMismatch)
        ));
    }

    #[test]
    fn parameters_lower_to_leading_apply_params_nodes() {
        let recipe = SceneRecipe::from_json_slice(
            courtyard_json(OUTLINE_CAMERA, "4", SETTLE_OBSERVE)
                .replace(
                    "\"parameters\": {}",
                    "\"parameters\": {\"trio.face.1.face_threshold\": 0.2}",
                )
                .as_bytes(),
        )
        .expect("recipe");
        let run = capture_run(&recipe);
        let plan = compile(&recipe, &run, None).expect("compile");
        assert_eq!(plan.actions.nodes.len(), 3, "param node + 2 recipe actions");
        let first = &plan.actions.nodes[0];
        assert_eq!(first.id.as_str(), "param.trio.face.1.face_threshold");
        assert_eq!(first.tick.0, 0);
        match &first.kind {
            scenario_model::action::ActionKind::ApplyParams { target, params } => {
                assert_eq!(target.id.as_str(), "trio.face.1");
                assert_eq!(params["face_threshold"], serde_json::json!(0.2));
            }
            other => panic!("lowered parameter must be apply_params: {other:?}"),
        }
    }

    #[test]
    fn rejects_undeclared_out_of_range_and_wrong_typed_parameters() {
        let recipe_with = |parameters: &str| {
            SceneRecipe::from_json_slice(
                courtyard_json(OUTLINE_CAMERA, "4", SETTLE_OBSERVE)
                    .replace("\"parameters\": {}", parameters)
                    .as_bytes(),
            )
            .expect("recipe")
        };
        // 未声明键
        let unknown = recipe_with("\"parameters\": {\"trio.face.1.not_a_param\": 1.0}");
        let run = capture_run(&unknown);
        assert!(matches!(
            compile(&unknown, &run, None),
            Err(CompileError::Parameters(_))
        ));
        // 越界（crowd ≤ 1000）
        let out_of_range = recipe_with("\"parameters\": {\"crowd\": 1001}");
        let run = capture_run(&out_of_range);
        assert!(matches!(
            compile(&out_of_range, &run, None),
            Err(CompileError::Parameters(_))
        ));
        // 类型不符（face_threshold 必须是 null 或有限数值）
        let wrong_type = recipe_with("\"parameters\": {\"trio.face.1.face_threshold\": \"high\"}");
        let run = capture_run(&wrong_type);
        assert!(matches!(
            compile(&wrong_type, &run, None),
            Err(CompileError::Parameters(_))
        ));
    }

    #[test]
    fn non_default_fov_maps_to_explicit_projection() {
        let recipe = SceneRecipe::from_json_slice(
            courtyard_json(
                r#"{"position": [0.0, 0.4, 6.0], "target": [0.0, 0.0, 0.0], "fov_degrees": 30.0}"#,
                "4",
                SETTLE_OBSERVE,
            )
            .as_bytes(),
        )
        .expect("recipe");
        let run = capture_run(&recipe);
        let plan = compile(&recipe, &run, None).expect("compile");
        match plan.camera.projection {
            Some(bevy::camera::Projection::Perspective(perspective)) => {
                assert!((perspective.fov - 30.0_f32.to_radians()).abs() < 1e-6);
            }
            other => panic!("non-default fov must map to explicit projection: {other:?}"),
        }
    }
}
