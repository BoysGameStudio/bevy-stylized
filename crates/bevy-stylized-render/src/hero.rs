//! Hero@Portable 材质：Character + authored ramp / packed normal /
//! metal matcap / spec ramp（docs/rendering/01 Hero class）。
//!
//! 薄层 backend（ADR-0001 Accepted）：与 cel/character 同属薄层临时 backend。
//!
//! ## 预算（超出 Cel 的 interim 子预算，实际计数见 interim_budget 测试输出）
//!
//! 8 纹理 / 8 sampler / 1 UBO（base + ILM + face SDF + authored ramp +
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
/// Genshin-profile ramp 寻址（docs/reference/07 §3.1 已核实公式）启用位；
/// 未设置时走通用 MeshTag 行寻址。
pub const HERO_FLAG_GENSHIN_RAMP: u32 = 1 << 5;
/// Genshin Face Factor control-map interpretation.
pub const HERO_FLAG_GENSHIN_FACE: u32 = 1 << 6;
/// Authored transparent/emissive pupil highlight composition.
pub const HERO_FLAG_PUPIL_HIGHLIGHT: u32 = 1 << 7;
/// Reconstruct a raw authored diffuse image using cubic B-splines.
pub const HERO_FLAG_CUBIC_BASE: u32 = 1 << 8;
/// Select UV1 for backfacing diffuse without changing face/control-map UVs.
pub const HERO_FLAG_BACKFACE_UV1: u32 = 1 << 9;

/// Genshin lighting fields, packed as vec4s without extra texture bindings.
#[derive(Clone, Copy, Debug, PartialEq, Reflect, ShaderType)]
pub struct StylizedGenshinLightingParams {
    /// x: lightmap AO; y: ramp enabled; z: body interpretation; w: vertex AO or face tints.
    pub controls: bevy::math::Vec4,
    /// Fallback lightmap when the authored shader has no image.
    pub lightmap_constant: bevy::math::Vec4,
    /// Non-metallic specular color.
    pub specular_color: bevy::math::Vec4,
    /// Body: shininess, multiplier, enabled, reserved per band; face: day tint rows.
    pub specular_rows: [bevy::math::Vec4; 5],
    /// Body: transition range, softness, reserved, reserved; face: night tint rows.
    pub shadow_rows: [bevy::math::Vec4; 5],
    /// Highlight second-ramp endpoint, opacity endpoint, reserved, reserved.
    pub highlight_curve: bevy::math::Vec4,
    /// Highlight first tint RGB and first-ramp endpoint.
    pub highlight_primary: bevy::math::Vec4,
    /// Highlight second tint RGB and second-ramp start.
    pub highlight_secondary: bevy::math::Vec4,
}
impl Default for StylizedGenshinLightingParams {
    fn default() -> Self {
        use bevy::math::Vec4;
        Self {
            controls: Vec4::ZERO,
            lightmap_constant: Vec4::ONE,
            specular_color: Vec4::ONE,
            specular_rows: [Vec4::new(10., 0.1, 1., 0.); 5],
            shadow_rows: [Vec4::new(0.01, 0.5, 0., 0.); 5],
            highlight_curve: Vec4::new(0.7, 0.2, 0., 0.),
            highlight_primary: Vec4::new(1., 1., 1., 0.2),
            highlight_secondary: Vec4::new(1., 1., 1., 0.25),
        }
    }
}

/// Opt-in authored matcap/specular branch; colors precede their gamma nodes.
#[derive(Clone, Copy, Debug, PartialEq, Reflect, ShaderType)]
pub struct StylizedMetalParams {
    /// Enabled, matcap brightness, horizontal tile scale, source specular-ramp switch.
    pub controls: bevy::math::Vec4,
    pub dark: bevy::math::Vec4,
    pub light: bevy::math::Vec4,
    pub shadow: bevy::math::Vec4,
    /// Shininess, response scale, attenuation, sharp-layer threshold.
    pub specular: bevy::math::Vec4,
    pub specular_color: bevy::math::Vec4,
    pub sharp_color: bevy::math::Vec4,
}
impl Default for StylizedMetalParams {
    fn default() -> Self {
        use bevy::math::Vec4;
        Self {
            controls: Vec4::new(0., 3., 1., 0.),
            dark: Vec4::ONE,
            light: Vec4::ONE,
            shadow: Vec4::ONE,
            specular: Vec4::new(4., 1.2, 0.2, 0.95),
            specular_color: Vec4::ONE,
            sharp_color: Vec4::ONE,
        }
    }
}

