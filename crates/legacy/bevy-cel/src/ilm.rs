use bevy::{
    math::{UVec4, Vec4},
    prelude::Reflect,
    render::render_resource::ShaderType,
};

const MAX_ILM_DIMENSION: u32 = 16_384;

/// Diagnostic outputs for this framework's linear ILM channel contract.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Reflect)]
pub enum IlmDebugMode {
    /// Render normal toon shading.
    #[default]
    Disabled,
    /// Render ILM R as grayscale.
    SpecularStrength,
    /// Render ILM G as grayscale.
    AoShadowInfluence,
    /// Render the resolved material ID normalized by ramp-row count.
    MaterialId,
    /// Render ILM A as grayscale.
    EmissionSpecialMask,
    /// Render the center coordinate of the selected ramp row.
    RampRow,
}

impl IlmDebugMode {
    const fn shader_value(self) -> u32 {
        match self {
            Self::Disabled => 0,
            Self::SpecularStrength => 1,
            Self::AoShadowInfluence => 2,
            Self::MaterialId => 3,
            Self::EmissionSpecialMask => 4,
            Self::RampRow => 5,
        }
    }

    const fn from_shader_value(value: u32) -> Self {
        match value {
            1 => Self::SpecularStrength,
            2 => Self::AoShadowInfluence,
            3 => Self::MaterialId,
            4 => Self::EmissionSpecialMask,
            5 => Self::RampRow,
            0 | 6..=u32::MAX => Self::Disabled,
        }
    }
}

/// Selects precedence between ILM B and the canonical vertex material ID.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Reflect)]
pub enum IlmMaterialIdSource {
    /// Use ILM B when a valid texture is active, otherwise use `_CEL_MATERIAL_ID`.
    #[default]
    TextureThenVertex,
    /// Use `_CEL_MATERIAL_ID` when present, otherwise use ILM B.
    VertexThenTexture,
}

/// Declares how a packed material lightmap maps its channels into the
/// framework's canonical ILM values.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Reflect)]
pub enum IlmLayout {
    /// R = specular, G = AO, B = an 8-bit ramp row, A = special/emission.
    #[default]
    Canonical,
    /// HoYoverse/Gacha character packing: R = specular, G = AO and A stores
    /// one of five material classes. The shader expands those classes onto
    /// the even rows of a 20-row daytime/nighttime shadow-ramp atlas.
    GenshinCharacter,
}

impl IlmLayout {
    const fn shader_value(self) -> u32 {
        match self {
            Self::Canonical => 0,
            Self::GenshinCharacter => 1,
        }
    }

    const fn from_shader_value(value: u32) -> Self {
        match value {
            1 => Self::GenshinCharacter,
            0 | 2..=u32::MAX => Self::Canonical,
        }
    }
}

impl IlmMaterialIdSource {
    const fn shader_value(self) -> u32 {
        match self {
            Self::TextureThenVertex => 0,
            Self::VertexThenTexture => 1,
        }
    }

    const fn from_shader_value(value: u32) -> Self {
        match value {
            1 => Self::VertexThenTexture,
            0 | 2..=u32::MAX => Self::TextureThenVertex,
        }
    }
}

/// Validated shader controls for this framework's linear ILM convention.
///
/// R is specular strength, G is AO/shadow influence, B is an 8-bit
/// material/ramp-row ID, and A is an emission/special mask. R and A are decoded
/// but intentionally do not affect final lighting until later tasks.
#[derive(Clone, Copy, Debug, PartialEq, Reflect, ShaderType)]
pub struct IlmSettings {
    fallback_channels: Vec4,
    state: UVec4,
    dimensions: UVec4,
}

impl Default for IlmSettings {
    fn default() -> Self {
        Self {
            fallback_channels: Vec4::new(0.0, 1.0, 0.0, 0.0),
            state: UVec4::ZERO,
            dimensions: UVec4::ZERO,
        }
    }
}

impl IlmSettings {
    /// Sets neutral channels used when no valid linear ILM texture is active.
    #[must_use]
    pub fn with_fallback_channels(mut self, channels: [f32; 4]) -> Self {
        self.fallback_channels = Vec4::from_array([
            finite_unit(channels[0], 0.0),
            finite_unit(channels[1], 1.0),
            finite_unit(channels[2], 0.0),
            finite_unit(channels[3], 0.0),
        ]);
        self
    }

    /// Records validated dimensions used to enable texture sampling.
    #[must_use]
    pub const fn with_texture_dimensions(mut self, width: u32, height: u32) -> Self {
        if width == 0 || height == 0 || width > MAX_ILM_DIMENSION || height > MAX_ILM_DIMENSION {
            self.dimensions = UVec4::ZERO;
            self.state.x = 0;
        } else {
            self.dimensions = UVec4::new(width, height, 0, 0);
        }
        self
    }

    /// Selects texture/vertex material-ID precedence.
    #[must_use]
    pub const fn with_material_id_source(mut self, source: IlmMaterialIdSource) -> Self {
        self.state.y = source.shader_value();
        self
    }

    /// Selects the packed texture channel layout without changing the
    /// canonical default used by existing materials.
    #[must_use]
    pub const fn with_layout(mut self, layout: IlmLayout) -> Self {
        self.state.w = layout.shader_value();
        self
    }

    /// Selects an ILM diagnostic output.
    #[must_use]
    pub const fn with_debug(mut self, mode: IlmDebugMode) -> Self {
        self.state.z = mode.shader_value();
        self
    }

    pub(crate) fn with_texture(mut self, enabled: bool) -> Self {
        self.state.x = u32::from(enabled && self.texture_dimensions().is_some());
        self
    }

    /// Returns the RGBA channels used when ILM sampling is unavailable.
    #[must_use]
    pub const fn fallback_channels(&self) -> [f32; 4] {
        self.fallback_channels.to_array()
    }

    /// Returns valid texture dimensions, or `None` when sampling is disabled.
    #[must_use]
    pub const fn texture_dimensions(&self) -> Option<(u32, u32)> {
        if self.dimensions.x == 0 || self.dimensions.y == 0 {
            None
        } else {
            Some((self.dimensions.x, self.dimensions.y))
        }
    }

    /// Returns texture/vertex material-ID precedence.
    #[must_use]
    pub const fn material_id_source(&self) -> IlmMaterialIdSource {
        IlmMaterialIdSource::from_shader_value(self.state.y)
    }

    /// Returns the active packed texture layout.
    #[must_use]
    pub const fn layout(&self) -> IlmLayout {
        IlmLayout::from_shader_value(self.state.w)
    }

    /// Returns the active diagnostic output.
    #[must_use]
    pub const fn debug_mode(&self) -> IlmDebugMode {
        IlmDebugMode::from_shader_value(self.state.z)
    }

    /// Returns whether valid linear ILM sampling is active.
    #[must_use]
    pub const fn uses_texture(&self) -> bool {
        self.state.x != 0
    }

    /// Resolves an authored scalar ID to a valid zero-based ramp row.
    #[must_use]
    pub const fn validated_ramp_row(&self, material_id: u32, row_count: u32) -> u32 {
        let safe_count = if row_count == 0 {
            1
        } else if row_count > 4_096 {
            4_096
        } else {
            row_count
        };
        if material_id >= safe_count {
            safe_count - 1
        } else {
            material_id
        }
    }
}

fn finite_unit(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        value
    } else {
        fallback
    }
}
