//! 动态质量 facade 接线（V2-c，docs/roadmap/02 §8 V2-c）。
//!
//! **opt-in（默认关闭）**：不装 [`StylizedDynamicQualityPlugin`] 时框架行为
//! 与 V1 完全一致；装上后默认启用（插件本身就是 opt-in 手势，可用
//! `enabled: false` 预装但暂停）。
//!
//! 数据流：
//!
//! ```text
//! FrameTimeDiagnosticsPlugin (FRAME_TIME)
//!   → dynamic_quality_system (PostUpdate)
//!   → core StylizedQualityGovernor::tick(QualitySignals)
//!   → QualityDecision（变更时发出 StylizedQualityDecisionEvent）
//!   → StylizedTierOverride（tier 覆盖；有效 tier = min(config.requested_tier, override)）
//! ```
//!
//! **边界**：决策当前只落到 tier override；应用 override 做实际渲染
//! variant 切换属于 V2-b（LOD / variant 切换）工作。分辨率 / 场景复杂度 /
//! 热压力 / 电量由调用方写入 [`StylizedDynamicQuality`] 相应字段（平台
//! 信号接线尚未提供，字段即注入点）。

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    ecs::message::Message,
    prelude::*,
};
use bevy_stylized_core::{
    QualityDecision, QualitySignals, StylizedExecutionTier, StylizedQualityGovernor,
    StylizedQualityGovernorConfig,
};

use crate::capability::{StylizedCapabilityConfig, StylizedResolvedCapabilities};

/// 动态质量资源：governor 实例 + 外部信号注入点。
///
/// `resolution` / `scene_complexity` / `thermal_pressure` / `battery_level`
/// 每帧由调用方（或未来的平台接线）写入，系统在下一 tick 消费。
#[derive(Resource)]
pub struct StylizedDynamicQuality {
    /// core 纯逻辑调节器（迟滞 / 冷却 / 历史全在其中）。
    pub governor: StylizedQualityGovernor,
    /// 是否启用（false 时系统完全空转，override 保持不变）。
    pub enabled: bool,
    /// 最近一次决策（每 tick 刷新；观测用，含 reason + 信号快照）。
    pub last_decision: Option<QualityDecision>,
    /// 渲染分辨率（宽, 高；0 = 未提供）。
    pub resolution: [u32; 2],
    /// 场景复杂度估计（0 = 未提供；单位由调用方与 governor 预算约定）。
    pub scene_complexity: f32,
    /// 平台热压力（0..=1；`None` = 平台不暴露）。
    pub thermal_pressure: Option<f32>,
    /// 电池电量（0..=1；`None` = 平台不暴露）。
    pub battery_level: Option<f32>,
    /// 合成帧时注入点（测试 / 真实 GPU timestamp 来源）：
    /// `Some(ms)` 时优先于 FrameTimeDiagnostics 的读数。
    pub gpu_frame_time_override_ms: Option<f32>,
}

impl StylizedDynamicQuality {
    /// 以 `config` 构造；初始 tier 与上限取 `initial_ceiling`
    /// （facade 用 Stage A 的 `requested_tier`，运行期由 Stage B 结果收紧）。
    #[must_use]
    pub fn new(
        config: StylizedQualityGovernorConfig,
        initial_ceiling: StylizedExecutionTier,
        enabled: bool,
    ) -> Self {
        Self {
            governor: StylizedQualityGovernor::new(config, initial_ceiling, initial_ceiling),
            enabled,
            last_decision: None,
            resolution: [0, 0],
            scene_complexity: 0.0,
            thermal_pressure: None,
            battery_level: None,
            gpu_frame_time_override_ms: None,
        }
    }
}

/// 动态质量给出的 tier 覆盖（`None` = 与 Stage A 请求档一致，无动态调整）。
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StylizedTierOverride(pub Option<StylizedExecutionTier>);

impl StylizedTierOverride {
    /// 有效请求档：`min(requested, override)`（动态调整只降不升于请求档）。
    #[must_use]
    pub fn effective_tier(&self, requested: StylizedExecutionTier) -> StylizedExecutionTier {
        match self.0 {
            Some(override_tier) => requested.min(override_tier),
            None => requested,
        }
    }
}

/// tier 变更事件（仅在 `QualityDecision.changed` 时发出，避免洪泛；
/// 完整逐 tick 决策见 [`StylizedDynamicQuality::last_decision`] 与 governor
/// 的 transition 历史）。
#[derive(Message, Clone, Copy, Debug)]
pub struct StylizedQualityDecisionEvent(pub QualityDecision);

/// 动态质量插件（opt-in）。
///
/// 若 App 尚未注册 [`FrameTimeDiagnosticsPlugin`] 会自动补装（帧时来源）。
pub struct StylizedDynamicQualityPlugin {
    /// governor 配置（阈值 / 冷却 / 历史容量）。
    pub config: StylizedQualityGovernorConfig,
    /// 是否启用（默认 true——装插件即 opt-in）。
    pub enabled: bool,
}

impl Default for StylizedDynamicQualityPlugin {
    fn default() -> Self {
        Self {
            config: StylizedQualityGovernorConfig::default(),
            enabled: true,
        }
    }
}

impl Plugin for StylizedDynamicQualityPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }
        if !app.world().contains_resource::<StylizedDynamicQuality>() {
            let ceiling = app
                .world()
                .get_resource::<StylizedCapabilityConfig>()
                .map_or_else(StylizedExecutionTier::default, |c| c.requested_tier);
            app.insert_resource(StylizedDynamicQuality::new(
                self.config,
                ceiling,
                self.enabled,
            ));
        }
        app.init_resource::<StylizedTierOverride>()
            .add_message::<StylizedQualityDecisionEvent>()
            .add_systems(PostUpdate, dynamic_quality_system);
    }
}

/// 每帧采集信号 → governor tick → 应用决策（tier override + 事件）。
pub fn dynamic_quality_system(
    diagnostics: Option<Res<DiagnosticsStore>>,
    resolved_caps: Option<Res<StylizedResolvedCapabilities>>,
    mut quality: ResMut<StylizedDynamicQuality>,
    mut tier_override: ResMut<StylizedTierOverride>,
    mut decisions: MessageWriter<StylizedQualityDecisionEvent>,
) {
    if !quality.enabled {
        return;
    }
    // Stage B 结果就绪后收紧能力上限（升档永不越过设备解析档）。
    if let Some(caps) = &resolved_caps
        && let Ok(resolved) = &caps.resolved
    {
        quality.governor.set_ceiling(resolved.tier);
    }

    let frame_time_ms = quality.gpu_frame_time_override_ms.or_else(|| {
        diagnostics
            .as_ref()
            .and_then(|store| store.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME))
            .and_then(|diagnostic| diagnostic.smoothed().or_else(|| diagnostic.value()))
            .map(|ms| ms as f32)
    });
    let Some(frame_time_ms) = frame_time_ms else {
        return; // 尚无帧时样本（首帧 / 无 diagnostics）：跳过本 tick
    };

    let signals = QualitySignals {
        gpu_frame_time_ms: frame_time_ms,
        resolution: quality.resolution,
        scene_complexity: quality.scene_complexity,
        thermal_pressure: quality.thermal_pressure,
        battery_level: quality.battery_level,
    };
    let decision = quality.governor.tick(&signals);

    if decision.changed {
        let ceiling = quality.governor.ceiling();
        tier_override.0 = (decision.tier < ceiling).then_some(decision.tier);
        decisions.write(StylizedQualityDecisionEvent(decision));
    }
    quality.last_decision = Some(decision);
}
