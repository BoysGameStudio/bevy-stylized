use bevy::{
    math::{UVec4, Vec3, Vec4},
    prelude::Reflect,
    render::render_resource::ShaderType,
};

use crate::{HairHighlightSettings, RimLightingSettings, ToonSpecularSettings};

/// Diagnostic isolation for masks and final additive effect terms.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Reflect)]
pub enum CelLightingDebugMode {
    #[default]
    /// Render normal lighting.
    Disabled,
    /// Render the resolved vertex/material/ILM hair mask.
    HairMask,
    /// Render the thresholded hair contribution.
    HairResult,
    /// Render ILM R.
    SpecularMask,
    /// Render the thresholded ILM-masked specular contribution.
    SpecularResult,
    /// Render the resolved material/ILM rim mask.
    RimMask,
    /// Render the thresholded rim contribution.
    RimResult,
}

impl CelLightingDebugMode {
    const fn shader_value(self) -> u32 {
        match self {
            Self::Disabled => 0,
            Self::HairMask => 1,
            Self::HairResult => 2,
            Self::SpecularMask => 3,
            Self::SpecularResult => 4,
            Self::RimMask => 5,
            Self::RimResult => 6,
        }
    }

    const fn from_shader_value(value: u32) -> Self {
        match value {
            1 => Self::HairMask,
            2 => Self::HairResult,
            3 => Self::SpecularMask,
            4 => Self::SpecularResult,
            5 => Self::RimMask,
            6 => Self::RimResult,
            0 | 7..=u32::MAX => Self::Disabled,
        }
    }
}

/// Unified shader settings for task-17 additive lighting effects.
#[derive(Clone, Copy, Debug, PartialEq, Reflect, ShaderType)]
pub struct CelLightingSettings {
    hair: HairHighlightSettings,
    specular: ToonSpecularSettings,
    rim: RimLightingSettings,
    material_masks: Vec4,
    state: UVec4,
}

impl Default for CelLightingSettings {
    fn default() -> Self {
        Self {
            hair: HairHighlightSettings::default(),
            specular: ToonSpecularSettings::default(),
            rim: RimLightingSettings::default(),
            material_masks: Vec4::new(1.0, 0.0, 1.0, 0.0),
            state: UVec4::ZERO,
        }
    }
}

impl CelLightingSettings {
    /// Replaces Kajiya-Kay hair controls.
    #[must_use]
    pub const fn with_hair(mut self, settings: HairHighlightSettings) -> Self {
        self.hair = settings;
        self
    }
    /// Replaces toon-specular controls.
    #[must_use]
    pub const fn with_specular(mut self, settings: ToonSpecularSettings) -> Self {
        self.specular = settings;
        self
    }
    /// Replaces Fresnel-rim controls.
    #[must_use]
    pub const fn with_rim(mut self, settings: RimLightingSettings) -> Self {
        self.rim = settings;
        self
    }
    /// Sets per-material hair/rim weights and fallback for missing ILM A.
    #[must_use]
    pub fn with_material_masks(mut self, hair: f32, rim: f32, special_fallback: f32) -> Self {
        let readability_floor = self.material_masks.w;
        self.material_masks = Vec4::new(
            finite_unit(hair, 1.0),
            finite_unit(rim, 0.0),
            finite_unit(special_fallback, 1.0),
            readability_floor,
        );
        self
    }

    /// Sets a host-authored minimum fraction of base color retained when all
    /// physical lights are dark. A positive floor also enables hue-preserving
    /// peak compression above display white. The default remains zero so
    /// ordinary PBR and existing CEL materials are unchanged.
    #[must_use]
    pub fn with_readability_floor(mut self, floor: f32) -> Self {
        self.material_masks.w = finite_unit(floor, 0.0);
        self
    }
    /// Enables or disables only the hair contribution.
    #[must_use]
    pub fn with_hair_enabled(mut self, enabled: bool) -> Self {
        self.state.x = u32::from(enabled);
        self
    }
    /// Enables or disables only the toon-specular contribution.
    #[must_use]
    pub fn with_specular_enabled(mut self, enabled: bool) -> Self {
        self.state.y = u32::from(enabled);
        self
    }
    /// Enables or disables only the rim contribution.
    #[must_use]
    pub fn with_rim_enabled(mut self, enabled: bool) -> Self {
        self.state.z = u32::from(enabled);
        self
    }
    /// Selects one isolated mask or result diagnostic.
    #[must_use]
    pub const fn with_debug(mut self, mode: CelLightingDebugMode) -> Self {
        self.state.w = mode.shader_value();
        self
    }
    /// Returns Kajiya-Kay hair controls.
    #[must_use]
    pub const fn hair(&self) -> &HairHighlightSettings {
        &self.hair
    }
    /// Returns toon-specular controls.
    #[must_use]
    pub const fn specular(&self) -> &ToonSpecularSettings {
        &self.specular
    }
    /// Returns Fresnel-rim controls.
    #[must_use]
    pub const fn rim(&self) -> &RimLightingSettings {
        &self.rim
    }
    /// Returns hair, rim, and missing-ILM-A fallback masks.
    #[must_use]
    pub fn material_masks(&self) -> [f32; 3] {
        self.material_masks.truncate().to_array()
    }
    /// Returns the configured base-color readability floor.
    #[must_use]
    pub fn readability_floor(&self) -> f32 {
        self.material_masks.w
    }
    /// Returns whether hair is enabled.
    #[must_use]
    pub const fn hair_enabled(&self) -> bool {
        self.state.x != 0
    }
    /// Returns whether toon specular is enabled.
    #[must_use]
    pub const fn specular_enabled(&self) -> bool {
        self.state.y != 0
    }
    /// Returns whether rim lighting is enabled.
    #[must_use]
    pub const fn rim_enabled(&self) -> bool {
        self.state.z != 0
    }
    /// Returns the selected isolated diagnostic.
    #[must_use]
    pub const fn debug_mode(&self) -> CelLightingDebugMode {
        CelLightingDebugMode::from_shader_value(self.state.w)
    }
    /// Mirrors finite Kajiya-Kay shader math for tests and tools.
    #[must_use]
    pub fn hair_response(&self, tangent: Vec3, normal: Vec3, view: Vec3, light: Vec3) -> f32 {
        self.hair.response(tangent, normal, view, light)
    }
}

const fn finite_unit(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        fallback
    }
}
