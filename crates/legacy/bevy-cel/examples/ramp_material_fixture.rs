//! Deterministic task-11 offscreen render fixture.

use std::{error::Error, fmt, path::PathBuf, time::Duration};

use bevy::{
    asset::RenderAssetUsages,
    camera::RenderTarget,
    ecs::system::SystemParam,
    image::ImageSampler,
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
    CharacterExtension, CharacterMaterial, CharacterMaterialPlugin, ToonRampDebugMode,
    ToonRampSettings,
};

const SIZE: u32 = 640;
const MAX_FRAMES: u32 = 300;

#[derive(Clone, Copy)]
enum FixtureMode {
    Ramp,
    Fallback,
    DebugBand,
}

impl FixtureMode {
    fn parse(value: &str) -> Result<Self, FixtureError> {
        match value {
            "ramp" => Ok(Self::Ramp),
            "fallback" => Ok(Self::Fallback),
            "debug-band" => Ok(Self::DebugBand),
            _ => Err(FixtureError::InvalidMode(value.to_owned())),
        }
    }
}

#[derive(Debug)]
enum FixtureError {
    MissingOutput,
    InvalidMode(String),
    GpuPoll,
    CaptureTimeout,
}

impl fmt::Display for FixtureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOutput => formatter
                .write_str("usage: ramp_material_fixture <output.png> <ramp|fallback|debug-band>"),
            Self::InvalidMode(mode) => write!(formatter, "invalid fixture mode: {mode}"),
            Self::GpuPoll => formatter.write_str("GPU poll failed"),
            Self::CaptureTimeout => formatter.write_str("fixture capture timed out"),
        }
    }
}

impl Error for FixtureError {}

#[derive(Resource)]
struct CaptureConfig {
    output: PathBuf,
    mode: FixtureMode,
}

#[derive(Resource)]
struct CaptureState {
    target: Handle<Image>,
    frames: u32,
    requested: bool,
    complete: bool,
}

#[derive(SystemParam)]
struct SetupParams<'w> {
    images: ResMut<'w, Assets<Image>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<CharacterMaterial>>,
}

fn main() -> Result<(), FixtureError> {
    let mut arguments = std::env::args().skip(1);
    let output = arguments
        .next()
        .map(PathBuf::from)
        .ok_or(FixtureError::MissingOutput)?;
    let mode = arguments
        .next()
        .ok_or(FixtureError::MissingOutput)
        .and_then(|value| FixtureMode::parse(&value))?;
    let mut app = App::new();
    app.insert_resource(CaptureConfig { output, mode })
        .add_plugins(
            DefaultPlugins
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
        .add_systems(Update, request_capture);
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

#[expect(
    clippy::needless_pass_by_value,
    reason = "Bevy system parameters are values"
)]
fn setup(mut commands: Commands, config: Res<CaptureConfig>, mut params: SetupParams) {
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
        target: target.clone(),
        frames: 0,
        requested: false,
        complete: false,
    });

    let mut ramp = Image::new_fill(
        Extent3d {
            width: 3,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[48, 48, 48, 255, 138, 138, 138, 255, 245, 245, 245, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    ramp.sampler = ImageSampler::linear();
    let ramp = params.images.add(ramp);
    let settings = ToonRampSettings::default().with_bands(3, 0.0);
    let extension = match config.mode {
        FixtureMode::Ramp => CharacterExtension::default()
            .with_ramp_texture(ramp)
            .with_ramp_settings(settings),
        FixtureMode::Fallback => CharacterExtension::default().with_ramp_settings(settings),
        FixtureMode::DebugBand => CharacterExtension::default()
            .with_ramp_texture(ramp)
            .with_ramp_settings(settings.with_debug(ToonRampDebugMode::Band)),
    };
    let material = params.materials.add(CharacterMaterial {
        base: StandardMaterial {
            base_color: Color::srgb(0.24, 0.55, 0.96),
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            ..default()
        },
        extension,
    });
    commands.spawn((
        Mesh3d(params.meshes.add(Sphere::new(1.65).mesh().uv(64, 32))),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, 0.2, 0.0),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 14_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-4.0, 5.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        RenderTarget::Image(target.into()),
        Transform::from_xyz(0.0, 0.3, 6.3).looking_at(Vec3::new(0.0, 0.2, 0.0), Vec3::Y),
    ));
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Bevy system parameters are values"
)]
fn request_capture(
    mut commands: Commands,
    config: Res<CaptureConfig>,
    mut state: ResMut<CaptureState>,
) {
    state.frames += 1;
    if state.frames < 8 || state.requested {
        return;
    }
    state.requested = true;
    commands
        .spawn(Screenshot::image(state.target.clone()))
        .observe(save_to_disk(config.output.clone()))
        .observe(mark_capture_complete);
}

fn mark_capture_complete(_capture: On<ScreenshotCaptured>, mut state: ResMut<CaptureState>) {
    state.complete = true;
}
