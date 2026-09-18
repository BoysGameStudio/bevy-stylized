//! Opt-in physical lighting followed by an authored color ramp.
//!
//! This retains Bevy's StandardMaterial texture, light and shadow handling.
//! It is independent of the authored-light Character/Hero shading paths.
use crate::host_material::{HostFrameInputs, HostMaterial, HostMaterialPlugin, HostMaterialSource};
use bevy::{
    asset::embedded_asset,
    mesh::MeshVertexBufferLayoutRef,
    pbr::{
        ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
        MaterialPlugin,
    },
    prelude::*,
    render::render_resource::{
        AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};

pub const PHYSICAL_CEL_SHADER: &str =
    "embedded://bevy_stylized_render/shaders/physical/physical_forward.wgsl";

#[derive(Clone, Copy, Debug, Reflect, ShaderType)]
pub struct PhysicalCelParams {
    /// Coordinate scale, offset, band smoothing and enable flag.
    pub transform: Vec4,
    /// Base row, row count, band count and second-page row offset.
    pub atlas: UVec4,
    /// Page blend, rim enable, rim mask and reserved.
    pub controls: Vec4,
    /// Rim power, threshold, smoothing and intensity.
    pub rim: Vec4,
    pub rim_color: Vec4,
}
impl Default for PhysicalCelParams {
    fn default() -> Self {
        Self {
            transform: Vec4::new(1., 0., 0., 0.),
            atlas: UVec4::new(0, 1, 1, 0),
            controls: Vec4::new(0., 0., 1., 0.),
            rim: Vec4::new(3., 0.5, 0.05, 0.),
            rim_color: Vec4::ONE,
        }
    }
}

#[derive(Asset, AsBindGroup, Clone, Debug, Reflect, Default)]
pub struct PhysicalCelExtension {
    #[uniform(100)]
    pub params: PhysicalCelParams,
    #[texture(101)]
    #[sampler(102)]
    pub ramp: Option<Handle<Image>>,
}

pub type PhysicalCelMaterial = ExtendedMaterial<StandardMaterial, PhysicalCelExtension>;

impl MaterialExtension for PhysicalCelExtension {
    fn vertex_shader() -> ShaderRef {
        PHYSICAL_CEL_SHADER.into()
    }
    fn fragment_shader() -> ShaderRef {
        PHYSICAL_CEL_SHADER.into()
    }
    fn prepass_vertex_shader() -> ShaderRef {
        crate::skinning::COMPLETE_SKIN_PREPASS_SHADER.into()
    }
    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        crate::skinning::specialize_secondary_skin(descriptor, layout)
    }
}

impl HostMaterialSource for PhysicalCelMaterial {
    fn set_host_alpha_mode(&mut self, alpha: crate::host_material::HostAlphaMode) {
        use crate::host_material::HostAlphaMode;
        match alpha {
            HostAlphaMode::Inherit => {}
            HostAlphaMode::Opaque => self.base.alpha_mode = AlphaMode::Opaque,
            HostAlphaMode::Blend => self.base.alpha_mode = AlphaMode::Blend,
        }
    }
    fn frame_uniform_binding() -> u32 {
        100
    }
    fn frame_uniform(&self, frame: &HostFrameInputs) -> Result<Vec<u8>, String> {
        crate::host_inputs::pack_host_material_uniform(
            &self.extension.params,
            &frame.instances,
            &frame.views,
        )
        .map_err(|error| format!("{error:?}"))
    }
}

/// Install only when physical-light ramp materials are requested by the host.
/// The base Stylized shader imports must already be registered by StylizedPlugin.
pub struct PhysicalCelPlugin;
impl Plugin for PhysicalCelPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/physical/physical_forward.wgsl");
        if !app.is_plugin_added::<HostMaterialPlugin>() {
            app.add_plugins(HostMaterialPlugin);
        }
        app.add_plugins((
            MaterialPlugin::<PhysicalCelMaterial>::default(),
            MaterialPlugin::<HostMaterial<PhysicalCelMaterial>>::default(),
        ));
    }
}
