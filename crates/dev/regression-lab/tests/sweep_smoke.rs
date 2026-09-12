//! regression-lab smoke 测试（GPU-gated，无 GPU graceful skip）。
//!
//! 完整 sweep 由 CLI（`cargo run -p regression-lab`）执行；测试只真实
//! 测量 1 个代表性 cell（crowd/n100/512²/portable/day——golden 子集
//! 之一不取，避免污染 fixtures；用临时目录），验证测量链路 + 报告
//! 结构。gate 语义由 lib 单测覆盖（无 GPU）。

use bevy_stylized::StylizedExecutionTier;
use regression_lab::{REGRESSION_THRESHOLD, SweepCell, measure_cell};

#[test]
fn sweep_smoke_single_cell_measures() {
    let Some(resources) = golden_harness::probe_render_resources() else {
        eprintln!("regression-lab smoke: no GPU adapter available; skipping");
        return;
    };
    let dir = std::env::temp_dir().join("regression-lab-smoke");
    let _ = std::fs::remove_dir_all(&dir);

    let outcome = measure_cell(
        &resources,
        "crowd",
        100,
        "static",
        [512, 512],
        StylizedExecutionTier::Portable,
        "day",
        &dir,
    );
    // 管线真实出图：帧时为正且有限；解析档在本机 = portable。
    assert!(outcome.stats.mean_ms > 0.0 && outcome.stats.mean_ms.is_finite());
    assert!(outcome.stats.p95_ms >= outcome.stats.min_ms, "统计有序性");
    assert_eq!(outcome.resolved_tier, "portable");
    eprintln!(
        "[smoke] crowd/n100/512x512/portable/day: mean={:.3}ms p95={:.3}ms",
        outcome.stats.mean_ms, outcome.stats.p95_ms
    );

    // 阈值常量约定 + SweepCell serde roundtrip（基线可读性是 gate 前提）。
    assert!((REGRESSION_THRESHOLD - 0.10).abs() < f64::EPSILON);
    let cell = SweepCell {
        target_id: "crowd/n100/512x512/portable/day".into(),
        scene: "crowd".into(),
        count: 100,
        pose: "static".into(),
        resolution: [512, 512],
        requested_tier: "portable".into(),
        resolved_tier: outcome.resolved_tier,
        lighting: "day".into(),
        backend: "vulkan".into(),
        class: "Cel".into(),
        frame_stats: outcome.stats,
        golden: outcome.golden,
        gpu_timing: None,
    };
    let json = serde_json::to_string(&cell).expect("serialize");
    let back: SweepCell = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.target_id, cell.target_id);
    assert_eq!(back.frame_stats.mean_ms, cell.frame_stats.mean_ms);
}
