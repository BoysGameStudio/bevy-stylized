//! Reusable cel-shading and NPR rendering foundations for Bevy.
//!
//! [`CharacterMaterial`] is deliberately forward-only. The plugin rewrites
//! `Auto` and `Deferred` requests to [`OpaqueRendererMethod::Forward`] before
//! material extraction because deferred extensions cannot alter final lit color.

use bevy::{
    mesh::MeshVertexBufferLayoutRef,
    pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline},
    prelude::{Asset, Handle, Image, Reflect, StandardMaterial, UVec4},
    render::render_resource::{
        AsBindGroup, Face, RenderPipelineDescriptor, SpecializedMeshPipelineError, TextureFormat,
    },
    shader::ShaderRef,
};
mod authoring;
mod authoring_material;
mod authoring_object;
mod eye_emission;
mod face;
mod face_runtime;
mod face_settings;
mod genshin_display_transform;
mod genshin_light;
mod genshin_lighting;
mod genshin_metallic;
mod hair;
mod ilm;
mod lighting;
mod outline;
mod outline_runtime;
mod plugin;
mod post_process;
mod post_process_runtime;
mod ramp;
mod rim;
mod skinning;
mod specular;
pub use skinning::skin_vertex_positions;
include!("extension.rs");
include!("specialization.rs");

pub use authoring::{
    CEL_NPR_DOMAIN, CEL_NPR_MATERIALS_VERSION, CelNprAuthoring, CelNprMetadataError,
};
pub use authoring_material::{
    CelNprMaterialAuthoring, CelNprMaterialEntry, HairAuthoring, IlmAuthoring, OutlineAuthoring,
    RampAuthoring, RimAuthoring, SpecularAuthoring, TextureFilter,
};
pub use authoring_object::{CelNprObjectAuthoring, FaceRigAuthoring, PostAuthoring};
pub use eye_emission::{EyeEmissionDebugMode, EyeEmissionSettings};
pub use face::{FaceBasis, FaceBasisError, FaceSdfDebugMode, FaceSdfLightSample, FaceSdfMapSide};
pub use face_runtime::{FaceSdfInstanceError, FaceSdfMaterialInstance, FaceSdfRig};
pub use face_settings::FaceSdfSettings;
pub use genshin_display_transform::{
    GENSHIN_DISPLAY_TRANSFORM_SHADER_PATH, GenshinDisplayTransform,
    genshin_display_transform_channel,
};
pub use genshin_light::GenshinLightSource;
pub use genshin_lighting::{GenshinLightingDebugMode, GenshinLightingPalette};
pub use genshin_metallic::GenshinMetallicSettings;
pub use hair::HairHighlightSettings;
pub use ilm::{IlmDebugMode, IlmLayout, IlmMaterialIdSource, IlmSettings};
pub use lighting::{CelLightingDebugMode, CelLightingSettings};
pub use outline::{
    INTERNAL_EDGE_SHADER_PATH, INTERNAL_EDGE_VERTEX_SHADER_PATH, InternalEdgeAccentSettings,
    InternalEdgeMaterial, OUTLINE_FRAGMENT_SHADER_PATH, OUTLINE_VERTEX_SHADER_PATH,
    OutlineMaterial, OutlineSettings,
};
pub use outline_runtime::{CharacterOutline, CharacterOutlineSystems};
pub use plugin::{
    CharacterFaceBasisSystems, CharacterMaterialPlugin, character_gltf_plugin, character_material,
};
pub use post_process::{
    PostProcessStage, STYLIZED_POST_PROCESS_ORDER, StylizedPostProcessProfile,
    StylizedPostProcessSettings,
};
pub use post_process_runtime::{
    STYLIZED_POST_PROCESS_SHADER_PATH, StylizedFullscreenEffect, StylizedPostProcessPlugin,
};
pub use ramp::{ToonRampDebugMode, ToonRampSettings};
pub use rim::RimLightingSettings;
pub use specular::ToonSpecularSettings;

