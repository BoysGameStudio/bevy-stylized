//! Cel@Portable 前向主材质（It-3b 切片）。
//!
//! 薄层 backend（ADR-0001 Accepted）：本切片建立在 Bevy `Material` 组合路径
//! 上，是薄层临时 backend；所有公共语义挂在 core 的
//! [`StylizedMaterialDescriptor`] 上，backend 相关返工被
//! 限制在本 crate 内部。
//!
//! Portable 约束（docs/rendering/01、治理不变量 6/7/8）：
//!
//! - 不依赖 bindless / FP16 / subgroup / compute / HDR；
//! - 固定 binding：1 UBO + 3 texture + 3 sampler（材质自身子预算
//!   ≤2 UBO / ≤6 texture / ≤3 sampler，interim 阈值 pending ADR-0002）；
//! - 无 per-frame material asset mutation：View 层数据经
//!   [`crate::view`] 同步进 Bevy 的 `DirectionalLight` / 相机 `Exposure`，
//!   shader 从 `mesh_view_bindings` 只读。

use bevy::{
    pbr::Material,
    prelude::{Asset, Handle, Image, Reflect},
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use bevy_stylized_core::{StylizedMaterialDescriptor, StylizedTextureSemantic};

/// Cel@Portable 前向着色器（单个编译单元，vertex+fragment 同文件；
/// 不引入任何 Character/Hero-only 代码，docs/rendering/03）。
pub const CEL_PORTABLE_FORWARD_SHADER_PATH: &str =
    "embedded://bevy_stylized_render/shaders/cel/portable_forward.wgsl";

/// `params.flags` 位：ramp 纹理存在（否则走解析 ramp fallback）。
pub const CEL_FLAG_RAMP_TEXTURE: u32 = 1 << 0;
/// `params.flags` 位：ILM/control 纹理存在（R 通道作为 AO）。
pub const CEL_FLAG_ILM_TEXTURE: u32 = 1 << 1;
/// `params.flags` 位：启用 rim。
pub const CEL_FLAG_RIM: u32 = 1 << 2;
/// `params.flags` 位：alpha mask（`rim.w` 为 cutoff；对应源资产的
/// Blend/Mask 在 Cel@Portable 切片中统一收敛为 Mask，完整 alpha 矩阵
/// ADR-0023）。
pub const CEL_FLAG_ALPHA_MASK: u32 = 1 << 3;

/// Cel 材质的 GPU uniform（单个 64 字节 UBO，packed by Class×Tier，
/// 禁止"每个 feature 一个 UBO"）。
#[derive(Clone, Copy, Debug, PartialEq, Reflect, ShaderType)]
pub struct StylizedCelParams {
    /// 基色因子（线性 RGBA 乘子）。
    pub base_color_factor: bevy::math::Vec4,
    /// x: 阴影阈值，y: 阴影软过渡宽度，z: 阴影下限亮度，w: 标志位
    /// （[`CEL_FLAG_RAMP_TEXTURE`] / [`CEL_FLAG_ILM_TEXTURE`] / [`CEL_FLAG_RIM`]）。
    pub shading: bevy::math::Vec4,
    /// x: rim 幂次，y: rim 强度，z: ramp 纹理行数（MeshTag 行选择的除数），
    /// w: alpha mask cutoff（0 = opaque）。
    pub rim: bevy::math::Vec4,
    /// rim 颜色（线性 rgb，a 预留）。
    pub rim_color: bevy::math::Vec4,
}

impl Default for StylizedCelParams {
    fn default() -> Self {
        Self {
            base_color_factor: bevy::math::Vec4::ONE,
            // threshold 0.5 / softness 0.1 / floor 0.3 / flags 0
            shading: bevy::math::Vec4::new(0.5, 0.1, 0.3, 0.0),
            rim: bevy::math::Vec4::new(3.0, 0.0, 1.0, 0.0),
            rim_color: bevy::math::Vec4::ONE,
        }
    }
}

impl StylizedCelParams {
    /// 设置阴影阈值/软过渡/下限亮度。
    #[must_use]
    pub fn with_shadow(mut self, threshold: f32, softness: f32, floor: f32) -> Self {
        self.shading.x = threshold.clamp(0.0, 1.0);
        self.shading.y = softness.max(0.0001);
        self.shading.z = floor.clamp(0.0, 1.0);
        self
    }

    /// 设置 rim 参数（强度 0 即关闭）。
    #[must_use]
    pub fn with_rim(mut self, power: f32, intensity: f32, color: bevy::color::LinearRgba) -> Self {
        self.rim.x = power.max(0.0);
        self.rim.y = intensity.max(0.0);
        self.rim_color = bevy::math::Vec4::new(color.red, color.green, color.blue, 1.0);
        self.set_flag(CEL_FLAG_RIM, intensity > 0.0);
        self
    }

    /// 设置 ramp 纹理行数（`MeshTag` 低 8 位作为行索引的除数；
    /// provisional ADR-0004 方案 A）。
    #[must_use]
    pub fn with_ramp_rows(mut self, rows: f32) -> Self {
        self.rim.z = rows.max(1.0);
        self
    }

    /// 启用 alpha mask（cutoff 以下的片元被 discard；0 回到 opaque）。
    #[must_use]
    pub fn with_alpha_cutoff(mut self, cutoff: f32) -> Self {
        self.rim.w = cutoff.clamp(0.0, 1.0);
        self.set_flag(CEL_FLAG_ALPHA_MASK, cutoff > 0.0);
        self
    }

    /// 读取标志位。
    #[must_use]
    pub fn has_flag(self, bit: u32) -> bool {
        self.shading.w.to_bits() & bit != 0
    }

    fn set_flag(&mut self, bit: u32, enabled: bool) {
        let bits = self.shading.w.to_bits();
        self.shading.w = f32::from_bits(if enabled { bits | bit } else { bits & !bit });
    }
}

/// Cel 材质的 pipeline 特化键（cull 继承语义）。
#[repr(C)]
#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub struct StylizedCelMaterialKey {
    /// 双面渲染（发卡/布料等）。
    pub double_sided: bool,
}

