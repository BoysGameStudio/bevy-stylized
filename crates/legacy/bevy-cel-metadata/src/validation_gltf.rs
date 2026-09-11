#![expect(
    clippy::redundant_pub_crate,
    reason = "validator internals are shared across private sibling modules"
)]

use std::{collections::BTreeSet, path::Path};

use serde_json::{Map, Value};

use crate::{ValidationCode, ValidationDiagnostic};

use crate::{
    validation_geometry::{collect_node_names, validate_attributes, validate_skins},
    validation_input::ParsedInput,
    validation_resources::{ResourceValidation, validate_file_references},
};

pub(crate) fn validate_gltf(
    path: &Path,
    input: &ParsedInput,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) -> BTreeSet<String> {
    let Some(root) = input.document.as_object() else {
        diagnostics.push(ValidationDiagnostic::error(
            ValidationCode::JsonShapeInvalid,
            path,
            "$",
        ));
        return BTreeSet::new();
    };
    validate_version(path, root, diagnostics);
    validate_file_references(
        root,
        ResourceValidation {
            path,
            input,
            diagnostics,
        },
    );
    validate_attributes(path, root, diagnostics);
    let node_names = collect_node_names(root);
    validate_skins(path, root, diagnostics);
    node_names
}

fn validate_version(
    path: &Path,
    root: &Map<String, Value>,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    if root
        .get("asset")
        .and_then(Value::as_object)
        .and_then(|asset| asset.get("version"))
        .and_then(Value::as_str)
        != Some("2.0")
    {
        diagnostics.push(ValidationDiagnostic::error(
            ValidationCode::GltfVersionUnsupported,
            path,
            "$.asset.version",
        ));
    }
}
