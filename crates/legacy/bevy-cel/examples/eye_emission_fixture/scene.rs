use bevy::{
    camera::{Hdr, RenderTarget},
    ecs::system::SystemParam,
    post_process::bloom::Bloom,
};
use bevy_cel::{
    CharacterExtension, CharacterMaterial, EyeEmissionDebugMode, EyeEmissionSettings,
};

#[derive(SystemParam)]
struct SceneAssets<'w> {
    images: ResMut<'w, Assets<Image>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<CharacterMaterial>>,
}

fn eye_settings() -> EyeEmissionSettings {
    EyeEmissionSettings::default()
        .with_eye_floor(0.28, 0.08)
        .with_eye_class(1.0, 1.0)
        .with_eye_enabled(true)
}

fn material(color: Color, settings: EyeEmissionSettings) -> CharacterMaterial {
    CharacterMaterial {
        base: StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.72,
            ..default()
        },
        extension: CharacterExtension::default().with_eye_emission_settings(settings),
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Bevy systems consume parameters by value"
)]
fn setup(mut commands: Commands, config: Res<CaptureConfig>, mut assets: SceneAssets) {
    let target = assets.images.add(render_target(SIZE));
    commands.insert_resource(CaptureState {
        target: target.clone(),
        frames: 0,
        requested: false,
        result: None,
    });
    let texture_image = emission_mask_texture();
    let texture = assets.images.add(texture_image.clone());
    let iris_mask = (!matches!(config.mode, FixtureMode::Missing)).then_some(1.0);
    let emission = eye_settings()
        .with_emission([0.15, 0.55, 1.0], 12.0)
        .with_masks(
            if matches!(config.mode, FixtureMode::Missing) { 0.0 } else { 1.0 },
            1.0,
        )
        .with_emission_enabled(config.mode.emits())
        .with_debug(if matches!(config.mode, FixtureMode::Debug) {
            EyeEmissionDebugMode::EmissionMask
        } else {
            EyeEmissionDebugMode::Disabled
        });
    let iris_extension = CharacterExtension::default()
        .with_emission_texture(texture, &texture_image)
        .with_eye_emission_settings(emission);
    commands.spawn((
        Mesh3d(assets.meshes.add(sphere_with_emission(1.45, None))),
        MeshMaterial3d(
            assets
                .materials
                .add(material(Color::srgb(0.42, 0.46, 0.52), eye_settings())),
        ),
    ));
    commands.spawn((
        Mesh3d(assets.meshes.add(sphere_with_emission(0.66, iris_mask))),
        MeshMaterial3d(assets.materials.add(CharacterMaterial {
            base: StandardMaterial {
                base_color: Color::srgb(0.025, 0.08, 0.16),
                perceptual_roughness: 0.5,
                ..default()
            },
            extension: iris_extension,
        })),
        Transform::from_xyz(0.0, 0.0, 1.28),
    ));
    commands.spawn((
        Mesh3d(assets.meshes.add(sphere_with_emission(0.22, None))),
        MeshMaterial3d(
            assets
                .materials
                .add(material(Color::srgb(0.005, 0.008, 0.012), eye_settings())),
        ),
        Transform::from_xyz(0.0, 0.0, 1.82),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 140.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-3.0, 1.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    let camera = commands
        .spawn((
            Camera3d::default(),
            Hdr,
            RenderTarget::Image(target.into()),
            Transform::from_xyz(0.0, 0.0, 7.0).looking_at(Vec3::ZERO, Vec3::Y),
        ))
        .id();
    if matches!(config.mode, FixtureMode::Bloom) {
        commands.entity(camera).insert(Bloom::NATURAL);
    }
}
