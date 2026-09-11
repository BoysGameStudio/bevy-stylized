use bevy::{
    color::LinearRgba,
    core_pipeline::prepass::{DepthPrepass, NormalPrepass},
    mesh::MeshVertexBufferLayoutRef,
    pbr::{Material, MaterialPipeline, MaterialPipelineKey},
    prelude::{
        Asset, Assets, Camera3d, Handle, Has, Image, Local, Query, Reflect, Res, Resource, With,
    },
    render::render_resource::{
        AsBindGroup, Face, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};

use crate::EyeEmissionSettings;

/// Embedded vertex shader used by the inverted hull.
pub const OUTLINE_VERTEX_SHADER_PATH: &str = "embedded://bevy_cel/shaders/outline_vertex.wgsl";
/// Embedded fragment shader used by the inverted hull.
pub const OUTLINE_FRAGMENT_SHADER_PATH: &str = "embedded://bevy_cel/shaders/outline_fragment.wgsl";
/// Embedded vertex shader used by the optional prepass accent.
///
/// Unlike the hull vertex shader this binds the 32-byte
/// [`InternalEdgeAccentSettings`] uniform; both vertex shaders share the same
/// deformation logic through the `bevy_cel::outline_shared` WGSL module.
pub const INTERNAL_EDGE_VERTEX_SHADER_PATH: &str =
    "embedded://bevy_cel/shaders/internal_edge_vertex.wgsl";
/// Embedded fragment shader used by the optional prepass accent.
pub const INTERNAL_EDGE_SHADER_PATH: &str = "embedded://bevy_cel/shaders/internal_edge.wgsl";

/// Validated per-material inverted-hull appearance and exclusion controls.
#[derive(Clone, Copy, Debug, PartialEq, Reflect, ShaderType)]
pub struct OutlineSettings {
    color_width: bevy::math::Vec4,
    masks: bevy::math::Vec4,
    source_color_1: bevy::math::Vec4,
    source_color_2: bevy::math::Vec4,
    source_color_3: bevy::math::Vec4,
    source_color_4: bevy::math::Vec4,
    source_color_5: bevy::math::Vec4,
    source_modulation: bevy::math::Vec4,
    source_params: bevy::math::Vec4,
    source_state: bevy::math::UVec4,
}

impl Default for OutlineSettings {
    fn default() -> Self {
        Self {
            color_width: bevy::math::Vec4::new(0.015, 0.02, 0.035, Self::DEFAULT_WIDTH),
            masks: bevy::math::Vec4::ZERO,
            source_color_1: bevy::math::Vec4::ONE,
            source_color_2: bevy::math::Vec4::ONE,
            source_color_3: bevy::math::Vec4::ONE,
            source_color_4: bevy::math::Vec4::ONE,
            source_color_5: bevy::math::Vec4::ONE,
            source_modulation: bevy::math::Vec4::ONE,
            source_params: bevy::math::Vec4::ZERO,
            source_state: bevy::math::UVec4::ZERO,
        }
    }
}

impl OutlineSettings {
    /// Default outline width in physical pixels.
    pub const DEFAULT_WIDTH: f32 = 2.0;
    /// Maximum outline width in physical pixels.
    pub const MAX_WIDTH: f32 = 24.0;

    /// Sets the linear outline color.
    #[must_use]
    pub const fn with_color(mut self, color: LinearRgba) -> Self {
        let values = self.color_width.to_array();
        self.color_width = bevy::math::Vec4::new(
            finite_nonnegative(color.red, 0.015),
            finite_nonnegative(color.green, 0.02),
            finite_nonnegative(color.blue, 0.035),
            values[3],
        );
        self
    }

    /// Sets a finite bounded width in physical pixels.
    #[must_use]
    pub const fn with_width(mut self, width: f32) -> Self {
        let values = self.color_width.to_array();
        let width = if width.is_nan() {
            Self::DEFAULT_WIDTH
        } else if width <= 0.0 {
            0.0
        } else if width.is_infinite() || width > Self::MAX_WIDTH {
            Self::MAX_WIDTH
        } else {
            width
        };
        self.color_width = bevy::math::Vec4::new(values[0], values[1], values[2], width);
        self
    }

    /// Sets per-material face and eye exclusion weights.
    #[must_use]
    pub const fn with_exclusion_masks(mut self, face: f32, eye: f32) -> Self {
        self.masks = bevy::math::Vec4::new(finite_unit(face), finite_unit(eye), 0.0, 0.0);
        self
    }

    /// Imports task18's eye-class outline-suppression seam.
    #[must_use]
    pub const fn with_task18_suppression(mut self, settings: &EyeEmissionSettings) -> Self {
        let (_, suppression) = settings.masks();
        let (eye_class, _) = settings.eye_class();
        let masks = self.masks.to_array();
        self.masks = bevy::math::Vec4::new(masks[0], suppression * eye_class, 0.0, 0.0);
        self
    }

    /// Selects the source Gacha local-space hull and five lightmap classes.
    #[must_use]
    pub fn with_genshin_source(
        mut self,
        colors: [LinearRgba; 5],
        enabled_mask: u32,
        local_width: f32,
        force_color_1: bool,
        has_lightmap: bool,
    ) -> Self {
        let color = |value: LinearRgba| {
            bevy::math::Vec4::new(
                finite_nonnegative(value.red, 1.0),
                finite_nonnegative(value.green, 1.0),
                finite_nonnegative(value.blue, 1.0),
                1.0,
            )
        };
        self.source_color_1 = color(colors[0]);
        self.source_color_2 = color(colors[1]);
        self.source_color_3 = color(colors[2]);
        self.source_color_4 = color(colors[3]);
        self.source_color_5 = color(colors[4]);
        self.source_params.x = if local_width.is_finite() {
            local_width.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.source_state = bevy::math::UVec4::new(
            1,
            enabled_mask & 0x0f,
            u32::from(force_color_1),
            u32::from(has_lightmap),
        );
        self
    }

    /// Updates Ambient × SharpLit without changing source class data.
    #[must_use]
    pub fn with_genshin_modulation(mut self, modulation: [f32; 3]) -> Self {
        self.source_modulation = bevy::math::Vec4::new(
            finite_nonnegative(modulation[0], 1.0),
            finite_nonnegative(modulation[1], 1.0),
            finite_nonnegative(modulation[2], 1.0),
            1.0,
        );
        self
    }

    /// Whether the source local-space outline path is selected.
    #[must_use]
    pub const fn uses_genshin_source(self) -> bool {
        self.source_state.x != 0
    }

    /// Source local-space extrusion width.
    #[must_use]
    pub const fn genshin_local_width(self) -> f32 {
        self.source_params.to_array()[0]
    }

    /// Returns the linear outline color.
    #[must_use]
    pub const fn color(self) -> LinearRgba {
        let values = self.color_width.to_array();
        LinearRgba::new(values[0], values[1], values[2], 1.0)
    }

    /// Returns the bounded width in physical pixels.
    #[must_use]
    pub const fn width(self) -> f32 {
        self.color_width.to_array()[3]
    }

    /// Returns face and eye exclusion weights.
    #[must_use]
    pub const fn exclusion_masks(self) -> (f32, f32) {
        let masks = self.masks.to_array();
        (masks[0], masks[1])
    }

    /// Reports whether the hull can match the source alpha mode.
    ///
    /// Alpha blending is unsupported because a depth-writing opaque hull cannot preserve
    /// order-dependent transparency. Alpha masks are also unsupported because the separate hull
    /// does not own the source material's alpha texture or cutoff.
    #[must_use]
    pub const fn supports_alpha_mode(self, mode: bevy::material::AlphaMode) -> bool {
        matches!(mode, bevy::material::AlphaMode::Opaque)
    }
}

/// Separate front-culled inverted-hull material.
#[derive(Asset, AsBindGroup, Clone, Debug, Default, Reflect)]
pub struct OutlineMaterial {
    /// Per-material outline appearance and exclusion controls.
    #[uniform(0)]
    pub settings: OutlineSettings,
    /// Optional packed source lightmap used for five outline color classes.
    #[texture(1)]
    #[sampler(2)]
    pub lightmap_texture: Option<Handle<Image>>,
}

impl OutlineMaterial {
    /// Returns the embedded deformation-preserving vertex shader path.
    #[must_use]
    pub const fn vertex_shader_path() -> &'static str {
        OUTLINE_VERTEX_SHADER_PATH
    }

    /// Returns the embedded hull fragment shader path.
    #[must_use]
    pub const fn fragment_shader_path() -> &'static str {
        OUTLINE_FRAGMENT_SHADER_PATH
    }

    /// Returns the fixed opaque hull alpha mode.
    #[must_use]
    pub const fn alpha_mode(&self) -> bevy::material::AlphaMode {
        bevy::material::AlphaMode::Opaque
    }
}

