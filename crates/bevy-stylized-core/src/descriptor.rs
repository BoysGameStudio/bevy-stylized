//! Material / Instance / View 三层描述符（docs/rendering/02）。
//!
//! 三层数据严格分离，禁止跨层写回：
//!
//! - [`StylizedMaterialDescriptor`]：纹理/Ramp/静态参数，资产级、帧间不变；
//! - [`StylizedInstanceDescriptor`]：per-instance 的 Face/Outline/Profile 索引；
//! - [`StylizedViewDescriptor`]：per-view/frame 的灯光、调色板、昼夜、曝光与相机 NPR 状态。
//!
//! 这些类型是 **schema-neutral 序列化类型**：只携带领域数据与
//! [`StylizedSchemaVersion`](crate::schema::StylizedSchemaVersion)，
//! 不携带任何 GPU 句柄或渲染内部类型。

use std::collections::BTreeMap;

use bevy_color::LinearRgba;
use bevy_math::Vec3;
use bevy_reflect::Reflect;

use crate::{
    axis::{StylizedExecutionTier, StylizedSurfaceClass, StylizedValidationMode},
    schema::{
        SCHEMA_ID_INSTANCE_V1, SCHEMA_ID_MATERIAL_V1, SCHEMA_ID_VIEW_V1, StylizedSchemaVersion,
    },
};

/// 纹理语义（docs/authoring/03）：决定 sRGB/Linear、压缩、过滤、Mipmap 与精度。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum StylizedTextureSemantic {
    /// 基色等颜色纹理（sRGB）。
    #[default]
    Color,
    /// 法线纹理（Linear）。
    Normal,
    /// 连续 Mask（如 ILM 通道），不得因压缩产生语义漂移。
    ContinuousMask,
    /// 离散分类（如 Material ID），压缩不得导致 ID 漂移。
    DiscreteClassification,
    /// Ramp / 阴影阶调查找。
    Ramp,
    /// MatCap 查找。
    MatCap,
}

/// 纹理引用（schema-neutral）：uri 为资产路径/URI，不含 GPU 句柄。
#[derive(Clone, Debug, Default, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StylizedTextureRef {
    /// 纹理语义。
    #[cfg_attr(feature = "serde", serde(default = "default_texture_semantic"))]
    pub semantic: StylizedTextureSemantic,
    /// 资产路径或 URI。
    #[cfg_attr(feature = "serde", serde(default))]
    pub uri: String,
}

#[cfg(feature = "serde")]
fn default_texture_semantic() -> StylizedTextureSemantic {
    StylizedTextureSemantic::Color
}

/// 静态材质开关（帧间不变，属 Material 层）。
#[derive(Clone, Debug, Default, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StylizedMaterialFlags {
    /// 双面渲染。
    #[cfg_attr(feature = "serde", serde(default))]
    pub double_sided: bool,
    /// Alpha 测试（cutout）。
    #[cfg_attr(feature = "serde", serde(default))]
    pub alpha_test: bool,
    /// 使用顶点色 AO。
    #[cfg_attr(feature = "serde", serde(default))]
    pub use_vertex_color_ao: bool,
}

#[cfg(feature = "serde")]
fn default_material_schema_version() -> StylizedSchemaVersion {
    SCHEMA_ID_MATERIAL_V1.version
}

/// Material 层描述符：纹理/Ramp、静态开关与 Hair/Metal 等静态参数。
///
/// **禁止**存放 Face Transform、Day/Night、相机后处理等 instance/view 状态。
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StylizedMaterialDescriptor {
    /// 内容所声明的 schema 版本（命名空间固定为
    /// [`SCHEMA_ID_MATERIAL_V1`]，故此处只携带版本号）。
    #[cfg_attr(feature = "serde", serde(default = "default_material_schema_version"))]
    pub schema_version: StylizedSchemaVersion,
    /// 材质名（与 glTF material name 对应）。
    #[cfg_attr(feature = "serde", serde(default))]
    pub name: String,
    /// 表面风格分类。
    #[cfg_attr(feature = "serde", serde(default))]
    pub surface_class: StylizedSurfaceClass,
    /// 纹理引用（含 Ramp，经 [`StylizedTextureSemantic::Ramp`] 表达）。
    #[cfg_attr(feature = "serde", serde(default))]
    pub textures: Vec<StylizedTextureRef>,
    /// 静态材质开关。
    #[cfg_attr(feature = "serde", serde(default))]
    pub static_flags: StylizedMaterialFlags,
    /// Hair/Metal 等静态 Profile 参数（键名为 profile 契约定义）。
    #[cfg_attr(feature = "serde", serde(default))]
    pub static_params: BTreeMap<String, f32>,
}

