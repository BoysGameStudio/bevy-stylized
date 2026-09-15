//! 三轴运行时模型（docs/architecture/01、ADR-0018）。
//!
//! 三个维度语义正交，禁止混用：
//!
//! - [`StylizedSurfaceClass`]：内容/风格语义（"这个材质必须表达什么"）；
//! - [`StylizedExecutionTier`]：设备/性能预算（"当前设备允许花多少成本"）；
//! - [`StylizedValidationMode`]：验证路径（"当前是在验证真值还是跑生产实现"）。
//!
//! 关键不变量：**降低 Execution Tier 绝不改变 Surface Class**。
//! Hero 在低端设备上解析为 `Hero @ Portable`，而不是退化成 Character。
//! 只有 Profile 显式声明了语义允许的 Surface-Class LOD 时才允许跨 Class 降级
//! （见 [`crate::profile::StylizedProfile::allows_surface_class_lod`]）。

use bevy_reflect::Reflect;

/// 表面风格分类：材质/角色必须表达的视觉语义。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum StylizedSurfaceClass {
    /// 基础卡通/赛璐璐表面。
    #[default]
    Cel,
    /// 角色表面（Face / Hair / Rim / Emission 等角色语义）。
    Character,
    /// 主角级表面（完整上游参考语义，如 Gacha ILM / Face Correction）。
    Hero,
}

/// 执行档位：当前设备允许消耗的 GPU/内存预算。
///
/// 顺序即预算顺序：`Portable < Standard < High`，fallback 只允许沿该序向下。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum StylizedExecutionTier {
    /// 最低能力路径；不得依赖任何可选 GPU Feature
    /// （见 [`crate::capability::PORTABLE_FORBIDDEN_REQUIREMENTS`]）。
    #[default]
    Portable,
    /// 主流 iOS/Android Vulkan / 普通 PC。
    Standard,
    /// 高端 PC、未来主机级 GPU。
    High,
}

impl StylizedExecutionTier {
    /// 全部档位，按预算从高到低排列（fallback 搜索顺序）。
    pub const DESCENDING: [StylizedExecutionTier; 3] = [
        StylizedExecutionTier::High,
        StylizedExecutionTier::Standard,
        StylizedExecutionTier::Portable,
    ];

    /// 返回预算严格低于 `self` 的下一档；`Portable` 已是最低档，返回 `None`。
    #[must_use]
    pub fn reduced(self) -> Option<Self> {
        match self {
            StylizedExecutionTier::Portable => None,
            StylizedExecutionTier::Standard => Some(StylizedExecutionTier::Portable),
            StylizedExecutionTier::High => Some(StylizedExecutionTier::Standard),
        }
    }
}

/// 验证模式：当前渲染路径是在执行生产实现还是在验证参考真值。
///
/// Reference 不属于 Execution Tier；它是独立的验证后端/模式。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum StylizedValidationMode {
    /// 生产渲染路径。
    #[default]
    Production,
    /// 对照上游（Blender / Gacha Setup）真值的验证路径。
    UpstreamReference,
    /// 对照引擎内 Reference 实现的验证路径。
    EngineReference,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_budget_ordering_is_total() {
        assert!(StylizedExecutionTier::Portable < StylizedExecutionTier::Standard);
        assert!(StylizedExecutionTier::Standard < StylizedExecutionTier::High);
    }

    #[test]
    fn tier_reduction_never_skips_portable() {
        assert_eq!(
            StylizedExecutionTier::High.reduced(),
            Some(StylizedExecutionTier::Standard)
        );
        assert_eq!(
            StylizedExecutionTier::Standard.reduced(),
            Some(StylizedExecutionTier::Portable)
        );
        assert_eq!(StylizedExecutionTier::Portable.reduced(), None);
    }

    #[test]
    fn three_axes_are_distinct_types() {
        // 编译期保证：三轴是三个不同类型，不存在互相转换的 From/Into。
        // 运行期只验证默认值语义不混淆。
        assert_ne!(
            std::any::TypeId::of::<StylizedSurfaceClass>(),
            std::any::TypeId::of::<StylizedExecutionTier>()
        );
        assert_ne!(
            std::any::TypeId::of::<StylizedExecutionTier>(),
            std::any::TypeId::of::<StylizedValidationMode>()
        );
    }
}
