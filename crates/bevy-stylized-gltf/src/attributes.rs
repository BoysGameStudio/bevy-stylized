//! `_NPR_*` 顶点属性注册表。
//!
//! 正本契约：`schemas/authoring/npr-attributes.v1.json`
//! （docs/reference/06 §4）。本模块把该契约编码为 Bevy
//! [`MeshVertexAttribute`] 常量与注册表，经
//! [`StylizedGltfPluginExt::add_npr_vertex_attributes`] 在
//! **GltfPlugin 加入 App 之前**注册（ADR-0028，机制标记为 provisional）。
//!
//! Canonical attribute registration is checked against the generated contract.
//! Only canonical NPR spellings are registered:
//!
//! - glTF 规范要求 application-specific attribute 以 `_` 开头；`gltf-json`
//!   解析时会剥掉首字符下划线，因此注册 key 为语义名去掉首 `_`
//!   （如 `_NPR_MASK` → `NPR_MASK`）；
//! - 每个 attribute 使用稳定的 `id`（base `0x4E50_5200_0000_0000`，"NPR"）。
//!
//! 量化/shader location 分配未冻结（pending ADR-0012）：v1 全部以未压缩
//! float32 传输，shader location 由 profile 在材质编译期分配。

use bevy::{mesh::MeshVertexAttribute, mesh::VertexFormat};

/// `_NPR_*` attribute id 的稳定 base（ASCII "NPR"）。
pub const NPR_ATTRIBUTE_ID_BASE: u64 = 0x4E50_5200_0000_0000;

/// 通用标量 NPR mask。
pub const ATTRIBUTE_NPR_MASK: MeshVertexAttribute = MeshVertexAttribute::new(
    "_NPR_MASK",
    NPR_ATTRIBUTE_ID_BASE + 1,
    VertexFormat::Float32,
);
/// Per-vertex 布料钉住权重。
pub const ATTRIBUTE_NPR_CLOTH_PIN: MeshVertexAttribute = MeshVertexAttribute::new(
    "_NPR_CLOTH_PIN",
    NPR_ATTRIBUTE_ID_BASE + 2,
    VertexFormat::Float32,
);
/// Per-vertex 自发光贡献。
pub const ATTRIBUTE_NPR_EMISSION: MeshVertexAttribute = MeshVertexAttribute::new(
    "_NPR_EMISSION",
    NPR_ATTRIBUTE_ID_BASE + 3,
    VertexFormat::Float32,
);
/// Face SDF 采样（per-vertex 形式；纹理路径为 fallback）。
pub const ATTRIBUTE_NPR_FACE_SDF: MeshVertexAttribute = MeshVertexAttribute::new(
    "_NPR_FACE_SDF",
    NPR_ATTRIBUTE_ID_BASE + 4,
    VertexFormat::Float32,
);
/// Face 基准侧符号。
pub const ATTRIBUTE_NPR_FACE_SIDE: MeshVertexAttribute = MeshVertexAttribute::new(
    "_NPR_FACE_SIDE",
    NPR_ATTRIBUTE_ID_BASE + 5,
    VertexFormat::Float32,
);
/// Hair 高光 mask。
pub const ATTRIBUTE_NPR_HAIR_MASK: MeshVertexAttribute = MeshVertexAttribute::new(
    "_NPR_HAIR_MASK",
    NPR_ATTRIBUTE_ID_BASE + 6,
    VertexFormat::Float32,
);
/// 物体空间发丝方向。
pub const ATTRIBUTE_NPR_HAIR_TANGENT: MeshVertexAttribute = MeshVertexAttribute::new(
    "_NPR_HAIR_TANGENT",
    NPR_ATTRIBUTE_ID_BASE + 7,
    VertexFormat::Float32x3,
);
/// 整数值材质区域，以标量编码。
pub const ATTRIBUTE_NPR_MATERIAL_ID: MeshVertexAttribute = MeshVertexAttribute::new(
    "_NPR_MATERIAL_ID",
    NPR_ATTRIBUTE_ID_BASE + 8,
    VertexFormat::Float32,
);
/// 场景表面角色标识，以标量编码。
pub const ATTRIBUTE_NPR_LEVEL_ROLE: MeshVertexAttribute = MeshVertexAttribute::new(
    "_NPR_LEVEL_ROLE",
    NPR_ATTRIBUTE_ID_BASE + 9,
    VertexFormat::Float32,
);
/// General control vector.
pub const ATTRIBUTE_NPR_CONTROL0: MeshVertexAttribute = MeshVertexAttribute::new(
    "_NPR_CONTROL0",
    NPR_ATTRIBUTE_ID_BASE + 10,
    VertexFormat::Float32x3,
);

pub use crate::contract::NprAttributeDefault;