impl From<&StylizedCelMaterial> for StylizedCelMaterialKey {
    fn from(material: &StylizedCelMaterial) -> Self {
        Self {
            double_sided: material.double_sided,
        }
    }
}

/// Cel@Portable 主材质。
///
/// Ramp 缺失时走解析 ramp（shader 内 smoothstep），不生成临时纹理资产。
/// `double_sided=false` 时 cull 继承 mesh pipeline 默认（背面剔除）。
#[derive(Asset, AsBindGroup, Clone, Debug, Reflect)]
#[bind_group_data(StylizedCelMaterialKey)]
pub struct StylizedCelMaterial {
    /// 材质 uniform（见 [`StylizedCelParams`]）。
    #[uniform(0)]
    pub params: StylizedCelParams,
    /// 基色纹理（缺失时由 Bevy fallback 白色纹理 × `base_color_factor`）。
    #[texture(1)]
    #[sampler(2)]
    pub base_color_texture: Option<Handle<Image>>,
    /// Toon ramp 纹理（行 = profile 索引 via `MeshTag`；缺失时解析 ramp）。
    #[texture(3)]
    #[sampler(4)]
    pub ramp_texture: Option<Handle<Image>>,
    /// ILM/control 纹理（当前切片只用 R 通道 AO）。
    #[texture(5)]
    #[sampler(6)]
    pub ilm_texture: Option<Handle<Image>>,
    /// 双面渲染（false = 继承默认背面剔除）。
    pub double_sided: bool,
}

impl Default for StylizedCelMaterial {
    fn default() -> Self {
        let mut material = Self {
            params: StylizedCelParams::default(),
            base_color_texture: None,
            ramp_texture: None,
            ilm_texture: None,
            double_sided: false,
        };
        material.sync_flags();
        material
    }
}

impl StylizedCelMaterial {
    /// 根据纹理句柄存在性同步 `params` 标志位（uniform 内容随资产事件
    /// 重新上传，见 [`sync_cel_material_flags`]）。
    pub fn sync_flags(&mut self) {
        self.params
            .set_flag(CEL_FLAG_RAMP_TEXTURE, self.ramp_texture.is_some());
        self.params
            .set_flag(CEL_FLAG_ILM_TEXTURE, self.ilm_texture.is_some());
    }

