//! 观测归一：把 harness 的中性捕获回执映射到工作台身份
//! （docs/showcase/01 §场景、配方与验证身份 / line 184）。
//!
//! 回执是纯数据：判定编排不在本层（validation 后续消费同一观测流）。
//! `sample_id` 标记一次被批准的时间/历史采样——每次 `observe` 分配新的
//! 采样身份，恢复或重新采样同一 tick 不复用（见 driver 的分配器）。

use scenario_model::ident::{Generation, RequestId, SampleId, SessionId, ViewId};

use golden_harness::{CaptureStamp, CapturedImage};

/// 观测回执：身份作用域 + 实验 tick + 中性 stamp + 不可变像素快照。
#[derive(Clone, Debug)]
pub struct ObservationReceipt {
    /// 一次实际 App/Device 生命周期。
    pub session: SessionId,
    /// 实验内容的一次实例化。
    pub generation: Generation,
    /// 视图身份（实验内唯一）。
    pub view: ViewId,
    /// 一次具体读回。
    pub request: RequestId,
    /// 一次被批准的时间/历史采样。
    pub sample: SampleId,
    /// 观测名（动作图的 `observe` 键）。
    pub observation: String,
    /// 发出观测时点的实验 tick（时钟域：ExperimentClock）。
    pub experiment_tick: u64,
    /// harness 中性捕获回执（时钟域定义见 golden-harness capture 模块文档）。
    pub stamp: CaptureStamp,
    /// 像素快照（独立拥有，见 harness 的快照不可变性）。
    pub image: CapturedImage,
}

/// 检查点记录：已应用命令水位 + 实验时钟。
#[derive(Clone, Debug, PartialEq)]
pub struct CheckpointRecord {
    /// 检查点标签（动作图的 `checkpoint` 键）。
    pub label: String,
    /// 记录时点的实验 tick。
    pub experiment_tick: u64,
    /// 已应用的动作节点数（稳定执行次序水位）。
    pub applied_actions: usize,
    /// 记录时点的内容代次。
    pub generation: Generation,
}