/// Hero 材质 GPU uniform：character 参数 + hero 扩展 2×vec4（packed 单
/// UBO，512B）。
#[derive(Clone, Copy, Debug, PartialEq, Reflect, ShaderType)]
pub struct StylizedHeroParams {
    /// Character 层参数（face/hair/emission/cel 基础 + palette + face up）。
    pub character: StylizedCharacterParams,
    /// x = matcap 强度，y = spec ramp 强度，z = shadow position（已核实
    /// ramp 公式的 ShadowPosition），w = ramp width（ShadowRampWidth）。
    pub hero_a: bevy::math::Vec4,
    /// rgb = matcap 染色（线性），w = 位标志（bitcast u32，见 `HERO_FLAG_*`）。
    pub hero_b: bevy::math::Vec4,
    /// Explicit Genshin body lighting; disabled by default.
    pub genshin: StylizedGenshinLightingParams,
    /// Opt-in base readability floor, authored-alpha gain, eye floor and highlight preservation.
    /// Zero preserves the existing profile behavior.
    pub authored_energy: bevy::math::Vec4,
    pub metal: StylizedMetalParams,
    /// Authored normal-map strength (0 selects generic reconstruction), sequential prepared ramp, level-zero controls, incoming-light body convention.
    pub source_conventions: bevy::math::Vec4,
    /// Source Alpha emission: gamma-decoded tint times strength in RGB, enable in W.
    pub source_emission: bevy::math::Vec4,
}

impl Default for StylizedHeroParams {
    fn default() -> Self {
        Self {
            character: StylizedCharacterParams::default(),
            hero_a: bevy::math::Vec4::new(0.0, 0.0, 0.5, 1.0),
            hero_b: bevy::math::Vec4::new(1.0, 1.0, 1.0, 0.0),
            genshin: StylizedGenshinLightingParams::default(),
            authored_energy: bevy::math::Vec4::ZERO,
            metal: StylizedMetalParams::default(),
            source_conventions: bevy::math::Vec4::ZERO,
            source_emission: bevy::math::Vec4::ZERO,
        }
    }
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

    /// 通用 ramp 路径的 atlas 行数（MeshTag 行选择除数；存于 cel 参数
    /// 的 ramp 行字段。Genshin 寻址启用时不使用该路径）。
    #[must_use]
    pub fn with_ramp_rows(mut self, rows: f32) -> Self {
        self.character.cel = self.character.cel.with_ramp_rows(rows);
        self
    }

    /// 启用 Genshin-profile ramp 寻址（已核实公式，docs/reference/07 §3.1）：
    /// `shadow_position` = ShadowPosition，`ramp_width` = ShadowRampWidth。
    #[must_use]
    pub fn with_genshin_ramp(mut self, shadow_position: f32, ramp_width: f32) -> Self {
        self.hero_a.z = shadow_position.clamp(0.01, 1.0);
        self.hero_a.w = ramp_width.max(0.01);
        self.set_flag(HERO_FLAG_GENSHIN_RAMP, true);
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

/// Optional renderer diagnostics; beauty has no additional shader work or bindings.
#[derive(Default, Eq, PartialEq, Hash, Copy, Clone, Debug, Reflect)]
pub enum StylizedHeroDebugView {
    /// Normal stylized rendering.
    #[default]
    Beauty,
    /// RGB = soft lighting factor, sharp lighting factor, geometry coverage.
    LightingFactors,
    /// Base-color texture/factor before stylized lighting.
    BaseColor,
    /// Diffuse/ramp/specular composition before the lighting palette.
    PrePalette,
    /// Normal/half-vector dot product used by specular lighting.
    NormalHalfDot,
}

/// Interpretation of the mutually exclusive surface-control texture.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum StylizedHeroSurfaceControl {
    /// No auxiliary surface control.
    #[default]
    None,
    /// Linear G/A face-shadow controls.
    GenshinFace,
    /// sRGB-encoded red-channel highlight controls.
    GenshinHighlight,
}

/// Hero 材质的 pipeline 特化键。
#[repr(C)]
#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub struct StylizedHeroMaterialKey {
    /// The pupil remains a depth-owning surface while its highlight closure blends.
    pub highlight_depth_write: bool,
    /// Compile-time diagnostic output selection.
    pub debug_view: StylizedHeroDebugView,
    /// 双面渲染。
    pub double_sided: bool,
}

