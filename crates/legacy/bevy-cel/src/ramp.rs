use bevy::{
    math::{UVec4, Vec4},
    prelude::Reflect,
    render::render_resource::ShaderType,
};

const MAX_RAMP_ROWS: u32 = 4_096;
// Full-width authored Gacha ramps commonly contain 256 texels. Keeping the
// clamp at that source width lets the shader sample every authored column
// without changing the compact three-band defaults used elsewhere.
const MAX_RAMP_BANDS: u32 = 256;
const MAX_RAMP_SCALE: f32 = 8.0;
const MAX_RAMP_OFFSET: f32 = 4.0;
const MAX_BAND_SMOOTHING: f32 = 0.5;
const MAX_AUTHORED_EMISSION_GAIN: f32 = 8.0;
const MAX_GENSHIN_SOURCE_SCALE: f32 = 1_024.0;

/// Shader-visible ramp diagnostic output.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Reflect)]
pub enum ToonRampDebugMode {
    /// Render the toon-shaded material.
    #[default]
    Disabled,
    /// Render the normalized ramp coordinate as grayscale.
    Coordinate,
    /// Render the sampled ramp band directly.
    Band,
}

impl ToonRampDebugMode {
    const fn shader_value(self) -> u32 {
        match self {
            Self::Disabled => 0,
            Self::Coordinate => 1,
            Self::Band => 2,
        }
    }

    const fn from_shader_value(value: u32) -> Self {
        match value {
            1 => Self::Coordinate,
            2 => Self::Band,
            0 | 3..=u32::MAX => Self::Disabled,
        }
    }
}

/// Validated controls for sampling one row of a toon ramp atlas.
#[derive(Clone, Copy, Debug, PartialEq, Reflect, ShaderType)]
pub struct ToonRampSettings {
    transform: Vec4,
    atlas: UVec4,
    atlas_blend: Vec4,
    authored_emission_gain: Vec4,
    genshin_body: Vec4,
    genshin_global_shadow: Vec4,
}

impl Default for ToonRampSettings {
    fn default() -> Self {
        Self {
            transform: Vec4::new(1.0, 0.0, 0.0, 0.0),
            atlas: UVec4::new(0, 1, 3, 0),
            atlas_blend: Vec4::ZERO,
            authored_emission_gain: Vec4::ZERO,
            genshin_body: Vec4::new(0.55, 1.0, 0.0, 1.0),
            genshin_global_shadow: Vec4::new(1.0, 1.0, 1.0, 0.0),
        }
    }
}

impl ToonRampSettings {
    /// Selects an atlas row and declares the number of rows in the atlas.
    #[must_use]
    pub fn with_atlas_row(mut self, row: u32, row_count: u32) -> Self {
        let safe_count = row_count.clamp(1, MAX_RAMP_ROWS);
        self.atlas.x = row.min(safe_count - 1);
        self.atlas.y = safe_count;
        self
    }