impl Material for OutlineMaterial {
    fn vertex_shader() -> ShaderRef {
        OUTLINE_VERTEX_SHADER_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        OUTLINE_FRAGMENT_SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> bevy::material::AlphaMode {
        self.alpha_mode()
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = Some(Face::Front);
        crate::append_skin_attributes(descriptor, layout)?;
        Ok(())
    }
}

/// Optional depth/normal-prepass internal-edge controls.
#[derive(Clone, Copy, Debug, PartialEq, Reflect, ShaderType)]
pub struct InternalEdgeAccentSettings {
    color_thresholds: bevy::math::Vec4,
    state: bevy::math::UVec4,
}

impl Default for InternalEdgeAccentSettings {
    fn default() -> Self {
        Self {
            color_thresholds: bevy::math::Vec4::new(0.03, 0.04, 0.07, 0.01),
            state: bevy::math::UVec4::ZERO,
        }
    }
}

impl InternalEdgeAccentSettings {
    /// Enables or disables prepass edge detection.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.state.x = u32::from(enabled);
        self
    }

    /// Returns whether edge detection was requested.
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        self.state.x != 0
    }

    /// Returns whether the requested accent has both required prepass inputs.
    #[must_use]
    pub const fn is_active(self, has_depth: bool, has_normal: bool) -> bool {
        self.is_enabled() && has_depth && has_normal
    }
}

