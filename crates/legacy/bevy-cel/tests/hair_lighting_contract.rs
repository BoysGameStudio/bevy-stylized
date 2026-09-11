//! Hair highlight, toon specular, and rim-lighting contracts.

use bevy::math::Vec3;
use bevy_cel::{
    ATTRIBUTE_CEL_HAIR_MASK, ATTRIBUTE_CEL_HAIR_TANGENT, CEL_LIGHTING_UNIFORM_BINDING,
    CHARACTER_HAIR_MASK_SHADER_LOCATION, CHARACTER_HAIR_TANGENT_SHADER_LOCATION,
    CelLightingDebugMode, CelLightingSettings, CharacterExtension, HairHighlightSettings,
    RimLightingSettings, ToonSpecularSettings,
};

#[test]
fn malformed_effect_parameters_resolve_to_finite_safe_values() {
    // Given
    let settings = CelLightingSettings::default()
        .with_hair(
            HairHighlightSettings::default()
                .with_tint_shift([f32::NAN, -1.0, f32::INFINITY], f32::NAN)
                .with_shape(0.0, 2.0)
                .with_softness_intensity(f32::NAN, -4.0),
        )
        .with_specular(
            ToonSpecularSettings::default()
                .with_tint_intensity([f32::NAN, -1.0, 2.0], f32::INFINITY)
                .with_shape(0.0, -1.0)
                .with_softness(f32::NAN),
        )
        .with_rim(
            RimLightingSettings::default()
                .with_tint_intensity([f32::NAN, -1.0, 2.0], f32::INFINITY)
                .with_shape(0.0, -1.0)
                .with_softness(f32::NAN),
        )
        .with_material_masks(f32::NAN, -1.0, 2.0);

    // When
    let hair = settings.hair();
    let specular = settings.specular();
    let rim = settings.rim();

    // Then
    assert_eq!((hair.tint(), hair.shift()), ([1.0; 3], 0.0));
    assert_eq!(
        (
            hair.power(),
            hair.threshold(),
            hair.softness(),
            hair.intensity()
        ),
        (1.0, 1.0, 0.0, 0.0)
    );
    assert_eq!(
        (specular.tint(), specular.power(), specular.threshold()),
        ([1.0; 3], 1.0, 0.0)
    );
    assert_eq!((specular.softness(), specular.intensity()), (0.0, 0.0));
    assert_eq!(
        (rim.tint(), rim.power(), rim.threshold()),
        ([1.0; 3], 1.0, 0.0)
    );
    assert_eq!((rim.softness(), rim.intensity()), (0.0, 0.0));
    assert!(
        settings
            .material_masks()
            .into_iter()
            .zip([1.0, 0.0, 1.0])
            .all(|(actual, expected)| (actual - expected).abs() < f32::EPSILON)
    );
    assert_eq!(settings.readability_floor(), 0.0);
}

#[test]
fn effects_and_diagnostics_are_independently_selectable() {
    // Given
    let settings = CelLightingSettings::default()
        .with_hair_enabled(true)
        .with_specular_enabled(false)
        .with_rim_enabled(true)
        .with_debug(CelLightingDebugMode::RimResult);

    // When
    let state = (
        settings.hair_enabled(),
        settings.specular_enabled(),
        settings.rim_enabled(),
        settings.debug_mode(),
    );

    // Then
    assert_eq!(state, (true, false, true, CelLightingDebugMode::RimResult));
}

#[test]
fn degenerate_hair_tangent_uses_a_finite_deterministic_fallback() {
    // Given
    let settings = CelLightingSettings::default().with_hair_enabled(true);

    // When
    let first = settings.hair_response(Vec3::ZERO, Vec3::Z, Vec3::Z, Vec3::Y);
    let second = settings.hair_response(Vec3::splat(f32::NAN), Vec3::Z, Vec3::Z, Vec3::Y);

    // Then
    assert!(first.is_finite());
    assert!(second.is_finite());
    assert!((first - second).abs() < f32::EPSILON);
}

#[test]
fn bindings_and_canonical_hair_attributes_are_stable_without_duplicate_ids() {
    // Given
    let identities = [ATTRIBUTE_CEL_HAIR_TANGENT.id, ATTRIBUTE_CEL_HAIR_MASK.id];

    // When
    let source = include_str!("../src/shaders/character_forward.wgsl");
    let vertex_source = include_str!("../src/shaders/character_vertex.wgsl");

    // Then
    assert_eq!(CEL_LIGHTING_UNIFORM_BINDING, 111);
    assert_eq!(CHARACTER_HAIR_TANGENT_SHADER_LOCATION, 12);
    assert_eq!(CHARACTER_HAIR_MASK_SHADER_LOCATION, 13);
    assert_ne!(identities[0], identities[1]);
    assert_eq!(ATTRIBUTE_CEL_HAIR_TANGENT.name, "_CEL_HAIR_TANGENT");
    assert_eq!(ATTRIBUTE_CEL_HAIR_MASK.name, "_CEL_HAIR_MASK");
    assert!(source.contains("@binding(111)"));
    assert!(vertex_source.contains("@location(12) hair_tangent: vec3<f32>"));
    assert!(vertex_source.contains("@location(13) hair_mask: f32"));
}

#[test]
fn shader_uses_guarded_hair_specular_rim_and_explicit_masks() {
    // Given
    let source = include_str!("../src/shaders/character_forward.wgsl");

    // When
    let required = [
        "safe_normalize",
        "hair_highlight",
        "toon_specular",
        "fresnel_rim",
        "ilm.specular_strength",
        "ilm.emission_special_mask",
        "CEL_LIGHTING_DEBUG_HAIR_MASK",
        "CEL_LIGHTING_DEBUG_HAIR_RESULT",
        "CEL_LIGHTING_DEBUG_SPECULAR_MASK",
        "CEL_LIGHTING_DEBUG_SPECULAR_RESULT",
        "CEL_LIGHTING_DEBUG_RIM_MASK",
        "CEL_LIGHTING_DEBUG_RIM_RESULT",
    ];

    // Then
    assert!(required.into_iter().all(|token| source.contains(token)));
    assert!(!source.contains("normalize(in.hair_tangent)"));
    assert!(source.contains("first_directional_light_active"));
    assert!(source.contains(") * hair_mask * proxy"));
    assert!(source.contains("* specular_mask\n                    * proxy"));
    assert!(source.contains("* rim_mask * proxy"));
    assert!(source.contains("apply_readability_bounds"));
    assert!(source.contains("base_color * floor"));
    assert!(source.contains("bounded /= max(peak, 1.0)"));
}

#[test]
fn readability_floor_is_explicit_clamped_and_default_off() {
    assert_eq!(CelLightingSettings::default().readability_floor(), 0.0);
    assert_eq!(
        CelLightingSettings::default()
            .with_readability_floor(0.25)
            .with_material_masks(0.0, 1.0, 1.0)
            .readability_floor(),
        0.25
    );
    assert_eq!(
        CelLightingSettings::default()
            .with_readability_floor(2.0)
            .readability_floor(),
        1.0
    );
    assert_eq!(
        CelLightingSettings::default()
            .with_readability_floor(f32::NAN)
            .readability_floor(),
        0.0
    );
}

#[test]
fn default_extension_disables_all_new_effects_for_exact_regressions() {
    // Given
    let extension = CharacterExtension::default();

    // When
    let settings = extension.cel_lighting_settings();

    // Then
    assert!(!settings.hair_enabled());
    assert!(!settings.specular_enabled());
    assert!(!settings.rim_enabled());
}
