use bevy::{
    camera::Hdr,
    core_pipeline::{
        Core3dSystems,
        fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin},
        prepass::{DepthPrepass, NormalPrepass},
        tonemapping::{Tonemapping, tonemapping},
    },
    ecs::{
        change_detection::DetectChanges,
        schedule::{IntoScheduleConfigs, ScheduleConfigs},
        system::BoxedSystem,
    },
    post_process::bloom::{Bloom, bloom},
    prelude::{
        App, Assets, Camera3d, Commands, Component, Entity, Has, Local, Plugin, Query, Ref,
        Reflect, Res, With,
    },
    render::{extract_component::ExtractComponent, render_resource::ShaderType},
    shader::ShaderRef,
};

use crate::{GenshinDisplayTransform, InternalEdgeMaterial, StylizedPostProcessProfile};

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

/// Owns the crate-managed camera rendering components and enforces the union of
/// their requirements every frame.
///
/// Cameras carrying a [`StylizedPostProcessProfile`] or a [`GenshinDisplayTransform`]
/// are *managed*: [`Hdr`], [`Bloom`], [`Tonemapping`], [`DepthPrepass`], and
/// [`NormalPrepass`] on them are owned by this system, and manual insertion or
/// removal of those components is overridden. The enforced state is the union of
///
/// - the profile's toggles (`hdr`, `bloom`, `tonemapping`, `prepass_edges`),
/// - the HDR intermediate required by an active [`GenshinDisplayTransform`], and
/// - the depth/normal prepasses required while any enabled
///   [`InternalEdgeMaterial`] exists.
///
/// Cameras without a profile or display transform are never touched, and
/// [`Tonemapping`] is only written while a profile is present.
fn apply_profiles(
    mut commands: Commands,
    internal_edge_materials: Option<Res<Assets<InternalEdgeMaterial>>>,
    mut cameras: Query<
        (
            Entity,
            Option<Ref<StylizedPostProcessProfile>>,
            Has<GenshinDisplayTransform>,
            Has<Hdr>,
            Has<Bloom>,
            Has<DepthPrepass>,
            Has<NormalPrepass>,
            Option<&Tonemapping>,
        ),
        With<Camera3d>,
    >,
) {
    let internal_edge_requested = internal_edge_materials
        .as_ref()
        .is_some_and(|materials| materials.iter().any(|(_, m)| m.settings.is_enabled()));
    for (entity, profile, genshin, has_hdr, has_bloom, has_depth, has_normal, tonemapping) in
        &mut cameras
    {
        if profile.is_none() && !genshin {
            continue;
        }
        let profile_changed = profile.as_ref().is_some_and(|p| p.is_changed());
        let profile = profile.as_deref().copied();
        let mut camera = commands.entity(entity);

        let hdr_required = profile.is_some_and(|p| p.hdr_enabled()) || genshin;
        if hdr_required && !has_hdr {
            camera.insert(Hdr);
        } else if !hdr_required && has_hdr {
            camera.remove::<Hdr>();
        }

        let bloom_required = profile.is_some_and(|p| p.bloom_enabled());
        if bloom_required && (!has_bloom || profile_changed) {
            camera.insert(Bloom {
                intensity: profile
                    .expect("bloom requires a profile")
                    .settings()
                    .bloom_intensity(),
                ..Bloom::NATURAL
            });
        } else if !bloom_required && has_bloom {
            camera.remove::<Bloom>();
        }

        if let Some(profile) = profile {
            let desired = if profile.tonemapping_enabled() {
                Tonemapping::TonyMcMapface
            } else {
                Tonemapping::None
            };
            if tonemapping != Some(&desired) {
                camera.insert(desired);
            }
        }

        let prepass_required =
            profile.is_some_and(|p| p.prepass_edges_enabled()) || internal_edge_requested;
        if prepass_required && !(has_depth && has_normal) {
            camera.insert((DepthPrepass, NormalPrepass));
        } else if !prepass_required && (has_depth || has_normal) {
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
