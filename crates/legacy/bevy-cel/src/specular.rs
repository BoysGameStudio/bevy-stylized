use bevy::{
    math::{Vec3, Vec4},
    prelude::Reflect,
    render::render_resource::ShaderType,
};

/// Validated smoothstep toon-specular controls masked by ILM R.
#[derive(Clone, Copy, Debug, PartialEq, Reflect, ShaderType)]
pub struct ToonSpecularSettings {
    tint_intensity: Vec4,
    shape: Vec4,
}

impl Default for ToonSpecularSettings {
    fn default() -> Self {
        Self {
            tint_intensity: Vec4::new(1.0, 1.0, 1.0, 0.0),
            shape: Vec4::new(1.0, 1.0, 0.0, 0.0),
        }
    }
}

impl ToonSpecularSettings {
    /// Sets linear RGB tint and additive linear intensity.
    #[must_use]
    pub fn with_tint_intensity(mut self, tint: [f32; 3], intensity: f32) -> Self {
        self.tint_intensity = Vec3::from_array(tint.map(finite_unit))
            .extend(finite_clamped(intensity, 0.0, 16.0, 0.0));
        self
    }

    /// Sets finite specular exponent and toon threshold.
    #[must_use]
    pub fn with_shape(mut self, power: f32, threshold: f32) -> Self {
        self.shape.x = finite_clamped(power, 1.0, 256.0, 1.0);
        self.shape.y = finite_clamped(threshold, 0.0, 1.0, 1.0);
        self
    }

    /// Sets threshold transition width in `[0, 0.5]`.
    #[must_use]
    pub fn with_softness(mut self, softness: f32) -> Self {
        self.shape.z = finite_clamped(softness, 0.0, 0.5, 0.0);
        self
    }

    /// Returns linear RGB tint.
    #[must_use]
    pub fn tint(&self) -> [f32; 3] {
        self.tint_intensity.truncate().to_array()
    }
    /// Returns additive linear intensity.
    #[must_use]
    pub fn intensity(&self) -> f32 {
        self.tint_intensity.w
    }
    /// Returns the specular exponent.
    #[must_use]
    pub fn power(&self) -> f32 {
        self.shape.x
    }
    /// Returns the toon cutoff.
    #[must_use]
    pub fn threshold(&self) -> f32 {
        self.shape.y
    }
    /// Returns cutoff transition width.
    #[must_use]
    pub fn softness(&self) -> f32 {
        self.shape.z
    }
}

const fn finite_unit(value: f32) -> f32 {
    if value.is_finite() && value >= 0.0 && value <= 1.0 {
        value
    } else {
        1.0
    }
}
const fn finite_clamped(value: f32, minimum: f32, maximum: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(minimum, maximum)
    } else {
        fallback
    }
}
