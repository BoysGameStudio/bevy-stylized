//! View 层数据路径（docs/rendering/02）。
//!
//! 公开语义挂在 core 的 [`StylizedViewDescriptor`] 上；本模块只做薄同步：
//! 把 key light / palette / day-night / exposure 写进 Bevy 自身的
//! view-layer 对象（`DirectionalLight` 实体 + 相机 `Exposure` /
//! `AmbientLight` 组件），shader 经 `mesh_view_bindings` 只读消费。
//! 这对应 docs/rendering/03 的 `NprLightingModel::Physical`（Bevy Lights →
//! NPR remap），是 Generic Cel 的默认 lighting backend。
//!
//! **禁止**反向路径：View 数据不得写入任何材质资产（per-frame material
//! mutation 禁令）。
//!
//! ## 语义映射（provisional，pending ADR-0009）
//!
//! - `key_light_direction` / `key_light_color` → 框架托管的
//!   `DirectionalLight`（[`StylizedKeyLight`]）；
//! - `exposure` → 每台 `Camera3d` 的 `Exposure`（`view.exposure`）；
//! - `palette[0]`（若存在）→ 环境光颜色（global palette tint 经 ambient
//!   通道进入 shader，不写材质）；
//! - `day_night_factor`（0 = 夜，1 = 昼）→ 在昼/夜环境亮度与色温之间
//!   lerp **环境光**（不改 ramp 行——ramp 行选择属于 Instance/Profile
//!   语义；该选择是本迭代的显式决策，见函数文档）。

use bevy::{
    camera::Exposure,
    color::LinearRgba,
    light::{AmbientLight, DirectionalLight},
    prelude::{Camera3d, Commands, Component, Entity, Query, Res, Resource, Transform, Vec3, With},
};
use bevy_stylized_core::StylizedViewDescriptor;

/// View 层 NPR 状态（资源形式，帧级）。
///
/// 写入即生效：[`sync_stylized_view`] 每帧把内容同步到 Bevy 的 key light
/// 实体与所有 `Camera3d` 的 `Exposure` / `AmbientLight`。
#[derive(Resource, Clone, Debug, Default)]
pub struct StylizedViewSettings(pub StylizedViewDescriptor);

/// 框架托管的 key light 实体标记（由 [`sync_stylized_view`] 生成/更新）。
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct StylizedKeyLight;

/// Bevy `Exposure` 的校准常数（`exposure() = exp2(-ev100) / 1.2`）。
const EXPOSURE_CALIBRATION: f32 = 1.2;

/// 把期望的曝光乘子转换为 Bevy `Exposure`（`view.exposure` 将等于该乘子）。
#[must_use]
pub fn exposure_component(multiplier: f32) -> Exposure {
    let multiplier = if multiplier.is_finite() && multiplier > 0.0 {
        multiplier
    } else {
        1.0
    };
    Exposure {
        ev100: -(EXPOSURE_CALIBRATION * multiplier).log2(),
    }
}

/// 夜间环境光（冷蓝低亮度；provisional 常数，pending ADR-0009）。
const NIGHT_AMBIENT_COLOR: LinearRgba = LinearRgba::new(0.09, 0.11, 0.19, 1.0);
/// 昼间环境亮度基准。
const DAY_AMBIENT_BRIGHTNESS: f32 = 0.35;
/// 夜间环境亮度基准。
const NIGHT_AMBIENT_BRIGHTNESS: f32 = 0.06;

/// Day/Night 解析（provisional，pending ADR-0009）：
///
/// **选择：lerp 环境光亮度与色温，不改 ramp 行**。ramp 行是
/// Instance/Profile 语义（ramp atlas 的行属于材质/profile 索引），View 层
/// 的昼夜变化只调制全局照明输入（ambient + key light tint），保持
/// "同一份 authoring data" 语义不变。
///
/// `palette[0]` 存在时作为昼间环境色；夜间环境色固定为
/// [`NIGHT_AMBIENT_COLOR`]（冷蓝）。
#[must_use]
pub fn resolve_ambient_light(descriptor: &StylizedViewDescriptor) -> AmbientLight {
    let t = descriptor.day_night_factor.clamp(0.0, 1.0);
    let day_color = descriptor
        .palette
        .first()
        .copied()
        .unwrap_or(LinearRgba::WHITE);
    let lerp = |a: f32, b: f32| a + (b - a) * t;
    let color = LinearRgba::new(
        lerp(NIGHT_AMBIENT_COLOR.red, day_color.red),
        lerp(NIGHT_AMBIENT_COLOR.green, day_color.green),
        lerp(NIGHT_AMBIENT_COLOR.blue, day_color.blue),
        1.0,
    );
    AmbientLight {
        color: bevy::prelude::Color::LinearRgba(color),
        brightness: lerp(NIGHT_AMBIENT_BRIGHTNESS, DAY_AMBIENT_BRIGHTNESS),
        ..AmbientLight::default()
    }
}

