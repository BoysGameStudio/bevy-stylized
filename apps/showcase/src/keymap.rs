//! 交互 V1 键位表（docs/showcase/05 §11.6）：按键状态 → 会话命令的纯翻译。
//!
//! 输入语义（winit KeyCode 采集）归宿主 interactive 模块；本模块不依赖
//! bevy_input，可在 headless 测试构建（无 `interactive` feature）中单测。
//!
//! 过渡期语义：←/→ 在 courtyard 用例循环机位 preset、其他用例仍切 case
//! （05 §10 退役后目录只有 courtyard，其他分支不再触发，保留作通用逻辑）；
//! `,`/`.` 承担全用例导航。§11.6 的 C=Capture 不在本阶段（driver 无
//! ad-hoc observe 通道，暂缓并在此注明）。

use scene_runtime::SessionCommand;
use scenes::metadata::COURTYARD_CAMERA_PRESETS;
use scenes::{CourtyardFlagsPose, CourtyardOutlineMode, CourtyardSettings, CourtyardTimeOfDay};

/// V1 按键集合（winit KeyCode 采集后的适配输入；纯数据）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyPresses {
    /// ←（courtyard：上一机位；其他用例：上一 case）。
    pub prev: bool,
    /// →（courtyard：下一机位；其他用例：下一 case）。
    pub next: bool,
    /// `,`（上一 case；任何用例可用）。
    pub prev_case: bool,
    /// `.`（下一 case；任何用例可用）。
    pub next_case: bool,
    /// ↑（time_of_day 前循环）。
    pub time_next: bool,
    /// ↓（time_of_day 后循环）。
    pub time_prev: bool,
    /// O（outline_mode 循环 off→hull→hybrid）。
    pub outline: bool,
    /// P（post 翻转）。
    pub post: bool,
    /// 1/2/3（crowd = 0/100/1000）。
    pub crowd: Option<u32>,
    /// Space（flags rest/posed 翻转）。
    pub flags: bool,
    /// R（ResetView）。
    pub reset: bool,
}

/// 机位 preset 循环序（表序，panorama 居首）。
fn preset_ids() -> Vec<&'static str> {
    COURTYARD_CAMERA_PRESETS.map(|preset| preset.id).to_vec()
}

fn step_cycle<T: PartialEq>(options: &[T], current: &T, forward: bool) -> usize {
    let index = options
        .iter()
        .position(|option| option == current)
        .unwrap_or(0);
    if forward {
        (index + 1) % options.len()
    } else {
        (index + options.len() - 1) % options.len()
    }
}

