//! Eye readability and HDR emission behavioral contracts.

use bevy::render::render_resource::TextureFormat;
use bevy_cel::{
    ATTRIBUTE_CEL_EMISSION, CHARACTER_EMISSION_SHADER_LOCATION, CharacterExtension,
    EMISSION_SAMPLER_BINDING, EMISSION_TEXTURE_BINDING, EYE_EMISSION_UNIFORM_BINDING,
    EyeEmissionDebugMode, EyeEmissionSettings,
};

#[test]
fn malformed_eye_emission_controls_resolve_to_visible_safe_values() {
    // Given
    let settings = EyeEmissionSettings::default()
        .with_eye_floor(f32::NAN, -1.0)
        .with_eye_class(2.0, f32::INFINITY)
        .with_emission([f32::NAN, -1.0, f32::INFINITY], f32::NAN)
        .with_masks(f32::NAN, 2.0);

    // When
    let values = (
        settings.eye_floor(),
        settings.eye_class(),
        settings.emission(),
        settings.masks(),
    );

    // Then
    assert_eq!(
        values,
        ((0.2, 0.0), (1.0, 1.0), ([1.0, 0.0, 1.0], 0.0), (0.0, 1.0))
    );
}

#[test]
fn eye_emission_states_and_debug_are_independent() {
    // Given
    let settings = EyeEmissionSettings::default()
        .with_eye_enabled(true)
        .with_emission_enabled(true)
        .with_debug(EyeEmissionDebugMode::OutlineSuppression);

    // When
    let state = (
        settings.eye_enabled(),
        settings.emission_enabled(),
        settings.debug_mode(),
    );

    // Then
    assert_eq!(
        state,
        (true, true, EyeEmissionDebugMode::OutlineSuppression)
    );
}

#[test]
fn finite_hdr_intensity_is_preserved_above_display_white() {
    // Given
    let settings = EyeEmissionSettings::default().with_emission([0.2, 0.5, 1.0], 12.0);

    // When
    let emission = settings.emission();

    // Then
    assert_eq!(emission, ([0.2, 0.5, 1.0], 12.0));
}

#[test]
fn task18_bindings_and_canonical_attribute_identity_are_stable() {
    // Given
    let forward = include_str!("../src/shaders/character_forward.wgsl");
    let vertex = include_str!("../src/shaders/character_vertex.wgsl");

    // When
    let bindings = [
        EYE_EMISSION_UNIFORM_BINDING,
        EMISSION_TEXTURE_BINDING,
        EMISSION_SAMPLER_BINDING,
    ];

    // Then
    assert_eq!(bindings, [112, 113, 114]);
    assert_eq!(CHARACTER_EMISSION_SHADER_LOCATION, 14);
    assert_eq!(ATTRIBUTE_CEL_EMISSION.name, "_CEL_EMISSION");
    assert_eq!(
        ATTRIBUTE_CEL_EMISSION.id,
        bevy_gltf_metadata::ATTRIBUTE_CEL_EMISSION.id
    );
    assert!(forward.contains("@binding(112)"));
    assert!(forward.contains("@binding(113)"));
    assert!(forward.contains("@binding(114)"));
    assert!(vertex.contains("@location(14) emission_mask: f32"));
}

#[test]
fn shader_combines_masks_and_adds_unclamped_hdr_before_post_processing() {
    // Given
    let source = include_str!("../src/shaders/character_forward.wgsl");

    // When
    let emission_add = source.find("out.color.rgb + emission_color");
    let post = source.find("main_pass_post_lighting_processing(pbr_input, out.color)");

    // Then
    assert!(source.contains("resolved_emission_mask"));
    assert!(source.contains("ilm.emission_special_mask"));
    assert!(source.contains("in.emission_mask"));
    assert!(emission_add.is_some_and(|add| post.is_some_and(|processing| add < processing)));
    assert!(!source.contains("clamp(emission_color"));
}

#[test]
fn missing_or_srgb_emission_texture_keeps_zero_fallback_without_binding_failure() {
    // Given
    let extension = CharacterExtension::default();

    // When
    let settings = extension.eye_emission_settings();

    // Then
    assert!(extension.emission_texture().is_none());
    assert!(!settings.uses_texture());
    assert!(settings.masks().0.abs() < f32::EPSILON);
    assert!(!EyeEmissionSettings::supports_texture_format(
        TextureFormat::Rgba8UnormSrgb
    ));
    assert!(EyeEmissionSettings::supports_texture_format(
        TextureFormat::Rgba8Unorm
    ));
}
