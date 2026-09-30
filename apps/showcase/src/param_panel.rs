//! 参数抽屉的纯数据部分（docs/showcase/06 §6，U2）：控件选择规则与
//! 当前值映射（可单测）；egui 渲染归宿主 overlay。控件元数据单一源 =
//! catalog 参数 schema（`ParameterSpec.kind` + `ui` 分组/预设）。
//!
//! 与 keymap/mousemap 同构：不依赖 bevy_input/egui，headless 可测。

use scenario_model::catalog::ParameterKind;
use scenes::{CourtyardFlagsPose, CourtyardOutlineMode, CourtyardSettings, CourtyardTimeOfDay};

/// 控件选型（06 §6 表）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetKind {
    /// Enum 且取值 ≤4：分段按钮。
    Segmented,
    /// Enum 且取值 >4：下拉。
    Combo,
    /// U32：步进滑杆（+ catalog `ui.presets` 快捷芯片）。
    StepSlider,
    /// Bool：开关。
    Toggle,
    /// F32：滑杆（范围取自 schema）。
    Slider,
    /// OptionalF32：滑杆 + 「自动」勾选（null）。
    OptionalSlider,
}

/// 参数类型 → 控件（06 §6 表的唯一翻译点）。
#[must_use]
pub fn widget_kind(kind: &ParameterKind) -> WidgetKind {
    match kind {
        ParameterKind::Enum { options } if options.len() <= 4 => WidgetKind::Segmented,
        ParameterKind::Enum { .. } => WidgetKind::Combo,
        ParameterKind::U32 { .. } => WidgetKind::StepSlider,
        ParameterKind::Bool => WidgetKind::Toggle,
        ParameterKind::F32 { .. } => WidgetKind::Slider,
        ParameterKind::OptionalF32 => WidgetKind::OptionalSlider,
    }
}

/// 当前参数值（CourtyardSettings → JSON，与控制器 apply_parameter 的
/// 取值表镜像；面板控件初值数据源）。
#[must_use]
pub fn courtyard_value(settings: &CourtyardSettings, key: &str) -> serde_json::Value {
    match key {
        "zone" => serde_json::json!(settings.zone.id()),
        "time_of_day" => serde_json::json!(match settings.time_of_day {
            CourtyardTimeOfDay::Day => "day",
            CourtyardTimeOfDay::Dusk => "dusk",
            CourtyardTimeOfDay::Night => "night",
        }),
        "outline_mode" => serde_json::json!(match settings.outline_mode {
            CourtyardOutlineMode::Off => "off",
            CourtyardOutlineMode::Hull => "hull",
            CourtyardOutlineMode::Hybrid => "hybrid",
        }),
        "crowd" => serde_json::json!(settings.crowd),
        "post" => serde_json::json!(settings.post),
        "flags" => serde_json::json!(match settings.flags {
            CourtyardFlagsPose::Rest => "rest",
            CourtyardFlagsPose::Posed => "posed",
        }),
        "stage_character" => serde_json::json!(settings.stage_character.id()),
        // trio.face.{i}.face_threshold：会话级覆盖（None = 自动 = 三阈值
        // 梯度），三个落点共享同一覆盖值（apply_parameter 同一语义）。
        _ if key.starts_with("trio.face.") => match settings.face_threshold {
            Some(value) => serde_json::json!(value),
            None => serde_json::Value::Null,
        },
        _ => serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_selection_follows_design_table() {
        // 06 §6：Enum ≤4 分段 / >4 下拉；U32 步进滑杆；Bool 开关；
        // OptionalF32 滑杆+自动；F32 滑杆。
        assert_eq!(
            widget_kind(&ParameterKind::Enum {
                options: vec!["a".into(), "b".into(), "c".into()]
            }),
            WidgetKind::Segmented
        );
        assert_eq!(
            widget_kind(&ParameterKind::Enum {
                options: (0..10).map(|i| format!("o{i}")).collect()
            }),
            WidgetKind::Combo
        );
        assert_eq!(
            widget_kind(&ParameterKind::U32 { max: 1000 }),
            WidgetKind::StepSlider
        );
        assert_eq!(widget_kind(&ParameterKind::Bool), WidgetKind::Toggle);
        assert_eq!(
            widget_kind(&ParameterKind::F32 {
                min: 0.0,
                max: 1.0
            }),
            WidgetKind::Slider
        );
        assert_eq!(
            widget_kind(&ParameterKind::OptionalF32),
            WidgetKind::OptionalSlider
        );
    }

    #[test]
    fn courtyard_value_mirrors_controller_parameter_table() {
        let settings = CourtyardSettings::default();
        // §0 默认状态：trio/day/hull/100/false/rest/None/fixture。
        assert_eq!(courtyard_value(&settings, "zone"), serde_json::json!("trio"));
        assert_eq!(
            courtyard_value(&settings, "time_of_day"),
            serde_json::json!("day")
        );
        assert_eq!(
            courtyard_value(&settings, "outline_mode"),
            serde_json::json!("hull")
        );
        assert_eq!(courtyard_value(&settings, "crowd"), serde_json::json!(100));
        assert_eq!(courtyard_value(&settings, "post"), serde_json::json!(false));
        assert_eq!(courtyard_value(&settings, "flags"), serde_json::json!("rest"));
        assert_eq!(
            courtyard_value(&settings, "stage_character"),
            serde_json::json!("fixture")
        );
        assert_eq!(
            courtyard_value(&settings, "trio.face.0.face_threshold"),
            serde_json::Value::Null
        );
        // 覆盖值在场时三个落点同读。
        let settings = CourtyardSettings {
            face_threshold: Some(0.5),
            ..settings
        };
        assert_eq!(
            courtyard_value(&settings, "trio.face.2.face_threshold"),
            serde_json::json!(0.5)
        );
        // 未知键 → Null（控件侧不生成）。
        assert_eq!(courtyard_value(&settings, "nope"), serde_json::Value::Null);
    }
}