impl Default for StylizedMaterialDescriptor {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_ID_MATERIAL_V1.version,
            name: String::new(),
            surface_class: StylizedSurfaceClass::default(),
            textures: Vec::new(),
            static_flags: StylizedMaterialFlags::default(),
            static_params: BTreeMap::new(),
        }
    }
}

/// Instance 层描述符：per-character / per-instance 的可变数据。
///
/// 对应 docs/rendering/02 的 `StylizedInstanceGpu` 逻辑数据模型；
/// GPU 传输机制由 ADR-0004（Accepted）决定；本类型是逻辑模型，
/// 稳定运输布局见 render 层 MeshTag 位布局与 StorageTable 合同。
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StylizedInstanceDescriptor {
    /// Face SDF 阈值。
    pub face_threshold: f32,
    /// Face 基准侧（0/1，对应 face side 选择）。
    pub face_side: u32,
    /// 描边宽度缩放。
    pub outline_scale: f32,
    /// Profile / 调色板索引。
    pub profile_index: u32,
    /// Per-character 开关位。
    pub per_character_flags: u32,
    /// Instance 级覆盖参数（键名为 profile 契约定义）。
    #[cfg_attr(feature = "serde", serde(default))]
    pub overrides: BTreeMap<String, f32>,
}

impl Default for StylizedInstanceDescriptor {
    fn default() -> Self {
        Self {
            face_threshold: 0.5,
            face_side: 1,
            outline_scale: 1.0,
            profile_index: 0,
            per_character_flags: 0,
            overrides: BTreeMap::new(),
        }
    }
}

/// Instance 层全精度记录（V2-e GPU Instance Database 的记录 schema；
/// ADR-0004 Accepted 的 StorageTable 通道记录）。
///
/// **两档保真契约**（同一逻辑数据的两条传输通道）：
///
/// - **Portable（MeshTag 位打包）**：只携带粗粒度子集——
///   `face_threshold` u8 量化（1/254 步进）、`face_side` 1 bit、
///   `outline_scale` u7（0..=2）、`profile_index` u8、`quality_flags` 的
///   低 8 bit；`palette_index` 与 `light_tint` **不可表达**；
/// - **Standard+（StorageTable）**：本记录全字段全精度（f32 / u32），
///   MeshTag 退化为 24-bit 槽位索引。
///
/// 通道选择由 resolver 决定（tier ≥ Standard 且设备具备 storage buffer
/// ⇒ StorageTable；否则 MeshTagBits + 登记
/// [`StylizedFallbackStep::InstanceTableToMeshTag`](crate::StylizedFallbackStep::InstanceTableToMeshTag)
/// 语义）。
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StylizedInstanceGpuRecord {
    /// 内容所声明的 schema 版本（命名空间固定为
    /// [`SCHEMA_ID_INSTANCE_V1`](crate::schema::SCHEMA_ID_INSTANCE_V1)）。
    #[cfg_attr(feature = "serde", serde(default = "default_instance_schema_version"))]
    pub schema_version: StylizedSchemaVersion,
    /// Face SDF 阈值（全精度；MeshTag 通道量化为 1/254）。
    pub face_threshold: f32,
    /// Face 基准侧（0/1）。
    pub face_side: u32,
    /// 描边宽度缩放（0..=2；MeshTag 通道 u7 量化）。
    pub outline_scale: f32,
    /// Profile / ramp 行索引（MeshTag 通道限 u8）。
    pub profile_index: u32,
    /// Palette override 索引（MeshTag 通道不可表达）。
    pub palette_index: u32,
    /// Quality override 位（LOD/quality 语义位，render 侧解释）。
    pub quality_flags: u32,
    /// Lighting override 染色（线性；MeshTag 通道不可表达）。
    #[cfg_attr(
        feature = "schema",
        schemars(with = "crate::schema::LinearColorSchema")
    )]
    pub light_tint: LinearRgba,
}

