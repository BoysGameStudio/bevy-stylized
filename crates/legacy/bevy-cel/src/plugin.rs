use bevy::{
    asset::{AssetEvent, AssetEventSystems, Assets, embedded_asset},
    ecs::message::MessageReader,
    gltf::GltfPlugin,
    material::OpaqueRendererMethod,
    pbr::MaterialPlugin,
    prelude::{
        App, GlobalTransform, IntoScheduleConfigs, MeshMaterial3d, Plugin, PostUpdate, Query,
        ResMut, StandardMaterial, TransformSystems,
    },
};
use bevy_gltf_metadata::CelGltfPluginExt;

use crate::{
    CharacterExtension, CharacterMaterial, FaceBasis, FaceSdfMaterialInstance,
    InternalEdgeMaterial, OutlineMaterial,
    outline::diagnose_missing_internal_edge_prepass,
    outline_runtime::{spawn_outline_replicas, sync_direct_morph_replicas, sync_outline_sources},
    post_process_runtime::StylizedPostProcessPlugin,
};

/// Host ordering seam for face-basis updates after custom presentation replay.
#[derive(bevy::prelude::SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CharacterFaceBasisSystems;

/// Converts a standard material into a supported forward character material.
///
/// Alpha mode, textures, and all other `StandardMaterial` behavior are retained.
#[must_use]
pub fn character_material(mut base: StandardMaterial) -> CharacterMaterial {
    base.opaque_render_method = OpaqueRendererMethod::Forward;
    CharacterMaterial {
        base,
        extension: CharacterExtension::default(),
    }
}

/// Configures Bevy's glTF loader for canonical optional character attributes.
#[must_use]
pub fn character_gltf_plugin() -> GltfPlugin {
    GltfPlugin::default().add_cel_vertex_attributes()
}

/// Registers embedded shaders, the material renderer, and face-basis updates.
#[derive(Default)]
pub struct CharacterMaterialPlugin;

impl Plugin for CharacterMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/character_forward.wgsl");
        embedded_asset!(app, "shaders/character_vertex.wgsl");
        embedded_asset!(app, "shaders/character_prepass.wgsl");
        embedded_asset!(app, "shaders/outline_vertex.wgsl");
        embedded_asset!(app, "shaders/outline_fragment.wgsl");
        embedded_asset!(app, "shaders/internal_edge.wgsl");
        embedded_asset!(app, "shaders/stylized_post_process.wgsl");
        app.add_plugins((
            MaterialPlugin::<CharacterMaterial>::default(),
            MaterialPlugin::<OutlineMaterial>::default(),
            MaterialPlugin::<InternalEdgeMaterial>::default(),
            StylizedPostProcessPlugin,
        ))
        .add_systems(
            PostUpdate,
            (
                enforce_forward_renderer.after(AssetEventSystems),
                crate::skinning::update_extended_skin_bounds
                    .run_if(
                        bevy::ecs::schedule::common_conditions::resource_exists::<
                            Assets<bevy::prelude::Mesh>,
                        >,
                    )
                    .run_if(
                        bevy::ecs::schedule::common_conditions::resource_exists::<
                            bevy::prelude::Messages<AssetEvent<bevy::prelude::Mesh>>,
                        >,
                    )
                    .after(AssetEventSystems)
                    .before(bevy::camera::visibility::VisibilitySystems::CalculateBounds),
                update_face_sdf_material_basis
                    .after(TransformSystems::Propagate)
                    .in_set(CharacterFaceBasisSystems),
                diagnose_missing_internal_edge_prepass,
                spawn_outline_replicas.in_set(crate::CharacterOutlineSystems),
                sync_direct_morph_replicas,
                sync_outline_sources
                    .after(crate::CharacterOutlineSystems)
                    .after(bevy::camera::visibility::VisibilitySystems::CalculateBounds)
                    .before(bevy::camera::visibility::VisibilitySystems::CheckVisibility),
            ),
        );
    }
}

fn enforce_forward_renderer(
    mut events: MessageReader<AssetEvent<CharacterMaterial>>,
    mut materials: ResMut<Assets<CharacterMaterial>>,
) {
    for event in events.read() {
        let id = match *event {
            AssetEvent::Added { id }
            | AssetEvent::Modified { id }
            | AssetEvent::LoadedWithDependencies { id } => id,
            AssetEvent::Removed { .. } | AssetEvent::Unused { .. } => continue,
        };
        if let Some(mut material) = materials.get_mut(id)
            && material.base.opaque_render_method != OpaqueRendererMethod::Forward
        {
            material.base.opaque_render_method = OpaqueRendererMethod::Forward;
        }
    }
}

fn update_face_sdf_material_basis(
    instances: Query<(&FaceSdfMaterialInstance, &MeshMaterial3d<CharacterMaterial>)>,
    heads: Query<&GlobalTransform>,
    mut materials: ResMut<Assets<CharacterMaterial>>,
) {
    for (instance, assigned_material) in &instances {
        if assigned_material.0.id() != instance.material().id() {
            continue;
        }
        let Some(mut material) = materials.get_mut(instance.material()) else {
            continue;
        };
        let basis = heads.get(instance.head()).ok().and_then(|head| {
            FaceBasis::try_new(
                head.affine()
                    .transform_vector3(instance.local_basis().forward()),
                head.affine()
                    .transform_vector3(instance.local_basis().right()),
            )
            .ok()
        });
        let settings = *material.extension.face_sdf_settings();
        material
            .extension
            .set_face_sdf_settings(settings.with_basis(basis));
    }
}
