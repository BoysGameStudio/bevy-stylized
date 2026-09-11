use bevy::{
    math::{UVec4, Vec4},
    prelude::Reflect,
    render::render_resource::ShaderType,
};

/// Source-authored HoYoverse metallic matcap and specular-ramp controls.
///
/// The packed ILM texture selects this branch with a strict `R > 0.9` test.
/// It deliberately does not map to `StandardMaterial::metallic`: the source
/// renderer is a view-space matcap, not a physically based metal surface.
#[derive(Clone, Copy, Debug, PartialEq, Reflect, ShaderType)]
pub struct GenshinMetallicSettings {
    matcap_dark: Vec4,
    matcap_light: Vec4,
    matcap_shadow: Vec4,
    specular_color: Vec4,
    sharp_layer_color: Vec4,
    matcap: Vec4,
    specular: Vec4,
    state: UVec4,
}

impl Default for GenshinMetallicSettings {
    fn default() -> Self {
        Self {
            matcap_dark: Vec4::new(0.647_058_8, 0.574_871_7, 0.542_387_5, 1.0),
            matcap_light: Vec4::new(0.985_293_9, 0.985_293_9, 0.985_293_9, 1.0),
            matcap_shadow: Vec4::new(0.935_831_1, 0.885_813_2, 0.941_176_5, 1.0),
            specular_color: Vec4::ONE,
            sharp_layer_color: Vec4::ONE,
            // brightness, tile scale, reserved, reserved
            matcap: Vec4::new(3.0, 1.0, 0.0, 0.0),
            // shininess, scale, attenuation, sharp-layer offset
            specular: Vec4::new(4.0, 1.2, 0.2, 0.95),
            // requested, validated texture pair, reserved, reserved
            state: UVec4::ZERO,
        }
    }
}

impl GenshinMetallicSettings {
    /// Requests the authored metallic branch. It becomes active only after a
    /// valid sRGB matcap and linear specular ramp have both been bound.
    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.state.x = enabled as u32;
        self
    }

    pub(crate) const fn with_textures(mut self, available: bool) -> Self {
        self.state.y = available as u32;
        self
    }

    /// Whether the host requested this source branch.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.state.x != 0
    }

    /// Whether both validated source textures are available.
    #[must_use]
    pub const fn has_textures(&self) -> bool {
        self.state.y != 0
    }

    /// Source matcap colors before the Blender Gamma 2.2 nodes.
    #[must_use]
    pub const fn matcap_colors(&self) -> ([f32; 4], [f32; 4], [f32; 4]) {
        (
            self.matcap_dark.to_array(),
            self.matcap_light.to_array(),
            self.matcap_shadow.to_array(),
        )
    }

    /// Brightness and tile scale of the view-space matcap.
    #[must_use]
    pub fn matcap_shape(&self) -> (f32, f32) {
        (self.matcap.x, self.matcap.y)
    }

    /// Shininess, scale, attenuation, and sharp-layer offset.
    #[must_use]
    pub const fn specular_shape(&self) -> [f32; 4] {
        self.specular.to_array()
    }
}
