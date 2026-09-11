//! GPU-gated integration test: builds the real render pipelines for every
//! bevy_cel material on an actual headless wgpu device (no window).
//!
//! This exercises the full path — shader module creation, bind group layout
//! compatibility, and render pipeline creation — that caught the
//! internal-edge uniform ABI mismatch. Without a GPU adapter the test skips
//! gracefully instead of failing.

use bevy::{
    DefaultPlugins,
    app::App,
    asset::{Assets, RenderAssetUsages},
    camera::RenderTarget,
    core_pipeline::fullscreen_material::FullscreenMaterialPipelineId,
    image::{Image, ImageSampler},
    math::{Vec3, primitives::Triangle3d},
    pbr::{MeshMaterial3d, PreparedMaterial, RenderMaterialInstances},
    prelude::*,
    render::{
        RenderApp, RenderPlugin,
        erased_render_asset::ErasedRenderAssets,
        render_resource::{
            CachedPipelineState, PipelineCache, TextureDimension, TextureFormat, TextureUsages,
        },
        renderer::initialize_renderer,
        settings::{RenderCreation, RenderResources, WgpuSettings},
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_cel::{
    CharacterMaterial, CharacterOutline, InternalEdgeAccentSettings, InternalEdgeMaterial,
    OutlineMaterial, StylizedPostProcessProfile, character_material,
};

/// Attempts to create a real render device without a window.
///
/// Returns `None` when no adapter is available (e.g. GPU-less CI). The adapter
/// probe runs on a separate thread because bevy's `initialize_renderer`
/// panics when no adapter is found; the thread boundary turns that panic into
/// a graceful skip.
fn try_create_render_resources() -> Option<RenderResources> {
    std::thread::spawn(|| {
        let settings = WgpuSettings::default();
        let backends = settings.backends.unwrap_or_default();
        bevy::tasks::block_on(initialize_renderer(backends, None, &settings))
    })
    .join()
    .ok()
}

fn pipeline_test_app(render_creation: RenderCreation) -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                render_creation,
                synchronous_pipeline_compilation: true,
                ..Default::default()
            })
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..Default::default()
            }),
    );
    app.add_plugins(bevy_cel::CharacterMaterialPlugin);
    // `App::update` does not finish plugins; RenderPlugin::finish is what
    // unpacks the render device into the main and render worlds.
    app.finish();
    app.cleanup();
    app
}

fn spawn_pipeline_scene(app: &mut App) {
    let mut target = Image::new_fill(
        bevy::render::render_resource::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    target.texture_descriptor.usage =
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC;
    target.sampler = ImageSampler::nearest();
    let target = app.world_mut().resource_mut::<Assets<Image>>().add(target);

    app.world_mut().spawn((
        Camera3d::default(),
        Camera::default(),
        RenderTarget::Image(target.into()),
        Transform::from_xyz(0.0, 0.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        // TonyMcMapface needs the `tonemapping_luts` feature, which this crate
        // does not enable; the fullscreen pass is what this test exercises.
        StylizedPostProcessProfile::default()
            .with_prepass_edges(true)
            .with_tonemapping(false),
        bevy_cel::GenshinDisplayTransform,
    ));

    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Triangle3d::default());
    let character = app
        .world_mut()
        .resource_mut::<Assets<CharacterMaterial>>()
        .add(character_material(StandardMaterial::default()));
    let hull = app
        .world_mut()
        .resource_mut::<Assets<OutlineMaterial>>()
        .add(OutlineMaterial::default());
    let edge = app
        .world_mut()
        .resource_mut::<Assets<InternalEdgeMaterial>>()
        .add(InternalEdgeMaterial {
            settings: InternalEdgeAccentSettings::default().with_enabled(true),
        });
    app.world_mut().spawn((
        Mesh3d(mesh),
        MeshMaterial3d(character),
        CharacterOutline::new(hull).with_internal_edge(edge),
    ));
}

/// Proves that every bevy_cel material produced a render-world material
/// instance (bind group created against the pipeline layout) and that every
/// pipeline queued during scene rendering was created without error.
fn assert_pipelines_built(app: &mut App, context: &str) {
    for _ in 0..10 {
        app.update();
    }

    let render_app = app
        .get_sub_app_mut(RenderApp)
        .expect("render sub-app must exist with a device");
    let world = render_app.world_mut();

    // The character source plus its hull and internal-edge replicas must all
    // reach the render world, and their prepared materials (bind groups
    // created against the pipeline layouts) must all exist.
    let instances = &world.resource::<RenderMaterialInstances>().instances;
    assert_eq!(
        instances.len(),
        3,
        "{context}: character, hull, and internal-edge instances must be extracted"
    );
    let prepared = world
        .resource::<ErasedRenderAssets<PreparedMaterial>>()
        .iter()
        .count();
    assert!(
        prepared >= 3,
        "{context}: expected at least 3 prepared materials, got {prepared}"
    );

    let (ok, pending, errors) = {
        let pipeline_cache = world.resource::<PipelineCache>();
        let mut ok = 0_usize;
        let mut pending = 0_usize;
        let mut errors = Vec::new();
        for pipeline in pipeline_cache.pipelines() {
            match &pipeline.state {
                CachedPipelineState::Ok(_) => ok += 1,
                CachedPipelineState::Err(error) => errors.push(error.to_string()),
                CachedPipelineState::Queued | CachedPipelineState::Creating(_) => pending += 1,
            }
        }
        (ok, pending, errors)
    };
    assert!(
        errors.is_empty(),
        "{context}: render pipelines failed to build: {errors:?}"
    );
    assert!(
        ok >= 4 && pending == 0,
        "{context}: expected several completed pipelines, got {ok} ok / {pending} pending"
    );

    // The stylized fullscreen pass must have a completed pipeline on the view.
    let mut views = world.query::<&FullscreenMaterialPipelineId>();
    let fullscreen_ids: Vec<_> = views.iter(world).map(|id| id.0).collect();
    assert_eq!(fullscreen_ids.len(), 1, "{context}: fullscreen pipeline");
    let pipeline_cache = world.resource::<PipelineCache>();
    assert!(
        matches!(
            pipeline_cache.get_render_pipeline_state(fullscreen_ids[0]),
            CachedPipelineState::Ok(_)
        ),
        "{context}: fullscreen pipeline must be created"
    );
}

/// Requires a real GPU adapter; skips gracefully when none is available.
#[test]
fn material_pipelines_build_on_a_real_gpu_device() {
    let Some(resources) = try_create_render_resources() else {
        eprintln!(
            "skipping material_pipelines_build_on_a_real_gpu_device: no GPU adapter available"
        );
        return;
    };

    let mut app = pipeline_test_app(RenderCreation::Manual(resources));
    spawn_pipeline_scene(&mut app);
    assert_pipelines_built(&mut app, "gpu");
}
