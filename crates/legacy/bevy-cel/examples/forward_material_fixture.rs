//! Deterministic render proof for the reusable character material.

use std::{error::Error, fmt, path::PathBuf, time::Duration};

use bevy::{
    asset::{AssetPlugin, RenderAssetUsages},
    camera::RenderTarget,
    ecs::system::SystemParam,
    gltf::GltfAssetLabel,
    image::ImageSampler,
    material::AlphaMode,
    mesh::{morph::MorphWeights, skinning::SkinnedMesh},
    prelude::*,
    render::{
        RenderPlugin,
        render_resource::{Extent3d, PollType, TextureDimension, TextureFormat, TextureUsages},
        renderer::RenderDevice,
        view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_cel::{
    CharacterMaterial, CharacterMaterialPlugin, character_gltf_plugin, character_material,
};

const WIDTH: u32 = 960;
const HEIGHT: u32 = 540;
const MAX_FRAMES: u32 = 300;

#[derive(Debug)]
enum FixtureError {
    GpuPoll,
    CaptureTimeout,
}

impl fmt::Display for FixtureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GpuPoll => formatter.write_str("GPU poll failed"),
            Self::CaptureTimeout => formatter.write_str("fixture capture timed out"),
        }
    }
}

impl Error for FixtureError {}

#[derive(Resource)]
struct CaptureOutput(PathBuf);

#[derive(Resource)]
struct CaptureState {
    output: PathBuf,
    target: Handle<Image>,
    ready_frames: u32,
    requested: bool,
    complete: bool,
}

#[derive(SystemParam)]
struct SetupParams<'w> {
    output: Res<'w, CaptureOutput>,
    asset_server: Res<'w, AssetServer>,
    images: ResMut<'w, Assets<Image>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<CharacterMaterial>>,
}

#[derive(SystemParam)]
struct FixtureParams<'w, 's> {
    standard_handles: Query<'w, 's, (Entity, &'static MeshMaterial3d<StandardMaterial>)>,
    standard_materials: ResMut<'w, Assets<StandardMaterial>>,
    character_materials: ResMut<'w, Assets<CharacterMaterial>>,
    morph_weights: Query<'w, 's, &'static mut MorphWeights>,
    bones: Query<'w, 's, (&'static Name, &'static mut Transform)>,
}

#[derive(SystemParam)]
struct CaptureParams<'w, 's> {
    state: ResMut<'w, CaptureState>,
    skinned: Query<'w, 's, (), With<SkinnedMesh>>,
    morphs: Query<'w, 's, (), With<MorphWeights>>,
    standard_handles: Query<'w, 's, (), With<MeshMaterial3d<StandardMaterial>>>,
}

fn main() -> Result<(), FixtureError> {
    let output = std::env::args().nth(1).map_or_else(
        || PathBuf::from("forward-material-fixture.png"),
        PathBuf::from,
    );
    let mut app = App::new();
    app.insert_resource(CaptureOutput(output))
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: "../../assets".into(),
                    ..Default::default()
                })
                .set(character_gltf_plugin())
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::DontExit,
                    ..Default::default()
                })
                .set(RenderPlugin {
                    synchronous_pipeline_compilation: true,
                    ..Default::default()
                }),
        )
        .add_plugins(CharacterMaterialPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (prepare_fixtures, request_capture).chain());
    app.finish();
    app.cleanup();

    let mut captured = false;
    for _ in 0..MAX_FRAMES {
        app.update();
        if app
            .world()
            .resource::<RenderDevice>()
            .wgpu_device()
            .poll(PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(5)),
            })
            .is_err()
        {
            return Err(FixtureError::GpuPoll);
        }
        captured = app.world().resource::<CaptureState>().complete;
        if captured {
            break;
        }
    }
    if !captured {
        return Err(FixtureError::CaptureTimeout);
    }
    Ok(())
}

