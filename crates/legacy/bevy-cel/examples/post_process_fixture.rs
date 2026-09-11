//! Deterministic Vulkan fixture for the stylized post-processing stack.
#![expect(
    clippy::needless_pass_by_value,
    reason = "Bevy observer systems consume event and resource parameters by value"
)]

use std::time::Duration;

use bevy::{
    prelude::*,
    render::{
        RenderPlugin, render_resource::PollType, renderer::RenderDevice,
        view::screenshot::ScreenshotCaptured,
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_cel::CharacterMaterialPlugin;

#[path = "post_process_fixture/capture.rs"]
mod capture;
#[path = "post_process_fixture/config.rs"]
mod config;
#[path = "post_process_fixture/scene.rs"]
mod scene;

use capture::{CaptureState, request_capture, validate_capture};
use config::{FixtureConfig, FixtureError};
use scene::{setup, strip_missing_prepasses};

const MAX_FRAMES: u32 = 300;

fn main() -> Result<(), FixtureError> {
    let config = FixtureConfig::parse(std::env::args().skip(1))?;
    let mut app = App::new();
    app.insert_resource(config)
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
        .add_systems(Update, (strip_missing_prepasses, request_capture).chain());
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
        let mut state = app.world_mut().resource_mut::<CaptureState>();
        if let Some(failure) = state.failure.take() {
            return Err(failure);
        }
        if state.complete {
            return Ok(());
        }
    }
    Err(FixtureError::CaptureTimeout)
}

fn mark_complete(
    capture: On<ScreenshotCaptured>,
    config: Res<FixtureConfig>,
    mut state: ResMut<CaptureState>,
) {
    state.failure = validate_capture(config.mode, &capture.image).err();
    state.complete = true;
}
