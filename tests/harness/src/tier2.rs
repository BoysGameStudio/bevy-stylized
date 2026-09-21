//! Tier-2 跨层对比指标（docs/reference/08 §3，草案值的执行载体）。
//!
//! 与逐像素 diff 不同，Tier-2 面向"不同渲染器"的跨层对比：
//!
//! - 高频边缘（3×3 梯度超阈值）排除出 per-pixel 上限判定（AA 差异集中区）；
//! - 语义遮罩 IoU：阴影遮罩（低于各自图中位亮度的像素集合）与剪影遮罩
//!   （前景/背景分割）——结构性错误不允许被像素均值掩盖。

use serde::Serialize;

use crate::capture::CapturedImage;

/// Tier-2 判据阈值（docs/reference/08 §3 草案值；实测校准后须回填文档）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tier2Thresholds {
    /// 边缘判定的 3×3 梯度阈值（0..=1）。
    pub edge_gradient: f32,
    /// 非边缘 per-pixel max abs delta（0..=1）。
    pub max_abs_delta_nonedge: f32,
    /// 全图 mean abs delta（0..=1）。
    pub mean_abs_delta: f32,
    /// PSNR 下限（dB）。
    pub min_psnr_db: f32,
    /// 阴影遮罩 IoU 下限。
    pub min_shadow_iou: f32,
    /// 剪影遮罩 IoU 下限。
    pub min_silhouette_iou: f32,
}

impl Default for Tier2Thresholds {
    /// docs/reference/08 §3 草案值。
    fn default() -> Self {
        Self {
            edge_gradient: 32.0 / 255.0,
            max_abs_delta_nonedge: 16.0 / 255.0,
            mean_abs_delta: 4.0 / 255.0,
            min_psnr_db: 30.0,
            min_shadow_iou: 0.95,
            min_silhouette_iou: 0.99,
        }
    }
}

/// Tier-2 对比报告（全部为实测值）。
#[derive(Clone, Debug, Serialize)]
pub struct Tier2Report {
    /// 用例名。
    pub case: String,
    /// 非边缘像素的最大通道差（0..=1）。
    pub max_abs_delta_nonedge: f32,
    /// 全图 mean abs delta（0..=1）。
    pub mean_abs_delta: f32,
    /// PSNR（dB）。
    pub psnr_db: f32,
    /// 边缘像素占比（被排除出 max 判定的比例）。
    pub edge_fraction: f32,
    /// 阴影遮罩 IoU。
    pub shadow_iou: f32,
    /// 剪影遮罩 IoU。
    pub silhouette_iou: f32,
    /// 使用的阈值。
    #[serde(skip)]
    pub thresholds: Tier2Thresholds,
    /// 是否通过全部判据。
    pub passed: bool,
    /// 未通过的判据描述。
    pub violations: Vec<String>,
}

fn luminance(pixel: &[u8]) -> f32 {
    let r = f32::from(pixel[0]) / 255.0;
    let g = f32::from(pixel[1]) / 255.0;
    let b = f32::from(pixel[2]) / 255.0;
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn pixel(data: &[u8], width: usize, x: usize, y: usize) -> [u8; 3] {
    let offset = (y * width + x) * 4;
    [data[offset], data[offset + 1], data[offset + 2]]
}

/// 3×3 邻域最大梯度（亮度差）。
fn is_edge(data: &[u8], width: usize, height: usize, x: usize, y: usize, threshold: f32) -> bool {
    let center = luminance(&pixel(data, width, x, y));
    let mut max_gradient = 0.0_f32;
    for dy in -1_i32..=1 {
        for dx in -1_i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x.saturating_add_signed(dx as isize).min(width - 1);
            let ny = y.saturating_add_signed(dy as isize).min(height - 1);
            max_gradient =
                max_gradient.max((luminance(&pixel(data, width, nx, ny)) - center).abs());
        }
    }
    max_gradient > threshold
}

/// 遮罩 IoU（intersection-over-union）。
fn mask_iou(a: &[bool], b: &[bool]) -> f32 {
    let (mut intersection, mut union) = (0_usize, 0_usize);
    for (x, y) in a.iter().zip(b) {
        intersection += usize::from(*x && *y);
        union += usize::from(*x || *y);
    }
    if union == 0 {
        1.0
    } else {
        intersection as f32 / union as f32
    }
}

