//! 导览模式（docs/showcase/06 §8，U3）：脚本化巡场。脚本是**数据**
//! （分区序 + 驻留时长），推进是纯函数；驱动源归宿主 desktop_tick——
//! 每条驻留命令照常走 SessionCommand 通道（经宿主 journal 录制），
//! 任意导航输入退出导览（宿主判定，本模块不感知输入）。
//!
//! 与键位/手势翻译层同构：不依赖 bevy_input，合成 dt 单测。

use scene_runtime::SessionCommand;

/// 一站：分区 + 驻留秒数。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TourStop {
    /// 分区 id（zone 参数值）。
    pub zone: &'static str,
    /// 驻留秒数（构建落定后计时；宿主在 driver Done 时才推进）。
    pub dwell_secs: f32,
}

/// 默认庭院巡场脚本（06 §8；v3：内容档 ×4s，收尾回 trio home 位——
/// overview 随 v3 退役；v4 增 5 个 cel 特性档）。
pub const COURTYARD_TOUR: &[TourStop] = &[
    TourStop { zone: "p1", dwell_secs: 4.0 },
    TourStop { zone: "p2", dwell_secs: 4.0 },
    TourStop { zone: "p3", dwell_secs: 4.0 },
    TourStop { zone: "p4", dwell_secs: 4.0 },
    TourStop { zone: "p5", dwell_secs: 4.0 },
    TourStop { zone: "lod", dwell_secs: 4.0 },
    TourStop { zone: "stage", dwell_secs: 4.0 },
    TourStop { zone: "lamps", dwell_secs: 4.0 },
    TourStop { zone: "rim", dwell_secs: 4.0 },
    TourStop { zone: "shadow", dwell_secs: 4.0 },
    TourStop { zone: "palette", dwell_secs: 4.0 },
    TourStop { zone: "metal", dwell_secs: 4.0 },
    TourStop { zone: "cutout", dwell_secs: 4.0 },
    TourStop { zone: "trio", dwell_secs: 4.0 },
];

/// 导览进度（宿主持有；`index` = 当前驻留站在脚本中的下标）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TourProgress {
    /// 当前站下标。
    pub index: usize,
    /// 当前站剩余驻留秒数。
    pub dwell_left: f32,
}

/// 推进结果。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TourStep {
    /// 驻留到期：聚焦下一站（命令照常走 SessionCommand 通道）。
    Focus(&'static str),
    /// 脚本走完（overview 收尾停留结束）。
    Finished,
}

/// 分区聚焦命令（与 ←/→ 同一变体）。
pub fn zone_command(zone: &str) -> SessionCommand {
    SessionCommand::SetParameter {
        key: "zone".to_string(),
        value: serde_json::json!(zone),
    }
}

/// 开始导览：返回进度与首站命令。
pub fn tour_start(script: &[TourStop]) -> Option<(TourProgress, SessionCommand)> {
    let first = script.first()?;
    Some((
        TourProgress {
            index: 0,
            dwell_left: first.dwell_secs,
        },
        zone_command(first.zone),
    ))
}

/// 推进导览（宿主在 driver 落定时逐帧调用）：驻留计时到期 → 下一站
/// 命令或结束；未到期 → None。
pub fn tour_advance(
    progress: &mut TourProgress,
    script: &[TourStop],
    dt: f32,
) -> Option<TourStep> {
    progress.dwell_left -= dt;
    if progress.dwell_left > 0.0 {
        return None;
    }
    progress.index += 1;
    match script.get(progress.index) {
        Some(stop) => {
            progress.dwell_left = stop.dwell_secs;
            Some(TourStep::Focus(stop.zone))
        }
        None => Some(TourStep::Finished),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tour_walks_script_and_finishes_back_home() {
        let script = COURTYARD_TOUR;
        assert_eq!(script.len(), 14);
        assert_eq!(script.last().expect("last stop").zone, "trio");
        let (mut progress, command) = tour_start(script).expect("non-empty script");
        assert_eq!(command, zone_command("p1"), "tour starts at p1");
        assert_eq!(progress.dwell_left, 4.0);
        // 驻留中不动作。
        assert_eq!(tour_advance(&mut progress, script, 2.0), None);
        // 到期 → p2。
        assert_eq!(
            tour_advance(&mut progress, script, 2.5),
            Some(TourStep::Focus("p2"))
        );
        assert_eq!(progress.dwell_left, 4.0);
        // 快进到脚本末尾：末站 trio（home 位）到期后 Finished。
        for _ in 0..12 {
            assert!(matches!(
                tour_advance(&mut progress, script, 10.0),
                Some(TourStep::Focus(_))
            ));
        }
        assert_eq!(progress.index, 13, "now dwelling on trio (home)");
        assert_eq!(
            tour_advance(&mut progress, script, 5.0),
            Some(TourStep::Finished)
        );
    }

    #[test]
    fn empty_script_cannot_start() {
        assert_eq!(tour_start(&[]), None);
    }
}
