//! 动态质量调节器（V2-c，docs/roadmap/02 §8 V2-c / 01 §Dynamic Quality）。
//!
//! 纯逻辑状态机，不含任何 Bevy App/ECS 类型：每 tick 消费 [`QualitySignals`]
//! （GPU 帧时 / 分辨率 / 场景复杂度 / 可选热压力与电量），产出
//! [`QualityDecision`]（目标 tier + 可选 feature/LOD 调整建议）。
//!
//! 不变量（roadmap 强制项）：
//!
//! - **迟滞（hysteresis）**：升档阈值（`target * upgrade_headroom_ratio`）严格
//!   低于降档阈值（`target * downgrade_overrun_ratio`），构造时 fail-closed
//!   校验；
//! - **冷却（cooldown）**：升/降档各有独立的最小 tick 间隔；
//! - **稳定过渡**：单次 transition 只移动一档（`High→Standard→Portable`），
//!   帧时经 EMA 平滑，升档另需连续 `upgrade_qualification_ticks` 次达标，
//!   抖动输入下不产生振荡（有 jitter 测试钉死）；
//! - **可观测**：每次决策携带 reason（`&'static str`，热路径零分配）与信号
//!   快照；tier 变更写入固定容量环形历史缓冲。
//!
//! 本模块是决策核心；facade 的 `StylizedDynamicQuality` 负责采集信号
//! （FrameTimeDiagnostics）并消费决策（tier override）。render 侧的
//! variant 切换落地属于 V2-b。

use crate::axis::StylizedExecutionTier;

/// 调节器配置（阈值 / 冷却 / 历史容量）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StylizedQualityGovernorConfig {
    /// 目标帧时（ms，如 60fps = 16.6）。
    pub target_frame_time_ms: f32,
    /// 降档超支比（> 1）：EMA 帧时 ≥ `target * ratio` 构成降档压力。
    pub downgrade_overrun_ratio: f32,
    /// 升档余量比（< 1）：EMA 帧时 ≤ `target * ratio` 计入升档资格。
    /// **必须严格低于** `downgrade_overrun_ratio`（迟滞带）。
    pub upgrade_headroom_ratio: f32,
    /// 场景复杂度预算（任意单位，由调用方定义）；≤ 0 表示不启用该信号。
    pub scene_complexity_budget: f32,
    /// 热压力阈值（0..=1）：达到即强制降档压力并锁死升档。
    pub thermal_throttle_threshold: f32,
    /// 电量节电阈值（0..=1；0 = 关闭）：低于即构成降档压力。
    pub battery_saver_below: f32,
    /// 降档后的最小间隔 tick 数（降档应快，默认较短）。
    pub downgrade_cooldown_ticks: u32,
    /// 升档后的最小间隔 tick 数（升档应慢，默认较长）。
    pub upgrade_cooldown_ticks: u32,
    /// 升档前所需的连续"处于余量带"tick 数（抗抖动资格计数）。
    pub upgrade_qualification_ticks: u32,
    /// 帧时 EMA 平滑系数（0, 1]；越小越平滑）。
    pub ema_alpha: f32,
    /// 历史环形缓冲容量（仅记录 tier 变更）。
    pub history_capacity: usize,
}

impl Default for StylizedQualityGovernorConfig {
    /// 默认取向：60fps 目标、±15% 迟滞带、降档 30 tick / 升档 120 tick
    /// 冷却 + 30 tick 升档资格（约 0.5s 降 / 2s 升，升级显著更保守）。
    fn default() -> Self {
        Self {
            target_frame_time_ms: 16.6,
            downgrade_overrun_ratio: 1.15,
            upgrade_headroom_ratio: 0.85,
            scene_complexity_budget: 0.0,
            thermal_throttle_threshold: 0.9,
            battery_saver_below: 0.0,
            downgrade_cooldown_ticks: 30,
            upgrade_cooldown_ticks: 120,
            upgrade_qualification_ticks: 30,
            ema_alpha: 0.2,
            history_capacity: 64,
        }
    }
}

