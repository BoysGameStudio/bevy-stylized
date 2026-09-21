//! 生产侧后处理（It-5）：单 pass 全屏效果（docs/rendering/03
//! "PostProcess 保留 Single Fullscreen Pass 思想"）。
//!
//! - **单 pass 约束**：一个相机同一时刻只允许一个
//!   `FullscreenMaterial` 类型生效（多个 FullscreenMaterial 会互相覆盖
//!   prepare 结果——legacy 已踩过）；所有相机级色彩效果（vignette /
//!   chromatic aberration / display transform hook）必须合成进这一个
//!   pass，不得再拆。
//! - **View 层驱动**：启用状态来自 core `StylizedViewDescriptor.camera`
//!   （[`crate::view::StylizedViewSettings`]），参数来自相机上的
//!   [`StylizedPostProcess`] 组件；不触碰任何材质资产。
//! - **Portable 合法**：LDR-safe（无 HDR 依赖）；`chromatic = 0` 时只采
//!   中心颜色（disabled feature 减少采样，docs/rendering/03）。

use bevy::{
    core_pipeline::{
        Core3dSystems,
        fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin},
        tonemapping::tonemapping,
    },
    ecs::schedule::{IntoScheduleConfigs, ScheduleConfigs},
    prelude::{App, Camera3d, Commands, Component, Entity, Or, Query, Reflect, Res, With, Without},
    render::{extract_component::ExtractComponent, render_resource::ShaderType},
    shader::ShaderRef,
};

use crate::view::StylizedViewSettings;

/// 全屏后处理 shader 路径。
pub const STYLIZED_FULLSCREEN_SHADER_PATH: &str =
    "embedded://bevy_stylized_render/shaders/post/stylized_fullscreen.wgsl";

/// Display transform hook 模式（provisional；profile 级 display transform
/// 属于 profile crate）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum StylizedDisplayTransform {
    /// 直通（LDR 默认）。
    #[default]
    None,
    /// 上游 Genshin Tonemapper 有理曲线（docs/reference/07 §3.8 已核实；
    /// 命名保持 hook 形态，profile 级变体 pending ADR-0009）。
    SimpleCompress,
}

/// 相机级后处理参数（View 层；挂在 Camera3d 上）。
///
/// 各强度为 0 时对应采样路径在 shader 内跳过。
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
pub struct StylizedPostProcess {
    /// Vignette 强度（0 关闭）。
    pub vignette_strength: f32,
    /// Vignette 半径（归一化）。
    pub vignette_radius: f32,
    /// Vignette 软过渡。
    pub vignette_softness: f32,
    /// Chromatic aberration（归一化 UV 偏移；0 = 单采样）。
    pub chromatic_aberration: f32,
    /// Display transform hook（`SimpleCompress` 的曲线已按
    /// docs/reference/07 §3.8 核实的上游有理函数实现）。
    pub display_transform: StylizedDisplayTransform,
}

impl Default for StylizedPostProcess {
    fn default() -> Self {
        Self {
            vignette_strength: 0.3,
            vignette_radius: 0.9,
            vignette_softness: 0.25,
            chromatic_aberration: 0.0,
            display_transform: StylizedDisplayTransform::None,
        }
    }
}

/// 提取进 render world 的全屏 uniform（`FullscreenMaterial` 载体）。
#[derive(
    Clone, Copy, Component, Debug, Default, ExtractComponent, PartialEq, Reflect, ShaderType,
)]
pub struct StylizedFullscreenEffect {
    /// x = vignette 强度，y = chromatic，z = display transform 模式，w 预留。
    pub strengths: bevy::math::Vec4,
    /// x = vignette 半径，y = vignette 软过渡。
    pub vignette: bevy::math::Vec4,
}

impl From<&StylizedPostProcess> for StylizedFullscreenEffect {
    fn from(settings: &StylizedPostProcess) -> Self {
        Self {
            strengths: bevy::math::Vec4::new(
                settings.vignette_strength,
                settings.chromatic_aberration,
                match settings.display_transform {
                    StylizedDisplayTransform::None => 0.0,
                    StylizedDisplayTransform::SimpleCompress => 1.0,
                },
                0.0,
            ),
            vignette: bevy::math::Vec4::new(
                settings.vignette_radius,
                settings.vignette_softness,
                0.0,
                0.0,
            ),
        }
    }
}

impl FullscreenMaterial for StylizedFullscreenEffect {
    fn fragment_shader() -> ShaderRef {
        STYLIZED_FULLSCREEN_SHADER_PATH.into()
    }

    fn schedule_configs(
        system: ScheduleConfigs<bevy::ecs::system::BoxedSystem>,
    ) -> ScheduleConfigs<bevy::ecs::system::BoxedSystem> {
        // 单 pass 约束：在 tonemapping 之前、bloom 之后的固定槽位。
        system
            .in_set(Core3dSystems::PostProcess)
            .before(tonemapping)
    }
}

#[derive(Component)]
struct ManagedPostEffect {
    previous: Option<StylizedFullscreenEffect>,
}

