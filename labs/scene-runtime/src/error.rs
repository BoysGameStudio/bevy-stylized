//! 结构化运行时错误：不以 panic 充当控制流。

/// `CaseDriver` / 会话协调的结构化失败原因。
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum RuntimeError {
    /// 执行计划级拒绝（动作图循环/悬空/冲突等；来自 scenario-model 校验）。
    #[error("invalid execution plan: {0}")]
    Plan(String),
    /// 就绪等待超过有界泵送预算。
    #[error("readiness wait exceeded its bounded pump budget")]
    ReadinessTimeout,
    /// 就绪探针报告管线错误。
    #[error("pipeline error: {0}")]
    Pipeline(String),
    /// 观测读回超过有界泵送预算。
    #[error("observation readback exceeded its bounded pump budget")]
    ReadbackTimeout,
    /// 桌面 headless 包装的泵送预算耗尽。
    #[error("pump budget exhausted before completion")]
    PumpBudgetExhausted,
    /// 骨架期未接线的动作目标绑定（apply_params/deform/move；phase G）。
    #[error("action target binding not wired in this phase: {0}")]
    UnsupportedAction(&'static str),
    /// 场景构建/切换失败。
    #[error("scene error: {0}")]
    Scene(String),
    /// 会话协调失败。
    #[error("session error: {0}")]
    Session(String),
}
