use std::path::PathBuf;

use bevy::ecs::reflect::ReflectComponent;
use thiserror::Error;

use crate::MetadataSource;

/// Errors produced while loading or applying CEL metadata.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MetadataError {
    /// A sidecar could not be read in full.
    #[error("failed to read metadata from {path}: {error}")]
    Io {
        /// Sidecar path.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        error: std::io::Error,
    },
    /// JSON syntax or shape did not match the schema.
    #[error("invalid metadata from {origin} at {json_path}: {error}")]
    Json {
        /// Document containing the error.
        origin: MetadataSource,
        /// Path within that JSON document.
        json_path: String,
        /// Underlying JSON error.
        #[source]
        error: serde_json::Error,
    },
    /// The envelope uses a schema this crate does not support.
    #[error(
        "unsupported metadata schema version {found} from {origin} at {json_path}; supported version is {supported}"
    )]
    UnsupportedVersion {
        /// Document containing the version.
        origin: MetadataSource,
        /// Path to the version field.
        json_path: String,
        /// Version found in the document.
        found: u32,
        /// Version supported by this crate.
        supported: u32,
    },
    /// Neither an extension nor a sidecar was supplied.
    #[error("no CEL metadata extension or sidecar was supplied")]
    MissingMetadata,
    /// A reflected component type is absent from the registry.
    #[error("unknown reflected component type `{type_path}` from {origin} at {json_path}")]
    UnknownReflectedType {
        /// Document containing the component.
        origin: MetadataSource,
        /// Path to the component value.
        json_path: String,
        /// Unregistered full Rust type path.
        type_path: String,
    },
    /// A registered type is not registered as a reflected component.
    #[error("type `{type_path}` is not a reflected component from {origin} at {json_path}")]
    MissingReflectComponent {
        /// Document containing the component.
        origin: MetadataSource,
        /// Path to the component value.
        json_path: String,
        /// Registered full Rust type path.
        type_path: String,
    },
    /// A component value did not match its registered reflected type.
    #[error("invalid reflected component `{type_path}` from {origin} at {json_path}: {error}")]
    ReflectedValue {
        /// Document containing the component.
        origin: MetadataSource,
        /// Path to the component value.
        json_path: String,
        /// Registered full Rust type path.
        type_path: String,
        /// Underlying JSON shape error.
        #[source]
        error: serde_json::Error,
    },
}

/// Serializable diagnostic suitable for attaching to a loaded scene entity.
#[derive(bevy::prelude::Component, bevy::prelude::Reflect, Clone, Debug, PartialEq, Eq)]
#[reflect(Component)]
pub struct MetadataDiagnostic {
    /// Metadata source that failed.
    pub origin: MetadataSource,
    /// JSON path associated with the failure.
    pub json_path: String,
    /// Stable diagnostic category.
    pub kind: MetadataDiagnosticKind,
}

/// Stable categories for metadata diagnostics.
#[derive(bevy::prelude::Reflect, Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetadataDiagnosticKind {
    /// JSON syntax or shape failure.
    Json,
    /// Unsupported envelope schema version.
    UnsupportedVersion,
    /// Missing reflected type registration.
    UnknownReflectedType,
    /// Missing reflected component registration.
    MissingReflectComponent,
    /// Invalid reflected component value.
    ReflectedValue,
}

impl MetadataError {
    /// Converts entity-applicable failures into stable scene diagnostics.
    #[must_use]
    pub fn diagnostic(&self) -> Option<MetadataDiagnostic> {
        match self {
            Self::Json {
                origin, json_path, ..
            } => Some(MetadataDiagnostic {
                origin: origin.clone(),
                json_path: json_path.clone(),
                kind: MetadataDiagnosticKind::Json,
            }),
            Self::UnsupportedVersion {
                origin, json_path, ..
            } => Some(MetadataDiagnostic {
                origin: origin.clone(),
                json_path: json_path.clone(),
                kind: MetadataDiagnosticKind::UnsupportedVersion,
            }),
            Self::UnknownReflectedType {
                origin, json_path, ..
            } => Some(MetadataDiagnostic {
                origin: origin.clone(),
                json_path: json_path.clone(),
                kind: MetadataDiagnosticKind::UnknownReflectedType,
            }),
            Self::MissingReflectComponent {
                origin, json_path, ..
            } => Some(MetadataDiagnostic {
                origin: origin.clone(),
                json_path: json_path.clone(),
                kind: MetadataDiagnosticKind::MissingReflectComponent,
            }),
            Self::ReflectedValue {
                origin, json_path, ..
            } => Some(MetadataDiagnostic {
                origin: origin.clone(),
                json_path: json_path.clone(),
                kind: MetadataDiagnosticKind::ReflectedValue,
            }),
            Self::Io { .. } | Self::MissingMetadata => None,
        }
    }
}
