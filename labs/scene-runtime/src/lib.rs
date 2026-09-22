//! `scene-runtime`：统一跨平台场景工作台的运行时层骨架
//! （docs/showcase/01 §依赖与代码所有权，Phase F）。
//!
//! 职责：非阻塞 [`CaseDriver`]、会话重配置协调（[`session`]）、场景所有权
//! 与生命周期（[`scene`]）、动作调度（依赖 + 稳定次序）、观测归一
//! （[`observe`]）。本层依赖 model/scenes 与中性 harness；通过宿主接口
//! （[`session::SessionHost`]）请求设备操作，**不持有平台事件循环**：
//! 宿主每帧调用一次 [`CaseDriver::update`]，渲染推进由宿主 pump
//! （桌面 headless 包装见 [`headless`]；Android/Web 宿主在各自 runner 中
//! 逐帧推进，属 phase G）。
//!
//! 时钟与身份（R12/R13 + phase E 证据层）：
//!
//! - **实验 tick**（[`clock::ExperimentClock`]）只由 `fixed_step` 动作推进；
//!   等待（`wait_ready`、观测在途）与宿主泵送次数不改变实验状态；
//! - 观测回执（[`observe::ObservationReceipt`]）把 harness 的中性
//!   `CaptureStamp` 映射到工作台身份（SessionId / Generation / ViewId /
//!   RequestId / SampleId），判定编排不在本层（validation 后续消费）；
//! - 严格 GPU gate 的每用例进程隔离不变（R4）：driver 的共享会话模式只
//!   用于交互与明确的生命周期测试。
//!
//! 已知缺口（不在本阶段关闭）：产品公共重配置 API（`StylizedPlugin::finish`
//! 当前只初始化能力结果，设备重建归产品/宿主）；配方编译器
//! （SceneRecipe→ScenePlan 的展开）；APK/浏览器资产桥接；消费者迁移
//! （showcase/tests/regression 上 driver，phase G）。

pub mod clock;
pub mod compiler;
pub mod controller;
pub mod cycle;
pub mod driver;
pub mod error;
pub mod headless;
pub mod journal;
pub mod observe;
pub mod scene;
pub mod session;

pub use clock::ExperimentClock;
pub use compiler::{CompileError, ScenePlan, compile};
pub use controller::{
    CaseEntry, CommandOutcome, CommandPlan, RefusalReason, SessionCommand, SessionController,
};
pub use cycle::{CaptureCycle, CycleResource, Settle, capture_in_app, set_cycle_projection};
pub use driver::{CaseDriver, DriverLimits, DriverStatus};
pub use error::RuntimeError;
pub use headless::run_to_completion;
pub use journal::{
    COMMAND_JOURNAL_SCHEMA, CommandJournal, JournalEntry, JournalEntryState, JournalError,
    ReplayReport,
};
pub use observe::{CheckpointRecord, ObservationReceipt};
pub use scene::{SceneCounts, SceneLedger};
pub use session::{
    CoordState, ReceiptVerdict, SceneSwitchVerdict, SessionCheckpoint, SessionCoordinator,
    SessionEvent, SessionHost,
};
