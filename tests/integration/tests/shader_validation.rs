//! Naga validation of every embedded WGSL shader with Bevy's own composer
//! （与 legacy `bevy-cel/tests/shader_validation.rs` 同一机制：经 Bevy 资产
//! 管线加载 → `ShaderCache` 按真实 shader-def 组合 → naga 全量校验；不需要
//! GPU 设备）。

use bevy::{
    app::App,
    asset::{AssetId, AssetPlugin, Assets, LoadState},
    core_pipeline::CorePipelinePlugin,
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
use bevy_stylized_render::{
    CEL_PORTABLE_FORWARD_SHADER_PATH, CHARACTER_PORTABLE_FORWARD_SHADER_PATH,
    OUTLINE_PORTABLE_SHADER_PATH, StylizedRenderPlugin, hero::HERO_FORWARD_SHADER_PATH,
    post::STYLIZED_FULLSCREEN_SHADER_PATH, screen_outline::SCREEN_OUTLINE_SHADER_PATH,
};

/// 纯 WGSL import 模块（无 pipeline 引用）。
const NPR_COMMON_SHADER_PATH: &str =
    "embedded://bevy_stylized_render/shaders/common/npr_common.wgsl";

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
        StylizedRenderPlugin,
    ));
    // fullscreen 顶点模块按需加载（FullscreenMaterial 的 import 目标）。
    app.world_mut()
        .init_resource::<bevy::core_pipeline::FullscreenShader>();
    app.finish();
    app.cleanup();
    app
}

/// Loads an embedded crate shader and drives the app until it is available.
fn load_crate_shader(app: &mut App, path: &'static str) -> AssetId<Shader> {
    let handle: Handle<Shader> = app.world().resource::<AssetServer>().load(path);
    let id = handle.id();
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

struct Case {
    path: &'static str,
    defs: Vec<&'static str>,
}

fn cases() -> Vec<Case> {
    const BASE: &[&str] = &["VERTEX_POSITIONS", "VERTEX_NORMALS", "VERTEX_UVS_A"];
    const SKINNED: &[&str] = &[
        "VERTEX_POSITIONS",
        "VERTEX_NORMALS",
        "VERTEX_UVS_A",
        "SKINNED",
    ];
    const MINIMAL: &[&str] = &["VERTEX_POSITIONS"];
    const SKINNED_MORPHED: &[&str] = &[
        "VERTEX_POSITIONS",
        "VERTEX_NORMALS",
        "VERTEX_UVS_A",
        "SKINNED",
        "MORPH_TARGETS",
    ];
    const MORPHED: &[&str] = &[
        "VERTEX_POSITIONS",
        "VERTEX_NORMALS",
        "VERTEX_UVS_A",
        "MORPH_TARGETS",
    ];
    let mut cases = Vec::new();
    for path in [
        CHARACTER_PORTABLE_FORWARD_SHADER_PATH,
        CEL_PORTABLE_FORWARD_SHADER_PATH,
        OUTLINE_PORTABLE_SHADER_PATH,
        HERO_FORWARD_SHADER_PATH,
    ] {
        for defs in [BASE, SKINNED, MINIMAL, MORPHED, SKINNED_MORPHED] {
            cases.push(Case {
                path,
                defs: defs.to_vec(),
            });
        }
    }
    // 环境 NPR（V2-d）：DISTANCE_FOG def 打开时 fog uniform binding 才存在
    // （view 层 DistanceFog 组件驱动），单独覆盖该组合。
    const FOGGED: &[&str] = &[
        "VERTEX_POSITIONS",
        "VERTEX_NORMALS",
        "VERTEX_UVS_A",
        "DISTANCE_FOG",
    ];
    for path in [
        CHARACTER_PORTABLE_FORWARD_SHADER_PATH,
        CEL_PORTABLE_FORWARD_SHADER_PATH,
        HERO_FORWARD_SHADER_PATH,
    ] {
        cases.push(Case {
            path,
            defs: FOGGED.to_vec(),
        });
    }
    // GPU Instance Database（V2-e）：StorageTable 变体（binding 7 =
    // instance_table storage buffer，由 StylizedCharacterTableMaterial 的
    // specialize 注入 def）。
    cases.push(Case {
        path: CHARACTER_PORTABLE_FORWARD_SHADER_PATH,
        defs: vec![
            "VERTEX_POSITIONS",
            "VERTEX_NORMALS",
            "VERTEX_UVS_A",
            "INSTANCE_STORAGE_TABLE",
        ],
    });
    // 全屏后处理 shader：无 mesh def（guard 在 compose 循环里）。
    for path in FULLSCREEN_CASES {
        cases.push(Case { path, defs: vec![] });
    }
    // screen-space 描边 overlay：mesh material + prepass defs。
    cases.push(Case {
        path: SCREEN_OUTLINE_SHADER_PATH,
        defs: vec!["VERTEX_POSITIONS", "DEPTH_PREPASS", "NORMAL_PREPASS"],
    });
    for class in [
        "NPR_PREPASS_CEL",
        "NPR_PREPASS_CHARACTER",
        "NPR_PREPASS_HERO",
    ] {
        for normal in [false, true] {
            for host in [false, true] {
                let mut defs = vec![
                    "VERTEX_POSITIONS",
                    "VERTEX_NORMALS",
                    "VERTEX_UVS_A",
                    "VERTEX_UVS_B",
                    "DEPTH_PREPASS",
                    "MAY_DISCARD",
                    class,
                ];
                if normal {
                    defs.extend([
                        "PREPASS_FRAGMENT",
                        "NORMAL_PREPASS",
                        "NORMAL_PREPASS_OR_DEFERRED_PREPASS",
                    ]);
                }
                if host {
                    defs.push("NPR_HOST_INPUTS");
                }
                cases.push(Case {
                    path: bevy_stylized_render::skinning::COMPLETE_SKIN_PREPASS_SHADER,
                    defs,
                });
            }
        }
    }
    cases
}

/// 全屏后处理 shader 不是 mesh material（无 MESH_PIPELINE /
/// MATERIAL_BIND_GROUP）。
const FULLSCREEN_CASES: &[&str] = &[STYLIZED_FULLSCREEN_SHADER_PATH];

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
    // Given the render-stack plugins so every shader library is registered.
    let mut app = shader_app();

    let mut shader_ids = Vec::new();
    let mut seen = Vec::new();
    for case in cases() {
        if seen.contains(&case.path) {
            continue;
        }
        seen.push(case.path);
        shader_ids.push(load_crate_shader(&mut app, case.path));
    }

    // 与 legacy 测试一致：把 Assets 中全部 shader（含 bevy_pbr 内部模块与
    // 主动加载的 npr_common）放入 cache，import 图才能完整解析。
    let common = load_crate_shader(&mut app, NPR_COMMON_SHADER_PATH);
    shader_ids.push(common);
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

    for (pipeline, case) in cases().into_iter().enumerate() {
        let shader = shader_ids[seen.iter().position(|p| *p == case.path).unwrap()];
        let mut defs: Vec<ShaderDefVal> = case
            .defs
            .iter()
            .map(|def| ShaderDefVal::from(*def))
            .collect();
        if !FULLSCREEN_CASES.contains(&case.path) {
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
            panic!(
                "shader {} with defs {:?} failed: {error}",
                case.path, case.defs
            );
        }
    }
}
