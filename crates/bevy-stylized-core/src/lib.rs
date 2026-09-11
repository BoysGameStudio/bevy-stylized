//! `bevy-stylized-core`：bevy-stylized 框架的领域层。
//!
//! 本 crate 只包含数据模型与纯函数逻辑（docs/architecture/03）：
//!
//! - 三轴类型（[`axis`]）：[`StylizedSurfaceClass`] / [`StylizedExecutionTier`] /
//!   [`StylizedValidationMode`]，三者语义正交，不可混用；
//! - 三层描述符（[`descriptor`]）：Material / Instance / View 数据分离
//!   （docs/rendering/02）；
//! - 能力模型（[`capability`]）：requested-vs-actual 能力集合、tier 需求表与
//!   确定性 [`resolve_variant`](capability::resolve_variant)；
//! - 质量政策（[`quality`]）与 Profile 契约（[`profile`]，ADR-0025：
//!   版本化 API/Schema，不承诺 ABI）；
//! - Schema 版本化（[`schema`]）：`bevy_stylized.*` 命名空间的版本载体。
//!
//! 依赖边界：仅 `bevy_math` / `bevy_color` / `bevy_reflect` / `serde`；
//! 禁止出现任何渲染内部类型（`BindGroup`、`RenderPhase`、`TextureView` 等）。

pub mod axis;
pub mod budget;
pub mod capability;
pub mod descriptor;
pub mod profile;
pub mod quality;
pub mod schema;

pub use axis::{StylizedExecutionTier, StylizedSurfaceClass, StylizedValidationMode};
pub use budget::{
    INTERIM_SAFETY_MARGIN, LIMIT_MODEL_GLES31, LIMIT_MODEL_WEBGL2, LIMIT_MODEL_WEBGPU_CORE,
    STYLIZED_LIMIT_MODELS, StylizedBudgetLimits, StylizedPipelineBudget, StylizedStageUsage,
};
pub use capability::{
    PORTABLE_FORBIDDEN_REQUIREMENTS, STYLIZED_FALLBACK_CHAINS, STYLIZED_TIER_REQUIREMENTS,
    StylizedCapabilitySet, StylizedFallbackPolicy, StylizedFallbackReport, StylizedFallbackStep,
    StylizedGpuFeature, StylizedGpuLimits, StylizedResolveError, StylizedResolvedVariant,
    StylizedTierRequirements, fallback_chain, resolve_variant, resolve_with_report,
};
pub use descriptor::{
    StylizedCameraNprState, StylizedInstanceDescriptor, StylizedMaterialDescriptor,
    StylizedMaterialFlags, StylizedTextureRef, StylizedTextureSemantic, StylizedViewDescriptor,
    StylizedViewQuality,
};
pub use profile::{StylizedProfile, StylizedProfileMetadata};
pub use quality::StylizedQualityPolicy;
pub use schema::{
    SCHEMA_ID_MATERIAL_V1, SCHEMA_ID_VIEW_V1, SCHEMA_NAMESPACE_PREFIX, StylizedSchemaId,
    StylizedSchemaVersion, StylizedSchemaVersionParseError,
};

/// 常用类型的一站式导出。
pub mod prelude {
    pub use crate::axis::{StylizedExecutionTier, StylizedSurfaceClass, StylizedValidationMode};
    pub use crate::capability::{
        StylizedCapabilitySet, StylizedFallbackPolicy, StylizedGpuFeature, StylizedGpuLimits,
        StylizedResolveError, StylizedResolvedVariant, resolve_variant, resolve_with_report,
    };
    pub use crate::descriptor::{
        StylizedCameraNprState, StylizedInstanceDescriptor, StylizedMaterialDescriptor,
        StylizedMaterialFlags, StylizedTextureRef, StylizedTextureSemantic, StylizedViewDescriptor,
        StylizedViewQuality,
    };
    pub use crate::profile::{StylizedProfile, StylizedProfileMetadata};
    pub use crate::quality::StylizedQualityPolicy;
    pub use crate::schema::{StylizedSchemaId, StylizedSchemaVersion};
}

#[cfg(test)]
mod tests {
    /// 依赖边界守卫（docs/architecture/03）：core 的 `[dependencies]` 只允许出现
    /// `bevy_math` / `bevy_color` / `bevy_reflect` / `serde` 四个依赖；
    /// `[dev-dependencies]` 额外允许 `serde_json`（仅供测试序列化）。
    /// 新增任何其它依赖（含 build/target 依赖）都必须先修订架构文档。
    #[test]
    fn dependency_boundary_guard() {
        const ALLOWED: [&str; 4] = ["bevy_math", "bevy_color", "bevy_reflect", "serde"];
        const ALLOWED_DEV: [&str; 1] = ["serde_json"];

        let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
            .expect("core Cargo.toml must be readable");

        let mut section = "";
        let mut found: Vec<String> = Vec::new();
        for line in manifest.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                section = match line {
                    "[dependencies]" => "dependencies",
                    "[dev-dependencies]" => "dev-dependencies",
                    "[build-dependencies]" => "build-dependencies",
                    other if other.starts_with("[target.") && other.ends_with("dependencies]") => {
                        "dependencies"
                    }
                    _ => "",
                };
                continue;
            }
            if section.is_empty() || line.is_empty() || line.starts_with('#') {
                continue;
            }
            let key = line
                .split(['=', ' ', '.'])
                .next()
                .unwrap_or_default()
                .trim()
                .to_string();
            if key.is_empty() {
                continue;
            }
            let allowed = if section == "dev-dependencies" {
                ALLOWED.contains(&key.as_str()) || ALLOWED_DEV.contains(&key.as_str())
            } else {
                ALLOWED.contains(&key.as_str())
            };
            assert!(
                allowed,
                "bevy-stylized-core gained a forbidden {section} entry: `{key}` \
                 (allowed: {ALLOWED:?}, dev-only: {ALLOWED_DEV:?})"
            );
            found.push(key);
        }
        for required in ["bevy_math", "bevy_color", "bevy_reflect"] {
            assert!(
                found.iter().any(|k| k == required),
                "expected dependency `{required}` missing from core Cargo.toml"
            );
        }
    }
}
