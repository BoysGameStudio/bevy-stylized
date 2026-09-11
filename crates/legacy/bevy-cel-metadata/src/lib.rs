//! Reusable, versioned CEL metadata integration for Bevy glTF assets.

mod error;
mod schema;
mod source;

#[cfg(all(feature = "gltf", feature = "reflect"))]
mod validation;
#[cfg(all(feature = "gltf", feature = "reflect"))]
mod validation_diagnostic;
#[cfg(all(feature = "gltf", feature = "reflect"))]
mod validation_geometry;
#[cfg(all(feature = "gltf", feature = "reflect"))]
mod validation_gltf;
#[cfg(all(feature = "gltf", feature = "reflect"))]
mod validation_input;
#[cfg(all(feature = "gltf", feature = "reflect"))]
mod validation_messages;
#[cfg(all(feature = "gltf", feature = "reflect"))]
mod validation_metadata;
#[cfg(all(feature = "gltf", feature = "reflect"))]
mod validation_metadata_error;
#[cfg(all(feature = "gltf", feature = "reflect"))]
mod validation_resources;

#[cfg(feature = "gltf")]
mod attributes;
#[cfg(feature = "gltf")]
mod gltf;
#[cfg(feature = "reflect")]
mod reflection;

#[cfg(feature = "gltf")]
pub use attributes::{
    ATTRIBUTE_CEL_CLOTH_PIN, ATTRIBUTE_CEL_EMISSION, ATTRIBUTE_CEL_FACE_SDF,
    ATTRIBUTE_CEL_FACE_SIDE, ATTRIBUTE_CEL_HAIR_MASK, ATTRIBUTE_CEL_HAIR_TANGENT,
    ATTRIBUTE_CEL_JOINTS_1, ATTRIBUTE_CEL_LEVEL_ROLE, ATTRIBUTE_CEL_MASK,
    ATTRIBUTE_CEL_MATERIAL_ID, ATTRIBUTE_CEL_WEIGHTS_1, CEL_VERTEX_ATTRIBUTES, CelGltfPluginExt,
    CelVertexAttributeRegistration,
};
pub use error::{MetadataDiagnostic, MetadataDiagnosticKind, MetadataError};
#[cfg(feature = "gltf")]
pub use gltf::{CEL_METADATA_EXTENSION, CelMetadataExtensionHandler, CelMetadataPlugin};
#[cfg(feature = "reflect")]
pub use reflection::{ReflectionInsertContext, insert_reflected_components};
pub use schema::{
    CEL_SCHEMA_VERSION, CelDomainMetadata, CelMetadataEnvelope, CelNodeMetadata,
    CelReferenceDeclarations, CelSchemaVersion,
};
pub use source::{
    MetadataInput, MetadataSource, ResolvedMetadata, decode_metadata, load_sidecar,
    resolve_metadata,
};
#[cfg(all(feature = "gltf", feature = "reflect"))]
pub use validation::validate_assets;
#[cfg(all(feature = "gltf", feature = "reflect"))]
pub use validation_diagnostic::{
    VALIDATION_OUTPUT_SCHEMA_VERSION, ValidationCode, ValidationDiagnostic, ValidationReport,
    ValidationSeverity,
};
