//! Naga validation of every embedded WGSL shader with Bevy's own composer.
//!
//! Bevy 0.19 shaders use `#import` / `#{...}` preprocessor directives that
//! plain naga cannot parse, so the test loads every shader through Bevy's
//! asset pipeline (with the render-stack plugins that register all bevy_pbr /
//! bevy_core_pipeline shader libraries; no render device is needed) and
//! composes each entry point with [`ShaderCache`], the same naga-oil machinery
//! the renderer uses. The composed module is then validated with naga directly.

use bevy::{
    app::App,
    asset::{AssetId, AssetPlugin, Assets, LoadState},
    core_pipeline::{CorePipelinePlugin, FullscreenShader},
    image::ImagePlugin,
    pbr::PbrPlugin,
    prelude::*,
    render::{
        RenderPlugin,
        render_resource::{DownlevelFlags, WgpuFeatures},
        settings::{RenderCreation, WgpuSettings},
    },
    shader::{Shader, ShaderCache, ShaderCacheError, ShaderCacheSource, ShaderDefVal},
};

/// Material shaders bind their group through this naga-oil substitution.
const MATERIAL_BIND_GROUP: u32 = 3;

fn shader_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        ImagePlugin::default(),
        bevy::mesh::MeshPlugin,
        bevy::window::WindowPlugin::default(),
    ));
    // RenderPlugin with no backend registers the Shader asset type, the
    // shader loader, and bevy_render's own shader libraries without ever
    // creating a GPU device.
    app.add_plugins(RenderPlugin {
        render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
            backends: None,
            ..Default::default()
        })),
        ..Default::default()
    });
    app.add_plugins((
        CorePipelinePlugin,
        PbrPlugin {
            gltf_enable_standard_materials: false,
            ..Default::default()
        },
        bevy_cel::CharacterMaterialPlugin,
    ));
    // `App::update` does not finish plugins; `finish` is where the crate
    // eagerly loads the pure WGSL import module (`outline_shared.wgsl`).
    app.finish();
    app.cleanup();
    app
}

/// Loads an embedded crate shader and drives the app until it is available.
fn load_crate_shader(app: &mut App, path: &'static str) -> AssetId<Shader> {
    let handle: Handle<Shader> = app.world().resource::<AssetServer>().load(path);
    let id = handle.id();
    // The handle is dropped after loading; keep the asset alive.
    core::mem::forget(handle);
    for _ in 0..600 {
        app.update();
        match app.world().resource::<AssetServer>().load_state(id) {
            LoadState::Loaded => return id,
            LoadState::Failed(error) => panic!("shader {path} failed to load: {error}"),
            _ => {}
        }
    }
    panic!("shader {path} did not finish loading");
}

/// A shader entry point plus one shader-def permutation to validate.
struct Case {
    path: &'static str,
    defs: &'static [&'static str],
}