/// 每 tick 的输入信号（热/电为平台可得时的可选信号）。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QualitySignals {
    /// 本 tick 观测到的帧时（ms；facade 侧为 diagnostics 平滑值）。
    pub gpu_frame_time_ms: f32,
    /// 渲染分辨率（宽高像素；当前仅入快照，供观测与未来分辨率调整）。
    pub resolution: [u32; 2],
    /// 场景复杂度估计（单位由调用方定义；0 = 未提供）。
    pub scene_complexity: f32,
    /// 平台热压力（0..=1；`None` = 平台不暴露）。
    pub thermal_pressure: Option<f32>,
    /// 电池电量（0..=1；`None` = 平台不暴露或插电忽略）。
    pub battery_level: Option<f32>,
}

/// 随决策附带的次级调整建议（feature / LOD 方向；落地在 V2-b）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum QualityAdjustment {
    /// 无附加调整（仅 tier 或不变）。
    #[default]
    None,
    /// 削减场景复杂度（LOD / 实例密度方向）。
    ReduceSceneComplexity,
    /// 降低内部分辨率（热/电压力方向）。
    ReduceResolution,
    /// 沿 per-class fallback 链削减特性（帧时超支方向）。
    RequestFeatureFallback,
}

/// 一次决策的输出（每 tick 一条）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QualityDecision {
    /// tick 序号（从 1 开始）。
    pub tick: u64,
    /// 决策后的目标 tier。
    pub tier: StylizedExecutionTier,
    /// 本 tick 是否发生了 tier 变更。
    pub changed: bool,
    /// 次级调整建议（不变更时为 `None`）。
    pub adjustment: QualityAdjustment,
    /// 人可读原因（静态字符串，热路径零分配）。
    pub reason: &'static str,
    /// 产生本决策时的信号快照（观测用）。
    pub signals: QualitySignals,
    /// EMA 平滑后的帧时（ms）。
    pub smoothed_frame_time_ms: f32,
}

/// 一次 tier 变更的历史记录（环形缓冲条目）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QualityTransition {
    /// 发生时的 tick 序号。
    pub tick: u64,
    /// 变更前档位。
    pub from: StylizedExecutionTier,
    /// 变更后档位。
    pub to: StylizedExecutionTier,
    /// 变更原因（静态字符串）。
    pub reason: &'static str,
    /// 当时的 EMA 帧时（ms）。
    pub smoothed_frame_time_ms: f32,
}

/// 动态质量调节器（纯逻辑状态机）。
///
/// 确定性：同输入序列必得同决策序列；热路径（`tick` 且 tier 不变）零分配
/// （历史缓冲在构造时预分配，reason 为静态字符串）。
pub struct StylizedQualityGovernor {
    config: StylizedQualityGovernorConfig,
    /// 设备能力上限（Stage B 解析档），升档永不越过。
    ceiling: StylizedExecutionTier,
    tier: StylizedExecutionTier,
    tick: u64,
    smoothed_ms: f32,
    has_smoothed: bool,
    up_qualification: u32,
    /// 最近一次降/升档的 tick（`None` = 从未发生，视为冷却已过）。
    last_down_tick: Option<u64>,
    last_up_tick: Option<u64>,
    history: Vec<QualityTransition>,
    history_head: usize,
    history_len: usize,
}

impl StylizedQualityGovernor {
    /// 构造调节器；`initial_tier` 会被 `ceiling` 截断。
    ///
    /// # Panics
    ///
    /// 配置非法时 fail-closed：`downgrade_overrun_ratio` 必须严格大于
    /// `upgrade_headroom_ratio`（迟滞带非空），`target_frame_time_ms > 0`，
    /// `ema_alpha` ∈ (0, 1]，`history_capacity > 0`。
    #[must_use]
    pub fn new(
        config: StylizedQualityGovernorConfig,
        initial_tier: StylizedExecutionTier,
        ceiling: StylizedExecutionTier,
    ) -> Self {
        assert!(
            config.downgrade_overrun_ratio > config.upgrade_headroom_ratio,
            "hysteresis band must be non-empty: downgrade ratio ({}) must be \
             strictly greater than upgrade ratio ({})",
            config.downgrade_overrun_ratio,
            config.upgrade_headroom_ratio,
        );
        assert!(
            config.target_frame_time_ms > 0.0,
            "target frame time must be positive"
        );
        assert!(
            config.ema_alpha > 0.0 && config.ema_alpha <= 1.0,
            "ema_alpha must be in (0, 1]"
        );
        assert!(config.history_capacity > 0, "history capacity must be > 0");
        Self {
            config,
            ceiling,
            tier: initial_tier.min(ceiling),
            tick: 0,
            smoothed_ms: 0.0,
            has_smoothed: false,
            up_qualification: 0,
            last_down_tick: None,
            last_up_tick: None,
            history: Vec::with_capacity(config.history_capacity),
            history_head: 0,
            history_len: 0,
        }
    }

