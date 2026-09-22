//! Tier-2 跨层对比的实现已提取至 `labs/validation`
//! （docs/showcase/01：纯 CPU 比较算法归 validation，阈值与数值行为不变）。
//!
//! 本模块保留 `&CapturedImage` 签名作为薄适配层：捕获类型在 harness 边界
//! 转换为纯数据 [`ImageView`]，委托 `validation::tier2` 的同一实现。

use validation::ImageView;
use validation::tier2 as metrics;

pub use validation::tier2::{Tier2RegionReport, Tier2Report, Tier2Thresholds};

use crate::capture::CapturedImage;
use crate::diff::DiffError;

/// `CapturedImage::new` 已保证 `width * height * 4` 尺寸不变量，转换不会失败。
fn view(image: &CapturedImage) -> ImageView<'_> {
    ImageView::new(image.width, image.height, &image.data)
        .expect("CapturedImage maintains the width*height*4 size invariant")
}

/// Tier-2 跨层对比。`background` 为两侧归一化后的共同背景色（sRGB 0..=255）。
///
/// 签名与提取前一致；委托 `validation::tier2::evaluate_tier2`。
pub fn evaluate_tier2(
    case: &str,
    a: &CapturedImage,
    b: &CapturedImage,
    background: [u8; 3],
    thresholds: Tier2Thresholds,
) -> Result<Tier2Report, DiffError> {
    metrics::evaluate_tier2(case, view(a), view(b), background, thresholds)
}

/// Region-restricted Tier-2 metrics. `mask` is one byte per pixel (nonzero =
/// in region); its length must match `width * height`.
///
/// 签名与提取前一致；委托 `validation::tier2::evaluate_tier2_region`。
pub fn evaluate_tier2_region(
    case: &str,
    region: &str,
    a: &CapturedImage,
    b: &CapturedImage,
    mask: &[bool],
    thresholds: Tier2Thresholds,
) -> Result<Tier2RegionReport, DiffError> {
    metrics::evaluate_tier2_region(case, region, view(a), view(b), mask, thresholds)
}

#[cfg(test)]
mod tests {
    //! 适配层等价性：同一输入经 `&CapturedImage` 包装与直接视图得到相同报告。
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
        assert_eq!(report.shadow_iou, 1.0);
        assert_eq!(report.silhouette_iou, 1.0);
    }

    #[test]
    fn adapter_matches_direct_view_evaluation() {
        let a = gradient_image(0);
        let mut b = gradient_image(0);
        for chunk in b.data.as_chunks_mut::<4>().0 {
            chunk[0] = 255 - chunk[0];
        }
        let adapted =
            evaluate_tier2("shifted", &a, &b, [0, 0, 0], Tier2Thresholds::default()).expect("diff");
        let direct = metrics::evaluate_tier2(
            "shifted",
            view(&a),
            view(&b),
            [0, 0, 0],
            Tier2Thresholds::default(),
        )
        .expect("diff");
        assert!(!adapted.passed);
        assert_eq!(adapted.max_abs_delta_nonedge, direct.max_abs_delta_nonedge);
        assert_eq!(adapted.mean_abs_delta, direct.mean_abs_delta);
        assert_eq!(adapted.psnr_db, direct.psnr_db);
        assert_eq!(adapted.edge_fraction, direct.edge_fraction);
        assert_eq!(adapted.shadow_iou, direct.shadow_iou);
        assert_eq!(adapted.silhouette_iou, direct.silhouette_iou);
        assert_eq!(adapted.violations, direct.violations);
    }

    #[test]
    fn rejects_equal_byte_counts_with_different_dimensions() {
        let a = CapturedImage::new(2, 4, vec![0; 32]).unwrap();
        let b = CapturedImage::new(4, 2, vec![0; 32]).unwrap();
        assert!(matches!(
            evaluate_tier2("wrong shape", &a, &b, [0; 3], Tier2Thresholds::default()),
            Err(DiffError::DimensionsMismatch { .. })
        ));
    }
}
