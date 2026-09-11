#![expect(
    clippy::redundant_pub_crate,
    reason = "validator internals are shared across private sibling modules"
)]

use std::path::{Path, PathBuf};

use crate::{MetadataError, MetadataSource, ValidationCode, ValidationDiagnostic};

pub(crate) fn metadata_error(error: MetadataError) -> ValidationDiagnostic {
    match error {
        MetadataError::Io { path, .. } => {
            ValidationDiagnostic::error(ValidationCode::InputUnreadable, &path, "$file")
        }
        MetadataError::Json {
            origin, json_path, ..
        } => {
            let code = if json_path.contains(".references.bones") {
                ValidationCode::BoneReferenceMalformed
            } else if json_path.contains(".references.prefabs") {
                ValidationCode::PrefabReferenceMalformed
            } else if json_path.contains(".references.colliders") {
                ValidationCode::ColliderReferenceMalformed
            } else {
                ValidationCode::MetadataJsonInvalid
            };
            ValidationDiagnostic::error(code, &source_path(&origin), json_path)
        }
        MetadataError::UnsupportedVersion {
            origin, json_path, ..
        } => ValidationDiagnostic::error(
            ValidationCode::MetadataVersionUnsupported,
            &source_path(&origin),
            json_path,
        ),
        MetadataError::MissingMetadata => ValidationDiagnostic::error(
            ValidationCode::MetadataJsonInvalid,
            Path::new("<metadata>"),
            "$",
        ),
        MetadataError::UnknownReflectedType {
            origin, json_path, ..
        } => ValidationDiagnostic::error(
            ValidationCode::ComponentTypeUnknown,
            &source_path(&origin),
            json_path,
        ),
        MetadataError::MissingReflectComponent {
            origin, json_path, ..
        } => ValidationDiagnostic::error(
            ValidationCode::ComponentRegistrationInvalid,
            &source_path(&origin),
            json_path,
        ),
        MetadataError::ReflectedValue {
            origin, json_path, ..
        } => ValidationDiagnostic::error(
            ValidationCode::ComponentValueInvalid,
            &source_path(&origin),
            json_path,
        ),
    }
}

fn source_path(source: &MetadataSource) -> PathBuf {
    match source {
        MetadataSource::Extension { asset, .. } | MetadataSource::Sidecar(asset) => asset.clone(),
    }
}
