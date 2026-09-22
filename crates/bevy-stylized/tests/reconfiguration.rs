//! 运行时能力重配置的 CPU 合同测试（ADR-0030）：
//!
//! - 只对 Stage B 快照做确定性重解析（测试中根本没有 RenderDevice）；
//! - 拒绝列出且仅列出缺失能力；失败请求保持资源逐字节不变；
//! - 代次/变更通知语义；幂等 no-op 请求不换代次、不发通知。

use std::collections::BTreeSet;

use bevy::prelude::{App, MinimalPlugins, World};
use bevy_stylized::prelude::*;
use bevy_stylized::stylized_core::resolve_variant;

/// 模拟 Metal 级设备的 Stage B 快照（Standard/High 可解析，Bindless 未启用）。
fn standard_capable_actual() -> StylizedCapabilitySet {
    StylizedCapabilitySet {
        features: BTreeSet::from([
            StylizedGpuFeature::Compute,
            StylizedGpuFeature::StorageBufferInVertexStage,
        ]),
        limits: StylizedGpuLimits {
            max_bind_groups: 4,
            max_storage_buffers_per_shader_stage: 8,
            max_uniform_buffers_per_shader_stage: 12,
        },
    }
}

fn portable_variant(actual: &StylizedCapabilitySet) -> StylizedResolvedVariant {
    resolve_variant(
        StylizedSurfaceClass::Cel,
        StylizedExecutionTier::Portable,
        actual,
        StylizedFallbackPolicy::ReduceTier,
    )
    .expect("portable resolves on default limits")
}

/// 手工装配的会话世界：Stage A 配置 + Stage B 快照（Portable 已解析），
/// 无 RenderDevice——证明重解析只读快照。
fn session_world(actual: StylizedCapabilitySet) -> World {
    let mut world = World::new();
    world.insert_resource(StylizedCapabilityConfig::default());
    let variant = portable_variant(&actual);
    world.insert_resource(StylizedRenderCapabilities::new(
        &actual,
        Ok(variant.clone()),
    ));
    world.insert_resource(StylizedResolvedCapabilities {
        actual,
        resolved: Ok(variant),
        report: None,
    });
    world
}

fn standard_request() -> StylizedCapabilityConfig {
    StylizedCapabilityConfig {
        requested_tier: StylizedExecutionTier::Standard,
        ..StylizedCapabilityConfig::default()
    }
}

#[test]
fn reconfiguration_resolves_against_stored_snapshot_without_device() {
    let actual = standard_capable_actual();
    let mut world = session_world(actual.clone());

    let applied = world
        .apply_stylized_reconfiguration(standard_request())
        .expect("standard resolves against the stored snapshot");
    assert!(applied.changed);
    assert_eq!(applied.generation, 1);
    assert_eq!(applied.current.tier, StylizedExecutionTier::Standard);
    assert!(!applied.current.tier_reduced);
    assert_eq!(
        applied.previous.map(|variant| variant.tier),
        Some(StylizedExecutionTier::Portable)
    );

    // 快照原样保留（绝不重读设备）；两份已解析资源同步更新。
    let resolved = world.resource::<StylizedResolvedCapabilities>();
    assert_eq!(resolved.actual, actual);
    assert_eq!(
        resolved.resolved.as_ref().map(|variant| variant.tier),
        Ok(StylizedExecutionTier::Standard)
    );
    assert!(resolved.report.is_some());
    let render = world.resource::<StylizedRenderCapabilities>();
    assert_eq!(render.actual, actual);
    assert_eq!(
        render.resolved.as_ref().map(|variant| variant.tier),
        Some(StylizedExecutionTier::Standard)
    );
    assert!(!render.hard_failure);
    // 请求配置成为新的现状；render crate 派生状态同事务刷新。
    assert_eq!(
        world.resource::<StylizedCapabilityConfig>().requested_tier,
        StylizedExecutionTier::Standard
    );
    assert_eq!(
        world
            .resource::<bevy_stylized::stylized_render::StylizedInstanceTransportMode>()
            .0,
        bevy_stylized::stylized_render::StylizedInstanceTransport::StorageTable
    );
}

#[test]
fn refusal_lists_exactly_the_missing_features() {
    let mut world = session_world(standard_capable_actual());
    let mut request = standard_request();
    request
        .requested_features
        .insert(StylizedGpuFeature::Bindless);
    request
        .requested_features
        .insert(StylizedGpuFeature::Float16);

    let error = world
        .apply_stylized_reconfiguration(request)
        .expect_err("bindless is not enabled on the device snapshot");
    assert_eq!(
        error,
        StylizedReconfigError::FeaturesNotEnabled {
            missing: vec![StylizedGpuFeature::Bindless, StylizedGpuFeature::Float16],
        }
    );
}

