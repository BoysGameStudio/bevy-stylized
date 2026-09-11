//! CPU geometry inspection uses the same complete skin as CEL's vertex passes.
use crate::{ATTRIBUTE_CEL_JOINTS_1, ATTRIBUTE_CEL_WEIGHTS_1};
use bevy::{mesh::VertexAttributeValues as V, prelude::*};

/// Evaluate the displayed linear skin, including an optional second influence set.
/// Reject incomplete data instead of producing a different body for diagnostics.
pub fn skin_vertex_positions(mesh: &Mesh, matrices: &[Mat4]) -> Result<Vec<Vec3>, String> {
    let positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .and_then(V::as_float3)
        .ok_or("skin positions missing")?;
    let mut points = vec![Vec3::ZERO; positions.len()];
    for (joint_attribute, weight_attribute, required) in [
        (
            Mesh::ATTRIBUTE_JOINT_INDEX,
            Mesh::ATTRIBUTE_JOINT_WEIGHT,
            true,
        ),
        (ATTRIBUTE_CEL_JOINTS_1, ATTRIBUTE_CEL_WEIGHTS_1, false),
    ] {
        let (joints, weights) = match (
            mesh.attribute(joint_attribute),
            mesh.attribute(weight_attribute),
        ) {
            (None, None) if !required => continue,
            (Some(V::Uint16x4(j)), Some(V::Float32x4(w))) => (j, w),
            _ => return Err("skin influence attributes missing or invalid".into()),
        };
        if joints.len() != positions.len() || weights.len() != positions.len() {
            return Err("skin influence coverage mismatch".into());
        }
        for (i, (joints, weights)) in joints.iter().zip(weights).enumerate() {
            for (&joint, &weight) in joints.iter().zip(weights) {
                if weight == 0. {
                    continue;
                }
                let matrix = matrices
                    .get(usize::from(joint))
                    .ok_or("skin joint outside palette")?;
                points[i] += matrix.transform_point3(Vec3::from(positions[i])) * weight;
            }
        }
    }
    if points.iter().any(|p| !p.is_finite()) {
        return Err("non-finite skinned point".into());
    }
    Ok(points)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{asset::RenderAssetUsages, mesh::PrimitiveTopology};
    #[test]
    fn later_influences_preserve_full_weight_and_translation() {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[1., 2., 3.]]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, V::Uint16x4(vec![[0, 0, 0, 0]]));
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![[0.25, 0., 0., 0.]]);
        mesh.insert_attribute(ATTRIBUTE_CEL_JOINTS_1, V::Uint16x4(vec![[1, 0, 0, 0]]));
        mesh.insert_attribute(ATTRIBUTE_CEL_WEIGHTS_1, vec![[0.75, 0., 0., 0.]]);
        let result = skin_vertex_positions(
            &mesh,
            &[Mat4::IDENTITY, Mat4::from_translation(Vec3::X * 4.)],
        )
        .unwrap();
        assert_eq!(result, vec![Vec3::new(4., 2., 3.)]);
        let bounds = extended_skin_bounds(&mesh).unwrap();
        assert_eq!(
            bounds
                .aabb_index_to_joint_index
                .iter()
                .map(|j| j.0)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(bounds.aabbs[1].center, Vec3::new(1., 2., 3.));
        mesh.remove_attribute(ATTRIBUTE_CEL_WEIGHTS_1);
        assert!(skin_vertex_positions(&mesh, &[Mat4::IDENTITY]).is_err());
    }
}

fn extended_skin_bounds(mesh: &Mesh) -> Result<bevy::mesh::skinning::SkinnedMeshBounds, String> {
    use bevy::mesh::skinning::{JointAabb, SkinnedMeshBounds};
    let base = SkinnedMeshBounds::from_mesh(mesh).map_err(|e| e.to_string())?;
    let mut ranges = base
        .iter()
        .map(|(index, bound)| {
            (
                index.0,
                (
                    bound.center - bound.half_size,
                    bound.center + bound.half_size,
                ),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let Some(V::Uint16x4(joints)) = mesh.attribute(ATTRIBUTE_CEL_JOINTS_1) else {
        return Err("extended bounds need joint indices".into());
    };
    let Some(V::Float32x4(weights)) = mesh.attribute(ATTRIBUTE_CEL_WEIGHTS_1) else {
        return Err("extended bounds need joint weights".into());
    };
    let positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .and_then(V::as_float3)
        .ok_or("extended bounds need positions")?;
    if joints.len() != positions.len() || weights.len() != positions.len() {
        return Err("extended bounds attribute coverage".into());
    }
    for ((p, joints), weights) in positions.iter().zip(joints).zip(weights) {
        let p = Vec3::from(*p);
        for (&joint, &weight) in joints.iter().zip(weights) {
            if weight <= 0. {
                continue;
            }
            let key = joint;
            ranges
                .entry(key)
                .and_modify(|(min, max)| {
                    *min = min.min(p);
                    *max = max.max(p);
                })
                .or_insert((p, p));
        }
    }
    Ok(SkinnedMeshBounds {
        aabb_index_to_joint_index: ranges
            .keys()
            .copied()
            .map(bevy::mesh::skinning::JointIndex)
            .collect(),
        aabbs: ranges
            .values()
            .map(|(min, max)| JointAabb {
                center: (*min + *max) * 0.5,
                half_size: (*max - *min) * 0.5,
            })
            .collect(),
    })
}

pub(crate) fn update_extended_skin_bounds(
    mut events: MessageReader<AssetEvent<Mesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for event in events.read() {
        let id = match *event {
            AssetEvent::Added { id }
            | AssetEvent::Modified { id }
            | AssetEvent::LoadedWithDependencies { id } => id,
            _ => continue,
        };
        // Only main-world culling metadata changes. Do not trigger another
        // mesh-modified event or re-upload unchanged vertex buffers.
        let Some(mesh) = meshes.get_mut_untracked(id) else {
            continue;
        };
        if !mesh.contains_attribute(ATTRIBUTE_CEL_JOINTS_1) {
            continue;
        }
        match extended_skin_bounds(mesh) {
            Ok(bounds) => mesh.set_skinned_mesh_bounds(Some(bounds)),
            Err(error) => error!("CEL extended skin bounds rejected: {error}"),
        }
    }
}
