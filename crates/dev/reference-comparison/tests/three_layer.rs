//! 三层 Golden 对比测试（GPU-gated；无 GPU 或 upstream golden 缺失时
//! graceful skip）。
//!
//! 对比结果是 **REPORT**（provisional，pending ADR-0007）：写报告并打印
//! 判据结果，不因未过草案判据而失败（不放宽容差，如实记录）。

use reference_comparison::run_three_layer;

#[test]
fn three_layer_golden_comparison_reports() {
    if !std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/golden-local/upstream/gacha_upstream.png")
        .is_file()
    {
        eprintln!("three-layer: upstream golden missing; skipping");
        return;
    }
    let Some(reports) = run_three_layer() else {
        eprintln!("three-layer: no GPU adapter available; skipping");
        return;
    };
    assert_eq!(reports.len(), 3);
    // 报告已写盘；判据通过与否都如实打印（见 lib.rs eprintln 输出）。
}
