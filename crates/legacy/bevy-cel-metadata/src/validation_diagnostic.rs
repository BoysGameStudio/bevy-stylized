use std::path::{Path, PathBuf};

use serde::Serialize;

/// Version of the validator's machine-readable output contract.
pub const VALIDATION_OUTPUT_SCHEMA_VERSION: u32 = 1;

/// Stable validation diagnostic identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum ValidationCode {
    /// No input was supplied to the CLI.
    CliInputMissing,
    /// An output format is not supported.
    CliFormatUnsupported,
    /// A CLI option is not recognized.
    CliArgumentUnknown,
    /// The input suffix is not a supported asset type.
    InputExtensionUnsupported,
    /// The input could not be read.
    InputUnreadable,
    /// A GLB header is shorter than twelve bytes.
    GlbHeaderTruncated,
    /// A GLB does not begin with the standard magic bytes.
    GlbMagicInvalid,
    /// A GLB container version is unsupported.
    GlbVersionUnsupported,
    /// A GLB's declared length differs from its file length.
    GlbLengthInvalid,
    /// A GLB chunk header is incomplete.
    GlbChunkHeaderTruncated,
    /// A GLB chunk extends outside the container.
    GlbChunkLengthInvalid,
    /// A GLB chunk is not four-byte aligned.
    GlbChunkAlignmentInvalid,
    /// A GLB contains no JSON chunk.
    GlbJsonMissing,
    /// A GLB contains more than one JSON chunk.
    GlbJsonDuplicate,
    /// A GLB contains more than one binary chunk.
    GlbBinaryDuplicate,
    /// JSON syntax is invalid.
    JsonSyntaxInvalid,
    /// JSON has the wrong structural type.
    JsonShapeInvalid,
    /// The glTF asset version is not 2.0.
    GltfVersionUnsupported,
    /// A referenced external file does not exist.
    FileReferenceMissing,
    /// A remote URI cannot be checked locally.
    RemoteReferenceUnverified,
    /// A URI or embedded resource declaration is malformed.
    FileReferenceMalformed,
    /// A declared embedded buffer has no GLB binary chunk.
    BinaryChunkMissing,
    /// A `_CEL_*` semantic is not in the canonical registry.
    CelAttributeUnknown,
    /// A CEL attribute does not point to an accessor.
    CelAttributeAccessorMalformed,
    /// A CEL attribute accessor index does not exist.
    CelAttributeAccessorUnknown,
    /// A CEL attribute accessor does not match its canonical format.
    CelAttributeFormatInvalid,
    /// A bone reference is not an integer or string where required.
    BoneReferenceMalformed,
    /// A bone target is absent from the glTF nodes.
    BoneTargetUnknown,
    /// A prefab reference has invalid syntax or suffix.
    PrefabReferenceMalformed,
    /// A prefab asset target does not exist.
    PrefabTargetUnknown,
    /// A collider reference is not a string.
    ColliderReferenceMalformed,
    /// A collider target is absent from the glTF nodes.
    ColliderTargetUnknown,
    /// CEL metadata JSON does not match its schema.
    MetadataJsonInvalid,
    /// A CEL metadata schema version is unsupported.
    MetadataVersionUnsupported,
    /// CEL metadata names a node absent from the asset.
    MetadataNodeUnknown,
    /// A reflected component type is not registered.
    ComponentTypeUnknown,
    /// A registered reflected type is not a component.
    ComponentRegistrationInvalid,
    /// A reflected component value has the wrong shape.
    ComponentValueInvalid,
}

/// Diagnostic severity used by human and JSON output.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationSeverity {
    /// Validation failed.
    Error,
    /// Validation completed with a deterministic caution.
    Warning,
}

/// One path-qualified validation finding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ValidationDiagnostic {
    /// Stable machine-consumable code.
    pub code: ValidationCode,
    /// Error or warning classification.
    pub severity: ValidationSeverity,
    /// Input or sidecar file containing the finding.
    pub file_path: String,
    /// JSON path or binary asset location containing the finding.
    pub asset_path: String,
    /// Concise human explanation.
    pub message: &'static str,
    /// Concise corrective action.
    pub remediation: &'static str,
}

impl ValidationDiagnostic {
    pub(crate) fn error(code: ValidationCode, file: &Path, asset_path: impl Into<String>) -> Self {
        Self {
            code,
            severity: ValidationSeverity::Error,
            file_path: file.to_string_lossy().into_owned(),
            asset_path: asset_path.into(),
            message: code.message(),
            remediation: code.remediation(),
        }
    }

    pub(crate) fn warning(
        code: ValidationCode,
        file: &Path,
        asset_path: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: ValidationSeverity::Warning,
            file_path: file.to_string_lossy().into_owned(),
            asset_path: asset_path.into(),
            message: code.message(),
            remediation: code.remediation(),
        }
    }

    pub(crate) fn sort_key(&self) -> (&str, &str, ValidationCode, ValidationSeverity) {
        (&self.file_path, &self.asset_path, self.code, self.severity)
    }
}

/// Versioned deterministic machine output.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ValidationReport {
    /// Output schema identifier.
    pub schema: &'static str,
    /// Output schema version.
    pub schema_version: u32,
    /// True only when no error diagnostics exist.
    pub valid: bool,
    /// Sorted input paths.
    pub files: Vec<String>,
    /// Deterministically sorted diagnostics.
    pub diagnostics: Vec<ValidationDiagnostic>,
}

impl ValidationReport {
    /// Builds a versioned report for one CLI usage error.
    #[must_use]
    pub fn cli_error(code: ValidationCode) -> Self {
        let asset_path = match code {
            ValidationCode::CliInputMissing => "$cli.input",
            ValidationCode::CliFormatUnsupported => "$cli.format",
            ValidationCode::CliArgumentUnknown => "$cli.arguments",
            _ => "$cli",
        };
        Self::from_parts(
            &[],
            vec![ValidationDiagnostic::error(
                code,
                Path::new("<cli>"),
                asset_path,
            )],
        )
    }

    pub(crate) fn from_parts(
        paths: &[PathBuf],
        mut diagnostics: Vec<ValidationDiagnostic>,
    ) -> Self {
        let mut files: Vec<_> = paths
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect();
        files.sort();
        diagnostics.sort_by(|left, right| left.sort_key().cmp(&right.sort_key()));
        let valid = diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != ValidationSeverity::Error);
        Self {
            schema: "cel_gltf_validate.output",
            schema_version: VALIDATION_OUTPUT_SCHEMA_VERSION,
            valid,
            files,
            diagnostics,
        }
    }
}
