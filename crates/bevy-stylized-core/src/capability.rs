//! 能力模型与确定性 variant 解析（ADR-0022、docs/architecture/01）。
//!
//! 两阶段生命周期（ADR-0022）：
//!
//! - **Stage A**（RenderDevice 创建前）：facade 的 `StylizedCapabilityConfig`
//!   把需求翻译成 `WgpuSettings` 贡献项；本模块只提供 backend-neutral 的
//!   数据表达；
//! - **Stage B**（RenderDevice 创建后）：只读 `RenderDevice::features()/limits()`，
//!   填入 [`StylizedCapabilitySet`]，交给 [`resolve_variant`] 做确定性解析。
//!
//! 本模块不依赖 wgpu 类型（core 依赖边界）；wgpu 映射在 facade 中完成。
//!
//! Tier 需求表（[`STYLIZED_TIER_REQUIREMENTS`]）当前为 **provisional**：
//! 数值将在 It-7 随 ADR-0010 / 0021 / 0026 关闭时按平台证据定稿。
//! 唯一已冻结的不变量：Portable 档不得要求
//! [`PORTABLE_FORBIDDEN_REQUIREMENTS`] 中的任何特性。

use std::collections::BTreeSet;

use bevy_reflect::Reflect;

use crate::axis::{StylizedExecutionTier, StylizedSurfaceClass};

/// 框架关心的可选 GPU 能力（backend-neutral 表达）。
///
/// 每个变体在 facade 中映射到具体 wgpu feature/limits；此处保持纯数据。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum StylizedGpuFeature {
    /// Bindless / binding-array 资源模型。
    Bindless,
    /// FP16（`shader-f16`）着色器能力。
    Float16,
    /// Subgroup（wave）操作。
    Subgroup,
    /// Compute shader 路径。
    Compute,
    /// HDR 输出。
    HdrOutput,
    /// vertex stage 的 storage buffer 读取。
    StorageBufferInVertexStage,
}

/// 能力相关的 GPU limits 子集（backend-neutral）。
///
/// `Default` 对应 Portable 基线：不假设 storage buffer 可用，bind group /
/// uniform buffer 数为最低通用值。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StylizedGpuLimits {
    /// 最大 bind group 数。
    #[cfg_attr(feature = "serde", serde(default = "default_max_bind_groups"))]
    pub max_bind_groups: u32,
    /// 每个 shader stage 最大 storage buffer 数（0 = 不可用）。
    #[cfg_attr(feature = "serde", serde(default))]
    pub max_storage_buffers_per_shader_stage: u32,
    /// 每个 shader stage 最大 uniform buffer 数。
    #[cfg_attr(feature = "serde", serde(default = "default_max_uniform_buffers"))]
    pub max_uniform_buffers_per_shader_stage: u32,
}

impl Default for StylizedGpuLimits {
    fn default() -> Self {
        Self {
            max_bind_groups: default_max_bind_groups(),
            max_storage_buffers_per_shader_stage: 0,
            max_uniform_buffers_per_shader_stage: default_max_uniform_buffers(),
        }
    }
}

fn default_max_bind_groups() -> u32 {
    4
}

fn default_max_uniform_buffers() -> u32 {
    8
}

/// 设备实际能力（Stage B 只读快照）。
#[derive(Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StylizedCapabilitySet {
    /// 设备已启用的可选特性集合。
    #[cfg_attr(feature = "serde", serde(default))]
    pub features: BTreeSet<StylizedGpuFeature>,
    /// 设备实际 limits。
    #[cfg_attr(feature = "serde", serde(default))]
    pub limits: StylizedGpuLimits,
}

impl StylizedCapabilitySet {
    /// 是否满足 `requirements`。
    #[must_use]
    pub fn satisfies(&self, requirements: &StylizedTierRequirements) -> bool {
        self.missing_features(requirements).is_empty() && self.limits_satisfy(requirements)
    }

    /// 缺失的必需特性（用于诊断）。
    #[must_use]
    pub fn missing_features(
        &self,
        requirements: &StylizedTierRequirements,
    ) -> Vec<StylizedGpuFeature> {
        requirements
            .required_features
            .iter()
            .copied()
            .filter(|feature| !self.features.contains(feature))
            .collect()
    }

    /// limits 是否达到要求。
    #[must_use]
    pub fn limits_satisfy(&self, requirements: &StylizedTierRequirements) -> bool {
        let required = requirements.required_limits;
        self.limits.max_bind_groups >= required.max_bind_groups
            && self.limits.max_storage_buffers_per_shader_stage
                >= required.max_storage_buffers_per_shader_stage
            && self.limits.max_uniform_buffers_per_shader_stage
                >= required.max_uniform_buffers_per_shader_stage
    }
}

