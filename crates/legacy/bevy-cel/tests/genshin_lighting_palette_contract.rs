use bevy::render::render_resource::ShaderType;
use bevy_cel::{
    GENSHIN_LIGHTING_PALETTE_UNIFORM_BINDING, GenshinLightingDebugMode, GenshinLightingPalette,
};

#[test]
fn palette_is_neutral_and_inert_until_explicitly_enabled() {
    let palette = GenshinLightingPalette::default();
    assert!(!palette.is_enabled());
    assert!(!palette.is_strict_reference());
    assert_eq!(palette.ambient(), [1.0; 3]);
    assert_eq!(palette.sharp_lit(), [1.0; 3]);
    assert_eq!(palette.soft_lit(), [1.0; 3]);
    assert_eq!(palette.sharp_shadow(), [1.0; 3]);
    assert_eq!(palette.soft_shadow(), [1.0; 3]);
    assert_eq!(palette.rim_lit(), [1.0; 3]);
    assert_eq!(palette.rim_shadow(), [1.0; 3]);
    assert_eq!(GenshinLightingPalette::min_size().get(), 112);
    assert_eq!(GENSHIN_LIGHTING_PALETTE_UNIFORM_BINDING, 125);
    assert_eq!(palette.debug_mode(), GenshinLightingDebugMode::Disabled);
}

#[test]
fn strict_reference_is_explicit_and_preserves_palette_values() {
    let palette = GenshinLightingPalette::default().with_strict_reference(true);
    assert!(palette.is_strict_reference());
    assert!(!palette.is_enabled());
    assert_eq!(palette.rim_lit(), [1.0; 3]);
}

#[test]
fn diagnostic_modes_are_typed_independent_and_shader_visible() {
    for mode in [
        GenshinLightingDebugMode::Disabled,
        GenshinLightingDebugMode::SourceLit,
        GenshinLightingDebugMode::RampCoordinate,
        GenshinLightingDebugMode::RampBand,
        GenshinLightingDebugMode::FaceFactor,
        GenshinLightingDebugMode::PaletteMultiplier,
        GenshinLightingDebugMode::SourceNormal,
    ] {
        let palette = GenshinLightingPalette::default().with_debug(mode);
        assert_eq!(palette.debug_mode(), mode);
        assert_eq!(palette.ambient(), [1.0; 3]);
    }
    let source = include_str!("../src/shaders/character_forward.wgsl");
    for required in [
        "GENSHIN_DEBUG_SOURCE_LIT",
        "GENSHIN_DEBUG_RAMP_COORDINATE",
        "GENSHIN_DEBUG_RAMP_BAND",
        "GENSHIN_DEBUG_FACE_FACTOR",
        "GENSHIN_DEBUG_PALETTE_MULTIPLIER",
        "GENSHIN_DEBUG_SOURCE_NORMAL",
    ] {
        assert!(source.contains(required));
    }
}

#[test]
fn enabled_palette_sanitizes_every_color_channel() {
    let palette = GenshinLightingPalette::enabled(
        [0.4, 0.45, 0.6],
        [0.65, 0.75, 0.95],
        [0.6, 0.7, 0.9],
        [0.2, 0.25, 0.45],
        [0.25, 0.3, 0.5],
        [0.5, 0.7, 1.0],
        [0.2, 0.3, 0.5],
    );
    assert!(palette.is_enabled());
    assert_eq!(palette.ambient(), [0.4, 0.45, 0.6]);
    assert_eq!(palette.rim_lit(), [0.5, 0.7, 1.0]);

    let sanitized = GenshinLightingPalette::enabled(
        [f32::NAN, -1.0, 2.0],
        [1.0; 3],
        [1.0; 3],
        [1.0; 3],
        [1.0; 3],
        [1.0; 3],
        [1.0; 3],
    );
    assert_eq!(sanitized.ambient(), [1.0, 0.0, 1.0]);
    assert!(!sanitized.with_enabled(false).is_enabled());
}

#[test]
fn shader_uses_the_exact_soft_sharp_ambient_product() {
    let source = include_str!("../src/shaders/character_forward.wgsl");
    for required in [
        "@binding(125)",
        "genshin_lighting_palette.soft_shadow.rgb",
        "genshin_lighting_palette.soft_lit.rgb",
        "genshin_lighting_palette.sharp_shadow.rgb",
        "genshin_lighting_palette.sharp_lit.rgb",
        "genshin_lighting_palette.ambient.rgb * soft * sharp",
        "genshin_character && !genshin_lighting_palette_enabled()",
        "out.color.rgb * genshin_lighting_multiplier",
    ] {
        assert!(
            source.contains(required),
            "missing shader contract: {required}"
        );
    }
}