impl From<&StylizedHeroMaterial> for StylizedHeroMaterialKey {
    fn from(material: &StylizedHeroMaterial) -> Self {
        Self {
            double_sided: material.double_sided,
            highlight_depth_write: material.params.has_flag(HERO_FLAG_PUPIL_HIGHLIGHT),
            debug_view: material.debug_view,
        }
    }
}

/// Hero@Portable 材质（共享材质；face/profile 状态逐实例经 MeshTag）。
#[derive(Asset, AsBindGroup, Clone, Debug, Reflect)]
#[bind_group_data(StylizedHeroMaterialKey)]
pub struct StylizedHeroMaterial {
    /// Explicit interpretation of the auxiliary control texture.
    pub surface_control: StylizedHeroSurfaceControl,
    /// Optional diagnostic output; defaults to beauty. Not serialized in asset descriptors.
    pub debug_view: StylizedHeroDebugView,
    /// 材质 uniform（见 [`StylizedHeroParams`]）。
    #[uniform(0)]
    pub params: StylizedHeroParams,
    /// 基色纹理。
    #[texture(1)]
    #[sampler(2)]
    pub base_color_texture: Option<Handle<Image>>,
    /// ILM/control：R=metal，G=AO，B=specular suppression，A=material band。
    /// 显式 Genshin 模式将 G 直接输入 AO 门控；通用模式保留其原有 AO 映射。
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
    /// Genshin face control map (G: coverage factor, A: fixed-shadow bands), linear.
    #[texture(15)]
    #[sampler(16)]
    pub surface_control_texture: Option<Handle<Image>>,
    /// 双面渲染（false = 继承默认背面剔除）。
    pub double_sided: bool,
}

impl Default for StylizedHeroMaterial {
    fn default() -> Self {
        let mut material = Self {
            params: StylizedHeroParams::default(),
            debug_view: StylizedHeroDebugView::Beauty,
            base_color_texture: None,
            ilm_texture: None,
            face_sdf_texture: None,
            ramp_texture: None,
            packed_normal_texture: None,
            matcap_texture: None,
            spec_ramp_texture: None,
            surface_control_texture: None,
            surface_control: StylizedHeroSurfaceControl::None,
            double_sided: false,
        };
        material.sync_flags();
        material
    }
}