    /// 当前目标 tier。
    #[must_use]
    pub fn tier(&self) -> StylizedExecutionTier {
        self.tier
    }

    /// 当前能力上限。
    #[must_use]
    pub fn ceiling(&self) -> StylizedExecutionTier {
        self.ceiling
    }

    /// 当前 EMA 平滑帧时（尚无样本时为 `None`）。
    #[must_use]
    pub fn smoothed_frame_time_ms(&self) -> Option<f32> {
        self.has_smoothed.then_some(self.smoothed_ms)
    }

    /// 更新能力上限（Stage B 解析结果就绪后由 facade 同步）。
    ///
    /// 若新上限低于当前 tier，立即截断并记录 transition
    /// （正确性优先于平滑；不消耗冷却）。
    pub fn set_ceiling(&mut self, ceiling: StylizedExecutionTier) {
        self.ceiling = ceiling;
        if self.tier > ceiling {
            let from = self.tier;
            self.tier = ceiling;
            self.record_transition(from, "capability ceiling lowered");
        }
    }

    /// 消费一个 tick 的信号并产出决策。
    pub fn tick(&mut self, signals: &QualitySignals) -> QualityDecision {
        self.tick += 1;
        self.smoothed_ms = if self.has_smoothed {
            self.config.ema_alpha * signals.gpu_frame_time_ms
                + (1.0 - self.config.ema_alpha) * self.smoothed_ms
        } else {
            self.has_smoothed = true;
            signals.gpu_frame_time_ms
        };
        let smoothed = self.smoothed_ms;
        let cfg = self.config;

        let down_threshold = cfg.target_frame_time_ms * cfg.downgrade_overrun_ratio;
        let up_threshold = cfg.target_frame_time_ms * cfg.upgrade_headroom_ratio;

        let thermal_forced = signals
            .thermal_pressure
            .is_some_and(|p| p >= cfg.thermal_throttle_threshold);
        let battery_saver = cfg.battery_saver_below > 0.0
            && signals
                .battery_level
                .is_some_and(|b| b <= cfg.battery_saver_below);
        let over_budget = cfg.scene_complexity_budget > 0.0
            && signals.scene_complexity > cfg.scene_complexity_budget;
        let over_time = smoothed >= down_threshold;
        let headroom = smoothed <= up_threshold;

        // 降档压力（任一路成立）；迟滞带内（既非超支也非余量）两路皆否。
        let down_pressure = over_time || over_budget || thermal_forced || battery_saver;

        if headroom && !down_pressure {
            self.up_qualification = self.up_qualification.saturating_add(1);
        } else {
            self.up_qualification = 0;
        }

        let down_cooled = self
            .last_down_tick
            .is_none_or(|t| self.tick >= t + u64::from(cfg.downgrade_cooldown_ticks));
        let up_cooled = self
            .last_up_tick
            .is_none_or(|t| self.tick >= t + u64::from(cfg.upgrade_cooldown_ticks));

        if down_pressure && self.tier > StylizedExecutionTier::Portable {
            let reason = if thermal_forced {
                "thermal pressure forces downgrade"
            } else if battery_saver {
                "battery saver forces downgrade"
            } else if over_budget {
                "scene complexity over budget"
            } else {
                "smoothed frame time over downgrade threshold"
            };
            if down_cooled {
                let from = self.tier;
                self.tier = from.reduced().expect("tier > Portable has a lower tier");
                self.last_down_tick = Some(self.tick);
                self.up_qualification = 0;
                self.record_transition(from, reason);
                let adjustment = if thermal_forced || battery_saver {
                    QualityAdjustment::ReduceResolution
                } else if over_budget {
                    QualityAdjustment::ReduceSceneComplexity
                } else {
                    QualityAdjustment::RequestFeatureFallback
                };
                return QualityDecision {
                    tick: self.tick,
                    tier: self.tier,
                    changed: true,
                    adjustment,
                    reason,
                    signals: *signals,
                    smoothed_frame_time_ms: smoothed,
                };
            }
            return self.steady(signals, smoothed, "downgrade pending: cooldown");
        }

        if !down_pressure
            && headroom
            && self.tier < self.ceiling
            && self.up_qualification >= cfg.upgrade_qualification_ticks
        {
            if up_cooled {
                let from = self.tier;
                self.tier = match from {
                    StylizedExecutionTier::Portable => StylizedExecutionTier::Standard,
                    StylizedExecutionTier::Standard => StylizedExecutionTier::High,
                    StylizedExecutionTier::High => unreachable!("tier < ceiling <= High"),
                };
                self.last_up_tick = Some(self.tick);
                self.up_qualification = 0;
                self.record_transition(from, "sustained headroom qualifies upgrade");
                return QualityDecision {
                    tick: self.tick,
                    tier: self.tier,
                    changed: true,
                    adjustment: QualityAdjustment::None,
                    reason: "sustained headroom qualifies upgrade",
                    signals: *signals,
                    smoothed_frame_time_ms: smoothed,
                };
            }
            return self.steady(signals, smoothed, "upgrade pending: cooldown");
        }

        let reason = if down_pressure {
            "downgrade pending: at floor"
        } else if !headroom {
            "steady: hysteresis band"
        } else if self.tier >= self.ceiling {
            "steady: at ceiling"
        } else {
            "upgrade pending: qualification"
        };
        self.steady(signals, smoothed, reason)
    }

