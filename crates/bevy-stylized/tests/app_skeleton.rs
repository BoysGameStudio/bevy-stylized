//! StylizedPlugin 骨架测试：headless、无 RenderPlugin 下可装配并运行。

use bevy::prelude::*;
use bevy_stylized::prelude::*;

fn update_n(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

#[test]
fn stylized_plugin_runs_headless_updates() {
    // Given / When
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        // StylizedPlugin 默认开启 `render` feature；MaterialPlugin 与
        // embedded shader 注册需要 AssetServer（Bevy 插件惯例）。
        .add_plugins(bevy::asset::AssetPlugin {
            meta_check: bevy::asset::AssetMetaCheck::Never,
            ..Default::default()
        })
        .add_plugins(StylizedPlugin::default());
    app.finish();
    app.cleanup();
    update_n(&mut app, 3);

    // Then：无 RenderPlugin 时 Stage B 落 CPU 兜底（Portable 基线可解析）。
    let caps = app.world().resource::<StylizedResolvedCapabilities>();
    let resolved = caps
        .resolved
        .as_ref()
        .expect("portable baseline must resolve on the CPU fallback");
    assert_eq!(resolved.class, StylizedSurfaceClass::Cel);
    assert_eq!(resolved.tier, StylizedExecutionTier::Portable);
    assert!(!resolved.tier_reduced);
    assert!(caps.actual.features.is_empty());
}

#[test]
fn capability_config_resource_overrides_plugin_default() {
    // Given：用户在 add_plugins 之前插入自定义 Stage A 配置。
    let mut app = App::new();
    app.insert_resource(StylizedCapabilityConfig {
        requested_class: StylizedSurfaceClass::Hero,
        requested_tier: StylizedExecutionTier::High,
        fallback_policy: StylizedFallbackPolicy::Fail,
        ..StylizedCapabilityConfig::default()
    });
    app.add_plugins(MinimalPlugins)
        // StylizedPlugin 默认开启 `render` feature；MaterialPlugin 与
        // embedded shader 注册需要 AssetServer（Bevy 插件惯例）。
        .add_plugins(bevy::asset::AssetPlugin {
            meta_check: bevy::asset::AssetMetaCheck::Never,
            ..Default::default()
        })
        .add_plugins(StylizedPlugin::default());
    app.finish();
    app.cleanup();
    update_n(&mut app, 1);

    // Then：CPU 兜底能力无法满足 High + Fail 政策 → fail-closed，
    // 且 class 不被降级（错误中保留 Hero）。
    let caps = app.world().resource::<StylizedResolvedCapabilities>();
    let error = caps
        .resolved
        .as_ref()
        .expect_err("High + Fail on empty caps must fail closed");
    assert_eq!(error.class, StylizedSurfaceClass::Hero);
    assert_eq!(error.requested_tier, StylizedExecutionTier::High);

    // 资源覆盖生效（插件默认配置不应覆盖用户 resource）。
    let config = app.world().resource::<StylizedCapabilityConfig>();
    assert_eq!(config.requested_tier, StylizedExecutionTier::High);
}

#[test]
fn reduce_tier_policy_keeps_class_on_cpu_fallback() {
    let mut app = App::new();
    app.insert_resource(StylizedCapabilityConfig {
        requested_class: StylizedSurfaceClass::Character,
        requested_tier: StylizedExecutionTier::Standard,
        fallback_policy: StylizedFallbackPolicy::ReduceTier,
        ..StylizedCapabilityConfig::default()
    });
    app.add_plugins(MinimalPlugins)
        // StylizedPlugin 默认开启 `render` feature；MaterialPlugin 与
        // embedded shader 注册需要 AssetServer（Bevy 插件惯例）。
        .add_plugins(bevy::asset::AssetPlugin {
            meta_check: bevy::asset::AssetMetaCheck::Never,
            ..Default::default()
        })
        .add_plugins(StylizedPlugin::default());
    app.finish();
    app.cleanup();
    update_n(&mut app, 1);

    let caps = app.world().resource::<StylizedResolvedCapabilities>();
    let resolved = caps
        .resolved
        .as_ref()
        .expect("ReduceTier must fall back to Portable");
    assert_eq!(resolved.class, StylizedSurfaceClass::Character);
    assert_eq!(resolved.tier, StylizedExecutionTier::Portable);
    assert!(resolved.tier_reduced);

    // FallbackReport：降档 + Character 链步骤齐全（It-7）。
    let report = caps.report.as_ref().expect("fallback report present");
    assert!(report.tier_reduced);
    assert_eq!(report.class, StylizedSurfaceClass::Character);
    assert_eq!(
        report.applied_steps,
        bevy_stylized::stylized_core::fallback_chain(StylizedSurfaceClass::Character)
    );
    assert!(!report.notes.is_empty());
}