/// 一条 `_NPR_*` attribute 的完整契约条目。
#[derive(Clone, Copy, Debug)]
pub struct NprVertexAttributeRegistration {
    /// 导出工具写入 glTF 的语义名（含前导下划线）。
    pub authored_semantic: &'static str,
    /// `gltf-json` 剥掉首个下划线后的 loader key（`GltfPlugin::add_custom_vertex_attribute`
    /// 的查询键）。
    pub loader_key: &'static str,
    /// 唯一的规范 Bevy mesh attribute。
    pub attribute: MeshVertexAttribute,
    /// Blender attribute domain（当前 v1 全部为 `POINT`）。
    pub blender_domain: &'static str,
    /// Blender attribute data type（`FLOAT` / `FLOAT_VECTOR`）。
    pub blender_data_type: &'static str,
    /// 缺失时的缺省值。
    pub default: NprAttributeDefault,
    /// 缺失时的语义 fallback（契约原文）。
    pub missing_fallback: &'static str,
}

/// 完整 `_NPR_*` 注册表（10 条，与 `npr-attributes.v1.json` 逐项对应；
/// 有测试对照 JSON 正本）。
pub const NPR_VERTEX_ATTRIBUTES: [NprVertexAttributeRegistration; 10] = [
    NprVertexAttributeRegistration {
        authored_semantic: "_NPR_MASK",
        loader_key: "NPR_MASK",
        attribute: ATTRIBUTE_NPR_MASK,
        blender_domain: "POINT",
        blender_data_type: "FLOAT",
        default: NprAttributeDefault::Scalar(0.0),
        missing_fallback: "shader-default 0.0",
    },
    NprVertexAttributeRegistration {
        authored_semantic: "_NPR_CLOTH_PIN",
        loader_key: "NPR_CLOTH_PIN",
        attribute: ATTRIBUTE_NPR_CLOTH_PIN,
        blender_domain: "POINT",
        blender_data_type: "FLOAT",
        default: NprAttributeDefault::Scalar(0.0),
        missing_fallback: "no cloth pinning",
    },
    NprVertexAttributeRegistration {
        authored_semantic: "_NPR_EMISSION",
        loader_key: "NPR_EMISSION",
        attribute: ATTRIBUTE_NPR_EMISSION,
        blender_domain: "POINT",
        blender_data_type: "FLOAT",
        default: NprAttributeDefault::Scalar(0.0),
        missing_fallback: "no per-vertex emission",
    },
    NprVertexAttributeRegistration {
        authored_semantic: "_NPR_FACE_SDF",
        loader_key: "NPR_FACE_SDF",
        attribute: ATTRIBUTE_NPR_FACE_SDF,
        blender_domain: "POINT",
        blender_data_type: "FLOAT",
        default: NprAttributeDefault::Scalar(0.0),
        missing_fallback: "face SDF texture path",
    },
    NprVertexAttributeRegistration {
        authored_semantic: "_NPR_FACE_SIDE",
        loader_key: "NPR_FACE_SIDE",
        attribute: ATTRIBUTE_NPR_FACE_SIDE,
        blender_domain: "POINT",
        blender_data_type: "FLOAT",
        default: NprAttributeDefault::Scalar(1.0),
        missing_fallback: "front face",
    },
    NprVertexAttributeRegistration {
        authored_semantic: "_NPR_HAIR_MASK",
        loader_key: "NPR_HAIR_MASK",
        attribute: ATTRIBUTE_NPR_HAIR_MASK,
        blender_domain: "POINT",
        blender_data_type: "FLOAT",
        default: NprAttributeDefault::Scalar(0.0),
        missing_fallback: "no hair highlight",
    },
    NprVertexAttributeRegistration {
        authored_semantic: "_NPR_HAIR_TANGENT",
        loader_key: "NPR_HAIR_TANGENT",
        attribute: ATTRIBUTE_NPR_HAIR_TANGENT,
        blender_domain: "POINT",
        blender_data_type: "FLOAT_VECTOR",
        default: NprAttributeDefault::Vec3([0.0, 1.0, 0.0]),
        missing_fallback: "object-space +Y",
    },
    NprVertexAttributeRegistration {
        authored_semantic: "_NPR_MATERIAL_ID",
        loader_key: "NPR_MATERIAL_ID",
        attribute: ATTRIBUTE_NPR_MATERIAL_ID,
        blender_domain: "POINT",
        blender_data_type: "FLOAT",
        default: NprAttributeDefault::Scalar(0.0),
        missing_fallback: "material slot index",
    },
    NprVertexAttributeRegistration {
        authored_semantic: "_NPR_LEVEL_ROLE",
        loader_key: "NPR_LEVEL_ROLE",
        attribute: ATTRIBUTE_NPR_LEVEL_ROLE,
        blender_domain: "POINT",
        blender_data_type: "FLOAT",
        default: NprAttributeDefault::Scalar(0.0),
        missing_fallback: "generic surface",
    },
    NprVertexAttributeRegistration {
        authored_semantic: "_NPR_CONTROL0",
        loader_key: "NPR_CONTROL0",
        attribute: ATTRIBUTE_NPR_CONTROL0,
        blender_domain: "POINT",
        blender_data_type: "FLOAT_VECTOR",
        default: NprAttributeDefault::Vec3([0.0, 0.0, 0.0]),
        missing_fallback: "zero control vector",
    },
];

