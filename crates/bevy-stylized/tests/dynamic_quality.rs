//! 动态质量 facade 接线测试（V2-c）：合成诊断喂入 → 资源更新 → 决策事件。

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    ecs::message::Messages,
    prelude::*,
};
use bevy_stylized::prelude::*;

/// 小冷却/资格的测试配置，少量 update 内即可观察迁移。
fn test_config() -> StylizedQualityGovernorConfig {
    StylizedQualityGovernorConfig {
        target_frame_time_ms: 10.0,
        downgrade_overrun_ratio: 1.2,
        upgrade_headroom_ratio: 0.8,
        downgrade_cooldown_ticks: 2,
        upgrade_cooldown_ticks: 2,
        upgrade_qualification_ticks: 2,
        ema_alpha: 1.0, // 不平滑：合成信号直接生效
        ..StylizedQualityGovernorConfig::default()
    }
}

fn build_app() -> App {
    let mut app = App::new();
    app.insert_resource(StylizedCapabilityConfig {
        requested_class: StylizedSurfaceClass::Cel,
        requested_tier: StylizedExecutionTier::High,
        ..StylizedCapabilityConfig::default()
    });
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::asset::AssetPlugin {
            meta_check: bevy::asset::AssetMetaCheck::Never,
            ..Default::default()
        })
        .add_plugins(StylizedPlugin::default())
        .add_plugins(StylizedDynamicQualityPlugin {
            config: test_config(),
            enabled: true,
        });
    app.finish();
    app.cleanup();
    app
}

/// 模拟一台解析为 High 的设备（headless 下 Stage B 落 CPU 兜底 Portable，
/// 会把 governor 上限同步钳到 Portable；覆盖资源以观察完整降/升链路）。
fn simulate_high_device(app: &mut App) {
    app.insert_resource(StylizedResolvedCapabilities {
        actual: StylizedCapabilitySet::default(),
        resolved: Ok(StylizedResolvedVariant {
            class: StylizedSurfaceClass::Cel,
            requested_tier: StylizedExecutionTier::High,
            tier: StylizedExecutionTier::High,
            tier_reduced: false,
            missing_features: Vec::new(),
        }),
        report: None,
    });
}

fn update_n(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

fn decision_events(app: &mut App) -> Vec<StylizedQualityDecisionEvent> {
    app.world_mut()
        .resource_mut::<Messages<StylizedQualityDecisionEvent>>()
        .drain()
        .collect()
}

#[test]
fn dynamic_quality_is_opt_in_off_by_default() {
    // 不装 StylizedDynamicQualityPlugin：无资源、无 override，行为同 V1。
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::asset::AssetPlugin {
            meta_check: bevy::asset::AssetMetaCheck::Never,
            ..Default::default()
        })
        .add_plugins(StylizedPlugin::default());
    app.finish();
    app.cleanup();
    update_n(&mut app, 2);

    assert!(!app.world().contains_resource::<StylizedDynamicQuality>());
    assert!(!app.world().contains_resource::<StylizedTierOverride>());
}

#[test]
fn synthetic_feed_downgrades_and_recovers_with_events() {
    let mut app = build_app();
    simulate_high_device(&mut app);

    // Given：合成帧时 20ms（降档阈值 12ms），持续超支。
    app.world_mut()
        .resource_mut::<StylizedDynamicQuality>()
        .frame_time_override_ms = Some(20.0);

    // When
    update_n(&mut app, 3);

    // Then：governor 降档（High→Standard→Portable，冷却 2 tick），
    // tier override 落为 Portable，每次变更都有事件。
    let quality = app.world().resource::<StylizedDynamicQuality>();
    assert_eq!(quality.governor.tier(), StylizedExecutionTier::Portable);
    let last = quality.last_decision.expect("decision recorded");
    assert!(!last.reason.is_empty());
    assert_eq!(last.signals.frame_time_ms, 20.0);

    let tier_override = app.world().resource::<StylizedTierOverride>();
    assert_eq!(tier_override.0, Some(StylizedExecutionTier::Portable));
    assert_eq!(
        tier_override.effective_tier(StylizedExecutionTier::High),
        StylizedExecutionTier::Portable
    );

    let events = decision_events(&mut app);
    assert_eq!(events.len(), 2, "High→Standard→Portable, one event each");
    assert!(events.iter().all(|e| e.0.changed));
    assert_eq!(events[0].0.tier, StylizedExecutionTier::Standard);
    assert_eq!(events[1].0.tier, StylizedExecutionTier::Portable);

    // Given：帧时恢复 5ms（< 8ms 余量带）。
    app.world_mut()
        .resource_mut::<StylizedDynamicQuality>()
        .frame_time_override_ms = Some(5.0);

    // When：资格 2 + 冷却 2 → 每次升档约 2 tick。
    update_n(&mut app, 8);

    // Then：升回上限（High），override 清空（回到请求档），事件齐全。
    let quality = app.world().resource::<StylizedDynamicQuality>();
    assert_eq!(quality.governor.tier(), StylizedExecutionTier::High);
    let tier_override = app.world().resource::<StylizedTierOverride>();
    assert_eq!(tier_override.0, None);

    let events = decision_events(&mut app);
    assert_eq!(events.len(), 2, "Portable→Standard→High, one event each");
    assert_eq!(events[0].0.tier, StylizedExecutionTier::Standard);
    assert_eq!(events[1].0.tier, StylizedExecutionTier::High);

    // 历史环形缓冲：降 2 + 升 2 全部可观测。
    let history_len = app
        .world()
        .resource::<StylizedDynamicQuality>()
        .governor
        .history_len();
    assert_eq!(history_len, 4);
}

