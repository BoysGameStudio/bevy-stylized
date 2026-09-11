//! Profile 契约（ADR-0025）。
//!
//! 契约承诺的是**版本化 Rust API + 版本化 Schema**，不是 Rust ABI：
//!
//! - Profile 以 crate / 数据包形式分发，SemVer 与迁移测试约束源码/schema
//!   兼容性；
//! - 不承诺动态二进制插件；二进制 ABI 需要未来的专门 ADR。
//!
//! Profile crate（如 `bevy-stylized-profile-genshin`）只依赖 core，
//! 尽量不接触 render internals。

use bevy_reflect::Reflect;

use crate::{
    axis::{StylizedExecutionTier, StylizedSurfaceClass},
    quality::StylizedQualityPolicy,
    schema::StylizedSchemaVersion,
};

/// Profile 的静态元数据（版本化契约的自描述部分）。
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StylizedProfileMetadata {
    /// Profile 标识（如 `bevy_stylized.profile.genshin`）。
    pub profile_id: String,
    /// 展示名。
    pub display_name: String,
    /// Profile 实现所针对的 framework API 版本。
    pub api_version: StylizedSchemaVersion,
    /// Profile 数据文件的 schema 版本。
    pub schema_version: StylizedSchemaVersion,
    /// Profile 支持的 Surface Class 集合。
    pub supported_classes: Vec<StylizedSurfaceClass>,
    /// Profile 支持的 Execution Tier 集合。
    pub supported_tiers: Vec<StylizedExecutionTier>,
}

/// Profile 契约 trait。
///
/// 实现方承诺：[`metadata`](StylizedProfile::metadata) 返回的版本号真实反映
/// 其数据 schema；[`allows_surface_class_lod`](StylizedProfile::allows_surface_class_lod)
/// 是跨 Class 降级的**唯一**合法入口（默认禁止）。
pub trait StylizedProfile {
    /// Profile 静态元数据。
    fn metadata(&self) -> &StylizedProfileMetadata;

    /// Profile 的默认质量政策。
    fn quality_policy(&self) -> StylizedQualityPolicy {
        StylizedQualityPolicy::default()
    }

    /// 是否允许 `from` → `to` 的 Surface-Class 语义 LOD。
    ///
    /// 默认 `false`（docs/architecture/01：跨 Class 降级不是默认模型，
    /// 只有 Profile 明确定义了语义允许的 LOD 时才返回 `true`）。
    fn allows_surface_class_lod(
        &self,
        _from: StylizedSurfaceClass,
        _to: StylizedSurfaceClass,
    ) -> bool {
        false
    }
}