/// 为 [`bevy::gltf::GltfPlugin`] 注册全部 `_NPR_*` 自定义顶点属性。
///
/// **调用时机（ADR-0028，provisional）**：必须在把 `GltfPlugin` 加入
/// `App` **之前**调用；`GltfPlugin::finish` 会把 attribute 表烤进
/// `GltfLoader`，之后再注册会被静默丢弃。facade 提供
/// `bevy_stylized::stylized_gltf_plugin()` 作为正确入口。
pub trait StylizedGltfPluginExt {
    /// 注册全部 10 个 `_NPR_*` 自定义顶点属性。
    #[must_use]
    fn add_npr_vertex_attributes(self) -> Self;
}

impl StylizedGltfPluginExt for bevy::gltf::GltfPlugin {
    fn add_npr_vertex_attributes(mut self) -> Self {
        for registration in NPR_VERTEX_ATTRIBUTES {
            self =
                self.add_custom_vertex_attribute(registration.loader_key, registration.attribute);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 与机器可读正本 `npr-attributes.v1.json` 逐项对照。
    #[test]
    fn registration_table_matches_contract_json() {
        let contract_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../schemas/authoring/npr-attributes.v1.json"
        );
        let contract: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(contract_path).expect("contract JSON must be readable"),
        )
        .expect("contract JSON must parse");

        let attributes = contract["attributes"].as_array().expect("attributes array");
        assert_eq!(
            attributes.len(),
            NPR_VERTEX_ATTRIBUTES.len(),
            "contract attribute count drifted from registration table"
        );

        for (entry, registration) in attributes.iter().zip(NPR_VERTEX_ATTRIBUTES.iter()) {
            assert_eq!(
                entry["name"].as_str().unwrap(),
                registration.authored_semantic
            );
            assert_eq!(
                entry["gltf_semantic"].as_str().unwrap(),
                registration.authored_semantic
            );
            assert_eq!(
                entry["domain"].as_str().unwrap(),
                registration.blender_domain
            );
            assert_eq!(
                entry["data_type"].as_str().unwrap(),
                registration.blender_data_type
            );
            assert_eq!(
                entry["missing_fallback"].as_str().unwrap(),
                registration.missing_fallback
            );
            let expected_format = match entry["bevy_vertex_format"].as_str().unwrap() {
                "Float32" => VertexFormat::Float32,
                "Float32x3" => VertexFormat::Float32x3,
                other => panic!("unexpected contract vertex format: {other}"),
            };
            assert_eq!(registration.attribute.format, expected_format);
            assert_eq!(
                registration.authored_semantic.strip_prefix('_'),
                Some(registration.loader_key),
                "loader key must be the authored semantic minus the leading underscore"
            );
        }


    }

    #[test]
    fn attribute_ids_are_stable_and_unique() {
        let mut ids = NPR_VERTEX_ATTRIBUTES
            .iter()
            .map(|r| r.attribute.id)
            .collect::<Vec<_>>();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), NPR_VERTEX_ATTRIBUTES.len());
        for (offset, registration) in NPR_VERTEX_ATTRIBUTES.iter().enumerate() {
            assert_eq!(
                registration.attribute.id,
                MeshVertexAttribute::new(
                    "stable-id-proof",
                    NPR_ATTRIBUTE_ID_BASE + 1 + offset as u64,
                    registration.attribute.format,
                )
                .id
            );
        }
    }

    #[test]
    fn plugin_ext_registers_all_attributes_under_loader_keys() {
        let plugin = bevy::gltf::GltfPlugin::default().add_npr_vertex_attributes();
        assert_eq!(
            plugin.custom_vertex_attributes.len(),
            NPR_VERTEX_ATTRIBUTES.len()
        );
        for registration in NPR_VERTEX_ATTRIBUTES {
            assert_eq!(
                plugin.custom_vertex_attributes[registration.loader_key],
                registration.attribute
            );
            assert!(
                !plugin
                    .custom_vertex_attributes
                    .contains_key(registration.authored_semantic)
            );
        }
        // 默认不注册任何 `_CEL_*`。
        assert!(
            !plugin
                .custom_vertex_attributes
                .keys()
                .any(|k| k.starts_with("CEL_"))
        );
    }
}
