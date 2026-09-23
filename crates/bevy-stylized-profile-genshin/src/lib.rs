//! Genshin Reference Family profile（V1 唯一锁定 family，ADR-0019 Accepted；
//! 无泛化 GachaProfile）。
//!
//! 本 crate 只依赖 `bevy-stylized-core`（ADR-0025：版本化 API + Schema，
//! 不承诺 ABI；profile 不接触 render internals）。
//!
//! 数据来源纪律：只有 docs/reference/07（行为契约 v1，draft）签名
//! **证据支撑**的条目才给出常量；其余一律标注 **待核实（needs
//! inspection）** 并拒绝填值。

pub mod mapping;
pub use mapping::{
    GenshinBundleInput, GenshinMaterialInput, GenshinMaterialRole, decode_effect_light_direction,
    map_bundle, map_view,
};

use bevy_stylized_core::{
    StylizedExecutionTier, StylizedFallbackPolicy, StylizedProfile, StylizedProfileMetadata,
    StylizedQualityPolicy, StylizedSchemaVersion, StylizedSurfaceClass, StylizedValidationMode,
};

/// Profile 标识（schema namespace 约定）。
pub const GENSHIN_PROFILE_ID: &str = "bevy_stylized.profile.genshin";

/// Current profile data schema version, shared by producers and validators.
pub const GENSHIN_SCHEMA_VERSION: StylizedSchemaVersion = StylizedSchemaVersion::new(1, 0);

/// metadata_map 键（docs/reference/07 §5；仅列出签名证据支撑的条目）。
pub mod keys {
    /// `HoYoverse - GI Body.Use Shadow Ramp`。
    pub const BODY_USE_SHADOW_RAMP: &str = "bevy_stylized.genshin.body.use_shadow_ramp";
    /// `HoYoverse - GI Body.Shadow Ramp Width`。
    pub const BODY_SHADOW_RAMP_WIDTH: &str = "bevy_stylized.genshin.body.shadow_ramp_width";
    /// `HoYoverse - GI Body.Enable Metallics?`。
    pub const BODY_ENABLE_METALLICS: &str = "bevy_stylized.genshin.body.enable_metallics";
    /// `HoYoverse - GI Body.Use Specular Ramp`。
    pub const BODY_USE_SPECULAR_RAMP: &str = "bevy_stylized.genshin.body.use_specular_ramp";
    /// `HoYoverse - Global Material Properties.Day/Night`。
    pub const GLOBAL_DAY_NIGHT: &str = "bevy_stylized.genshin.global.day_night";
}

/// 全局 palette 槽位语义（docs/reference/07 §3.7：`Genshin Lighting` /
/// `Global Material Properties` 节点组的输出接口，签名证据支撑槽位
/// **存在性**；各槽位的具体取色公式为**待核实**）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenshinPaletteSlot {
    /// Ambient Colour。
    AmbientColor,
    /// Sharp Lit Colour。
    SharpLitColor,
    /// Soft Lit Colour。
    SoftLitColor,
    /// Sharp Shadow Color。
    SharpShadowColor,
    /// Soft Shadow Color。
    SoftShadowColor,
}

/// 槽位总数（[`GenshinPaletteSlot`] 的稳定顺序表）。
pub const GENSHIN_PALETTE_SLOTS: [GenshinPaletteSlot; 5] = [
    GenshinPaletteSlot::AmbientColor,
    GenshinPaletteSlot::SharpLitColor,
    GenshinPaletteSlot::SoftLitColor,
    GenshinPaletteSlot::SharpShadowColor,
    GenshinPaletteSlot::SoftShadowColor,
];

/// Ramp atlas 行约定（provisional）：行为契约只证明"ramp 以纹理采样
/// 存在、采样坐标由外部输入"（§3.1）；行序与 material-id 的具体映射
/// **待核实**，此处只承诺"行索引 = profile_index（instance 层
/// MeshTag 低 8 位）"。
pub const GENSHIN_RAMP_ROW_CONVENTION: &str =
    "row index == instance profile_index (MeshTag low 8 bits); row-per-material-id mapping 待核实";

/// Genshin Reference Family profile。
///
/// 支持 Character 与 Hero class（Cel 是公共基线，不在 profile 内重复
/// 声明）；Portable/Standard tier。High 档语义待 profile schema 定稿。
#[derive(Clone, Debug)]
pub struct GenshinProfile {
    metadata: StylizedProfileMetadata,
}

impl Default for GenshinProfile {
    fn default() -> Self {
        Self {
            metadata: StylizedProfileMetadata {
                profile_id: GENSHIN_PROFILE_ID.to_string(),
                display_name: "Genshin Reference Family".to_string(),
                api_version: GENSHIN_SCHEMA_VERSION,
                schema_version: GENSHIN_SCHEMA_VERSION,
                supported_classes: vec![
                    StylizedSurfaceClass::Character,
                    StylizedSurfaceClass::Hero,
                ],
                supported_tiers: vec![
                    StylizedExecutionTier::Portable,
                    StylizedExecutionTier::Standard,
                ],
            },
        }
    }
}

impl StylizedProfile for GenshinProfile {
    fn metadata(&self) -> &StylizedProfileMetadata {
        &self.metadata
    }

    fn quality_policy(&self) -> StylizedQualityPolicy {
        StylizedQualityPolicy {
            default_tier: StylizedExecutionTier::Portable,
            fallback_policy: StylizedFallbackPolicy::ReduceTier,
            validation_mode: StylizedValidationMode::Production,
            // ADR-0018：跨 Class 降级需要 profile 显式声明；本 profile 不声明。
            allow_surface_class_lod: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_metadata_matches_contract() {
        let profile = GenshinProfile::default();
        let metadata = profile.metadata();
        assert_eq!(metadata.profile_id, GENSHIN_PROFILE_ID);
        assert_eq!(metadata.api_version, GENSHIN_SCHEMA_VERSION);
        assert!(
            metadata
                .supported_classes
                .contains(&StylizedSurfaceClass::Hero)
        );
        assert!(
            metadata
                .supported_tiers
                .contains(&StylizedExecutionTier::Portable)
        );
    }

    #[test]
    fn profile_never_allows_class_lod() {
        let profile = GenshinProfile::default();
        assert!(
            !profile.allows_surface_class_lod(
                StylizedSurfaceClass::Hero,
                StylizedSurfaceClass::Character,
            )
        );
    }

    #[test]
    fn metadata_map_keys_are_namespaced() {
        for key in [
            keys::BODY_USE_SHADOW_RAMP,
            keys::BODY_SHADOW_RAMP_WIDTH,
            keys::BODY_ENABLE_METALLICS,
            keys::BODY_USE_SPECULAR_RAMP,
            keys::GLOBAL_DAY_NIGHT,
        ] {
            assert!(key.starts_with("bevy_stylized.genshin."), "{key}");
        }
    }
}
