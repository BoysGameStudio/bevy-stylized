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
//!   语义；该选择是本迭代的显式决策，见函数文档）；
//! - `environment`（V2-d）→ 相机 `DistanceFog` 组件（雾参数 + 阴影染色
//!   通道，见 [`environment_distance_fog`]）与 zenith 天色清屏色。

use bevy::{
    camera::{Camera, ClearColorConfig, Exposure},
    color::{Color, LinearRgba},
    light::{AmbientLight, DirectionalLight},
    pbr::{DistanceFog, FogFalloff},
    prelude::{Camera3d, Commands, Component, Entity, Query, Res, Resource, Transform, Vec3, With},
};
use bevy_stylized_core::{StylizedEnvironment, StylizedViewDescriptor};

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
    if let Some(color) = descriptor.ambient_override {
        return AmbientLight {
            color: bevy::prelude::Color::LinearRgba(color),
            brightness: 1.0,
            ..AmbientLight::default()
        };
    }
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

/// 环境模型 → Bevy `DistanceFog` 组件映射（V2-d，provisional pending
/// ADR-0009）。
///
/// 通道约定（与 `npr_common.wgsl` 的环境段一致）：
///
/// - `color` = 雾色（rgb + alpha 强度）；`falloff = Linear(start, end)`；
/// - `directional_light_color` 复用为**风格化阴影染色**通道（rgb + alpha
///   强度）——Bevy 原生散射语义我方 shader 不消费；正式 view uniform 属
///   ADR-0009 决策。
///
/// `None` = 恒等（不挂组件，shader 侧无 `DISTANCE_FOG` def，零开销）。
/// 阴影染色强度 > 0 但雾未启用时仍挂组件（雾参数取 `f32::MAX`，因子恒 0）
/// ——阴影染色需要这条 uniform 通道。
#[must_use]
pub fn environment_distance_fog(environment: &StylizedEnvironment) -> Option<DistanceFog> {
    let fog_active = environment.fog.enabled && environment.fog.is_valid();
    let shadow_active = environment.shadow_tint.strength > 0.0;
    if !fog_active && !shadow_active {
        return None;
    }
    let (start, end) = if fog_active {
        (environment.fog.start, environment.fog.end)
    } else {
        // 不产雾：stylized_fog_factor 对 end <= start fail-closed 为 0。
        (f32::MAX, f32::MAX)
    };
    let tint = environment.shadow_tint;
    Some(DistanceFog {
        color: Color::LinearRgba(environment.fog.color),
        // 阴影染色通道（见函数文档的通道约定）。
        directional_light_color: Color::LinearRgba(LinearRgba::new(
            tint.color.red,
            tint.color.green,
            tint.color.blue,
            tint.strength.clamp(0.0, 1.0),
        )),
        // 我方 shader 不使用散射指数。
        directional_light_exponent: 0.0,
        falloff: FogFalloff::Linear { start, end },
    })
}

