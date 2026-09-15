//! 环境 NPR 数据模型（V2-d，docs/roadmap/01 §Environment NPR；
//! provisional pending ADR-0009 的环境层条款）。
//!
//! 纯 View 层数据：大气调色板（雾色 / zenith 天色）、距离雾参数、
//! 风格化阴影染色。**默认值全部恒等**（雾关、阴影染色强度 0、zenith
//! `None`），保证既有渲染逐像素不变。
//!
//! 范围说明（V2 first cut，诚实记录）：
//!
//! - 完整 stylized IBL / reflection probes 超出本次切片；本模块提供
//!   调色板驱动的近似（fog 色 + zenith 天色 + ambient/key light 既有通道）；
//! - 雾仅线性 start/end（height falloff 需额外数据通道，推迟到 ADR-0009
//!   的正式 view uniform 决策）；
//! - 阴影染色为乘法 tint 作用于 shadow 侧颜色。

use bevy_color::LinearRgba;
use bevy_reflect::Reflect;

/// 距离雾（线性 start/end；View 层）。
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StylizedFog {
    /// 是否启用（false = 完全恒等，渲染层不挂 fog 组件）。
    #[cfg_attr(feature = "serde", serde(default))]
    pub enabled: bool,
    /// 雾色（= horizon 大气色；线性空间，alpha 调制整体强度）。
    #[cfg_attr(feature = "serde", serde(default = "default_fog_color"))]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "crate::schema::LinearColorSchema")
    )]
    pub color: LinearRgba,
    /// 雾起始距离。
    #[cfg_attr(feature = "serde", serde(default = "default_fog_start"))]
    pub start: f32,
    /// 雾终止距离（>= start；非法时 fail-closed 按全透明处理）。
    #[cfg_attr(feature = "serde", serde(default = "default_fog_end"))]
    pub end: f32,
}

#[cfg(feature = "serde")]
fn default_fog_color() -> LinearRgba {
    StylizedFog::default().color
}
#[cfg(feature = "serde")]
fn default_fog_start() -> f32 {
    StylizedFog::default().start
}
#[cfg(feature = "serde")]
fn default_fog_end() -> f32 {
    StylizedFog::default().end
}

impl Default for StylizedFog {
    fn default() -> Self {
        Self {
            enabled: false,
            color: LinearRgba::new(0.62, 0.72, 0.85, 1.0),
            start: 20.0,
            end: 120.0,
        }
    }
}

impl StylizedFog {
    /// 参数合法性（`end > start` 且两者有限非负）。
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.start.is_finite() && self.end.is_finite() && self.start >= 0.0 && self.end > self.start
    }
}

/// 风格化阴影染色（乘法 tint 作用于 shadow 侧颜色）。
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StylizedShadowTint {
    /// 阴影染色（线性空间；冷色偏移是常见风格化选择）。
    #[cfg_attr(feature = "serde", serde(default = "default_shadow_tint_color"))]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "crate::schema::LinearColorSchema")
    )]
    pub color: LinearRgba,
    /// 染色强度 0..=1（0 = 恒等，默认）。
    #[cfg_attr(feature = "serde", serde(default))]
    pub strength: f32,
}

#[cfg(feature = "serde")]
fn default_shadow_tint_color() -> LinearRgba {
    StylizedShadowTint::default().color
}

impl Default for StylizedShadowTint {
    fn default() -> Self {
        Self {
            color: LinearRgba::new(0.55, 0.58, 0.78, 1.0),
            strength: 0.0,
        }
    }
}

/// 环境模型（View 层；附加于
/// [`StylizedViewDescriptor`](crate::StylizedViewDescriptor)）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StylizedEnvironment {
    /// 距离雾 / horizon 大气色。
    #[cfg_attr(feature = "serde", serde(default))]
    pub fog: StylizedFog,
    /// 风格化阴影染色。
    #[cfg_attr(feature = "serde", serde(default))]
    pub shadow_tint: StylizedShadowTint,
    /// Zenith 天色（渲染层映射为相机清屏色 = 平涂天空；渐变天空球
    /// 超出 V2 first cut）。`None` = 不触碰清屏色。
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "Option<crate::schema::LinearColorSchema>")
    )]
    pub zenith_tint: Option<LinearRgba>,
}

impl StylizedEnvironment {
    /// 默认是否恒等（fog 关 + 染色强度 0 + 无 zenith）——渲染层据此
    /// 跳过全部环境组件写入。
    #[must_use]
    pub fn is_identity(&self) -> bool {
        !self.fog.enabled && self.shadow_tint.strength <= 0.0 && self.zenith_tint.is_none()
    }
}

/// 线性雾因子（WGSL `stylized_fog_factor` 的 host 镜像，两边必须一致）：
/// `clamp((distance - start) / (end - start), 0, 1)`；非法参数 fail-closed
/// 返回 0（无雾）。
#[must_use]
pub fn stylized_fog_factor(distance: f32, start: f32, end: f32) -> f32 {
    if !(start.is_finite() && end.is_finite() && end > start) {
        return 0.0;
    }
    ((distance - start) / (end - start)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_identity() {
        let env = StylizedEnvironment::default();
        assert!(env.is_identity());
        assert!(!env.fog.enabled);
        assert_eq!(env.shadow_tint.strength, 0.0);
        assert_eq!(env.zenith_tint, None);
    }

    #[test]
    fn fog_factor_matches_documented_curve() {
        // 边界：start 前 0，end 后 1，中点 0.5。
        assert_eq!(stylized_fog_factor(0.0, 20.0, 120.0), 0.0);
        assert_eq!(stylized_fog_factor(20.0, 20.0, 120.0), 0.0);
        assert_eq!(stylized_fog_factor(70.0, 20.0, 120.0), 0.5);
        assert_eq!(stylized_fog_factor(120.0, 20.0, 120.0), 1.0);
        assert_eq!(stylized_fog_factor(1000.0, 20.0, 120.0), 1.0);
        // 非法参数 fail-closed（无雾）。
        assert_eq!(stylized_fog_factor(50.0, 120.0, 20.0), 0.0);
        assert_eq!(stylized_fog_factor(50.0, 20.0, 20.0), 0.0);
        assert_eq!(stylized_fog_factor(50.0, f32::NAN, 20.0), 0.0);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn environment_serde_roundtrip_and_additive_compat() {
        let env = StylizedEnvironment {
            fog: StylizedFog {
                enabled: true,
                ..StylizedFog::default()
            },
            shadow_tint: StylizedShadowTint {
                strength: 0.4,
                ..StylizedShadowTint::default()
            },
            zenith_tint: Some(LinearRgba::new(0.1, 0.2, 0.5, 1.0)),
        };
        let json = serde_json::to_string(&env).expect("serialize");
        let back: StylizedEnvironment = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(env, back);

        // additive：空 payload 走默认（v1.0 的 view payload 没有
        // environment 字段，读取方必须能缺省解析）。
        let empty: StylizedEnvironment = serde_json::from_str("{}").expect("defaults");
        assert!(empty.is_identity());
    }
}
