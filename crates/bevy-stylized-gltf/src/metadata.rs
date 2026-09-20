//! Typed extras / sidecar 导入（`bevy_stylized.*` namespace）。
//!
//! 两条元数据通道（docs/reference/03-blender-export-contract.md）：
//!
//! - **extras**：glTF 文档内 `extras` 字段中 `bevy_stylized.material/v1`
//!   键下的 payload；
//! - **sidecar**：与 `.gltf` / `.glb` 同目录的 `.npr.json` 文件。
//!
//! **优先级未定（ADR-0020 仍为 Proposed）**：本模块同时实现两条通道，
//! 二者冲突时产生显式 [`StylizedImportDiagnostic`] 并**拒绝静默取舍**
//! （冲突项的 descriptor 置 `None`，由调用方显式解决）。该行为本身也是
//! provisional，随 ADR-0020 关闭可能调整。

use std::{collections::BTreeMap, path::Path};

use bevy_stylized_core::{
    SCHEMA_ID_MATERIAL_V1, StylizedMaterialDescriptor, StylizedSchemaVersion,
};
use serde::{Deserialize, Serialize};

/// glTF extras 中 material domain 的挂载键（`bevy_stylized.material/v1`）。
pub const MATERIAL_EXTRAS_KEY: &str = "bevy_stylized.material/v1";

/// sidecar 文件的 schema 标识（骨架；正式 schema 随导出契约冻结）。
pub const STYLIZED_SIDECAR_SCHEMA: &str = "bevy-stylized.sidecar/v1";

/// sidecar 文件的扩展名（`<asset>.npr.json`）。
pub const STYLIZED_SIDECAR_EXTENSION: &str = "npr.json";

/// extras / sidecar 导入错误。
#[derive(Debug, thiserror::Error)]
pub enum StylizedImportError {
    /// JSON 结构/类型错误（带 serde_path_to_error 的路径信息）。
    #[error("invalid {context}: {source}")]
    Json {
        /// 出错位置的人类可读上下文（如 `material 'body' extras`）。
        context: String,
        /// 带 JSON 路径的底层错误。
        source: serde_path_to_error::Error<serde_json::Error>,
    },
    /// glTF extras 不是合法 JSON（RawValue 重解析失败）。
    #[error("invalid JSON in {context}: {source}")]
    ExtrasNotJson {
        /// 出错位置的人类可读上下文。
        context: String,
        /// 底层 JSON 错误。
        source: serde_json::Error,
    },
    /// sidecar 文件读取失败。
    #[error("failed to read sidecar {path}: {source}")]
    Io {
        /// sidecar 路径。
        path: String,
        /// 底层 IO 错误。
        source: std::io::Error,
    },
    /// sidecar 顶层 schema 标识不符。
    #[error("unsupported sidecar schema {found:?}; expected {STYLIZED_SIDECAR_SCHEMA:?}")]
    UnsupportedSidecarSchema {
        /// 实际读到的 schema 标识。
        found: String,
    },
    /// payload 声明的 schema 版本超出当前读取能力（fail-closed，
    /// 见 [`StylizedSchemaVersion`] 的演进规则）。
    #[error("unsupported schema version {found} in {namespace}; this reader supports {supported}")]
    UnsupportedSchemaVersion {
        /// schema namespace。
        namespace: String,
        /// payload 声明的版本。
        found: StylizedSchemaVersion,
        /// 读取方支持的版本。
        supported: StylizedSchemaVersion,
    },
}

/// `.npr.json` sidecar 的顶层结构（骨架，provisional）。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StylizedSidecar {
    /// 顶层 schema 标识，必须等于 [`STYLIZED_SIDECAR_SCHEMA`]。
    pub schema: String,
    /// 按材质名索引的 material descriptor。
    #[serde(default)]
    pub materials: BTreeMap<String, StylizedMaterialDescriptor>,
}

/// 由 `.gltf` / `.glb` 路径推导 sidecar 路径（`<stem>.npr.json`）。
#[must_use]
pub fn sidecar_path_for_gltf(gltf_path: &Path) -> std::path::PathBuf {
    let stem = gltf_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    gltf_path.with_file_name(format!("{stem}.{STYLIZED_SIDECAR_EXTENSION}"))
}

