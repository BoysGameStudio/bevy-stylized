//! 第三方 profile 注册测试：自定义 profile 只经 facade re-export 的 core
//! 契约工作，不接触 render internals（ADR-0025 契约证明）。

use bevy_stylized::prelude::*;

/// 第三方 profile（与 examples/custom_profile.rs 同形）。
struct ThirdPartyProfile {
    metadata: StylizedProfileMetadata,
}

impl StylizedProfile for ThirdPartyProfile {
    fn metadata(&self) -> &StylizedProfileMetadata {
        &self.metadata
    }
}

#[test]
fn third_party_profile_works_through_core_contract_only() {
    let profile = ThirdPartyProfile {
        metadata: StylizedProfileMetadata {
            profile_id: "com.example.thirdparty".into(),
            display_name: "ThirdParty".into(),
            api_version: StylizedSchemaVersion::new(1, 0),
            schema_version: StylizedSchemaVersion::new(1, 0),
            supported_classes: vec![StylizedSurfaceClass::Cel],
            supported_tiers: vec![StylizedExecutionTier::Portable],
        },
    };
    assert_eq!(profile.metadata().profile_id, "com.example.thirdparty");
    // 与框架 resolver 组合：Portable 基线可解析。
    let (variant, _report) = resolve_with_report(
        StylizedSurfaceClass::Cel,
        StylizedExecutionTier::Portable,
        &StylizedCapabilitySet::default(),
        StylizedFallbackPolicy::ReduceTier,
    )
    .expect("portable baseline resolves");
    assert_eq!(variant.tier, StylizedExecutionTier::Portable);
}
