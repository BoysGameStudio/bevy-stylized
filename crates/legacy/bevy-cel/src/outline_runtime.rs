use bevy::{
    camera::primitives::Aabb,
    mesh::{
        Mesh3d,
        morph::{MeshMorphWeights, MorphWeights},
        skinning::SkinnedMesh,
    },
    pbr::MeshMaterial3d,
    prelude::{Added, ChildOf, Commands, Component, Entity, Handle, Query, Transform, Without},
};

use crate::{InternalEdgeMaterial, OutlineMaterial};

/// Ordering seam for source-mesh replacement before outline replicas spawn.
#[derive(bevy::prelude::SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CharacterOutlineSystems;

/// Outline passes attached to one source mesh without copying its geometry asset.
#[derive(Clone, Debug, Component)]
pub struct CharacterOutline {
    hull: Handle<OutlineMaterial>,
    internal_edge: Option<Handle<InternalEdgeMaterial>>,
}

impl CharacterOutline {
    /// Creates an inverted-hull pass using the source entity's mesh and deformation state.
    #[must_use]
    pub const fn new(hull: Handle<OutlineMaterial>) -> Self {
        Self {
            hull,
            internal_edge: None,
        }
    }

    /// Adds a separate optional depth/normal-prepass internal-edge pass.
    #[must_use]
    pub fn with_internal_edge(mut self, material: Handle<InternalEdgeMaterial>) -> Self {
        self.internal_edge = Some(material);
        self
    }

    /// Returns the stable inverted-hull material handle.
    #[must_use]
    pub const fn hull(&self) -> &Handle<OutlineMaterial> {
        &self.hull
    }
}

/// Renderer-owned link; dynamic geometry/bounds remain owned by the source.
#[derive(Component)]
pub(crate) struct OutlineReplica(Entity);

#[derive(Component)]
#[expect(
    clippy::redundant_pub_crate,
    reason = "the sibling plugin registers a system whose signature contains this marker"
)]
pub(crate) struct DirectMorphReplica(Entity);

type OutlineSource<'a> = (
    Entity,
    &'a Mesh3d,
    &'a CharacterOutline,
    Option<&'a SkinnedMesh>,
    Option<&'a MorphWeights>,
    Option<&'a MeshMorphWeights>,
);

#[expect(
    clippy::redundant_pub_crate,
    reason = "the sibling plugin module registers this private runtime system"
)]
pub(crate) fn spawn_outline_replicas(
    mut commands: Commands,
    sources: Query<OutlineSource<'static>, Added<CharacterOutline>>,
) {
    for (source, mesh, outline, skin, morph, morph_reference) in &sources {
        spawn_replica(
            &mut commands,
            ReplicaInput {
                source,
                mesh,
                skin,
                morph,
                morph_reference,
            },
            MeshMaterial3d(outline.hull.clone()),
        );
        if let Some(material) = outline.internal_edge.as_ref() {
            spawn_replica(
                &mut commands,
                ReplicaInput {
                    source,
                    mesh,
                    skin,
                    morph,
                    morph_reference,
                },
                MeshMaterial3d(material.clone()),
            );
        }
    }
}

#[derive(Clone, Copy)]
struct ReplicaInput<'a> {
    source: Entity,
    mesh: &'a Mesh3d,
    skin: Option<&'a SkinnedMesh>,
    morph: Option<&'a MorphWeights>,
    morph_reference: Option<&'a MeshMorphWeights>,
}

fn spawn_replica<M: bevy::pbr::Material>(
    commands: &mut Commands,
    input: ReplicaInput<'_>,
    material: MeshMaterial3d<M>,
) {
    let mut replica = commands.spawn((
        input.mesh.clone(),
        OutlineReplica(input.source),
        material,
        Transform::IDENTITY,
        ChildOf(input.source),
    ));
    if let Some(skin) = input.skin {
        replica.insert(skin.clone());
    }
    if let Some(morph_reference) = input.morph_reference {
        replica.insert(morph_reference.clone());
    }
    if let Some(morph) = input.morph {
        replica.insert((morph.clone(), DirectMorphReplica(input.source)));
    }
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "the sibling plugin module registers this private runtime system"
)]
pub(crate) fn sync_direct_morph_replicas(
    sources: Query<&MorphWeights, Without<DirectMorphReplica>>,
    mut replicas: Query<(&DirectMorphReplica, &mut MorphWeights)>,
) {
    for (replica, mut weights) in &mut replicas {
        let Ok(source) = sources.get(replica.0) else {
            continue;
        };
        weights.weights_mut().copy_from_slice(source.weights());
    }
}

/// Follow source mesh replacement and deformation bounds before visibility.
/// This applies to any dynamic source and has no dependency on a cloth backend.
pub(crate) fn sync_outline_sources(
    mut commands: Commands,
    sources: Query<(&Mesh3d, Option<&Aabb>), Without<OutlineReplica>>,
    mut replicas: Query<(Entity, &OutlineReplica, &mut Mesh3d, Option<&mut Aabb>)>,
) {
    for (entity, replica, mut mesh, bounds) in &mut replicas {
        let Ok((source_mesh, source_bounds)) = sources.get(replica.0) else {
            continue;
        };
        if mesh.0 != source_mesh.0 {
            *mesh = source_mesh.clone();
        }
        if let Some(source) = source_bounds {
            if let Some(mut bounds) = bounds {
                if bounds.center != source.center || bounds.half_extents != source.half_extents {
                    *bounds = *source;
                }
            } else {
                commands.entity(entity).insert(*source);
            }
        }
    }
}