impl StylizedHeroMaterial {
    /// Construct a Hero material from engine-neutral descriptors.
    /// Texture loading remains the application's responsibility.
    #[must_use]
    pub fn from_descriptor(
        descriptor: &bevy_stylized_core::StylizedMaterialDescriptor,
        mut load_texture: impl FnMut(&str) -> bevy::asset::Handle<bevy::image::Image>,
    ) -> Self {
        let character = StylizedCharacterMaterial::from_descriptor(descriptor, &mut load_texture);
        let mut material = Self {
            base_color_texture: character.base_color_texture,
            ilm_texture: character.ilm_texture,
            face_sdf_texture: character.face_sdf_texture,
            double_sided: descriptor.static_flags.double_sided,
            ..Default::default()
        };
        material.params.character = character.params;
        material.params.set_flag(
            HERO_FLAG_BACKFACE_UV1,
            descriptor
                .static_params
                .get("backface_uv1")
                .is_some_and(|v| *v > 0.5),
        );
        material.params.set_flag(
            HERO_FLAG_CUBIC_BASE,
            descriptor
                .static_params
                .get("cubic_base_color")
                .is_some_and(|v| *v > 0.5),
        );
        for (index, texture) in descriptor.textures.iter().enumerate() {
            if descriptor
                .static_params
                .get("specular_ramp_index")
                .is_some_and(|v| *v == index as f32)
            {
                material.spec_ramp_texture = Some(load_texture(&texture.uri));
                continue;
            }
            if descriptor
                .static_params
                .get("surface_control_index")
                .is_some_and(|v| *v == index as f32)
            {
                material.surface_control_texture = Some(load_texture(&texture.uri));
                material.surface_control = if descriptor
                    .static_params
                    .get("pupil_highlight")
                    .copied()
                    .unwrap_or(0.)
                    > 0.5
                {
                    StylizedHeroSurfaceControl::GenshinHighlight
                } else {
                    StylizedHeroSurfaceControl::GenshinFace
                };
                material.params.hero_a.z = descriptor
                    .static_params
                    .get("shadow_position")
                    .copied()
                    .unwrap_or(0.55);
                continue;
            }

            match texture.semantic {
                bevy_stylized_core::StylizedTextureSemantic::ContinuousMask
                    if descriptor
                        .static_params
                        .get("genshin_face")
                        .copied()
                        .unwrap_or(0.0)
                        > 0.0 =>
                {
                    material.surface_control_texture = Some(load_texture(&texture.uri));
                    material.surface_control = StylizedHeroSurfaceControl::GenshinFace;
                    material.params.hero_a.z = descriptor
                        .static_params
                        .get("shadow_position")
                        .copied()
                        .unwrap_or(0.55);
                }
                bevy_stylized_core::StylizedTextureSemantic::Ramp => {
                    material.ramp_texture = Some(load_texture(&texture.uri))
                }
                bevy_stylized_core::StylizedTextureSemantic::Normal => {
                    material.packed_normal_texture = Some(load_texture(&texture.uri))
                }
                bevy_stylized_core::StylizedTextureSemantic::MatCap => {
                    material.matcap_texture = Some(load_texture(&texture.uri))
                }
                _ => {}
            }
        }
        let get = |key: &str, default: f32| {
            descriptor
                .static_params
                .get(key)
                .copied()
                .unwrap_or(default)
        };
        let metal_vector = |key: &str, fallback: bevy::math::Vec4| {
            bevy::math::Vec4::from_array(std::array::from_fn(|i| {
                get(&format!("{key}_{i}"), fallback[i])
            }))
        };
        material.params.source_conventions =
            metal_vector("source_conventions", material.params.source_conventions);
        material.params.source_emission =
            metal_vector("source_emission", material.params.source_emission);
        let m = &mut material.params.metal;
        m.controls = metal_vector("metal_controls", m.controls);
        m.dark = metal_vector("metal_dark", m.dark);
        m.light = metal_vector("metal_light", m.light);
        m.shadow = metal_vector("metal_shadow", m.shadow);
        m.specular = metal_vector("metal_specular", m.specular);
        m.specular_color = metal_vector("metal_specular_color", m.specular_color);
        m.sharp_color = metal_vector("metal_sharp_color", m.sharp_color);
        material.params.authored_energy = bevy::math::Vec4::new(
            get("readability_floor", 0.0).clamp(0.0, 1.0),
            get("authored_emission_gain", 0.0).max(0.0),
            get("eye_floor", 0.0).clamp(0.0, 1.0),
            get("eye_highlight_preservation", 0.0).max(0.0),
        );
        if get("genshin_ramp", 0.0) != 0.0 {
            material.params = material
                .params
                .with_genshin_ramp(get("shadow_position", 0.5), get("ramp_width", 1.0));
        }
        material.params.character = material
            .params
            .character
            .with_day_night(get("day_night", 1.0));
        for (key, color) in [
            (
                "palette_soft_lit",
                &mut material.params.character.palette_soft_lit,
            ),
            (
                "palette_soft_shadow",
                &mut material.params.character.palette_soft_shadow,
            ),
            (
                "palette_sharp_lit",
                &mut material.params.character.palette_sharp_lit,
            ),
            (
                "palette_sharp_shadow",
                &mut material.params.character.palette_sharp_shadow,
            ),
        ] {
            color.x = get(&format!("{key}_r"), color.x);
            color.y = get(&format!("{key}_g"), color.y);
            color.z = get(&format!("{key}_b"), color.z);
        }
        if get("genshin_lighting", 0.) > 0.5 {
            use bevy::math::Vec4;
            let vector = |key: &str, fallback: Vec4| {
                Vec4::from_array(std::array::from_fn(|i| {
                    get(&format!("{key}_{i}"), fallback[i])
                }))
            };
            material.params.hero_a.z = get("shadow_position", 0.55);
            let g = &mut material.params.genshin;
            g.controls = Vec4::new(
                get("use_lightmap_ao", 1.),
                get("use_shadow_ramp", 1.),
                1.,
                get("use_vertex_ao", 0.),
            );
            g.lightmap_constant = vector("lightmap_constant", g.lightmap_constant);
            g.specular_color = vector("specular_color", g.specular_color);
            g.highlight_curve = vector("highlight_curve", g.highlight_curve);
            g.highlight_primary = vector("highlight_primary", g.highlight_primary);
            g.highlight_secondary = vector("highlight_secondary", g.highlight_secondary);
            for i in 0..5 {
                g.specular_rows[i] = vector(&format!("specular_rows_{i}"), g.specular_rows[i]);
                g.shadow_rows[i] = vector(&format!("shadow_rows_{i}"), g.shadow_rows[i]);
            }
        }
        if get("face_tints", 0.) > 0.5 {
            material.params.genshin.controls.w = 1.0;
            for r in 0..3 {
                for c in 0..4 {
                    material.params.genshin.specular_rows[r][c] =
                        get(&format!("face_day_{r}_{c}"), 1.0);
                    material.params.genshin.shadow_rows[r][c] =
                        get(&format!("face_night_{r}_{c}"), 1.0);
                }
            }
        }
        for (key, field) in [
            (
                "highlight_primary",
                &mut material.params.genshin.highlight_primary,
            ),
            (
                "highlight_secondary",
                &mut material.params.genshin.highlight_secondary,
            ),
            (
                "highlight_curve",
                &mut material.params.genshin.highlight_curve,
            ),
        ] {
            for i in 0..4 {
                field[i] = get(&format!("{key}_{i}"), field[i]);
            }
        }
        material.sync_flags();
        material
    }

