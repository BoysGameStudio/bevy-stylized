//! Hero@Portable 材质：Character + authored ramp / packed normal /
//! metal matcap / spec ramp（docs/rendering/01 Hero class）。
//!
//! **Provisional（ADR-0001 未决）**：与 cel/character 同属薄层临时 backend。
//!
//! ## 预算（超出 Cel 的 interim 子预算，实际计数见 interim_budget 测试输出）
//!
//! 7 纹理 / 7 sampler / 1 UBO（base + ILM + face SDF + authored ramp +
//! packed normal + matcap + spec ramp）。ILM 通道约定：R=AO、G=hair mask、
//! B=metal mask、A=emission mask。Portable 合法性：全部 fixed binding +
//! filterable float 纹理，无 storage/compute/bindless——超限的是
//! **数量**而非特性；Hero@Portable fallback（[`apply_hero_portable_fallback`]）
//! 关闭 matcap / spec ramp / packed normal 三个最贵路径。
//!
//! ## Face SDF / 逐实例语义
//!
//! 与 Character 一致：共享材质 + MeshTag 实例数据（face 阈值/侧选择、
//! profile 行索引）。

use bevy::{
    pbr::Material,
    prelude::{Asset, Handle, Image, Reflect},
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};

use crate::character::{StylizedCharacterMaterial, StylizedCharacterParams};

/// Hero@Portable 前向着色器（独立编译单元，文件名唯一——naga-oil 对同名
/// 模块有冲突）。
pub const HERO_FORWARD_SHADER_PATH: &str =
    "embedded://bevy_stylized_render/shaders/hero/hero_forward.wgsl";

/// `hero_b.w` 位标志：authored ramp 纹理存在。
pub const HERO_FLAG_RAMP_TEXTURE: u32 = 1 << 0;
/// packed normal 纹理存在。
pub const HERO_FLAG_PACKED_NORMAL: u32 = 1 << 1;
/// metal matcap 启用（纹理存在 + 强度 > 0）。
pub const HERO_FLAG_MATCAP: u32 = 1 << 2;
/// spec ramp 启用。
pub const HERO_FLAG_SPEC_RAMP: u32 = 1 << 3;
/// Hero@Portable fallback 生效（matcap / spec ramp / packed normal 关闭）。
pub const HERO_FLAG_PORTABLE_FALLBACK: u32 = 1 << 4;

/// Hero 材质 GPU uniform：character 参数 + hero 扩展 2×vec4
/// （packed 单 UBO，160B）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect, ShaderType)]
pub struct StylizedHeroParams {
    /// Character 层参数（face/hair/emission/cel 基础）。
    pub character: StylizedCharacterParams,
    /// x = matcap 强度，y = spec ramp 强度，z = ramp atlas 行数，w 预留。
    pub hero_a: bevy::math::Vec4,
    /// rgb = matcap 染色（线性），w = 位标志（bitcast u32，见 `HERO_FLAG_*`）。
    pub hero_b: bevy::math::Vec4,
}

impl StylizedHeroParams {
    /// 设置 metal matcap（强度 0 即关闭）。
    #[must_use]
    pub fn with_matcap(mut self, intensity: f32, tint: bevy::color::LinearRgba) -> Self {
        self.hero_a.x = intensity.max(0.0);
        self.hero_b.x = tint.red;
        self.hero_b.y = tint.green;
        self.hero_b.z = tint.blue;
        self.set_flag(HERO_FLAG_MATCAP, intensity > 0.0);
        self
    }

    /// 设置 spec ramp 强度（0 即关闭）。
    #[must_use]
    pub fn with_spec_ramp_intensity(mut self, intensity: f32) -> Self {
        self.hero_a.y = intensity.max(0.0);
        self.set_flag(HERO_FLAG_SPEC_RAMP, intensity > 0.0);
        self
    }

    /// authored ramp atlas 行数（profile_index 行选择除数）。
    #[must_use]
    pub fn with_ramp_rows(mut self, rows: f32) -> Self {
        self.hero_a.z = rows.max(1.0);
        self
    }

    /// 读取 hero 标志位。
    #[must_use]
    pub fn has_flag(self, bit: u32) -> bool {
        self.hero_b.w.to_bits() & bit != 0
    }