/// Compatibility name for task 5's canonical `_CEL_MASK` attribute.
pub use bevy_gltf_metadata::ATTRIBUTE_CEL_MASK as ATTRIBUTE_CHARACTER_MASK;
pub use bevy_gltf_metadata::{
    ATTRIBUTE_CEL_EMISSION, ATTRIBUTE_CEL_HAIR_MASK, ATTRIBUTE_CEL_HAIR_TANGENT,
    ATTRIBUTE_CEL_JOINTS_1, ATTRIBUTE_CEL_MATERIAL_ID, ATTRIBUTE_CEL_WEIGHTS_1,
};

/// Embedded shader path used by [`CharacterExtension`].
pub const CHARACTER_FORWARD_SHADER_PATH: &str =
    "embedded://bevy_cel/shaders/character_forward.wgsl";
/// Embedded vertex shader path used by [`CharacterExtension`].
pub const CHARACTER_VERTEX_SHADER_PATH: &str = "embedded://bevy_cel/shaders/character_vertex.wgsl";

/// Shader location reserved for the first optional character vertex attribute.
pub const CHARACTER_MASK_SHADER_LOCATION: u32 = 10;
/// Shader location reserved for task 5's canonical `_CEL_MATERIAL_ID` attribute.
pub const CHARACTER_MATERIAL_ID_SHADER_LOCATION: u32 = 11;
/// Shader location for task 5's canonical `_CEL_HAIR_TANGENT` attribute.
pub const CHARACTER_HAIR_TANGENT_SHADER_LOCATION: u32 = 12;
/// Shader location for task 5's canonical `_CEL_HAIR_MASK` attribute.
pub const CHARACTER_HAIR_MASK_SHADER_LOCATION: u32 = 13;
/// Shader location for task 5's canonical `_CEL_EMISSION` attribute.
pub const CHARACTER_EMISSION_SHADER_LOCATION: u32 = 14;

/// Uniform binding for [`ToonRampSettings`].
pub const TOON_RAMP_UNIFORM_BINDING: u32 = 100;
/// Linear 2D ramp-atlas texture binding.
pub const RAMP_TEXTURE_BINDING: u32 = 101;
/// Ramp-atlas sampler binding.
pub const RAMP_SAMPLER_BINDING: u32 = 102;
/// Uniform binding for [`IlmSettings`].
pub const ILM_UNIFORM_BINDING: u32 = 103;
/// Linear-data 2D ILM texture binding. Authored images must use a non-sRGB format.
pub const ILM_TEXTURE_BINDING: u32 = 104;
/// ILM sampler binding.
pub const ILM_SAMPLER_BINDING: u32 = 105;
/// Uniform binding for directional face-SDF controls and world basis.
pub const FACE_SDF_UNIFORM_BINDING: u32 = 106;
/// Linear left-facing face threshold texture binding.
pub const FACE_SDF_LEFT_TEXTURE_BINDING: u32 = 107;
/// Left-facing face threshold sampler binding.
pub const FACE_SDF_LEFT_SAMPLER_BINDING: u32 = 108;
/// Linear right-facing face threshold texture binding.
pub const FACE_SDF_RIGHT_TEXTURE_BINDING: u32 = 109;
/// Right-facing face threshold sampler binding.
pub const FACE_SDF_RIGHT_SAMPLER_BINDING: u32 = 110;
/// Uniform binding for hair highlight, toon specular, rim, and their material masks.
pub const CEL_LIGHTING_UNIFORM_BINDING: u32 = 111;
/// Uniform binding for eye readability and HDR emission controls.
pub const EYE_EMISSION_UNIFORM_BINDING: u32 = 112;
/// Optional linear emission-mask texture binding.
pub const EMISSION_TEXTURE_BINDING: u32 = 113;
/// Emission-mask sampler binding.
pub const EMISSION_SAMPLER_BINDING: u32 = 114;
/// Uniform binding for optional Blender-style backface UV1 base-color sampling.
pub const BACKFACE_BASE_COLOR_UNIFORM_BINDING: u32 = 115;
/// Optional packed HoYoverse face-shadow correction texture binding.
pub const FACE_SHADOW_TEXTURE_BINDING: u32 = 116;
/// Packed face-shadow correction sampler binding.
pub const FACE_SHADOW_SAMPLER_BINDING: u32 = 117;
/// Source HoYoverse sRGB metallic-matcap texture binding.
pub const GENSHIN_METAL_MAP_TEXTURE_BINDING: u32 = 118;
/// Metallic-matcap sampler binding.
pub const GENSHIN_METAL_MAP_SAMPLER_BINDING: u32 = 119;
/// Source HoYoverse linear specular-ramp texture binding.
pub const GENSHIN_SPECULAR_RAMP_TEXTURE_BINDING: u32 = 120;
/// Specular-ramp sampler binding.
pub const GENSHIN_SPECULAR_RAMP_SAMPLER_BINDING: u32 = 121;
/// Uniform binding for HoYoverse metallic matcap/specular controls.
pub const GENSHIN_METALLIC_UNIFORM_BINDING: u32 = 122;
/// Packed HoYoverse RG normal texture binding.
pub const GENSHIN_PACKED_NORMAL_TEXTURE_BINDING: u32 = 123;
/// Packed HoYoverse RG normal sampler binding.
pub const GENSHIN_PACKED_NORMAL_SAMPLER_BINDING: u32 = 124;
/// Uniform binding for the source-authored Genshin global lighting palette.
pub const GENSHIN_LIGHTING_PALETTE_UNIFORM_BINDING: u32 = 125;
/// Uniform binding for an explicit world-space Genshin light source.
pub const GENSHIN_LIGHT_SOURCE_UNIFORM_BINDING: u32 = 126;

