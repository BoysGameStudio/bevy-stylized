//! `bevy-stylized-gltf`：glTF 导入层。
//!
//! 职责（docs/architecture/03，依赖方向 `gltf → core`）：
//!
//! - [`attributes`]：把 `npr-attributes.v1.json` 契约的 10 个 `_NPR_*`
//!   顶点属性注册为 Bevy 自定义顶点属性；`_CEL_*` 别名表仅以数据形式
//!   提供，默认不注册；
//! - [`metadata`]：`bevy_stylized.*` namespaced extras 与 `.npr.json`
//!   sidecar 的 typed 导入；extras/sidecar 冲突显式暴露（ADR-0020
//!   Proposed，绝不静默取舍）；
//! - [`setup`]：`GltfPlugin` 配置入口与初始化顺序守卫（ADR-0028，
//!   机制标记为 provisional）。

#[cfg(feature = "bevy-integration")]
pub mod attributes;
#[cfg(feature = "bevy-integration")]
pub mod skin_attributes;
pub mod contract;
pub mod metadata;
#[cfg(feature = "bevy-integration")]
pub mod setup;

#[cfg(feature = "bevy-integration")]
pub use attributes::{
    ATTRIBUTE_NPR_CLOTH_PIN, ATTRIBUTE_NPR_CONTROL0, ATTRIBUTE_NPR_EMISSION,
    ATTRIBUTE_NPR_FACE_SDF, ATTRIBUTE_NPR_FACE_SIDE, ATTRIBUTE_NPR_HAIR_MASK,
    ATTRIBUTE_NPR_HAIR_TANGENT, ATTRIBUTE_NPR_LEVEL_ROLE, ATTRIBUTE_NPR_MASK,
    ATTRIBUTE_NPR_MATERIAL_ID, NPR_ATTRIBUTE_ID_BASE, NPR_VERTEX_ATTRIBUTES,
    NprVertexAttributeRegistration, StylizedGltfPluginExt,
};
pub use contract::{NPR_ATTRIBUTE_SPECS, NprAttributeDefault, NprAttributeSpec};
pub use metadata::{
    MATERIAL_EXTRAS_KEY, STYLIZED_SIDECAR_EXTENSION, STYLIZED_SIDECAR_SCHEMA,
    StylizedImportDiagnostic, StylizedImportDiagnosticCode, StylizedImportError,
    StylizedImportSeverity, StylizedMaterialImport, StylizedSidecar, extract_material_extras,
    extract_materials_from_document, import_material, load_sidecar, parse_sidecar_str,
    sidecar_path_for_gltf,
};
#[cfg(feature = "bevy-integration")]
pub use setup::{StylizedGltfConfigError, check_gltf_plugin_not_added, npr_gltf_plugin};
