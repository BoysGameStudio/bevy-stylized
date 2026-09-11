#![expect(
    clippy::needless_pass_by_value,
    clippy::redundant_pub_crate,
    reason = "Bevy systems consume parameters by value and the parent fixture consumes split-module items"
)]

use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};

use super::{
    config::{FixtureConfig, FixtureError, FixtureMode},
    mark_complete,
};

const CENTER_PROBE: UVec3 = UVec3::new(320, 320, 0);
const EDGE_PROBES: [UVec3; 2] = [UVec3::new(48, 320, 0), UVec3::new(592, 320, 0)];

#[derive(Resource)]
pub(crate) struct CaptureState {
    pub(crate) target: Handle<Image>,
    pub(crate) frames: u32,
    pub(crate) requested: bool,
    pub(crate) complete: bool,
    pub(crate) failure: Option<FixtureError>,
}

pub(crate) fn request_capture(
    mut commands: Commands,
    config: Res<FixtureConfig>,
    mut state: ResMut<CaptureState>,
) {
    state.frames += 1;
    if state.frames < 16 || state.requested {
        return;
    }
    state.requested = true;
    commands
        .spawn(Screenshot::image(state.target.clone()))
        .observe(save_to_disk(config.output.clone()))
        .observe(mark_complete);
}

pub(crate) fn validate_capture(mode: FixtureMode, image: &Image) -> Result<(), FixtureError> {
    let center = pixel_luma(image, CENTER_PROBE)?;
    let edge = EDGE_PROBES
        .into_iter()
        .map(|probe| pixel_luma(image, probe))
        .sum::<Result<f32, FixtureError>>()?
        / 2.0;
    if matches!(mode, FixtureMode::StackOn | FixtureMode::VignetteOff)
        && !(0.54..=0.58).contains(&center)
    {
        return Err(FixtureError::PixelAssertion("vignette center stability"));
    }
    match mode {
        FixtureMode::StackOn => {
            if edge <= center * 0.82 {
                Ok(())
            } else {
                Err(FixtureError::PixelAssertion("vignette attenuation"))
            }
        }
        FixtureMode::VignetteOff => {
            if edge >= center * 0.95 && edge >= 0.4 {
                Ok(())
            } else {
                Err(FixtureError::PixelAssertion("vignette-off edge clarity"))
            }
        }
        FixtureMode::StackOff
        | FixtureMode::BloomOff
        | FixtureMode::ChromaticOff
        | FixtureMode::DistortionOff
        | FixtureMode::MissingPrepass
        | FixtureMode::Malformed => Ok(()),
    }
}

fn pixel_luma(image: &Image, coordinate: UVec3) -> Result<f32, FixtureError> {
    let pixel = image
        .pixel_bytes(coordinate)
        .map_err(|_| FixtureError::PixelAccess)?;
    let [red, green, blue, ..] = pixel else {
        return Err(FixtureError::PixelAccess);
    };
    Ok(0.0722_f32.mul_add(
        f32::from(*blue),
        0.7152_f32.mul_add(f32::from(*green), 0.2126 * f32::from(*red)),
    ) / 255.0)
}
