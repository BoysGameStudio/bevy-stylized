//! 两阶段能力生命周期管线（ADR-0022）。
//!
//! - **Stage A**（RenderDevice 创建前）：[`StylizedCapabilityConfig`] 由用户在
//!   构建 `App` 前设定，经
//!   [`apply_to_wgpu_settings`](StylizedCapabilityConfig::apply_to_wgpu_settings)
//!   贡献进 `WgpuSettings`；
//! - **Stage B**（RenderDevice 创建后）：[`StylizedPlugin`](crate::StylizedPlugin)
//!   的 `finish` 只读 `RenderDevice::features()/limits()`，产出
//!   [`StylizedResolvedCapabilities`]；无 RenderPlugin 时落 CPU 兜底
//!   （`StylizedCapabilitySet::default()`，Portable 基线）。
//!
//! 本模块是 core 能力模型与 wgpu 类型之间的**唯一**映射点。

use std::collections::BTreeSet;

use bevy::{
    prelude::Resource,
    render::{render_resource::WgpuFeatures, settings::WgpuSettings},
};
use bevy_stylized_core::{
    StylizedCapabilitySet, StylizedExecutionTier, StylizedFallbackPolicy, StylizedFallbackReport,
    StylizedGpuFeature, StylizedGpuLimits, StylizedResolveError, StylizedResolvedVariant,
    StylizedSurfaceClass, resolve_with_report,
};

/// Stage A 配置：用户在构建 `App` / 创建 RenderDevice **之前**设定。
///
/// 注意（ADR-0022）：本配置只能**请求**特性；设备实际启用了什么以
/// Stage B 的 `RenderDevice` 读数为准。Portable 默认值不请求任何可选
/// 特性、不要求超过 Bevy 基线的 limits。
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct StylizedCapabilityConfig {
    /// 期望的 Surface Class。
    pub requested_class: StylizedSurfaceClass,
    /// 期望的 Execution Tier。
    pub requested_tier: StylizedExecutionTier,
    /// 期望的 tier 需求之外，由可选高端路径追加请求的 GPU 特性
    /// （ADR-0022 Stage A 的贡献项）。
    pub requested_features: BTreeSet<StylizedGpuFeature>,
    /// 能力不满足时的 fallback 政策（永不降低 Surface Class）。
    pub fallback_policy: StylizedFallbackPolicy,
}

impl StylizedCapabilityConfig {
    /// 把请求的特性并入 `WgpuSettings`（Stage A 贡献点）。
    ///
    /// 用法：在 `DefaultPlugins.set(RenderPlugin { render_creation, .. })`
    /// 之前构造 settings，或在自定义 `RenderCreation` 闭包中调用。
    pub fn apply_to_wgpu_settings(&self, settings: &mut WgpuSettings) {
        settings.features |= requested_wgpu_features(&self.requested_features);
    }

    /// 以本配置对实际能力做确定性解析（Stage B 核心，纯函数转发；
    /// 返回 variant + 人可读 FallbackReport——降了什么、为什么）。
    pub fn resolve(
        &self,
        actual: &StylizedCapabilitySet,
    ) -> Result<(StylizedResolvedVariant, StylizedFallbackReport), StylizedResolveError> {
        resolve_with_report(
            self.requested_class,
            self.requested_tier,
            actual,
            self.fallback_policy,
        )
    }
}

/// Stage B 结果：实际能力快照 + 解析出的运行时 variant。
///
/// 有 RenderPlugin 时由 `RenderDevice` 读数构造；headless（无 RenderPlugin）
/// 时为 Portable 基线的 CPU 兜底。
#[derive(Resource, Clone, Debug)]
pub struct StylizedResolvedCapabilities {
    /// 设备实际能力（Stage B 只读快照）。
    pub actual: StylizedCapabilitySet,
    /// 按 [`StylizedCapabilityConfig`] 解析出的 variant。
    pub resolved: Result<StylizedResolvedVariant, StylizedResolveError>,
    /// 人可读 fallback 报告（解析成功时存在）。
    pub report: Option<StylizedFallbackReport>,
}

