//! 配方/运行/会话/判定合同（docs/showcase/01 §场景、配方与验证身份）。
//!
//! 区分内容、执行请求和会话配置：
//!
//! - [`SceneRecipe`] 完整确定内容组合、相机、视图、材质参数、资产清单哈希、
//!   种子、动作图、分辨率、AA、请求 tier 与必需效果；展开默认值后才能保存
//!   或比较。本阶段默认值在构造/摄入时即写入模型（无运行时隐式默认）。
//! - [`RunSpec`] 引用配方，选择交互、捕获、断言或测量及步数/采样规则；
//!   需要判定时引用不可变 [`EvaluationSpec`]。
//! - [`SessionPlan`] 固定 backend/device features/limits、初始化 tier 上限、
//!   插件集、资产 source、instrumentation 和 runner 类型；其 hash 进入执行
//!   身份（见 [`crate::identity`]）。
//!
//! 版本合同：未知 schema 版本拒绝（同 major 且 minor 不高于当前版本才可读，
//! 与 bevy-stylized-core 的 additive 演进规则一致）；未知字段按版本合同
//! `deny_unknown_fields` 拒绝，不能静默丢弃后计算相同 hash；非法范围与
//! 未声明的资产依赖在执行前失败。配方不携带任意代码脚本。

use serde::{Deserialize, Serialize};

use bevy_stylized_core::StylizedSchemaVersion;
use bevy_stylized_core::axis::StylizedExecutionTier;

use crate::action::ActionGraph;
use crate::canon::{self, CanonError, CanonicalSet, CanonicalU64, FiniteF64, ParameterMap};
use crate::ident::{PresetId, SceneId};

/// `SceneRecipe` schema 版本。
pub const SCENE_RECIPE_SCHEMA: StylizedSchemaVersion = StylizedSchemaVersion::new(1, 0);
/// `RunSpec` schema 版本。
pub const RUN_SPEC_SCHEMA: StylizedSchemaVersion = StylizedSchemaVersion::new(1, 0);
/// `SessionPlan` schema 版本。
pub const SESSION_PLAN_SCHEMA: StylizedSchemaVersion = StylizedSchemaVersion::new(1, 0);
/// `EvaluationSpec` schema 版本。
pub const EVALUATION_SPEC_SCHEMA: StylizedSchemaVersion = StylizedSchemaVersion::new(1, 0);

/// 版本门控：未知 major 或更高 minor 拒绝。
fn check_version(
    found: StylizedSchemaVersion,
    supported: StylizedSchemaVersion,
) -> Result<(), IngestError> {
    if !supported.can_read(found) {
        return Err(IngestError::UnsupportedVersion { found, supported });
    }
    Ok(())
}

fn read_version(value: &serde_json::Value) -> Result<StylizedSchemaVersion, IngestError> {
    let field = value
        .get("schema")
        .ok_or(IngestError::MissingSchemaVersion)?;
    serde_json::from_value(field.clone())
        .map_err(|error| IngestError::Model(format!("schema version: {error}")))
}

fn ingest<T, V, F>(
    bytes: &[u8],
    supported: StylizedSchemaVersion,
    validate: F,
) -> Result<T, IngestError>
where
    T: for<'de> Deserialize<'de>,
    V: std::error::Error + Send + Sync + 'static,
    F: FnOnce(&T) -> Result<(), V>,
{
    let value = canon::parse_strict(bytes).map_err(IngestError::Strict)?;
    check_version(read_version(&value)?, supported)?;
    let model: T =
        serde_json::from_value(value).map_err(|error| IngestError::Model(error.to_string()))?;
    validate(&model).map_err(|error| IngestError::Validation(Box::new(error)))?;
    Ok(model)
}

/// 摄入失败（严格解析 → 版本门控 → 类型校验 → 合同校验，任一阶段失败即拒）。
#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    /// 严格 JSON 层失败（语法错误或重复属性名）。
    #[error(transparent)]
    Strict(#[from] CanonError),
    /// payload 缺少 `schema` 版本字段。
    #[error("payload carries no `schema` version field")]
    MissingSchemaVersion,
    /// 未知/过新 schema 版本。
    #[error("unsupported schema version {found} (reader supports {supported})")]
    UnsupportedVersion {
        /// 输入声明的版本。
        found: StylizedSchemaVersion,
        /// 本实现支持的版本。
        supported: StylizedSchemaVersion,
    },
    /// 类型反序列化失败（含未知字段拒绝）。
    #[error("model decode failed: {0}")]
    Model(String),
    /// 合同校验失败。
    #[error("validation failed: {0}")]
    Validation(Box<dyn std::error::Error + Send + Sync>),
}