    fn set_flag(&mut self, bit: u32, enabled: bool) {
        let bits = self.hero_b.w.to_bits();
        self.hero_b.w = f32::from_bits(if enabled { bits | bit } else { bits & !bit });
    }
}

/// Hero@Portable fallback（显式降级定义，provisional pending It-7
/// fallback 链）：关闭 matcap / spec ramp / packed normal 三条最贵路径，
/// 保留 authored ramp + face SDF + ILM。
#[must_use]
pub fn apply_hero_portable_fallback(mut params: StylizedHeroParams) -> StylizedHeroParams {
    params.set_flag(HERO_FLAG_PORTABLE_FALLBACK, true);
    params.set_flag(HERO_FLAG_MATCAP, false);
    params.set_flag(HERO_FLAG_SPEC_RAMP, false);
    params.set_flag(HERO_FLAG_PACKED_NORMAL, false);
    params
}

/// 由 resolver 结果决定是否应用 fallback（被降级即应用；provisional）。
#[must_use]
pub fn hero_params_for_variant(
    mut params: StylizedHeroParams,
    resolved: &bevy_stylized_core::StylizedResolvedVariant,
) -> StylizedHeroParams {
    if resolved.tier_reduced {
        params = apply_hero_portable_fallback(params);
        params.character =
            crate::character::apply_character_portable_fallback(params.character, 0.25);
        params
    } else {
        params
    }
}

/// Hero 材质的 pipeline 特化键。
#[repr(C)]
#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub struct StylizedHeroMaterialKey {
    /// 双面渲染。
    pub double_sided: bool,
}

impl From<&StylizedHeroMaterial> for StylizedHeroMaterialKey {
    fn from(material: &StylizedHeroMaterial) -> Self {
        Self {
            double_sided: material.double_sided,
        }
    }
}

/// Hero@Portable 材质（共享材质；face/profile 状态逐实例经 MeshTag）。
#[derive(Asset, AsBindGroup, Clone, Debug, Reflect)]
#[bind_group_data(StylizedHeroMaterialKey)]
pub struct StylizedHeroMaterial {
    /// 材质 uniform（见 [`StylizedHeroParams`]）。
    #[uniform(0)]
    pub params: StylizedHeroParams,
    /// 基色纹理。
    #[texture(1)]
    #[sampler(2)]
    pub base_color_texture: Option<Handle<Image>>,
    /// ILM/control 纹理（R=AO，G=hair mask，B=metal mask，A=emission mask）。
    #[texture(3)]
    #[sampler(4)]
    pub ilm_texture: Option<Handle<Image>>,
    /// Face SDF 纹理（共享材质 + 实例侧选择）。
    #[texture(5)]
    #[sampler(6)]
    pub face_sdf_texture: Option<Handle<Image>>,
    /// Authored ramp atlas（行 = profile_index，经 MeshTag）。
    #[texture(7)]
    #[sampler(8)]
    pub ramp_texture: Option<Handle<Image>>,
    /// Packed normal（xy 存储，z 重建）。
    #[texture(9)]
    #[sampler(10)]
    pub packed_normal_texture: Option<Handle<Image>>,
    /// Metal matcap 查找纹理。
    #[texture(11)]
    #[sampler(12)]
    pub matcap_texture: Option<Handle<Image>>,
    /// Specular ramp。
    #[texture(13)]
    #[sampler(14)]
    pub spec_ramp_texture: Option<Handle<Image>>,
    /// 双面渲染（false = 继承默认背面剔除）。
    pub double_sided: bool,
}

impl Default for StylizedHeroMaterial {
    fn default() -> Self {
        let mut material = Self {
            params: StylizedHeroParams::default(),
            base_color_texture: None,
            ilm_texture: None,
            face_sdf_texture: None,
            ramp_texture: None,
            packed_normal_texture: None,
            matcap_texture: None,
            spec_ramp_texture: None,
            double_sided: false,
        };
        material.sync_flags();
        material
    }
}