    fn steady(
        &self,
        signals: &QualitySignals,
        smoothed: f32,
        reason: &'static str,
    ) -> QualityDecision {
        QualityDecision {
            tick: self.tick,
            tier: self.tier,
            changed: false,
            adjustment: QualityAdjustment::None,
            reason,
            signals: *signals,
            smoothed_frame_time_ms: smoothed,
        }
    }

    fn record_transition(&mut self, from: StylizedExecutionTier, reason: &'static str) {
        let entry = QualityTransition {
            tick: self.tick,
            from,
            to: self.tier,
            reason,
            smoothed_frame_time_ms: self.smoothed_ms,
        };
        if self.history_len < self.config.history_capacity {
            self.history.push(entry);
            self.history_len += 1;
        } else {
            self.history[self.history_head] = entry;
            self.history_head = (self.history_head + 1) % self.config.history_capacity;
        }
    }

    /// 历史 transition 数（≤ `history_capacity`）。
    #[must_use]
    pub fn history_len(&self) -> usize {
        self.history_len
    }

    /// 按时间顺序（最老 → 最新）遍历 transition 历史。
    pub fn history(&self) -> impl Iterator<Item = &QualityTransition> {
        let cap = self.config.history_capacity;
        let head = self.history_head;
        let history = &self.history;
        (0..self.history_len).map(move |i| &history[(head + i) % cap])
    }
}