/// 单个 Execution Tier 的设备需求（数据表条目）。
#[derive(Clone, Copy, Debug)]
pub struct StylizedTierRequirements {
    /// 该条目对应的档位。
    pub tier: StylizedExecutionTier,
    /// 必需的可选特性集合。
    pub required_features: &'static [StylizedGpuFeature],
    /// 必需的最小 limits。
    pub required_limits: StylizedGpuLimits,
}

/// Portable 档**不得**要求的特性（docs/architecture/01：
/// "最低能力路径；不得依赖可选 GPU Feature"）。
pub const PORTABLE_FORBIDDEN_REQUIREMENTS: [StylizedGpuFeature; 5] = [
    StylizedGpuFeature::Bindless,
    StylizedGpuFeature::Float16,
    StylizedGpuFeature::Subgroup,
    StylizedGpuFeature::Compute,
    StylizedGpuFeature::HdrOutput,
];

const PORTABLE_LIMITS: StylizedGpuLimits = StylizedGpuLimits {
    max_bind_groups: 4,
    max_storage_buffers_per_shader_stage: 0,
    max_uniform_buffers_per_shader_stage: 8,
};

const STANDARD_LIMITS: StylizedGpuLimits = StylizedGpuLimits {
    max_bind_groups: 4,
    max_storage_buffers_per_shader_stage: 4,
    max_uniform_buffers_per_shader_stage: 12,
};

// provisional：不得超过 wgpu defaults（bind groups 4 / UBO 12），
// 否则 High 在现代基线上永不可满足。
const HIGH_LIMITS: StylizedGpuLimits = StylizedGpuLimits {
    max_bind_groups: 4,
    max_storage_buffers_per_shader_stage: 8,
    max_uniform_buffers_per_shader_stage: 12,
};

/// Tier 需求表（**provisional**，pending ADR-0010 / 0021 / 0026）。
///
/// 已冻结不变量：Portable 条目不包含
/// [`PORTABLE_FORBIDDEN_REQUIREMENTS`] 中的任何特性（有测试钉死）。
pub const STYLIZED_TIER_REQUIREMENTS: [StylizedTierRequirements; 3] = [
    StylizedTierRequirements {
        tier: StylizedExecutionTier::Portable,
        required_features: &[],
        required_limits: PORTABLE_LIMITS,
    },
    StylizedTierRequirements {
        tier: StylizedExecutionTier::Standard,
        required_features: &[StylizedGpuFeature::StorageBufferInVertexStage],
        required_limits: STANDARD_LIMITS,
    },
    StylizedTierRequirements {
        tier: StylizedExecutionTier::High,
        required_features: &[
            StylizedGpuFeature::StorageBufferInVertexStage,
            StylizedGpuFeature::Compute,
        ],
        required_limits: HIGH_LIMITS,
    },
];

/// 查询某档位的需求条目。
#[must_use]
pub fn tier_requirements(tier: StylizedExecutionTier) -> &'static StylizedTierRequirements {
    STYLIZED_TIER_REQUIREMENTS
        .iter()
        .find(|entry| entry.tier == tier)
        .expect("tier requirements table covers every StylizedExecutionTier")
}

/// Capability 不满足时的 fallback 政策。
///
/// **不存在** `ReduceClass` 变体：tier reduction 永不降低 Surface Class
/// （docs/architecture/01）。跨 Class 降级只能由 Profile 显式声明的
/// 语义 LOD 表达，不属于本政策的职责。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum StylizedFallbackPolicy {
    /// 严格：任何需求不满足即失败（fail-closed）。
    Fail,
    /// 允许沿 `requested_tier` 向下降档（Class 不变）。
    #[default]
    ReduceTier,
}

/// 解析结果：实际选定的运行时 variant。
#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StylizedResolvedVariant {
    /// 表面风格分类。**保证等于请求值**，永不静默降级。
    pub class: StylizedSurfaceClass,
    /// 请求的档位。
    pub requested_tier: StylizedExecutionTier,
    /// 实际选定的档位（可能低于 `requested_tier`）。
    pub tier: StylizedExecutionTier,
    /// 是否发生了降档（`tier != requested_tier`）。
    pub tier_reduced: bool,
    /// 请求档位缺失的特性（降档时的诊断信息）。
    pub missing_features: Vec<StylizedGpuFeature>,
}

