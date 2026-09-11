//! Per-pixel image diffing with configurable tolerance for golden compares.

use serde::{Deserialize, Serialize};

/// Aggregated difference between two equally sized RGBA8 images.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiffStats {
    /// Largest absolute per-channel difference across all pixels.
    pub max_channel_delta: u8,
    /// Mean absolute difference across all channels of all pixels.
    pub mean_absolute_delta: f64,
    /// Peak signal-to-noise ratio in dB; infinite when identical.
    pub psnr_db: f64,
    /// Pixels with at least one differing channel.
    pub differing_pixels: usize,
    /// Total pixel count.
    pub total_pixels: usize,
}

/// Tolerance applied to a [`DiffStats`] golden comparison.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tolerance {
    /// Maximum allowed absolute per-channel difference.
    pub max_channel_delta: u8,
    /// Maximum allowed mean absolute per-channel difference.
    pub mean_absolute_delta: f64,
    /// Minimum allowed PSNR in dB.
    pub min_psnr_db: f64,
}

impl Default for Tolerance {
    fn default() -> Self {
        Self {
            max_channel_delta: 4,
            mean_absolute_delta: 0.5,
            min_psnr_db: 40.0,
        }
    }
}

/// Error returned when two images cannot be compared at all.
#[derive(Debug, thiserror::Error)]
pub enum DiffError {
    /// The two images have different byte counts.
    #[error("image size mismatch: baseline {baseline} bytes, candidate {candidate} bytes")]
    SizeMismatch {
        /// Baseline byte count.
        baseline: usize,
        /// Candidate byte count.
        candidate: usize,
    },
}

/// Computes the per-pixel difference between two tightly packed RGBA8 images.
///
/// # Errors
///
/// Returns [`DiffError::SizeMismatch`] when the inputs differ in length.
pub fn diff_rgba8(baseline: &[u8], candidate: &[u8]) -> Result<DiffStats, DiffError> {
    if baseline.len() != candidate.len() {
        return Err(DiffError::SizeMismatch {
            baseline: baseline.len(),
            candidate: candidate.len(),
        });
    }
    let mut max_delta = 0_u8;
    let mut absolute_sum = 0_u64;
    let mut squared_sum = 0_u64;
    let mut differing = 0_usize;
    for (a, b) in baseline
        .as_chunks::<4>()
        .0
        .iter()
        .zip(candidate.as_chunks::<4>().0)
    {
        let mut pixel_differs = false;
        for (x, y) in a.iter().zip(b) {
            let delta = x.abs_diff(*y);
            max_delta = max_delta.max(delta);
            absolute_sum += u64::from(delta);
            squared_sum += u64::from(delta) * u64::from(delta);
            pixel_differs |= delta != 0;
        }
        differing += usize::from(pixel_differs);
    }
    let channels = baseline.len() as f64;
    let mse = squared_sum as f64 / channels.max(1.0);
    let psnr_db = if mse == 0.0 {
        f64::INFINITY
    } else {
        10.0 * (255.0 * 255.0 / mse).log10()
    };
    Ok(DiffStats {
        max_channel_delta: max_delta,
        mean_absolute_delta: absolute_sum as f64 / channels.max(1.0),
        psnr_db,
        differing_pixels: differing,
        total_pixels: baseline.len() / 4,
    })
}

