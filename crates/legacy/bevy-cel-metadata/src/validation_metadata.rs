#![expect(
    clippy::redundant_pub_crate,
    reason = "validator internals are shared across private sibling modules"
)]

use std::{collections::BTreeSet, fs, path::Path};

use bevy::{ecs::reflect::ReflectComponent, reflect::TypeRegistry};
use serde::de::DeserializeSeed;
use serde_json::Value;

use crate::{
    CEL_METADATA_EXTENSION, CelNodeMetadata, MetadataInput, MetadataSource, ResolvedMetadata,
    ValidationCode, ValidationDiagnostic, resolve_metadata,
};

use crate::validation_metadata_error::metadata_error;

pub(crate) struct MetadataValidation<'a> {
    pub(crate) asset_path: &'a Path,
    pub(crate) node_names: &'a BTreeSet<String>,
    pub(crate) registry: &'a TypeRegistry,
    pub(crate) diagnostics: &'a mut Vec<ValidationDiagnostic>,
}

pub(crate) struct SidecarValidation<'a> {
    pub(crate) path: &'a Path,
    pub(crate) registry: &'a TypeRegistry,
    pub(crate) diagnostics: &'a mut Vec<ValidationDiagnostic>,
}

pub(crate) fn validate_metadata(document: &Value, context: MetadataValidation<'_>) {
    let extension_bytes = document
        .get("extensions")
        .and_then(Value::as_object)
        .and_then(|extensions| extensions.get(CEL_METADATA_EXTENSION))
        .and_then(|value| serde_json::to_vec(value).ok());
    let sidecar_path = context.asset_path.with_extension("cel.json");
    let sidecar_bytes = sidecar_path
        .is_file()
        .then(|| fs::read(&sidecar_path))
        .transpose();
    let Ok(sidecar_bytes) = sidecar_bytes else {
        context.diagnostics.push(ValidationDiagnostic::error(
            ValidationCode::InputUnreadable,
            &sidecar_path,
            "$file",
        ));
        return;
    };
    let extension = extension_bytes.as_deref().map(|bytes| {
        MetadataInput::new(
            bytes,
            MetadataSource::Extension {
                asset: context.asset_path.to_path_buf(),
                extension: CEL_METADATA_EXTENSION.to_owned(),
            },
        )
    });
    let sidecar = sidecar_bytes
        .as_deref()
        .map(|bytes| MetadataInput::new(bytes, MetadataSource::Sidecar(sidecar_path)));
    if extension.is_none() && sidecar.is_none() {
        return;
    }
    match resolve_metadata(extension, sidecar) {
        Ok(resolved) => MetadataValidator(context).validate(&resolved),
        Err(error) => context.diagnostics.push(metadata_error(error)),
    }
}

pub(crate) fn validate_sidecar(document: &Value, context: &mut SidecarValidation<'_>) {
    let Ok(bytes) = serde_json::to_vec(document) else {
        context.diagnostics.push(ValidationDiagnostic::error(
            ValidationCode::MetadataJsonInvalid,
            context.path,
            "$",
        ));
        return;
    };
    let input = MetadataInput::new(&bytes, MetadataSource::Sidecar(context.path.to_path_buf()));
    match resolve_metadata(None, Some(input)) {
        Ok(resolved) => {
            let node_names = resolved.metadata.nodes.keys().cloned().collect();
            MetadataValidator(MetadataValidation {
                asset_path: context.path,
                node_names: &node_names,
                registry: context.registry,
                diagnostics: context.diagnostics,
            })
            .validate(&resolved);
        }
        Err(error) => context.diagnostics.push(metadata_error(error)),
    }
}

struct MetadataValidator<'a>(MetadataValidation<'a>);

struct NodeValidation<'a> {
    node_path: String,
    node: &'a CelNodeMetadata,
    sources: &'a std::collections::BTreeMap<String, MetadataSource>,
    default_source: std::path::PathBuf,
}

