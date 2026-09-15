//! App, calibrated targets and static scene construction for reference capture.
use super::{calibration::calibrate, diagnostics::DiagnosticMode, input::Package, io::read};
use bevy::{
    asset::AssetMetaCheck,
    core_pipeline::tonemapping::Tonemapping,
    gltf::GltfAssetLabel,
    prelude::*,
    render::{RenderPlugin, settings::RenderCreation},
    window::ExitCondition,
    world_serialization::WorldAssetRoot,
};
use bevy_stylized::{StylizedAssetIntegrationPlugin, StylizedMaterialSet, prelude::*};
use golden_harness::capture;
use serde_json::Value;
use std::path::Path;

pub(super) struct Scene {
    pub(super) app: App,
    pub(super) target: capture::CaptureTarget,
    pub(super) linear_target: Option<capture::CaptureTarget>,
    pub(super) camera: Entity,
    pub(super) sidecar: bevy_stylized::stylized_gltf::StylizedSidecar,
    pub(super) view: StylizedViewDescriptor,
    pub(super) scene: Handle<bevy::world_serialization::WorldAsset>,
    pub(super) expected: usize,
    pub(super) textures: Vec<String>,
    pub(super) calibration: Value,
}
pub(super) fn setup(root: &Path, out: &Path, p: &Package) -> Result<Scene, String> {
    let sidecar =
        bevy_stylized::stylized_gltf::load_sidecar(root.join("runtime/character.npr.json"))
            .map_err(|e| e.to_string())?;
    let view: StylizedViewDescriptor =
        serde_json::from_slice(&read(root.join("runtime/character.view.json"))?)
            .map_err(|e| e.to_string())?;
    let resources = std::panic::catch_unwind(capture::probe_render_resources)
        .map_err(|_| "required GPU unavailable")?
        .ok_or("required GPU unavailable")?;
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .build()
            .set(bevy_stylized::stylized_gltf_plugin())
            .set(AssetPlugin {
                file_path: root.join("runtime").to_string_lossy().into_owned(),
                meta_check: AssetMetaCheck::Never,
                ..default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Manual(resources),
                synchronous_pipeline_compilation: true,
                ..default()
            })
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            }),
    );
    app.add_plugins((StylizedPlugin::default(), StylizedAssetIntegrationPlugin));
    capture::install(&mut app);
    app.insert_resource(ClearColor(Color::linear_rgba(
        p.background_linear[0],
        p.background_linear[1],
        p.background_linear[2],
        p.background_linear[3],
    )));
    let target = capture::create_capture_target(
        &mut app.world_mut().resource_mut::<Assets<Image>>(),
        512,
        512,
    );
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            capture::capture_target_bundle(&target),
            Tonemapping::None,
            Msaa::Sample4,
            Transform::default(),
        ))
        .id();
    let diagnostic_mode = if p.diagnostics["policy"] == "beauty-center/v1" {
        StylizedHeroDebugView::Beauty
    } else if p.diagnostics["policy"] == "ndoth/v1" {
        StylizedHeroDebugView::NormalHalfDot
    } else if p.diagnostics["policy"] == "pre-palette/v1" {
        StylizedHeroDebugView::PrePalette
    } else if p.diagnostics["policy"] == "base-color/v1" {
        StylizedHeroDebugView::BaseColor
    } else {
        StylizedHeroDebugView::LightingFactors
    };
    app.insert_resource(DiagnosticMode(diagnostic_mode));
    app.finish();
    app.cleanup();
    if p.supersample.is_some_and(|s| ![2, 4, 8].contains(&s)) {
        return Err("unsupported matched sampling grid".into());
    }
    let linear_target = p.supersample.map(|scale| {
        capture::create_linear_capture_target(
            &mut app.world_mut().resource_mut::<Assets<Image>>(),
            512 * scale,
            512 * scale,
        )
    });
    let calibration = calibrate(&mut app, root, out, &target, camera)?;
    let scene: Handle<bevy::world_serialization::WorldAsset> = app
        .world()
        .resource::<AssetServer>()
        .load(GltfAssetLabel::Scene(0).from_asset("character.glb"));
    app.world_mut().spawn((
        WorldAssetRoot(scene.clone()),
        StylizedMaterialSet {
            materials: sidecar.materials.clone(),
            texture_prefix: String::new(),
        },
        Transform::default(),
    ));
    let glb = read(root.join("runtime/character.glb"))?;
    let len = u32::from_le_bytes(glb[12..16].try_into().map_err(|_| "GLB header")?) as usize;
    let metadata: Value = serde_json::from_slice(glb.get(20..20 + len).ok_or("GLB bounds")?)
        .map_err(|e| e.to_string())?;
    let expected = metadata["meshes"]
        .as_array()
        .ok_or("GLB meshes")?
        .iter()
        .map(|m| m["primitives"].as_array().map_or(0, Vec::len))
        .sum::<usize>();
    if expected == 0 {
        return Err("empty character".into());
    }
    let textures: Vec<_> = sidecar
        .materials
        .values()
        .flat_map(|m| m.textures.iter().map(|t| t.uri.clone()))
        .collect();
    Ok(Scene {
        app,
        target,
        linear_target,
        camera,
        sidecar,
        view,
        scene,
        expected,
        textures,
        calibration,
    })
}
