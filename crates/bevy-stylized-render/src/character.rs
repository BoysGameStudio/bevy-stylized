//! Character@Portable 材质：Cel + Face SDF / Hair / Rim / Emission
//! （docs/rendering/01 Character class 定义）。
//!
//! **Provisional（ADR-0001 未决）**：与 [`crate::material::StylizedCelMaterial`]
//! 同属薄层临时 backend。
//!
//! ## 关键设计
//!
//! - **Face SDF 用共享材质 + per-instance 状态**：阈值/侧选择经
//!   [`crate::instance::StylizedInstanceData`]（MeshTag 通道）逐实例传入，
//!   绝不为此克隆 unique material（rendering/02 的禁令）；
//! - **预算**：3 纹理 / 3 sampler / 1 UBO（base + ILM + face SDF）；
//!   ILM 通道布局（docs/reference/07 §3.2，已核实）：**R = 金属遮罩
//!   （>0.9 阈值）、G = AO、B = 高光抑制（因子 1.03−B）、A = 材质分带
//!   ID**（ramp 分带阈值 0.2/0.4/0.6/0.8）；顶点色层：R = vertex AO、
//!   G = ramp 阴影宽度缩放。Emission mask 走 base color alpha（上游经
//!   alpha switch `0=Emission,1=Transparency` 路由——我们没有 ILM
//!   emission 通道；此决策为 provisional，pending ADR-0023 矩阵）；
//! - **Character@Portable fallback**（显式降级定义，非静默）：
//!   - Face SDF 缺失或 fallback 标志 → 半兰伯特 step 近似（复用 Cel 阴影）；
//!   - Hair 高光缺失/关闭 → rim 近似（用 rim 通道补偿）。
//!
//! ## Face basis 约定（provisional）
//!
//! v1 假定角色直立：face forward/right 取**水平面（XZ）投影**。
//! `face_forward` 存于材质（局部空间，随 mesh transform 变换）；逐骨骼
//! 驱动的 face rig 属于后续迭代（见 rendering/02 的 Face SDF 演化注记）。