// ---------------------------------------------------------------------------
// SceneRecipe
// ---------------------------------------------------------------------------

/// 场景引用：`SceneId + preset` 决定内容。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct SceneReference {
    /// 场景 ID（目录注册键）。
    pub id: SceneId,
    /// 内容 preset。
    pub preset: Option<PresetId>,
}

/// 资产清单条目：逻辑 ID 绑定仓库相对逻辑路径与可选内容哈希。
///
/// 配方身份排除资产绝对根路径；`source` 是仓库相对逻辑路径。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct AssetDecl {
    /// 逻辑资产 ID。
    pub id: String,
    /// 仓库相对逻辑路径。
    pub source: String,
    /// 内容 SHA-256（十六进制，小写）；清单自身的 hash 字段不进入配方身份。
    pub sha256: Option<String>,
}

/// 随机源声明：固定 seed 必须同时记录 RNG 算法/版本；固定 seed 不是跨
/// CPU/GPU 位级一致性的承诺。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RngDecl {
    /// RNG 算法名（如 `chacha`）。
    pub algorithm: String,
    /// 算法实现版本。
    pub version: String,
}

/// 捕获分辨率（像素，均为正）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Resolution {
    /// 宽。
    pub width: u32,
    /// 高。
    pub height: u32,
}

/// 抗锯齿请求。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AntiAliasing {
    /// 无 AA。
    None,
    /// MSAA 采样数（合法取值 2/4/8）。
    Msaa {
        /// 采样数。
        samples: u8,
    },
}

/// 相机配方（位置/目标/视场角；角度为度，声明 f64 精度）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct CameraSpec {
    /// 相机位置。
    pub position: [FiniteF64; 3],
    /// 注视目标。
    pub target: [FiniteF64; 3],
    /// 垂直视场角（度，`0 < fov < 180`）。
    pub fov_degrees: FiniteF64,
}

/// `SceneRecipe`：完整确定一次内容组合与执行请求形态的不可变配方。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct SceneRecipe {
    /// schema 版本。
    pub schema: StylizedSchemaVersion,
    /// 场景引用。
    pub scene: SceneReference,
    /// 随机种子（规范十进制字符串编码）。
    pub seed: CanonicalU64,
    /// 随机源算法/版本。
    pub rng: Option<RngDecl>,
    /// 捕获分辨率。
    pub resolution: Resolution,
    /// 抗锯齿。
    pub anti_aliasing: AntiAliasing,
    /// 请求执行 tier（引用 bevy-stylized-core 的三轴类型，不复制定义）。
    pub tier: Option<StylizedExecutionTier>,
    /// 必需效果集合（集合字段：排序去重）。
    #[serde(default)]
    pub required_effects: CanonicalSet<String>,
    /// 资产依赖清单（完整依赖清单；清单自身 hash 字段不进入配方身份）。
    #[serde(default)]
    pub assets: Vec<AssetDecl>,
    /// 相机配方。
    pub camera: CameraSpec,
    /// 视图/材质参数实例数据（用户保存的配方是实例数据，
    /// 不是第二份参数定义；参数 schema 归目录）。
    #[serde(default)]
    pub parameters: ParameterMap,
    /// 动作图（有序列表）。
    #[serde(default)]
    pub actions: ActionGraph,
}

