//! `validation`：统一跨平台场景工作台的纯 CPU 判定层
//! （docs/showcase/01 §依赖与代码所有权）。
//!
//! 职责：对观测执行断言——独立期望、比较域、容差和失败解释。只依赖纯数据
//! 与 CPU 算法；不引用产品/场景类型，不要求 App/Device，无文件系统/GPU
//! 依赖。GPU 观测转为纯数据（[`ImageView`]）后交给本层，判定逻辑可在设备
//! 执行，也可在主机对取回的相同产物执行。
//!
//! 本阶段是纯算法提取：[`diff`] 与 [`tier2`] 自 `tests/harness` 迁入，
//! 阈值与数值行为逐位保留（docs/reference/08 的容差合同不变），
//! `tests/harness` 保留中性 pipeline 探针、捕获关联与 GPU 观测，并以
//! 相同签名委托本 crate。`EvaluationSpec`（`scenario-model`）的绑定属于
//! 后续阶段，本阶段不引入模型依赖。

pub mod diff;
pub mod frame;
pub mod image;
pub mod tier2;

pub use image::ImageView;