/// backend-neutral 特性集合 → wgpu feature 位（Stage A 映射）。
///
/// 映射说明：
///
/// - `Float16` → `SHADER_F16`；`Subgroup` → `SUBGROUP`；
/// - `Bindless` → binding-array 相关位集合；
/// - `Compute` / `HdrOutput` / `StorageBufferInVertexStage` 在 wgpu 中
///   不是 device feature（由 backend/limits/格式决定），此处不贡献位，
///   由 Stage B 经 limits 读回。
#[must_use]
pub fn requested_wgpu_features(features: &BTreeSet<StylizedGpuFeature>) -> WgpuFeatures {
    let mut out = WgpuFeatures::empty();
    for feature in features {
        out |= match feature {
            StylizedGpuFeature::Float16 => WgpuFeatures::SHADER_F16,
            StylizedGpuFeature::Subgroup => WgpuFeatures::SUBGROUP,
            StylizedGpuFeature::Bindless => {
                WgpuFeatures::TEXTURE_BINDING_ARRAY
                    | WgpuFeatures::STORAGE_RESOURCE_BINDING_ARRAY
                    | WgpuFeatures::BUFFER_BINDING_ARRAY
                    | WgpuFeatures::PARTIALLY_BOUND_BINDING_ARRAY
            }
            StylizedGpuFeature::Compute
            | StylizedGpuFeature::HdrOutput
            | StylizedGpuFeature::StorageBufferInVertexStage => WgpuFeatures::empty(),
        };
    }
    out
}

/// wgpu 设备能力 → backend-neutral 能力集合（Stage B 映射）。
///
/// 说明：`Compute` / `StorageBufferInVertexStage` 由 limits 推断
/// （> 0 即视为可用）；`HdrOutput` 无法从 device 推导（view/格式层面），
/// 永不置位。
#[must_use]
pub fn stylized_capabilities_from_wgpu(
    features: WgpuFeatures,
    limits: &bevy::render::render_resource::WgpuLimits,
) -> StylizedCapabilitySet {
    let mut out_features = BTreeSet::new();
    if features.contains(WgpuFeatures::SHADER_F16) {
        out_features.insert(StylizedGpuFeature::Float16);
    }
    if features.contains(WgpuFeatures::SUBGROUP) {
        out_features.insert(StylizedGpuFeature::Subgroup);
    }
    if features.contains(WgpuFeatures::TEXTURE_BINDING_ARRAY)
        && features.contains(WgpuFeatures::STORAGE_RESOURCE_BINDING_ARRAY)
    {
        out_features.insert(StylizedGpuFeature::Bindless);
    }
    if limits.max_compute_workgroup_size_x > 0 {
        out_features.insert(StylizedGpuFeature::Compute);
    }
    if limits.max_storage_buffers_per_shader_stage > 0 {
        out_features.insert(StylizedGpuFeature::StorageBufferInVertexStage);
    }
    StylizedCapabilitySet {
        features: out_features,
        limits: StylizedGpuLimits {
            max_bind_groups: limits.max_bind_groups,
            max_storage_buffers_per_shader_stage: limits.max_storage_buffers_per_shader_stage,
            max_uniform_buffers_per_shader_stage: limits.max_uniform_buffers_per_shader_stage,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_default_config_requests_nothing() {
        let config = StylizedCapabilityConfig::default();
        assert_eq!(config.requested_tier, StylizedExecutionTier::Portable);
        assert!(config.requested_features.is_empty());

        let mut settings = WgpuSettings::default();
        let before = settings.features;
        config.apply_to_wgpu_settings(&mut settings);
        assert_eq!(settings.features, before);
    }

    #[test]
    fn stage_a_helper_contributes_requested_features() {
        let config = StylizedCapabilityConfig {
            requested_features: BTreeSet::from([StylizedGpuFeature::Float16]),
            ..StylizedCapabilityConfig::default()
        };
        let mut settings = WgpuSettings::default();
        config.apply_to_wgpu_settings(&mut settings);
        assert!(settings.features.contains(WgpuFeatures::SHADER_F16));
        assert!(!settings.features.contains(WgpuFeatures::SUBGROUP));
    }

    #[test]
    fn wgpu_roundtrip_mapping() {
        let limits = bevy::render::render_resource::WgpuLimits {
            max_storage_buffers_per_shader_stage: 4,
            max_compute_workgroup_size_x: 64,
            ..bevy::render::render_resource::WgpuLimits::default()
        };
        let caps = stylized_capabilities_from_wgpu(
            WgpuFeatures::SHADER_F16 | WgpuFeatures::SUBGROUP,
            &limits,
        );
        assert!(caps.features.contains(&StylizedGpuFeature::Float16));
        assert!(caps.features.contains(&StylizedGpuFeature::Subgroup));
        assert!(caps.features.contains(&StylizedGpuFeature::Compute));
        assert!(
            caps.features
                .contains(&StylizedGpuFeature::StorageBufferInVertexStage)
        );
        assert!(!caps.features.contains(&StylizedGpuFeature::HdrOutput));
    }
}
