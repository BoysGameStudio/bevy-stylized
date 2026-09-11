//! 质量政策（Quality Policy）。
//!
//! 质量政策是"在某一设备上如何把 authoring data 解析为 runtime variant"
//! 的声明式配置。它与 Cargo feature（ADR-0024：纯编译期 additive 能力）
//! 完全解耦：tier 选择只由设备能力与政策决定。

use bevy_reflect::Reflect;

use crate::{
    axis::{StylizedExecutionTier, StylizedValidationMode},
    capability::StylizedFallbackPolicy,
};

/// 质量政策：tier 选择的默认意图与失败行为。
#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StylizedQualityPolicy {
    /// 期望的执行档位（设备能力不足时按 `fallback_policy` 处理）。
    pub default_tier: StylizedExecutionTier,
    /// 能力不满足时的 fallback 政策（永不降低 Surface Class）。
    pub fallback_policy: StylizedFallbackPolicy,
    /// 验证模式（独立于 tier 的第三轴）。
    pub validation_mode: StylizedValidationMode,
    /// 是否允许 Profile 声明的 Surface-Class 语义 LOD。
    ///
    /// 默认 `false`：只有 Profile 显式声明语义允许的跨 Class LOD 时
    /// 才可置 `true`（docs/architecture/01）。
    pub allow_surface_class_lod: bool,
}

impl Default for StylizedQualityPolicy {
    fn default() -> Self {
        Self {
            default_tier: StylizedExecutionTier::Portable,
            fallback_policy: StylizedFallbackPolicy::default(),
            validation_mode: StylizedValidationMode::default(),
            allow_surface_class_lod: false,
        }
    }
}
