//! Deterministic task-17 hair, specular, and rim Vulkan fixture.

use std::{error::Error, fmt, path::PathBuf, time::Duration};

use bevy::{
    camera::RenderTarget,
    ecs::system::SystemParam,
    prelude::*,
    render::{
        RenderPlugin,
        render_resource::PollType,
        renderer::RenderDevice,
        view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_cel::{
    CelLightingDebugMode, CelLightingSettings, CharacterExtension, CharacterMaterial,
    CharacterMaterialPlugin, HairHighlightSettings, RimLightingSettings, ToonRampSettings,
    ToonSpecularSettings,
};

include!("cel_lighting_fixture/assets.rs");

const SIZE: u32 = 640;

#[derive(Clone, Copy)]
enum FixtureMode {
    Hair,
    HairShift,
    HairWide,
    Degenerate,
    Specular,
    SpecularSoft,
    Rim,
    RimPower,
    DebugHair,
    DebugSpecular,
    DebugRim,
    Disabled,
}

impl FixtureMode {
    fn parse(value: &str) -> Result<Self, FixtureError> {
        match value {
            "hair" => Ok(Self::Hair),
            "hair-shift" => Ok(Self::HairShift),
            "hair-wide" => Ok(Self::HairWide),
            "degenerate" => Ok(Self::Degenerate),
            "specular" => Ok(Self::Specular),
            "specular-soft" => Ok(Self::SpecularSoft),
            "rim" => Ok(Self::Rim),
            "rim-power" => Ok(Self::RimPower),
            "debug-hair" => Ok(Self::DebugHair),
            "debug-specular" => Ok(Self::DebugSpecular),
            "debug-rim" => Ok(Self::DebugRim),
            "disabled" => Ok(Self::Disabled),
            _ => Err(FixtureError::InvalidMode(value.to_owned())),
        }
    }

    const fn debug(self) -> CelLightingDebugMode {
        match self {
            Self::DebugHair => CelLightingDebugMode::HairResult,
            Self::DebugSpecular => CelLightingDebugMode::SpecularResult,
            Self::DebugRim => CelLightingDebugMode::RimResult,
            Self::Hair
            | Self::HairShift
            | Self::HairWide
            | Self::Degenerate
            | Self::Specular
            | Self::SpecularSoft
            | Self::Rim
            | Self::RimPower
            | Self::Disabled => CelLightingDebugMode::Disabled,
        }
    }
}

include!("cel_lighting_fixture/config.rs");

#[derive(Debug)]
enum FixtureError {
    MissingOutput,
    InvalidMode(String),
    GpuPoll,
    CaptureTimeout,
    PixelValidation,
}

impl fmt::Display for FixtureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOutput => {
                formatter.write_str("usage: cel_lighting_fixture <output.png> <mode>")
            }
            Self::InvalidMode(mode) => write!(formatter, "invalid fixture mode: {mode}"),
            Self::GpuPoll => formatter.write_str("GPU poll failed"),
            Self::CaptureTimeout => formatter.write_str("fixture capture timed out"),
            Self::PixelValidation => formatter.write_str("fixture pixel validation failed"),
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
    valid: bool,
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
    for _ in 0..300 {
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
            return if app.world().resource::<CaptureState>().valid {
                Ok(())
            } else {
                Err(FixtureError::PixelValidation)
            };
        }
    }
    Err(FixtureError::CaptureTimeout)
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Bevy system parameters are values"
)]
fn setup(mut commands: Commands, config: Res<CaptureConfig>, mut params: SetupParams) {
    let target = params.images.add(render_target(SIZE));
    commands.insert_resource(CaptureState {
        target: target.clone(),
        frames: 0,
        requested: false,
        complete: false,
        valid: false,
    });
    let ramp = params.images.add(white_ramp());
    let common = fixture_settings(config.mode);
    commands.spawn((
        Mesh3d(
            params
                .meshes
                .add(hair_strip(matches!(config.mode, FixtureMode::Degenerate))),
        ),
        MeshMaterial3d(
            params
                .materials
                .add(fixture_material(ramp.clone(), common, [1.0, 0.0])),
        ),
        Transform::from_xyz(-1.35, 0.0, 0.0),
    ));
    commands.spawn((
        Mesh3d(params.meshes.add(Sphere::new(1.15))),
        MeshMaterial3d(
            params
                .materials
                .add(fixture_material(ramp, common, [0.0, 1.0])),
        ),
        Transform::from_xyz(1.35, 0.0, 0.0),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 16_000.0,
            ..default()
        },
        Transform::from_xyz(-3.0, 2.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        RenderTarget::Image(target.into()),
        Transform::from_xyz(0.0, 0.0, 7.0).looking_at(Vec3::ZERO, Vec3::Y),
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

#[expect(
    clippy::needless_pass_by_value,
    reason = "Bevy observer parameters are values"
)]
fn mark_capture_complete(capture: On<ScreenshotCaptured>, mut state: ResMut<CaptureState>) {
    let signal = signal_pixel_count(&capture.image);
    state.valid = signal > 100;
    state.complete = true;
}
