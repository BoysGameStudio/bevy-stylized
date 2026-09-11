use bevy::{
    camera::Hdr,
    core_pipeline::{
        Core3dSystems,
        fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin},
        prepass::{DepthPrepass, NormalPrepass},
        tonemapping::{Tonemapping, tonemapping},
    },
    ecs::{
        schedule::{IntoScheduleConfigs, ScheduleConfigs},
        system::BoxedSystem,
    },
    post_process::bloom::{Bloom, bloom},
    prelude::{
        App, Changed, Commands, Component, Entity, Has, Local, Plugin, Query, Reflect, With,
    },
    render::{extract_component::ExtractComponent, render_resource::ShaderType},
    shader::ShaderRef,
};

use crate::{GenshinDisplayTransform, StylizedPostProcessProfile};

/// Embedded WGSL path for the ordered stylized fullscreen pass.
pub const STYLIZED_POST_PROCESS_SHADER_PATH: &str =
    "embedded://bevy_cel/shaders/stylized_post_process.wgsl";

/// Extracted fullscreen uniform installed from a stylized camera profile.
#[derive(
    Clone, Copy, Component, Debug, Default, ExtractComponent, PartialEq, Reflect, ShaderType,
)]
pub struct StylizedFullscreenEffect {
    strengths: bevy::math::Vec4,
    vignette: bevy::math::Vec4,
}

impl StylizedFullscreenEffect {
    const fn from_profile(profile: StylizedPostProcessProfile) -> Self {
        let [vignette_strength, radius, softness, distortion, chromatic] =
            profile.settings().fullscreen_values();
        Self {
            strengths: bevy::math::Vec4::new(
                if profile.vignette_enabled() {
                    vignette_strength
                } else {
                    0.0
                },
                if profile.lens_distortion_enabled() {
                    distortion
                } else {
                    0.0
                },
                if profile.chromatic_aberration_enabled() {
                    chromatic
                } else {
                    0.0
                },
                0.0,
            ),
            vignette: bevy::math::Vec4::new(radius, softness, 0.0, 0.0),
        }
    }

    const fn is_enabled(profile: StylizedPostProcessProfile) -> bool {
        profile.vignette_enabled()
            || profile.lens_distortion_enabled()
            || profile.chromatic_aberration_enabled()
    }

    fn with_genshin_display_transform(mut self, enabled: bool) -> Self {
        self.strengths.w = if enabled { 1.0 } else { 0.0 };
        self
    }
}

impl FullscreenMaterial for StylizedFullscreenEffect {
    fn fragment_shader() -> ShaderRef {
        STYLIZED_POST_PROCESS_SHADER_PATH.into()
    }

    fn schedule_configs(system: ScheduleConfigs<BoxedSystem>) -> ScheduleConfigs<BoxedSystem> {
        system
            .in_set(Core3dSystems::PostProcess)
            .after(bloom)
            .before(tonemapping)
    }
}

/// Registers the Bevy 0.19 render-as-systems fullscreen material.
#[derive(Default)]
pub struct StylizedPostProcessPlugin;

impl Plugin for StylizedPostProcessPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FullscreenMaterialPlugin::<StylizedFullscreenEffect>::default())
            .add_systems(
                bevy::prelude::PostUpdate,
                (
                    apply_profiles,
                    sync_fullscreen_effect,
                    diagnose_missing_prepasses,
                )
                    .chain(),
            );
    }
}

fn apply_profiles(
    mut commands: Commands,
    profiles: Query<(Entity, &StylizedPostProcessProfile), Changed<StylizedPostProcessProfile>>,
) {
    for (entity, profile) in &profiles {
        let mut camera = commands.entity(entity);
        if profile.hdr_enabled() {
            camera.insert(Hdr);
        } else {
            camera.remove::<Hdr>();
        }
        if profile.bloom_enabled() {
            camera.insert(Bloom {
                intensity: profile.settings().bloom_intensity(),
                ..Bloom::NATURAL
            });
        } else {
            camera.remove::<Bloom>();
        }
        camera.insert(if profile.tonemapping_enabled() {
            Tonemapping::TonyMcMapface
        } else {
            Tonemapping::None
        });
        if profile.prepass_edges_enabled() {
            camera.insert((DepthPrepass, NormalPrepass));
        } else {
            camera.remove::<DepthPrepass>().remove::<NormalPrepass>();
        }
    }
}

/// Compose every camera-level color effect into one FullscreenMaterial pass.
/// Bevy 0.19's helper stores a non-generic pipeline-id component on each view,
/// so installing two different FullscreenMaterial types makes their prepare
/// systems overwrite/remove one another. One uniform and one pass avoids that
/// silent no-op while retaining independent opt-in components in the main world.
fn sync_fullscreen_effect(
    mut commands: Commands,
    cameras: Query<
        (
            Entity,
            Option<&StylizedPostProcessProfile>,
            Has<GenshinDisplayTransform>,
            Option<&StylizedFullscreenEffect>,
        ),
        With<bevy::prelude::Camera3d>,
    >,
) {
    for (entity, profile, genshin, current) in &cameras {
        let profile_enabled =
            profile.is_some_and(|profile| StylizedFullscreenEffect::is_enabled(*profile));
        if !profile_enabled && !genshin {
            if current.is_some() {
                commands.entity(entity).remove::<StylizedFullscreenEffect>();
            }
            continue;
        }
        let desired = profile
            .copied()
            .map(StylizedFullscreenEffect::from_profile)
            .unwrap_or_default()
            .with_genshin_display_transform(genshin);
        if current != Some(&desired) {
            commands.entity(entity).insert(desired);
        }
    }
}

type PrepassCamera<'a> = (
    &'a StylizedPostProcessProfile,
    Has<DepthPrepass>,
    Has<NormalPrepass>,
);

fn diagnose_missing_prepasses(
    cameras: Query<PrepassCamera<'static>, With<bevy::prelude::Camera3d>>,
    mut reported: Local<bool>,
) {
    let missing = cameras
        .iter()
        .any(|(profile, depth, normal)| profile.prepass_edges_enabled() && !(depth && normal));
    if missing && !*reported {
        bevy::log::warn!(
            "bevy_cel prepass-dependent edges disabled: depth and normal textures are required"
        );
        *reported = true;
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn genshin_marker_composes_into_the_single_fullscreen_uniform() {
        let mut world = bevy::prelude::World::new();
        let camera = world
            .spawn((bevy::prelude::Camera3d::default(), GenshinDisplayTransform))
            .id();

        world.run_system_once(sync_fullscreen_effect).unwrap();
        assert_eq!(
            world
                .get::<StylizedFullscreenEffect>(camera)
                .expect("composed fullscreen effect")
                .strengths
                .w,
            1.0
        );

        world.entity_mut(camera).remove::<GenshinDisplayTransform>();
        world.run_system_once(sync_fullscreen_effect).unwrap();
        assert!(world.get::<StylizedFullscreenEffect>(camera).is_none());
    }
}
