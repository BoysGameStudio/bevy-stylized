#![expect(
    clippy::needless_pass_by_value,
    clippy::redundant_pub_crate,
    reason = "Bevy systems consume parameters by value and the parent fixture consumes split-module items"
)]

use bevy::{
    asset::RenderAssetUsages,
    camera::RenderTarget,
    color::LinearRgba,
    core_pipeline::prepass::{DepthPrepass, NormalPrepass},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};
use bevy_cel::StylizedPostProcessProfile;

use super::{
    capture::CaptureState,
    config::{FixtureConfig, FixtureMode},
};

const SIZE: u32 = 640;

pub(crate) fn setup(
    mut commands: Commands,
    config: Res<FixtureConfig>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut target = Image::new_uninit(
        Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    target.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
    let target = images.add(target);
    commands.insert_resource(CaptureState {
        target: target.clone(),
        frames: 0,
        requested: false,
        complete: false,
        failure: None,
    });

    let background = materials.add(StandardMaterial {
        base_color: Color::linear_rgb(0.012, 0.018, 0.032),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Rectangle::new(14.0, 10.0))),
        MeshMaterial3d(background),
        Transform::from_xyz(0.0, 0.0, -1.0),
    ));

    let calibration = materials.add(StandardMaterial {
        base_color: Color::linear_rgb(0.35, 0.35, 0.35),
        unlit: true,
        ..default()
    });
    for position in [
        Vec3::new(-2.9, 0.0, 0.0),
        Vec3::ZERO,
        Vec3::new(2.9, 0.0, 0.0),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Rectangle::new(0.62, 1.9))),
            MeshMaterial3d(calibration.clone()),
            Transform::from_translation(position),
        ));
    }

    let bright = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::new(18.0, 14.0, 8.0, 1.0),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(0.22))),
        MeshMaterial3d(bright),
        Transform::from_xyz(-1.7, 0.65, 0.0),
    ));

    for (position, color) in [
        (Vec3::new(2.55, 1.65, 0.0), Color::WHITE),
        (Vec3::new(2.55, -1.65, 0.0), Color::srgb(0.2, 0.75, 1.0)),
        (Vec3::new(-2.55, -1.65, 0.0), Color::srgb(1.0, 0.25, 0.12)),
    ] {
        let marker = materials.add(StandardMaterial {
            base_color: color,
            unlit: true,
            ..default()
        });
        commands.spawn((
            Mesh3d(meshes.add(Rectangle::new(0.14, 1.1))),
            MeshMaterial3d(marker),
            Transform::from_translation(position),
        ));
    }

    commands.spawn((
        Camera3d::default(),
        RenderTarget::Image(target.into()),
        Transform::from_xyz(0.0, 0.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        config.mode.profile(),
    ));
}

pub(crate) fn strip_missing_prepasses(
    mut commands: Commands,
    config: Res<FixtureConfig>,
    cameras: Query<Entity, (With<Camera3d>, With<StylizedPostProcessProfile>)>,
) {
    if config.mode != FixtureMode::MissingPrepass {
        return;
    }
    for camera in &cameras {
        commands
            .entity(camera)
            .remove::<DepthPrepass>()
            .remove::<NormalPrepass>();
    }
}
