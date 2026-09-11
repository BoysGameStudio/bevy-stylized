use bevy::{
    math::{UVec4, Vec3, Vec4},
    prelude::Reflect,
    render::render_resource::ShaderType,
};

use crate::{FaceBasis, FaceSdfDebugMode, FaceSdfLightSample, FaceSdfMapSide};

const BASIS_EPSILON: f32 = 1.0e-5;
const MAX_SOFTNESS: f32 = 0.5;

/// Validated shader controls, basis, texture state, and diagnostics for face SDFs.
#[derive(Clone, Copy, Debug, PartialEq, Reflect, ShaderType)]
pub struct FaceSdfSettings {
    basis_forward_bias: Vec4,
    basis_right_rotation: Vec4,
    shadow_tint_softness: Vec4,
    state: Vec4,
    debug_dimensions: UVec4,
    genshin_direction: Vec4,
}

impl Default for FaceSdfSettings {
    fn default() -> Self {
        Self {
            basis_forward_bias: Vec4::ZERO,
            basis_right_rotation: Vec4::ZERO,
            shadow_tint_softness: Vec4::new(1.0, 1.0, 1.0, 0.0),
            state: Vec4::new(0.0, 0.0, 0.0, 1.0),
            debug_dimensions: UVec4::ZERO,
            genshin_direction: Vec4::ZERO,
        }
    }
}

impl FaceSdfSettings {
    /// Creates enabled controls with a validated basis but no bound textures.
    #[must_use]
    pub fn from_basis(basis: FaceBasis) -> Self {
        Self::default().with_basis(Some(basis))
    }

    /// Sets normalized threshold bias and Y-axis light rotation in radians.
    #[must_use]
    pub fn with_bias_rotation(mut self, bias: f32, rotation: f32) -> Self {
        self.basis_forward_bias.w = finite_clamped(bias, -1.0, 1.0, 0.0);
        self.basis_right_rotation.w =
            finite_clamped(rotation, -std::f32::consts::PI, std::f32::consts::PI, 0.0);
        self
    }

    /// Sets normalized threshold transition width.
    #[must_use]
    pub fn with_softness(mut self, softness: f32) -> Self {
        self.shadow_tint_softness.w = finite_clamped(softness, 0.0, MAX_SOFTNESS, 0.0);
        self
    }

    /// Sets linear RGB tint applied to the SDF shadow region.
    #[must_use]
    pub fn with_shadow_tint(mut self, tint: [f32; 3]) -> Self {
        self.shadow_tint_softness.x = finite_unit(tint[0]);
        self.shadow_tint_softness.y = finite_unit(tint[1]);
        self.shadow_tint_softness.z = finite_unit(tint[2]);
        self
    }

    /// Sets how strongly verified forward-light attenuation modulates the SDF result.
    #[must_use]
    pub fn with_external_shadow_influence(mut self, influence: f32) -> Self {
        self.state.x = finite_clamped(influence, 0.0, 1.0, 0.0);
        self
    }