/// Checks the diff against a tolerance, listing every violated criterion.
///
/// # Errors
///
/// Returns the human-readable violations when any criterion fails.
pub fn evaluate(stats: DiffStats, tolerance: Tolerance) -> Result<(), Vec<String>> {
    let mut violations = Vec::new();
    if stats.max_channel_delta > tolerance.max_channel_delta {
        violations.push(format!(
            "max channel delta {} exceeds {}",
            stats.max_channel_delta, tolerance.max_channel_delta
        ));
    }
    if stats.mean_absolute_delta > tolerance.mean_absolute_delta {
        violations.push(format!(
            "mean absolute delta {:.6} exceeds {:.6}",
            stats.mean_absolute_delta, tolerance.mean_absolute_delta
        ));
    }
    if stats.psnr_db < tolerance.min_psnr_db {
        violations.push(format!(
            "PSNR {:.2} dB below {:.2} dB",
            stats.psnr_db, tolerance.min_psnr_db
        ));
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// A serializable per-case diff outcome for CI logs and reports.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiffReport {
    /// Golden case name.
    pub case: String,
    /// Baseline PNG path.
    pub baseline: String,
    /// Measured difference.
    pub stats: DiffStats,
    /// Tolerance applied.
    pub tolerance: Tolerance,
    /// Whether the candidate passed.
    pub passed: bool,
    /// Violated criteria when the candidate failed.
    pub violations: Vec<String>,
}

impl DiffReport {
    /// Builds a report by evaluating stats against a tolerance.
    #[must_use]
    pub fn evaluate(
        case: String,
        baseline: String,
        stats: DiffStats,
        tolerance: Tolerance,
    ) -> Self {
        let (passed, violations) = match evaluate(stats, tolerance) {
            Ok(()) => (true, Vec::new()),
            Err(violations) => (false, violations),
        };
        Self {
            case,
            baseline,
            stats,
            tolerance,
            passed,
            violations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_images_pass_with_zero_delta() {
        // Given
        let image: Vec<u8> = (0..64).map(|i| i * 3).collect();

        // When
        let stats = diff_rgba8(&image, &image).expect("same length");

        // Then
        assert_eq!(stats.max_channel_delta, 0);
        assert_eq!(stats.mean_absolute_delta, 0.0);
        assert!(stats.psnr_db.is_infinite());
        assert_eq!(stats.differing_pixels, 0);
        assert_eq!(stats.total_pixels, 16);
        assert!(evaluate(stats, Tolerance::default()).is_ok());
    }

    #[test]
    fn single_channel_step_is_measured_exactly() {
        // Given a 1x1 image with one channel off by 10
        let baseline = vec![100_u8, 100, 100, 255];
        let candidate = vec![110_u8, 100, 100, 255];

        // When
        let stats = diff_rgba8(&baseline, &candidate).expect("same length");

        // Then
        assert_eq!(stats.max_channel_delta, 10);
        assert!((stats.mean_absolute_delta - 2.5).abs() < f64::EPSILON);
        assert_eq!(stats.differing_pixels, 1);
        // MSE = 100 / 4 = 25 -> PSNR = 10 * log10(255^2 / 25)
        let expected_psnr = 10.0 * (255.0_f64 * 255.0 / 25.0).log10();
        assert!((stats.psnr_db - expected_psnr).abs() < 0.001);
    }

    #[test]
    fn tolerance_violations_are_all_reported() {
        // Given a fully flipped 2x1 image
        let baseline = vec![0_u8; 8];
        let candidate = vec![255_u8; 8];

        // When
        let stats = diff_rgba8(&baseline, &candidate).expect("same length");
        let verdict = evaluate(stats, Tolerance::default());

        // Then
        let violations = verdict.expect_err("must fail every criterion");
        assert_eq!(violations.len(), 3);
        let report = DiffReport::evaluate(
            "case".into(),
            "case.png".into(),
            stats,
            Tolerance::default(),
        );
        assert!(!report.passed);
        assert_eq!(report.violations.len(), 3);
        let json = serde_json::to_string(&report).expect("report serializes");
        assert!(json.contains("\"passed\":false"));
    }

    #[test]
    fn small_drift_within_default_tolerance_passes() {
        // Given a candidate drifting by 1 code on the red channel only
        let baseline: Vec<u8> = (0..64).map(|i| (i * 2) as u8).collect();
        let candidate: Vec<u8> = baseline
            .iter()
            .enumerate()
            .map(|(i, v)| if i % 4 == 0 { v.saturating_add(1) } else { *v })
            .collect();

        // When
        let stats = diff_rgba8(&baseline, &candidate).expect("same length");

        // Then
        assert_eq!(stats.max_channel_delta, 1);
        assert!((stats.mean_absolute_delta - 0.25).abs() < f64::EPSILON);
        assert!(evaluate(stats, Tolerance::default()).is_ok());
    }

    #[test]
    fn mismatched_lengths_are_rejected() {
        let result = diff_rgba8(&[0; 8], &[0; 4]);
        assert!(matches!(result, Err(DiffError::SizeMismatch { .. })));
    }
}
