//! 运行时能力重配置（ADR-0030；扩展 ADR-0022 的两阶段生命周期，不改动其
//! "设备创建后不得追加启用特性"的不变量）。
//!
//! - **会话内可变**：`requested_class` / `requested_tier` / `fallback_policy`
//!   及断言的 `requested_features`，前提是设备 Stage B 快照已具备；
//! - **需要新 device 会话**：任何 Device features/limits 变更——本模块绝不
//!   重读设备，只对 Stage B 存入的 `actual` 快照做同一确定性解析
//!   （`resolve_with_report`）；
//! - **原子应用**：成功时在同一个 `&mut World` 作用域内更新
//!   [`StylizedCapabilityConfig`]、[`StylizedResolvedCapabilities`] 与
//!   （render feature）`StylizedRenderCapabilities` 及其派生状态，代次 +1
//!   并发出 [`StylizedCapabilitiesChanged`]；失败时资源保持逐字节不变，
//!   结构化 [`StylizedReconfigError`] 列出缺失能力，绝不部分应用。
//!
//! 入口是 `World` 扩展方法而非延迟消息：重配置是会话协调事务，其结果
//! （应用/拒绝）必须在依赖动作（如场景切换）之前同步可知（ADR-0030
//! 选项 3）。

use bevy::prelude::{Message, Resource, World};
use bevy_stylized_core::{
    StylizedFallbackReport, StylizedGpuFeature, StylizedResolveError, StylizedResolvedVariant,
};

use crate::capability::{StylizedCapabilityConfig, StylizedResolvedCapabilities};

/// 重配置代次：每次应用*实际变更*单调递增；幂等 no-op 不递增。
///
/// 跨代次的捕获不属于同一执行条件（宿主记录捕获时应带上本代次）。
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct StylizedReconfigGeneration(pub u64);

/// 能力变更通知（仅在实际变更后发出；observer/宿主经 MessageReader 消费）。
#[derive(Message, Clone, Debug)]
pub struct StylizedCapabilitiesChanged {
    /// 变更后的代次。
    pub generation: u64,
    /// 变更前解析出的 variant（初次解析失败时为 `None`）。
    pub previous: Option<StylizedResolvedVariant>,
    /// 变更后解析出的 variant。
    pub current: StylizedResolvedVariant,
    /// 本次解析的人可读 fallback 报告。
    pub report: StylizedFallbackReport,
}

/// 重配置应用结果（同步返回给事务调用方）。
#[derive(Clone, Debug)]
pub struct StylizedReconfigApplied {
    /// 当前代次（no-op 时不变更）。
    pub generation: u64,
    /// 是否发生了实际变更（幂等重放同一请求为 `false`）。
    pub changed: bool,
    /// 变更前解析出的 variant（初次解析失败时为 `None`）。
    pub previous: Option<StylizedResolvedVariant>,
    /// 本次解析出的 variant。
    pub current: StylizedResolvedVariant,
    /// 本次解析的人可读 fallback 报告。
    pub report: StylizedFallbackReport,
}

/// 结构化重配置拒绝（ADR-0030：绝不部分应用）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StylizedReconfigError {
    /// 会话尚无 Stage B 结果（`StylizedResolvedCapabilities` 缺失），
    /// 无法尝试重配置。
    NoCapabilitySnapshot,
    /// 请求断言的设备特性不在 Stage B 快照中——需要新 device 会话
    /// （本 API 永远不能追加启用设备特性）。
    FeaturesNotEnabled {
        /// 快照中缺失的特性（确定性排序）。
        missing: Vec<StylizedGpuFeature>,
    },
    /// 确定性解析失败（含 `blocking_tier` / `missing_features` /
    /// `limits_satisfied`）。
    ResolutionFailed(StylizedResolveError),
}

