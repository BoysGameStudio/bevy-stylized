//! 鼠标手势翻译层（docs/showcase/06 §5，U1）：winit 鼠标手势 → 会话命令的
//! 纯翻译，与 [`crate::keymap`] 同构——不依赖 bevy_input，可在 headless
//! 测试构建（无 `interactive` feature）中用合成输入单测。
//!
//! 手感常量与 Inspector（inspector/panel.rs）一致：拖拽 orbit 0.006 rad/px、
//! 滚轮指数缩放 0.003/单位、pan 按 distance/viewport 比例——跨窗口零学习
//! 成本。本层只输出相机命令（v3：场景内点击聚焦随展亭模式退役——同一
//! 位置无空间导航）。
//!
//! 确定性：翻译是纯函数；双击判定由 [`ClickTracker`] 以调用方提供的时间戳
//! 驱动，合成时间戳可复现。

use scene_runtime::SessionCommand;

/// 拖拽 orbit 灵敏度（弧度/px，Inspector 同值）。
pub const ORBIT_RADIANS_PER_PX: f32 = 0.006;
/// 滚轮缩放系数（指数底，Inspector 同值）。
pub const ZOOM_FACTOR_PER_SCROLL: f32 = 0.003;
/// 距离下限（米，Inspector clamp 同值；控制器硬限 0.001 之上）。
pub const MIN_DISTANCE: f32 = 0.01;
/// 距离上限（米，Inspector clamp 同值）。
pub const MAX_DISTANCE: f32 = 10000.0;
/// 俯仰软限（弧度，Inspector clamp 同值；控制器硬限 ±1.5 之下）。
pub const MAX_PITCH: f32 = 1.45;
/// 点击/拖拽判定位移阈值（px）：按下→抬起位移在此之内 = 点击。
pub const CLICK_MAX_DISTANCE_PX: f32 = 4.0;
/// 双击判定窗口（秒）。
pub const DOUBLE_CLICK_SECS: f32 = 0.3;

/// 一帧内采集到的鼠标手势（纯数据；采集归宿主）。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MouseGestures {
    /// 主键拖拽位移（px，已超点击阈值）。
    pub orbit_delta: Option<(f32, f32)>,
    /// 次键拖拽位移（px）。
    pub pan_delta: Option<(f32, f32)>,
    /// 滚轮 y 累计（行/格，向上为正）。
    pub scroll: f32,
    /// 双击（主键，由 [`ClickTracker`] 判定）。
    pub double_click: bool,
}

/// 手势 + 当前相机状态 → 命令序列（唯一翻译点；执行归 SessionController）。
///
/// 俯仰/距离在翻译层预夹（软限之内），避免控制器逐帧拒绝噪声。`viewport`
/// 是渲染区高度（px），pan 按 distance/viewport 折算（Inspector 同式）。
/// 相机命令仅对 courtyard 用例翻译（与键位表的相机键同一门禁）。
#[must_use]
pub fn translate_mouse(
    gestures: &MouseGestures,
    is_courtyard: bool,
    current_pitch: f32,
    current_distance: f32,
    viewport: f32,
) -> Vec<SessionCommand> {
    let mut commands = Vec::new();
    if !is_courtyard {
        return commands;
    }
    if let Some((dx, dy)) = gestures.orbit_delta {
        let yaw = -dx * ORBIT_RADIANS_PER_PX;
        let pitch = (current_pitch + dy * ORBIT_RADIANS_PER_PX)
            .clamp(-MAX_PITCH, MAX_PITCH)
            - current_pitch;
        if yaw != 0.0 || pitch != 0.0 {
            commands.push(SessionCommand::CameraOrbit { yaw, pitch });
        }
    }
    if let Some((dx, dy)) = gestures.pan_delta {
        let scale = current_distance / viewport.max(1.0);
        commands.push(SessionCommand::CameraPan {
            dx: -dx * scale,
            dy: dy * scale,
        });
    }
    if gestures.scroll != 0.0 {
        let factor = (-gestures.scroll * ZOOM_FACTOR_PER_SCROLL).exp();
        let clamped = (current_distance / factor).clamp(MIN_DISTANCE, MAX_DISTANCE);
        let factor = current_distance / clamped;
        if factor != 1.0 {
            commands.push(SessionCommand::CameraZoom { factor });
        }
    }
    if gestures.double_click {
        commands.push(SessionCommand::ResetView);
    }
    commands
}

/// 双击判定（纯数据 + 调用方时间戳；位移窗口复用点击阈值）。
#[derive(Clone, Copy, Debug, Default)]
pub struct ClickTracker {
    last: Option<(f32, f32, f32)>,
}

