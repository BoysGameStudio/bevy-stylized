//! Consume an application-owned head transform without controlling its pose.
//! This adapter owns only the two face fields of MeshTag instance data while bound.
use crate::{
    hero::{HERO_FLAG_GENSHIN_FACE, StylizedHeroMaterial},
    instance::{StylizedInstanceData, StylizedInstanceSlot},
    view::StylizedViewSettings,
};
use bevy::prelude::*;

/// Attach to the rendered Hero face mesh; `head` is supplied by the application.
/// Local directions are expressed in that head entity's coordinates.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[require(StylizedInstanceData)]
pub struct StylizedFaceInput {
    pub head: Entity,
    pub forward: Vec3,
    pub up: Vec3,
}
/// Inspectable binding failure. Missing head/material data never retains stale overrides.
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
pub struct StylizedFaceInputStatus {
    pub error: Option<String>,
}
#[derive(Component)]
#[doc(hidden)]
pub struct StylizedFaceInputRestore {
    threshold: Option<f32>,
    flip: bool,
}

fn dots(light: Vec3, forward: Vec3, up: Vec3, project: bool) -> Result<(f32, f32), String> {
    if !forward.is_finite()
        || !up.is_finite()
        || forward.length_squared() < 1e-8
        || up.length_squared() < 1e-8
        || forward.cross(up).length_squared() < 1e-8
    {
        return Err("invalid external face basis".into());
    }
    let forward = forward.normalize();
    let up = up.normalize();
    let light = if project {
        light - up * light.dot(up)
    } else {
        light
    };
    if !light.is_finite() {
        return Err("invalid face light direction".into());
    }
    if light.length_squared() <= 1e-8 {
        return Ok((1., 0.));
    }
    let light = light.normalize();
    Ok((
        forward.dot(light).clamp(-1., 1.),
        forward.cross(up).normalize().dot(light),
    ))
}
fn overrides(
    binding: &StylizedFaceInput,
    head: &GlobalTransform,
    mesh: &GlobalTransform,
    material: &StylizedHeroMaterial,
    light: Vec3,
) -> Result<(f32, bool), String> {
    if material.params.character.face_misc.z > 0.5 {
        return Err("explicit material face-side override conflicts with external binding".into());
    }
    let genshin = material.params.has_flag(HERO_FLAG_GENSHIN_FACE);
    let h = head.to_matrix();
    let m = mesh.to_matrix();
    let (front, side) = dots(
        light,
        h.transform_vector3(binding.forward),
        h.transform_vector3(binding.up),
        genshin,
    )?;
    let (_, base_side) = dots(
        light,
        m.transform_vector3(material.params.character.face_forward_softness.truncate()),
        m.transform_vector3(material.params.character.face_up.truncate()),
        genshin,
    )?;
    let threshold = if genshin {
        ((1. - front) * 0.5)
            .min(0.999)
            .powf((1.5 - material.params.hero_a.z).powi(3))
    } else {
        front.acos() / std::f32::consts::PI
    };
    if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
        return Err("invalid face threshold parameters".into());
    }
    let flip = if genshin {
        (side >= 0.) != (base_side >= 0.)
    } else {
        (side < 0.) != (base_side < 0.)
    };
    Ok((threshold, flip))
}
pub fn sync_face_inputs(
    mut commands: Commands,
    view: Res<StylizedViewSettings>,
    transforms: Query<&GlobalTransform>,
    materials: Res<Assets<StylizedHeroMaterial>>,
    mut faces: Query<(
        Entity,
        &StylizedFaceInput,
        &GlobalTransform,
        Option<&MeshMaterial3d<StylizedHeroMaterial>>,
        Option<&StylizedInstanceSlot>,
        &mut StylizedInstanceData,
        Option<&StylizedFaceInputRestore>,
        Option<&StylizedFaceInputStatus>,
    )>,
) {
    for (entity, binding, mesh, handle, slot, mut data, restore, status) in &mut faces {
        let old = restore
            .map(|r| (r.threshold, r.flip))
            .unwrap_or((data.face_threshold, data.face_side_flip));
        if restore.is_none() {
            commands.entity(entity).insert(StylizedFaceInputRestore {
                threshold: old.0,
                flip: old.1,
            });
        }
        let result = (|| {
            if slot.is_some() {
                return Err("external Hero face binding requires MeshTag transport".into());
            }
            let head = transforms
                .get(binding.head)
                .map_err(|_| "external head entity/transform unavailable")?;
            let material = handle
                .and_then(|h| materials.get(&h.0))
                .ok_or("Hero face material not ready")?;
            if material.face_sdf_texture.is_none() {
                return Err("face SDF texture required for external face input".into());
            }
            overrides(binding, head, mesh, material, view.0.key_light_direction)
        })();
        let desired = result.as_ref().map(|(t, f)| (Some(*t), *f)).unwrap_or(old);
        if (data.face_threshold, data.face_side_flip) != desired {
            data.face_threshold = desired.0;
            data.face_side_flip = desired.1;
        }
        let next = StylizedFaceInputStatus {
            error: result.err(),
        };
        if status != Some(&next) {
            commands.entity(entity).insert(next);
        }
    }
}
pub fn restore_removed_face_inputs(
    mut commands: Commands,
    mut removed: RemovedComponents<StylizedFaceInput>,
    mut data: Query<
        (&mut StylizedInstanceData, &StylizedFaceInputRestore),
        Without<StylizedFaceInput>,
    >,
) {
    for entity in removed.read() {
        if let Ok((mut data, old)) = data.get_mut(entity) {
            data.face_threshold = old.threshold;
            data.face_side_flip = old.flip;
            commands
                .entity(entity)
                .remove::<(StylizedFaceInputRestore, StylizedFaceInputStatus)>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_pose_updates_instance_without_cloning_material_and_restores_on_removal() {
        let mut app = App::new();
        app.init_resource::<Assets<StylizedHeroMaterial>>()
            .init_resource::<StylizedViewSettings>();
        app.world_mut()
            .resource_mut::<StylizedViewSettings>()
            .0
            .key_light_direction = Vec3::Z;
        app.add_systems(
            Update,
            (
                sync_face_inputs,
                restore_removed_face_inputs,
                crate::instance::sync_instance_tags,
            )
                .chain(),
        );
        let head = app.world_mut().spawn(GlobalTransform::IDENTITY).id();
        let material = StylizedHeroMaterial {
            face_sdf_texture: Some(Handle::default()),
            ..Default::default()
        };
        let material = app
            .world_mut()
            .resource_mut::<Assets<StylizedHeroMaterial>>()
            .add(material);
        let face = app
            .world_mut()
            .spawn((
                GlobalTransform::IDENTITY,
                MeshMaterial3d(material),
                StylizedFaceInput {
                    head,
                    forward: Vec3::Z,
                    up: Vec3::Y,
                },
            ))
            .id();
        app.update();
        assert_eq!(
            app.world()
                .get::<StylizedInstanceData>(face)
                .unwrap()
                .face_threshold,
            Some(0.)
        );
        app.world_mut()
            .entity_mut(head)
            .insert(GlobalTransform::from(Transform::from_rotation(
                Quat::from_rotation_y(std::f32::consts::PI),
            )));
        app.update();
        assert!(
            app.world()
                .get::<StylizedInstanceData>(face)
                .unwrap()
                .face_threshold
                .unwrap()
                > 0.99
        );
        assert_eq!(
            app.world().resource::<Assets<StylizedHeroMaterial>>().len(),
            1
        );
        app.world_mut().despawn(head);
        app.update();
        assert!(
            app.world()
                .get::<StylizedFaceInputStatus>(face)
                .unwrap()
                .error
                .is_some()
        );
        assert_eq!(
            app.world()
                .get::<StylizedInstanceData>(face)
                .unwrap()
                .face_threshold,
            None
        );
        app.world_mut()
            .entity_mut(face)
            .remove::<StylizedFaceInput>();
        app.update();
        assert!(app.world().get::<StylizedFaceInputRestore>(face).is_none());
    }
}
