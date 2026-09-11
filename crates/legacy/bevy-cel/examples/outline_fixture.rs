//! Deterministic GPU fixture for inverted-hull and optional prepass outlines.

use std::{path::PathBuf, time::Duration};

use bevy::{
    asset::{AssetPlugin, RenderAssetUsages},
    camera::RenderTarget,
    core_pipeline::prepass::{DepthPrepass, NormalPrepass},
    ecs::system::SystemParam,
    gltf::GltfAssetLabel,
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
    CharacterMaterial, CharacterMaterialPlugin, CharacterOutline, InternalEdgeAccentSettings,
    InternalEdgeMaterial, OutlineMaterial, OutlineSettings, character_gltf_plugin,
    character_material,
};

#[path = "outline_fixture/config.rs"]
mod config;
use config::{CaptureState, FixtureError, FixtureMode, OutputPath};

const SIZE: u32 = 640;
const MAX_FRAMES: u32 = 300;

fn main() -> Result<(), FixtureError> {
    let mut args = std::env::args().skip(1);
    let mode = FixtureMode::parse(&args.next().ok_or(FixtureError::MissingMode)?)?;
    let output = PathBuf::from(args.next().ok_or(FixtureError::MissingOutput)?);
    let mut app = App::new();
    app.insert_resource(mode)
        .insert_resource(OutputPath(output))
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: "../../assets".into(),
                    ..default()
                })
                .set(character_gltf_plugin())
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::DontExit,
                    ..default()
                })
                .set(RenderPlugin {
                    synchronous_pipeline_compilation: true,
                    ..default()
                }),
        )
        .add_plugins(CharacterMaterialPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (prepare_deformed, request_capture).chain());
    app.finish();
    app.cleanup();
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
        if app.world().resource::<CaptureState>().complete {
            return Ok(());
        }
    }
    Err(FixtureError::CaptureTimeout)
}

#[derive(SystemParam)]
struct SetupParams<'w> {
    mode: Res<'w, FixtureMode>,
    output: Res<'w, OutputPath>,
    asset_server: Res<'w, AssetServer>,
    images: ResMut<'w, Assets<Image>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    characters: ResMut<'w, Assets<CharacterMaterial>>,
    outlines: ResMut<'w, Assets<OutlineMaterial>>,
    accents: ResMut<'w, Assets<InternalEdgeMaterial>>,
}

