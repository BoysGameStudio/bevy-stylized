//! Whole-pipeline 资源预算模型（docs/rendering/01；interim，pending
//! ADR-0002）。
//!
//! 预算对象不是单个材质，而是整条 pipeline 的 per-stage 可见性：
//!
//! ```text
//! View + Mesh + Instance + Material + Profile + Safety Margin
//!     = Whole-Pipeline Stage Budget
//! ```
//!
//! 本模块只携带**数据**（baseline 限制模型 + 用量结构）；测量与反射在
//! `bevy-stylized-render`（GPU-gated 测试）。

use bevy_reflect::Reflect;

/// 一个 baseline 限制模型（per-stage 上限；数值来自 wgpu-types 29 的
/// `Limits::defaults()` / `downlevel_defaults()` / `downlevel_webgl2_defaults()`，
/// 与 docs/rendering/01 的 baseline 表一致）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StylizedBudgetLimits {
    /// 模型名（报告用）。
    pub name: &'static str,
    /// 每个 shader stage 最大 sampled texture 数。
    pub max_sampled_textures_per_stage: u32,
    /// 每个 shader stage 最大 sampler 数。
    pub max_samplers_per_stage: u32,
    /// 每个 shader stage 最大 uniform buffer 数。
    pub max_uniform_buffers_per_stage: u32,
    /// 每个 shader stage 最大 storage buffer 数。
    pub max_storage_buffers_per_stage: u32,
    /// 最大 bind group 数。
    pub max_bind_groups: u32,
}

/// WebGPU Core / 现代 wgpu defaults。
pub const LIMIT_MODEL_WEBGPU_CORE: StylizedBudgetLimits = StylizedBudgetLimits {
    name: "webgpu-core",
    max_sampled_textures_per_stage: 16,
    max_samplers_per_stage: 16,
    max_uniform_buffers_per_stage: 12,
    max_storage_buffers_per_stage: 8,
    max_bind_groups: 4,
};

/// GLES 3.1 / wgpu downlevel_defaults。
pub const LIMIT_MODEL_GLES31: StylizedBudgetLimits = StylizedBudgetLimits {
    name: "gles31",
    max_sampled_textures_per_stage: 16,
    max_samplers_per_stage: 16,
    max_uniform_buffers_per_stage: 12,
    max_storage_buffers_per_stage: 4,
    max_bind_groups: 4,
};

/// GLES 3.0 + WebGL2 / wgpu downlevel_webgl2_defaults（storage=0，
/// uniform 11/stage）。
pub const LIMIT_MODEL_WEBGL2: StylizedBudgetLimits = StylizedBudgetLimits {
    name: "webgl2",
    max_sampled_textures_per_stage: 16,
    max_samplers_per_stage: 16,
    max_uniform_buffers_per_stage: 11,
    max_storage_buffers_per_stage: 0,
    max_bind_groups: 4,
};

/// 全部 baseline 模型。
pub const STYLIZED_LIMIT_MODELS: [StylizedBudgetLimits; 3] = [
    LIMIT_MODEL_WEBGPU_CORE,
    LIMIT_MODEL_GLES31,
    LIMIT_MODEL_WEBGL2,
];

/// 单 stage 的资源用量计数。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StylizedStageUsage {
    /// sampled texture 数。
    pub textures: u32,
    /// sampler 数。
    pub samplers: u32,
    /// uniform buffer 数。
    pub uniform_buffers: u32,
    /// storage buffer 数。
    pub storage_buffers: u32,
}

impl std::ops::Add for StylizedStageUsage {
    type Output = Self;

    /// 逐项相加。
    fn add(self, other: Self) -> Self {
        Self {
            textures: self.textures + other.textures,
            samplers: self.samplers + other.samplers,
            uniform_buffers: self.uniform_buffers + other.uniform_buffers,
            storage_buffers: self.storage_buffers + other.storage_buffers,
        }
    }
}

impl StylizedStageUsage {
    /// 逐项是否不超过 `limits`。
    #[must_use]
    pub fn fits_within(&self, limits: &StylizedBudgetLimits) -> bool {
        self.textures <= limits.max_sampled_textures_per_stage
            && self.samplers <= limits.max_samplers_per_stage
            && self.uniform_buffers <= limits.max_uniform_buffers_per_stage
            && self.storage_buffers <= limits.max_storage_buffers_per_stage
    }
}

/// 整条 pipeline 的预算分解（per-stage）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StylizedPipelineBudget {
    /// View bind group（Bevy mesh view layout 反射）。
    pub view: StylizedStageUsage,
    /// Mesh bind group（mesh uniform / skinning）。
    pub mesh: StylizedStageUsage,
    /// Instance 数据（当前 MeshTag 通道 = 0 绑定；storage 表方案落地后
    /// 在此计数）。
    pub instance: StylizedStageUsage,
    /// 材质 bind group（AsBindGroup 反射）。
    pub material: StylizedStageUsage,
    /// Profile 附加（当前无 profile 绑定；保留位）。
    pub profile: StylizedStageUsage,
    /// Safety margin（对抗驱动/后端差异的固定余量）。
    pub safety_margin: StylizedStageUsage,
}

impl StylizedPipelineBudget {
    /// 合计。
    #[must_use]
    pub fn total(&self) -> StylizedStageUsage {
        self.view + self.mesh + self.instance + self.material + self.profile + self.safety_margin
    }
}

/// 建议的 safety margin（interim 值，pending ADR-0002 实测校准）。
pub const INTERIM_SAFETY_MARGIN: StylizedStageUsage = StylizedStageUsage {
    textures: 2,
    samplers: 1,
    uniform_buffers: 1,
    storage_buffers: 0,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_models_match_wgpu_baselines() {
        // WebGL2：storage = 0，uniform 11（docs/rendering/01 锚定）。
        assert_eq!(LIMIT_MODEL_WEBGL2.max_storage_buffers_per_stage, 0);
        assert_eq!(LIMIT_MODEL_WEBGL2.max_uniform_buffers_per_stage, 11);
        assert_eq!(LIMIT_MODEL_GLES31.max_storage_buffers_per_stage, 4);
        assert_eq!(LIMIT_MODEL_WEBGPU_CORE.max_uniform_buffers_per_stage, 12);
    }

    #[test]
    fn usage_add_and_fits() {
        let a = StylizedStageUsage {
            textures: 3,
            samplers: 3,
            uniform_buffers: 1,
            storage_buffers: 0,
        };
        let total = a + a;
        assert_eq!(total.textures, 6);
        assert!(total.fits_within(&LIMIT_MODEL_WEBGL2));
        assert!(
            !StylizedStageUsage {
                storage_buffers: 1,
                ..StylizedStageUsage::default()
            }
            .fits_within(&LIMIT_MODEL_WEBGL2)
        );
    }
}
