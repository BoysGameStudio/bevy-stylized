#![expect(
    clippy::redundant_pub_crate,
    reason = "validator internals are shared across private sibling modules"
)]

use std::path::Path;

use serde_json::{Map, Value};

use crate::{
    ValidationCode, ValidationDiagnostic,
    validation_input::{AssetKind, ParsedInput},
};

pub(crate) struct ResourceValidation<'a> {
    pub(crate) path: &'a Path,
    pub(crate) input: &'a ParsedInput,
    pub(crate) diagnostics: &'a mut Vec<ValidationDiagnostic>,
}

pub(crate) fn validate_file_references(root: &Map<String, Value>, context: ResourceValidation<'_>) {
    ResourceValidator(context).validate(root);
}

struct ResourceValidator<'a>(ResourceValidation<'a>);

impl ResourceValidator<'_> {
    fn validate(&mut self, root: &Map<String, Value>) {
        self.validate_uri_section(root, "images");
        let Some(buffers) = root.get("buffers") else {
            return;
        };
        let Some(buffers) = buffers.as_array() else {
            self.push_error(ValidationCode::JsonShapeInvalid, "$.buffers");
            return;
        };
        for (index, buffer) in buffers.iter().enumerate() {
            let location = format!("$.buffers[{index}].uri");
            match buffer.as_object().and_then(|object| object.get("uri")) {
                Some(uri) => self.validate_uri(uri, &location),
                None if self.0.input.kind == AssetKind::Glb && self.0.input.has_binary_chunk => {}
                None => self.push_error(ValidationCode::BinaryChunkMissing, location),
            }
        }
    }

    fn validate_uri_section(&mut self, root: &Map<String, Value>, section: &str) {
        let Some(values) = root.get(section) else {
            return;
        };
        let Some(values) = values.as_array() else {
            self.push_error(ValidationCode::JsonShapeInvalid, format!("$.{section}"));
            return;
        };
        for (index, value) in values.iter().enumerate() {
            let Some(object) = value.as_object() else {
                self.push_error(
                    ValidationCode::JsonShapeInvalid,
                    format!("$.{section}[{index}]"),
                );
                continue;
            };
            if let Some(uri) = object.get("uri") {
                self.validate_uri(uri, &format!("$.{section}[{index}].uri"));
            } else if object.get("bufferView").and_then(Value::as_u64).is_none() {
                self.push_error(
                    ValidationCode::FileReferenceMalformed,
                    format!("$.{section}[{index}]"),
                );
            }
        }
    }

    fn validate_uri(&mut self, uri: &Value, location: &str) {
        let Some(uri) = uri.as_str() else {
            self.push_error(ValidationCode::FileReferenceMalformed, location);
            return;
        };
        if uri.starts_with("data:") {
            return;
        }
        if uri.contains("://") {
            self.0.diagnostics.push(ValidationDiagnostic::warning(
                ValidationCode::RemoteReferenceUnverified,
                self.0.path,
                location,
            ));
            return;
        }
        if !self
            .0
            .path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(uri)
            .is_file()
        {
            self.push_error(ValidationCode::FileReferenceMissing, location);
        }
    }

    fn push_error(&mut self, code: ValidationCode, location: impl Into<String>) {
        self.0
            .diagnostics
            .push(ValidationDiagnostic::error(code, self.0.path, location));
    }
}
