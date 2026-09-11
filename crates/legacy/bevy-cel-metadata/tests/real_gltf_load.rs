//! Real Bevy loader coverage for Task-4 custom vertex attributes.

#![cfg(feature = "gltf")]

use std::{io, path::Path, thread};

use bevy::{
    asset::{AssetMetaCheck, LoadState},
    gltf::{GltfAssetLabel, GltfPlugin},
    mesh::VertexAttributeValues,
    prelude::*,
};
use bevy_gltf_metadata::{ATTRIBUTE_CEL_HAIR_TANGENT, ATTRIBUTE_CEL_MASK, CelGltfPluginExt};

fn build_loader_app() -> App {
    let asset_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::transform::TransformPlugin)
        .add_plugins(AssetPlugin {
            file_path: asset_root.to_string_lossy().into_owned(),
            meta_check: AssetMetaCheck::Never,
            ..default()
        })
        .add_plugins(bevy::world_serialization::WorldSerializationPlugin)
        .add_plugins(bevy::scene::ScenePlugin)
        .add_plugins(bevy::image::ImagePlugin::default())
        .add_plugins(bevy::mesh::MeshPlugin)
        .add_plugins(bevy::animation::AnimationPlugin)
        .init_asset::<StandardMaterial>()
        .add_plugins(GltfPlugin::default().add_cel_vertex_attributes());
    app.finish();
    app.cleanup();
    app
}

fn load_mesh(app: &mut App, asset: &'static str) -> io::Result<Handle<Mesh>> {
    let handle = app.world().resource::<AssetServer>().load(
        GltfAssetLabel::Primitive {
            mesh: 0,
            primitive: 0,
        }
        .from_asset(asset),
    );
    for _ in 0..2_000 {
        app.update();
        match app
            .world()
            .resource::<AssetServer>()
            .get_load_state(&handle)
        {
            Some(LoadState::Loaded) => return Ok(handle),
            Some(LoadState::Failed(error)) => {
                return Err(io::Error::other(format!(
                    "mesh asset failed to load: {asset}: {error}"
                )));
            }
            Some(LoadState::NotLoaded | LoadState::Loading) | None => thread::yield_now(),
        }
    }
    Err(io::Error::other(format!(
        "mesh asset did not load: {asset}"
    )))
}

#[test]
fn task_4_glbs_load_canonical_scalar_and_vector_cel_attributes() -> io::Result<()> {
    // Given
    let mut app = build_loader_app();

    // When
    let toon = load_mesh(&mut app, "test/toon_sphere.glb")?;
    let hair = load_mesh(&mut app, "test/hair_tangent_strip.glb")?;

    // Then
    let meshes = app.world().resource::<Assets<Mesh>>();
    let toon = meshes
        .get(&toon)
        .ok_or_else(|| io::Error::other("toon mesh handle was loaded without a Mesh asset"))?;
    let hair = meshes
        .get(&hair)
        .ok_or_else(|| io::Error::other("hair mesh handle was loaded without a Mesh asset"))?;
    assert!(toon.contains_attribute(ATTRIBUTE_CEL_MASK));
    assert!(hair.contains_attribute(ATTRIBUTE_CEL_HAIR_TANGENT));
    assert!(matches!(
        toon.attribute(ATTRIBUTE_CEL_MASK),
        Some(VertexAttributeValues::Float32(_))
    ));
    assert!(matches!(
        hair.attribute(ATTRIBUTE_CEL_HAIR_TANGENT),
        Some(VertexAttributeValues::Float32x3(_))
    ));
    Ok(())
}
