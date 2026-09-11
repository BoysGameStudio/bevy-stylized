//! Stage B 能力消费（ADR-0022）。
//!
//! Facade 的 `StylizedPlugin` 在 Stage B 完成解析后把结果写入
//! [`StylizedRenderCapabilities`]；本 crate 的渲染路径只读它：
//!
//! - Portable 切片本身不要求任何可选 GPU 特性（治理不变量 6/8）；
//! - 解析失败时 fail-closed（debug 构建 panic + error 日志），绝不带着
//!   未满足的假设继续建 pipeline。

use bevy::prelude::Resource;
use bevy_stylized_core::{StylizedCapabilitySet, StylizedExecutionTier, StylizedResolvedVariant};

/// Stage B 解析结果在渲染侧的只读快照（由 facade 写入）。
#[derive(Resource, Clone, Debug)]
pub struct StylizedRenderCapabilities {
    /// 设备实际能力。
    pub actual: StylizedCapabilitySet,
    /// 解析出的运行时 variant；`None` 表示解析失败（fail-closed）。
    pub resolved: Option<StylizedResolvedVariant>,
    /// 解析失败的错误描述。
    pub resolve_error: Option<String>,
    /// 硬失败：连 Portable 基线都不满足（`blocking_tier == Portable`）。
    /// 区别于"用户显式选择 Fail 政策而拒绝高档位"的预期失败。
    pub hard_failure: bool,
}

impl StylizedRenderCapabilities {
    /// 由 Stage B 结果构造。
    #[must_use]
    pub fn new(
        actual: &StylizedCapabilitySet,
        resolved: Result<StylizedResolvedVariant, bevy_stylized_core::StylizedResolveError>,
    ) -> Self {
        match resolved {
            Ok(variant) => Self {
                actual: actual.clone(),
                resolved: Some(variant),
                resolve_error: None,
                hard_failure: false,
            },
            Err(error) => Self {
                actual: actual.clone(),
                hard_failure: error.blocking_tier == StylizedExecutionTier::Portable,
                resolve_error: Some(error.to_string()),
                resolved: None,
            },
        }
    }

    /// Cel@Portable 切片的能力门槛：variant 解析成功即可（Portable 不要求
    /// 任何可选特性）。
    #[must_use]
    pub fn cel_portable_satisfied(&self) -> bool {
        self.resolved.is_some()
    }
}

/// fail-closed 检查：
///
/// - 解析成功：放行；
/// - 解析失败但属预期（Fail 政策拒绝高档位）：error 日志，渲染侧可自行
///   检查 [`StylizedRenderCapabilities::cel_portable_satisfied`]；
/// - 硬失败（Portable 基线都不满足）：error 日志 + debug 断言中断——
///   设备连最低预算都达不到，继续建 pipeline 没有任何意义。
pub fn debug_assert_capabilities_satisfied(capabilities: Option<&StylizedRenderCapabilities>) {
    match capabilities {
        None => {
            bevy::log::debug!(
                "bevy-stylized-render: StylizedRenderCapabilities not present; \
                 assuming default Portable baseline (headless or StylizedPlugin absent)"
            );
        }
        Some(capabilities) => {
            if let Some(error) = &capabilities.resolve_error {
                bevy::log::error!("bevy-stylized-render: capability resolution failed: {error}");
            }
            debug_assert!(
                !capabilities.hard_failure,
                "bevy-stylized-render: device cannot satisfy even the Portable baseline: {:?}",
                capabilities.resolve_error
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_stylized_core::{
        StylizedExecutionTier, StylizedFallbackPolicy, StylizedGpuLimits, StylizedSurfaceClass,
        resolve_variant,
    };

    #[test]
    fn failed_resolution_is_not_satisfied() {
        let actual = StylizedCapabilitySet {
            limits: bevy_stylized_core::StylizedGpuLimits {
                max_bind_groups: 2,
                ..StylizedGpuLimits::default()
            },
            ..StylizedCapabilitySet::default()
        };
        let resolved = resolve_variant(
            StylizedSurfaceClass::Cel,
            StylizedExecutionTier::Portable,
            &actual,
            StylizedFallbackPolicy::ReduceTier,
        );
        let capabilities = StylizedRenderCapabilities::new(&actual, resolved);
        assert!(!capabilities.cel_portable_satisfied());
        assert!(capabilities.resolve_error.is_some());
        assert!(capabilities.hard_failure);
    }

    #[test]
    fn successful_resolution_is_satisfied() {
        let actual = StylizedCapabilitySet::default();
        let resolved = resolve_variant(
            StylizedSurfaceClass::Cel,
            StylizedExecutionTier::Portable,
            &actual,
            StylizedFallbackPolicy::ReduceTier,
        );
        let capabilities = StylizedRenderCapabilities::new(&actual, resolved);
        assert!(capabilities.cel_portable_satisfied());
        assert_eq!(
            capabilities.resolved.map(|v| v.tier),
            Some(StylizedExecutionTier::Portable)
        );
    }
}