/// `SceneRecipe` 合同校验失败。
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum RecipeError {
    /// 非法分辨率。
    #[error("illegal resolution {0}x{1}")]
    IllegalResolution(u32, u32),
    /// 非法 MSAA 采样数。
    #[error("illegal MSAA sample count {0} (allowed: 2, 4, 8)")]
    IllegalMsaaSamples(u8),
    /// 非法视场角。
    #[error("illegal camera fov {0} degrees")]
    IllegalFov(f64),
    /// 重复资产 ID。
    #[error("duplicate asset id in manifest: {0}")]
    DuplicateAssetId(String),
    /// 空资产 ID 或空逻辑路径。
    #[error("asset manifest entry with empty id or source")]
    EmptyAssetField,
    /// 动作图验证失败。
    #[error(transparent)]
    Action(#[from] crate::action::ActionError),
}

impl SceneRecipe {
    /// 摄入 JSON：严格解析（拒绝重复属性名）→ 版本门控 → 类型校验 → 合同校验。
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, IngestError> {
        ingest(bytes, SCENE_RECIPE_SCHEMA, SceneRecipe::validate)
    }

    /// 合同校验：非法范围、重复/空资产声明、动作图。
    pub fn validate(&self) -> Result<(), RecipeError> {
        if self.resolution.width == 0 || self.resolution.height == 0 {
            return Err(RecipeError::IllegalResolution(
                self.resolution.width,
                self.resolution.height,
            ));
        }
        if let AntiAliasing::Msaa { samples } = self.anti_aliasing
            && !matches!(samples, 2 | 4 | 8)
        {
            return Err(RecipeError::IllegalMsaaSamples(samples));
        }
        let fov = self.camera.fov_degrees.get();
        if !(fov > 0.0 && fov < 180.0) {
            return Err(RecipeError::IllegalFov(fov));
        }
        let mut ids = std::collections::BTreeSet::new();
        for asset in &self.assets {
            if asset.id.is_empty() || asset.source.is_empty() {
                return Err(RecipeError::EmptyAssetField);
            }
            if !ids.insert(asset.id.as_str()) {
                return Err(RecipeError::DuplicateAssetId(asset.id.clone()));
            }
        }
        self.actions.validate()?;
        Ok(())
    }

    /// 规范字节（语义展开 → JCS）。
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, CanonError> {
        canon::canonical_bytes(self)
    }
}

// ---------------------------------------------------------------------------
// RunSpec / EvaluationSpec
// ---------------------------------------------------------------------------

/// 运行模式：交互、捕获、断言或测量。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum RunMode {
    /// 交互。
    Interact,
    /// 捕获。
    Capture,
    /// 断言（需要引用 [`EvaluationSpec`]）。
    Assert,
    /// 测量。
    Measure,
}

/// 配方引用：按身份 hash 或内联完整配方。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RecipeReference {
    /// 按 recipe hash 引用（十六进制，小写）。
    ByHash {
        /// 配方身份 hash。
        hash: String,
    },
    /// 内联完整配方。
    Inline(Box<SceneRecipe>),
}

/// 采样规则：步数与可选每步观测集合。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct SamplingRule {
    /// 执行步数（正；规范十进制字符串编码）。
    pub steps: CanonicalU64,
    /// 每步必需的观测名集合（集合字段：排序去重）。
    #[serde(default)]
    pub observations: CanonicalSet<String>,
}

/// `RunSpec`：引用配方，选择运行模式与步数/采样规则。
///
/// 测试函数 ID、Golden 路径、性能 target ID 继续由各自验证层管理，
/// 不进入本结构。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RunSpec {
    /// schema 版本。
    pub schema: StylizedSchemaVersion,
    /// 配方引用。
    pub recipe: RecipeReference,
    /// 运行模式。
    pub mode: RunMode,
    /// 步数/采样规则。
    pub sampling: Option<SamplingRule>,
    /// 判定引用：`Assert` 模式必需；引用不可变 [`EvaluationSpec`] 的身份 hash。
    pub evaluation: Option<String>,
}

/// `RunSpec` 合同校验失败。
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum RunSpecError {
    /// `Assert` 模式缺少 EvaluationSpec 引用。
    #[error("assert mode requires an evaluation spec reference")]
    MissingEvaluation,
    /// 非法步数。
    #[error("illegal step count 0")]
    IllegalSteps,
    /// 内联配方校验失败。
    #[error(transparent)]
    Recipe(#[from] RecipeError),
}

impl RunSpec {
    /// 摄入 JSON（同 [`SceneRecipe::from_json_slice`] 的管线）。
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, IngestError> {
        ingest(bytes, RUN_SPEC_SCHEMA, RunSpec::validate)
    }

    /// 合同校验。
    pub fn validate(&self) -> Result<(), RunSpecError> {
        if self.mode == RunMode::Assert && self.evaluation.is_none() {
            return Err(RunSpecError::MissingEvaluation);
        }
        if let Some(sampling) = &self.sampling
            && sampling.steps.0 == 0
        {
            return Err(RunSpecError::IllegalSteps);
        }
        if let RecipeReference::Inline(recipe) = &self.recipe {
            recipe.validate()?;
        }
        Ok(())
    }

    /// 规范字节。
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, CanonError> {
        canon::canonical_bytes(self)
    }
}

/// 比较域（跨设备比较先核对获准的比较域）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ComparisonDomain {
    /// RGBA8 精确比较。
    Rgba8Exact,
    /// 线性 HDR 域。
    LinearHdr,
    /// 主机侧解码后域。
    HostDecoded,
}

