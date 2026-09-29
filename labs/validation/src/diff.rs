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
    /// Equal byte counts do not imply identical two-dimensional images.
    #[error("image dimensions mismatch: baseline {baseline:?}, candidate {candidate:?}")]
    DimensionsMismatch {
        /// Baseline width and height.
        baseline: [u32; 2],
        /// Candidate width and height.
        candidate: [u32; 2],
    },
    /// The two images have different byte counts.
    #[error("image size mismatch: baseline {baseline} bytes, candidate {candidate} bytes")]
    SizeMismatch {
        /// Baseline byte count.
        baseline: usize,
        /// Candidate byte count.
        candidate: usize,
    },
    /// ROI 矩形越出画幅。
    #[error("region rect {rect:?} exceeds image dimensions {dimensions:?}")]
    RegionOutOfBounds {
        /// 区域矩形（x, y, width, height）。
        rect: [u32; 4],
        /// 画幅尺寸（宽, 高）。
        dimensions: [u32; 2],
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

// ---------------------------------------------------------------------------
// ROI 区域判定（docs/showcase/05 §9）：per-pixel 每通道阈值 + 失败像素比
// 双层门限（Babylon.js/Unity 模式），阈值按区域分别锁定。
// ---------------------------------------------------------------------------

use crate::image::ImageView;

/// 屏幕矩形（纯数据；捕获侧的矩形类型在各自所有者边界转换为本类型）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionRect {
    /// 左边缘。
    pub x: u32,
    /// 上边缘。
    pub y: u32,
    /// 宽。
    pub width: u32,
    /// 高。
    pub height: u32,
}

/// ROI 双层门限。
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegionTolerance {
    /// 失败像素的每通道阈值（任一通道绝对差超过即计为失败像素）。
    pub max_channel_delta: u8,
    /// 失败像素比上限（0..=1）。
    pub max_fail_ratio: f64,
}

/// 区域 diff 统计。
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegionStats {
    /// 区域内最大每通道绝对差（诊断报告用）。
    pub max_channel_delta: u8,
    /// 失败像素数（任一通道绝对差 > 阈值）。
    pub failing_pixels: usize,
    /// 区域总像素数。
    pub total_pixels: usize,
    /// 失败像素比。
    pub fail_ratio: f64,
}

/// 计算两幅等尺寸 RGBA8 图像在 `rect` 区域内的差异统计。
///
/// # Errors
///
/// 图像尺寸不一致或矩形越界时失败。
pub fn diff_rgba8_region(
    baseline: ImageView,
    candidate: ImageView,
    rect: RegionRect,
    tolerance: RegionTolerance,
) -> Result<RegionStats, DiffError> {
    if (baseline.width, baseline.height) != (candidate.width, candidate.height) {
        return Err(DiffError::DimensionsMismatch {
            baseline: [baseline.width, baseline.height],
            candidate: [candidate.width, candidate.height],
        });
    }
    if rect.x + rect.width > baseline.width || rect.y + rect.height > baseline.height {
        return Err(DiffError::RegionOutOfBounds {
            rect: [rect.x, rect.y, rect.width, rect.height],
            dimensions: [baseline.width, baseline.height],
        });
    }
    let mut max_delta = 0_u8;
    let mut failing = 0_usize;
    let total = (rect.width * rect.height) as usize;
    for row in rect.y..rect.y + rect.height {
        let start = ((row * baseline.width + rect.x) * 4) as usize;
        let end = start + (rect.width * 4) as usize;
        for (a, b) in baseline.rgba8[start..end]
            .as_chunks::<4>()
            .0
            .iter()
            .zip(candidate.rgba8[start..end].as_chunks::<4>().0)
        {
            let mut pixel_max = 0_u8;
            for (x, y) in a.iter().zip(b) {
                let delta = x.abs_diff(*y);
                pixel_max = pixel_max.max(delta);
            }
            max_delta = max_delta.max(pixel_max);
            failing += usize::from(pixel_max > tolerance.max_channel_delta);
        }
    }
    Ok(RegionStats {
        max_channel_delta: max_delta,
        failing_pixels: failing,
        total_pixels: total,
        fail_ratio: if total == 0 {
            0.0
        } else {
            failing as f64 / total as f64
        },
    })
}

/// ROI 双层门限判定。
///
/// # Errors
///
/// 返回人可读的违规项（失败像素比超上限）。
pub fn evaluate_region(stats: RegionStats, tolerance: RegionTolerance) -> Result<(), Vec<String>> {
    if stats.fail_ratio > tolerance.max_fail_ratio {
        return Err(vec![format!(
            "region fail ratio {:.6} exceeds {:.6} ({} / {} pixels above channel delta {})",
            stats.fail_ratio,
            tolerance.max_fail_ratio,
            stats.failing_pixels,
            stats.total_pixels,
            tolerance.max_channel_delta
        )]);
    }
    Ok(())
}

