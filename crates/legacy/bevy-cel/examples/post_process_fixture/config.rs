#![expect(
    clippy::redundant_pub_crate,
    reason = "the parent fixture consumes split-module items"
)]

use std::{error::Error, fmt, path::PathBuf};

use bevy::prelude::Resource;
use bevy_cel::{StylizedPostProcessProfile, StylizedPostProcessSettings};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FixtureMode {
    StackOn,
    StackOff,
    BloomOff,
    ChromaticOff,
    VignetteOff,
    DistortionOff,
    MissingPrepass,
    Malformed,
}

impl FixtureMode {
    fn parse(value: &str) -> Result<Self, FixtureError> {
        match value {
            "stack-on" => Ok(Self::StackOn),
            "stack-off" => Ok(Self::StackOff),
            "bloom-off" => Ok(Self::BloomOff),
            "chromatic-off" => Ok(Self::ChromaticOff),
            "vignette-off" => Ok(Self::VignetteOff),
            "distortion-off" => Ok(Self::DistortionOff),
            "missing-prepass" => Ok(Self::MissingPrepass),
            "malformed" => Ok(Self::Malformed),
            _ => Err(FixtureError::InvalidMode(value.to_owned())),
        }
    }

    pub(crate) fn profile(self) -> StylizedPostProcessProfile {
        let vivid = StylizedPostProcessSettings::default()
            .with_bloom_intensity(0.32)
            .with_vignette(0.8, 0.68, 0.25)
            .with_lens_distortion(0.16)
            .with_chromatic_aberration(0.012);
        let base = StylizedPostProcessProfile::default().with_settings(vivid);
        match self {
            Self::StackOn => base,
            Self::StackOff => base
                .with_hdr(false)
                .with_bloom(false)
                .with_tonemapping(false)
                .with_vignette(false)
                .with_lens_distortion(false)
                .with_chromatic_aberration(false),
            Self::BloomOff => base.with_bloom(false),
            Self::ChromaticOff => base.with_chromatic_aberration(false),
            Self::VignetteOff => base.with_vignette(false),
            Self::DistortionOff => base.with_lens_distortion(false),
            Self::MissingPrepass => base.with_prepass_edges(true),
            Self::Malformed => base.with_settings(
                StylizedPostProcessSettings::default()
                    .with_bloom_intensity(f32::INFINITY)
                    .with_vignette(f32::NAN, -1000.0, f32::INFINITY)
                    .with_lens_distortion(1000.0)
                    .with_chromatic_aberration(f32::NEG_INFINITY),
            ),
        }
    }
}

#[derive(Resource)]
pub(crate) struct FixtureConfig {
    pub(crate) output: PathBuf,
    pub(crate) mode: FixtureMode,
}

impl FixtureConfig {
    pub(crate) fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, FixtureError> {
        let mode = args
            .next()
            .ok_or(FixtureError::MissingMode)
            .and_then(|value| FixtureMode::parse(&value))?;
        let output = args
            .next()
            .map(PathBuf::from)
            .ok_or(FixtureError::MissingOutput)?;
        Ok(Self { output, mode })
    }
}

#[derive(Debug)]
pub(crate) enum FixtureError {
    MissingMode,
    MissingOutput,
    InvalidMode(String),
    GpuPoll,
    CaptureTimeout,
    PixelAccess,
    PixelAssertion(&'static str),
}

impl fmt::Display for FixtureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMode => formatter.write_str("missing fixture mode"),
            Self::MissingOutput => formatter.write_str("missing fixture output path"),
            Self::InvalidMode(mode) => write!(formatter, "invalid fixture mode: {mode}"),
            Self::GpuPoll => formatter.write_str("GPU poll failed"),
            Self::CaptureTimeout => formatter.write_str("fixture capture timed out"),
            Self::PixelAccess => formatter.write_str("fixture pixel probe was unavailable"),
            Self::PixelAssertion(assertion) => {
                write!(formatter, "fixture pixel assertion failed: {assertion}")
            }
        }
    }
}

impl Error for FixtureError {}
