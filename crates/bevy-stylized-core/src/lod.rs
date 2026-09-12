//! NPR LOD 政策类型（V2-b，docs/roadmap/01 §NPR LOD；provisional pending
//! ADR-0018 的 LOD 落地条款）。
//!
//! 不变量：**LOD 只削减 tier/feature/几何，永不降低 Surface Class**
//! （Hero 远处仍是 Hero；ADR-0018 三轴不变量）。
//!
//! 语义（roadmap V2）：
//!
//! ```text
//! Hero Near → Hero @ High
//! Hero Mid  → Hero @ Standard
//! Hero Far  → Hero @ Portable
//! Very Far  → Profile 声明的简化 variant
//! ```

use bevy_reflect::Reflect;

use crate::{axis::StylizedExecutionTier, capability::StylizedFallbackStep};

/// 距离分带的阈值（单调递增，构造时校验）。
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StylizedLodThresholds {
    /// near→mid 起始距离。
    pub mid_start: f32,
    /// mid→far 起始距离。
    pub far_start: f32,
    /// far→very_far 起始距离。
    pub very_far_start: f32,
}

impl Default for StylizedLodThresholds {
    fn default() -> Self {
        Self {
            mid_start: 10.0,
            far_start: 25.0,
            very_far_start: 50.0,
        }
    }
}

impl StylizedLodThresholds {
    /// 校验单调递增（fail-closed）。
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.mid_start < self.far_start && self.far_start < self.very_far_start
    }
}

/// 单个距离带的行为声明。
#[derive(Clone, Debug, Default, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StylizedLodBandPolicy {
    /// 该带的 tier 上限（`None` = 不限制）。
    pub tier_cap: Option<StylizedExecutionTier>,
    /// 该带削减的 feature（按 per-class fallback 链的步骤语义）。
    pub drop_features: Vec<StylizedFallbackStep>,
    /// Profile 声明的简化 variant 名（very far 带；`None` = 无）。
    pub simplified_variant: Option<String>,
}

/// 完整 LOD 政策（4 带：near / mid / far / very_far）。
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StylizedLodPolicy {
    /// 距离阈值。
    pub thresholds: StylizedLodThresholds,
    /// 各带行为（[near, mid, far, very_far]）。
    pub bands: [StylizedLodBandPolicy; 4],
}

impl Default for StylizedLodPolicy {
    fn default() -> Self {
        Self {
            thresholds: StylizedLodThresholds::default(),
            bands: [
                StylizedLodBandPolicy::default(), // near: 不动
                StylizedLodBandPolicy {
                    tier_cap: Some(StylizedExecutionTier::Standard),
                    ..StylizedLodBandPolicy::default()
                },
                StylizedLodBandPolicy {
                    tier_cap: Some(StylizedExecutionTier::Portable),
                    drop_features: vec![
                        StylizedFallbackStep::DropMatcap,
                        StylizedFallbackStep::DropSpecRamp,
                        StylizedFallbackStep::DropPackedNormal,
                    ],
                    ..StylizedLodBandPolicy::default()
                },
                StylizedLodBandPolicy {
                    tier_cap: Some(StylizedExecutionTier::Portable),
                    drop_features: vec![
                        StylizedFallbackStep::DropMatcap,
                        StylizedFallbackStep::DropSpecRamp,
                        StylizedFallbackStep::DropPackedNormal,
                        StylizedFallbackStep::FaceSdfHardStep,
                        StylizedFallbackStep::HairSpecularToRim,
                    ],
                    simplified_variant: None,
                },
            ],
        }
    }
}

/// LOD 解析结果。
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StylizedLodResolution {
    /// 命中的带索引（0 = near … 3 = very_far）。
    pub band: u32,
    /// 带的 tier 上限。
    pub tier_cap: Option<StylizedExecutionTier>,
    /// 削减的 feature 列表。
    pub drop_features: Vec<StylizedFallbackStep>,
    /// profile 声明的简化 variant（若有）。
    pub simplified_variant: Option<String>,
}

/// 确定性距离分带：同距离同政策必得同带。
///
/// 非法阈值（非单调）fail-closed 返回 near 带。
#[must_use]
pub fn resolve_lod_band(distance: f32, thresholds: &StylizedLodThresholds) -> u32 {
    if !thresholds.is_valid() || distance < thresholds.mid_start {
        return 0;
    }
    if distance < thresholds.far_start {
        1
    } else if distance < thresholds.very_far_start {
        2
    } else {
        3
    }
}

/// 解析某距离的 LOD 行为（纯函数，确定性）。
#[must_use]
pub fn resolve_lod(distance: f32, policy: &StylizedLodPolicy) -> StylizedLodResolution {
    let band = resolve_lod_band(distance, &policy.thresholds);
    let band_policy = &policy.bands[band as usize];
    StylizedLodResolution {
        band,
        tier_cap: band_policy.tier_cap,
        drop_features: band_policy.drop_features.clone(),
        simplified_variant: band_policy.simplified_variant.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_boundaries_are_deterministic() {
        let thresholds = StylizedLodThresholds::default();
        assert_eq!(resolve_lod_band(0.0, &thresholds), 0);
        assert_eq!(resolve_lod_band(9.99, &thresholds), 0);
        assert_eq!(resolve_lod_band(10.0, &thresholds), 1);
        assert_eq!(resolve_lod_band(24.99, &thresholds), 1);
        assert_eq!(resolve_lod_band(25.0, &thresholds), 2);
        assert_eq!(resolve_lod_band(49.99, &thresholds), 2);
        assert_eq!(resolve_lod_band(50.0, &thresholds), 3);
        assert_eq!(resolve_lod_band(500.0, &thresholds), 3);
    }

    #[test]
    fn invalid_thresholds_fall_back_to_near() {
        let thresholds = StylizedLodThresholds {
            mid_start: 30.0,
            far_start: 10.0,
            very_far_start: 50.0,
        };
        assert!(!thresholds.is_valid());
        assert_eq!(resolve_lod_band(100.0, &thresholds), 0);
    }

    #[test]
    fn default_policy_far_drops_hero_features_but_keeps_class() {
        let policy = StylizedLodPolicy::default();
        let far = resolve_lod(30.0, &policy);
        assert_eq!(far.band, 2);
        assert_eq!(far.tier_cap, Some(StylizedExecutionTier::Portable));
        assert!(
            far.drop_features
                .contains(&StylizedFallbackStep::DropMatcap)
        );
        let near = resolve_lod(1.0, &policy);
        assert_eq!(near.band, 0);
        assert_eq!(near.tier_cap, None);
        assert!(near.drop_features.is_empty());
    }
}
