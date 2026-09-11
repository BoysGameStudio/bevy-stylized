use std::{error::Error, fmt, path::PathBuf};

use bevy::prelude::{Handle, Image, Resource};
use bevy_cel::{EyeEmissionSettings, OutlineSettings};

#[derive(Clone, Copy, Resource)]
#[expect(
    clippy::redundant_pub_crate,
    reason = "the parent fixture module consumes this split-module type"
)]
pub(crate) enum FixtureMode {
    Width,
    Deformed,
    Exclusion,
    Malformed,
    Accent,
}

impl FixtureMode {
    pub(crate) fn parse(value: &str) -> Result<Self, FixtureError> {
        match value {
            "width" => Ok(Self::Width),
            "deformed" => Ok(Self::Deformed),
            "exclusion" => Ok(Self::Exclusion),
            "malformed" => Ok(Self::Malformed),
            "accent" => Ok(Self::Accent),
            _ => Err(FixtureError::InvalidMode(value.to_owned())),
        }
    }

    pub(crate) fn apply_exclusion(self, x: f32, mut settings: OutlineSettings) -> OutlineSettings {
        if matches!(self, Self::Exclusion) && x.abs() < f32::EPSILON {
            settings = settings.with_exclusion_masks(0.5, 0.0);
        } else if matches!(self, Self::Exclusion) && x > 0.0 {
            settings = settings.with_task18_suppression(
                &EyeEmissionSettings::default()
                    .with_eye_class(1.0, 1.0)
                    .with_masks(0.0, 1.0),
            );
        }
        settings
    }
}

#[derive(Debug)]
#[expect(
    clippy::redundant_pub_crate,
    reason = "the parent fixture module consumes this split-module type"
)]
pub(crate) enum FixtureError {
    MissingMode,
    MissingOutput,
    InvalidMode(String),
    GpuPoll,
    CaptureTimeout,
}

impl fmt::Display for FixtureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMode => formatter.write_str("missing fixture mode"),
            Self::MissingOutput => formatter.write_str("missing output path"),
            Self::InvalidMode(mode) => write!(formatter, "invalid fixture mode: {mode}"),
            Self::GpuPoll => formatter.write_str("GPU poll failed"),
            Self::CaptureTimeout => formatter.write_str("fixture capture timed out"),
        }
    }
}

impl Error for FixtureError {}

#[derive(Resource)]
#[expect(
    clippy::redundant_pub_crate,
    reason = "the parent fixture module consumes this split-module resource"
)]
pub(crate) struct OutputPath(pub(crate) PathBuf);

#[derive(Resource)]
#[expect(
    clippy::redundant_pub_crate,
    reason = "the parent fixture module consumes this split-module resource"
)]
pub(crate) struct CaptureState {
    pub(crate) output: PathBuf,
    pub(crate) target: Handle<Image>,
    pub(crate) ready_frames: u32,
    pub(crate) requested: bool,
    pub(crate) complete: bool,
}