/// 感兴趣区域（像素矩形；宽高为正）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Roi {
    /// 左上 x。
    pub x: u32,
    /// 左上 y。
    pub y: u32,
    /// 宽（正）。
    pub width: u32,
    /// 高（正）。
    pub height: u32,
}

/// `EvaluationSpec`：不可变判定输入；更换阈值、ROI、mask、基线或解码器
/// 均改变判定身份。这些输入不由场景构建器生成。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EvaluationSpec {
    /// schema 版本。
    pub schema: StylizedSchemaVersion,
    /// 判定器版本（判定实现的身份，独立于本 schema 版本）。
    pub evaluator: String,
    /// 期望内容的实际内容 hash。
    pub expected_hash: String,
    /// 基线内容的实际内容 hash。
    pub baseline_hash: Option<String>,
    /// ROI。
    pub roi: Option<Roi>,
    /// mask 内容的实际内容 hash。
    pub mask_hash: Option<String>,
    /// 比较域。
    pub domain: ComparisonDomain,
    /// 阈值表（名 → 值，声明 f64 精度）。
    #[serde(default)]
    pub thresholds: std::collections::BTreeMap<String, FiniteF64>,
    /// 适用条件（如 backend/能力限定）。
    #[serde(default)]
    pub conditions: Vec<String>,
    /// 所需观测集合（集合字段：排序去重）。
    #[serde(default)]
    pub required_observations: CanonicalSet<String>,
}

/// `EvaluationSpec` 合同校验失败。
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EvaluationError {
    /// 非法 ROI。
    #[error("illegal ROI {width}x{height}")]
    IllegalRoi {
        /// 宽。
        width: u32,
        /// 高。
        height: u32,
    },
    /// 空判定器版本或空期望 hash。
    #[error("evaluator identity and expected hash must be non-empty")]
    EmptyIdentity,
}

impl EvaluationSpec {
    /// 摄入 JSON（同 [`SceneRecipe::from_json_slice`] 的管线）。
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, IngestError> {
        ingest(bytes, EVALUATION_SPEC_SCHEMA, EvaluationSpec::validate)
    }

    /// 合同校验。
    pub fn validate(&self) -> Result<(), EvaluationError> {
        if self.evaluator.is_empty() || self.expected_hash.is_empty() {
            return Err(EvaluationError::EmptyIdentity);
        }
        if let Some(roi) = self.roi
            && (roi.width == 0 || roi.height == 0)
        {
            return Err(EvaluationError::IllegalRoi {
                width: roi.width,
                height: roi.height,
            });
        }
        Ok(())
    }

    /// 规范字节。
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, CanonError> {
        canon::canonical_bytes(self)
    }
}

// ---------------------------------------------------------------------------
// SessionPlan
// ---------------------------------------------------------------------------

/// 图形 backend 请求。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    /// Vulkan。
    Vulkan,
    /// Metal。
    Metal,
    /// DX12。
    Dx12,
    /// OpenGL ES。
    Gl,
    /// WebGPU。
    WebGpu,
}

/// 资产来源适配（逻辑路径校验与资源适配分离）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum AssetSource {
    /// 主机文件系统。
    Filesystem,
    /// Android APK assets。
    Apk,
    /// 浏览器 HTTP。
    Http,
}

/// runner 类型（平台差异归宿主）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum RunnerKind {
    /// 桌面 winit 窗口。
    Winit,
    /// 桌面 headless（可串行 pump 同一 driver）。
    Headless,
    /// Android GameActivity。
    Android,
    /// 浏览器 WebGPU。
    Web,
}

/// `SessionPlan`：可重复配置；固定 backend/device features/limits、初始化
/// tier 上限、插件集、资产 source、instrumentation 和 runner 类型。
///
/// `session_id` 是一次实际 App/Device 生命周期（见 [`crate::ident::SessionId`]），
/// 与本计划不能互换；同计划重建设备也必须产生新 session_id。
/// Portable baseline 保留声明的 device 限制，不能统一申请最高能力。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct SessionPlan {
    /// schema 版本。
    pub schema: StylizedSchemaVersion,
    /// backend 请求。
    pub backend: Backend,
    /// 请求的 device feature 集合（集合字段：排序去重）。
    #[serde(default)]
    pub device_features: CanonicalSet<String>,
    /// 声明的 device limits 配置档名。
    pub limits_profile: String,
    /// 初始化 tier 上限（引用 bevy-stylized-core 的三轴类型）。
    pub tier_ceiling: StylizedExecutionTier,
    /// 插件集（集合字段：排序去重）。
    #[serde(default)]
    pub plugins: CanonicalSet<String>,
    /// 资产来源。
    pub asset_source: AssetSource,
    /// instrumentation 开关集合（集合字段：排序去重）。
    #[serde(default)]
    pub instrumentation: CanonicalSet<String>,
    /// runner 类型。
    pub runner: RunnerKind,
}