impl std::fmt::Debug for StylizedQualityGovernor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StylizedQualityGovernor")
            .field("tier", &self.tier)
            .field("ceiling", &self.ceiling)
            .field("tick", &self.tick)
            .field("smoothed_ms", &self.smoothed_frame_time_ms())
            .field("history_len", &self.history_len)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use StylizedExecutionTier::{High, Portable, Standard};

    /// 测试配置：小冷却/资格数，便于少量 tick 内观察行为。
    fn test_config() -> StylizedQualityGovernorConfig {
        StylizedQualityGovernorConfig {
            target_frame_time_ms: 10.0,
            downgrade_overrun_ratio: 1.2, // 降档阈值 12ms
            upgrade_headroom_ratio: 0.8,  // 升档阈值 8ms（严格迟滞带 8..12）
            downgrade_cooldown_ticks: 5,
            upgrade_cooldown_ticks: 10,
            upgrade_qualification_ticks: 4,
            ema_alpha: 0.5,
            history_capacity: 8,
            ..StylizedQualityGovernorConfig::default()
        }
    }

    fn signals(ms: f32) -> QualitySignals {
        QualitySignals {
            gpu_frame_time_ms: ms,
            ..QualitySignals::default()
        }
    }

    fn tick_n(governor: &mut StylizedQualityGovernor, ms: f32, n: usize) -> QualityDecision {
        let mut last = governor.tick(&signals(ms));
        for _ in 1..n {
            last = governor.tick(&signals(ms));
        }
        last
    }

    #[test]
    #[should_panic(expected = "hysteresis band must be non-empty")]
    fn config_rejects_empty_hysteresis_band() {
        let config = StylizedQualityGovernorConfig {
            downgrade_overrun_ratio: 1.0,
            upgrade_headroom_ratio: 1.0,
            ..test_config()
        };
        let _ = StylizedQualityGovernor::new(config, High, High);
    }

    #[test]
    fn downgrade_triggers_on_sustained_overrun_one_step_at_a_time() {
        let mut governor = StylizedQualityGovernor::new(test_config(), High, High);
        // 持续 20ms（阈值 12ms）：EMA(alpha=0.5) 首 tick 即达 20。
        let decision = governor.tick(&signals(20.0));
        assert!(decision.changed);
        assert_eq!(decision.tier, Standard); // 只降一档，不跳档
        assert_eq!(
            decision.reason,
            "smoothed frame time over downgrade threshold"
        );
        assert_eq!(
            decision.adjustment,
            QualityAdjustment::RequestFeatureFallback
        );
        assert_eq!(decision.signals.gpu_frame_time_ms, 20.0);
        assert!(decision.smoothed_frame_time_ms >= 12.0);

        // 冷却内继续超支：不再降。
        for i in 0..4 {
            let d = governor.tick(&signals(20.0));
            assert!(!d.changed, "cooldown tick {i} must not transition");
            assert_eq!(d.reason, "downgrade pending: cooldown");
        }
        // 冷却结束（距上次降档 5 tick）：降到 Portable。
        let d = governor.tick(&signals(20.0));
        assert!(d.changed);
        assert_eq!(d.tier, Portable);

        // 已在地板：稳定，不再变化。
        let d = tick_n(&mut governor, 20.0, 20);
        assert!(!d.changed);
        assert_eq!(d.tier, Portable);
        assert_eq!(d.reason, "downgrade pending: at floor");
    }

    #[test]
    fn hysteresis_band_is_stable_from_both_sides() {
        // 带内信号（10ms，介于升 8 / 降 12 之间）：从 High 永不降、从
        // Portable 永不升。
        let mut from_high = StylizedQualityGovernor::new(test_config(), High, High);
        let d = tick_n(&mut from_high, 10.0, 100);
        assert_eq!(d.tier, High);
        assert_eq!(d.reason, "steady: hysteresis band");
        assert_eq!(from_high.history_len(), 0);

        let mut from_low = StylizedQualityGovernor::new(test_config(), Portable, High);
        let d = tick_n(&mut from_low, 10.0, 100);
        assert_eq!(d.tier, Portable);
        assert_eq!(from_low.history_len(), 0);
    }

    #[test]
    fn upgrade_requires_qualification_and_cooldown() {
        let mut governor = StylizedQualityGovernor::new(test_config(), Portable, High);
        // 6ms < 8ms 余量带：前 3 tick 资格不足。
        for i in 1..4 {
            let d = governor.tick(&signals(6.0));
            assert!(!d.changed, "tick {i}: qualification not met");
            assert_eq!(d.reason, "upgrade pending: qualification");
        }
        // 第 4 tick 资格满足（升档冷却初始即过期）→ 升一档。
        let d = governor.tick(&signals(6.0));
        assert!(d.changed);
        assert_eq!(d.tier, Standard);
        assert_eq!(d.reason, "sustained headroom qualifies upgrade");

        // 升档冷却 10 tick 内即便资格满足也不升。
        let d = tick_n(&mut governor, 6.0, 9);
        assert!(!d.changed);
        // 冷却结束 + 资格满足 → 升到 High（窗口内某个 tick 发生变更）。
        let mut upgraded = false;
        for _ in 0..5 {
            if governor.tick(&signals(6.0)).changed {
                upgraded = true;
            }
        }
        assert!(upgraded);
        assert_eq!(governor.tier(), High);

        // 到顶后稳定。
        let d = tick_n(&mut governor, 6.0, 20);
        assert!(!d.changed);
        assert_eq!(d.reason, "steady: at ceiling");
    }

    /// 确定性伪随机（LCG），避免测试引入 rand 依赖。
    fn lcg_sequence(seed: u64, n: usize) -> impl Iterator<Item = u64> {
        let mut state = seed;
        (0..n).map(move |_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            state >> 33
        })
    }

    #[test]
    fn jitter_around_downgrade_threshold_does_not_oscillate() {
        // 信号在降档阈值（12ms）附近 ±4ms 抖动，均值 12ms。
        // EMA(0.5) 平滑 + 冷却约束下：transition 数必须有界且最终稳定。
        let mut governor = StylizedQualityGovernor::new(test_config(), High, High);
        let mut transitions = 0u32;
        let mut tail_tiers = Vec::new();
        for (i, noise) in lcg_sequence(42, 2000).enumerate() {
            let ms = 12.0 + (noise % 8000) as f32 / 1000.0 - 4.0; // 8..16ms
            let d = governor.tick(&signals(ms));
            if d.changed {
                transitions += 1;
            }
            if i >= 1800 {
                tail_tiers.push(d.tier);
            }
        }
        // 最坏情况也受降档冷却约束（每 5 tick 至多一次降档，共 3 档 →
        // 最多 2 次降档；升档需连续 4 tick ≤8ms 且过 10 tick 冷却，
        // 在 8..16ms 均匀噪声下概率极低）。给一个保守上界。
        assert!(
            transitions <= 6,
            "jittery input must not oscillate: {transitions} transitions"
        );
        // 尾部 200 tick 内档位恒定（无振荡）。
        let final_tier = *tail_tiers.last().unwrap();
        assert!(
            tail_tiers.iter().all(|t| *t == final_tier),
            "tier must be stable in the tail: {tail_tiers:?}"
        );
    }

    #[test]
    fn alternating_fast_slow_frames_do_not_flip_flop() {
        // 快慢帧交替（20ms / 6ms，均值 13ms > 降档阈值 12ms）：
        // 只允许单调降档，绝不允许降完立刻升的乒乓。
        let mut governor = StylizedQualityGovernor::new(test_config(), High, High);
        let mut tiers = Vec::new();
        for i in 0..600 {
            let ms = if i % 2 == 0 { 20.0 } else { 6.0 };
            tiers.push(governor.tick(&signals(ms)).tier);
        }
        // 档位序列单调不升（EMA 交替后稳定在 13 附近，只降不升）。
        assert!(
            tiers.windows(2).all(|w| w[1] <= w[0]),
            "alternating jitter must never produce an upgrade flip-flop: {tiers:?}"
        );
        assert_eq!(*tiers.last().unwrap(), Portable);
        // 总 transition 数 == 2（High→Standard→Portable），无多余抖动。
        assert_eq!(governor.history_len(), 2);
    }

    #[test]
    fn thermal_and_battery_force_downgrade_regardless_of_frame_time() {
        let mut config = test_config();
        config.battery_saver_below = 0.2;
        let mut governor = StylizedQualityGovernor::new(config, High, High);
        let thermal = QualitySignals {
            gpu_frame_time_ms: 5.0, // 帧时极好
            thermal_pressure: Some(0.95),
            ..QualitySignals::default()
        };
        let d = governor.tick(&thermal);
        assert!(d.changed);
        assert_eq!(d.tier, Standard);
        assert_eq!(d.reason, "thermal pressure forces downgrade");
        assert_eq!(d.adjustment, QualityAdjustment::ReduceResolution);

        // 热压力持续期间锁死升档：帧时持续良好也只会单调降档，绝不回升。
        for _ in 0..30 {
            let d = governor.tick(&thermal);
            assert!(!d.changed || d.reason.contains("downgrade") || d.reason.contains("thermal"));
        }
        assert_eq!(governor.tier(), Portable);
        assert!(
            governor.history().all(|t| t.to < t.from),
            "thermal pressure must never allow an upgrade"
        );

        // 电量低于阈值同样构成降档压力。
        let mut governor = StylizedQualityGovernor::new(config, High, High);
        let battery = QualitySignals {
            gpu_frame_time_ms: 5.0,
            battery_level: Some(0.1),
            ..QualitySignals::default()
        };
        let d = governor.tick(&battery);
        assert!(d.changed);
        assert_eq!(d.reason, "battery saver forces downgrade");
    }

    #[test]
    fn scene_complexity_over_budget_suggests_lod_reduction() {
        let mut config = test_config();
        config.scene_complexity_budget = 100.0;
        let mut governor = StylizedQualityGovernor::new(config, High, High);
        let d = governor.tick(&QualitySignals {
            gpu_frame_time_ms: 5.0,
            scene_complexity: 250.0,
            ..QualitySignals::default()
        });
        assert!(d.changed);
        assert_eq!(d.reason, "scene complexity over budget");
        assert_eq!(d.adjustment, QualityAdjustment::ReduceSceneComplexity);
    }

    #[test]
    fn ceiling_clamps_initial_tier_and_later_lowered_ceiling() {
        let mut governor = StylizedQualityGovernor::new(test_config(), High, Standard);
        assert_eq!(governor.tier(), Standard);

        governor.set_ceiling(Portable);
        assert_eq!(governor.tier(), Portable);
        let last = governor.history().last().copied().unwrap();
        assert_eq!(last.from, Standard);
        assert_eq!(last.to, Portable);
        assert_eq!(last.reason, "capability ceiling lowered");
    }

    #[test]
    fn history_ring_buffer_keeps_newest_in_order() {
        let mut config = test_config();
        config.history_capacity = 4;
        config.downgrade_cooldown_ticks = 1;
        config.upgrade_cooldown_ticks = 1;
        config.upgrade_qualification_ticks = 1;
        config.ema_alpha = 1.0; // 不平滑：交替信号每轮必产生降/升各一次
        let mut governor = StylizedQualityGovernor::new(config, High, High);
        // 交替超支/余量，制造 6+ 次 transition（容量 4，应丢最老的）。
        let mut first_transition_tick = None;
        let mut last_transition_tick = 0;
        let mut transitions = 0;
        for _ in 0..5 {
            for ms in [20.0, 1.0, 1.0] {
                let d = governor.tick(&signals(ms));
                if d.changed {
                    transitions += 1;
                    if first_transition_tick.is_none() {
                        first_transition_tick = Some(d.tick);
                    }
                    last_transition_tick = d.tick;
                }
            }
        }
        assert!(
            transitions > 4,
            "setup must overflow the ring: {transitions}"
        );
        assert_eq!(governor.history_len(), 4);
        let history: Vec<_> = governor.history().copied().collect();
        // 顺序：最老 → 最新，tick 严格递增；首尾对应保留窗口。
        assert!(history.windows(2).all(|w| w[1].tick > w[0].tick));
        assert_eq!(history[3].tick, last_transition_tick);
        assert!(history[0].tick > first_transition_tick.unwrap());
    }

    #[test]
    fn decisions_are_deterministic_for_same_input_sequence() {
        let inputs: Vec<f32> = lcg_sequence(7, 500)
            .map(|n| 6.0 + (n % 16000) as f32 / 1000.0)
            .collect();
        let run = || {
            let mut governor = StylizedQualityGovernor::new(test_config(), High, High);
            inputs
                .iter()
                .map(|ms| {
                    let d = governor.tick(&signals(*ms));
                    (d.tier, d.changed, d.reason, d.adjustment)
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn every_decision_carries_reason_and_signal_snapshot() {
        let mut governor = StylizedQualityGovernor::new(test_config(), High, High);
        for ms in [20.0, 5.0, 10.0, 30.0, 1.0] {
            let sig = QualitySignals {
                gpu_frame_time_ms: ms,
                resolution: [1920, 1080],
                scene_complexity: 42.0,
                thermal_pressure: Some(0.3),
                battery_level: Some(0.8),
            };
            let d = governor.tick(&sig);
            assert!(!d.reason.is_empty());
            assert_eq!(d.signals, sig);
            assert!(d.smoothed_frame_time_ms.is_finite());
        }
    }
}