    /// Blends the selected atlas row range with a second range at `row_offset`.
    ///
    /// This is useful for paired authored pages such as Genshin's ten daytime
    /// rows followed by ten nighttime rows. The default offset and blend are
    /// both zero, preserving single-page ramps exactly.
    #[must_use]
    pub fn with_atlas_blend(mut self, row_offset: u32, blend: f32) -> Self {
        self.atlas_blend.x = row_offset.min(MAX_RAMP_ROWS - 1) as f32;
        self.atlas_blend.y = if blend.is_finite() {
            blend.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self
    }

    /// Declares how many rendered world units equal one source-Blender unit.
    ///
    /// Blender's UV-driven Bump basis is sensitive to object scale. Keeping
    /// this explicit lets a scaled glTF wrapper reconstruct the same surface
    /// gradient without coupling the generic shader to one character profile.
    #[must_use]
    pub fn with_genshin_source_scale(mut self, scale: f32) -> Self {
        self.atlas_blend.z = finite_clamped(scale, 0.001, MAX_GENSHIN_SOURCE_SCALE, 1.0);
        self
    }

    /// Enables source material-ID classes 2 through 5 as a four-bit mask.
    /// Disabled classes fall back to class 1, matching the Blender groups.
    #[must_use]
    pub fn with_genshin_material_id_mask(mut self, enabled_mask: u32) -> Self {
        self.atlas_blend.w = (enabled_mask & 0x0f) as f32;
        self
    }

    /// Sets the source-authored diffuse emission multiplier for opt-in
    /// character layouts.
    ///
    /// Zero disables the branch. Positive values reproduce the source's
    /// cutoff and twice-applied diffuse-alpha mask around this gain. The
    /// fragment shader applies it only to the explicit Genshin layout, so the
    /// default cannot alter canonical materials.
    #[must_use]
    pub fn with_authored_emission_gain(mut self, gain: f32) -> Self {
        self.authored_emission_gain.x = if gain.is_finite() {
            gain.clamp(0.0, MAX_AUTHORED_EMISSION_GAIN)
        } else {
            0.0
        };
        self
    }

    /// Sets the source body-ramp position/width and whether vertex-color AO
    /// participates. These values are consumed only by the Genshin ILM layout.
    #[must_use]
    pub fn with_genshin_body(
        mut self,
        shadow_position: f32,
        shadow_ramp_width: f32,
        vertex_color_ao: bool,
    ) -> Self {
        self.genshin_body.x = finite_clamped(shadow_position, 0.01, 1.0, 0.55);
        self.genshin_body.y = finite_clamped(shadow_ramp_width, 0.01, 4.0, 1.0);
        self.genshin_body.z = if vertex_color_ao { 1.0 } else { 0.0 };
        self
    }

    /// Calibrates one source material after Bevy's physical-light proxy has
    /// been converted into the authored hero-light domain.
    #[must_use]
    pub fn with_genshin_exposure(mut self, exposure: f32) -> Self {
        self.genshin_body.w = finite_clamped(exposure, 0.25, 1.5, 1.0);
        self
    }

    /// Sets the global authored shadow colour used by the source lighting
    /// combiner. Both its soft and sharp shadow stages use this value; white
    /// keeps the generic/default material path unchanged.
    #[must_use]
    pub fn with_genshin_global_shadow(mut self, color: [f32; 3]) -> Self {
        self.genshin_global_shadow.x = finite_unit(color[0], 1.0);
        self.genshin_global_shadow.y = finite_unit(color[1], 1.0);
        self.genshin_global_shadow.z = finite_unit(color[2], 1.0);
        self
    }

    /// Blends from the source-authored lighting chain to an optional physical
    /// Bevy light-energy/colour modulation. Zero is exact source parity;
    /// positive values are intended for explicit interactive-lighting modes.
    #[must_use]
    pub fn with_genshin_physical_light_influence(mut self, influence: f32) -> Self {
        self.genshin_global_shadow.w = finite_unit(influence, 0.0);
        self
    }

    /// Sets the number of light bands and their normalized transition width.
    #[must_use]
    pub fn with_bands(mut self, count: u32, smoothing: f32) -> Self {
        self.atlas.z = count.clamp(1, MAX_RAMP_BANDS);
        self.transform.z = if smoothing.is_finite() {
            smoothing.clamp(0.0, MAX_BAND_SMOOTHING)
        } else {
            0.0
        };
        self
    }

    /// Applies scale and offset before the ramp coordinate is clamped.
    #[must_use]
    pub fn with_transform(mut self, scale: f32, offset: f32) -> Self {
        self.transform.x = if scale.is_finite() {
            scale.clamp(0.0, MAX_RAMP_SCALE)
        } else {
            1.0
        };
        self.transform.y = if offset.is_finite() {
            offset.clamp(-MAX_RAMP_OFFSET, MAX_RAMP_OFFSET)
        } else {
            0.0
        };
        self
    }

    /// Selects a shader diagnostic output.
    #[must_use]
    pub const fn with_debug(mut self, mode: ToonRampDebugMode) -> Self {
        self.atlas.w = mode.shader_value();
        self
    }

    pub(crate) fn with_texture(mut self, enabled: bool) -> Self {
        self.transform.w = if enabled { 1.0 } else { 0.0 };
        self
    }

    /// Zero-based atlas row selected for sampling.
    #[must_use]
    pub const fn ramp_row(&self) -> u32 {
        self.atlas.x
    }

    /// Declared number of rows in the atlas.
    #[must_use]
    pub const fn ramp_row_count(&self) -> u32 {
        self.atlas.y
    }

    /// Row offset of the optional secondary atlas page.
    #[must_use]
    pub fn atlas_blend_row_offset(&self) -> u32 {
        self.atlas_blend.x.round() as u32
    }

    /// Blend factor from the primary to the secondary atlas page.
    #[must_use]
    pub fn atlas_blend_factor(&self) -> f32 {
        self.atlas_blend.y
    }

    /// Rendered-world units per source-Blender unit for the UV Bump basis.
    #[must_use]
    pub fn genshin_source_scale(&self) -> f32 {
        let scale = self.atlas_blend.z;
        if scale > 0.0 { scale } else { 1.0 }
    }

    /// Four-bit source enable mask for material-ID classes 2 through 5.
    #[must_use]
    pub fn genshin_material_id_mask(&self) -> u32 {
        self.atlas_blend.w.round() as u32 & 0x0f
    }

    /// Source-authored diffuse emission multiplier. Zero means disabled.
    #[must_use]
    pub fn authored_emission_gain(&self) -> f32 {
        self.authored_emission_gain.x
    }

    /// Returns source body-ramp position, width, and vertex-color-AO enable.
    #[must_use]
    pub fn genshin_body(&self) -> (f32, f32, bool) {
        (
            self.genshin_body.x,
            self.genshin_body.y,
            self.genshin_body.z > 0.5,
        )
    }

    /// Returns the source-material exposure calibration.
    #[must_use]
    pub fn genshin_exposure(&self) -> f32 {
        self.genshin_body.w
    }

    /// Returns the global authored RGB shadow colour.
    #[must_use]
    pub fn genshin_global_shadow(&self) -> [f32; 3] {
        self.genshin_global_shadow.truncate().to_array()
    }

    /// Returns the optional physical-light modulation weight.
    #[must_use]
    pub fn genshin_physical_light_influence(&self) -> f32 {
        self.genshin_global_shadow.w
    }

    /// Number of lighting bands.
    #[must_use]
    pub const fn band_count(&self) -> u32 {
        self.atlas.z
    }

    /// Normalized smoothing width at each band boundary.
    #[must_use]
    pub fn band_smoothing(&self) -> f32 {
        self.transform.z
    }

    /// Scale applied to the light proxy.
    #[must_use]
    pub fn ramp_scale(&self) -> f32 {
        self.transform.x
    }

    /// Offset applied to the light proxy.
    #[must_use]
    pub fn ramp_offset(&self) -> f32 {
        self.transform.y
    }

    /// Active debug output.
    #[must_use]
    pub const fn debug_mode(&self) -> ToonRampDebugMode {
        ToonRampDebugMode::from_shader_value(self.atlas.w)
    }

    /// Whether an authored ramp texture is active.
    #[must_use]
    pub fn uses_texture(&self) -> bool {
        self.transform.w != 0.0
    }
}

fn finite_clamped(value: f32, minimum: f32, maximum: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(minimum, maximum)
    } else {
        fallback
    }
}

fn finite_unit(value: f32, fallback: f32) -> f32 {
    finite_clamped(value, 0.0, 1.0, fallback)
}
