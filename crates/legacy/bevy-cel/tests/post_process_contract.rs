//! Stylized post-processing public contracts.

use bevy::{
    app::App,
    asset::{AssetPlugin, Assets},
    camera::Hdr,
    core_pipeline::{
        prepass::{DepthPrepass, NormalPrepass},
        tonemapping::Tonemapping,
    },
    post_process::bloom::Bloom,
    prelude::{Camera3d, MinimalPlugins},
};
use bevy_cel::{
    CharacterMaterialPlugin, GenshinDisplayTransform, InternalEdgeAccentSettings,
    InternalEdgeMaterial, PostProcessStage, STYLIZED_POST_PROCESS_ORDER, StylizedFullscreenEffect,
    StylizedPostProcessProfile, StylizedPostProcessSettings,
};

#[test]
fn malformed_profile_values_are_finite_and_bounded() {
    // Given
    let malformed = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1000.0, 1000.0];

    // When
    let settings = malformed.map(|value| {
        StylizedPostProcessSettings::default()
            .with_bloom_intensity(value)
            .with_vignette(value, value, value)
            .with_lens_distortion(value)
            .with_chromatic_aberration(value)
    });

    // Then
    assert!(
        settings
            .into_iter()
            .all(StylizedPostProcessSettings::is_finite)
    );
    assert!(
        settings
            .into_iter()
            .all(StylizedPostProcessSettings::is_bounded)
    );
}

#[test]
fn effects_have_independent_profile_toggles() {
    // Given
    let profile = StylizedPostProcessProfile::default();

    // When
    let profile = profile
        .with_hdr(false)
        .with_bloom(false)
        .with_tonemapping(false)
        .with_vignette(false)
        .with_lens_distortion(false)
        .with_chromatic_aberration(false)
        .with_prepass_edges(false);

    // Then
    assert!(!profile.hdr_enabled());
    assert!(!profile.bloom_enabled());
    assert!(!profile.tonemapping_enabled());
    assert!(!profile.vignette_enabled());
    assert!(!profile.lens_distortion_enabled());
    assert!(!profile.chromatic_aberration_enabled());
    assert!(!profile.prepass_edges_enabled());
}

#[test]
fn fullscreen_order_is_main_bloom_stylized_then_tonemapping() {
    // Given
    let expected = [
        PostProcessStage::MainPass,
        PostProcessStage::Bloom,
        PostProcessStage::StylizedFullscreen,
        PostProcessStage::Tonemapping,
    ];

    // When
    let actual = STYLIZED_POST_PROCESS_ORDER;

    // Then
    assert_eq!(actual, expected);
}

#[test]
fn enabled_profile_installs_camera_stack_and_optional_prepasses() {
    // Given
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        CharacterMaterialPlugin,
    ));
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            StylizedPostProcessProfile::default().with_prepass_edges(true),
        ))
        .id();

    // When
    app.update();

    // Then
    let world = app.world();
    assert!(world.get::<Hdr>(camera).is_some());
    assert!(world.get::<Bloom>(camera).is_some());
    assert_eq!(
        world.get::<Tonemapping>(camera),
        Some(&Tonemapping::TonyMcMapface)
    );
    assert!(world.get::<StylizedFullscreenEffect>(camera).is_some());
    assert!(world.get::<DepthPrepass>(camera).is_some());
    assert!(world.get::<NormalPrepass>(camera).is_some());
}

#[test]
fn disabled_profile_removes_each_optional_effect() {
    // Given
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        CharacterMaterialPlugin,
    ));
    let profile = StylizedPostProcessProfile::default()
        .with_hdr(false)
        .with_bloom(false)
        .with_tonemapping(false)
        .with_vignette(false)
        .with_lens_distortion(false)
        .with_chromatic_aberration(false)
        .with_prepass_edges(false);
    let camera = app.world_mut().spawn((Camera3d::default(), profile)).id();

    // When
    app.update();

    // Then
    let world = app.world();
    assert!(world.get::<Hdr>(camera).is_none());
    assert!(world.get::<Bloom>(camera).is_none());
    assert_eq!(world.get::<Tonemapping>(camera), Some(&Tonemapping::None));
    assert!(world.get::<StylizedFullscreenEffect>(camera).is_none());
    assert!(world.get::<DepthPrepass>(camera).is_none());
    assert!(world.get::<NormalPrepass>(camera).is_none());
}