/// 解析失败：即使最低档也不满足，或政策为
/// [`StylizedFallbackPolicy::Fail`] 且请求档不满足。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StylizedResolveError {
    /// 请求的表面分类（保留在错误中以强调 class 未被替换）。
    pub class: StylizedSurfaceClass,
    /// 请求的档位。
    pub requested_tier: StylizedExecutionTier,
    /// 最终无法放行的档位及其缺失特性。
    pub blocking_tier: StylizedExecutionTier,
    /// 阻塞档位的缺失特性。
    pub missing_features: Vec<StylizedGpuFeature>,
    /// 阻塞档位的 limits 是否满足（false 表示 limits 不足）。
    pub limits_satisfied: bool,
}

impl std::fmt::Display for StylizedResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "cannot resolve {:?} @ {:?}: {:?} requirements unmet \
             (missing features: {:?}, limits satisfied: {})",
            self.class,
            self.requested_tier,
            self.blocking_tier,
            self.missing_features,
            self.limits_satisfied,
        )
    }
}

impl std::error::Error for StylizedResolveError {}

// ---------------------------------------------------------------------------
// It-7：per-class fallback 链 + FallbackReport
// ---------------------------------------------------------------------------

/// 单条特性削减步骤（per-class fallback 链的组成单元）。
///
/// 语义锚点（render 侧的落地形态）：
/// - `DropMatcap` / `DropSpecRamp` / `DropPackedNormal`：Hero 的三条最贵
///   纹理路径（`apply_hero_portable_fallback`）；
/// - `FaceSdfHardStep`：face SDF 从 smoothstep 降为硬 step；
/// - `HairSpecularToRim`：hair 高光关闭，rim 补偿（`apply_character_portable_fallback`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum StylizedFallbackStep {
    /// 关闭 metal matcap。
    DropMatcap,
    /// 关闭 spec ramp。
    DropSpecRamp,
    /// 关闭 packed normal。
    DropPackedNormal,
    /// Face SDF 降为硬 step 阈值。
    FaceSdfHardStep,
    /// Hair 高光关闭，rim 补偿。
    HairSpecularToRim,
}

/// Per-class fallback 链（docs/rendering/01 的 class fallback 定义，
/// 数据化；provisional pending ADR-0018/It-7 关闭）。
///
/// 语义：当 tier 被降级（`tier_reduced`）时，按序应用该 class 链上的全部
/// 步骤；**Cel 是基线，链为空，永不降级**。步骤只削减特性，永不跨 class。
pub const STYLIZED_FALLBACK_CHAINS: [(StylizedSurfaceClass, &[StylizedFallbackStep]); 3] = [
    (StylizedSurfaceClass::Cel, &[]),
    (
        StylizedSurfaceClass::Character,
        &[
            StylizedFallbackStep::FaceSdfHardStep,
            StylizedFallbackStep::HairSpecularToRim,
        ],
    ),
    (
        StylizedSurfaceClass::Hero,
        &[
            StylizedFallbackStep::DropMatcap,
            StylizedFallbackStep::DropSpecRamp,
            StylizedFallbackStep::DropPackedNormal,
            StylizedFallbackStep::FaceSdfHardStep,
            StylizedFallbackStep::HairSpecularToRim,
        ],
    ),
];

/// 查询某 class 的 fallback 链。
#[must_use]
pub fn fallback_chain(class: StylizedSurfaceClass) -> &'static [StylizedFallbackStep] {
    STYLIZED_FALLBACK_CHAINS
        .iter()
        .find(|(c, _)| *c == class)
        .map(|(_, chain)| *chain)
        .expect("fallback chain table covers every StylizedSurfaceClass")
}

/// 人可读的 fallback 报告（解析结果的诊断载体）。
#[derive(Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StylizedFallbackReport {
    /// 表面分类（与请求一致）。
    pub class: StylizedSurfaceClass,
    /// 请求档位。
    pub requested_tier: StylizedExecutionTier,
    /// 实际档位。
    pub resolved_tier: StylizedExecutionTier,
    /// 是否降档。
    pub tier_reduced: bool,
    /// 请求档位缺失的特性。
    pub missing_features: Vec<StylizedGpuFeature>,
    /// 应用的 class fallback 步骤（降档时 = 全链）。
    pub applied_steps: Vec<StylizedFallbackStep>,
    /// 人可读说明（每步一条）。
    pub notes: Vec<String>,
}