/// 每帧同步 View 层数据。
///
/// 只写 Bevy 的 view-layer 对象（key light 实体、相机组件），不触碰任何
/// 材质资产。
pub fn sync_stylized_view(
    mut commands: Commands,
    settings: Option<Res<StylizedViewSettings>>,
    mut key_lights: Query<(&mut Transform, &mut DirectionalLight), With<StylizedKeyLight>>,
    mut cameras: Query<
        (
            Entity,
            Option<&mut Exposure>,
            Option<&mut AmbientLight>,
            Option<&mut DistanceFog>,
            &mut Camera,
        ),
        With<Camera3d>,
    >,
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
    // 环境 NPR（V2-d）：DistanceFog 组件（雾 + 阴影染色通道）与 zenith
    // 天色（清屏色）；恒等默认下 DistanceFog 移除/不挂（零开销路径）。
    let distance_fog = environment_distance_fog(&descriptor.environment);
    for (entity, current_exposure, current_ambient, current_fog, mut camera) in &mut cameras {
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
        match (&distance_fog, current_fog) {
            (Some(fog), Some(mut current)) => *current = fog.clone(),
            (Some(fog), None) => {
                if let Ok(mut entity_commands) = commands.get_entity(entity) {
                    entity_commands.insert(fog.clone());
                }
            }
            (None, Some(_)) => {
                if let Ok(mut entity_commands) = commands.get_entity(entity) {
                    entity_commands.remove::<DistanceFog>();
                }
            }
            (None, None) => {}
        }
        // zenith 天色 → 平涂天空（清屏色）；`None` = 不触碰（渐变天空球
        // 超出 V2 first cut）。
        if let Some(zenith) = descriptor.environment.zenith_tint {
            camera.clear_color = ClearColorConfig::Custom(Color::LinearRgba(zenith));
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
    fn authored_ambient_is_explicit_and_default_policy_is_unchanged() {
        let mut descriptor = StylizedViewDescriptor::default();
        let previous = resolve_ambient_light(&descriptor);
        let color = LinearRgba::new(0.2, 0.3, 0.4, 1.0);
        descriptor.ambient_override = Some(color);
        for time in [0.0, 1.0] {
            descriptor.day_night_factor = time;
            let actual = resolve_ambient_light(&descriptor);
            assert_eq!(actual.color, bevy::prelude::Color::LinearRgba(color));
            assert_eq!(actual.brightness, 1.0);
        }
        descriptor.ambient_override = None;
        let restored = resolve_ambient_light(&descriptor);
        assert_eq!(restored.color, previous.color);
        assert_eq!(restored.brightness, previous.brightness);
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
        // 环境恒等默认：不挂 DistanceFog，清屏色不动。
        assert_eq!(
            app.world_mut()
                .query::<&DistanceFog>()
                .iter(app.world())
                .count(),
            0
        );
    }

    #[test]
    fn environment_distance_fog_mapping() {
        use bevy_stylized_core::{StylizedFog, StylizedShadowTint};

        // 恒等 → 不挂组件。
        assert!(environment_distance_fog(&StylizedEnvironment::default()).is_none());

        // 雾启用 → Linear(start, end) + 雾色。
        let env = StylizedEnvironment {
            fog: StylizedFog {
                enabled: true,
                color: LinearRgba::new(0.5, 0.6, 0.7, 1.0),
                start: 10.0,
                end: 80.0,
            },
            ..StylizedEnvironment::default()
        };
        let fog = environment_distance_fog(&env).expect("fog component");
        match fog.falloff {
            FogFalloff::Linear { start, end } => {
                assert_eq!((start, end), (10.0, 80.0));
            }
            other => panic!("expected linear falloff, got {other:?}"),
        }
        assert_eq!(
            fog.color,
            Color::LinearRgba(LinearRgba::new(0.5, 0.6, 0.7, 1.0))
        );

        // 雾参数非法（end <= start）→ fail-closed 不产雾（但阴影通道仍可
        // 独立激活）。
        let invalid = StylizedEnvironment {
            fog: StylizedFog {
                enabled: true,
                start: 80.0,
                end: 10.0,
                ..StylizedFog::default()
            },
            ..StylizedEnvironment::default()
        };
        assert!(environment_distance_fog(&invalid).is_none());

        // 纯阴影染色 → 仍挂组件，雾参数取 MAX（因子恒 0），染色经
        // directional_light_color（rgb + alpha 强度）。
        let shadow_only = StylizedEnvironment {
            shadow_tint: StylizedShadowTint {
                color: LinearRgba::new(0.4, 0.4, 0.6, 1.0),
                strength: 0.5,
            },
            ..StylizedEnvironment::default()
        };
        let fog = environment_distance_fog(&shadow_only).expect("shadow tint channel");
        match fog.falloff {
            FogFalloff::Linear { start, end } => {
                assert_eq!(start, f32::MAX);
                assert_eq!(end, f32::MAX);
            }
            other => panic!("expected linear falloff, got {other:?}"),
        }
        let LinearRgba {
            red,
            green,
            blue,
            alpha,
        } = LinearRgba::from(fog.directional_light_color);
        assert!((red - 0.4).abs() < 1e-6);
        assert!((blue - 0.6).abs() < 1e-6);
        assert!((alpha - 0.5).abs() < 1e-6);
        let _ = green;
    }

    #[test]
    fn view_sync_manages_fog_component_and_zenith_clear_color() {
        use bevy::prelude::*;
        use bevy_stylized_core::{StylizedFog, StylizedShadowTint};
        let mut app = App::new();
        app.add_plugins(bevy::app::TaskPoolPlugin::default());
        app.init_resource::<StylizedViewSettings>();
        app.add_systems(PostUpdate, sync_stylized_view);
        let camera = app.world_mut().spawn(Camera3d::default()).id();
        app.update();
        // 恒等默认：无 fog 组件。
        assert!(
            app.world().entity(camera).get::<DistanceFog>().is_none(),
            "identity default must not attach fog"
        );

        // 开雾 + 阴影染色 + zenith → 组件出现、清屏色更新。
        app.world_mut().resource_mut::<StylizedViewSettings>().0 = StylizedViewDescriptor {
            environment: StylizedEnvironment {
                fog: StylizedFog {
                    enabled: true,
                    ..StylizedFog::default()
                },
                shadow_tint: StylizedShadowTint {
                    strength: 0.6,
                    ..StylizedShadowTint::default()
                },
                zenith_tint: Some(LinearRgba::new(0.2, 0.3, 0.6, 1.0)),
            },
            ..StylizedViewDescriptor::default()
        };
        app.update();
        assert!(app.world().entity(camera).get::<DistanceFog>().is_some());
        let clear = &app
            .world()
            .entity(camera)
            .get::<Camera>()
            .unwrap()
            .clear_color;
        match clear {
            ClearColorConfig::Custom(Color::LinearRgba(color)) => {
                let expected = LinearRgba::new(0.2, 0.3, 0.6, 1.0);
                assert!((color.red - expected.red).abs() < 1e-6);
                assert!((color.blue - expected.blue).abs() < 1e-6);
            }
            other => panic!("expected custom zenith clear color, got {other:?}"),
        }

        // 回到恒等 → fog 组件移除。
        app.world_mut().resource_mut::<StylizedViewSettings>().0 =
            StylizedViewDescriptor::default();
        app.update();
        assert!(
            app.world().entity(camera).get::<DistanceFog>().is_none(),
            "identity must remove the fog component (zero-cost path)"
        );
    }
}