/// 解析 sidecar JSON 字符串（serde_path_to_error 级别的错误信息）。
pub fn parse_sidecar_str(json: &str) -> Result<StylizedSidecar, StylizedImportError> {
    let sidecar: StylizedSidecar = serde_path_to_error::deserialize(
        &mut serde_json::Deserializer::from_str(json),
    )
    .map_err(|source| StylizedImportError::Json {
        context: "sidecar".to_string(),
        source,
    })?;
    if sidecar.schema != STYLIZED_SIDECAR_SCHEMA {
        return Err(StylizedImportError::UnsupportedSidecarSchema {
            found: sidecar.schema,
        });
    }
    Ok(sidecar)
}

/// 从磁盘读取并解析 sidecar。
pub fn load_sidecar(path: impl AsRef<Path>) -> Result<StylizedSidecar, StylizedImportError> {
    let path = path.as_ref();
    let json = std::fs::read_to_string(path).map_err(|source| StylizedImportError::Io {
        path: path.display().to_string(),
        source,
    })?;
    parse_sidecar_str(&json)
}

/// 从 glTF extras JSON（任意节点的 `extras` 值）提取 material descriptor。
///
/// extras 中不存在 [`MATERIAL_EXTRAS_KEY`] 时返回 `Ok(None)`；
/// 存在但结构/版本不合法时 fail-closed。
pub fn extract_material_extras(
    extras: &serde_json::Value,
) -> Result<Option<StylizedMaterialDescriptor>, StylizedImportError> {
    let Some(payload) = extras.get(MATERIAL_EXTRAS_KEY) else {
        return Ok(None);
    };
    let descriptor: StylizedMaterialDescriptor = serde_path_to_error::deserialize(payload.clone())
        .map_err(|source| StylizedImportError::Json {
            context: format!("extras[{MATERIAL_EXTRAS_KEY:?}]"),
            source,
        })?;
    check_material_version(&descriptor)?;
    Ok(Some(descriptor))
}

/// 遍历 glTF 文档的全部 material，提取其 extras 中的 material descriptor。
///
/// 返回 `(material name, descriptor)` 对；无 name 的 material 用索引名
/// `"material_<index>"`，以便与 sidecar 按键对齐。
pub fn extract_materials_from_document(
    document: &gltf::Document,
) -> Result<Vec<(String, StylizedMaterialDescriptor)>, StylizedImportError> {
    let mut out = Vec::new();
    for (index, material) in document.materials().enumerate() {
        let Some(extras) = material.extras() else {
            continue;
        };
        let extras_json: serde_json::Value =
            serde_json::from_str(extras.get()).map_err(|source| {
                StylizedImportError::ExtrasNotJson {
                    context: format!("material #{index} extras"),
                    source,
                }
            })?;
        if let Some(mut descriptor) = extract_material_extras(&extras_json)? {
            let name = material
                .name()
                .map(str::to_string)
                .unwrap_or_else(|| format!("material_{index}"));
            descriptor.name.clone_from(&name);
            out.push((name, descriptor));
        }
    }
    Ok(out)
}

fn check_material_version(
    descriptor: &StylizedMaterialDescriptor,
) -> Result<(), StylizedImportError> {
    let supported = SCHEMA_ID_MATERIAL_V1.version;
    if supported.can_read(descriptor.schema_version) {
        Ok(())
    } else {
        Err(StylizedImportError::UnsupportedSchemaVersion {
            namespace: SCHEMA_ID_MATERIAL_V1.namespace.to_string(),
            found: descriptor.schema_version,
            supported,
        })
    }
}

/// 诊断严重级别。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StylizedImportSeverity {
    /// 警告：导入可继续，但需要人工关注。
    Warning,
    /// 错误：fail-closed。
    Error,
}

/// 诊断类别。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StylizedImportDiagnosticCode {
    /// extras 与 sidecar 同时提供且内容冲突（ADR-0020 未定优先级，
    /// 拒绝静默取舍）。
    ExtrasSidecarConflict,
}