/// 每帧同步 View 层数据。
///
/// 只写 Bevy 的 view-layer 对象（key light 实体、相机组件），不触碰任何
/// 材质资产。
pub fn sync_stylized_view(
    mut commands: Commands,
    settings: Option<Res<StylizedViewSettings>>,
    mut key_lights: Query<(&mut Transform, &mut DirectionalLight), With<StylizedKeyLight>>,
    mut cameras: Query<(Entity, Option<&mut Exposure>, Option<&mut AmbientLight>), With<Camera3d>>,
) {
    let descriptor = settings.map(|s| s.0.clone()).unwrap_or_default();

    // Key light：方向（transform）+ 颜色/强度（premultiplied by illuminance，
    // 此处固定 illuminance = 1，颜色直接取 descriptor 的线性色）。
    let direction = {
        let d = descriptor.key_light_direction;
        if d.length_squared() > 1e-6 {
            d.normalize()
        } else {
            Vec3::Y
        }
    };
    // DirectionalLight 沿 transform 的 -Z 照射；`looking_to(-dir)` 使
    // direction_to_light == descriptor.key_light_direction。
    let light_transform = Transform::default().looking_to(-direction, Vec3::Y);
    let light = DirectionalLight {
        color: bevy::prelude::Color::LinearRgba(descriptor.key_light_color),
        // 固定 1.0：NPR 语义光强由颜色本身携带（GPU 侧 color 会乘 illuminance）。
        illuminance: 1.0,
        shadow_maps_enabled: false,
        ..DirectionalLight::default()
    };
    if key_lights.is_empty() {
        commands.spawn((StylizedKeyLight, light, light_transform));
    } else {
        for (mut transform, mut current) in &mut key_lights {
            *transform = light_transform;
            *current = light;
        }
    }

    // Exposure + Ambient（palette / day-night 经 ambient 通道进入 shader）。
    let exposure = exposure_component(descriptor.exposure);
    let ambient = resolve_ambient_light(&descriptor);
    for (entity, current_exposure, current_ambient) in &mut cameras {
        match current_exposure {
            Some(mut current) => *current = exposure,
            None => {
                if let Ok(mut entity_commands) = commands.get_entity(entity) {
                    entity_commands.insert(exposure);
                }
            }
        }
        match current_ambient {
            Some(mut current) => *current = ambient.clone(),
            None => {
                if let Ok(mut entity_commands) = commands.get_entity(entity) {
                    entity_commands.insert(ambient.clone());
                }
            }
        }
    }
}

/// 注册 View 同步系统。
pub(crate) fn register(app: &mut bevy::prelude::App) {
    use bevy::prelude::PostUpdate;
    app.init_resource::<StylizedViewSettings>();
    app.add_systems(PostUpdate, sync_stylized_view);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposure_component_roundtrips_multiplier() {
        let exposure = exposure_component(1.0);
        assert!((exposure.exposure() - 1.0).abs() < 1e-5);
        let exposure = exposure_component(2.0);
        assert!((exposure.exposure() - 2.0).abs() < 1e-4);
        // 非法输入 fail 到 1.0
        assert!((exposure_component(f32::NAN).exposure() - 1.0).abs() < 1e-5);
        assert!((exposure_component(-3.0).exposure() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn ambient_lerps_between_night_and_day() {
        let mut descriptor = StylizedViewDescriptor {
            day_night_factor: 1.0,
            palette: vec![LinearRgba::new(0.9, 0.8, 0.7, 1.0)],
            ..StylizedViewDescriptor::default()
        };
        let day = resolve_ambient_light(&descriptor);
        let day_color = LinearRgba::from(day.color);
        assert!((day.brightness - DAY_AMBIENT_BRIGHTNESS).abs() < 1e-6);
        assert!((day_color.red - 0.9).abs() < 1e-5);

        descriptor.day_night_factor = 0.0;
        let night = resolve_ambient_light(&descriptor);
        let night_color = LinearRgba::from(night.color);
        assert!((night.brightness - NIGHT_AMBIENT_BRIGHTNESS).abs() < 1e-6);
        assert!(night_color.blue > night_color.red); // 夜间冷蓝

        descriptor.day_night_factor = 0.5;
        let mid = resolve_ambient_light(&descriptor);
        assert!(mid.brightness > night.brightness && mid.brightness < day.brightness);
    }

    #[test]
    fn view_sync_spawns_key_light_exposure_and_ambient() {
        use bevy::prelude::*;
        let mut app = App::new();
        app.add_plugins(bevy::app::TaskPoolPlugin::default());
        app.init_resource::<StylizedViewSettings>();
        app.add_systems(PostUpdate, sync_stylized_view);
        app.world_mut().spawn(Camera3d::default());
        app.world_mut().resource_mut::<StylizedViewSettings>().0 = StylizedViewDescriptor {
            key_light_direction: Vec3::new(0.0, 1.0, 0.0),
            exposure: 1.0,
            day_night_factor: 0.5,
            ..StylizedViewDescriptor::default()
        };
        app.update();

        let mut lights = app
            .world_mut()
            .query_filtered::<&DirectionalLight, With<StylizedKeyLight>>();
        assert_eq!(lights.iter(app.world()).count(), 1);
        let mut exposures = app.world_mut().query::<&Exposure>();
        assert_eq!(exposures.iter(app.world()).count(), 1);
        let exposure = *exposures.single(app.world()).unwrap();
        assert!((exposure.exposure() - 1.0).abs() < 1e-5);
        let mut ambients = app.world_mut().query::<&AmbientLight>();
        assert_eq!(ambients.iter(app.world()).count(), 1);
    }
}