#[cfg(test)]
mod region_tests {
    use super::*;
    use crate::image::ImageView;

    /// 4×2 图：左半基线 100，右半 200；candidate 左半同，右半差 delta。
    fn pair(delta: u8) -> (Vec<u8>, Vec<u8>) {
        let mut baseline = Vec::new();
        let mut candidate = Vec::new();
        for _ in 0..8 {
            let v: u8 = if baseline.len() / 4 % 4 < 2 { 100 } else { 200 };
            baseline.extend_from_slice(&[v, v, v, 255]);
        }
        for (i, b) in baseline.iter().enumerate() {
            let x = (i / 4) % 4;
            let drift = if x >= 2 { delta } else { 0 };
            candidate.push((*b).saturating_add(drift));
        }
        (baseline, candidate)
    }

    #[test]
    fn region_diff_measures_only_the_rect() {
        let (baseline, candidate) = pair(10);
        let baseline = ImageView::new(4, 2, &baseline).expect("view");
        let candidate = ImageView::new(4, 2, &candidate).expect("view");
        let tolerance = RegionTolerance {
            max_channel_delta: 4,
            max_fail_ratio: 0.0,
        };
        // 左半（无差异）：零失败像素。
        let left = diff_rgba8_region(
            baseline,
            candidate,
            RegionRect {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            },
            tolerance,
        )
        .expect("diff");
        assert_eq!(left.max_channel_delta, 0);
        assert_eq!(left.fail_ratio, 0.0);
        assert_eq!(left.total_pixels, 4);
        // 右半（全部差 10）：max=10，全失败。
        let right = diff_rgba8_region(
            baseline,
            candidate,
            RegionRect {
                x: 2,
                y: 0,
                width: 2,
                height: 2,
            },
            tolerance,
        )
        .expect("diff");
        assert_eq!(right.max_channel_delta, 10);
        assert_eq!(right.failing_pixels, 4);
        assert_eq!(right.fail_ratio, 1.0);
        assert!(evaluate_region(right, tolerance).is_err());
        // 放宽阈值到 10：无失败像素 → 通过。
        let relaxed = RegionTolerance {
            max_channel_delta: 10,
            max_fail_ratio: 0.0,
        };
        let right_relaxed =
            diff_rgba8_region(baseline, candidate, right_rect(), relaxed).expect("diff");
        assert_eq!(right_relaxed.fail_ratio, 0.0);
        assert_eq!(evaluate_region(right_relaxed, relaxed), Ok(()));
    }

    fn right_rect() -> RegionRect {
        RegionRect {
            x: 2,
            y: 0,
            width: 2,
            height: 2,
        }
    }

    #[test]
    fn region_fail_ratio_gates_sparse_failures() {
        let (baseline, mut candidate) = pair(0);
        // 区域内 4 像素中 1 个超标。
        let index = (1 * 4 + 2) * 4;
        candidate[index] = candidate[index].saturating_add(50);
        let baseline = ImageView::new(4, 2, &baseline).expect("view");
        let candidate = ImageView::new(4, 2, &candidate).expect("view");
        let stats = diff_rgba8_region(
            baseline,
            candidate,
            right_rect(),
            RegionTolerance {
                max_channel_delta: 4,
                max_fail_ratio: 0.0,
            },
        )
        .expect("diff");
        assert_eq!(stats.failing_pixels, 1);
        assert!((stats.fail_ratio - 0.25).abs() < f64::EPSILON);
        // 比例门限 0.25：恰好在界（含）；0.2：拒绝。
        assert_eq!(
            evaluate_region(
                stats,
                RegionTolerance {
                    max_channel_delta: 4,
                    max_fail_ratio: 0.25,
                }
            ),
            Ok(())
        );
        assert!(
            evaluate_region(
                stats,
                RegionTolerance {
                    max_channel_delta: 4,
                    max_fail_ratio: 0.2,
                }
            )
            .is_err()
        );
        // 矩形越界 → 结构化错误。
        assert!(matches!(
            diff_rgba8_region(
                baseline,
                candidate,
                RegionRect {
                    x: 3,
                    y: 0,
                    width: 2,
                    height: 2,
                },
                RegionTolerance {
                    max_channel_delta: 4,
                    max_fail_ratio: 0.0,
                }
            ),
            Err(DiffError::RegionOutOfBounds { .. })
        ));
    }
}