struct NodeIssue {
    code: ValidationCode,
    location: String,
    source_key: String,
}

impl MetadataValidator<'_> {
    fn validate(&mut self, resolved: &ResolvedMetadata) {
        for (node_name, node) in &resolved.metadata.nodes {
            let node_path = format!("$.nodes.{node_name}");
            if !self.0.node_names.contains(node_name) {
                self.0.diagnostics.push(ValidationDiagnostic::error(
                    ValidationCode::MetadataNodeUnknown,
                    self.0.asset_path,
                    &node_path,
                ));
            }
            let context = NodeValidation {
                node_path,
                node,
                sources: &resolved.sources,
                default_source: self.0.asset_path.to_path_buf(),
            };
            self.validate_components(&context);
            self.validate_references(&context);
        }
    }

    fn validate_components(&mut self, context: &NodeValidation<'_>) {
        for (type_path, value) in &context.node.components {
            let location = format!("{}.components.{type_path}", context.node_path);
            let Some(registration) = self.0.registry.get_with_type_path(type_path) else {
                self.push_error(
                    context,
                    NodeIssue {
                        code: ValidationCode::ComponentTypeUnknown,
                        source_key: location.clone(),
                        location,
                    },
                );
                continue;
            };
            if registration.data::<ReflectComponent>().is_none() {
                self.push_error(
                    context,
                    NodeIssue {
                        code: ValidationCode::ComponentRegistrationInvalid,
                        source_key: location.clone(),
                        location,
                    },
                );
                continue;
            }
            if bevy::reflect::serde::TypedReflectDeserializer::new(registration, self.0.registry)
                .deserialize(value.clone())
                .is_err()
            {
                self.push_error(
                    context,
                    NodeIssue {
                        code: ValidationCode::ComponentValueInvalid,
                        source_key: location.clone(),
                        location,
                    },
                );
            }
        }
    }

    fn validate_references(&mut self, context: &NodeValidation<'_>) {
        for (index, target) in context.node.references.bones.iter().enumerate() {
            if !self.0.node_names.contains(target) {
                self.push_error(
                    context,
                    NodeIssue {
                        code: ValidationCode::BoneTargetUnknown,
                        location: format!("{}.references.bones[{index}]", context.node_path),
                        source_key: format!("{}.references.bones", context.node_path),
                    },
                );
            }
        }
        for (index, target) in context.node.references.colliders.iter().enumerate() {
            if !self.0.node_names.contains(target) {
                self.push_error(
                    context,
                    NodeIssue {
                        code: ValidationCode::ColliderTargetUnknown,
                        location: format!("{}.references.colliders[{index}]", context.node_path),
                        source_key: format!("{}.references.colliders", context.node_path),
                    },
                );
            }
        }
        for (index, target) in context.node.references.prefabs.iter().enumerate() {
            let location = format!("{}.references.prefabs[{index}]", context.node_path);
            let code = if !matches!(
                Path::new(target)
                    .extension()
                    .and_then(|value| value.to_str()),
                Some("glb" | "gltf")
            ) {
                Some(ValidationCode::PrefabReferenceMalformed)
            } else if !self
                .0
                .asset_path
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .join(target)
                .is_file()
            {
                Some(ValidationCode::PrefabTargetUnknown)
            } else {
                None
            };
            if let Some(code) = code {
                self.push_error(
                    context,
                    NodeIssue {
                        code,
                        location,
                        source_key: format!("{}.references.prefabs", context.node_path),
                    },
                );
            }
        }
    }

    fn push_error(&mut self, context: &NodeValidation<'_>, issue: NodeIssue) {
        let source =
            context
                .sources
                .get(&issue.source_key)
                .map_or(&context.default_source, |source| match source {
                    MetadataSource::Extension { asset, .. } | MetadataSource::Sidecar(asset) => {
                        asset
                    }
                });
        self.0.diagnostics.push(ValidationDiagnostic::error(
            issue.code,
            source,
            issue.location,
        ));
    }
}
