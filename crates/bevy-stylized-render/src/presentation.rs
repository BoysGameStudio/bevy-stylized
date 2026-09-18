//! Opt-in final-pose synchronization for hosts with their own replay clock.
use bevy::{
    ecs::schedule::{InternedScheduleLabel, ScheduleLabel},
    prelude::*,
};

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativePresentationSystems;

/// Insert before adding `StylizedPlugin` when the host owns the fullscreen pass.
/// Bevy's fullscreen-material helper shares one pipeline-id component between
/// material types; registering another type can remove the host's pipeline even
/// when no Stylized post-process component is present. This startup-only choice
/// leaves surface materials and the host's camera components intact.
#[derive(Resource, Default)]
pub struct ExternalPostProcess;

#[derive(Resource, Default)]
pub struct PresentationSchedule(Option<InternedScheduleLabel>);

pub(crate) fn native_schedule(owner: Option<Res<PresentationSchedule>>) -> bool {
    owner.is_none_or(|owner| owner.0.is_none())
}

/// Select the single schedule run by the host after its final animation and
/// physics pose. This does not advance time, animate bones or run a solver.
/// The host owns camera/light settings and final source GlobalTransforms.
/// Outline synchronization copies only replica globals; this schedule never
/// propagates or rewrites the host's transform hierarchy.
pub fn configure_presentation_schedule(app: &mut App, schedule: impl ScheduleLabel + Clone) {
    let label = schedule.intern();
    if let Some(owner) = app.world().get_resource::<PresentationSchedule>()
        && let Some(existing) = owner.0
    {
        assert_eq!(
            existing, label,
            "Stylized supports one host presentation schedule per App"
        );
        return;
    }
    app.insert_resource(PresentationSchedule(Some(label)));
    app.add_systems(
        schedule,
        (
            crate::lod::sync_lod,
            crate::outline::sync_outline_replicas,
            ApplyDeferred,
            crate::face_input::sync_face_inputs,
            crate::face_input::restore_removed_face_inputs,
            crate::instance_table::sync_instance_table,
            ApplyDeferred,
            crate::instance::sync_instance_tags,
            ApplyDeferred,
            crate::host_material::publish_host_inputs
                .run_if(resource_exists::<crate::host_material::HostFrameInputs>),
            ApplyDeferred,
        )
            .chain(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{instance::StylizedInstanceData, view::StylizedViewSettings};
    #[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
    struct FinalPose;
    #[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
    struct OtherPose;
    #[derive(Resource, Default)]
    struct NativeCount(u32);
    fn count(mut n: ResMut<NativeCount>) {
        n.0 += 1;
    }
    #[test]
    fn host_schedule_disables_native_sync_and_packs_without_advancing_a_clock() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin))
            .init_resource::<NativeCount>()
            .init_resource::<StylizedViewSettings>()
            .init_resource::<Assets<crate::hero::StylizedHeroMaterial>>()
            .configure_sets(
                PostUpdate,
                NativePresentationSystems.run_if(native_schedule),
            )
            .add_systems(PostUpdate, count.in_set(NativePresentationSystems));
        configure_presentation_schedule(&mut app, FinalPose);
        configure_presentation_schedule(&mut app, FinalPose);
        let entity = app
            .world_mut()
            .spawn((
                Transform::IDENTITY,
                GlobalTransform::IDENTITY,
                StylizedInstanceData::default(),
            ))
            .id();
        app.update();
        assert_eq!(app.world().resource::<NativeCount>().0, 0);
        assert!(app.world().get::<bevy::mesh::MeshTag>(entity).is_none());
        let before = app.world().resource::<Time>().elapsed();
        app.world_mut().run_schedule(FinalPose);
        assert!(app.world().get::<bevy::mesh::MeshTag>(entity).is_some());
        assert_eq!(app.world().resource::<Time>().elapsed(), before);
    }
    #[test]
    fn external_sync_preserves_host_globals_and_initializes_only_its_replica() {
        let mut app = App::new();
        app.init_resource::<StylizedViewSettings>()
            .init_resource::<Assets<crate::hero::StylizedHeroMaterial>>();
        configure_presentation_schedule(&mut app, FinalPose);
        let expected = GlobalTransform::from(Transform::from_xyz(9., 8., 7.));
        let source = app
            .world_mut()
            .spawn((
                Mesh3d(Handle::default()),
                crate::outline::StylizedOutline::new(Handle::default()),
                Transform::from_xyz(1., 0., 0.),
                expected,
            ))
            .id();
        app.world_mut().run_schedule(FinalPose);
        assert_eq!(
            *app.world().get::<GlobalTransform>(source).unwrap(),
            expected
        );
        let replica = app
            .world_mut()
            .query_filtered::<&GlobalTransform, With<crate::outline::StylizedOutlineReplica>>()
            .single(app.world())
            .unwrap();
        assert_eq!(*replica, expected);
    }

    #[test]
    #[should_panic(expected = "one host presentation schedule")]
    fn conflicting_schedule_is_rejected() {
        let mut app = App::new();
        configure_presentation_schedule(&mut app, FinalPose);
        configure_presentation_schedule(&mut app, OtherPose);
    }
}
