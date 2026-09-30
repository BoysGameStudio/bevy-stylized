//! 交互 V1 键位表（docs/showcase/05 §11.6）：按键状态 → 会话命令的纯翻译。
//!
//! 输入语义（winit KeyCode 采集）归宿主 interactive 模块；本模块不依赖
//! bevy_input，可在 headless 测试构建（无 `interactive` feature）中单测。
//!
//! v3 语义（05 §0/§11.6）：←/→ 在 courtyard 用例循环**内容档**（zone
//! 参数；机位随 builder 配方联动，不再单独发 SetCameraPreset）；overview/
//! Tab 随 v3 退役（展亭模式同位必穿模；浏览全部内容归导览模式 G）。
//! 其他用例 ←/→ 仍切 case（05 §10 退役后目录只有
//! courtyard，其他分支不再触发，保留作通用逻辑）；`,`/`.` 承担全用例
//! 导航。§11.6 的 C=Capture 自 U3 落地（06 §9：driver ad-hoc observe
//! 入口，[`SessionCommand::Capture`]；浏览快照不参与 golden 判定）。
//! N = stage_character fixture/odette 翻转（2026-09-29
//! 角色台展品参数，odette 档经宿主注册的 odette:// 资产源加载真 Genshin
//! 角色；golden/证据内容恒为 fixture 档，本键不影响判定）。

use scene_runtime::SessionCommand;
use scenes::{CourtyardFlagsPose, CourtyardOutlineMode, CourtyardSettings, CourtyardTimeOfDay, CourtyardZone};

/// V1 按键集合（winit KeyCode 采集后的适配输入；纯数据）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyPresses {
    /// ←（courtyard：上一内容档；其他用例：上一 case）。
    pub prev: bool,
    /// →（courtyard：下一内容档；其他用例：下一 case）。
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
    /// N（stage_character fixture/odette 翻转；odette 档需要宿主注册的
    /// odette:// 资产源，缺席时加载失败并在 stderr 可见——不做空台降级）。
    pub character: bool,
    /// R（ResetView）。
    pub reset: bool,
    /// C（Capture：交互捕获，driver ad-hoc observe；06 §9）。
    pub capture: bool,
}

/// 内容循环序（§11.6：trio → p1 → … → cutout；v3 起 overview 退役，
/// v4 增 5 个 cel 特性档）。
const ZONE_CYCLE: [CourtyardZone; 14] = [
    CourtyardZone::Trio,
    CourtyardZone::P1,
    CourtyardZone::P2,
    CourtyardZone::P3,
    CourtyardZone::P4,
    CourtyardZone::P5,
    CourtyardZone::Lod,
    CourtyardZone::Stage,
    CourtyardZone::Lamps,
    CourtyardZone::Rim,
    CourtyardZone::Shadow,
    CourtyardZone::Palette,
    CourtyardZone::Metal,
    CourtyardZone::Cutout,
];

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
///
#[must_use]
pub fn translate_keys(
    keys: &KeyPresses,
    current_case: &str,
    current_preset: Option<&str>,
    courtyard_settings: CourtyardSettings,
) -> Vec<SessionCommand> {
    let _ = current_preset; // v2：机位随分区联动，循环不再读取当前 preset。
    let mut commands = Vec::new();
    let is_courtyard = current_case == "courtyard";
    // ←/→：courtyard 下循环分区（zone 参数；机位随 builder 配方联动）；
    // 其他用例保持切 case（过渡期语义）。
    if is_courtyard {
        let current = courtyard_settings.zone;
        if keys.next || keys.prev {
            let index = step_cycle(&ZONE_CYCLE, &current, keys.next);
            commands.push(SessionCommand::SetParameter {
                key: "zone".to_string(),
                value: serde_json::json!(ZONE_CYCLE[index].id()),
            });
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
    if keys.character {
        let next = match courtyard_settings.stage_character {
            scenes::CourtyardStageCharacter::Fixture => "odette",
            scenes::CourtyardStageCharacter::Odette => "fixture",
        };
        commands.push(SessionCommand::SetParameter {
            key: "stage_character".to_string(),
            value: serde_json::json!(next),
        });
    }
    if keys.reset {
        commands.push(SessionCommand::ResetView);
    }
    if keys.capture {
        commands.push(SessionCommand::Capture);
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
    fn arrows_cycle_zones_on_courtyard_and_cases_elsewhere() {
        // courtyard：→ 从默认（trio）进到 p1；← 回绕到 cutout（v4 循环序
        // 末位）；机位不再单独发命令（随 builder 配方联动）。
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
            vec![SessionCommand::SetParameter {
                key: "zone".to_string(),
                value: serde_json::json!("p1"),
            }]
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
            vec![SessionCommand::SetParameter {
                key: "zone".to_string(),
                value: serde_json::json!("cutout"),
            }]
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
                character: true,
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
    fn capture_key_emits_capture_command() {
        // C → Capture（06 §9）；非 courtyard 不翻译（与 reset 同一门禁）。
        let commands = translate_keys(
            &KeyPresses {
                capture: true,
                ..keys()
            },
            "courtyard",
            None,
            CourtyardSettings::default(),
        );
        assert_eq!(commands, vec![SessionCommand::Capture]);
        let commands = translate_keys(
            &KeyPresses {
                capture: true,
                ..keys()
            },
            "outline",
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
                character: true,
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
                SessionCommand::SetParameter {
                    key: "stage_character".to_string(),
                    value: serde_json::json!("odette"),
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
