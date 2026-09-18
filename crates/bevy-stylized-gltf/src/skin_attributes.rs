//! Compatible import of an optional second glTF skin-influence set.
//! The authored CEL aliases and IDs are retained as a data protocol; no CEL
//! renderer or game dependency is required to consume them.
use bevy::{gltf::GltfPlugin, mesh::MeshVertexAttribute, render::render_resource::VertexFormat};

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

/// Both attributes must be present together; renderer specialization rejects
/// an incomplete set rather than silently normalizing the remaining weights.
pub fn add_secondary_skin_attributes(plugin: GltfPlugin) -> GltfPlugin {
    plugin
        .add_custom_vertex_attribute("CEL_JOINTS_1", ATTRIBUTE_SECONDARY_JOINTS)
        .add_custom_vertex_attribute("CEL_WEIGHTS_1", ATTRIBUTE_SECONDARY_WEIGHTS)
}
