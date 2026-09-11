use bevy::{
    math::{Vec3, Vec4},
    prelude::Reflect,
    render::render_resource::ShaderType,
};

const DIRECTION_EPSILON: f32 = 1.0e-5;

/// Validated Kajiya-Kay tangent-shift hair highlight controls.
#[derive(Clone, Copy, Debug, PartialEq, Reflect, ShaderType)]
pub struct HairHighlightSettings {
    tint_shift: Vec4,
    shape: Vec4,
}

impl Default for HairHighlightSettings {
    fn default() -> Self {
        Self {
            tint_shift: Vec4::new(1.0, 1.0, 1.0, 0.0),
            shape: Vec4::new(1.0, 1.0, 0.0, 0.0),
        }
    }
}

impl HairHighlightSettings {
    /// Sets linear RGB highlight tint and normal-direction tangent shift.
    #[must_use]
    pub fn with_tint_shift(mut self, tint: [f32; 3], shift: f32) -> Self {
        self.tint_shift =
            Vec3::from_array(tint.map(finite_unit)).extend(finite_clamped(shift, -1.0, 1.0, 0.0));
        self
    }

    /// Sets anisotropic exponent and toon threshold.
    #[must_use]
    pub fn with_shape(mut self, power: f32, threshold: f32) -> Self {
        self.shape.x = finite_clamped(power, 1.0, 256.0, 1.0);
        self.shape.y = finite_clamped(threshold, 0.0, 1.0, 1.0);
        self
    }

    /// Sets toon transition width and additive linear intensity.
    #[must_use]
    pub fn with_softness_intensity(mut self, softness: f32, intensity: f32) -> Self {
        self.shape.z = finite_clamped(softness, 0.0, 0.5, 0.0);
        self.shape.w = finite_clamped(intensity, 0.0, 16.0, 0.0);
        self
    }

    #[must_use]
    /// Returns linear RGB highlight tint.
    pub fn tint(&self) -> [f32; 3] {
        self.tint_shift.truncate().to_array()
    }
    #[must_use]
    /// Returns normal-direction tangent shift in `[-1, 1]`.
    pub fn shift(&self) -> f32 {
        self.tint_shift.w
    }
    #[must_use]
    /// Returns the anisotropic exponent.
    pub fn power(&self) -> f32 {
        self.shape.x
    }
    #[must_use]
    /// Returns the toon cutoff in `[0, 1]`.
    pub fn threshold(&self) -> f32 {
        self.shape.y
    }
    #[must_use]
    /// Returns cutoff transition width in `[0, 0.5]`.
    pub fn softness(&self) -> f32 {
        self.shape.z
    }
    #[must_use]
    /// Returns additive linear intensity.
    pub fn intensity(&self) -> f32 {
        self.shape.w
    }

    pub(crate) fn response(&self, tangent: Vec3, normal: Vec3, view: Vec3, light: Vec3) -> f32 {
        let normal = safe_normalize(normal, Vec3::Z);
        let view = safe_normalize(view, normal);
        let light = safe_normalize(light, normal);
        let fallback = safe_normalize(normal.cross(Vec3::Y), normal.cross(Vec3::X));
        let tangent = safe_normalize(tangent, fallback);
        let shifted = safe_normalize(tangent + normal * self.shift(), fallback);
        let half_vector = safe_normalize(view + light, normal);
        let tangent_dot_half = shifted.dot(half_vector);
        let anisotropic = tangent_dot_half
            .mul_add(-tangent_dot_half, 1.0)
            .max(0.0)
            .sqrt();
        let powered = anisotropic.powf(self.power());
        let lower = (self.threshold() - self.softness()).max(0.0);
        let upper = (self.threshold() + self.softness()).min(1.0);
        if upper <= lower {
            f32::from(powered >= self.threshold())
        } else {
            let value = ((powered - lower) / (upper - lower)).clamp(0.0, 1.0);
            value * value * 2.0_f32.mul_add(-value, 3.0)
        }
    }
}

fn safe_normalize(value: Vec3, fallback: Vec3) -> Vec3 {
    if value.is_finite() && value.length_squared() > DIRECTION_EPSILON {
        value.normalize()
    } else if fallback.is_finite() && fallback.length_squared() > DIRECTION_EPSILON {
        fallback.normalize()
    } else {
        Vec3::X
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