fn setup(mut commands: Commands, mut params: SetupParams) {
    let mut target = Image::new_uninit(
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    target.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
    let target = params.images.add(target);
    commands.insert_resource(CaptureState {
        output: params.output.0.clone(),
        target: target.clone(),
        ready_frames: 0,
        requested: false,
        complete: false,
    });

    commands.spawn((
        Name::new("ExtensionMaterialSphere"),
        Mesh3d(params.meshes.add(Sphere::new(1.0).mesh().uv(32, 18))),
        MeshMaterial3d(params.materials.add(character_material(StandardMaterial {
            base_color: Color::srgb(0.9, 0.35, 0.18),
            perceptual_roughness: 0.7,
            ..Default::default()
        }))),
        Transform::from_xyz(-3.0, 1.1, 0.0),
    ));

    let mut alpha_texture = Image::new_fill(
        Extent3d {
            width: 2,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255, 255, 255, 0, 255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    alpha_texture.sampler = ImageSampler::nearest();
    let alpha_texture = params.images.add(alpha_texture);
    commands.spawn((
        Name::new("AlphaMaskProof"),
        Mesh3d(params.meshes.add(Rectangle::new(2.0, 2.0))),
        MeshMaterial3d(params.materials.add(character_material(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(alpha_texture),
            alpha_mode: AlphaMode::Mask(0.5),
            double_sided: true,
            ..Default::default()
        }))),
        Transform::from_xyz(3.2, 1.2, 0.0),
    ));

    for (path, position) in [
        ("test/skinned_two_bone_mesh.glb", Vec3::new(-1.0, 0.0, 0.0)),
        ("test/morph_cube.glb", Vec3::new(1.2, 1.0, 0.0)),
    ] {
        commands.spawn((
            WorldAssetRoot(
                params
                    .asset_server
                    .load(GltfAssetLabel::Scene(0).from_asset(path)),
            ),
            Transform::from_translation(position),
        ));
    }

    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadow_maps_enabled: true,
            ..Default::default()
        },
        Transform::from_xyz(3.0, 6.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        RenderTarget::Image(target.into()),
        Transform::from_xyz(0.0, 3.2, 10.5).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));
}

fn prepare_fixtures(mut commands: Commands, mut params: FixtureParams) {
    for (entity, handle) in &params.standard_handles {
        if let Some(base) = params.standard_materials.get_mut(handle) {
            let character = character_material(base.clone());
            commands
                .entity(entity)
                .remove::<MeshMaterial3d<StandardMaterial>>()
                .insert(MeshMaterial3d(params.character_materials.add(character)));
        }
    }
    for mut weights in &mut params.morph_weights {
        if let Some(first) = weights.weights_mut().first_mut() {
            *first = 1.0;
        }
    }
    for (name, mut transform) in &mut params.bones {
        if name.as_str() == "TipBone" {
            transform.rotation = Quat::from_rotation_z(-0.45);
        }
    }
}

fn request_capture(mut commands: Commands, mut params: CaptureParams) {
    let fixtures_ready = !params.skinned.is_empty()
        && !params.morphs.is_empty()
        && params.standard_handles.is_empty();
    if fixtures_ready {
        params.state.ready_frames += 1;
    }
    if params.state.ready_frames < 8 || params.state.requested {
        return;
    }

    params.state.requested = true;
    info!(
        "TASK7_RENDER_READY sphere=1 skinned={} morph={} alpha_mask=Mask(0.5)",
        params.skinned.iter().count(),
        params.morphs.iter().count()
    );
    commands
        .spawn(Screenshot::image(params.state.target.clone()))
        .observe(save_to_disk(params.state.output.clone()))
        .observe(mark_capture_complete);
}

fn mark_capture_complete(_capture: On<ScreenshotCaptured>, mut state: ResMut<CaptureState>) {
    state.complete = true;
    info!("TASK7_RENDER_CAPTURED path={}", state.output.display());
}
