//! Forward-material architecture contract tests.

use bevy::{
    app::App,
    asset::{AssetPlugin, Assets},
    material::{AlphaMode, OpaqueRendererMethod},
    pbr::MaterialExtension,
    prelude::{Color, MinimalPlugins, StandardMaterial},
    render::render_resource::ShaderType,
    shader::ShaderRef,
};
use bevy_cel::{
    ATTRIBUTE_CHARACTER_MASK, BACKFACE_BASE_COLOR_UNIFORM_BINDING, CHARACTER_FORWARD_SHADER_PATH,
    CharacterExtension, CharacterMaterial, CharacterMaterialPlugin, OutlineCullMode,
    RAMP_SAMPLER_BINDING, RAMP_TEXTURE_BINDING, TOON_RAMP_UNIFORM_BINDING, ToonRampDebugMode,
    ToonRampSettings, character_material,
};

#[test]
fn material_forces_forward_when_deferred_is_requested() {
    // Given
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        CharacterMaterialPlugin,
    ));
    let material = CharacterMaterial {
        base: StandardMaterial {
            opaque_render_method: OpaqueRendererMethod::Deferred,
            ..Default::default()
        },
        extension: CharacterExtension::default(),
    };

    // When
    let handle = app
        .world_mut()
        .resource_mut::<Assets<CharacterMaterial>>()
        .add(material);
    app.update();

    // Then
    let materials = app.world().resource::<Assets<CharacterMaterial>>();
    assert_eq!(
        materials
            .get(&handle)
            .map(|material| material.base.opaque_render_method),
        Some(OpaqueRendererMethod::Forward)
    );
}

#[test]
fn constructor_preserves_alpha_mask_behavior() {
    // Given
    let base = StandardMaterial {
        base_color: Color::srgba(0.8, 0.4, 0.2, 0.25),
        alpha_mode: AlphaMode::Mask(0.5),
        ..Default::default()
    };

    // When
    let material = character_material(base);

    // Then
    assert_eq!(material.base.alpha_mode, AlphaMode::Mask(0.5));
}

#[test]
fn extension_uses_forward_shader_and_builtin_depth_hooks() {
    // Given
    let expected_path = CHARACTER_FORWARD_SHADER_PATH;

    // When
    let fragment_shader = CharacterExtension::fragment_shader();

    // Then
    assert!(matches!(
        fragment_shader,
        ShaderRef::Path(path) if path == expected_path.into()
    ));
    assert!(CharacterExtension::enable_prepass());
    assert!(CharacterExtension::enable_shadows());
    assert!(matches!(
        CharacterExtension::deferred_fragment_shader(),
        ShaderRef::Default
    ));
}

#[test]
fn shader_contract_uses_official_forward_helpers_and_reserved_binding_range() {
    // Given
    let source = include_str!("../src/shaders/character_forward.wgsl");

    // When
    let has_contract = [
        "pbr_input_from_standard_material",
        "alpha_discard",
        "apply_pbr_lighting",
        "main_pass_post_lighting_processing",
        "@binding(100)",
    ]
    .into_iter()
    .all(|required| source.contains(required));

    // Then
    assert!(has_contract);
    assert!(!source.contains("PREPASS_PIPELINE"));
    assert!(!source.contains("deferred_output"));
}

#[test]
fn specialization_seams_have_stable_defaults() {
    // Given
    let extension = CharacterExtension::default();

    // When
    let cull_mode = extension.outline_cull_mode;

    // Then
    assert_eq!(cull_mode, OutlineCullMode::Back);
    assert_eq!(ATTRIBUTE_CHARACTER_MASK.name, "_CEL_MASK");
    assert!(!extension.uses_backface_base_color_uv_b());
}

#[test]
fn backface_base_color_uv_b_is_explicit_and_reversible() {
    let enabled = CharacterExtension::default().with_backface_base_color_uv_b(true);
    let disabled = enabled.clone().with_backface_base_color_uv_b(false);
    let control_only = CharacterExtension::default().with_control_only_vertex_color(true);

    assert!(enabled.uses_backface_base_color_uv_b());
    assert!(!disabled.uses_backface_base_color_uv_b());
    assert!(control_only.uses_control_only_vertex_color());
    assert!(!disabled.uses_control_only_vertex_color());
    assert_eq!(BACKFACE_BASE_COLOR_UNIFORM_BINDING, 115);
}