/// Separate transparent pass for optional prepass-derived internal edges.
#[derive(Asset, AsBindGroup, Clone, Debug, Default, Reflect)]
pub struct InternalEdgeMaterial {
    /// Per-material internal-edge settings.
    #[uniform(0)]
    pub settings: InternalEdgeAccentSettings,
}

impl InternalEdgeMaterial {
    /// Returns the embedded vertex shader path binding the small accent uniform.
    #[must_use]
    pub const fn vertex_shader_path() -> &'static str {
        INTERNAL_EDGE_VERTEX_SHADER_PATH
    }

    /// Returns the embedded prepass-edge fragment shader path.
    #[must_use]
    pub const fn fragment_shader_path() -> &'static str {
        INTERNAL_EDGE_SHADER_PATH
    }
}

impl Material for InternalEdgeMaterial {
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        crate::append_skin_attributes(descriptor, layout)
    }

    fn vertex_shader() -> ShaderRef {
        INTERNAL_EDGE_VERTEX_SHADER_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        INTERNAL_EDGE_SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> bevy::material::AlphaMode {
        bevy::material::AlphaMode::Blend
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn depth_bias(&self) -> f32 {
        1.0
    }
}

#[derive(Resource, Default)]
#[expect(
    clippy::redundant_pub_crate,
    reason = "the sibling plugin module registers this private system state"
)]
pub(crate) struct InternalEdgeDiagnosticState(bool);

#[expect(
    clippy::needless_pass_by_value,
    clippy::redundant_pub_crate,
    reason = "Bevy systems consume parameters by value and the sibling plugin registers this system"
)]
pub(crate) fn diagnose_missing_internal_edge_prepass(
    materials: Res<Assets<InternalEdgeMaterial>>,
    cameras: Query<(Has<DepthPrepass>, Has<NormalPrepass>), With<Camera3d>>,
    mut state: Local<InternalEdgeDiagnosticState>,
) {
    let requested = materials
        .iter()
        .any(|(_, material)| material.settings.is_enabled());
    let available = cameras.iter().any(|(depth, normal)| depth && normal);
    if should_report_missing_prepass(requested, available, state.0) {
        bevy::log::warn!(
            "bevy_cel internal-edge accent disabled: add DepthPrepass and NormalPrepass to a Camera3d"
        );
        state.0 = true;
    }
}

const fn should_report_missing_prepass(requested: bool, available: bool, reported: bool) -> bool {
    requested && !available && !reported
}

const fn finite_nonnegative(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        fallback
    }
}

const fn finite_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::should_report_missing_prepass;

    #[test]
    fn missing_prepass_diagnostic_is_once_only() {
        // Given
        let requested = true;
        let available = false;

        // When
        let first = should_report_missing_prepass(requested, available, false);
        let repeated = should_report_missing_prepass(requested, available, first);

        // Then
        assert!(first);
        assert!(!repeated);
    }
}