use bevy::{
    pbr::Material,
    prelude::{Asset, Handle, Image, Reflect},
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use bevy_stylized_core::{StylizedMaterialDescriptor, StylizedTextureSemantic};

use crate::material::{CEL_FLAG_ALPHA_MASK, StylizedCelParams};

/// Character@Portable 前向着色器（独立编译单元，不含 Hero-only 代码）。
pub const CHARACTER_PORTABLE_FORWARD_SHADER_PATH: &str =
    "embedded://bevy_stylized_render/shaders/character/character_forward.wgsl";

/// `face_misc.y` 位标志：ILM 纹理存在。
pub const CHAR_FLAG_ILM_TEXTURE: u32 = 1 << 0;
/// `face_misc.y` 位标志：Face SDF 纹理存在。
pub const CHAR_FLAG_FACE_SDF_TEXTURE: u32 = 1 << 1;
/// `face_misc.y` 位标志：Hair 高光启用（mask = 顶点色 G？否——
/// ILM 无 hair 通道；hair mask 取 ILM.A? 见 shader 注释）。
pub const CHAR_FLAG_HAIR_SPECULAR: u32 = 1 << 2;
/// `face_misc.y` 位标志：Emission 启用（ILM.A 为 mask）。
pub const CHAR_FLAG_EMISSION: u32 = 1 << 3;
/// `face_misc.y` 位标志：Character@Portable fallback（face SDF → step
/// 近似；hair 高光 → rim 近似）。由 resolver 结果驱动设置，
/// 见 [`apply_character_portable_fallback`]。
pub const CHAR_FLAG_PORTABLE_FALLBACK: u32 = 1 << 4;

/// Character 材质 GPU uniform（cel 基础 + character 扩展，packed，
/// 禁止 per-feature UBO）。
///
/// 布局（13×vec4 = 208B，单 UBO）：cel 基础 4 + face_forward/face_misc/
/// hair/emission 4 + palette 4（sharp/soft × lit/shadow）+ face_up。
///
/// Palette 语义（docs/reference/07 §3.7，已核实）：sharp/soft 双档光色与
/// 双档阴影色为**串联相乘**结构——soft = lerp(softShadow, softLit,
/// litFactor)，sharp = (litFactor > 0.5) ? sharpLit : sharpShadow，
/// 最终 = base × soft × sharp × ambient。上游这些色值是 authored 全局
/// 材质属性（`Global Material Properties` 节点组），故放材质层（静态，
/// 非 per-frame 写）。
#[derive(Clone, Copy, Debug, PartialEq, Reflect, ShaderType)]
pub struct StylizedCharacterParams {
    /// 与 [`StylizedCelParams`] 相同语义的基础参数（base/rim/alpha mask）。
    pub cel: StylizedCelParams,
    /// face basis：xyz = 局部空间 face forward，w = face SDF 软过渡宽度。
    pub face_forward_softness: bevy::math::Vec4,
    /// x = 材质级 face 阈值基准，y = 位标志（bitcast u32，见
    /// `CHAR_FLAG_*`），z = side 强制模式（0 自动 / 1 翻转），
    /// w = day/night 因子（上游为材质属性 `Global Material
    /// Properties.Day/Night`——docs/reference/07 §3.1；材质静态携带，
    /// 昼夜对 ramp 行分带生效）。
    pub face_misc: bevy::math::Vec4,
    /// x = hair 高光幂次，y = hair 高光强度，zw 预留。
    pub hair: bevy::math::Vec4,
    /// rgb = emission 颜色，w = emission 强度。
    pub emission: bevy::math::Vec4,
    /// Palette：soft 档受光色。
    pub palette_soft_lit: bevy::math::Vec4,
    /// Palette：soft 档阴影色。
    pub palette_soft_shadow: bevy::math::Vec4,
    /// Palette：sharp 档受光色。
    pub palette_sharp_lit: bevy::math::Vec4,
    /// Palette：sharp 档阴影色。
    pub palette_sharp_shadow: bevy::math::Vec4,
    /// face basis 的 up 向量（head basis：headright =
    /// normalize(cross(forward, up))，docs/reference/07 §3.3 已核实）。
    pub face_up: bevy::math::Vec4,
}

impl Default for StylizedCharacterParams {
    fn default() -> Self {
        Self {
            cel: StylizedCelParams::default(),
            face_forward_softness: bevy::math::Vec4::new(0.0, 0.0, 1.0, 0.02),
            face_misc: bevy::math::Vec4::new(0.5, 0.0, 0.0, 1.0),
            hair: bevy::math::Vec4::new(24.0, 0.0, 0.0, 0.0),
            emission: bevy::math::Vec4::ZERO,
            // 中性默认：lit=白；shadow=中灰（视觉效果接近旧 ambient+lit 结构，
            // golden 重捕获以此为准）。
            palette_soft_lit: bevy::math::Vec4::ONE,
            palette_soft_shadow: bevy::math::Vec4::new(0.55, 0.55, 0.62, 1.0),
            palette_sharp_lit: bevy::math::Vec4::ONE,
            palette_sharp_shadow: bevy::math::Vec4::new(0.7, 0.7, 0.75, 1.0),
            face_up: bevy::math::Vec4::Y,
        }
    }
}

impl StylizedCharacterParams {
    /// 设置 face basis（局部空间 forward + up）与 SDF 软过渡宽度。
    #[must_use]
    pub fn with_face_basis(
        mut self,
        forward: bevy::math::Vec3,
        up: bevy::math::Vec3,
        softness: f32,
    ) -> Self {
        self.face_forward_softness =
            bevy::math::Vec4::new(forward.x, forward.y, forward.z, softness.max(0.0001));
        self.face_up = bevy::math::Vec4::new(up.x, up.y, up.z, 0.0);
        self
    }

    /// 设置 face basis（仅 forward；up 默认 +Y）。
    #[must_use]
    pub fn with_face_forward(mut self, forward: bevy::math::Vec3, softness: f32) -> Self {
        self.face_forward_softness =
            bevy::math::Vec4::new(forward.x, forward.y, forward.z, softness.max(0.0001));
        self
    }

    /// 设置材质级 face SDF 阈值基准（实例可经 MeshTag 覆盖）。
    #[must_use]
    pub fn with_face_threshold_bias(mut self, bias: f32) -> Self {
        self.face_misc.x = bias.clamp(0.0, 1.0);
        self
    }

    /// 设置 day/night 因子（材质静态携带；上游为材质属性，见
    /// `face_misc` 文档）。
    #[must_use]
    pub fn with_day_night(mut self, factor: f32) -> Self {
        self.face_misc.w = factor.clamp(0.0, 1.0);
        self
    }

    /// 设置 palette 四色（sharp/soft × lit/shadow）。
    #[must_use]
    pub fn with_palette(
        mut self,
        soft_lit: bevy::color::LinearRgba,
        soft_shadow: bevy::color::LinearRgba,
        sharp_lit: bevy::color::LinearRgba,
        sharp_shadow: bevy::color::LinearRgba,
    ) -> Self {
        let v = |c: bevy::color::LinearRgba| bevy::math::Vec4::new(c.red, c.green, c.blue, 1.0);
        self.palette_soft_lit = v(soft_lit);
        self.palette_soft_shadow = v(soft_shadow);
        self.palette_sharp_lit = v(sharp_lit);
        self.palette_sharp_shadow = v(sharp_shadow);
        self
    }

    /// 设置 hair 高光（强度 0 即关闭）。
    #[must_use]
    pub fn with_hair_specular(mut self, power: f32, intensity: f32) -> Self {
        self.hair.x = power.max(1.0);
        self.hair.y = intensity.max(0.0);
        self.set_flag(CHAR_FLAG_HAIR_SPECULAR, intensity > 0.0);
        self
    }

    /// 设置 emission（mask 通道 = base color alpha，见 emission 文档与
    /// ILM 布局注记）。
    #[must_use]
    pub fn with_emission(mut self, color: bevy::color::LinearRgba, intensity: f32) -> Self {
        self.emission =
            bevy::math::Vec4::new(color.red, color.green, color.blue, intensity.max(0.0));
        self.set_flag(CHAR_FLAG_EMISSION, intensity > 0.0);
        self
    }

    /// 读取扩展标志位。
    #[must_use]
    pub fn has_flag(self, bit: u32) -> bool {
        self.face_misc.y.to_bits() & bit != 0
    }

    fn set_flag(&mut self, bit: u32, enabled: bool) {
        let bits = self.face_misc.y.to_bits();
        self.face_misc.y = f32::from_bits(if enabled { bits | bit } else { bits & !bit });
    }
}

/// 应用 Character@Portable fallback 标志（显式降级，由 resolver 结果驱动；
/// provisional pending ADR-0018/0001 的 fallback 链定稿）。
///
/// - face SDF → step 硬阈值近似（少一次 smoothstep 精度开销）；
/// - hair 高光 → 关闭 specular，rim 强度补偿（`rim_intensity`）。
#[must_use]
pub fn apply_character_portable_fallback(
    mut params: StylizedCharacterParams,
    rim_intensity: f32,
) -> StylizedCharacterParams {
    params.set_flag(CHAR_FLAG_PORTABLE_FALLBACK, true);
    params.set_flag(CHAR_FLAG_HAIR_SPECULAR, false);
    params.cel = params
        .cel
        .with_rim(2.0, rim_intensity, bevy::color::LinearRgba::WHITE);
    params
}

/// 由 resolver 结果决定是否应用 fallback（tier 被降到 Portable 以下…
/// 当前定义：请求档被降级即应用——provisional，pending It-7 的
/// fallback 链）。
#[must_use]
pub fn character_params_for_variant(
    params: StylizedCharacterParams,
    resolved: &bevy_stylized_core::StylizedResolvedVariant,
) -> StylizedCharacterParams {
    if resolved.tier_reduced {
        apply_character_portable_fallback(params, 0.25)
    } else {
        params
    }
}

/// Character 材质的 pipeline 特化键。
#[repr(C)]
#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub struct StylizedCharacterMaterialKey {
    /// 双面渲染。
    pub double_sided: bool,
}

