//! Dev-only golden-image harness: headless offscreen capture, pixel diffing,
//! and baseline storage for Bevy renderers.
//!
//! This crate is shared test tooling. Production and legacy crates never
//! depend on it; they use it through dev-dependencies only.

pub mod capture;
pub mod diff;
pub mod golden;
pub mod regions;
pub mod tier2;

pub use capture::{
    CaptureRequest, CaptureStamp, CaptureTarget, CapturedImage, StampedCapture, cancel_capture,
    capture_target_bundle, create_capture_target, depad_rgba8, install, probe_render_resources,
    request_capture, try_take, try_take_stamped,
};
pub use diff::{DiffReport, DiffStats, Tolerance, diff_rgba8, evaluate};
pub use golden::{
    CANONICAL_GOLDEN_PLATFORM, GoldenError, GoldenPaths, GoldenProvenance, current_golden_platform,
    decode_png, encode_png, engine_git_commit, fnv1a64_hex, read_baseline, resolve_baseline_paths,
    resolve_capture_paths, write_baseline,
};
pub use regions::{GoldenRegion, ScreenRect, project_aabb_to_screen, validate_regions};
pub use tier2::{
    Tier2RegionReport, Tier2Report, Tier2Thresholds, evaluate_tier2, evaluate_tier2_region,
};

pub mod readiness;