    /// Enables or disables only the face override.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.state.w = if enabled { 1.0 } else { 0.0 };
        self
    }

    /// Selects a shader diagnostic output.
    #[must_use]
    pub const fn with_debug(mut self, mode: FaceSdfDebugMode) -> Self {
        self.debug_dimensions.x = mode.shader_value();
        self
    }

    /// Mirrors the right-facing threshold lookup horizontally. This supports
    /// the common Gacha face-lightmap convention where a single authored SDF
    /// is sampled normally for one light side and with flipped U for the other.
    #[must_use]
    pub const fn with_right_uv_mirrored(mut self, mirrored: bool) -> Self {
        self.debug_dimensions.w = mirrored as u32;
        self
    }

    /// Selects the HoYoverse half-dot/power direction transfer function and
    /// its authored `Shadow Position`. Generic faces retain the angular model.
    #[must_use]
    pub fn with_genshin_direction(mut self, shadow_position: f32) -> Self {
        self.genshin_direction.x = 1.0;
        self.genshin_direction.y = finite_clamped(shadow_position, 0.0, 1.0, 0.55);
        self
    }

    pub(crate) fn with_genshin_face_shadow_texture(mut self, enabled: bool) -> Self {
        self.genshin_direction.z = if enabled { 1.0 } else { 0.0 };
        self
    }

    pub(crate) fn with_basis(mut self, basis: Option<FaceBasis>) -> Self {
        if let Some(basis) = basis {
            self.basis_forward_bias = basis.forward().extend(self.basis_forward_bias.w);
            self.basis_right_rotation = basis.right().extend(self.basis_right_rotation.w);
            self.state.z = 1.0;
        } else {
            self.basis_forward_bias = Vec3::ZERO.extend(self.basis_forward_bias.w);
            self.basis_right_rotation = Vec3::ZERO.extend(self.basis_right_rotation.w);
            self.state.z = 0.0;
        }
        self
    }

    pub(crate) const fn with_texture_dimensions(mut self, width: u32, height: u32) -> Self {
        self.debug_dimensions.y = width;
        self.debug_dimensions.z = height;
        self
    }

    pub(crate) fn with_textures(mut self, enabled: bool) -> Self {
        self.state.y = if enabled { 1.0 } else { 0.0 };
        self
    }

    /// Returns the current validated world basis, if available.
    #[must_use]
    pub fn basis(&self) -> Option<FaceBasis> {
        if self.state.z <= 0.5 {
            return None;
        }
        FaceBasis::try_new(
            self.basis_forward_bias.truncate(),
            self.basis_right_rotation.truncate(),
        )
        .ok()
    }

    /// Returns normalized threshold bias.
    #[must_use]
    pub fn bias(&self) -> f32 {
        self.basis_forward_bias.w
    }

    /// Returns Y-axis light rotation in radians.
    #[must_use]
    pub fn rotation(&self) -> f32 {
        self.basis_right_rotation.w
    }

    /// Returns normalized threshold transition width.
    #[must_use]
    pub fn softness(&self) -> f32 {
        self.shadow_tint_softness.w
    }

    /// Returns linear RGB face-shadow tint.
    #[must_use]
    pub fn shadow_tint(&self) -> [f32; 3] {
        self.shadow_tint_softness.truncate().to_array()
    }

    /// Returns external forward-shadow modulation strength.
    #[must_use]
    pub fn external_shadow_influence(&self) -> f32 {
        self.state.x
    }

    /// Returns the explicit user enable state.
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.state.w > 0.5
    }

    /// Returns whether matching validated threshold textures are bound.
    #[must_use]
    pub fn uses_textures(&self) -> bool {
        self.state.y > 0.5
    }

    pub(crate) const fn texture_dimensions(&self) -> Option<(u32, u32)> {
        if self.debug_dimensions.y == 0 || self.debug_dimensions.z == 0 {
            None
        } else {
            Some((self.debug_dimensions.y, self.debug_dimensions.z))
        }
    }

    /// Returns the active diagnostic output.
    #[must_use]
    pub const fn debug_mode(&self) -> FaceSdfDebugMode {
        FaceSdfDebugMode::from_shader_value(self.debug_dimensions.x)
    }

    /// Returns whether right-facing SDF samples flip the U coordinate.
    #[must_use]
    pub const fn right_uv_mirrored(&self) -> bool {
        self.debug_dimensions.w != 0
    }

    /// Returns whether the source HoYoverse direction transfer is active.
    #[must_use]
    pub const fn uses_genshin_direction(&self) -> bool {
        self.genshin_direction.to_array()[0] > 0.5
    }

    /// Returns the validated source `Shadow Position`.
    #[must_use]
    pub const fn genshin_shadow_position(&self) -> f32 {
        self.genshin_direction.to_array()[1]
    }

    /// Returns whether the source packed face-shadow correction map is bound.
    #[must_use]
    pub const fn uses_genshin_face_shadow_texture(&self) -> bool {
        self.genshin_direction.to_array()[2] > 0.5
    }

    /// Mirrors shader direction selection for tools and deterministic tests.
    #[must_use]
    pub fn light_sample(&self, light_direction: Vec3) -> Option<FaceSdfLightSample> {
        let basis = self.basis()?;
        let light = Vec3::new(light_direction.x, 0.0, light_direction.z);
        if !light.is_finite() || light.length_squared() <= BASIS_EPSILON {
            return None;
        }
        let light = light.normalize();
        let (sin_rotation, cos_rotation) = self.rotation().sin_cos();
        let rotated = Vec3::new(
            light.z.mul_add(-sin_rotation, light.x * cos_rotation),
            0.0,
            light.z.mul_add(cos_rotation, light.x * sin_rotation),
        );
        let right_dot = basis.right().dot(rotated).clamp(-1.0, 1.0);
        let forward_dot = basis.forward().dot(rotated).clamp(-1.0, 1.0);
        let threshold = if self.uses_genshin_direction() {
            genshin_direction_threshold(forward_dot, self.genshin_shadow_position())
        } else {
            (forward_dot.acos() / std::f32::consts::PI + self.bias()).clamp(0.0, 1.0)
        };
        let selected_map = if self.uses_genshin_direction() {
            if right_dot > 0.0 {
                FaceSdfMapSide::Left
            } else {
                FaceSdfMapSide::Right
            }
        } else if right_dot < 0.0 {
            FaceSdfMapSide::Left
        } else {
            FaceSdfMapSide::Right
        };
        Some(FaceSdfLightSample::new(
            selected_map,
            threshold,
            [right_dot.mul_add(0.5, 0.5), forward_dot.mul_add(0.5, 0.5)],
        ))
    }
}

fn genshin_direction_threshold(forward_dot: f32, shadow_position: f32) -> f32 {
    let half_forward = forward_dot.mul_add(0.5, 0.5).clamp(0.0, 0.999);
    let exponent = (1.5 - shadow_position).clamp(0.0, 1.5).powi(3);
    half_forward.powf(exponent).clamp(0.0, 1.0)
}

const fn finite_clamped(value: f32, minimum: f32, maximum: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(minimum, maximum)
    } else {
        fallback
    }
}

fn finite_unit(value: f32) -> f32 {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        value
    } else {
        1.0
    }
}