#[cfg(feature = "serde")]
fn default_instance_schema_version() -> StylizedSchemaVersion {
    crate::schema::SCHEMA_ID_INSTANCE_V1.version
}

impl Default for StylizedInstanceGpuRecord {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_ID_INSTANCE_V1.version,
            face_threshold: 0.5,
            face_side: 0,
            outline_scale: 1.0,
            profile_index: 0,
            palette_index: 0,
            quality_flags: 0,
            light_tint: LinearRgba::WHITE,
        }
    }
}

impl From<StylizedInstanceDescriptor> for StylizedInstanceGpuRecord {
    /// 无损方向：descriptor → record（record 是超集）。
    fn from(descriptor: StylizedInstanceDescriptor) -> Self {
        Self {
            face_threshold: descriptor.face_threshold,
            face_side: descriptor.face_side,
            outline_scale: descriptor.outline_scale,
            profile_index: descriptor.profile_index,
            quality_flags: descriptor.per_character_flags,
            ..StylizedInstanceGpuRecord::default()
        }
    }
}

/// View 层质量状态（当前 view 实际运行的 tier 与验证模式快照）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StylizedViewQuality {
    /// 当前 view 解析后的执行档位。
    #[cfg_attr(feature = "serde", serde(default))]
    pub tier: StylizedExecutionTier,
    /// 当前验证模式。
    #[cfg_attr(feature = "serde", serde(default))]
    pub validation_mode: StylizedValidationMode,
}

/// 相机 NPR 状态（属 View 层，**禁止**写入角色材质）。
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StylizedCameraNprState {
    /// 是否启用描边。
    pub outline_enabled: bool,
    /// 是否启用生产后处理链。
    pub post_enabled: bool,
}

impl Default for StylizedCameraNprState {
    fn default() -> Self {
        Self {
            outline_enabled: true,
            post_enabled: true,
        }
    }
}

/// View / Frame 层描述符：NPR Key Light、全局调色板、昼夜、曝光与相机 NPR 状态。
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StylizedViewDescriptor {
    /// 内容所声明的 schema 版本（命名空间固定为
    /// [`SCHEMA_ID_VIEW_V1`]）。
    #[cfg_attr(feature = "serde", serde(default = "default_view_schema_version"))]
    pub schema_version: StylizedSchemaVersion,
    /// NPR Key Light 方向（世界空间，指向光源）。
    #[cfg_attr(feature = "schema", schemars(with = "[f32; 3]"))]
    pub key_light_direction: Vec3,
    /// NPR Key Light 颜色（线性空间）。
    #[cfg_attr(
        feature = "schema",
        schemars(with = "crate::schema::LinearColorSchema")
    )]
    pub key_light_color: LinearRgba,
    /// Explicit authored ambient radiance. None preserves day/night interpolation.
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "Option<crate::schema::LinearColorSchema>")
    )]
    pub ambient_override: Option<LinearRgba>,
    /// 全局调色板（一份全局数据，**禁止**逐材质复制）。
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "Vec<crate::schema::LinearColorSchema>")
    )]
    pub palette: Vec<LinearRgba>,
    /// 昼夜因子（0 = 夜，1 = 昼）。
    pub day_night_factor: f32,
    /// 曝光。
    pub exposure: f32,
    /// View 质量状态。
    #[cfg_attr(feature = "serde", serde(default))]
    pub quality: StylizedViewQuality,
    /// 相机 NPR 状态。
    #[cfg_attr(feature = "serde", serde(default))]
    pub camera: StylizedCameraNprState,
    /// 环境模型（V2-d；additive since schema v1.1——缺省解析为恒等默认，
    /// 既有 v1.0 payload 行为不变）。
    #[cfg_attr(feature = "serde", serde(default))]
    pub environment: crate::environment::StylizedEnvironment,
}

