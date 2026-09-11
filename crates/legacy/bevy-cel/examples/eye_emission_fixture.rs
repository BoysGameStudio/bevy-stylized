//! Deterministic dark-scene task18 eye and HDR emission Vulkan fixture.

use std::{error::Error, fmt, path::PathBuf, time::Duration};

use bevy::{
    prelude::*,
    render::{
        RenderPlugin,
        render_resource::PollType,
        renderer::RenderDevice,
        view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_cel::CharacterMaterialPlugin;

include!("eye_emission_fixture/assets.rs");
include!("eye_emission_fixture/scene.rs");

const SIZE: u32 = 640;

#[derive(Clone, Copy)]
enum FixtureMode {
    On,
    Off,
    Missing,
    Debug,
    Bloom,
}

impl FixtureMode {
    fn parse(value: &str) -> Result<Self, FixtureError> {
        match value {
            "on" => Ok(Self::On),
            "off" => Ok(Self::Off),
            "missing" => Ok(Self::Missing),
            "debug" => Ok(Self::Debug),
            "bloom" => Ok(Self::Bloom),
            _ => Err(FixtureError::InvalidMode(value.to_owned())),
        }
    }

    const fn emits(self) -> bool {
        matches!(self, Self::On | Self::Debug | Self::Bloom)
    }
}

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
            Self::MissingOutput => formatter
                .write_str("usage: eye_emission_fixture <output.png> <on|off|missing|debug|bloom>"),
            Self::InvalidMode(mode) => write!(formatter, "invalid fixture mode: {mode}"),
            Self::GpuPoll => formatter.write_str("GPU poll failed"),
            Self::CaptureTimeout => formatter.write_str("fixture capture timed out"),
            Self::PixelValidation => formatter.write_str("eye/emission pixel validation failed"),
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
    result: Option<bool>,
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
        if let Some(valid) = app.world().resource::<CaptureState>().result {
            return if valid {
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
    reason = "Bevy systems consume parameters by value"
)]
fn request_capture(
    mut commands: Commands,
    config: Res<CaptureConfig>,
    mut state: ResMut<CaptureState>,
) {
    state.frames += 1;
    if state.frames < 10 || state.requested {
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
    reason = "Bevy observers consume parameters by value"
)]
fn mark_capture_complete(
    capture: On<ScreenshotCaptured>,
    config: Res<CaptureConfig>,
    mut state: ResMut<CaptureState>,
) {
    let iris = pixel_luma(&capture.image, SIZE / 2 - 60, SIZE / 2).unwrap_or(0.0);
    let sclera = pixel_luma(&capture.image, SIZE / 2 + 105, SIZE / 2).unwrap_or(0.0);
    let background = pixel_luma(&capture.image, 32, 32).unwrap_or(1.0);
    let eye_is_readable = sclera > background && iris / sclera.max(0.001) > 0.02;
    let emission_is_visible = !config.mode.emits() || iris > sclera;
    let debug_is_visible = !matches!(config.mode, FixtureMode::Debug) || iris > 0.7;
    state.result = Some(eye_is_readable && emission_is_visible && debug_is_visible);
}