/// 中位亮度（用于阴影遮罩分割）。
fn median_luminance(data: &[u8], width: usize, height: usize) -> f32 {
    let mut histogram = [0_u32; 256];
    for y in 0..height {
        for x in 0..width {
            let bucket = (luminance(&pixel(data, width, x, y)) * 255.0) as usize;
            histogram[bucket.min(255)] += 1;
        }
    }
    let total = (width * height) as u32;
    let mut cumulative = 0_u32;
    for (bucket, count) in histogram.iter().enumerate() {
        cumulative += count;
        if cumulative * 2 >= total {
            return bucket as f32 / 255.0;
        }
    }
    0.5
}

/// Tier-2 跨层对比。`background` 为两侧归一化后的共同背景色（sRGB 0..=255）。
pub fn evaluate_tier2(
    case: &str,
    a: &CapturedImage,
    b: &CapturedImage,
    background: [u8; 3],
    thresholds: Tier2Thresholds,
) -> Result<Tier2Report, crate::diff::DiffError> {
    if a.width != b.width || a.height != b.height {
        return Err(crate::diff::DiffError::DimensionsMismatch {
            baseline: [a.width, a.height],
            candidate: [b.width, b.height],
        });
    }
    let stats = crate::diff::diff_rgba8(&a.data, &b.data)?;
    let (width, height) = (a.width as usize, a.height as usize);
    let total = width * height;

    let mut edge_count = 0_usize;
    let mut max_nonedge = 0.0_f32;
    for y in 0..height {
        for x in 0..width {
            let pa = pixel(&a.data, width, x, y);
            let pb = pixel(&b.data, width, x, y);
            let delta = (0..3)
                .map(|c| (f32::from(pa[c]) - f32::from(pb[c])).abs() / 255.0)
                .fold(0.0_f32, f32::max);
            if is_edge(&a.data, width, height, x, y, thresholds.edge_gradient)
                || is_edge(&b.data, width, height, x, y, thresholds.edge_gradient)
            {
                edge_count += 1;
            } else {
                max_nonedge = max_nonedge.max(delta);
            }
        }
    }

    // 阴影遮罩：低于各自中位亮度的像素集合（各自归一，结构性对比）。
    let median_a = median_luminance(&a.data, width, height);
    let median_b = median_luminance(&b.data, width, height);
    let shadow_a: Vec<bool> = (0..total)
        .map(|i| luminance(&a.data[i * 4..]) < median_a)
        .collect();
    let shadow_b: Vec<bool> = (0..total)
        .map(|i| luminance(&b.data[i * 4..]) < median_b)
        .collect();

    // 剪影遮罩：与背景色差超阈值的像素为前景。
    let silhouette = |data: &[u8]| {
        (0..total)
            .map(|i| {
                let p = pixel(data, width, i % width, i / width);
                (0..3)
                    .map(|c| (f32::from(p[c]) - f32::from(background[c])).abs())
                    .fold(0.0_f32, f32::max)
                    > 8.0
            })
            .collect::<Vec<bool>>()
    };
    let silhouette_a = silhouette(&a.data);
    let silhouette_b = silhouette(&b.data);

    let mut report = Tier2Report {
        case: case.to_owned(),
        max_abs_delta_nonedge: max_nonedge,
        mean_abs_delta: stats.mean_absolute_delta as f32 / 255.0,
        psnr_db: stats.psnr_db as f32,
        edge_fraction: edge_count as f32 / total as f32,
        shadow_iou: mask_iou(&shadow_a, &shadow_b),
        silhouette_iou: mask_iou(&silhouette_a, &silhouette_b),
        thresholds,
        passed: true,
        violations: Vec::new(),
    };
    if report.max_abs_delta_nonedge > thresholds.max_abs_delta_nonedge {
        report.violations.push(format!(
            "non-edge max delta {:.4} exceeds {:.4}",
            report.max_abs_delta_nonedge, thresholds.max_abs_delta_nonedge
        ));
    }
    if report.mean_abs_delta > thresholds.mean_abs_delta {
        report.violations.push(format!(
            "mean abs delta {:.4} exceeds {:.4}",
            report.mean_abs_delta, thresholds.mean_abs_delta
        ));
    }
    if report.psnr_db < thresholds.min_psnr_db {
        report.violations.push(format!(
            "PSNR {:.2}dB below {:.2}dB",
            report.psnr_db, thresholds.min_psnr_db
        ));
    }
    if report.shadow_iou < thresholds.min_shadow_iou {
        report.violations.push(format!(
            "shadow mask IoU {:.4} below {:.4}",
            report.shadow_iou, thresholds.min_shadow_iou
        ));
    }
    if report.silhouette_iou < thresholds.min_silhouette_iou {
        report.violations.push(format!(
            "silhouette mask IoU {:.4} below {:.4}",
            report.silhouette_iou, thresholds.min_silhouette_iou
        ));
    }
    report.passed = report.violations.is_empty();
    Ok(report)
}

