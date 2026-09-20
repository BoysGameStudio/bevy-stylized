//! Current optional second skin-influence set, shared with render passes.
use bevy::{gltf::GltfPlugin, mesh::{MeshVertexAttribute, VertexFormat}};

pub const ATTRIBUTE_SECONDARY_JOINTS: MeshVertexAttribute = MeshVertexAttribute::new(
    "_NPR_JOINTS_1",
    0x4345_4c00_0000_000a,
    VertexFormat::Uint16x4,
);
pub const ATTRIBUTE_SECONDARY_WEIGHTS: MeshVertexAttribute = MeshVertexAttribute::new(
    "_NPR_WEIGHTS_1",
    0x4345_4c00_0000_000b,
    VertexFormat::Float32x4,
);

/// Both attributes must be present together; renderer specialization rejects
/// an incomplete set rather than silently normalizing the remaining weights.
pub fn add_secondary_skin_attributes(plugin: GltfPlugin) -> GltfPlugin {
    plugin
        .add_custom_vertex_attribute("NPR_JOINTS_1", ATTRIBUTE_SECONDARY_JOINTS)
        .add_custom_vertex_attribute("NPR_WEIGHTS_1", ATTRIBUTE_SECONDARY_WEIGHTS)
}