fn setup(mut commands: Commands, mut params: SetupParams) {
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
    let target = params.images.add(target);
    commands.insert_resource(CaptureState {
        output: params.output.0.clone(),
        target: target.clone(),
        ready_frames: 0,
        requested: false,
        complete: false,
    });

    if matches!(*params.mode, FixtureMode::Deformed) {
        for (path, position) in [
            ("test/skinned_two_bone_mesh.glb", Vec3::new(-1.1, -0.8, 0.0)),
            ("test/morph_cube.glb", Vec3::new(0.7, 0.0, 0.0)),
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
    } else {
        let mesh = if matches!(*params.mode, FixtureMode::Accent) {
            params.meshes.add(Cuboid::new(2.0, 2.0, 2.0))
        } else {
            params.meshes.add(Sphere::new(1.0).mesh().uv(48, 24))
        };
        let entries: &[(f32, f32)] = match *params.mode {
            FixtureMode::Width => &[(-1.4, 2.0), (1.4, 4.0)],
            FixtureMode::Exclusion => &[(-2.0, 4.0), (0.0, 4.0), (2.0, 4.0)],
            FixtureMode::Malformed => &[(-1.4, f32::NAN), (1.4, f32::INFINITY)],
            FixtureMode::Accent => &[(0.0, 3.0)],
            FixtureMode::Deformed => &[],
        };
        for (x, width) in entries {
            let settings = params
                .mode
                .apply_exclusion(*x, OutlineSettings::default().with_width(*width));
            let hull = params.outlines.add(OutlineMaterial {
                settings,
                ..default()
            });
            let accent = matches!(*params.mode, FixtureMode::Accent).then(|| {
                params.accents.add(InternalEdgeMaterial {
                    settings: InternalEdgeAccentSettings::default().with_enabled(true),
                })
            });
            let outline = match accent {
                Some(accent) => CharacterOutline::new(hull).with_internal_edge(accent),
                None => CharacterOutline::new(hull),
            };
            commands.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(params.characters.add(character_material(StandardMaterial {
                    base_color: Color::srgb(0.86, 0.42, 0.2),
                    unlit: true,
                    perceptual_roughness: 0.72,
                    ..default()
                }))),
                outline,
                Transform::from_xyz(*x, 0.0, 0.0).with_rotation(
                    matches!(*params.mode, FixtureMode::Accent)
                        .then(|| Quat::from_euler(EulerRot::XYZ, -0.35, 0.65, 0.0))
                        .unwrap_or(Quat::IDENTITY),
                ),
            ));
        }
    }

    let mut camera = commands.spawn((
        Camera3d::default(),
        RenderTarget::Image(target.into()),
        Transform::from_xyz(0.0, 1.1, 8.5).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    if matches!(*params.mode, FixtureMode::Accent) {
        camera.insert((DepthPrepass, NormalPrepass, Msaa::Off));
    }
}

#[derive(SystemParam)]
struct DeformedParams<'w, 's> {
    mode: Res<'w, FixtureMode>,
    standard: Query<'w, 's, (Entity, &'static MeshMaterial3d<StandardMaterial>)>,
    standard_assets: Res<'w, Assets<StandardMaterial>>,
    characters: ResMut<'w, Assets<CharacterMaterial>>,
    outlines: ResMut<'w, Assets<OutlineMaterial>>,
    morphs: Query<'w, 's, &'static mut MorphWeights>,
    bones: Query<'w, 's, (&'static Name, &'static mut Transform)>,
}

fn prepare_deformed(mut commands: Commands, mut params: DeformedParams) {
    if !matches!(*params.mode, FixtureMode::Deformed) {
        return;
    }
    for (entity, material) in &params.standard {
        if let Some(base) = params.standard_assets.get(material) {
            let mut base = base.clone();
            base.unlit = true;
            commands
                .entity(entity)
                .remove::<MeshMaterial3d<StandardMaterial>>()
                .insert((
                    MeshMaterial3d(params.characters.add(character_material(base))),
                    CharacterOutline::new(params.outlines.add(OutlineMaterial {
                        settings: OutlineSettings::default().with_width(3.0),
                        ..default()
                    })),
                ));
        }
    }
    for mut weights in &mut params.morphs {
        if let Some(weight) = weights.weights_mut().first_mut() {
            *weight = 1.0;
        }
    }
    for (name, mut transform) in &mut params.bones {
        if name.as_str() == "TipBone" {
            transform.rotation = Quat::from_rotation_z(-0.55);
        }
    }
}

#[derive(SystemParam)]
struct CaptureParams<'w, 's> {
    mode: Res<'w, FixtureMode>,
    state: ResMut<'w, CaptureState>,
    standard: Query<'w, 's, (), With<MeshMaterial3d<StandardMaterial>>>,
    skinned: Query<'w, 's, (), With<SkinnedMesh>>,
    morphs: Query<'w, 's, (), With<MorphWeights>>,
}

fn request_capture(mut commands: Commands, mut params: CaptureParams) {
    let ready = match *params.mode {
        FixtureMode::Deformed => {
            !params.skinned.is_empty() && !params.morphs.is_empty() && params.standard.is_empty()
        }
        FixtureMode::Width
        | FixtureMode::Exclusion
        | FixtureMode::Malformed
        | FixtureMode::Accent => true,
    };
    if ready {
        params.state.ready_frames += 1;
    }
    if params.state.ready_frames < 12 || params.state.requested {
        return;
    }
    params.state.requested = true;
    commands
        .spawn(Screenshot::image(params.state.target.clone()))
        .observe(save_to_disk(params.state.output.clone()))
        .observe(mark_complete);
}

fn mark_complete(_capture: On<ScreenshotCaptured>, mut state: ResMut<CaptureState>) {
    state.complete = true;
}