#[test]
fn resolution_failure_is_structured_and_lists_blocker() {
    // Portable-only 快照（无 storage buffer）：Standard + Fail 政策被拒绝。
    let mut actual = standard_capable_actual();
    actual
        .features
        .remove(&StylizedGpuFeature::StorageBufferInVertexStage);
    actual.limits.max_storage_buffers_per_shader_stage = 0;
    let mut world = session_world(actual);
    let mut request = standard_request();
    request.fallback_policy = StylizedFallbackPolicy::Fail;

    let error = world
        .apply_stylized_reconfiguration(request)
        .expect_err("standard cannot resolve without storage buffers");
    let StylizedReconfigError::ResolutionFailed(resolve_error) = error else {
        panic!("expected resolution failure, got {error:?}");
    };
    assert_eq!(resolve_error.blocking_tier, StylizedExecutionTier::Standard);
    assert_eq!(
        resolve_error.missing_features,
        vec![StylizedGpuFeature::StorageBufferInVertexStage]
    );
    assert!(!resolve_error.limits_satisfied);
}

#[test]
fn failed_request_leaves_session_state_byte_identical() {
    let mut world = session_world(standard_capable_actual());
    world.insert_resource(StylizedReconfigGeneration(7));
    let config_before = world.resource::<StylizedCapabilityConfig>().clone();
    let resolved_before = world.resource::<StylizedResolvedCapabilities>().clone();
    let render_before = world.resource::<StylizedRenderCapabilities>().clone();
    let mut request = standard_request();
    request
        .requested_features
        .insert(StylizedGpuFeature::Bindless);

    let _ = world
        .apply_stylized_reconfiguration(request)
        .expect_err("refused");

    let config_after = world.resource::<StylizedCapabilityConfig>();
    assert_eq!(*config_after, config_before);
    let resolved_after = world.resource::<StylizedResolvedCapabilities>();
    assert_eq!(resolved_after.actual, resolved_before.actual);
    assert_eq!(resolved_after.resolved, resolved_before.resolved);
    let render_after = world.resource::<StylizedRenderCapabilities>();
    assert_eq!(render_after.actual, render_before.actual);
    assert_eq!(render_after.resolved, render_before.resolved);
    assert_eq!(render_after.resolve_error, render_before.resolve_error);
    assert_eq!(
        world.resource::<StylizedReconfigGeneration>().0,
        7,
        "refused requests never bump the generation"
    );
}

#[test]
fn idempotent_request_is_a_noop() {
    let mut world = session_world(standard_capable_actual());
    world.init_resource::<bevy::ecs::message::Messages<StylizedCapabilitiesChanged>>();

    let first = world
        .apply_stylized_reconfiguration(standard_request())
        .expect("first application");
    assert!(first.changed);
    assert_eq!(first.generation, 1);
    let second = world
        .apply_stylized_reconfiguration(standard_request())
        .expect("idempotent replay");
    assert!(!second.changed);
    assert_eq!(second.generation, 1, "no-op never bumps the generation");
    assert_eq!(second.current, first.current);

    let messages = world.resource::<bevy::ecs::message::Messages<StylizedCapabilitiesChanged>>();
    assert_eq!(
        messages.len(),
        1,
        "only the real change emits a notification"
    );
}

#[test]
fn change_notification_carries_generation_and_variants() {
    let mut world = session_world(standard_capable_actual());
    world.init_resource::<bevy::ecs::message::Messages<StylizedCapabilitiesChanged>>();

    let applied = world
        .apply_stylized_reconfiguration(standard_request())
        .expect("applied");
    let messages = world.resource::<bevy::ecs::message::Messages<StylizedCapabilitiesChanged>>();
    let notification = messages
        .iter_current_update_messages()
        .last()
        .expect("one message");
    assert_eq!(notification.generation, applied.generation);
    assert_eq!(
        notification.previous.as_ref().map(|variant| variant.tier),
        Some(StylizedExecutionTier::Portable)
    );
    assert_eq!(notification.current, applied.current);
    assert_eq!(
        notification.report.resolved_tier,
        StylizedExecutionTier::Standard
    );
}

#[test]
fn missing_snapshot_is_a_structured_refusal() {
    let mut world = World::new();
    let error = world
        .apply_stylized_reconfiguration(standard_request())
        .expect_err("no Stage B snapshot");
    assert_eq!(error, StylizedReconfigError::NoCapabilitySnapshot);
}

/// 与 `StylizedPlugin` 的真实装配集成：CPU 兜底会话上请求 Standard 被拒绝
/// （兜底 actual 不含 storage buffer），会话保持 Portable 且不崩溃。
#[test]
fn plugin_session_refuses_upgrade_beyond_cpu_fallback() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::asset::AssetPlugin {
            meta_check: bevy::asset::AssetMetaCheck::Never,
            ..Default::default()
        })
        .add_plugins(StylizedPlugin::default());
    app.finish();
    app.cleanup();

    let mut request = standard_request();
    request.fallback_policy = StylizedFallbackPolicy::Fail;
    let error = app
        .world_mut()
        .apply_stylized_reconfiguration(request)
        .expect_err("cpu fallback cannot resolve Standard under Fail policy");
    assert!(matches!(error, StylizedReconfigError::ResolutionFailed(_)));
    let resolved = app.world().resource::<StylizedResolvedCapabilities>();
    assert_eq!(
        resolved.resolved.as_ref().map(|variant| variant.tier),
        Ok(StylizedExecutionTier::Portable),
        "failed reconfiguration never impersonates reinitialization"
    );
}