    /// 标志位是否已与纹理句柄一致（避免事件回声导致的重复写）。
    fn flags_consistent(&self) -> bool {
        self.params.has_flag(CEL_FLAG_RAMP_TEXTURE) == self.ramp_texture.is_some()
            && self.params.has_flag(CEL_FLAG_ILM_TEXTURE) == self.ilm_texture.is_some()
    }

    /// 从 core 的 [`StylizedMaterialDescriptor`] 构建（公开语义在 core，
    /// render 只做薄转换）。
    ///
    /// `load_texture` 把 descriptor 的 texture uri 解析为 `Handle<Image>`
    /// （通常是 `AssetServer::load` 的闭包）。
    #[must_use]
    pub fn from_descriptor(
        descriptor: &StylizedMaterialDescriptor,
        mut load_texture: impl FnMut(&str) -> Handle<Image>,
    ) -> Self {
        let mut material = StylizedCelMaterial::default();
        for texture in &descriptor.textures {
            match texture.semantic {
                StylizedTextureSemantic::Color => {
                    material.base_color_texture = Some(load_texture(&texture.uri));
                }
                StylizedTextureSemantic::Ramp => {
                    material.ramp_texture = Some(load_texture(&texture.uri));
                }
                StylizedTextureSemantic::ContinuousMask => {
                    material.ilm_texture = Some(load_texture(&texture.uri));
                }
                // Normal / DiscreteClassification / MatCap 属于
                // Character/Hero 语义，Cel@Portable 切片不消费。
                _ => {}
            }
        }
        material.double_sided = descriptor.static_flags.double_sided;
        let params = &descriptor.static_params;
        material.params.base_color_factor = bevy::math::Vec4::new(
            params.get("base_color_r").copied().unwrap_or(1.0),
            params.get("base_color_g").copied().unwrap_or(1.0),
            params.get("base_color_b").copied().unwrap_or(1.0),
            params.get("base_color_a").copied().unwrap_or(1.0),
        );
        if descriptor.static_flags.alpha_test {
            material.params = material
                .params
                .with_alpha_cutoff(params.get("alpha_cutoff").copied().unwrap_or(0.5));
        }
        let threshold = params.get("shadow_threshold").copied().unwrap_or(0.5);
        let softness = params.get("shadow_softness").copied().unwrap_or(0.1);
        let floor = params.get("shadow_floor").copied().unwrap_or(0.3);
        material.params = material.params.with_shadow(threshold, softness, floor);
        if let Some(&intensity) = params.get("rim_intensity") {
            let power = params.get("rim_power").copied().unwrap_or(3.0);
            material.params =
                material
                    .params
                    .with_rim(power, intensity, bevy::color::LinearRgba::WHITE);
        }
        material.sync_flags();
        material
    }
}

impl Material for StylizedCelMaterial {
    fn vertex_shader() -> ShaderRef {
        CEL_PORTABLE_FORWARD_SHADER_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        CEL_PORTABLE_FORWARD_SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> bevy::material::AlphaMode {
        if self.params.has_flag(CEL_FLAG_ALPHA_MASK) {
            bevy::material::AlphaMode::Mask(self.params.rim.w)
        } else {
            bevy::material::AlphaMode::Opaque
        }
    }

    // Cel prepass 由相机选择，shadow 路径关闭；完整语义矩阵仍 pending
    // ADR-0023（Accepted）：类型级可参与不等于所有组合已资格化。
    // prepass 参与在类型级开启（共享裁切/完整蒙皮 shader）；**成本门控在
    // 相机**：只有带 DepthPrepass/NormalPrepass 组件的相机才产生 prepass
    // 开销（StylizedScreenOutline 的 per-camera opt-in），默认相机零成本。
    fn prepass_vertex_shader() -> ShaderRef {
        crate::skinning::COMPLETE_SKIN_PREPASS_SHADER.into()
    }

    fn prepass_fragment_shader() -> ShaderRef {
        crate::skinning::COMPLETE_SKIN_PREPASS_SHADER.into()
    }

    fn enable_prepass() -> bool {
        true
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        crate::skinning::specialize_secondary_skin(descriptor, layout)?;
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader_defs.push("NPR_PREPASS_CEL".into());
        }
        // 双面渲染经 bind_group_data 特化（cull 继承 = 默认背面剔除）。
        if key.bind_group_data.double_sided {
            descriptor.primitive.cull_mode = None;
        }
        Ok(())
    }
}

