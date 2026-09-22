//! 固定输入 → 期望输出向量：把 diff/Tier-2 的数值行为钉死在字面值上，
//! 使行为独立于"代码从 tests/harness 迁入"这一事实被验证。
//!
//! 期望值来源：迁移前由 `tests/harness` 原实现计算（scratch 集成测试打印，
//! 已删除），再以字面值记录于此；与迁移后实现逐位比对。f32/f64 均为
//! Debug 往返精确值。

use validation::diff::{DiffStats, diff_rgba8};
use validation::image::ImageView;
use validation::tier2::{Tier2Thresholds, evaluate_tier2, evaluate_tier2_region};

fn v1_pair() -> (Vec<u8>, Vec<u8>) {
    let baseline: Vec<u8> = (0..32_u8)
        .map(|i| i.wrapping_mul(7).wrapping_add(13))
        .collect();
    let mut candidate = baseline.clone();
    // pixel (1,0): red +40
    candidate[4] = candidate[4].wrapping_add(40);
    // pixel (3,1): all channels to 0
    candidate[28..32].fill(0);
    (baseline, candidate)
}

#[test]
fn v1_diff_stats_match_pre_move_literals() {
    let (baseline, candidate) = v1_pair();
    let stats = diff_rgba8(&baseline, &candidate).expect("same length");
    assert_eq!(
        stats,
        DiffStats {
            max_channel_delta: 230,
            mean_absolute_delta: 28.6875,
            psnr_db: 10.291633886741597,
            differing_pixels: 2,
            total_pixels: 8,
        }
    );
}

fn v2_pair() -> (Vec<u8>, Vec<u8>) {
    let mut data = Vec::new();
    for y in 0..16_u8 {
        for x in 0..16_u8 {
            data.extend_from_slice(&[x * 8, y * 8, 100, 255]);
        }
    }
    let mut changed = data.clone();
    // 2x2 block at (4..6, 9..11): green -10 (moderate local change)
    for y in 9..11_usize {
        for x in 4..6_usize {
            let offset = (y * 16 + x) * 4;
            changed[offset + 1] = changed[offset + 1].saturating_sub(10);
        }
    }
    (data, changed)
}

fn view16(data: &[u8]) -> ImageView<'_> {
    ImageView::new(16, 16, data).expect("size")
}

#[test]
fn v2_tier2_report_matches_pre_move_literals() {
    let (a, b) = v2_pair();
    let report = evaluate_tier2(
        "v2",
        view16(&a),
        view16(&b),
        [0, 0, 0],
        Tier2Thresholds::default(),
    )
    .expect("diff");
    assert_eq!(report.max_abs_delta_nonedge, 0.039215688);
    assert_eq!(report.mean_abs_delta, 0.00015318628);
    assert_eq!(report.psnr_db, 52.213203);
    assert_eq!(report.edge_fraction, 0.0);
    assert_eq!(report.shadow_iou, 0.9609375);
    assert_eq!(report.silhouette_iou, 1.0);
    assert!(report.passed);
    assert!(report.violations.is_empty());
}

#[test]
fn v3_region_report_matches_pre_move_literals() {
    let (a, b) = v2_pair();
    let mask: Vec<bool> = (0..256).map(|i| i % 16 < 8).collect();
    let region = evaluate_tier2_region(
        "v2",
        "left",
        view16(&a),
        view16(&b),
        &mask,
        Tier2Thresholds::default(),
    )
    .expect("region");
    assert_eq!(region.pixels, 128);
    assert_eq!(region.max_abs_delta_nonedge, 0.039215688);
    assert_eq!(region.mean_abs_delta, 0.0012254902);
    assert_eq!(region.psnr_db, 43.182304);
    assert!(region.passed);
    assert!(region.violations.is_empty());
}