#[test]
fn stage_b_portable_fallback_clamps_governor_ceiling() {
    // 不覆盖 Stage B 结果：CPU 兜底 Portable 上限应把 governor 从 High
    // 钳到 Portable（正确性优先，不消耗冷却）。
    let mut app = build_app();
    app.world_mut()
        .resource_mut::<StylizedDynamicQuality>()
        .frame_time_override_ms = Some(5.0);
    update_n(&mut app, 1);

    let quality = app.world().resource::<StylizedDynamicQuality>();
    assert_eq!(quality.governor.ceiling(), StylizedExecutionTier::Portable);
    assert_eq!(quality.governor.tier(), StylizedExecutionTier::Portable);
    let last_transition = quality
        .governor
        .history()
        .last()
        .expect("ceiling clamp recorded");
    assert_eq!(last_transition.reason, "capability ceiling lowered");
}

#[test]
fn disabled_resource_freezes_governor() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(StylizedDynamicQualityPlugin {
            config: test_config(),
            enabled: false,
        });
    app.finish();
    app.cleanup();

    app.world_mut()
        .resource_mut::<StylizedDynamicQuality>()
        .frame_time_override_ms = Some(50.0);
    update_n(&mut app, 5);

    let quality = app.world().resource::<StylizedDynamicQuality>();
    assert!(
        quality.last_decision.is_none(),
        "disabled: no ticks consumed"
    );
    assert_eq!(app.world().resource::<StylizedTierOverride>().0, None);
}

#[test]
fn real_diagnostics_feed_reaches_governor() {
    // 不经 override：直接向 DiagnosticsStore 写测量值，验证
    // FrameTimeDiagnostics 链路（插件自动补装 + smoothed/value 读取）。
    // （MinimalPlugins 无 FrameCountPlugin，FrameTime 自带系统不产生
    // 干扰测量。）
    let mut app = build_app();
    simulate_high_device(&mut app);

    for _ in 0..2 {
        let mut store = app.world_mut().resource_mut::<DiagnosticsStore>();
        if let Some(diagnostic) = store.get_mut(&FrameTimeDiagnosticsPlugin::FRAME_TIME) {
            diagnostic.add_measurement(bevy::diagnostic::DiagnosticMeasurement {
                time: std::time::Instant::now(),
                value: 20.0,
            });
        }
        app.update();
    }

    let quality = app.world().resource::<StylizedDynamicQuality>();
    let last = quality.last_decision.expect("decision recorded");
    assert!(
        last.smoothed_frame_time_ms > 10.0,
        "diagnostic feed must reach the governor: {:?}",
        last.smoothed_frame_time_ms
    );
    assert!(quality.governor.tier() < StylizedExecutionTier::High);
    assert_eq!(
        app.world()
            .resource::<StylizedDynamicQuality>()
            .last_decision
            .unwrap()
            .signals
            .timing_source,
        QualityTimingSource::FrameInterval
    );
}

#[test]
fn governor_override_reaches_per_instance_lod() {
    // 端到端（V2-b × V2-c）：合成帧时 → governor 降档 → StylizedTierOverride
    // → render sync_lod 把 near 实例压为 portable fallback（MeshTag flags）。
    use bevy_stylized::stylized_render::{INSTANCE_FLAG_DROP_HERO_EXTRAS, StylizedInstanceData};

    let mut app = build_app();
    simulate_high_device(&mut app);
    app.world_mut().spawn((
        Camera::default(),
        Transform::from_xyz(0.0, 0.0, 0.0),
        GlobalTransform::from_xyz(0.0, 0.0, 0.0),
    ));
    // near 带（距离 2 < mid_start 10）：LOD 自身不削减，只有 override 能压它。
    let instance = app
        .world_mut()
        .spawn((
            StylizedInstanceData::default(),
            Transform::from_xyz(0.0, 0.0, -2.0),
            GlobalTransform::from_xyz(0.0, 0.0, -2.0),
        ))
        .id();

    app.world_mut()
        .resource_mut::<StylizedDynamicQuality>()
        .frame_time_override_ms = Some(20.0);
    update_n(&mut app, 4);

    assert_eq!(
        app.world().resource::<StylizedTierOverride>().0,
        Some(StylizedExecutionTier::Portable),
        "governor override 应已生效"
    );
    let flags = app
        .world()
        .entity(instance)
        .get::<StylizedInstanceData>()
        .expect("instance data")
        .flags;
    assert_ne!(
        flags & INSTANCE_FLAG_DROP_HERO_EXTRAS,
        0,
        "override=Portable 时 LOD 必须压低头实例（min 语义）"
    );
}

#[test]
fn invalid_override_does_not_advance_the_governor() {
    let mut app = build_app();
    simulate_high_device(&mut app);
    app.world_mut()
        .resource_mut::<StylizedDynamicQuality>()
        .frame_time_override_ms = Some(10.0);
    app.update();
    let before = app
        .world()
        .resource::<StylizedDynamicQuality>()
        .last_decision;
    assert_eq!(
        before.unwrap().signals.timing_source,
        QualityTimingSource::HostProvided
    );
    assert!(before.unwrap().signals.observed_frame.is_some());
    for value in [f32::NAN, f32::INFINITY, -1.0, 0.0] {
        app.world_mut()
            .resource_mut::<StylizedDynamicQuality>()
            .frame_time_override_ms = Some(value);
        app.update();
        assert_eq!(
            app.world()
                .resource::<StylizedDynamicQuality>()
                .last_decision,
            before
        );
    }
}