/// Region-restricted report (Tenacity migration critical regions). The mask
/// comes from renderer-produced isolation captures; metrics reuse the same
/// per-pixel delta and edge exclusion as [`evaluate_tier2`], restricted to
/// in-region pixels. Shadow/silhouette IoU stay global-only checks.
#[derive(Clone, Debug, Serialize)]
pub struct Tier2RegionReport {
    /// `case@region` identity.
    pub case: String,
    /// In-region pixel count.
    pub pixels: usize,
    /// 非边缘区域像素的最大通道差（0..=1）。
    pub max_abs_delta_nonedge: f32,
    /// 区域 mean abs delta（0..=1）。
    pub mean_abs_delta: f32,
    /// 区域 PSNR（dB）。
    pub psnr_db: f32,
    /// 使用的阈值。
    #[serde(skip)]
    pub thresholds: Tier2Thresholds,
    /// 是否通过全部区域判据。
    pub passed: bool,
    /// 未通过的判据描述。
    pub violations: Vec<String>,
}

/// Region-restricted Tier-2 metrics. `mask` is one byte per pixel (nonzero =
/// in region); its length must match `width * height`.
pub fn evaluate_tier2_region(
    case: &str,
    region: &str,
    a: &CapturedImage,
    b: &CapturedImage,
    mask: &[bool],
    thresholds: Tier2Thresholds,
) -> Result<Tier2RegionReport, crate::diff::DiffError> {
    if a.width != b.width || a.height != b.height {
        return Err(crate::diff::DiffError::DimensionsMismatch {
            baseline: [a.width, a.height],
            candidate: [b.width, b.height],
        });
    }
    let (width, height) = (a.width as usize, a.height as usize);
    if mask.len() != width * height {
        return Err(crate::diff::DiffError::SizeMismatch {
            baseline: width * height,
            candidate: mask.len(),
        });
    }
    let mut pixels = 0_usize;
    let mut max_nonedge = 0.0_f32;
    let mut sum = 0.0_f64;
    let mut squared = 0.0_f64;
    for y in 0..height {
        for x in 0..width {
            if !mask[y * width + x] {
                continue;
            }
            pixels += 1;
            let pa = pixel(&a.data, width, x, y);
            let pb = pixel(&b.data, width, x, y);
            let delta = (0..3)
                .map(|c| (f32::from(pa[c]) - f32::from(pb[c])).abs() / 255.0)
                .fold(0.0_f32, f32::max);
            sum += f64::from(delta);
            squared += f64::from(delta * delta);
            if !(is_edge(&a.data, width, height, x, y, thresholds.edge_gradient)
                || is_edge(&b.data, width, height, x, y, thresholds.edge_gradient))
            {
                max_nonedge = max_nonedge.max(delta);
            }
        }
    }
    if pixels == 0 {
        return Err(crate::diff::DiffError::SizeMismatch {
            baseline: 1,
            candidate: 0,
        });
    }
    let mean = (sum / pixels as f64) as f32;
    let mse = squared / pixels as f64;
    let psnr = if mse > 0.0 {
        (10.0 * (1.0 / mse).log10()) as f32
    } else {
        f32::INFINITY
    };
    let mut report = Tier2RegionReport {
        case: format!("{case}@{region}"),
        pixels,
        max_abs_delta_nonedge: max_nonedge,
        mean_abs_delta: mean,
        psnr_db: psnr,
        thresholds,
        passed: true,
        violations: Vec::new(),
    };
    if report.max_abs_delta_nonedge > thresholds.max_abs_delta_nonedge {
        report.violations.push(format!(
            "non-edge max delta {:.4} exceeds {:.4}",
            report.max_abs_delta_nonedge, thresholds.max_abs_delta_nonedge
        ));
    }
    if report.mean_abs_delta > thresholds.mean_abs_delta {
        report.violations.push(format!(
            "mean abs delta {:.4} exceeds {:.4}",
            report.mean_abs_delta, thresholds.mean_abs_delta
        ));
    }
    if report.psnr_db < thresholds.min_psnr_db {
        report.violations.push(format!(
            "PSNR {:.2}dB below {:.2}dB",
            report.psnr_db, thresholds.min_psnr_db
        ));
    }
    report.passed = report.violations.is_empty();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient_image(seed: u8) -> CapturedImage {
        let mut data = Vec::new();
        for y in 0..64_u8 {
            for x in 0..64_u8 {
                data.extend_from_slice(&[x.wrapping_add(seed) * 3, y * 3, 128, 255]);
            }
        }
        CapturedImage::new(64, 64, data).expect("size")
    }

    #[test]
    fn identical_images_pass_tier2() {
        let a = gradient_image(0);
        let report =
            evaluate_tier2("self", &a, &a, [0, 0, 0], Tier2Thresholds::default()).expect("diff");
        assert!(report.passed, "{:?}", report.violations);
        assert_eq!(report.max_abs_delta_nonedge, 0.0);
        assert_eq!(report.shadow_iou, 1.0);
        assert_eq!(report.silhouette_iou, 1.0);
    }

    #[test]
    fn large_shift_fails_tier2() {
        let a = gradient_image(0);
        let mut b = gradient_image(0);
        for chunk in b.data.as_chunks_mut::<4>().0 {
            chunk[0] = 255 - chunk[0];
        }
        let report =
            evaluate_tier2("shifted", &a, &b, [0, 0, 0], Tier2Thresholds::default()).expect("diff");
        assert!(!report.passed);
        assert!(!report.violations.is_empty());
    }

    #[test]
    fn region_metrics_cover_only_masked_pixels() {
        let a = gradient_image(0);
        let mut b = gradient_image(0);
        // Flip the red channel outside the masked top-left quadrant.
        for (index, chunk) in b.data.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let (x, y) = (index % 64, index / 64);
            if x >= 32 || y >= 32 {
                chunk[0] = 255 - chunk[0];
            }
        }
        let global =
            evaluate_tier2("global", &a, &b, [0, 0, 0], Tier2Thresholds::default()).expect("diff");
        assert!(!global.passed);
        let mask: Vec<bool> = (0..64 * 64).map(|i| i % 64 < 32 && i / 64 < 32).collect();
        let region = evaluate_tier2_region(
            "global",
            "quadrant",
            &a,
            &b,
            &mask,
            Tier2Thresholds::default(),
        )
        .expect("region");
        assert!(region.passed, "{:?}", region.violations);
        assert_eq!(region.pixels, 32 * 32);
        assert!(
            evaluate_tier2_region(
                "global",
                "empty",
                &a,
                &b,
                &vec![false; 64 * 64],
                Tier2Thresholds::default()
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod dimension_regression {
    use super::*;
    #[test]
    fn rejects_equal_byte_counts_with_different_dimensions() {
        let a = CapturedImage::new(2, 4, vec![0; 32]).unwrap();
        let b = CapturedImage::new(4, 2, vec![0; 32]).unwrap();
        assert!(matches!(
            evaluate_tier2("wrong shape", &a, &b, [0; 3], Tier2Thresholds::default()),
            Err(crate::diff::DiffError::DimensionsMismatch { .. })
        ));
    }
}
