use bevy::{math::Vec3, render::render_resource::ShaderType};
use bevy_cel::{CharacterExtension, GENSHIN_LIGHT_SOURCE_UNIFORM_BINDING, GenshinLightSource};

#[test]
fn explicit_source_is_normalized_bounded_and_default_disabled() {
    let default = GenshinLightSource::default();
    assert!(!default.is_enabled());
    assert_eq!(default.direction_to_light_world(), Vec3::Y);
    assert_eq!(GenshinLightSource::min_size().get(), 32);
    assert_eq!(GENSHIN_LIGHT_SOURCE_UNIFORM_BINDING, 126);

    let source = GenshinLightSource::explicit(Vec3::new(0.0, 3.0, 4.0), [1.2, -1.0, 0.5], 20.0);
    assert!(source.is_enabled());
    assert_eq!(source.direction_to_light_world(), Vec3::new(0.0, 0.6, 0.8));
    assert_eq!(source.tint(), [1.0, 0.0, 0.5]);
    assert_eq!(source.energy(), 16.0);
}

#[test]
fn invalid_direction_fails_closed_while_zero_energy_remains_explicit() {
    for direction in [Vec3::ZERO, Vec3::NAN, Vec3::INFINITY] {
        assert!(!GenshinLightSource::explicit(direction, [1.0; 3], 1.0).is_enabled());
    }
    let blackout = GenshinLightSource::explicit(Vec3::Y, [f32::NAN, 0.5, 0.25], -1.0);
    assert!(blackout.is_enabled());
    assert_eq!(blackout.tint(), [1.0, 0.5, 0.25]);
    assert_eq!(blackout.energy(), 0.0);
}

#[test]
fn shader_declares_the_explicit_source_binding() {
    let source = include_str!("../src/shaders/character_forward.wgsl");
    for required in [
        "struct GenshinLightSourceUniform",
        "@binding(126)",
        "var<uniform> genshin_light_source",
    ] {
        assert!(
            source.contains(required),
            "missing shader contract: {required}"
        );
    }
}

#[test]
fn character_extension_keeps_the_source_opt_in_and_reversible() {
    let explicit = GenshinLightSource::explicit(Vec3::X, [1.0; 3], 1.0);
    let material = CharacterExtension::default().with_genshin_light_source(explicit);
    assert_eq!(material.genshin_light_source(), &explicit);
    assert!(
        !CharacterExtension::default()
            .genshin_light_source()
            .is_enabled()
    );

    let source = include_str!("../src/shaders/character_forward.wgsl");
    assert_eq!(source.matches("lights.directional_lights[0]").count(), 3);
    assert!(source.contains("resolved_genshin_source_direction(normal)"));
    assert!(source.contains("resolved_genshin_direction_to_light(pbr_input.N)"));
    assert!(source.contains("resolved_genshin_light_energy(physical_raw_proxy)"));
    assert!(source.contains("cel_lighting.state.z != 0u && !genshin_explicit_light_enabled()"));
}