/// 带报告的解析（[`resolve_variant`] 的诊断形态；同输入同输出，
/// 报告内容确定性生成）。
pub fn resolve_with_report(
    class: StylizedSurfaceClass,
    requested_tier: StylizedExecutionTier,
    actual: &StylizedCapabilitySet,
    policy: StylizedFallbackPolicy,
) -> Result<(StylizedResolvedVariant, StylizedFallbackReport), StylizedResolveError> {
    let resolved = resolve_variant(class, requested_tier, actual, policy)?;
    let applied_steps: Vec<StylizedFallbackStep> = if resolved.tier_reduced {
        fallback_chain(class).to_vec()
    } else {
        Vec::new()
    };
    let mut notes = Vec::new();
    if resolved.tier_reduced {
        notes.push(format!(
            "tier reduced {requested_tier:?} -> {:?}; class {class:?} preserved",
            resolved.tier
        ));
        if !resolved.missing_features.is_empty() {
            notes.push(format!(
                "missing features at requested tier: {:?}",
                resolved.missing_features
            ));
        }
        for step in &applied_steps {
            notes.push(format!("fallback step applied: {step:?}"));
        }
    } else {
        notes.push("requested tier satisfied; no fallback".to_string());
    }
    let report = StylizedFallbackReport {
        class,
        requested_tier,
        resolved_tier: resolved.tier,
        tier_reduced: resolved.tier_reduced,
        missing_features: resolved.missing_features.clone(),
        applied_steps,
        notes,
    };
    Ok((resolved, report))
}

