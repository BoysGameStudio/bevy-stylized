//! Bevy glTF and reflection integration contract tests.

#![cfg(all(feature = "gltf", feature = "reflect"))]

use bevy::{
    app::App,
    asset::AssetPlugin,
    ecs::reflect::{AppTypeRegistry, ReflectComponent},
    gltf::{GltfPlugin, extensions::GltfExtensionHandlers},
    mesh::MeshVertexAttribute,
    prelude::{Component, Reflect},
};
use bevy_gltf_metadata::{
    ATTRIBUTE_CEL_CLOTH_PIN, ATTRIBUTE_CEL_EMISSION, ATTRIBUTE_CEL_FACE_SDF,
    ATTRIBUTE_CEL_FACE_SIDE, ATTRIBUTE_CEL_HAIR_MASK, ATTRIBUTE_CEL_HAIR_TANGENT,
    ATTRIBUTE_CEL_JOINTS_1, ATTRIBUTE_CEL_LEVEL_ROLE, ATTRIBUTE_CEL_MASK,
    ATTRIBUTE_CEL_MATERIAL_ID, ATTRIBUTE_CEL_WEIGHTS_1, CEL_VERTEX_ATTRIBUTES, CelGltfPluginExt,
    CelMetadataPlugin, CelNodeMetadata, CelVertexAttributeRegistration, MetadataSource,
    ReflectionInsertContext, insert_reflected_components,
};

#[derive(Component, Reflect, Debug, PartialEq)]
#[reflect(Component)]
struct TestHealth {
    value: u32,
}

#[test]
fn custom_attribute_registration_is_stable_and_complete() {
    // Given
    let expected: [CelVertexAttributeRegistration; 11] = [
        CelVertexAttributeRegistration {
            authored_semantic: "_CEL_MASK",
            loader_key: "CEL_MASK",
            attribute: ATTRIBUTE_CEL_MASK,
        },
        CelVertexAttributeRegistration {
            authored_semantic: "_CEL_CLOTH_PIN",
            loader_key: "CEL_CLOTH_PIN",
            attribute: ATTRIBUTE_CEL_CLOTH_PIN,
        },
        CelVertexAttributeRegistration {
            authored_semantic: "_CEL_EMISSION",
            loader_key: "CEL_EMISSION",
            attribute: ATTRIBUTE_CEL_EMISSION,
        },
        CelVertexAttributeRegistration {
            authored_semantic: "_CEL_FACE_SDF",
            loader_key: "CEL_FACE_SDF",
            attribute: ATTRIBUTE_CEL_FACE_SDF,
        },
        CelVertexAttributeRegistration {
            authored_semantic: "_CEL_FACE_SIDE",
            loader_key: "CEL_FACE_SIDE",
            attribute: ATTRIBUTE_CEL_FACE_SIDE,
        },
        CelVertexAttributeRegistration {
            authored_semantic: "_CEL_HAIR_MASK",
            loader_key: "CEL_HAIR_MASK",
            attribute: ATTRIBUTE_CEL_HAIR_MASK,
        },
        CelVertexAttributeRegistration {
            authored_semantic: "_CEL_HAIR_TANGENT",
            loader_key: "CEL_HAIR_TANGENT",
            attribute: ATTRIBUTE_CEL_HAIR_TANGENT,
        },
        CelVertexAttributeRegistration {
            authored_semantic: "_CEL_MATERIAL_ID",
            loader_key: "CEL_MATERIAL_ID",
            attribute: ATTRIBUTE_CEL_MATERIAL_ID,
        },
        CelVertexAttributeRegistration {
            authored_semantic: "_CEL_LEVEL_ROLE",
            loader_key: "CEL_LEVEL_ROLE",
            attribute: ATTRIBUTE_CEL_LEVEL_ROLE,
        },
        CelVertexAttributeRegistration {
            authored_semantic: "_CEL_JOINTS_1",
            loader_key: "CEL_JOINTS_1",
            attribute: ATTRIBUTE_CEL_JOINTS_1,
        },
        CelVertexAttributeRegistration {
            authored_semantic: "_CEL_WEIGHTS_1",
            loader_key: "CEL_WEIGHTS_1",
            attribute: ATTRIBUTE_CEL_WEIGHTS_1,
        },
    ];
    let stable_ids = [
        0x4345_4c00_0000_0001,
        0x4345_4c00_0000_0002,
        0x4345_4c00_0000_0003,
        0x4345_4c00_0000_0004,
        0x4345_4c00_0000_0005,
        0x4345_4c00_0000_0006,
        0x4345_4c00_0000_0007,
        0x4345_4c00_0000_0008,
        0x4345_4c00_0000_0009,
        0x4345_4c00_0000_000a,
        0x4345_4c00_0000_000b,
    ];

    // When
    let plugin = bevy::gltf::GltfPlugin::default().add_cel_vertex_attributes();

    // Then
    assert_eq!(plugin.custom_vertex_attributes.len(), expected.len());
    for ((registration, expected), stable_id) in CEL_VERTEX_ATTRIBUTES
        .into_iter()
        .zip(expected)
        .zip(stable_ids)
    {
        assert_eq!(registration.authored_semantic, expected.authored_semantic);
        assert_eq!(registration.loader_key, expected.loader_key);
        assert_eq!(registration.attribute, expected.attribute);
        assert_eq!(
            registration.authored_semantic.strip_prefix('_'),
            Some(registration.loader_key)
        );
        assert_eq!(
            plugin.custom_vertex_attributes[registration.loader_key],
            registration.attribute
        );
        assert!(
            !plugin
                .custom_vertex_attributes
                .contains_key(registration.authored_semantic)
        );
        assert!(registration.attribute.name.starts_with("_CEL_"));
        assert_eq!(
            registration.attribute.id,
            MeshVertexAttribute::new("stable-id-proof", stable_id, registration.attribute.format)
                .id
        );
    }
}

#[test]
fn reflected_component_hook_inserts_registered_value_into_world()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let mut app = App::new();
    app.register_type::<TestHealth>();
    let type_path = std::any::type_name::<TestHealth>();
    let metadata: CelNodeMetadata = serde_json::from_value(serde_json::json!({
        "components": {type_path: {"value": 42}}
    }))?;
    let registry = app.world().resource::<AppTypeRegistry>().clone();
    let entity_id = app.world_mut().spawn_empty().id();

    // When
    let mut entity = app.world_mut().entity_mut(entity_id);
    insert_reflected_components(
        &mut entity,
        &metadata.components,
        ReflectionInsertContext {
            registry: &registry.read(),
            origin: &MetadataSource::Sidecar("test.cel.json".into()),
            json_path: "$.nodes.Test.components",
        },
    )?;

    // Then
    assert_eq!(
        app.world().get::<TestHealth>(entity_id),
        Some(&TestHealth { value: 42 })
    );
    Ok(())
}

#[test]
fn metadata_plugin_registers_official_gltf_extension_handler() {
    // Given
    let mut app = App::new();

    // When
    app.add_plugins((
        AssetPlugin::default(),
        GltfPlugin::default(),
        CelMetadataPlugin,
    ));

    // Then
    let handlers = app.world().resource::<GltfExtensionHandlers>();
    assert_eq!(handlers.0.read_blocking().len(), 1);
}
