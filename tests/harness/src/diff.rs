//! Per-pixel image diffing 已提取至 `labs/validation`
//! （docs/showcase/01：纯 CPU 比较算法归 validation，阈值与数值行为不变）。
//! 本模块保持 `golden_harness::diff` 的既有路径与类型，仅做再导出。

pub use validation::diff::{DiffError, DiffReport, DiffStats, Tolerance, diff_rgba8, evaluate};