/// The high-level controller owns its native pass while attached. Disabled/zero
/// settings remove that pass; removing the controller restores any prior manual
/// effect, without touching unrelated cameras or material assets.
fn sync_post_process_enabled(
    mut commands: Commands,
    settings: Option<Res<StylizedViewSettings>>,
    mut cameras: Query<
        (
            Entity,
            &StylizedPostProcess,
            Option<&mut StylizedFullscreenEffect>,
            Option<&ManagedPostEffect>,
        ),
        With<Camera3d>,
    >,
    removed: Query<
        (Entity, &ManagedPostEffect),
        Or<(Without<StylizedPostProcess>, Without<Camera3d>)>,
    >,
) {
    let descriptor = settings.map(|s| s.0.clone()).unwrap_or_default();
    let enabled = descriptor.camera.post_enabled;
    for (entity, owned) in &removed {
        let mut target = commands.entity(entity);
        if let Some(previous) = owned.previous {
            target.insert(previous);
        } else {
            target.remove::<StylizedFullscreenEffect>();
        }
        target.remove::<ManagedPostEffect>();
    }
    for (entity, settings, effect, owned) in &mut cameras {
        if owned.is_none() {
            commands.entity(entity).insert(ManagedPostEffect {
                previous: effect.as_deref().copied(),
            });
        }
        let active = enabled
            && (settings.vignette_strength != 0.
                || settings.chromatic_aberration != 0.
                || settings.display_transform != StylizedDisplayTransform::None);
        if active {
            let next = StylizedFullscreenEffect::from(settings);
            if let Some(mut effect) = effect {
                if *effect != next {
                    *effect = next;
                }
            } else {
                commands.entity(entity).insert(next);
            }
        } else if effect.is_some() {
            commands.entity(entity).remove::<StylizedFullscreenEffect>();
        }
    }
}

/// 注册后处理（由 `StylizedRenderPlugin` 调用）。
pub(crate) fn register(app: &mut App) {
    if app
        .world()
        .contains_resource::<crate::presentation::ExternalPostProcess>()
    {
        return;
    }
    use bevy::prelude::PostUpdate;
    app.add_plugins(FullscreenMaterialPlugin::<StylizedFullscreenEffect>::default())
        .add_systems(PostUpdate, sync_post_process_enabled);
}

#[cfg(test)]
mod tests {
    #[test]
    fn external_owner_does_not_register_a_competing_fullscreen_pipeline() {
        let mut app = bevy::prelude::App::new();
        app.insert_resource(crate::presentation::ExternalPostProcess);
        super::register(&mut app);
        assert!(!app.is_plugin_added::<bevy::core_pipeline::fullscreen_material::FullscreenMaterialPlugin<super::StylizedFullscreenEffect>>());
    }
    use super::*;
    use bevy::prelude::PostUpdate;
    use bevy_stylized_core::StylizedViewDescriptor;

    #[test]
    fn effect_maps_settings() {
        let settings = StylizedPostProcess {
            vignette_strength: 0.4,
            chromatic_aberration: 0.005,
            display_transform: StylizedDisplayTransform::SimpleCompress,
            ..StylizedPostProcess::default()
        };
        let effect = StylizedFullscreenEffect::from(&settings);
        assert!((effect.strengths.x - 0.4).abs() < f32::EPSILON);
        assert!((effect.strengths.y - 0.005).abs() < f32::EPSILON);
        assert!((effect.strengths.z - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn disabled_view_state_removes_owned_pass_and_restores_manual_effect() {
        let mut app = App::new();
        app.init_resource::<StylizedViewSettings>();
        app.world_mut().resource_mut::<StylizedViewSettings>().0 = StylizedViewDescriptor {
            camera: bevy_stylized_core::StylizedCameraNprState {
                outline_enabled: true,
                post_enabled: false,
            },
            ..StylizedViewDescriptor::default()
        };
        let camera = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                StylizedPostProcess::default(),
                StylizedFullscreenEffect {
                    strengths: bevy::math::Vec4::ONE,
                    vignette: bevy::math::Vec4::ONE,
                },
            ))
            .id();
        app.add_systems(PostUpdate, sync_post_process_enabled);
        app.update();
        assert!(
            app.world()
                .get::<StylizedFullscreenEffect>(camera)
                .is_none()
        );
        app.world_mut()
            .entity_mut(camera)
            .remove::<StylizedPostProcess>();
        app.update();
        assert_eq!(
            app.world()
                .get::<StylizedFullscreenEffect>(camera)
                .unwrap()
                .strengths,
            bevy::math::Vec4::ONE
        );
    }

    #[test]
    fn controller_creates_pass_then_zero_and_removal_reclaim_it() {
        let mut app = App::new();
        app.init_resource::<StylizedViewSettings>();
        app.add_systems(PostUpdate, sync_post_process_enabled);
        let camera = app
            .world_mut()
            .spawn((Camera3d::default(), StylizedPostProcess::default()))
            .id();
        app.update();
        assert!(
            app.world()
                .get::<StylizedFullscreenEffect>(camera)
                .is_some()
        );
        app.world_mut()
            .get_mut::<StylizedPostProcess>(camera)
            .unwrap()
            .vignette_strength = 0.;
        app.update();
        assert!(
            app.world()
                .get::<StylizedFullscreenEffect>(camera)
                .is_none()
        );
        app.world_mut()
            .get_mut::<StylizedPostProcess>(camera)
            .unwrap()
            .vignette_strength = 0.3;
        app.update();
        assert!(
            app.world()
                .get::<StylizedFullscreenEffect>(camera)
                .is_some()
        );
        app.world_mut()
            .entity_mut(camera)
            .remove::<StylizedPostProcess>();
        app.update();
        assert!(
            app.world()
                .get::<StylizedFullscreenEffect>(camera)
                .is_none()
        );
        assert!(app.world().get::<ManagedPostEffect>(camera).is_none());
    }
}
