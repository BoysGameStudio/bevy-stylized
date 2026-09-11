//! Canonical task-5 custom attribute integration tests.

use std::{
    io,
    path::Path,
    sync::{Mutex, MutexGuard},
    thread,
};

use bevy::{
    asset::{AssetMetaCheck, LoadState},
    gltf::GltfAssetLabel,
    mesh::VertexAttributeValues,
    prelude::*,
};
use bevy_cel::{
    ATTRIBUTE_CEL_HAIR_MASK, ATTRIBUTE_CEL_HAIR_TANGENT, ATTRIBUTE_CEL_MATERIAL_ID,
    ATTRIBUTE_CHARACTER_MASK, character_gltf_plugin,
};
use bevy_gltf_metadata::ATTRIBUTE_CEL_MASK;

static LOADER_TEST_LOCK: Mutex<()> = Mutex::new(());

fn loader_test_guard() -> MutexGuard<'static, ()> {
    LOADER_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

#[test]
fn compatibility_alias_matches_canonical_mask_identity() {
    // Given
    let canonical = ATTRIBUTE_CEL_MASK;

    // When
    let compatibility = ATTRIBUTE_CHARACTER_MASK;

    // Then
    assert_eq!(compatibility.id, canonical.id);
    assert_eq!(compatibility.name, canonical.name);
    assert_eq!(compatibility.format, canonical.format);
}

#[test]
fn task_4_hair_strip_loads_canonical_tangent_and_mask() -> io::Result<()> {
    let _guard = loader_test_guard();
    // Given
    let asset_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../bevy-cel-metadata/assets");
    let mut app = App::new();
    add_loader_plugins(&mut app, &asset_root);
    app.finish();
    app.cleanup();
    let handle = app.world().resource::<AssetServer>().load(
        GltfAssetLabel::Primitive {
            mesh: 0,
            primitive: 0,
        }
        .from_asset("test/hair_tangent_strip.glb"),
    );

    // When
    for _ in 0..2_000 {
        app.update();
        match app
            .world()
            .resource::<AssetServer>()
            .get_load_state(&handle)
        {
            Some(LoadState::Loaded) => break,
            Some(LoadState::Failed(error)) => {
                return Err(io::Error::other(format!(
                    "hair fixture failed to load: {error}"
                )));
            }
            Some(LoadState::NotLoaded | LoadState::Loading) | None => thread::yield_now(),
        }
    }

    // Then
    let meshes = app.world().resource::<Assets<Mesh>>();
    let mesh = meshes
        .get(&handle)
        .ok_or_else(|| io::Error::other("hair fixture did not load within 2000 updates"))?;
    assert!(matches!(
        mesh.attribute(ATTRIBUTE_CEL_HAIR_TANGENT),
        Some(VertexAttributeValues::Float32x3(_))
    ));
    assert!(matches!(
        mesh.attribute(ATTRIBUTE_CEL_HAIR_MASK),
        Some(VertexAttributeValues::Float32(_))
    ));
    Ok(())
}

#[test]
fn task_4_ilm_mesh_loads_two_canonical_material_id_regions() -> io::Result<()> {
    let _guard = loader_test_guard();
    // Given
    let asset_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../bevy-cel-metadata/assets");
    let mut app = App::new();
    add_loader_plugins(&mut app, &asset_root);
    app.finish();
    app.cleanup();
    let first_region = app.world().resource::<AssetServer>().load(
        GltfAssetLabel::Primitive {
            mesh: 0,
            primitive: 0,
        }
        .from_asset("test/ilm_two_region_mesh.glb"),
    );
    let second_region = app.world().resource::<AssetServer>().load(
        GltfAssetLabel::Primitive {
            mesh: 0,
            primitive: 1,
        }
        .from_asset("test/ilm_two_region_mesh.glb"),
    );

    // When
    for _ in 0..2_000 {
        app.update();
        match app
            .world()
            .resource::<AssetServer>()
            .get_load_state(&second_region)
        {
            Some(LoadState::Loaded) => break,
            Some(LoadState::Failed(error)) => {
                return Err(io::Error::other(format!(
                    "ILM fixture failed to load: {error}"
                )));
            }
            Some(LoadState::NotLoaded | LoadState::Loading) | None => thread::yield_now(),
        }
    }

    // Then
    let meshes = app.world().resource::<Assets<Mesh>>();
    let first_mesh = meshes
        .get(&first_region)
        .ok_or_else(|| io::Error::other("ILM fixture did not load within 2000 updates"))?;
    let second_mesh = meshes
        .get(&second_region)
        .ok_or_else(|| io::Error::other("ILM fixture did not load within 2000 updates"))?;
    let Some(VertexAttributeValues::Float32(first_ids)) =
        first_mesh.attribute(ATTRIBUTE_CEL_MATERIAL_ID)
    else {
        return Err(io::Error::other(
            "canonical material ID attribute is absent",
        ));
    };
    let Some(VertexAttributeValues::Float32(second_ids)) =
        second_mesh.attribute(ATTRIBUTE_CEL_MATERIAL_ID)
    else {
        return Err(io::Error::other(
            "canonical material ID attribute is absent",
        ));
    };
    assert_eq!(first_ids, &[0.0, 0.0, 0.0, 0.0]);
    assert_eq!(second_ids, &[1.0, 1.0, 1.0, 1.0]);
    Ok(())
}

#[test]
fn task_4_mesh_loads_canonical_mask_through_character_plugin() -> io::Result<()> {
    let _guard = loader_test_guard();
    // Given
    let asset_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../bevy-cel-metadata/assets");
    let mut app = App::new();
    add_loader_plugins(&mut app, &asset_root);
    app.finish();
    app.cleanup();
    let handle = app.world().resource::<AssetServer>().load(
        GltfAssetLabel::Primitive {
            mesh: 0,
            primitive: 0,
        }
        .from_asset("test/toon_sphere.glb"),
    );

    // When
    for _ in 0..2_000 {
        app.update();
        match app
            .world()
            .resource::<AssetServer>()
            .get_load_state(&handle)
        {
            Some(LoadState::Loaded) => break,
            Some(LoadState::Failed(error)) => {
                return Err(io::Error::other(format!(
                    "toon sphere failed to load: {error}"
                )));
            }
            Some(LoadState::NotLoaded | LoadState::Loading) | None => thread::yield_now(),
        }
    }

    // Then
    let meshes = app.world().resource::<Assets<Mesh>>();
    let mesh = meshes
        .get(&handle)
        .ok_or_else(|| io::Error::other("toon sphere did not load within 2000 updates"))?;
    assert!(mesh.contains_attribute(ATTRIBUTE_CEL_MASK));
    assert!(matches!(
        mesh.attribute(ATTRIBUTE_CEL_MASK),
        Some(VertexAttributeValues::Float32(_))
    ));
    Ok(())
}

fn add_loader_plugins(app: &mut App, asset_root: &Path) {
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
        .add_plugins(character_gltf_plugin());
}