impl From<&StylizedCharacterMaterial> for StylizedCharacterMaterialKey {
    fn from(material: &StylizedCharacterMaterial) -> Self {
        Self {
            double_sided: material.double_sided,
        }
    }
}

/// Character@Portable 材质（共享材质；face 状态逐实例经 MeshTag）。
#[derive(Asset, AsBindGroup, Clone, Debug, Reflect)]
#[bind_group_data(StylizedCharacterMaterialKey)]
pub struct StylizedCharacterMaterial {
    /// 材质 uniform（见 [`StylizedCharacterParams`]）。
    #[uniform(0)]
    pub params: StylizedCharacterParams,
    /// 基色纹理。
    #[texture(1)]
    #[sampler(2)]
    pub base_color_texture: Option<Handle<Image>>,
    /// ILM/control 纹理（R=AO，G=hair mask，A=emission mask）。
    #[texture(3)]
    #[sampler(4)]
    pub ilm_texture: Option<Handle<Image>>,
    /// Face SDF 纹理（单张；左右侧经 UV 镜像共享，docs/rendering/03
    /// "选择 Side 后只采一张目标 SDF"）。
    #[texture(5)]
    #[sampler(6)]
    pub face_sdf_texture: Option<Handle<Image>>,
    /// 双面渲染（false = 继承默认背面剔除）。
    pub double_sided: bool,
}