/// 导入诊断（warning 级别也显式暴露，绝不静默）。
#[derive(Clone, Debug, PartialEq)]
pub struct StylizedImportDiagnostic {
    /// 严重级别。
    pub severity: StylizedImportSeverity,
    /// 类别。
    pub code: StylizedImportDiagnosticCode,
    /// 人类可读消息。
    pub message: String,
    /// extras 通道的值（用于对照）。
    pub extras_value: Option<serde_json::Value>,
    /// sidecar 通道的值（用于对照）。
    pub sidecar_value: Option<serde_json::Value>,
}

/// 单个材质的导入结果。
#[derive(Clone, Debug, Default)]
pub struct StylizedMaterialImport {
    /// 解析出的 descriptor；extras/sidecar 冲突时为 `None`（拒绝静默取舍）。
    pub material: Option<StylizedMaterialDescriptor>,
    /// 诊断列表。
    pub diagnostics: Vec<StylizedImportDiagnostic>,
}

/// 合并 extras 与 sidecar 两条通道的 material 元数据。
///
/// ADR-0020（优先级）仍为 Proposed，因此本函数实现**双通道读取 +
/// 冲突暴露**，不实现任何优先级：
///
/// - 仅一条通道有数据：直接采用；
/// - 两条通道都有且语义相等：采用并附 warning 记录双通道来源；
/// - 两条通道都有且冲突：返回 `material: None` + Conflict 诊断。
///
/// `extras` 为 glTF material 的原始 extras JSON（可为 `None`）。
pub fn import_material(
    name: &str,
    extras: Option<&serde_json::Value>,
    sidecar: Option<&StylizedMaterialDescriptor>,
) -> Result<StylizedMaterialImport, StylizedImportError> {
    let from_extras = match extras {
        Some(value) => extract_material_extras(value)?,
        None => None,
    };

    let mut import = StylizedMaterialImport::default();
    match (from_extras, sidecar.cloned()) {
        (None, None) => {}
        (Some(descriptor), None) | (None, Some(descriptor)) => {
            import.material = Some(descriptor);
        }
        (Some(extras_descriptor), Some(sidecar_descriptor)) => {
            let extras_value = serde_json::to_value(&extras_descriptor)
                .expect("descriptor serialization is infallible");
            let sidecar_value = serde_json::to_value(sidecar_descriptor)
                .expect("descriptor serialization is infallible");
            if extras_value == sidecar_value {
                import.material = Some(extras_descriptor);
            } else {
                import.diagnostics.push(StylizedImportDiagnostic {
                    severity: StylizedImportSeverity::Warning,
                    code: StylizedImportDiagnosticCode::ExtrasSidecarConflict,
                    message: format!(
                        "material {name:?}: extras and sidecar both provide \
                         {MATERIAL_EXTRAS_KEY} metadata with different contents; \
                         precedence is undecided (ADR-0020 Proposed), refusing to \
                         pick one silently"
                    ),
                    extras_value: Some(extras_value),
                    sidecar_value: Some(sidecar_value),
                });
            }
        }
    }
    Ok(import)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extras_with_material(payload: serde_json::Value) -> serde_json::Value {
        serde_json::json!({ MATERIAL_EXTRAS_KEY: payload })
    }

    #[test]
    fn extras_without_namespace_returns_none() {
        let extras = serde_json::json!({"unrelated": true});
        assert_eq!(extract_material_extras(&extras).unwrap(), None);
    }

    #[test]
    fn extras_roundtrip_into_descriptor() {
        let extras = extras_with_material(serde_json::json!({
            "name": "body",
            "surface_class": "character",
            "static_params": { "hair_specular": 0.5 }
        }));
        let descriptor = extract_material_extras(&extras)
            .unwrap()
            .expect("descriptor present");
        assert_eq!(descriptor.name, "body");
        assert_eq!(
            descriptor.surface_class,
            bevy_stylized_core::StylizedSurfaceClass::Character
        );
        assert_eq!(descriptor.static_params["hair_specular"], 0.5);
    }

    #[test]
    fn extras_type_error_carries_json_path() {
        let extras = extras_with_material(serde_json::json!({
            "surface_class": "not-a-class"
        }));
        let error = extract_material_extras(&extras).expect_err("must fail closed");
        let message = error.to_string();
        assert!(
            message.contains("surface_class"),
            "error should carry the field path, got: {message}"
        );
    }

    #[test]
    fn unsupported_major_version_fails_closed() {
        let extras = extras_with_material(serde_json::json!({
            "schema_version": { "major": 2, "minor": 0 },
            "name": "future"
        }));
        let error = extract_material_extras(&extras).expect_err("must fail closed");
        assert!(matches!(
            error,
            StylizedImportError::UnsupportedSchemaVersion { .. }
        ));
    }

    #[test]
    fn sidecar_parses_with_path_quality_errors() {
        let sidecar = parse_sidecar_str(
            r#"{
                "schema": "bevy-stylized.sidecar/v1",
                "materials": { "body": { "surface_class": "hero" } }
            }"#,
        )
        .expect("valid sidecar");
        assert_eq!(
            sidecar.materials["body"].surface_class,
            bevy_stylized_core::StylizedSurfaceClass::Hero
        );

        let error = parse_sidecar_str(
            r#"{
                "schema": "bevy-stylized.sidecar/v1",
                "materials": { "body": { "surface_class": 42 } }
            }"#,
        )
        .expect_err("bad field type must fail");
        let message = error.to_string();
        assert!(
            message.contains("materials.body.surface_class"),
            "sidecar error should carry the JSON path, got: {message}"
        );
    }

    #[test]
    fn sidecar_wrong_schema_is_rejected() {
        let error = parse_sidecar_str(r#"{"schema": "other/v9", "materials": {}}"#)
            .expect_err("must fail closed");
        assert!(matches!(
            error,
            StylizedImportError::UnsupportedSidecarSchema { .. }
        ));
    }

    #[test]
    fn conflict_is_diagnosed_never_silently_picked() {
        let extras = extras_with_material(serde_json::json!({
            "name": "body",
            "surface_class": "character"
        }));
        let sidecar_descriptor = StylizedMaterialDescriptor {
            name: "body".into(),
            surface_class: bevy_stylized_core::StylizedSurfaceClass::Hero,
            ..StylizedMaterialDescriptor::default()
        };

        let import = import_material("body", Some(&extras), Some(&sidecar_descriptor)).unwrap();
        assert!(
            import.material.is_none(),
            "conflict must not silently pick a channel"
        );
        assert_eq!(import.diagnostics.len(), 1);
        assert_eq!(
            import.diagnostics[0].code,
            StylizedImportDiagnosticCode::ExtrasSidecarConflict
        );
        assert!(import.diagnostics[0].extras_value.is_some());
        assert!(import.diagnostics[0].sidecar_value.is_some());
    }

    #[test]
    fn agreeing_channels_merge_cleanly() {
        let extras = extras_with_material(serde_json::json!({
            "name": "body",
            "surface_class": "cel"
        }));
        let sidecar_descriptor = StylizedMaterialDescriptor {
            name: "body".into(),
            ..StylizedMaterialDescriptor::default()
        };

        let import = import_material("body", Some(&extras), Some(&sidecar_descriptor)).unwrap();
        assert!(import.material.is_some());
        assert!(import.diagnostics.is_empty());
    }

    #[test]
    fn single_channel_is_used_directly() {
        let sidecar_descriptor = StylizedMaterialDescriptor {
            name: "body".into(),
            ..StylizedMaterialDescriptor::default()
        };
        let import = import_material("body", None, Some(&sidecar_descriptor)).unwrap();
        assert_eq!(import.material, Some(sidecar_descriptor));
    }

    #[test]
    fn sidecar_path_derivation() {
        assert_eq!(
            sidecar_path_for_gltf(Path::new("assets/hero.glb")),
            Path::new("assets/hero.npr.json")
        );
        assert_eq!(
            sidecar_path_for_gltf(Path::new("assets/hero.gltf")),
            Path::new("assets/hero.npr.json")
        );
    }
}