#[test]
fn ramp_settings_clamp_invalid_values_to_visible_safe_defaults() {
    // Given
    let invalid = ToonRampSettings::default()
        .with_atlas_row(u32::MAX, 0)
        .with_bands(0, f32::NAN)
        .with_transform(f32::INFINITY, f32::NAN)
        .with_debug(ToonRampDebugMode::Band);

    // When
    let values = (
        invalid.ramp_row(),
        invalid.ramp_row_count(),
        invalid.atlas_blend_row_offset(),
        invalid.atlas_blend_factor(),
        invalid.band_count(),
        invalid.band_smoothing(),
        invalid.ramp_scale(),
        invalid.ramp_offset(),
        invalid.debug_mode(),
    );

    // Then
    assert_eq!(
        values,
        (0, 1, 0, 0.0, 1, 0.0, 1.0, 0.0, ToonRampDebugMode::Band)
    );
}

#[test]
fn paired_atlas_page_controls_are_bounded_and_default_inert() {
    let default = ToonRampSettings::default();
    assert_eq!(default.atlas_blend_row_offset(), 0);
    assert_eq!(default.atlas_blend_factor(), 0.0);
    assert_eq!(default.genshin_source_scale(), 1.0);
    assert_eq!(default.genshin_material_id_mask(), 0);

    let clamped = default.with_atlas_blend(u32::MAX, f32::INFINITY);
    assert_eq!(clamped.atlas_blend_row_offset(), 4_095);
    assert_eq!(clamped.atlas_blend_factor(), 0.0);

    let night = default.with_atlas_blend(10, 1.5);
    assert_eq!(night.atlas_blend_row_offset(), 10);
    assert_eq!(night.atlas_blend_factor(), 1.0);

    let source_scale = default.with_genshin_source_scale(31.5);
    assert_eq!(source_scale.genshin_source_scale(), 31.5);
    assert_eq!(
        default
            .with_genshin_material_id_mask(0b1_1110)
            .genshin_material_id_mask(),
        0b1110
    );
    assert_eq!(
        default
            .with_genshin_source_scale(f32::NAN)
            .genshin_source_scale(),
        1.0
    );

    let source = include_str!("../src/shaders/character_forward.wgsl");
    assert!(source.contains("base_row + requested_row"));
    assert!(source.contains("character_extension.atlas_blend"));
    assert!(source.contains("primary_row + secondary_offset"));
}

#[test]
fn authored_emission_gain_is_bounded_explicit_and_default_disabled() {
    let default = ToonRampSettings::default();
    assert_eq!(default.authored_emission_gain(), 0.0);
    assert_eq!(ToonRampSettings::min_size().get(), 96);
    assert_eq!(
        default
            .with_authored_emission_gain(1.0)
            .authored_emission_gain(),
        1.0
    );

    assert_eq!(
        default
            .with_authored_emission_gain(3.5)
            .authored_emission_gain(),
        3.5
    );
    assert_eq!(
        default
            .with_authored_emission_gain(f32::MAX)
            .authored_emission_gain(),
        8.0
    );
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(
            default
                .with_authored_emission_gain(invalid)
                .authored_emission_gain(),
            0.0
        );
    }
    assert_eq!(
        default
            .with_authored_emission_gain(-1.0)
            .authored_emission_gain(),
        0.0
    );
}

#[test]
fn genshin_body_ramp_controls_are_explicit_and_bounded() {
    let defaults = ToonRampSettings::default();
    assert_eq!(defaults.genshin_body(), (0.55, 1.0, false));
    assert_eq!(defaults.genshin_exposure(), 1.0);
    assert_eq!(defaults.genshin_global_shadow(), [1.0; 3]);
    assert_eq!(defaults.genshin_physical_light_influence(), 0.0);
    assert_eq!(
        defaults.with_genshin_body(0.6, 1.5, true).genshin_body(),
        (0.6, 1.5, true)
    );
    assert_eq!(
        defaults
            .with_genshin_body(f32::NAN, f32::INFINITY, true)
            .genshin_body(),
        (0.55, 1.0, true)
    );
    assert_eq!(
        defaults.with_genshin_exposure(f32::NAN).genshin_exposure(),
        1.0
    );
    assert_eq!(defaults.with_genshin_exposure(0.0).genshin_exposure(), 0.25);
    assert_eq!(defaults.with_genshin_exposure(8.0).genshin_exposure(), 1.5);
    assert_eq!(
        defaults
            .with_genshin_global_shadow([0.7, f32::NAN, 2.0])
            .genshin_global_shadow(),
        [0.7, 1.0, 1.0]
    );
    assert_eq!(
        defaults
            .with_genshin_physical_light_influence(2.0)
            .genshin_physical_light_influence(),
        1.0
    );

    let source = include_str!("../src/shaders/character_forward.wgsl");
    for token in [
        "genshin_body: vec4<f32>",
        "genshin_global_shadow: vec4<f32>",
        "genshin_global_lighting_multiplier",
        "metallic_lit_factor > 0.999",
        "let source_direction = resolved_genshin_source_direction(normal)",
        "genshin_explicit_light_enabled()",
        "shadow_position * 2.0 * vertex_color_linear.g * shadow_width",
        "1.0 - (shadow_position - source_lit) / denominator",
        "source_lit < shadow_position",
        "ilm.ao_shadow_influence <= 0.95",
        "select(vec3<f32>(1.0), band, use_ramp)",
        "character_extension.genshin_body.w",
    ] {
        assert!(
            source.contains(token),
            "missing Genshin body contract: {token}"
        );
    }
    assert!(!source.contains("smoothstep(0.90, 0.98, source_lit)"));
}