impl Default for StylizedCharacterMaterial {
    fn default() -> Self {
        let mut material = Self {
            params: StylizedCharacterParams::default(),
            base_color_texture: None,
            ilm_texture: None,
            face_sdf_texture: None,
            double_sided: false,
        };
        material.sync_flags();
        material
    }
}

impl StylizedCharacterMaterial {
    /// 同步纹理存在性标志位（资产事件驱动，见
    /// [`sync_character_material_flags`]）。
    pub fn sync_flags(&mut self) {
        self.params
            .set_flag(CHAR_FLAG_ILM_TEXTURE, self.ilm_texture.is_some());
        self.params
            .set_flag(CHAR_FLAG_FACE_SDF_TEXTURE, self.face_sdf_texture.is_some());
    }

    /// 标志位是否已与纹理句柄一致（避免事件回声导致的重复写）。
    fn flags_consistent(&self) -> bool {
        self.params.has_flag(CHAR_FLAG_ILM_TEXTURE) == self.ilm_texture.is_some()
            && self.params.has_flag(CHAR_FLAG_FACE_SDF_TEXTURE) == self.face_sdf_texture.is_some()
    }

    /// 从 core 的 [`StylizedMaterialDescriptor`] 构建（薄转换）。
    ///
    /// 语义映射：Color→base，ContinuousMask→ILM，DiscreteClassification→
    /// face SDF（face SDF 是离散阈值数据）。`static_params` 键：
    /// `face_forward_x/y/z`、`face_softness`、`face_threshold_bias`、
    /// `hair_specular_power/intensity`、`emission_r/g/b/intensity`。
    #[must_use]
    pub fn from_descriptor(
        descriptor: &StylizedMaterialDescriptor,
        mut load_texture: impl FnMut(&str) -> Handle<Image>,
    ) -> Self {
        let mut material = StylizedCharacterMaterial::default();
        for texture in &descriptor.textures {
            match texture.semantic {
                StylizedTextureSemantic::Color => {
                    material.base_color_texture = Some(load_texture(&texture.uri));
                }
                StylizedTextureSemantic::ContinuousMask => {
                    material.ilm_texture = Some(load_texture(&texture.uri));
                }
                StylizedTextureSemantic::DiscreteClassification => {
                    material.face_sdf_texture = Some(load_texture(&texture.uri));
                }
                _ => {}
            }
        }
        let params = &descriptor.static_params;
        let get = |key: &str| params.get(key).copied();
        if let (Some(x), Some(y), Some(z)) = (
            get("face_forward_x"),
            get("face_forward_y"),
            get("face_forward_z"),
        ) {
            material.params = material.params.with_face_forward(
                bevy::math::Vec3::new(x, y, z),
                get("face_softness").unwrap_or(0.02),
            );
        }
        if let Some(bias) = get("face_threshold_bias") {
            material.params = material.params.with_face_threshold_bias(bias);
        }
        if let Some(intensity) = get("hair_specular_intensity") {
            material.params = material
                .params
                .with_hair_specular(get("hair_specular_power").unwrap_or(24.0), intensity);
        }
        if let Some(intensity) = get("emission_intensity") {
            material.params = material.params.with_emission(
                bevy::color::LinearRgba::new(
                    get("emission_r").unwrap_or(1.0),
                    get("emission_g").unwrap_or(1.0),
                    get("emission_b").unwrap_or(1.0),
                    1.0,
                ),
                intensity,
            );
        }
        material.sync_flags();
        material
    }
}

