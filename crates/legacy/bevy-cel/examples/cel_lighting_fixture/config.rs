fn fixture_settings(mode: FixtureMode) -> CelLightingSettings {
    let hair = HairHighlightSettings::default()
        .with_tint_shift(
            [1.0, 0.72, 0.35],
            if matches!(mode, FixtureMode::HairShift) { 0.2 } else { 0.0 },
        )
        .with_shape(if matches!(mode, FixtureMode::HairWide) { 4.0 } else { 24.0 }, 0.45)
        .with_softness_intensity(0.08, 2.4);
    let specular = ToonSpecularSettings::default()
        .with_tint_intensity([0.45, 0.75, 1.0], 2.0)
        .with_shape(24.0, 0.55)
        .with_softness(if matches!(mode, FixtureMode::SpecularSoft) { 0.25 } else { 0.04 });
    let rim = RimLightingSettings::default()
        .with_tint_intensity([0.4, 0.7, 1.0], 2.4)
        .with_shape(if matches!(mode, FixtureMode::RimPower) { 8.0 } else { 2.0 }, 0.25)
        .with_softness(0.08);
    CelLightingSettings::default()
        .with_hair(hair)
        .with_specular(specular)
        .with_rim(rim)
        .with_hair_enabled(matches!(mode, FixtureMode::Hair | FixtureMode::HairShift | FixtureMode::HairWide | FixtureMode::Degenerate | FixtureMode::DebugHair))
        .with_specular_enabled(matches!(mode, FixtureMode::Specular | FixtureMode::SpecularSoft | FixtureMode::DebugSpecular))
        .with_rim_enabled(matches!(mode, FixtureMode::Rim | FixtureMode::RimPower | FixtureMode::DebugRim))
        .with_debug(mode.debug())
}

fn fixture_material(
    ramp: Handle<Image>,
    settings: CelLightingSettings,
    masks: [f32; 2],
) -> CharacterMaterial {
    CharacterMaterial {
        base: StandardMaterial {
            base_color: Color::srgb(0.06, 0.035, 0.09),
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            ..default()
        },
        extension: CharacterExtension::default()
            .with_ramp_texture(ramp)
            .with_ramp_settings(ToonRampSettings::default().with_bands(1, 0.0))
            .with_ilm_settings(
                bevy_cel::IlmSettings::default().with_fallback_channels([1.0, 1.0, 0.0, 0.0]),
            )
            .with_cel_lighting_settings(settings.with_material_masks(masks[0], masks[1], 1.0)),
    }
}
