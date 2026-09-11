//! 第三方自定义 Profile 最小示例（ADR-0025：版本化 API + Schema 契约，
//! 不承诺 ABI）。
//!
//! 第三方 profile = 一个普通 Rust crate，只依赖 `bevy-stylized-core`
//! （经 facade re-export），实现 [`StylizedProfile`] 契约——**不接触
//! render internals**。
//!
//! 运行：`cargo run -p bevy-stylized --example custom_profile`

use bevy_stylized::prelude::*;

/// 一个假想的第三方 NPR profile（如"水彩"风格）。
struct WatercolorProfile {
    metadata: StylizedProfileMetadata,
}

impl Default for WatercolorProfile {
    fn default() -> Self {
        Self {
            metadata: StylizedProfileMetadata {
                profile_id: "com.example.watercolor".into(),
                display_name: "Watercolor".into(),
                // profile 声明其实现针对的 framework API / 数据 schema 版本。
                api_version: StylizedSchemaVersion::new(1, 0),
                schema_version: StylizedSchemaVersion::new(1, 0),
                supported_classes: vec![StylizedSurfaceClass::Cel],
                supported_tiers: vec![StylizedExecutionTier::Portable],
            },
        }
    }
}

impl StylizedProfile for WatercolorProfile {
    fn metadata(&self) -> &StylizedProfileMetadata {
        &self.metadata
    }

    fn quality_policy(&self) -> StylizedQualityPolicy {
        StylizedQualityPolicy {
            default_tier: StylizedExecutionTier::Portable,
            fallback_policy: StylizedFallbackPolicy::ReduceTier,
            validation_mode: StylizedValidationMode::Production,
            allow_surface_class_lod: false,
        }
    }
}

fn main() {
    let profile = WatercolorProfile::default();
    let metadata = profile.metadata();
    println!(
        "profile: {} ({}) api={} schema={}",
        metadata.display_name, metadata.profile_id, metadata.api_version, metadata.schema_version
    );

    // 用框架的确定性 resolver 解析该 profile 的默认质量政策。
    let policy = profile.quality_policy();
    let (variant, report) = resolve_with_report(
        StylizedSurfaceClass::Cel,
        policy.default_tier,
        &StylizedCapabilitySet::default(),
        policy.fallback_policy,
    )
    .expect("portable baseline resolves");
    println!("resolved: {:?} @ {:?}", variant.class, variant.tier);
    for note in &report.notes {
        println!("  note: {note}");
    }
}