impl Material for StylizedCharacterMaterial {
    fn vertex_shader() -> ShaderRef {
        CHARACTER_PORTABLE_FORWARD_SHADER_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        CHARACTER_PORTABLE_FORWARD_SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> bevy::material::AlphaMode {
        if self.params.cel.has_flag(CEL_FLAG_ALPHA_MASK) {
            bevy::material::AlphaMode::Mask(self.params.cel.rim.w)
        } else {
            bevy::material::AlphaMode::Opaque
        }
    }

    // 与 Cel 切片一致：prepass/shadow 显式关闭（pending ADR-0023）。
    // prepass 参与在类型级开启（默认 prepass shader 即可）；**成本门控在
    // 相机**：只有带 DepthPrepass/NormalPrepass 组件的相机才产生 prepass
    // 开销（StylizedScreenOutline 的 per-camera opt-in），默认相机零成本。
    fn enable_prepass() -> bool {
        true
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

/// 材质资产变更后同步扩展标志位。
pub fn sync_character_material_flags(
    mut events: bevy::ecs::message::MessageReader<
        bevy::asset::AssetEvent<StylizedCharacterMaterial>,
    >,
    mut materials: bevy::prelude::ResMut<bevy::prelude::Assets<StylizedCharacterMaterial>>,
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
    fn default_flags_are_clear() {
        let material = StylizedCharacterMaterial::default();
        assert!(!material.params.has_flag(CHAR_FLAG_ILM_TEXTURE));
        assert!(!material.params.has_flag(CHAR_FLAG_FACE_SDF_TEXTURE));
        assert!(!material.params.has_flag(CHAR_FLAG_PORTABLE_FALLBACK));
    }

    #[test]
    fn portable_fallback_swaps_hair_specular_for_rim() {
        let params = StylizedCharacterParams::default().with_hair_specular(24.0, 0.5);
        assert!(params.has_flag(CHAR_FLAG_HAIR_SPECULAR));
        let fallback = apply_character_portable_fallback(params, 0.25);
        assert!(fallback.has_flag(CHAR_FLAG_PORTABLE_FALLBACK));
        assert!(!fallback.has_flag(CHAR_FLAG_HAIR_SPECULAR));
        assert!(fallback.cel.has_flag(crate::material::CEL_FLAG_RIM));
    }

    #[test]
    fn fallback_applies_only_when_tier_reduced() {
        use bevy_stylized_core::{
            StylizedExecutionTier, StylizedResolvedVariant, StylizedSurfaceClass,
        };
        let params = StylizedCharacterParams::default();
        let not_reduced = StylizedResolvedVariant {
            class: StylizedSurfaceClass::Character,
            requested_tier: StylizedExecutionTier::Portable,
            tier: StylizedExecutionTier::Portable,
            tier_reduced: false,
            missing_features: Vec::new(),
        };
        let reduced = StylizedResolvedVariant {
            tier_reduced: true,
            ..not_reduced.clone()
        };
        assert!(
            !character_params_for_variant(params, &not_reduced)
                .has_flag(CHAR_FLAG_PORTABLE_FALLBACK)
        );
        assert!(
            character_params_for_variant(params, &reduced).has_flag(CHAR_FLAG_PORTABLE_FALLBACK)
        );
    }

    #[test]
    fn from_descriptor_maps_character_textures() {
        let descriptor = StylizedMaterialDescriptor {
            textures: vec![
                bevy_stylized_core::StylizedTextureRef {
                    semantic: StylizedTextureSemantic::Color,
                    uri: "face.png".into(),
                },
                bevy_stylized_core::StylizedTextureRef {
                    semantic: StylizedTextureSemantic::DiscreteClassification,
                    uri: "face_sdf.png".into(),
                },
            ],
            static_params: [
                ("face_forward_z".to_string(), 1.0_f32),
                ("face_threshold_bias".to_string(), 0.55),
            ]
            .into_iter()
            .collect(),
            ..StylizedMaterialDescriptor::default()
        };
        let material =
            StylizedCharacterMaterial::from_descriptor(&descriptor, |_| Handle::default());
        assert!(material.base_color_texture.is_some());
        assert!(material.face_sdf_texture.is_some());
        assert!(material.params.has_flag(CHAR_FLAG_FACE_SDF_TEXTURE));
        assert!((material.params.face_misc.x - 0.55).abs() < f32::EPSILON);
    }
}