    /// 同步纹理存在性标志位（资产事件驱动）。
    pub fn sync_flags(&mut self) {
        self.params.set_flag(
            HERO_FLAG_PUPIL_HIGHLIGHT,
            self.surface_control_texture.is_some()
                && self.surface_control == StylizedHeroSurfaceControl::GenshinHighlight,
        );
        self.params.set_flag(
            HERO_FLAG_GENSHIN_FACE,
            self.surface_control_texture.is_some()
                && self.surface_control == StylizedHeroSurfaceControl::GenshinFace,
        );
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
        self.params.has_flag(HERO_FLAG_PUPIL_HIGHLIGHT)
            == (self.surface_control_texture.is_some()
                && self.surface_control == StylizedHeroSurfaceControl::GenshinHighlight)
            && self.params.has_flag(HERO_FLAG_GENSHIN_FACE)
                == (self.surface_control_texture.is_some()
                    && self.surface_control == StylizedHeroSurfaceControl::GenshinFace)
            && self.params.has_flag(HERO_FLAG_RAMP_TEXTURE) == self.ramp_texture.is_some()
            && self
                .params
                .character
                .has_flag(crate::character::CHAR_FLAG_ILM_TEXTURE)
                == self.ilm_texture.is_some()
            && self
                .params
                .character
                .has_flag(crate::character::CHAR_FLAG_FACE_SDF_TEXTURE)
                == self.face_sdf_texture.is_some()
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
        if self.params.has_flag(HERO_FLAG_PUPIL_HIGHLIGHT) {
            return bevy::material::AlphaMode::Blend;
        }
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
        // Casting is opt-in for hosts whose scenes keep shadow-mapped ground
        // contact readable (`shadow-casting` feature). Receiving is unchanged:
        // the shader never samples shadow maps, so the authored ramp shading
        // is unaffected either way.
        cfg!(feature = "shadow-casting")
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        crate::skinning::specialize_secondary_skin(descriptor, layout)?;
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader_defs.push("NPR_PREPASS_HERO".into());
        }
        let debug_define = match key.bind_group_data.debug_view {
            StylizedHeroDebugView::Beauty => None,
            StylizedHeroDebugView::LightingFactors => Some("STYLIZED_LIGHTING_FACTORS"),
            StylizedHeroDebugView::BaseColor => Some("STYLIZED_BASE_COLOR"),
            StylizedHeroDebugView::PrePalette => Some("STYLIZED_PRE_PALETTE"),
            StylizedHeroDebugView::NormalHalfDot => Some("STYLIZED_NDOTH"),
        };
        if let Some(name) = debug_define
            && let Some(fragment) = descriptor.fragment.as_mut()
        {
            fragment.shader_defs.push(name.into());
        }
        if key.bind_group_data.highlight_depth_write
            && let Some(depth) = descriptor.depth_stencil.as_mut()
        {
            depth.depth_write_enabled = Some(true);
        }
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

#[cfg(test)]
mod texture_flag_regression {
    use super::*;
    #[test]
    fn inherited_texture_changes_invalidate_hero_flags() {
        let mut material = StylizedHeroMaterial::default();
        assert!(material.flags_consistent());
        material.ilm_texture = Some(Handle::default());
        assert!(!material.flags_consistent());
        material.sync_flags();
        assert!(material.flags_consistent());
        material.face_sdf_texture = Some(Handle::default());
        assert!(!material.flags_consistent());
        material.sync_flags();
        assert!(material.flags_consistent());
        material.ilm_texture = None;
        material.face_sdf_texture = None;
        assert!(!material.flags_consistent());
        material.sync_flags();
        assert!(material.flags_consistent());
    }
}