/// Cull-mode specialization reserved for a later outline pass.
///
/// This only selects raster culling. It does not implement outline geometry or shading.
/// The default [`OutlineCullMode::Inherit`] keeps the cull mode derived from the base
/// [`StandardMaterial`]; the other variants override it during pipeline specialization.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Reflect)]
pub enum OutlineCullMode {
    /// Keep the cull mode derived from the base material unchanged.
    #[default]
    Inherit,
    /// Render front faces, culling back faces.
    Back,
    /// Render back faces, culling front faces.
    Front,
    /// Render both sides.
    None,
}

impl OutlineCullMode {
    /// Returns the overriding raster cull face, or `None` for
    /// [`OutlineCullMode::Inherit`] and [`OutlineCullMode::None`].
    const fn face(self) -> Option<Face> {
        match self {
            Self::Inherit => None,
            Self::Back => Some(Face::Back),
            Self::Front => Some(Face::Front),
            Self::None => None,
        }
    }

    /// Applies this cull mode to a specialized pipeline descriptor.
    ///
    /// [`OutlineCullMode::Inherit`] leaves the base-material-derived cull mode
    /// untouched; the other variants override it.
    pub fn apply(self, descriptor: &mut RenderPipelineDescriptor) {
        if self != Self::Inherit {
            descriptor.primitive.cull_mode = self.face();
        }
    }
}

