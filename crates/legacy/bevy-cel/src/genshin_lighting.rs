use bevy::{
    math::{Vec3, Vec4},
    prelude::Reflect,
    render::render_resource::ShaderType,
};

/// Isolated source-lighting channels for Blender/Bevy parity captures.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Reflect)]
pub enum GenshinLightingDebugMode {
    #[default]
    Disabled,
    SourceLit,
    RampCoordinate,
    RampBand,
    FaceFactor,
    PaletteMultiplier,
    SourceNormal,
}

impl GenshinLightingDebugMode {
    const fn shader_value(self) -> u32 {
        match self {
            Self::Disabled => 0,
            Self::SourceLit => 1,
            Self::RampCoordinate => 2,
            Self::RampBand => 3,
            Self::FaceFactor => 4,
            Self::PaletteMultiplier => 5,
            Self::SourceNormal => 6,
        }
    }

    const fn from_shader_value(value: u32) -> Self {
        match value {
            1 => Self::SourceLit,
            2 => Self::RampCoordinate,
            3 => Self::RampBand,
            4 => Self::FaceFactor,
            5 => Self::PaletteMultiplier,
            6 => Self::SourceNormal,
            _ => Self::Disabled,
        }
    }
}

/// Source-authored Genshin global lighting palette.
///
/// The five body/face colors reproduce the Blender `Genshin Lighting`
/// multiplier. Rim colors are carried alongside them so one preset remains a
/// complete value object even when a material deliberately disables Fresnel
/// and depth-rim branches. The default is neutral and disabled, preserving
/// every existing generic CEL/PBR material.
#[derive(Clone, Copy, Debug, PartialEq, Reflect, ShaderType)]
pub struct GenshinLightingPalette {
    ambient: Vec4,
    sharp_lit: Vec4,
    soft_lit: Vec4,
    sharp_shadow: Vec4,
    soft_shadow: Vec4,
    rim_lit: Vec4,
    rim_shadow: Vec4,
}

impl Default for GenshinLightingPalette {
    fn default() -> Self {
        Self {
            ambient: Vec4::new(1.0, 1.0, 1.0, 0.0),
            sharp_lit: Vec3::ONE.extend(0.0),
            soft_lit: Vec3::ONE.extend(0.0),
            sharp_shadow: Vec3::ONE.extend(0.0),
            soft_shadow: Vec3::ONE.extend(0.0),
            rim_lit: Vec3::ONE.extend(0.0),
            rim_shadow: Vec3::ONE.extend(0.0),
        }
    }
}

impl GenshinLightingPalette {
    /// Constructs an enabled, finite palette from Blender scene-linear RGB.
    #[must_use]
    pub fn enabled(
        ambient: [f32; 3],
        sharp_lit: [f32; 3],
        soft_lit: [f32; 3],
        sharp_shadow: [f32; 3],
        soft_shadow: [f32; 3],
        rim_lit: [f32; 3],
        rim_shadow: [f32; 3],
    ) -> Self {
        Self {
            ambient: color(ambient).extend(1.0),
            sharp_lit: color(sharp_lit).extend(0.0),
            soft_lit: color(soft_lit).extend(0.0),
            sharp_shadow: color(sharp_shadow).extend(0.0),
            soft_shadow: color(soft_shadow).extend(0.0),
            rim_lit: color(rim_lit).extend(0.0),
            rim_shadow: color(rim_shadow).extend(0.0),
        }
    }

    /// Enables or disables palette multiplication without discarding values.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.ambient.w = enabled as u32 as f32;
        self
    }

    /// Selects the exact Blender material ordering for reference captures.
    ///
    /// Production leaves this disabled so its explicit extreme-light
    /// readability contract remains intact.
    #[must_use]
    pub fn with_strict_reference(mut self, enabled: bool) -> Self {
        self.rim_lit.w = enabled as u32 as f32;
        self
    }

    /// Whether Blender-exact material ordering is requested.
    #[must_use]
    pub fn is_strict_reference(&self) -> bool {
        self.rim_lit.w > 0.5
    }

    /// Returns whether the source palette multiplier is active.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.ambient.w > 0.5
    }

    /// Returns Ambient Colour in Blender scene-linear RGB.
    #[must_use]
    pub fn ambient(&self) -> [f32; 3] {
        self.ambient.truncate().to_array()
    }

    /// Returns Sharp Lit Colour in Blender scene-linear RGB.
    #[must_use]
    pub fn sharp_lit(&self) -> [f32; 3] {
        self.sharp_lit.truncate().to_array()
    }

    /// Returns Soft Lit Colour in Blender scene-linear RGB.
    #[must_use]
    pub fn soft_lit(&self) -> [f32; 3] {
        self.soft_lit.truncate().to_array()
    }

    /// Returns Sharp Shadow Colour in Blender scene-linear RGB.
    #[must_use]
    pub fn sharp_shadow(&self) -> [f32; 3] {
        self.sharp_shadow.truncate().to_array()
    }

    /// Returns Soft Shadow Colour in Blender scene-linear RGB.
    #[must_use]
    pub fn soft_shadow(&self) -> [f32; 3] {
        self.soft_shadow.truncate().to_array()
    }

    /// Returns Rim Lit in Blender scene-linear RGB.
    #[must_use]
    pub fn rim_lit(&self) -> [f32; 3] {
        self.rim_lit.truncate().to_array()
    }

    /// Returns Rim Shadow in Blender scene-linear RGB.
    #[must_use]
    pub fn rim_shadow(&self) -> [f32; 3] {
        self.rim_shadow.truncate().to_array()
    }

    /// Selects one source-lighting diagnostic without altering palette RGB.
    #[must_use]
    pub fn with_debug(mut self, mode: GenshinLightingDebugMode) -> Self {
        self.rim_shadow.w = mode.shader_value() as f32;
        self
    }

    /// Returns the selected source-lighting diagnostic.
    #[must_use]
    pub fn debug_mode(&self) -> GenshinLightingDebugMode {
        GenshinLightingDebugMode::from_shader_value(self.rim_shadow.w.round() as u32)
    }
}

fn color(value: [f32; 3]) -> Vec3 {
    Vec3::from_array(value.map(finite_unit))
}

fn finite_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        1.0
    }
}
