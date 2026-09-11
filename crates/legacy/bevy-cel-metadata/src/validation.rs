use std::path::PathBuf;

use bevy::reflect::TypeRegistry;

use crate::{ValidationDiagnostic, ValidationReport};

use crate::{
    validation_gltf::validate_gltf,
    validation_input::{AssetKind, parse_input},
    validation_metadata::{
        MetadataValidation, SidecarValidation, validate_metadata, validate_sidecar,
    },
};

/// Validates assets without modifying input files.
#[must_use]
pub fn validate_assets(paths: &[PathBuf], registry: &TypeRegistry) -> ValidationReport {
    let mut sorted_paths = paths.to_vec();
    sorted_paths.sort();
    let mut diagnostics = Vec::<ValidationDiagnostic>::new();
    for path in &sorted_paths {
        let Some(input) = parse_input(path, &mut diagnostics) else {
            continue;
        };
        match input.kind {
            AssetKind::Sidecar => {
                validate_sidecar(
                    &input.document,
                    &mut SidecarValidation {
                        path,
                        registry,
                        diagnostics: &mut diagnostics,
                    },
                );
            }
            AssetKind::Glb | AssetKind::Gltf => {
                let node_names = validate_gltf(path, &input, &mut diagnostics);
                validate_metadata(
                    &input.document,
                    MetadataValidation {
                        asset_path: path,
                        node_names: &node_names,
                        registry,
                        diagnostics: &mut diagnostics,
                    },
                );
            }
        }
    }
    ValidationReport::from_parts(&sorted_paths, diagnostics)
}