impl StylizedHeroMaterial {
    /// 同步纹理存在性标志位（资产事件驱动）。
    pub fn sync_flags(&mut self) {
        self.params
            .set_flag(HERO_FLAG_RAMP_TEXTURE, self.ramp_texture.is_some());
        self.params.set_flag(
            HERO_FLAG_PACKED_NORMAL,
            self.packed_normal_texture.is_some(),
        );
        // matcap/spec ramp 由强度控制启用，纹理缺失时强制关闭。
        self.params.set_flag(
            HERO_FLAG_MATCAP,
            self.matcap_texture.is_some() && self.params.hero_a.x > 0.0,
        );
        self.params.set_flag(
            HERO_FLAG_SPEC_RAMP,
            self.spec_ramp_texture.is_some() && self.params.hero_a.y > 0.0,
        );
        // character 层标志（ILM/face SDF）。
        let mut character = StylizedCharacterMaterial {
            params: self.params.character,
            base_color_texture: self.base_color_texture.clone(),
            ilm_texture: self.ilm_texture.clone(),
            face_sdf_texture: self.face_sdf_texture.clone(),
            double_sided: self.double_sided,
        };
        character.sync_flags();
        self.params.character = character.params;
    }

    /// 标志位是否已与纹理句柄一致（避免事件回声）。
    fn flags_consistent(&self) -> bool {
        self.params.has_flag(HERO_FLAG_RAMP_TEXTURE) == self.ramp_texture.is_some()
            && self.params.has_flag(HERO_FLAG_PACKED_NORMAL) == self.packed_normal_texture.is_some()
            && self.params.has_flag(HERO_FLAG_MATCAP)
                == (self.matcap_texture.is_some() && self.params.hero_a.x > 0.0)
            && self.params.has_flag(HERO_FLAG_SPEC_RAMP)
                == (self.spec_ramp_texture.is_some() && self.params.hero_a.y > 0.0)
    }
}

impl Material for StylizedHeroMaterial {
    fn vertex_shader() -> ShaderRef {
        HERO_FORWARD_SHADER_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        HERO_FORWARD_SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> bevy::material::AlphaMode {
        if self
            .params
            .character
            .cel
            .has_flag(crate::material::CEL_FLAG_ALPHA_MASK)
        {
            bevy::material::AlphaMode::Mask(self.params.character.cel.rim.w)
        } else {
            bevy::material::AlphaMode::Opaque
        }
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        if key.bind_group_data.double_sided {
            descriptor.primitive.cull_mode = None;
        }
        Ok(())
    }
}

/// 材质资产变更后同步 hero 标志位（仅不一致时写入，避免事件回声）。
pub fn sync_hero_material_flags(
    mut events: bevy::ecs::message::MessageReader<bevy::asset::AssetEvent<StylizedHeroMaterial>>,
    mut materials: bevy::prelude::ResMut<bevy::prelude::Assets<StylizedHeroMaterial>>,
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
    fn default_flags_are_clear() {
        let material = StylizedHeroMaterial::default();
        assert!(!material.params.has_flag(HERO_FLAG_RAMP_TEXTURE));
        assert!(!material.params.has_flag(HERO_FLAG_MATCAP));
        assert!(!material.params.has_flag(HERO_FLAG_PORTABLE_FALLBACK));
    }

    #[test]
    fn portable_fallback_disables_expensive_paths() {
        let mut material = StylizedHeroMaterial {
            matcap_texture: Some(Handle::default()),
            spec_ramp_texture: Some(Handle::default()),
            packed_normal_texture: Some(Handle::default()),
            ..StylizedHeroMaterial::default()
        };
        material.params = material
            .params
            .with_matcap(0.8, bevy::color::LinearRgba::WHITE);
        material.params = material.params.with_spec_ramp_intensity(0.6);
        material.sync_flags();
        assert!(material.params.has_flag(HERO_FLAG_MATCAP));

        material.params = apply_hero_portable_fallback(material.params);
        assert!(material.params.has_flag(HERO_FLAG_PORTABLE_FALLBACK));
        assert!(!material.params.has_flag(HERO_FLAG_MATCAP));
        assert!(!material.params.has_flag(HERO_FLAG_SPEC_RAMP));
        assert!(!material.params.has_flag(HERO_FLAG_PACKED_NORMAL));
    }
}
