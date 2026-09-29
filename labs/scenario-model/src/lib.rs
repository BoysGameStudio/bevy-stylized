//! `scenario-model`：统一跨平台场景工作台的纯数据合同
//! （docs/showcase/01，Proposed design 的第一阶段实现）。
//!
//! 职责边界（docs/showcase/01 §依赖与代码所有权）：目录类型、配方/动作/
//! 观测/协议 schema、规范化编译、身份与约束展开。本 crate 无 App、GPU、UI、
//! 文件系统依赖；产品语义（执行 tier、schema 版本载体）引用
//! `bevy-stylized-core` 的现有纯数据合同，不复制字段定义。
//!
//! 模块：
//!
//! - [`ident`]：SceneId/preset/session_id/generation/revision/sample_id/
//!   ViewId/request_id 等作用域 newtype；
//! - [`recipe`]：[`SceneRecipe`](recipe::SceneRecipe) /
//!   [`RunSpec`](recipe::RunSpec) / [`SessionPlan`](recipe::SessionPlan) /
//!   [`EvaluationSpec`](recipe::EvaluationSpec)，含版本门控摄入与合同校验；
//! - [`action`]：动作图（施加参数、变形/移动、等待就绪、固定步进、观测、
//!   检查点），整数 tick + 有理步长，拒绝循环/悬空引用/未声明同 tick 写冲突；
//! - [`canon`]：两层规范化——语义层（精度映射、非有限拒绝、±0 归一、
//!   64 位整数十进制字符串、集合排序去重、重复属性名拒绝）与字节层
//!   （RFC 8785 JCS：UTF-16 码元属性排序、ECMAScript 数字表示、UTF-8）；
//! - [`identity`]：recipe/session/execution/artifact 四域 SHA-256，
//!   结构化长度前缀编码；
//! - [`catalog`]：纯数据场景目录（courtyard 单场景的元数据；
//!   所有者位置与构建符号记录为数据，绑定核对由 `labs/scenes` 执行）。

pub mod action;
pub mod canon;
pub mod capture_spec;
pub mod catalog;
pub mod coverage;
pub mod ident;
pub mod identity;
pub mod recipe;

pub use action::{
    ActionError, ActionGraph, ActionKind, ActionNode, ReadyCondition, Step, TargetRef, TargetScope,
    Tick,
};
pub use canon::{
    CANON_COMPILER_VERSION, CanonError, CanonicalSet, CanonicalU64, FiniteF32, FiniteF64,
    ParameterMap, canonical_bytes, ecmascript_number_string, jcs_bytes, parse_strict,
};
pub use capture_spec::{
    CaptureSpec, CaptureSpecError, WholeFrameJudgment, courtyard_capture_specs,
    validate_capture_specs,
};
pub use catalog::{
    BuilderBinding, CATALOG_LEN, CatalogEntry, CatalogError, ParameterError, ParameterKind,
    ParameterSpec, ParameterTarget, catalog, catalog_entry, require_declared_assets,
    validate_catalog, validate_parameters,
};
pub use coverage::{
    CaptureCoverage, CoverageReport, CoverageState, FamilyCoverage, Judgment, coverage_json,
    coverage_report,
};
pub use ident::{
    ActionId, Generation, IdentError, PresetId, RequestId, Revision, SampleId, SceneId, SessionId,
    TargetId, ViewId,
};
pub use identity::{ContractKind, contract_digest, digest_hex, recipe_identity, session_identity};
pub use recipe::{
    AntiAliasing, AssetDecl, AssetSource, Backend, CameraSpec, ComparisonDomain, EvaluationError,
    EvaluationSpec, IngestError, RecipeError, RecipeReference, Resolution, RngDecl, RunMode,
    RunSpec, RunSpecError, RunnerKind, SamplingRule, SceneRecipe, SceneReference, SessionPlan,
    SessionPlanError,
};
