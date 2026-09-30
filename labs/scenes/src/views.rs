//! 内容级视图 preset（无 Bevy App 依赖；只操作产品纯数据
//! `StylizedViewDescriptor`）。输出绑定（把 preset 应用到哪台相机）
//! 归宿主。

use bevy_color::LinearRgba;
use bevy_stylized_core::{
    StylizedEnvironment, StylizedFog, StylizedShadowTint, StylizedViewDescriptor,
};

// ---------------------------------------------------------------------------
// courtyard 时刻 preset（docs/showcase/05 §5）。
// ---------------------------------------------------------------------------

/// courtyard 昼：浅蓝灰雾 28–110 + 淡蓝天色 + 轻阴影染色（色板同旧
/// environment_day；雾范围按庭院景深重调——展台环/中央对照组不受雾
/// 染色，远景渐没；旧 environment_* preset 随旧场景退役删除）。
pub fn courtyard_day(view: &mut StylizedViewDescriptor) {
    view.environment = StylizedEnvironment {
        fog: StylizedFog {
            enabled: true,
            color: LinearRgba::new(0.58, 0.68, 0.84, 1.0),
            start: 28.0,
            end: 110.0,
        },
        shadow_tint: StylizedShadowTint {
            color: LinearRgba::new(0.6, 0.68, 0.95, 1.0),
            strength: 0.5,
        },
        zenith_tint: Some(LinearRgba::new(0.45, 0.6, 0.85, 1.0)),
    };
}

/// courtyard 黄昏：暖橙雾 14–70 + 紫红天色 + 紫色阴影染色 + 暖 key light。
pub fn courtyard_dusk(view: &mut StylizedViewDescriptor) {
    view.key_light_color = LinearRgba::new(1.0, 0.75, 0.55, 1.0);
    view.environment = StylizedEnvironment {
        fog: StylizedFog {
            enabled: true,
            color: LinearRgba::new(0.85, 0.55, 0.38, 1.0),
            start: 14.0,
            end: 70.0,
        },
        shadow_tint: StylizedShadowTint {
            color: LinearRgba::new(0.55, 0.4, 0.7, 1.0),
            strength: 0.5,
        },
        zenith_tint: Some(LinearRgba::new(0.4, 0.25, 0.45, 1.0)),
    };
}

/// courtyard 夜：深蓝雾 10–48（end 必须 > 灯阵纵深 ~27m；LOD very_far
/// d≈60 没入雾中属预期，05 §5 已声明）+ 深蓝天色 + 强冷阴影染色 +
/// 夜间 ambient。
pub fn courtyard_night(view: &mut StylizedViewDescriptor) {
    view.day_night_factor = 0.0;
    // 夜景 key light 调暗偏冷（2026-09-30 owner 拍板修正夜景曝光）：夜间
    // ambient 有夜间档而 key 此前仍全白——地面读到 ~83% 日间亮度（偏亮）。
    // 调暗后地面落 ~46% 日间亮度，灯阵成为画面主光源。
    view.key_light_color = LinearRgba::new(0.22, 0.26, 0.42, 1.0);
    view.environment = StylizedEnvironment {
        fog: StylizedFog {
            enabled: true,
            color: LinearRgba::new(0.08, 0.1, 0.2, 1.0),
            start: 10.0,
            end: 48.0,
        },
        shadow_tint: StylizedShadowTint {
            color: LinearRgba::new(0.45, 0.5, 0.85, 1.0),
            strength: 0.7,
        },
        zenith_tint: Some(LinearRgba::new(0.03, 0.05, 0.12, 1.0)),
    };
}