/// Extension data and specialization state for forward character materials.
///
/// Binding 100 starts the extension range; bindings 0-99 remain reserved for
/// `StandardMaterial` and Bevy's material infrastructure.
#[derive(Asset, AsBindGroup, Clone, Debug, Reflect)]
#[bind_group_data(CharacterExtensionKey)]
pub struct CharacterExtension {
    #[uniform(100)]
    ramp_settings: ToonRampSettings,
    #[texture(101)]
    #[sampler(102)]
    ramp_texture: Option<Handle<Image>>,
    #[uniform(103)]
    ilm_settings: IlmSettings,
    #[texture(104)]
    #[sampler(105)]
    ilm_texture: Option<Handle<Image>>,
    #[uniform(106)]
    face_sdf_settings: FaceSdfSettings,
    #[texture(107)]
    #[sampler(108)]
    face_sdf_left_texture: Option<Handle<Image>>,
    #[texture(109)]
    #[sampler(110)]
    face_sdf_right_texture: Option<Handle<Image>>,
    #[uniform(111)]
    cel_lighting_settings: CelLightingSettings,
    #[uniform(112)]
    eye_emission_settings: EyeEmissionSettings,
    #[texture(113)]
    #[sampler(114)]
    emission_texture: Option<Handle<Image>>,
    #[uniform(115)]
    backface_base_color: UVec4,
    #[texture(116)]
    #[sampler(117)]
    face_shadow_texture: Option<Handle<Image>>,
    #[texture(118)]
    #[sampler(119)]
    genshin_metal_map: Option<Handle<Image>>,
    #[texture(120)]
    #[sampler(121)]
    genshin_specular_ramp: Option<Handle<Image>>,
    #[uniform(122)]
    genshin_metallic_settings: GenshinMetallicSettings,
    #[texture(123)]
    #[sampler(124)]
    genshin_packed_normal: Option<Handle<Image>>,
    #[uniform(125)]
    genshin_lighting_palette: GenshinLightingPalette,
    #[uniform(126)]
    genshin_light_source: GenshinLightSource,
    /// Raster culling seam used by a future outline pass.
    ///
    /// Defaults to [`OutlineCullMode::Inherit`], which keeps the cull mode derived
    /// from the base `StandardMaterial` instead of overriding it.
    pub outline_cull_mode: OutlineCullMode,
}

/// Cheap pipeline key for character extension permutations.
#[repr(C)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct CharacterExtensionKey {
    outline_cull_mode: OutlineCullMode,
}

impl From<&CharacterExtension> for CharacterExtensionKey {
    fn from(extension: &CharacterExtension) -> Self {
        Self {
            outline_cull_mode: extension.outline_cull_mode,
        }
    }
}

impl MaterialExtension for CharacterExtension {
    fn prepass_vertex_shader() -> ShaderRef {
        "embedded://bevy_cel/shaders/character_prepass.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        CHARACTER_VERTEX_SHADER_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        CHARACTER_FORWARD_SHADER_PATH.into()
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        key.bind_group_data.outline_cull_mode.apply(descriptor);
        append_skin_attributes(descriptor, layout)?;

        append_optional_attribute(
            descriptor,
            layout,
            OptionalAttribute::new(
                ATTRIBUTE_CHARACTER_MASK,
                CHARACTER_MASK_SHADER_LOCATION,
                "CHARACTER_MASK_ATTRIBUTE",
            ),
        );
        append_optional_attribute(
            descriptor,
            layout,
            OptionalAttribute::new(
                ATTRIBUTE_CEL_EMISSION,
                CHARACTER_EMISSION_SHADER_LOCATION,
                "CHARACTER_EMISSION_ATTRIBUTE",
            ),
        );
        append_optional_attribute(
            descriptor,
            layout,
            OptionalAttribute::new(
                ATTRIBUTE_CEL_HAIR_TANGENT,
                CHARACTER_HAIR_TANGENT_SHADER_LOCATION,
                "CHARACTER_HAIR_TANGENT_ATTRIBUTE",
            ),
        );
        append_optional_attribute(
            descriptor,
            layout,
            OptionalAttribute::new(
                ATTRIBUTE_CEL_HAIR_MASK,
                CHARACTER_HAIR_MASK_SHADER_LOCATION,
                "CHARACTER_HAIR_MASK_ATTRIBUTE",
            ),
        );
        append_optional_attribute(
            descriptor,
            layout,
            OptionalAttribute::new(
                ATTRIBUTE_CEL_MATERIAL_ID,
                CHARACTER_MATERIAL_ID_SHADER_LOCATION,
                "CHARACTER_MATERIAL_ID_ATTRIBUTE",
            ),
        );

        Ok(())
    }
}

/// Supported character material type.
pub type CharacterMaterial = ExtendedMaterial<StandardMaterial, CharacterExtension>;
