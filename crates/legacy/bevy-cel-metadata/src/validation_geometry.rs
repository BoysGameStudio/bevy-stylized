#![expect(
    clippy::redundant_pub_crate,
    reason = "validator internals are shared across private sibling modules"
)]

use std::{collections::BTreeSet, path::Path};

use bevy::render::render_resource::VertexFormat;
use serde_json::{Map, Value};

use crate::{CEL_VERTEX_ATTRIBUTES, ValidationCode, ValidationDiagnostic};

pub(crate) fn validate_attributes(
    path: &Path,
    root: &Map<String, Value>,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    let accessors = root.get("accessors").and_then(Value::as_array);
    let Some(meshes) = root.get("meshes").and_then(Value::as_array) else {
        return;
    };
    for (mesh_index, mesh) in meshes.iter().enumerate() {
        let Some(primitives) = mesh
            .as_object()
            .and_then(|object| object.get("primitives"))
            .and_then(Value::as_array)
        else {
            continue;
        };
        for (primitive_index, primitive) in primitives.iter().enumerate() {
            let Some(attributes) = primitive
                .as_object()
                .and_then(|object| object.get("attributes"))
                .and_then(Value::as_object)
            else {
                continue;
            };
            for (semantic, accessor) in attributes {
                if semantic.starts_with("_CEL_") {
                    let location = format!(
                        "$.meshes[{mesh_index}].primitives[{primitive_index}].attributes.{semantic}"
                    );
                    validate_attribute(
                        &AttributeContext {
                            path,
                            accessors,
                            semantic,
                            location: &location,
                        },
                        accessor,
                        diagnostics,
                    );
                }
            }
        }
    }
}

struct AttributeContext<'a> {
    path: &'a Path,
    accessors: Option<&'a Vec<Value>>,
    semantic: &'a str,
    location: &'a str,
}

fn validate_attribute(
    context: &AttributeContext<'_>,
    accessor: &Value,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    let Some(registration) = CEL_VERTEX_ATTRIBUTES
        .iter()
        .find(|item| item.authored_semantic == context.semantic)
    else {
        diagnostics.push(ValidationDiagnostic::error(
            ValidationCode::CelAttributeUnknown,
            context.path,
            context.location,
        ));
        return;
    };
    let Some(index) = accessor
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
    else {
        diagnostics.push(ValidationDiagnostic::error(
            ValidationCode::CelAttributeAccessorMalformed,
            context.path,
            context.location,
        ));
        return;
    };
    let Some(accessor) = context
        .accessors
        .and_then(|values| values.get(index))
        .and_then(Value::as_object)
    else {
        diagnostics.push(ValidationDiagnostic::error(
            ValidationCode::CelAttributeAccessorUnknown,
            context.path,
            context.location,
        ));
        return;
    };
    let (expected_type, expected_component) = match registration.attribute.format {
        VertexFormat::Float32x3 => ("VEC3", 5126),
        VertexFormat::Float32x4 => ("VEC4", 5126),
        VertexFormat::Uint16x4 => ("VEC4", 5123),
        _ => ("SCALAR", 5126),
    };
    if accessor.get("componentType").and_then(Value::as_u64) != Some(expected_component)
        || accessor.get("type").and_then(Value::as_str) != Some(expected_type)
    {
        diagnostics.push(ValidationDiagnostic::error(
            ValidationCode::CelAttributeFormatInvalid,
            context.path,
            format!("$.accessors[{index}]"),
        ));
    }
}

pub(crate) fn collect_node_names(root: &Map<String, Value>) -> BTreeSet<String> {
    root.get("nodes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .filter_map(|node| node.get("name"))
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

pub(crate) fn validate_skins(
    path: &Path,
    root: &Map<String, Value>,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    let node_count = root
        .get("nodes")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let Some(skins) = root.get("skins").and_then(Value::as_array) else {
        return;
    };
    for (skin_index, skin) in skins.iter().enumerate() {
        let Some(joints) = skin
            .as_object()
            .and_then(|object| object.get("joints"))
            .and_then(Value::as_array)
        else {
            diagnostics.push(ValidationDiagnostic::error(
                ValidationCode::BoneReferenceMalformed,
                path,
                format!("$.skins[{skin_index}].joints"),
            ));
            continue;
        };
        for (joint_index, joint) in joints.iter().enumerate() {
            let location = format!("$.skins[{skin_index}].joints[{joint_index}]");
            match joint.as_u64().and_then(|value| usize::try_from(value).ok()) {
                None => diagnostics.push(ValidationDiagnostic::error(
                    ValidationCode::BoneReferenceMalformed,
                    path,
                    location,
                )),
                Some(index) if index >= node_count => diagnostics.push(
                    ValidationDiagnostic::error(ValidationCode::BoneTargetUnknown, path, location),
                ),
                Some(_) => {}
            }
        }
    }
}