/// 材质资产变更后同步标志位（`Option<Handle>` → shader flag）。
pub fn sync_cel_material_flags(
    mut events: bevy::ecs::message::MessageReader<bevy::asset::AssetEvent<StylizedCelMaterial>>,
    mut materials: bevy::prelude::ResMut<bevy::prelude::Assets<StylizedCelMaterial>>,
) {
    for event in events.read() {
        let id = match *event {
            bevy::asset::AssetEvent::Added { id }
            | bevy::asset::AssetEvent::Modified { id }
            | bevy::asset::AssetEvent::LoadedWithDependencies { id } => id,
            bevy::asset::AssetEvent::Removed { .. } | bevy::asset::AssetEvent::Unused { .. } => {
                continue;
            }
        };
        // 只在标志位确实不一致时写入，避免 Modified 事件回声。
        if materials.get(id).is_some_and(|m| m.flags_consistent()) {
            continue;
        }
        if let Some(mut material) = materials.get_mut(id) {
            material.sync_flags();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_material_has_all_texture_flags_clear() {
        let material = StylizedCelMaterial::default();
        assert!(!material.params.has_flag(CEL_FLAG_RAMP_TEXTURE));
        assert!(!material.params.has_flag(CEL_FLAG_ILM_TEXTURE));
        assert!(!material.params.has_flag(CEL_FLAG_RIM));
    }

    #[test]
    fn sync_flags_reflect_texture_handles() {
        let mut material = StylizedCelMaterial {
            ramp_texture: Some(Handle::default()),
            ..StylizedCelMaterial::default()
        };
        material.sync_flags();
        assert!(material.params.has_flag(CEL_FLAG_RAMP_TEXTURE));
        assert!(!material.params.has_flag(CEL_FLAG_ILM_TEXTURE));
    }

    #[test]
    fn from_descriptor_maps_semantics_to_textures() {
        let descriptor = StylizedMaterialDescriptor {
            textures: vec![
                bevy_stylized_core::StylizedTextureRef {
                    semantic: StylizedTextureSemantic::Color,
                    uri: "base.png".into(),
                },
                bevy_stylized_core::StylizedTextureRef {
                    semantic: StylizedTextureSemantic::Ramp,
                    uri: "ramp.png".into(),
                },
                bevy_stylized_core::StylizedTextureRef {
                    semantic: StylizedTextureSemantic::ContinuousMask,
                    uri: "ilm.png".into(),
                },
                // Character/Hero 语义在 Cel 切片被忽略。
                bevy_stylized_core::StylizedTextureRef {
                    semantic: StylizedTextureSemantic::MatCap,
                    uri: "ignored.png".into(),
                },
            ],
            static_params: [("shadow_threshold".to_string(), 0.42_f32)]
                .into_iter()
                .collect(),
            ..StylizedMaterialDescriptor::default()
        };
        let mut loaded = Vec::new();
        let material = StylizedCelMaterial::from_descriptor(&descriptor, |uri| {
            loaded.push(uri.to_string());
            Handle::default()
        });
        assert!(material.base_color_texture.is_some());
        assert!(material.ramp_texture.is_some());
        assert!(material.ilm_texture.is_some());
        assert_eq!(loaded, ["base.png", "ramp.png", "ilm.png"]);
        assert!((material.params.shading.x - 0.42).abs() < f32::EPSILON);
        assert!(material.params.has_flag(CEL_FLAG_RAMP_TEXTURE));
        assert!(material.params.has_flag(CEL_FLAG_ILM_TEXTURE));
    }
}
