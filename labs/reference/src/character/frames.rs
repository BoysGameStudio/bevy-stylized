//! Readiness, bounded waits and readback; no qualification thresholds.
use bevy::{
    prelude::*,
    render::{RenderApp, render_resource::PipelineCache},
};
use golden_harness::{CapturedImage, capture};
use std::time::{Duration, Instant};

pub(super) fn pipelines_ready(app: &App) -> Result<bool, String> {
    golden_harness::readiness::pipeline_status(
        app.sub_app(RenderApp).world().resource::<PipelineCache>(),
    )
}
pub(super) fn frame(
    app: &mut App,
    target: &capture::CaptureTarget,
) -> Result<CapturedImage, String> {
    let req = capture::request_capture(app.world_mut(), target.clone());
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(30) {
        app.update();
        pipelines_ready(app)?;
        if let Some(image) = capture::try_take(app.world_mut(), req.clone()) {
            return Ok(image);
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Err("capture timeout".into())
}
pub(super) fn readiness_timeout() -> Duration {
    Duration::from_millis(
        std::env::var("STYLIZED_REFERENCE_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60_000),
    )
}

pub(super) fn frame_linear(
    app: &mut App,
    target: &capture::CaptureTarget,
    scale: u32,
) -> Result<CapturedImage, String> {
    let request = capture::request_capture(app.world_mut(), target.clone());
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(60) {
        app.update();
        pipelines_ready(app)?;
        if let Some(image) =
            capture::try_take_linear_resolved(app.world_mut(), request.clone(), scale)?
        {
            return Ok(image);
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Err("linear capture timeout".into())
}
