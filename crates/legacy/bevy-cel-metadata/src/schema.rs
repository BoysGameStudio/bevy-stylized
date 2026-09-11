use std::collections::BTreeMap;

use bevy::{
    ecs::reflect::ReflectComponent,
    prelude::Component,
    reflect::{Reflect, ReflectDeserialize, ReflectSerialize},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The single CEL metadata schema version supported by this release.
pub const CEL_SCHEMA_VERSION: u32 = 1;

/// A schema version proven to be supported.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(into = "u32")]
pub enum CelSchemaVersion {
    /// Initial CEL metadata schema.
    V1,
}

impl CelSchemaVersion {
    /// Returns the wire-format integer.
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        match self {
            Self::V1 => CEL_SCHEMA_VERSION,
        }
    }
}

impl From<CelSchemaVersion> for u32 {
    fn from(version: CelSchemaVersion) -> Self {
        version.as_u32()
    }
}

/// Metadata associated with one named glTF node.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CelNodeMetadata {
    /// Domain payloads keyed by reusable integration domain.
    #[serde(default)]
    pub domains: BTreeMap<String, Value>,
    /// Reflected component values keyed by full Rust type path.
    #[serde(default)]
    pub components: BTreeMap<String, Value>,
    /// Generic asset references declared by reusable authoring integrations.
    #[serde(default)]
    pub references: CelReferenceDeclarations,
}

/// Domain payloads retained on loaded glTF node entities as canonical JSON strings.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub struct CelDomainMetadata {
    /// Payloads keyed by reusable integration domain.
    pub domains: BTreeMap<String, String>,
}

/// Reference categories shared by character, prefab, and physics authoring.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CelReferenceDeclarations {
    /// Exact glTF node names used as bones.
    #[serde(default)]
    pub bones: Vec<String>,
    /// Relative `.glb` or `.gltf` prefab asset paths.
    #[serde(default)]
    pub prefabs: Vec<String>,
    /// Exact glTF node names used as colliders.
    #[serde(default)]
    pub colliders: Vec<String>,
}

/// Validated CEL metadata for one asset.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CelMetadataEnvelope {
    /// Validated schema version.
    pub schema_version: CelSchemaVersion,
    /// Metadata keyed by exact glTF node name.
    pub nodes: BTreeMap<String, CelNodeMetadata>,
}