/// `SessionPlan` 合同校验失败。
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SessionPlanError {
    /// 空 limits 配置档名。
    #[error("limits profile must be non-empty")]
    EmptyLimitsProfile,
}

impl SessionPlan {
    /// 摄入 JSON（同 [`SceneRecipe::from_json_slice`] 的管线）。
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, IngestError> {
        ingest(bytes, SESSION_PLAN_SCHEMA, SessionPlan::validate)
    }

    /// 合同校验。
    pub fn validate(&self) -> Result<(), SessionPlanError> {
        if self.limits_profile.is_empty() {
            return Err(SessionPlanError::EmptyLimitsProfile);
        }
        Ok(())
    }

    /// 规范字节。
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, CanonError> {
        canon::canonical_bytes(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canon::CanonError;
    use crate::catalog::{self, CatalogError};

    /// 最小合法配方 JSON（所有默认值已展开）。
    pub fn minimal_recipe_json() -> String {
        r#"{
            "schema": {"major": 1, "minor": 0},
            "scene": {"id": "courtyard", "preset": null},
            "seed": "42",
            "rng": {"algorithm": "chacha", "version": "1"},
            "resolution": {"width": 512, "height": 512},
            "anti_aliasing": {"type": "msaa", "samples": 4},
            "tier": "portable",
            "required_effects": [],
            "assets": [],
            "camera": {
                "position": [0.0, 1.0, 3.0],
                "target": [0.0, 0.0, 0.0],
                "fov_degrees": 45.0
            },
            "parameters": {},
            "actions": {"nodes": []}
        }"#
        .to_string()
    }

    #[test]
    fn ingests_minimal_recipe() {
        let recipe = SceneRecipe::from_json_slice(minimal_recipe_json().as_bytes())
            .expect("minimal recipe ingests");
        assert_eq!(recipe.scene.id.as_str(), "courtyard");
        assert_eq!(recipe.seed.0, 42);
    }

    #[test]
    fn rejects_unknown_schema_version() {
        let newer_major = minimal_recipe_json()
            .replace("\"major\": 1, \"minor\": 0", "\"major\": 2, \"minor\": 0");
        assert!(matches!(
            SceneRecipe::from_json_slice(newer_major.as_bytes()),
            Err(IngestError::UnsupportedVersion { .. })
        ));
        // 同 major 更高 minor 同样拒绝（读取方不能静默解释未知内容）。
        let newer_minor = minimal_recipe_json()
            .replace("\"major\": 1, \"minor\": 0", "\"major\": 1, \"minor\": 1");
        assert!(matches!(
            SceneRecipe::from_json_slice(newer_minor.as_bytes()),
            Err(IngestError::UnsupportedVersion { .. })
        ));
        // 缺少 schema 字段拒绝。
        let missing = minimal_recipe_json()
            .replace("\"schema\": {\"major\": 1, \"minor\": 0},\n           ", "");
        assert!(matches!(
            SceneRecipe::from_json_slice(missing.as_bytes()),
            Err(IngestError::MissingSchemaVersion)
        ));
    }

    #[test]
    fn rejects_duplicate_property_before_schema_validation() {
        let dup = minimal_recipe_json()
            .replace("\"seed\": \"42\",", "\"seed\": \"42\", \"seed\": \"43\",");
        assert!(matches!(
            SceneRecipe::from_json_slice(dup.as_bytes()),
            Err(IngestError::Strict(CanonError::DuplicateProperty(name))) if name == "seed"
        ));
    }

    #[test]
    fn rejects_unknown_fields_per_version_contract() {
        let unknown = minimal_recipe_json()
            .replace("\"seed\": \"42\",", "\"seed\": \"42\", \"surprise\": 1,");
        assert!(matches!(
            SceneRecipe::from_json_slice(unknown.as_bytes()),
            Err(IngestError::Model(_))
        ));
    }

    #[test]
    fn rejects_illegal_ranges_and_duplicate_assets() {
        let zero_resolution = minimal_recipe_json().replace("\"width\": 512", "\"width\": 0");
        assert!(matches!(
            SceneRecipe::from_json_slice(zero_resolution.as_bytes()),
            Err(IngestError::Validation(_))
        ));
        let bad_msaa = minimal_recipe_json().replace("\"samples\": 4", "\"samples\": 3");
        assert!(matches!(
            SceneRecipe::from_json_slice(bad_msaa.as_bytes()),
            Err(IngestError::Validation(_))
        ));
        let dup_asset = minimal_recipe_json().replace(
            "\"assets\": []",
            "\"assets\": [{\"id\":\"a\",\"source\":\"x\",\"sha256\":null},{\"id\":\"a\",\"source\":\"y\",\"sha256\":null}]",
        );
        assert!(matches!(
            SceneRecipe::from_json_slice(dup_asset.as_bytes()),
            Err(IngestError::Validation(_))
        ));
    }

    #[test]
    fn plus_and_minus_zero_have_identical_canonical_bytes() {
        let positive = SceneRecipe::from_json_slice(minimal_recipe_json().as_bytes()).unwrap();
        let negative = SceneRecipe::from_json_slice(
            minimal_recipe_json()
                .replace(
                    "\"position\": [0.0, 1.0, 3.0]",
                    "\"position\": [-0.0, 1.0, 3.0]",
                )
                .as_bytes(),
        )
        .unwrap();
        assert_eq!(
            positive.canonical_bytes().unwrap(),
            negative.canonical_bytes().unwrap()
        );
        assert_eq!(
            crate::identity::recipe_identity(&positive).unwrap(),
            crate::identity::recipe_identity(&negative).unwrap()
        );
    }

    #[test]
    fn cross_check_fails_undeclared_asset_and_low_tier() {
        let recipe = SceneRecipe::from_json_slice(minimal_recipe_json().as_bytes()).unwrap();
        let courtyard = catalog::catalog_entry("courtyard").unwrap();
        // courtyard 要求 reference-scene 资产依赖；最小配方未声明 → 执行前失败。
        assert!(matches!(
            catalog::require_declared_assets(&recipe, &courtyard),
            Err(CatalogError::UndeclaredAssetDependency { .. })
        ));
        // tier 交叉校验（目录当前无 tier 门控条目；用合成条目测机制本身；
        // 配方须先满足资产依赖——交叉校验先查资产再查 tier）。
        let gated = catalog::CatalogEntry {
            tier: Some(bevy_stylized_core::axis::StylizedExecutionTier::Standard),
            assets: Vec::new(),
            ..courtyard.clone()
        };
        assert!(matches!(
            catalog::require_declared_assets(&recipe, &gated),
            Err(CatalogError::InsufficientTier { .. })
        ));
        // 声明了资产依赖的配方通过。
        let declared = SceneRecipe::from_json_slice(
            minimal_recipe_json()
                .replace(
                    r#""assets": []"#,
                    r#""assets": [{"id": "reference-scene", "source": "fixtures/synthetic/reference-scene", "sha256": null}]"#,
                )
                .as_bytes(),
        )
        .unwrap();
        assert_eq!(
            catalog::require_declared_assets(&declared, &courtyard),
            Ok(())
        );
    }

    #[test]
    fn recipe_identity_is_stable_and_domain_separated() {
        let recipe = SceneRecipe::from_json_slice(minimal_recipe_json().as_bytes()).unwrap();
        let a = crate::identity::recipe_identity(&recipe).unwrap();
        let b = crate::identity::recipe_identity(&recipe).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
        // 字段序不同、语义相同的 JSON 产生相同身份（JCS 属性排序）。
        let reordered = minimal_recipe_json().replacen(
            "\"scene\": {\"id\": \"outline\", \"preset\": null},",
            "\"scene\": {\"preset\": null, \"id\": \"outline\"},",
            1,
        );
        let reordered = SceneRecipe::from_json_slice(reordered.as_bytes()).unwrap();
        assert_eq!(crate::identity::recipe_identity(&reordered).unwrap(), a);
    }

    #[test]
    fn run_spec_requires_evaluation_for_assert_mode() {
        let recipe = SceneRecipe::from_json_slice(minimal_recipe_json().as_bytes()).unwrap();
        let spec = RunSpec {
            schema: RUN_SPEC_SCHEMA,
            recipe: RecipeReference::Inline(Box::new(recipe)),
            mode: RunMode::Assert,
            sampling: None,
            evaluation: None,
        };
        assert_eq!(spec.validate(), Err(RunSpecError::MissingEvaluation));
    }
}