fn post_process_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        // Render apps always carry SyncWorldPlugin; without it, removing a
        // synced component (e.g. StylizedFullscreenEffect) panics on the
        // missing PendingSyncEntity resource.
        bevy::render::sync_world::SyncWorldPlugin,
        CharacterMaterialPlugin,
    ));
    app
}

fn fully_disabled_profile() -> StylizedPostProcessProfile {
    StylizedPostProcessProfile::default()
        .with_hdr(false)
        .with_bloom(false)
        .with_tonemapping(false)
        .with_vignette(false)
        .with_lens_distortion(false)
        .with_chromatic_aberration(false)
        .with_prepass_edges(false)
}

#[test]
fn display_transform_keeps_hdr_when_the_profile_disables_it() {
    // Given a managed camera whose profile disables HDR but runs the display transform
    let mut app = post_process_app();
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            fully_disabled_profile(),
            GenshinDisplayTransform,
        ))
        .id();

    // When
    app.update();

    // Then the union of requirements keeps the HDR intermediate
    let world = app.world();
    assert!(world.get::<Hdr>(camera).is_some());
    assert!(world.get::<Bloom>(camera).is_none());
    assert_eq!(world.get::<Tonemapping>(camera), Some(&Tonemapping::None));

    // When the display transform is removed, the profile alone owns the camera
    app.world_mut()
        .entity_mut(camera)
        .remove::<GenshinDisplayTransform>();
    app.update();

    // Then
    assert!(app.world().get::<Hdr>(camera).is_none());
}

#[test]
fn transform_only_cameras_are_managed_without_a_profile() {
    // Given
    let mut app = post_process_app();
    let camera = app
        .world_mut()
        .spawn((Camera3d::default(), GenshinDisplayTransform))
        .id();

    // When
    app.update();

    // Then
    let world = app.world();
    assert!(world.get::<Hdr>(camera).is_some());
    // No profile, so tonemapping is left untouched.
    assert!(world.get::<Tonemapping>(camera).is_none());
}

#[test]
fn internal_edge_prepass_needs_survive_profile_changes() {
    // Given a profiled camera and an enabled internal-edge material.
    // The handle must stay alive or Bevy frees the unused asset on update.
    let mut app = post_process_app();
    let _material = app
        .world_mut()
        .resource_mut::<Assets<InternalEdgeMaterial>>()
        .add(InternalEdgeMaterial {
            settings: InternalEdgeAccentSettings::default().with_enabled(true),
        });
    let camera = app
        .world_mut()
        .spawn((Camera3d::default(), fully_disabled_profile()))
        .id();

    // When
    app.update();

    // Then the union keeps both prepasses despite the profile disabling them
    let world = app.world();
    assert!(world.get::<DepthPrepass>(camera).is_some());
    assert!(world.get::<NormalPrepass>(camera).is_some());

    // When the profile changes, the prepass requirement still survives
    app.world_mut()
        .entity_mut(camera)
        .insert(fully_disabled_profile().with_bloom(true));
    app.update();

    // Then
    let world = app.world();
    assert!(world.get::<DepthPrepass>(camera).is_some());
    assert!(world.get::<NormalPrepass>(camera).is_some());
    assert!(world.get::<Bloom>(camera).is_some());
}

#[test]
fn managed_camera_overrides_manually_inserted_prepasses() {
    // Given a managed camera with everything disabled and a manual prepass
    let mut app = post_process_app();
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            fully_disabled_profile(),
            DepthPrepass,
            NormalPrepass,
        ))
        .id();

    // When
    app.update();

    // Then the single writer removes the manually inserted prepasses
    let world = app.world();
    assert!(world.get::<DepthPrepass>(camera).is_none());
    assert!(world.get::<NormalPrepass>(camera).is_none());
}
