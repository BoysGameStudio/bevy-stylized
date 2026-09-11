use bevy::{
    camera::primitives::Aabb,
    ecs::lifecycle::RemovedComponents,
    mesh::{
        Mesh3d,
        morph::{MeshMorphWeights, MorphWeights},
        skinning::SkinnedMesh,
    },
    pbr::MeshMaterial3d,
    prelude::{Changed, ChildOf, Commands, Component, Entity, Handle, Query, Transform, Without},
};

use crate::{InternalEdgeMaterial, OutlineMaterial};

/// Ordering seam for source-mesh replacement before outline replicas spawn.
#[derive(bevy::prelude::SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CharacterOutlineSystems;

/// Outline passes attached to one source mesh without copying its geometry asset.
///
/// Runtime contract:
///
/// - `hull` and `internal_edge` are *mutable at runtime*: re-inserting a changed
///   `CharacterOutline` updates the replica `MeshMaterial3d` handles, spawns a
///   newly added internal-edge replica, and despawns a removed one.
/// - Mesh, [`Aabb`], skinning, and morph state are *followed* from the source
///   entity every frame; they are never owned by the replicas.
/// - Removing `CharacterOutline` despawns all of its replicas. Despawning the
///   source entity also despawns them through the [`ChildOf`] relationship.
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
    ///
    /// The pass may also be attached after the outline was added by re-inserting
    /// the component; the replica is then spawned late.
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

    /// Returns the optional internal-edge material handle.
    #[must_use]
    pub const fn internal_edge(&self) -> Option<&Handle<InternalEdgeMaterial>> {
        self.internal_edge.as_ref()
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

/// Spawns late replicas and follows changed material handles onto replicas.
/// `Added` also matches the `Changed` filter, so first-time spawning is covered.
#[expect(
    clippy::redundant_pub_crate,
    reason = "the sibling plugin module registers this private runtime system"
)]
pub(crate) fn sync_outline_replicas(
    mut commands: Commands,
    sources: Query<OutlineSource<'static>, Changed<CharacterOutline>>,
    mut hull_replicas: Query<(&OutlineReplica, &mut MeshMaterial3d<OutlineMaterial>)>,
    mut edge_replicas: Query<(
        Entity,
        &OutlineReplica,
        &mut MeshMaterial3d<InternalEdgeMaterial>,
    )>,
) {
    for (source, mesh, outline, skin, morph, morph_reference) in &sources {
        let input = ReplicaInput {
            source,
            mesh,
            skin,
            morph,
            morph_reference,
        };
        let mut hull_found = false;
        for (replica, mut material) in &mut hull_replicas {
            if replica.0 != source {
                continue;
            }
            hull_found = true;
            if material.0.id() != outline.hull.id() {
                material.0 = outline.hull.clone();
            }
        }
        if !hull_found {
            spawn_replica(&mut commands, input, MeshMaterial3d(outline.hull.clone()));
        }
        let mut edge_found = false;
        for (entity, replica, mut material) in &mut edge_replicas {
            if replica.0 != source {
                continue;
            }
            match outline.internal_edge.as_ref() {
                Some(handle) => {
                    edge_found = true;
                    if material.0.id() != handle.id() {
                        material.0 = handle.clone();
                    }
                }
                None => commands.entity(entity).despawn(),
            }
        }
        if !edge_found && let Some(material) = outline.internal_edge.as_ref() {
            spawn_replica(&mut commands, input, MeshMaterial3d(material.clone()));
        }
    }
}

/// Despawns outline replicas when the source drops its `CharacterOutline`.
#[expect(
    clippy::redundant_pub_crate,
    reason = "the sibling plugin module registers this private runtime system"
)]
pub(crate) fn despawn_outline_replicas(
    mut commands: Commands,
    mut removed: RemovedComponents<CharacterOutline>,
    replicas: Query<(Entity, &OutlineReplica)>,
) {
    for source in removed.read() {
        for (entity, replica) in &replicas {
            if replica.0 == source {
                commands.entity(entity).despawn();
            }
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
