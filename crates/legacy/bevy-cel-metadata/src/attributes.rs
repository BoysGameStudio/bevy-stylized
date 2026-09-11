use bevy::{gltf::GltfPlugin, mesh::MeshVertexAttribute, render::render_resource::VertexFormat};

const CEL_ATTRIBUTE_ID_BASE: u64 = 0x4345_4c00_0000_0000;

/// Exact alias of glTF JOINTS_1, which Bevy's stock loader does not import.
pub const ATTRIBUTE_CEL_JOINTS_1: MeshVertexAttribute = MeshVertexAttribute::new(
    "_CEL_JOINTS_1",
    CEL_ATTRIBUTE_ID_BASE + 10,
    VertexFormat::Uint16x4,
);
/// Exact alias of glTF WEIGHTS_1; weights remain normalized across both sets.
pub const ATTRIBUTE_CEL_WEIGHTS_1: MeshVertexAttribute = MeshVertexAttribute::new(
    "_CEL_WEIGHTS_1",
    CEL_ATTRIBUTE_ID_BASE + 11,
    VertexFormat::Float32x4,
);

/// General scalar cel-shading mask.
pub const ATTRIBUTE_CEL_MASK: MeshVertexAttribute = MeshVertexAttribute::new(
    "_CEL_MASK",
    CEL_ATTRIBUTE_ID_BASE + 1,
    VertexFormat::Float32,
);
/// Per-vertex cloth pin weight.
pub const ATTRIBUTE_CEL_CLOTH_PIN: MeshVertexAttribute = MeshVertexAttribute::new(
    "_CEL_CLOTH_PIN",
    CEL_ATTRIBUTE_ID_BASE + 2,
    VertexFormat::Float32,
);
/// Per-vertex emissive contribution.
pub const ATTRIBUTE_CEL_EMISSION: MeshVertexAttribute = MeshVertexAttribute::new(
    "_CEL_EMISSION",
    CEL_ATTRIBUTE_ID_BASE + 3,
    VertexFormat::Float32,
);
/// Face signed-distance sample.
pub const ATTRIBUTE_CEL_FACE_SDF: MeshVertexAttribute = MeshVertexAttribute::new(
    "_CEL_FACE_SDF",
    CEL_ATTRIBUTE_ID_BASE + 4,
    VertexFormat::Float32,
);
/// Face basis side sign.
pub const ATTRIBUTE_CEL_FACE_SIDE: MeshVertexAttribute = MeshVertexAttribute::new(
    "_CEL_FACE_SIDE",
    CEL_ATTRIBUTE_ID_BASE + 5,
    VertexFormat::Float32,
);
/// Hair highlight mask.
pub const ATTRIBUTE_CEL_HAIR_MASK: MeshVertexAttribute = MeshVertexAttribute::new(
    "_CEL_HAIR_MASK",
    CEL_ATTRIBUTE_ID_BASE + 6,
    VertexFormat::Float32,
);
/// Object-space hair strand direction.
pub const ATTRIBUTE_CEL_HAIR_TANGENT: MeshVertexAttribute = MeshVertexAttribute::new(
    "_CEL_HAIR_TANGENT",
    CEL_ATTRIBUTE_ID_BASE + 7,
    VertexFormat::Float32x3,
);
/// Integer-valued material region encoded as a scalar.
pub const ATTRIBUTE_CEL_MATERIAL_ID: MeshVertexAttribute = MeshVertexAttribute::new(
    "_CEL_MATERIAL_ID",
    CEL_ATTRIBUTE_ID_BASE + 8,
    VertexFormat::Float32,
);
/// Level surface role identifier encoded as a scalar.
pub const ATTRIBUTE_CEL_LEVEL_ROLE: MeshVertexAttribute = MeshVertexAttribute::new(
    "_CEL_LEVEL_ROLE",
    CEL_ATTRIBUTE_ID_BASE + 9,
    VertexFormat::Float32,
);

/// One canonical CEL attribute and its glTF provider mapping.
#[derive(Clone, Copy, Debug)]
pub struct CelVertexAttributeRegistration {
    /// Semantic written into the glTF document by authoring tools.
    pub authored_semantic: &'static str,
    /// Key produced after `gltf-json` removes the semantic's first underscore.
    pub loader_key: &'static str,
    /// Sole canonical Bevy mesh attribute for this semantic.
    pub attribute: MeshVertexAttribute,
}

/// Complete stable registration table for Task-4 `_CEL_*` fixture attributes.
pub const CEL_VERTEX_ATTRIBUTES: [CelVertexAttributeRegistration; 11] = [
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

/// Builder extension that adds every stable CEL vertex attribute to Bevy's glTF loader.
pub trait CelGltfPluginExt {
    /// Adds all CEL custom vertex attribute mappings.
    #[must_use]
    fn add_cel_vertex_attributes(self) -> Self;
}

impl CelGltfPluginExt for GltfPlugin {
    fn add_cel_vertex_attributes(mut self) -> Self {
        for registration in CEL_VERTEX_ATTRIBUTES {
            self =
                self.add_custom_vertex_attribute(registration.loader_key, registration.attribute);
        }
        self
    }
}