#[cfg(feature = "serde")]
fn default_view_schema_version() -> StylizedSchemaVersion {
    SCHEMA_ID_VIEW_V1.version
}

impl Default for StylizedViewDescriptor {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_ID_VIEW_V1.version,
            ambient_override: None,
            key_light_direction: Vec3::new(0.5, 1.0, 0.5).normalize(),
            key_light_color: LinearRgba::WHITE,
            palette: Vec::new(),
            day_night_factor: 1.0,
            exposure: 1.0,
            quality: StylizedViewQuality::default(),
            camera: StylizedCameraNprState::default(),
            environment: crate::environment::StylizedEnvironment::default(),
        }
    }
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;

    #[test]
    fn material_descriptor_serde_roundtrip() {
        let mut descriptor = StylizedMaterialDescriptor {
            name: "body".into(),
            surface_class: StylizedSurfaceClass::Character,
            ..StylizedMaterialDescriptor::default()
        };
        descriptor.textures.push(StylizedTextureRef {
            semantic: StylizedTextureSemantic::Ramp,
            uri: "textures/body_ramp.png".into(),
        });
        descriptor.static_flags.double_sided = true;
        descriptor
            .static_params
            .insert("hair_specular".into(), 0.75);

        let json = serde_json::to_string(&descriptor).expect("serialize");
        let back: StylizedMaterialDescriptor = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(descriptor, back);
    }

    #[test]
    fn instance_and_view_descriptor_serde_roundtrip() {
        let instance = StylizedInstanceDescriptor {
            face_threshold: 0.42,
            profile_index: 7,
            ..StylizedInstanceDescriptor::default()
        };
        let json = serde_json::to_string(&instance).expect("serialize");
        let back: StylizedInstanceDescriptor = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(instance, back);

        let view = StylizedViewDescriptor {
            exposure: 1.25,
            palette: vec![LinearRgba::BLACK, LinearRgba::WHITE],
            ..StylizedViewDescriptor::default()
        };
        let json = serde_json::to_string(&view).expect("serialize");
        let back: StylizedViewDescriptor = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(view, back);
    }

    #[test]
    fn instance_gpu_record_roundtrip_and_descriptor_widening() {
        let record = StylizedInstanceGpuRecord {
            face_threshold: 0.5001, // MeshTag 量化网格外的全精度值
            palette_index: 7,
            light_tint: LinearRgba::new(0.9, 0.95, 1.1, 1.0),
            ..StylizedInstanceGpuRecord::default()
        };
        let json = serde_json::to_string(&record).expect("serialize");
        let back: StylizedInstanceGpuRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(record, back);

        // descriptor → record 是无损 widening（record 为超集）。
        let descriptor = StylizedInstanceDescriptor {
            face_threshold: 0.75,
            face_side: 1,
            profile_index: 9,
            ..StylizedInstanceDescriptor::default()
        };
        let widened = StylizedInstanceGpuRecord::from(descriptor);
        assert_eq!(widened.face_threshold, 0.75);
        assert_eq!(widened.face_side, 1);
        assert_eq!(widened.profile_index, 9);
        assert_eq!(widened.schema_version, SCHEMA_ID_INSTANCE_V1.version);
    }

    #[test]
    fn partial_material_json_uses_schema_defaults() {
        let descriptor: StylizedMaterialDescriptor =
            serde_json::from_str(r#"{"name":"face","surface_class":"hero"}"#)
                .expect("partial payload must deserialize via defaults");
        assert_eq!(descriptor.name, "face");
        assert_eq!(descriptor.surface_class, StylizedSurfaceClass::Hero);
        assert_eq!(descriptor.schema_version, SCHEMA_ID_MATERIAL_V1.version);
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    #[test]
    fn instance_defaults_match_rendering_doc_model() {
        let instance = StylizedInstanceDescriptor::default();
        assert_eq!(instance.face_threshold, 0.5);
        assert_eq!(instance.face_side, 1);
        assert_eq!(instance.outline_scale, 1.0);
        assert_eq!(instance.profile_index, 0);
    }
}
