//! Complete optional eight-influence skinning for render passes and inspection.
use bevy::{
    mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef, VertexAttributeValues as V},
    prelude::*,
    render::render_resource::{
        RenderPipelineDescriptor, SpecializedMeshPipelineError, VertexFormat,
    },
};

// Stable serialized provider IDs, shared with the glTF adapter without making
// the rendering crate depend on the importer.
pub const ATTRIBUTE_SECONDARY_JOINTS: MeshVertexAttribute = MeshVertexAttribute::new(
    "_CEL_JOINTS_1",
    0x4345_4c00_0000_000a,
    VertexFormat::Uint16x4,
);
pub const ATTRIBUTE_SECONDARY_WEIGHTS: MeshVertexAttribute = MeshVertexAttribute::new(
    "_CEL_WEIGHTS_1",
    0x4345_4c00_0000_000b,
    VertexFormat::Float32x4,
);
pub const COMPLETE_SKIN_PREPASS_SHADER: &str =
    "embedded://bevy_stylized_render/shaders/common/complete_skin_prepass.wgsl";

pub fn specialize_secondary_skin(
    descriptor: &mut RenderPipelineDescriptor,
    layout: &MeshVertexBufferLayoutRef,
) -> Result<(), SpecializedMeshPipelineError> {
    if !layout.0.contains(ATTRIBUTE_SECONDARY_JOINTS)
        && !layout.0.contains(ATTRIBUTE_SECONDARY_WEIGHTS)
    {
        return Ok(());
    }
    let extra = layout.0.get_layout(&[
        ATTRIBUTE_SECONDARY_JOINTS.at_shader_location(8),
        ATTRIBUTE_SECONDARY_WEIGHTS.at_shader_location(9),
    ])?;
    descriptor.vertex.buffers[0]
        .attributes
        .extend(extra.attributes);
    descriptor
        .vertex
        .shader_defs
        .push("NPR_SKIN_EXTENDED".into());
    Ok(())
}

pub fn skin_vertex_positions(mesh: &Mesh, matrices: &[Mat4]) -> Result<Vec<Vec3>, String> {
    let positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .and_then(V::as_float3)
        .ok_or("skin positions missing")?;
    let mut points = vec![Vec3::ZERO; positions.len()];
    for (joints, weights, required) in [
        (
            Mesh::ATTRIBUTE_JOINT_INDEX,
            Mesh::ATTRIBUTE_JOINT_WEIGHT,
            true,
        ),
        (
            ATTRIBUTE_SECONDARY_JOINTS,
            ATTRIBUTE_SECONDARY_WEIGHTS,
            false,
        ),
    ] {
        let (joints, weights) = match (mesh.attribute(joints), mesh.attribute(weights)) {
            (None, None) if !required => continue,
            (Some(V::Uint16x4(joints)), Some(V::Float32x4(weights))) => (joints, weights),
            _ => return Err("skin influence attributes missing or invalid".into()),
        };
        if joints.len() != positions.len() || weights.len() != positions.len() {
            return Err("skin influence coverage mismatch".into());
        }
        for (i, (joints, weights)) in joints.iter().zip(weights).enumerate() {
            for (&joint, &weight) in joints.iter().zip(weights) {
                if !weight.is_finite() || weight < 0.0 {
                    return Err("invalid skin weight".into());
                }
                if weight == 0.0 {
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

pub fn complete_skin_bounds(
    mesh: &Mesh,
) -> Result<bevy::mesh::skinning::SkinnedMeshBounds, String> {
    use bevy::mesh::skinning::{JointAabb, JointIndex, SkinnedMeshBounds};
    let base = SkinnedMeshBounds::from_mesh(mesh).map_err(|e| e.to_string())?;
    let mut ranges = base
        .iter()
        .map(|(i, b)| (i.0, (b.center - b.half_size, b.center + b.half_size)))
        .collect::<std::collections::BTreeMap<_, _>>();
    let (joints, weights) = match (
        mesh.attribute(ATTRIBUTE_SECONDARY_JOINTS),
        mesh.attribute(ATTRIBUTE_SECONDARY_WEIGHTS),
    ) {
        (None, None) => return Ok(base),
        (Some(V::Uint16x4(j)), Some(V::Float32x4(w))) => (j, w),
        _ => return Err("incomplete secondary skin set".into()),
    };
    let positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .and_then(V::as_float3)
        .ok_or("skin positions missing")?;
    if joints.len() != positions.len() || weights.len() != positions.len() {
        return Err("skin coverage mismatch".into());
    }
    for ((position, joints), weights) in positions.iter().zip(joints).zip(weights) {
        let point = Vec3::from(*position);
        if !point.is_finite() {
            return Err("non-finite skin position".into());
        }
        for (&joint, &weight) in joints.iter().zip(weights) {
            if !weight.is_finite() || weight < 0.0 {
                return Err("invalid skin weight".into());
            }
            if weight == 0.0 {
                continue;
            }
            ranges
                .entry(joint)
                .and_modify(|(min, max)| {
                    *min = min.min(point);
                    *max = max.max(point);
                })
                .or_insert((point, point));
        }
    }
    Ok(SkinnedMeshBounds {
        aabb_index_to_joint_index: ranges.keys().copied().map(JointIndex).collect(),
        aabbs: ranges
            .values()
            .map(|(min, max)| JointAabb {
                center: (*min + *max) * 0.5,
                half_size: (*max - *min) * 0.5,
            })
            .collect(),
    })
}

pub(crate) fn update_complete_skin_bounds(
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
        let Some(mesh) = meshes.get_mut_untracked(id) else {
            continue;
        };
        if !mesh.contains_attribute(ATTRIBUTE_SECONDARY_JOINTS)
            && !mesh.contains_attribute(ATTRIBUTE_SECONDARY_WEIGHTS)
        {
            continue;
        }
        match complete_skin_bounds(mesh) {
            Ok(bounds) => mesh.set_skinned_mesh_bounds(Some(bounds)),
            Err(error) => error!("Stylized complete skin bounds rejected: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{asset::RenderAssetUsages, mesh::PrimitiveTopology};
    #[test]
    fn secondary_influences_move_surface_and_bounds_and_reject_half_sets() {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[1.0, 2.0, 3.0]]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, V::Uint16x4(vec![[0, 0, 0, 0]]));
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![[0.25, 0.0, 0.0, 0.0]]);
        mesh.insert_attribute(ATTRIBUTE_SECONDARY_JOINTS, V::Uint16x4(vec![[1, 0, 0, 0]]));
        mesh.insert_attribute(ATTRIBUTE_SECONDARY_WEIGHTS, vec![[0.75, 0.0, 0.0, 0.0]]);
        assert_eq!(
            skin_vertex_positions(
                &mesh,
                &[Mat4::IDENTITY, Mat4::from_translation(Vec3::X * 4.0)]
            )
            .unwrap(),
            vec![Vec3::new(4.0, 2.0, 3.0)]
        );
        assert_eq!(
            complete_skin_bounds(&mesh)
                .unwrap()
                .aabb_index_to_joint_index
                .len(),
            2
        );
        mesh.remove_attribute(ATTRIBUTE_SECONDARY_WEIGHTS);
        assert!(skin_vertex_positions(&mesh, &[Mat4::IDENTITY]).is_err());
        assert!(complete_skin_bounds(&mesh).is_err());
    }
}
