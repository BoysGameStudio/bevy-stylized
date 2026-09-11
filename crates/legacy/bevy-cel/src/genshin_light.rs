use bevy::{
    math::{Vec3, Vec4},
    prelude::Reflect,
    render::render_resource::ShaderType,
};

const DIRECTION_EPSILON_SQUARED: f32 = 1.0e-10;
const MAX_REFERENCE_ENERGY: f32 = 16.0;

/// Optional world-space light input for source-authored Genshin materials.
///
/// `direction_to_light_world` has the same NdotL-ready meaning as Bevy's
/// `GpuDirectionalLight::dir_to_light`, but is independent of the view light
/// list. The disabled default preserves the existing physical-light fallback.
#[derive(Clone, Copy, Debug, PartialEq, Reflect, ShaderType)]
pub struct GenshinLightSource {
    direction_enabled: Vec4,
    tint_energy: Vec4,
}

impl Default for GenshinLightSource {
    fn default() -> Self {
        Self {
            direction_enabled: Vec3::Y.extend(0.0),
            tint_energy: Vec4::ONE,
        }
    }
}

impl GenshinLightSource {
    /// Creates an enabled light from a finite nonzero world direction.
    ///
    /// Invalid directions fail closed to the disabled default. Tint is
    /// clamped to display-independent unit RGB; finite energy supports HDR.
    #[must_use]
    pub fn explicit(direction_to_light_world: Vec3, tint: [f32; 3], energy: f32) -> Self {
        if !direction_to_light_world.is_finite()
            || direction_to_light_world.length_squared() <= DIRECTION_EPSILON_SQUARED
        {
            return Self::default();
        }
        Self {
            direction_enabled: direction_to_light_world.normalize().extend(1.0),
            tint_energy: Vec3::from_array(tint.map(finite_unit)).extend(finite_energy(energy)),
        }
    }

    /// Returns whether the explicit source replaces the physical-light list.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.direction_enabled.w > 0.5
    }

    /// Returns the normalized world-space NdotL direction.
    #[must_use]
    pub fn direction_to_light_world(&self) -> Vec3 {
        self.direction_enabled.truncate()
    }

    /// Returns the source-authored linear RGB light tint.
    #[must_use]
    pub fn tint(&self) -> [f32; 3] {
        self.tint_energy.truncate().to_array()
    }

    /// Returns source light energy. Zero is a valid explicit blackout.
    #[must_use]
    pub fn energy(&self) -> f32 {
        self.tint_energy.w
    }
}

fn finite_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

fn finite_energy(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, MAX_REFERENCE_ENERGY)
    } else {
        1.0
    }
}