impl Case {
    const fn new(path: &'static str, defs: &'static [&'static str]) -> Self {
        Self { path, defs }
    }
}

fn cases() -> Vec<Case> {
    const VERTEX_BASE: &[&str] = &["VERTEX_POSITIONS", "VERTEX_NORMALS", "VERTEX_UVS_A"];
    const VERTEX_SKINNED: &[&str] = &[
        "VERTEX_POSITIONS",
        "VERTEX_NORMALS",
        "VERTEX_UVS_A",
        "SKINNED",
    ];
    const VERTEX_SKINNED_EXTENDED: &[&str] = &[
        "VERTEX_POSITIONS",
        "VERTEX_NORMALS",
        "VERTEX_UVS_A",
        "SKINNED",
        "CEL_SKIN_EXTENDED",
        "CEL_SKIN_WEIGHTS_EXTENDED",
    ];
    const VERTEX_MORPHED: &[&str] = &[
        "VERTEX_POSITIONS",
        "VERTEX_NORMALS",
        "VERTEX_UVS_A",
        "MORPH_TARGETS",
    ];
    const VERTEX_FULL: &[&str] = &[
        "VERTEX_POSITIONS",
        "VERTEX_NORMALS",
        "VERTEX_UVS_A",
        "VERTEX_UVS_B",
        "VERTEX_TANGENTS",
        "VERTEX_COLORS",
        "SKINNED",
        "CEL_SKIN_EXTENDED",
        "CEL_SKIN_WEIGHTS_EXTENDED",
        "MORPH_TARGETS",
    ];
    const FRAGMENT_FULL: &[&str] = &[
        "VERTEX_POSITIONS",
        "VERTEX_NORMALS",
        "VERTEX_UVS_A",
        "VERTEX_UVS_B",
        "VERTEX_TANGENTS",
        "VERTEX_COLORS",
        "CHARACTER_MASK_ATTRIBUTE",
        "CHARACTER_MATERIAL_ID_ATTRIBUTE",
        "CHARACTER_HAIR_TANGENT_ATTRIBUTE",
        "CHARACTER_HAIR_MASK_ATTRIBUTE",
        "CHARACTER_EMISSION_ATTRIBUTE",
    ];
    const PREPASS_FULL: &[&str] = &[
        "VERTEX_POSITIONS",
        "VERTEX_NORMALS",
        "VERTEX_UVS_A",
        "VERTEX_TANGENTS",
        "VERTEX_COLORS",
        "SKINNED",
        "CEL_SKIN_EXTENDED",
        "MORPH_TARGETS",
        "HAS_PREVIOUS_SKIN",
        "HAS_PREVIOUS_MORPH",
        "MOTION_VECTOR_PREPASS",
        "NORMAL_PREPASS_OR_DEFERRED_PREPASS",
        "DEPTH_PREPASS",
        "NORMAL_PREPASS",
    ];
    let outline_vertex = "embedded://bevy_cel/shaders/outline_vertex.wgsl";
    let internal_edge_vertex = "embedded://bevy_cel/shaders/internal_edge_vertex.wgsl";
    let mut cases = Vec::new();
    for vertex in [outline_vertex, internal_edge_vertex] {
        cases.push(Case::new(vertex, VERTEX_BASE));
        cases.push(Case::new(vertex, VERTEX_SKINNED));
        cases.push(Case::new(vertex, VERTEX_SKINNED_EXTENDED));
        cases.push(Case::new(vertex, VERTEX_MORPHED));
        cases.push(Case::new(vertex, VERTEX_FULL));
    }
    cases.push(Case::new(
        "embedded://bevy_cel/shaders/outline_fragment.wgsl",
        &["VERTEX_POSITIONS", "VERTEX_NORMALS", "VERTEX_UVS_A"],
    ));
    cases.push(Case::new(
        "embedded://bevy_cel/shaders/outline_fragment.wgsl",
        &["VERTEX_POSITIONS", "VERTEX_NORMALS"],
    ));
    cases.push(Case::new(
        "embedded://bevy_cel/shaders/internal_edge.wgsl",
        &["VERTEX_POSITIONS", "VERTEX_NORMALS"],
    ));
    cases.push(Case::new(
        "embedded://bevy_cel/shaders/internal_edge.wgsl",
        &[
            "VERTEX_POSITIONS",
            "VERTEX_NORMALS",
            "VERTEX_UVS_A",
            "DEPTH_PREPASS",
            "NORMAL_PREPASS",
        ],
    ));
    cases.push(Case::new(
        "embedded://bevy_cel/shaders/character_vertex.wgsl",
        VERTEX_BASE,
    ));
    cases.push(Case::new(
        "embedded://bevy_cel/shaders/character_vertex.wgsl",
        VERTEX_SKINNED_EXTENDED,
    ));
    cases.push(Case::new(
        "embedded://bevy_cel/shaders/character_vertex.wgsl",
        VERTEX_MORPHED,
    ));
    cases.push(Case::new(
        "embedded://bevy_cel/shaders/character_vertex.wgsl",
        &[
            "VERTEX_POSITIONS",
            "VERTEX_NORMALS",
            "VERTEX_UVS_A",
            "CHARACTER_MASK_ATTRIBUTE",
            "CHARACTER_MATERIAL_ID_ATTRIBUTE",
            "CHARACTER_HAIR_TANGENT_ATTRIBUTE",
            "CHARACTER_HAIR_MASK_ATTRIBUTE",
            "CHARACTER_EMISSION_ATTRIBUTE",
        ],
    ));
    cases.push(Case::new(
        "embedded://bevy_cel/shaders/character_forward.wgsl",
        &["VERTEX_POSITIONS", "VERTEX_NORMALS", "VERTEX_UVS_A"],
    ));
    cases.push(Case::new(
        "embedded://bevy_cel/shaders/character_forward.wgsl",
        FRAGMENT_FULL,
    ));
    cases.push(Case::new(
        "embedded://bevy_cel/shaders/character_prepass.wgsl",
        VERTEX_BASE,
    ));
    cases.push(Case::new(
        "embedded://bevy_cel/shaders/character_prepass.wgsl",
        PREPASS_FULL,
    ));
    cases.push(Case::new(
        "embedded://bevy_cel/shaders/stylized_post_process.wgsl",
        &[],
    ));
    cases
}

/// naga-validates a fully composed shader module.
fn validate_composed(
    _device: &(),
    source: ShaderCacheSource,
    _validate: &bevy::shader::ValidateShader,
) -> Result<(), ShaderCacheError> {
    let module = match &source {
        ShaderCacheSource::Wgsl(source) => naga::front::wgsl::parse_str(source)
            .map_err(|error| ShaderCacheError::CreateShaderModule(error.to_string()))?,
        ShaderCacheSource::Naga(module) => module.clone(),
        ShaderCacheSource::SpirV(_) => {
            return Err(ShaderCacheError::CreateShaderModule(
                "unexpected spirv source".into(),
            ));
        }
    };
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    validator
        .validate(&module)
        .map(|_| ())
        .map_err(|error| ShaderCacheError::CreateShaderModule(error.to_string()))
}

#[test]
fn every_embedded_wgsl_shader_composes_and_validates_with_naga() {
    // Given the render-stack plugins so every shader library (bevy_pbr,
    // bevy_core_pipeline, bevy_render-less, bevy_cel) is registered.
    let mut app = shader_app();
    // The fullscreen vertex shader module is only loaded on demand.
    app.world_mut().init_resource::<FullscreenShader>();

    let mut shader_ids = Vec::new();
    let mut seen = Vec::new();
    for case in cases() {
        if seen.contains(&case.path) {
            continue;
        }
        seen.push(case.path);
        shader_ids.push(load_crate_shader(&mut app, case.path));
    }

    // When every crate shader is composed through Bevy's shader cache with
    // realistic shader-def permutations
    let shaders: Vec<(AssetId<Shader>, Shader)> = app
        .world()
        .resource::<Assets<Shader>>()
        .iter()
        .map(|(id, shader)| (id, shader.clone()))
        .collect();
    let mut cache = ShaderCache::new(
        (),
        WgpuFeatures::all(),
        DownlevelFlags::all(),
        validate_composed,
    );
    for (id, shader) in shaders {
        cache.set_shader(id, shader);
    }

    // Then every permutation composes and passes full naga validation
    for (pipeline, case) in cases().into_iter().enumerate() {
        let shader = shader_ids[seen.iter().position(|p| *p == case.path).unwrap()];
        let mut defs: Vec<ShaderDefVal> = case
            .defs
            .iter()
            .map(|def| ShaderDefVal::from(*def))
            .collect();
        if !case.path.ends_with("stylized_post_process.wgsl") {
            // The mesh pipeline pushes these into every mesh-material compose.
            defs.push(ShaderDefVal::from("MESH_PIPELINE"));
            defs.push(ShaderDefVal::from("VERTEX_OUTPUT_INSTANCE_INDEX"));
            defs.push(ShaderDefVal::UInt(
                "MATERIAL_BIND_GROUP".into(),
                MATERIAL_BIND_GROUP,
            ));
        }
        // PipelineCache pushes this device-derived def into every compose.
        defs.push(ShaderDefVal::UInt(
            "AVAILABLE_STORAGE_BUFFER_BINDINGS".into(),
            8,
        ));
        if let Err(error) = cache.get(pipeline, shader, &defs) {
            let available: Vec<String> = app
                .world()
                .resource::<Assets<Shader>>()
                .iter()
                .map(|(_, s)| s.import_path.module_name().into_owned())
                .collect();
            let needed = app
                .world()
                .resource::<Assets<Shader>>()
                .get(shader)
                .map(|s| s.imports.clone());
            panic!(
                "shader {} with defs {:?} failed: {error}\ndirect imports: {needed:?}\navailable modules: {available:?}",
                case.path, case.defs
            );
        }
    }
}