#[test]
fn authored_emission_is_genshin_only_alpha_masked_and_post_readability() {
    let source = include_str!("../src/shaders/character_forward.wgsl");

    for token in [
        "authored_emission_gain: vec4<f32>",
        "fn apply_authored_emission_gain(",
        "base_diffuse_color: vec3<f32>",
        "base_diffuse_alpha - 0.03",
        "base_diffuse_color * gain * emission_mask",
        "mix(shaded_color, emission_branch, emission_mask)",
        "if genshin_character && !character_debug_active()",
        "let authored_diffuse_alpha = pbr_input.material.base_color.a;",
        "out.color.rgb,\n                        pbr_input.material.base_color.rgb,\n                        authored_diffuse_alpha,",
    ] {
        assert!(
            source.contains(token),
            "missing authored emission contract: {token}"
        );
    }

    let readability_call = source
        .rfind("apply_readability_bounds(")
        .expect("readability call");
    let emission_call = source
        .rfind("apply_authored_emission_gain(")
        .expect("authored emission call");
    let preserve_alpha = source
        .find("let authored_diffuse_alpha = pbr_input.material.base_color.a;")
        .expect("raw authored alpha preservation");
    let alpha_discard = source
        .find("pbr_input.material.base_color = alpha_discard(")
        .expect("Bevy alpha-mode normalization");
    assert!(
        preserve_alpha < alpha_discard,
        "opaque alpha normalization must not erase the authored emission mask"
    );
    assert!(
        emission_call > readability_call,
        "authored HDR restoration must run after LDR readability bounds"
    );
}

#[test]
fn full_width_authored_ramps_keep_all_256_columns() {
    assert_eq!(
        ToonRampSettings::default()
            .with_bands(256, 0.0)
            .band_count(),
        256
    );
    assert_eq!(
        ToonRampSettings::default()
            .with_bands(u32::MAX, 0.0)
            .band_count(),
        256
    );
}

#[test]
fn missing_ramp_uses_the_visible_forward_lit_fallback() {
    // Given
    let extension = CharacterExtension::default();

    // When
    let texture = extension.ramp_texture();

    // Then
    assert!(texture.is_none());
    assert!(!extension.ramp_settings().uses_texture());
}

#[test]
fn ramp_bindings_are_stable_and_above_standard_material_range() {
    // Given
    let bindings = [
        TOON_RAMP_UNIFORM_BINDING,
        RAMP_TEXTURE_BINDING,
        RAMP_SAMPLER_BINDING,
    ];

    // When
    let are_stable = bindings == [100, 101, 102];

    // Then
    assert!(are_stable);
    assert!(bindings.into_iter().all(|binding| binding >= 100));
}

#[test]
fn ramp_shader_contract_uses_linear_atlas_and_post_lighting_proxy() {
    // Given
    let source = include_str!("../src/shaders/character_forward.wgsl");

    // When
    let required_contract = [
        "@binding(101)",
        "@binding(102)",
        "textureSampleLevel",
        "toon_light_proxy",
        "sample_toon_ramp",
        "pbr_input.material.emissive",
        "TOON_RAMP_DEBUG_COORDINATE",
        "TOON_RAMP_DEBUG_BAND",
    ];

    // Then
    assert!(
        required_contract
            .into_iter()
            .all(|token| source.contains(token))
    );
    assert!(!source.contains("clusterable_objects"));
    assert!(!source.contains("point_lights"));
}

#[test]
fn shader_contract_switches_all_standard_and_ilm_sampling_on_backfaces() {
    let source = include_str!("../src/shaders/character_forward.wgsl");

    for token in [
        "@binding(115)",
        "backface_base_color.x != 0u && !is_front",
        "out.uv = in.uv_b",
        "standard_vertex_output(in, is_front)",
        "ilm_uv = in.uv_b",
        "decoded_ilm(in, is_front)",
        "backface_base_color.y != 0u",
        "out.color = select(\n        in.color,\n        vec4<f32>(1.0)",
    ] {
        assert!(
            source.contains(token),
            "missing backface UV contract: {token}"
        );
    }
}
