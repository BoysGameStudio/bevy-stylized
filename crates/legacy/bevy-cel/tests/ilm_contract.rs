//! ILM and material-region architecture contract tests.

use bevy::{
    asset::RenderAssetUsages,
    image::Image,
    prelude::Handle,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_cel::{
    ATTRIBUTE_CEL_MATERIAL_ID, CHARACTER_MATERIAL_ID_SHADER_LOCATION, CharacterExtension,
    ILM_SAMPLER_BINDING, ILM_TEXTURE_BINDING, ILM_UNIFORM_BINDING, IlmDebugMode, IlmLayout,
    IlmMaterialIdSource, IlmSettings, RAMP_SAMPLER_BINDING, RAMP_TEXTURE_BINDING,
    TOON_RAMP_UNIFORM_BINDING,
};

#[test]
fn ilm_channels_decode_when_values_are_valid() {
    // Given
    let settings = IlmSettings::default().with_fallback_channels([0.25, 0.5, 0.75, 1.0]);

    // When
    let channels = settings.fallback_channels();

    // Then
    assert!(
        channels
            .into_iter()
            .zip([0.25, 0.5, 0.75, 1.0])
            .all(|(actual, expected)| (actual - expected).abs() < f32::EPSILON)
    );
}

#[test]
fn ilm_settings_use_safe_values_when_inputs_are_malformed() {
    // Given
    let settings = IlmSettings::default()
        .with_fallback_channels([f32::NAN, f32::INFINITY, -1.0, 2.0])
        .with_texture_dimensions(0, u32::MAX);

    // When
    let channels = settings.fallback_channels();

    // Then
    assert!(
        channels
            .into_iter()
            .zip([0.0, 1.0, 0.0, 0.0])
            .all(|(actual, expected)| (actual - expected).abs() < f32::EPSILON)
    );
    assert_eq!(settings.texture_dimensions(), None);
}

#[test]
fn absent_ilm_texture_keeps_neutral_channels_and_vertex_id_fallback() {
    // Given
    let extension = CharacterExtension::default();

    // When
    let settings = extension.ilm_settings();

    // Then
    assert!(extension.ilm_texture().is_none());
    assert!(!settings.uses_texture());
    assert!(
        settings
            .fallback_channels()
            .into_iter()
            .zip([0.0, 1.0, 0.0, 0.0])
            .all(|(actual, expected)| (actual - expected).abs() < f32::EPSILON)
    );
    assert_eq!(
        settings.material_id_source(),
        IlmMaterialIdSource::TextureThenVertex
    );
}

#[test]
fn ilm_texture_activation_requires_valid_dimensions() {
    // Given
    let texture = Handle::default();
    let linear = Image::new_fill(
        Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 255, 0, 0],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    let srgb = Image::new_fill(
        Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 255, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    // When
    let malformed = CharacterExtension::default().with_ilm_texture(texture.clone(), &srgb);
    let valid = CharacterExtension::default().with_ilm_texture(texture, &linear);
    let reordered = valid
        .clone()
        .with_ilm_settings(IlmSettings::default().with_debug(IlmDebugMode::SpecularStrength));

    // Then
    assert!(!malformed.ilm_settings().uses_texture());
    assert!(malformed.ilm_texture().is_none());
    assert!(valid.ilm_settings().uses_texture());
    assert_eq!(valid.ilm_settings().texture_dimensions(), Some((8, 8)));
    assert!(reordered.ilm_settings().uses_texture());
    assert_eq!(reordered.ilm_settings().texture_dimensions(), Some((8, 8)));
}

#[test]
fn material_id_source_and_debug_modes_are_exhaustive_typed_seams() {
    // Given
    let settings = IlmSettings::default()
        .with_material_id_source(IlmMaterialIdSource::VertexThenTexture)
        .with_debug(IlmDebugMode::RampRow);

    // When
    let state = (settings.material_id_source(), settings.debug_mode());

    // Then
    assert_eq!(
        state,
        (
            IlmMaterialIdSource::VertexThenTexture,
            IlmDebugMode::RampRow
        )
    );
}

#[test]
fn genshin_layout_is_explicit_and_canonical_remains_the_default() {
    assert_eq!(IlmSettings::default().layout(), IlmLayout::Canonical);
    assert_eq!(
        IlmSettings::default()
            .with_layout(IlmLayout::GenshinCharacter)
            .layout(),
        IlmLayout::GenshinCharacter
    );

    let source = include_str!("../src/shaders/character_forward.wgsl");
    assert!(source.contains("ILM_LAYOUT_GENSHIN_CHARACTER"));
    for threshold in ["alpha > 0.2", "alpha > 0.4", "alpha > 0.6", "alpha > 0.8"] {
        assert!(source.contains(threshold));
    }
    assert!(source.contains("enabled_mask & (1u << (source_id - 1u))"));
    assert!(source.contains("fn final_ao_multiplier"));
    assert!(source.contains("ilm_settings.state.w == ILM_LAYOUT_GENSHIN_CHARACTER"));
    assert!(source.contains("let physical_raw_proxy = raw_toon_light_proxy("));
    assert!(source.contains("let raw_proxy = resolved_genshin_light_energy("));
    assert!(source.contains("let calibrated_proxy = select("));
    assert!(source.contains("fn genshin_source_lit("));
    assert!(source.contains("let ao_gate = smoothstep(0.01, 0.4, ao)"));
    assert!(source.contains("let use_ramp = source_lit < shadow_position"));
    assert!(source.contains("let genshin_band = select(vec3<f32>(1.0), band, use_ramp)"));
    assert!(source.contains("fn genshin_light_tint("));
    assert!(source.contains("lit_color / reference"));
    assert!(source.contains("base_color * max(raw_light_proxy, 0.05)"));
    assert!(source.contains("fn genshin_hero_energy("));
    assert!(source.contains("return mix(1.0, physical_energy"));
    assert!(source.contains("physical_light_influence"));
    assert!(source.contains("* hero_energy\n                        * ao_multiplier"));
    assert!(source.contains(
        "* band\n                        * hero_light_tint\n                        * hero_energy"
    ));
}

#[test]
fn out_of_range_material_ids_select_safe_ramp_rows() {
    // Given
    let settings = IlmSettings::default();

    // When
    let rows = [
        settings.validated_ramp_row(0, 0),
        settings.validated_ramp_row(u32::MAX, 0),
        settings.validated_ramp_row(u32::MAX, 2),
        settings.validated_ramp_row(1, 2),
    ];

    // Then
    assert_eq!(rows, [0, 0, 1, 1]);
}

#[test]
fn binding_and_attribute_identities_are_stable() {
    // Given
    let ramp_bindings = [
        TOON_RAMP_UNIFORM_BINDING,
        RAMP_TEXTURE_BINDING,
        RAMP_SAMPLER_BINDING,
    ];
    let ilm_bindings = [
        ILM_UNIFORM_BINDING,
        ILM_TEXTURE_BINDING,
        ILM_SAMPLER_BINDING,
    ];

    // When
    let source = include_str!("../src/shaders/character_forward.wgsl");
    let vertex_source = include_str!("../src/shaders/character_vertex.wgsl");

    // Then
    assert_eq!(ramp_bindings, [100, 101, 102]);
    assert_eq!(ilm_bindings, [103, 104, 105]);
    assert_eq!(ATTRIBUTE_CEL_MATERIAL_ID.name, "_CEL_MATERIAL_ID");
    assert_eq!(CHARACTER_MATERIAL_ID_SHADER_LOCATION, 11);
    assert!(vertex_source.contains("@location(11) material_id: f32"));
    assert!(vertex_source.contains("CHARACTER_MATERIAL_ID_ATTRIBUTE"));
    assert!(
        ["@binding(103)", "@binding(104)", "@binding(105)"]
            .into_iter()
            .all(|token| source.contains(token))
    );
}

#[test]
fn shader_decodes_channels_without_applying_future_specular_or_emission() {
    // Given
    let source = include_str!("../src/shaders/character_forward.wgsl");

    // When
    let required = [
        "ilm.specular_strength",
        "ilm.ao_shadow_influence",
        "ilm.material_id",
        "ilm.emission_special_mask",
        "ILM_DEBUG_SPECULAR",
        "ILM_DEBUG_AO_SHADOW",
        "ILM_DEBUG_MATERIAL_ID",
        "ILM_DEBUG_EMISSION_SPECIAL",
        "ILM_DEBUG_RAMP_ROW",
    ];

    // Then
    assert!(required.into_iter().all(|token| source.contains(token)));
    assert!(!source.contains("apply_ilm_specular"));
    assert!(!source.contains("apply_ilm_emission"));
}