impl std::fmt::Display for StylizedReconfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoCapabilitySnapshot => write!(
                f,
                "no Stage B capability snapshot in this session; reconfiguration cannot be attempted"
            ),
            Self::FeaturesNotEnabled { missing } => write!(
                f,
                "requested features are not enabled on this device (a new device session is required): {missing:?}"
            ),
            Self::ResolutionFailed(error) => write!(f, "capability re-resolution failed: {error}"),
        }
    }
}

impl std::error::Error for StylizedReconfigError {}

/// 运行时重配置入口（ADR-0030）。`StylizedPlugin` 登记
/// [`StylizedReconfigGeneration`] 与 [`StylizedCapabilitiesChanged`] 消息；
/// 手工装配的世界不登记消息时仅跳过通知，语义不变。
pub trait StylizedReconfigurationExt {
    /// 对当前会话请求一次能力重配置：校验 → 对 Stage B 快照确定性重解析
    /// → 原子应用或结构化拒绝。
    fn apply_stylized_reconfiguration(
        &mut self,
        config: StylizedCapabilityConfig,
    ) -> Result<StylizedReconfigApplied, StylizedReconfigError>;
}

impl StylizedReconfigurationExt for World {
    fn apply_stylized_reconfiguration(
        &mut self,
        config: StylizedCapabilityConfig,
    ) -> Result<StylizedReconfigApplied, StylizedReconfigError> {
        // 只读 Stage B 快照；绝不重读 RenderDevice（ADR-0022/0030）。
        let (actual, previous) = {
            let snapshot = self
                .get_resource::<StylizedResolvedCapabilities>()
                .ok_or(StylizedReconfigError::NoCapabilitySnapshot)?;
            (
                snapshot.actual.clone(),
                snapshot.resolved.as_ref().ok().cloned(),
            )
        };

        // 断言的设备特性必须已在快照中启用（缺失即需要新 device 会话）。
        let missing: Vec<StylizedGpuFeature> = config
            .requested_features
            .difference(&actual.features)
            .copied()
            .collect();
        if !missing.is_empty() {
            return Err(StylizedReconfigError::FeaturesNotEnabled { missing });
        }

        // 同一确定性解析器、同一快照（与 Stage B 完全同源）。
        let (variant, report) = config
            .resolve(&actual)
            .map_err(StylizedReconfigError::ResolutionFailed)?;

        // 幂等：请求配置与解析结果都与现状一致 → 不换代次、不发通知。
        let incumbent_config = self.get_resource::<StylizedCapabilityConfig>();
        let changed = incumbent_config != Some(&config) || previous.as_ref() != Some(&variant);
        let generation = {
            let mut generation = self
                .get_resource_mut::<StylizedReconfigGeneration>()
                .map(|g| *g)
                .unwrap_or_default();
            if changed {
                generation.0 += 1;
                self.insert_resource(generation);
            }
            generation.0
        };

        if changed {
            // 同一 &mut World 作用域内原子更新全部已解析状态。
            self.insert_resource(config);
            self.insert_resource(StylizedResolvedCapabilities {
                actual: actual.clone(),
                resolved: Ok(variant.clone()),
                report: Some(report.clone()),
            });
            #[cfg(feature = "render")]
            {
                self.insert_resource(bevy_stylized_render::StylizedRenderCapabilities::new(
                    &actual,
                    Ok(variant.clone()),
                ));
                // render crate 拥有的能力派生状态在同事务内刷新（ADR-0030 §6）。
                bevy_stylized_render::instance::refresh_capability_dependents(self);
            }
            if let Some(mut messages) =
                self.get_resource_mut::<bevy::ecs::message::Messages<StylizedCapabilitiesChanged>>()
            {
                messages.write(StylizedCapabilitiesChanged {
                    generation,
                    previous: previous.clone(),
                    current: variant.clone(),
                    report: report.clone(),
                });
            }
        }

        Ok(StylizedReconfigApplied {
            generation,
            changed,
            previous,
            current: variant,
            report,
        })
    }
}