impl ClickTracker {
    /// 记录一次点击（按下→抬起位移已在阈值内），返回是否构成双击。
    pub fn click(&mut self, now: f32, x: f32, y: f32) -> bool {
        let double = self.last.is_some_and(|(t, lx, ly)| {
            now - t <= DOUBLE_CLICK_SECS
                && (x - lx).abs() <= CLICK_MAX_DISTANCE_PX
                && (y - ly).abs() <= CLICK_MAX_DISTANCE_PX
        });
        // 双击后重置：三连击是「双击 + 单击」，不是两次双击。
        self.last = if double { None } else { Some((now, x, y)) };
        double
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gestures() -> MouseGestures {
        MouseGestures::default()
    }

    #[test]
    fn orbit_drag_maps_with_inspector_sensitivity_and_sign() {
        // 右拖 100px = yaw −0.6 rad（Inspector：yaw -= dx * 0.006）。
        let commands = translate_mouse(
            &MouseGestures {
                orbit_delta: Some((100.0, 0.0)),
                ..gestures()
            },
            true,
            0.0,
            10.0,
            720.0,
        );
        assert_eq!(
            commands,
            vec![SessionCommand::CameraOrbit {
                yaw: -0.6,
                pitch: 0.0
            }]
        );
    }

    #[test]
    fn orbit_pitch_preclamped_inside_soft_limit() {
        // 当前 pitch 1.4，上拖 100px（+0.6）应夹到软限 1.45 → 增量 ≈0.05，
        // 不触发控制器 ±1.5 硬限拒绝。
        let commands = translate_mouse(
            &MouseGestures {
                orbit_delta: Some((0.0, 100.0)),
                ..gestures()
            },
            true,
            1.4,
            10.0,
            720.0,
        );
        let [SessionCommand::CameraOrbit { yaw, pitch }] = commands.as_slice() else {
            panic!("expected one orbit command");
        };
        assert_eq!(*yaw, -0.0);
        assert!(
            (*pitch - 0.05).abs() < 1e-6,
            "pitch delta clamped to soft limit: {pitch}"
        );
    }

    #[test]
    fn pan_scales_with_distance_over_viewport() {
        // Inspector 同式：shift = (−right·dx + up·dy) · distance/size。
        let commands = translate_mouse(
            &MouseGestures {
                pan_delta: Some((36.0, -72.0)),
                ..gestures()
            },
            true,
            0.0,
            10.0,
            720.0,
        );
        assert_eq!(
            commands,
            vec![SessionCommand::CameraPan {
                dx: -0.5,
                dy: -1.0
            }]
        );
    }

    #[test]
    fn zoom_is_exponential_and_distance_clamped() {
        // 滚轮 +1：factor = exp(−0.003) < 1 → 拉远（距离除以 factor）。
        let commands = translate_mouse(
            &MouseGestures {
                scroll: 1.0,
                ..gestures()
            },
            true,
            0.0,
            10.0,
            720.0,
        );
        let [SessionCommand::CameraZoom { factor }] = commands.as_slice() else {
            panic!("expected one zoom command");
        };
        assert!((factor - (-0.003f32).exp()).abs() < 1e-6);
        // 已在上限：滚轮拉远被夹，无命令。
        let commands = translate_mouse(
            &MouseGestures {
                scroll: 1.0,
                ..gestures()
            },
            true,
            0.0,
            MAX_DISTANCE,
            720.0,
        );
        assert!(commands.is_empty());
    }

    #[test]
    fn double_click_resets_view() {
        let commands = translate_mouse(
            &MouseGestures {
                double_click: true,
                ..gestures()
            },
            true,
            0.0,
            10.0,
            720.0,
        );
        assert_eq!(commands, vec![SessionCommand::ResetView]);
    }

    #[test]
    fn gestures_only_apply_to_courtyard() {
        let commands = translate_mouse(
            &MouseGestures {
                orbit_delta: Some((10.0, 10.0)),
                pan_delta: Some((10.0, 10.0)),
                scroll: 1.0,
                double_click: true,
            },
            false,
            0.0,
            10.0,
            720.0,
        );
        assert!(commands.is_empty());
    }

    #[test]
    fn click_tracker_double_click_window() {
        let mut tracker = ClickTracker::default();
        assert!(!tracker.click(1.00, 100.0, 100.0));
        // 窗口内 + 位移内 → 双击。
        assert!(tracker.click(1.20, 102.0, 100.0));
        // 双击后重置：紧随的第三次点击是新的单击。
        assert!(!tracker.click(1.30, 100.0, 100.0));
        // 超时（> 0.3s）→ 单击。
        assert!(!tracker.click(5.00, 100.0, 100.0));
        // 窗口内 → 双击。
        assert!(tracker.click(5.10, 100.0, 100.0));
        // 位移超阈值 → 单击（双击已重置，且 120−100 > 4px）。
        assert!(!tracker.click(5.20, 120.0, 100.0));
    }
}
