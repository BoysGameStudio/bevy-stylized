//! Genshin camera display-transform public and shader contracts.

use bevy::{
    app::App,
    asset::AssetPlugin,
    camera::Hdr,
    prelude::{Camera3d, MinimalPlugins},
};
use bevy_cel::{
    CharacterMaterialPlugin, GENSHIN_DISPLAY_TRANSFORM_SHADER_PATH, GenshinDisplayTransform,
    genshin_display_transform_channel,
};

#[test]
fn character_plugin_registers_transform_without_enabling_unmarked_cameras() {
    // Given
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        CharacterMaterialPlugin,
    ));
    let camera = app.world_mut().spawn(Camera3d::default()).id();

    // When
    app.update();

    // Then
    assert!(app.world().get::<GenshinDisplayTransform>(camera).is_none());
    assert!(app.world().get::<Hdr>(camera).is_none());
}

#[test]
fn enabling_transform_requests_an_hdr_intermediate() {
    // Given
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    // When
    let camera = app
        .world_mut()
        .spawn((Camera3d::default(), GenshinDisplayTransform))
        .id();

    // Then
    assert!(app.world().get::<GenshinDisplayTransform>(camera).is_some());
    assert!(app.world().get::<Hdr>(camera).is_some());
}

#[test]
fn cpu_reference_uses_the_fixed_source_curve() {
    // Given
    let at_display_white = 1.407_f32 / 1.63_f32;

    // When
    let mapped = [
        genshin_display_transform_channel(0.0),
        genshin_display_transform_channel(1.0),
        genshin_display_transform_channel(2.0),
    ];

    // Then
    assert_eq!(mapped[0], 0.0);
    assert!((mapped[1] - at_display_white).abs() < f32::EPSILON);
    assert_eq!(mapped[2], 1.0);
}

#[test]
fn shader_uses_only_the_fixed_curve_and_preserves_alpha() {
    // Given
    let source = include_str!("../src/shaders/stylized_post_process.wgsl");

    // Then
    assert_eq!(
        GENSHIN_DISPLAY_TRANSFORM_SHADER_PATH,
        "embedded://bevy_cel/shaders/stylized_post_process.wgsl"
    );
    assert!(source.contains("x * (1.36 * x + vec3<f32>(0.047))"));
    assert!(source.contains("x * (0.93 * x + vec3<f32>(0.56)) + vec3<f32>(0.14)"));
    assert!(source.contains("clamp(mapped, vec3<f32>(0.0), vec3<f32>(1.0))"));
    assert!(source.contains("output_color = genshin_display_transform(output_color)"));
    assert!(source.contains("center_sample.a"));
    assert!(!source.contains("exposure"));
    assert!(!source.contains("gamma"));
}

#[test]
fn render_pass_is_strictly_between_bloom_and_bevy_tonemapping() {
    // Given
    let source = include_str!("../src/post_process_runtime.rs");
    let plugin = include_str!("../src/plugin.rs");

    // When
    let after_bloom = source.find(".after(bloom)");
    let before_tonemapping = source.find(".before(tonemapping)");

    // Then
    assert!(
        after_bloom
            .is_some_and(|after| { before_tonemapping.is_some_and(|before| after < before) })
    );
    assert_eq!(source.matches("FullscreenMaterialPlugin::<").count(), 1);
    assert!(!plugin.contains("GenshinDisplayTransformPlugin"));
    assert!(source.contains("Bevy 0.19's helper stores a non-generic pipeline-id"));
}