/// 按键 + 会话状态 → 命令序列（唯一翻译点；执行归 SessionController）。
///
/// `courtyard_settings` 由宿主从世界资源读取（缺席 = §0 默认），供循环类
/// 参数推导下一值。参数/相机键只对 courtyard 用例翻译——其他用例无参数
/// 面，发出命令只会得到结构化拒绝噪声。
#[must_use]
pub fn translate_keys(
    keys: &KeyPresses,
    current_case: &str,
    current_preset: Option<&str>,
    courtyard_settings: CourtyardSettings,
) -> Vec<SessionCommand> {
    let mut commands = Vec::new();
    let is_courtyard = current_case == "courtyard";
    // ←/→：courtyard 下循环机位 preset（无 preset = 场景默认 panorama，
    // 循环从索引 0 起）；其他用例保持切 case（过渡期语义）。
    if is_courtyard {
        let presets = preset_ids();
        let index = current_preset
            .and_then(|id| presets.iter().position(|preset| *preset == id))
            .unwrap_or(0);
        if keys.next {
            commands.push(SessionCommand::SetCameraPreset(
                presets[(index + 1) % presets.len()].to_string(),
            ));
        }
        if keys.prev {
            commands.push(SessionCommand::SetCameraPreset(
                presets[(index + presets.len() - 1) % presets.len()].to_string(),
            ));
        }
    } else {
        if keys.next {
            commands.push(SessionCommand::NextCase);
        }
        if keys.prev {
            commands.push(SessionCommand::PrevCase);
        }
    }
    // `,`/`.`：任何用例下的 case 导航（过渡期）。
    if keys.next_case {
        commands.push(SessionCommand::NextCase);
    }
    if keys.prev_case {
        commands.push(SessionCommand::PrevCase);
    }
    if !is_courtyard {
        return commands;
    }
    // 参数键（courtyard 参数面，§8）。
    const TIMES: [CourtyardTimeOfDay; 3] = [
        CourtyardTimeOfDay::Day,
        CourtyardTimeOfDay::Dusk,
        CourtyardTimeOfDay::Night,
    ];
    const TIME_NAMES: [&str; 3] = ["day", "dusk", "night"];
    if keys.time_next || keys.time_prev {
        let index = step_cycle(&TIMES, &courtyard_settings.time_of_day, keys.time_next);
        commands.push(SessionCommand::SetParameter {
            key: "time_of_day".to_string(),
            value: serde_json::json!(TIME_NAMES[index]),
        });
    }
    const OUTLINE_MODES: [CourtyardOutlineMode; 3] = [
        CourtyardOutlineMode::Off,
        CourtyardOutlineMode::Hull,
        CourtyardOutlineMode::Hybrid,
    ];
    const OUTLINE_NAMES: [&str; 3] = ["off", "hull", "hybrid"];
    if keys.outline {
        let index = step_cycle(&OUTLINE_MODES, &courtyard_settings.outline_mode, true);
        commands.push(SessionCommand::SetParameter {
            key: "outline_mode".to_string(),
            value: serde_json::json!(OUTLINE_NAMES[index]),
        });
    }
    if keys.post {
        commands.push(SessionCommand::SetParameter {
            key: "post".to_string(),
            value: serde_json::json!(!courtyard_settings.post),
        });
    }
    if let Some(crowd) = keys.crowd {
        commands.push(SessionCommand::SetParameter {
            key: "crowd".to_string(),
            value: serde_json::json!(crowd),
        });
    }
    if keys.flags {
        let next = match courtyard_settings.flags {
            CourtyardFlagsPose::Rest => "posed",
            CourtyardFlagsPose::Posed => "rest",
        };
        commands.push(SessionCommand::SetParameter {
            key: "flags".to_string(),
            value: serde_json::json!(next),
        });
    }
    if keys.reset {
        commands.push(SessionCommand::ResetView);
    }
    commands
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys() -> KeyPresses {
        KeyPresses::default()
    }

    #[test]
    fn arrows_cycle_presets_on_courtyard_and_cases_elsewhere() {
        let presets = preset_ids();
        // courtyard：→ 从默认（None = panorama）进到 center-trio；← 回绕。
        let commands = translate_keys(
            &KeyPresses {
                next: true,
                ..keys()
            },
            "courtyard",
            None,
            CourtyardSettings::default(),
        );
        assert_eq!(
            commands,
            vec![SessionCommand::SetCameraPreset("center-trio".to_string())]
        );
        let commands = translate_keys(
            &KeyPresses {
                prev: true,
                ..keys()
            },
            "courtyard",
            None,
            CourtyardSettings::default(),
        );
        assert_eq!(
            commands,
            vec![SessionCommand::SetCameraPreset(
                presets[presets.len() - 1].to_string()
            )]
        );
        // 其他用例：←/→ 保持切 case（过渡期语义）。
        let commands = translate_keys(
            &KeyPresses {
                next: true,
                prev_case: true,
                ..keys()
            },
            "outline",
            None,
            CourtyardSettings::default(),
        );
        assert_eq!(
            commands,
            vec![SessionCommand::NextCase, SessionCommand::PrevCase]
        );
    }

    #[test]
    fn parameter_keys_only_apply_to_courtyard() {
        // 非 courtyard：参数/相机键不翻译（避免结构化拒绝噪声）。
        let commands = translate_keys(
            &KeyPresses {
                time_next: true,
                outline: true,
                post: true,
                crowd: Some(1000),
                flags: true,
                reset: true,
                ..keys()
            },
            "face",
            None,
            CourtyardSettings::default(),
        );
        assert!(commands.is_empty());
    }

    #[test]
    fn parameter_cycles_follow_current_settings() {
        // 默认状态（day/hull/rest/post off）：各循环的下一值。
        let commands = translate_keys(
            &KeyPresses {
                time_next: true,
                outline: true,
                post: true,
                crowd: Some(0),
                flags: true,
                reset: true,
                ..keys()
            },
            "courtyard",
            None,
            CourtyardSettings::default(),
        );
        assert_eq!(
            commands,
            vec![
                SessionCommand::SetParameter {
                    key: "time_of_day".to_string(),
                    value: serde_json::json!("dusk"),
                },
                SessionCommand::SetParameter {
                    key: "outline_mode".to_string(),
                    value: serde_json::json!("hybrid"),
                },
                SessionCommand::SetParameter {
                    key: "post".to_string(),
                    value: serde_json::json!(true),
                },
                SessionCommand::SetParameter {
                    key: "crowd".to_string(),
                    value: serde_json::json!(0),
                },
                SessionCommand::SetParameter {
                    key: "flags".to_string(),
                    value: serde_json::json!("posed"),
                },
                SessionCommand::ResetView,
            ]
        );
        // 反向循环与翻转回环。
        let commands = translate_keys(
            &KeyPresses {
                time_prev: true,
                ..keys()
            },
            "courtyard",
            None,
            CourtyardSettings::default(),
        );
        assert_eq!(
            commands,
            vec![SessionCommand::SetParameter {
                key: "time_of_day".to_string(),
                value: serde_json::json!("night"),
            }]
        );
        let commands = translate_keys(
            &KeyPresses {
                flags: true,
                ..keys()
            },
            "courtyard",
            None,
            CourtyardSettings {
                flags: CourtyardFlagsPose::Posed,
                ..CourtyardSettings::default()
            },
        );
        assert_eq!(
            commands,
            vec![SessionCommand::SetParameter {
                key: "flags".to_string(),
                value: serde_json::json!("rest"),
            }]
        );
    }
}