/// 确定性 variant 解析（Stage B 的纯函数核心）。
///
/// 语义：
///
/// - 只沿 `requested_tier` 向下搜索可行档位；**永不**返回与 `class`
///   不同的结果（类型层面即不可能）；
/// - [`StylizedFallbackPolicy::Fail`]：请求档不满足即 `Err`；
/// - [`StylizedFallbackPolicy::ReduceTier`]：返回满足需求的最高档
///   （不超过请求档），全部不满足则 `Err`；
/// - 纯函数：同输入必得同输出，无内部状态、无环境读取。
pub fn resolve_variant(
    class: StylizedSurfaceClass,
    requested_tier: StylizedExecutionTier,
    actual: &StylizedCapabilitySet,
    policy: StylizedFallbackPolicy,
) -> Result<StylizedResolvedVariant, StylizedResolveError> {
    let requested_requirements = tier_requirements(requested_tier);
    let requested_missing = actual.missing_features(requested_requirements);
    let requested_limits_ok = actual.limits_satisfy(requested_requirements);

    if requested_missing.is_empty() && requested_limits_ok {
        return Ok(StylizedResolvedVariant {
            class,
            requested_tier,
            tier: requested_tier,
            tier_reduced: false,
            missing_features: Vec::new(),
        });
    }

    if policy == StylizedFallbackPolicy::Fail {
        return Err(StylizedResolveError {
            class,
            requested_tier,
            blocking_tier: requested_tier,
            missing_features: requested_missing,
            limits_satisfied: requested_limits_ok,
        });
    }

    // ReduceTier：沿预算序向下搜索；Portable 已是最后一站。
    let mut candidate = requested_tier;
    while let Some(lower) = candidate.reduced() {
        let requirements = tier_requirements(lower);
        if actual.satisfies(requirements) {
            return Ok(StylizedResolvedVariant {
                class,
                requested_tier,
                tier: lower,
                tier_reduced: true,
                missing_features: requested_missing,
            });
        }
        candidate = lower;
    }

    Err(StylizedResolveError {
        class,
        requested_tier,
        blocking_tier: StylizedExecutionTier::Portable,
        missing_features: actual
            .missing_features(tier_requirements(StylizedExecutionTier::Portable)),
        limits_satisfied: actual.limits_satisfy(tier_requirements(StylizedExecutionTier::Portable)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- It-7：确定性 fallback 矩阵（快照式显式期望表） ----

    fn caps_model(
        features: &[StylizedGpuFeature],
        storage_buffers: u32,
        uniform_buffers: u32,
    ) -> StylizedCapabilitySet {
        StylizedCapabilitySet {
            features: features.iter().copied().collect(),
            limits: StylizedGpuLimits {
                max_bind_groups: 4,
                max_storage_buffers_per_shader_stage: storage_buffers,
                max_uniform_buffers_per_shader_stage: uniform_buffers,
            },
        }
    }

    fn webgl2_caps() -> StylizedCapabilitySet {
        caps_model(&[], 0, 11)
    }

    fn gles31_caps() -> StylizedCapabilitySet {
        caps_model(
            &[
                StylizedGpuFeature::StorageBufferInVertexStage,
                StylizedGpuFeature::Compute,
            ],
            4,
            12,
        )
    }

    fn webgpu_core_caps() -> StylizedCapabilitySet {
        caps_model(
            &[
                StylizedGpuFeature::StorageBufferInVertexStage,
                StylizedGpuFeature::Compute,
            ],
            8,
            12,
        )
    }

    fn high_end_caps() -> StylizedCapabilitySet {
        caps_model(
            &[
                StylizedGpuFeature::StorageBufferInVertexStage,
                StylizedGpuFeature::Compute,
                StylizedGpuFeature::Bindless,
                StylizedGpuFeature::Float16,
                StylizedGpuFeature::Subgroup,
            ],
            64,
            12,
        )
    }

    /// 显式期望表：requested tier × caps model →（解析档位, 是否降档）。
    /// class 永不改变（另有穷举测试钉死），此处锁 tier 分辨率。
    const EXPECTED_TIER_TABLE: &[(&str, StylizedExecutionTier, StylizedExecutionTier, bool)] = &[
        (
            "webgl2",
            StylizedExecutionTier::High,
            StylizedExecutionTier::Portable,
            true,
        ),
        (
            "webgl2",
            StylizedExecutionTier::Standard,
            StylizedExecutionTier::Portable,
            true,
        ),
        (
            "webgl2",
            StylizedExecutionTier::Portable,
            StylizedExecutionTier::Portable,
            false,
        ),
        // gles31 的 storage/stage = 4 < High 要求的 8 → 降到 Standard。
        (
            "gles31",
            StylizedExecutionTier::High,
            StylizedExecutionTier::Standard,
            true,
        ),
        (
            "gles31",
            StylizedExecutionTier::Standard,
            StylizedExecutionTier::Standard,
            false,
        ),
        (
            "gles31",
            StylizedExecutionTier::Portable,
            StylizedExecutionTier::Portable,
            false,
        ),
        (
            "webgpu-core",
            StylizedExecutionTier::High,
            StylizedExecutionTier::High,
            false,
        ),
        (
            "high-end",
            StylizedExecutionTier::High,
            StylizedExecutionTier::High,
            false,
        ),
    ];

    #[test]
    fn fallback_matrix_matches_expected_table() {
        for (model_name, requested, expected_tier, expected_reduced) in EXPECTED_TIER_TABLE {
            let caps = match *model_name {
                "webgl2" => webgl2_caps(),
                "gles31" => gles31_caps(),
                "webgpu-core" => webgpu_core_caps(),
                "high-end" => high_end_caps(),
                other => panic!("unknown model {other}"),
            };
            for class in [
                StylizedSurfaceClass::Cel,
                StylizedSurfaceClass::Character,
                StylizedSurfaceClass::Hero,
            ] {
                let (resolved, report) = resolve_with_report(
                    class,
                    *requested,
                    &caps,
                    StylizedFallbackPolicy::ReduceTier,
                )
                .unwrap_or_else(|e| panic!("{model_name} {class:?} @{requested:?}: {e}"));
                assert_eq!(
                    resolved.tier, *expected_tier,
                    "{model_name} {class:?} @{requested:?}"
                );
                assert_eq!(resolved.tier_reduced, *expected_reduced);
                assert_eq!(resolved.class, class);
                assert_eq!(report.resolved_tier, *expected_tier);
                assert_eq!(
                    report.applied_steps.len(),
                    if *expected_reduced {
                        fallback_chain(class).len()
                    } else {
                        0
                    },
                    "{model_name} {class:?} @{requested:?}: fallback steps"
                );
                assert!(!report.notes.is_empty());
            }
        }
    }

    #[test]
    fn resolve_with_report_is_deterministic() {
        let caps = webgl2_caps();
        let first = resolve_with_report(
            StylizedSurfaceClass::Hero,
            StylizedExecutionTier::High,
            &caps,
            StylizedFallbackPolicy::ReduceTier,
        );
        let second = resolve_with_report(
            StylizedSurfaceClass::Hero,
            StylizedExecutionTier::High,
            &caps,
            StylizedFallbackPolicy::ReduceTier,
        );
        assert_eq!(first, second);
    }

    #[test]
    fn cel_chain_is_empty_baseline() {
        assert!(fallback_chain(StylizedSurfaceClass::Cel).is_empty());
        assert!(!fallback_chain(StylizedSurfaceClass::Character).is_empty());
        assert!(!fallback_chain(StylizedSurfaceClass::Hero).is_empty());
    }

    fn caps(features: &[StylizedGpuFeature], limits: StylizedGpuLimits) -> StylizedCapabilitySet {
        StylizedCapabilitySet {
            features: features.iter().copied().collect(),
            limits,
        }
    }

    fn high_caps() -> StylizedCapabilitySet {
        caps(
            &[
                StylizedGpuFeature::StorageBufferInVertexStage,
                StylizedGpuFeature::Compute,
                StylizedGpuFeature::Bindless,
            ],
            HIGH_LIMITS,
        )
    }

    #[test]
    fn portable_tier_requires_no_forbidden_features() {
        let portable = tier_requirements(StylizedExecutionTier::Portable);
        for forbidden in PORTABLE_FORBIDDEN_REQUIREMENTS {
            assert!(
                !portable.required_features.contains(&forbidden),
                "Portable tier must not require {forbidden:?}"
            );
        }
        assert_eq!(
            portable
                .required_limits
                .max_storage_buffers_per_shader_stage,
            0
        );
    }

    #[test]
    fn resolve_is_deterministic() {
        let actual = high_caps();
        let first = resolve_variant(
            StylizedSurfaceClass::Hero,
            StylizedExecutionTier::High,
            &actual,
            StylizedFallbackPolicy::ReduceTier,
        );
        let second = resolve_variant(
            StylizedSurfaceClass::Hero,
            StylizedExecutionTier::High,
            &actual,
            StylizedFallbackPolicy::ReduceTier,
        );
        assert_eq!(first, second);
    }

    #[test]
    fn resolve_never_downgrades_class() {
        // 对所有 class × tier × 能力组合：成功结果的 class 必等于请求 class。
        let classes = [
            StylizedSurfaceClass::Cel,
            StylizedSurfaceClass::Character,
            StylizedSurfaceClass::Hero,
        ];
        let cap_sets = [
            StylizedCapabilitySet::default(),
            caps(&[], PORTABLE_LIMITS),
            caps(
                &[StylizedGpuFeature::StorageBufferInVertexStage],
                STANDARD_LIMITS,
            ),
            high_caps(),
        ];
        for class in classes {
            for tier in StylizedExecutionTier::DESCENDING {
                for actual in &cap_sets {
                    for policy in [
                        StylizedFallbackPolicy::Fail,
                        StylizedFallbackPolicy::ReduceTier,
                    ] {
                        if let Ok(resolved) = resolve_variant(class, tier, actual, policy) {
                            assert_eq!(resolved.class, class);
                            assert!(resolved.tier <= tier);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn resolve_reduces_tier_when_requested_tier_unmet() {
        let actual = caps(&[], PORTABLE_LIMITS);
        let resolved = resolve_variant(
            StylizedSurfaceClass::Hero,
            StylizedExecutionTier::High,
            &actual,
            StylizedFallbackPolicy::ReduceTier,
        )
        .expect("portable baseline must satisfy Portable tier");
        assert_eq!(resolved.class, StylizedSurfaceClass::Hero);
        assert_eq!(resolved.tier, StylizedExecutionTier::Portable);
        assert!(resolved.tier_reduced);
        assert!(
            resolved
                .missing_features
                .contains(&StylizedGpuFeature::Compute)
        );
    }

    #[test]
    fn resolve_fail_policy_is_fail_closed() {
        let actual = caps(&[], PORTABLE_LIMITS);
        let error = resolve_variant(
            StylizedSurfaceClass::Cel,
            StylizedExecutionTier::Standard,
            &actual,
            StylizedFallbackPolicy::Fail,
        )
        .expect_err("Fail policy must reject unmet tiers");
        assert_eq!(error.blocking_tier, StylizedExecutionTier::Standard);
    }

    #[test]
    fn resolve_errors_when_even_portable_unmet() {
        let mut actual = caps(&[], PORTABLE_LIMITS);
        actual.limits.max_bind_groups = 2; // 低于 Portable 基线
        let error = resolve_variant(
            StylizedSurfaceClass::Cel,
            StylizedExecutionTier::High,
            &actual,
            StylizedFallbackPolicy::ReduceTier,
        )
        .expect_err("sub-portable limits must fail closed");
        assert_eq!(error.blocking_tier, StylizedExecutionTier::Portable);
        assert!(!error.limits_satisfied);
    }
}
