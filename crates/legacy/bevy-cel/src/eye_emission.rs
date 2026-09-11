use bevy::{
    math::{UVec4, Vec4},
    prelude::Reflect,
    render::render_resource::{ShaderType, TextureFormat},
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Reflect)]
/// Diagnostic isolation for task18 masks and HDR emission.
pub enum EyeEmissionDebugMode {
    #[default]
    /// Render normal shading.
    Disabled,
    /// Render the configured eye-class mask.
    EyeClass,
    /// Render the fully resolved emission mask.
    EmissionMask,
    /// Render the unclamped HDR emission term.
    EmissionResult,
    /// Render the value exposed to task19 outline suppression.
    OutlineSuppression,
}

impl EyeEmissionDebugMode {
    const fn shader_value(self) -> u32 {
        match self {
            Self::Disabled => 0,
            Self::EyeClass => 1,
            Self::EmissionMask => 2,
            Self::EmissionResult => 3,
            Self::OutlineSuppression => 4,
        }
    }

    const fn from_shader_value(value: u32) -> Self {
        match value {
            1 => Self::EyeClass,
            2 => Self::EmissionMask,
            3 => Self::EmissionResult,
            4 => Self::OutlineSuppression,
            0 | 5..=u32::MAX => Self::Disabled,
        }
    }
}

/// Validated eye readability, HDR emission, and future outline-suppression controls.
#[derive(Clone, Copy, Debug, PartialEq, Reflect, ShaderType)]
pub struct EyeEmissionSettings {
    eye: Vec4,
    emission: Vec4,
    masks: Vec4,
    state: UVec4,
}

impl Default for EyeEmissionSettings {
    fn default() -> Self {
        Self {
            eye: Vec4::new(0.2, 0.04, 1.0, 1.0),
            emission: Vec4::new(1.0, 1.0, 1.0, 0.0),
            masks: Vec4::new(0.0, 1.0, 0.0, 0.0),
            state: UVec4::ZERO,
        }
    }
}

impl EyeEmissionSettings {
    #[must_use]
    /// Sets ordinary shadow and ambient readability floors for eye-class pixels.
    pub const fn with_eye_floor(mut self, shadow: f32, ambient: f32) -> Self {
        let eye = self.eye.to_array();
        self.eye = Vec4::new(
            finite_unit(shadow, 0.2),
            finite_unit(ambient, 0.04),
            eye[2],
            eye[3],
        );
        self
    }

    #[must_use]
    /// Sets the material eye-class weight and authored-highlight preservation.
    pub const fn with_eye_class(mut self, mask: f32, highlight_preservation: f32) -> Self {
        let eye = self.eye.to_array();
        self.eye = Vec4::new(
            eye[0],
            eye[1],
            finite_unit(mask, 1.0),
            finite_unit(highlight_preservation, 1.0),
        );
        self
    }

    #[must_use]
    /// Sets linear HDR emission tint and unbounded finite intensity.
    pub const fn with_emission(mut self, tint: [f32; 3], intensity: f32) -> Self {
        self.emission = Vec4::new(
            finite_nonnegative(tint[0], 1.0),
            finite_nonnegative(tint[1], 1.0),
            finite_nonnegative(tint[2], 1.0),
            finite_nonnegative(intensity, 0.0),
        );
        self
    }

    #[must_use]
    /// Sets the missing-ILM fallback and task19 outline-suppression weight.
    pub const fn with_masks(mut self, material_fallback: f32, outline_suppression: f32) -> Self {
        self.masks = Vec4::new(
            finite_unit(material_fallback, 0.0),
            finite_unit(outline_suppression, 1.0),
            0.0,
            0.0,
        );
        self
    }

    #[must_use]
    /// Enables or disables eye readability independently.
    pub fn with_eye_enabled(mut self, enabled: bool) -> Self {
        self.state.x = u32::from(enabled);
        self
    }

    #[must_use]
    /// Enables or disables custom HDR emission independently.
    pub fn with_emission_enabled(mut self, enabled: bool) -> Self {
        self.state.y = u32::from(enabled);
        self
    }

    #[must_use]
    /// Selects one task18 diagnostic output.
    pub const fn with_debug(mut self, mode: EyeEmissionDebugMode) -> Self {
        self.state.w = mode.shader_value();
        self
    }

    pub(crate) fn with_texture(mut self, enabled: bool) -> Self {
        self.state.z = u32::from(enabled);
        self
    }

    #[must_use]
    /// Returns shadow and ambient eye floors.
    pub const fn eye_floor(&self) -> (f32, f32) {
        let eye = self.eye.to_array();
        (eye[0], eye[1])
    }

    #[must_use]
    /// Returns eye-class and highlight-preservation weights.
    pub const fn eye_class(&self) -> (f32, f32) {
        let eye = self.eye.to_array();
        (eye[2], eye[3])
    }

    #[must_use]
    /// Returns linear emission tint and HDR intensity.
    pub const fn emission(&self) -> ([f32; 3], f32) {
        let emission = self.emission.to_array();
        ([emission[0], emission[1], emission[2]], emission[3])
    }

    #[must_use]
    /// Returns material fallback and outline-suppression weights.
    pub const fn masks(&self) -> (f32, f32) {
        let masks = self.masks.to_array();
        (masks[0], masks[1])
    }

    #[must_use]
    /// Returns whether eye readability is enabled.
    pub const fn eye_enabled(&self) -> bool {
        self.state.x != 0
    }

    #[must_use]
    /// Returns whether custom emission is enabled.
    pub const fn emission_enabled(&self) -> bool {
        self.state.y != 0
    }

    #[must_use]
    /// Returns whether a validated linear texture is active.
    pub const fn uses_texture(&self) -> bool {
        self.state.z != 0
    }

    #[must_use]
    /// Returns the selected diagnostic.
    pub const fn debug_mode(&self) -> EyeEmissionDebugMode {
        EyeEmissionDebugMode::from_shader_value(self.state.w)
    }

    #[must_use]
    /// Returns whether a texture format stores linear emission-mask data.
    pub const fn supports_texture_format(format: TextureFormat) -> bool {
        matches!(
            format,
            TextureFormat::R8Unorm
                | TextureFormat::Rg8Unorm
                | TextureFormat::Rgba8Unorm
                | TextureFormat::R16Float
                | TextureFormat::Rg16Float
                | TextureFormat::Rgba16Float
                | TextureFormat::R32Float
                | TextureFormat::Rg32Float
                | TextureFormat::Rgba32Float
        )
    }
}

const fn finite_unit(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        fallback
    }
}

const fn finite_nonnegative(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        fallback
    }
}
