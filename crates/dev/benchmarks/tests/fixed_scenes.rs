//! Benchmark 集成测试（GPU-gated，无 GPU graceful skip）。
//!
//! 默认运行 4 个固定场景并打印表格；写 baseline 到
//! `docs/performance/baselines/` 仅在显式请求时：
//! `BENCH_WRITE_BASELINE=1 cargo test -p benchmarks -- --nocapture`（写入
//! `linux-vulkan-rtx5090-<date>.json`，可入库——纯测量数据，无美术资产）。

use benchmarks::run_benchmarks;

#[test]
fn fixed_benchmark_scenes_run() {
    let Some(report) = run_benchmarks() else {
        eprintln!("benchmarks: no GPU adapter available; skipping");
        return;
    };
    eprintln!(
        "benchmark on {} ({}), commit {}",
        report.gpu, report.driver, report.engine_git_commit
    );
    for record in &report.records {
        eprintln!(
            "[bench] {}: mean={:.3}ms median={:.3}ms p95={:.3}ms (min {:.3} / max {:.3})",
            record.target_id,
            record.frame_stats.mean_ms,
            record.frame_stats.median_ms,
            record.frame_stats.p95_ms,
            record.frame_stats.min_ms,
            record.frame_stats.max_ms,
        );
    }
    assert_eq!(report.records.len(), 4);

    if std::env::var("BENCH_WRITE_BASELINE").as_deref() == Ok("1") {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/performance/baselines");
        std::fs::create_dir_all(&dir).expect("create baselines dir");
        let date = {
            // YYYY-MM-DD（UTC，来自 Unix 秒的手算）。
            let secs = report.captured_unix_seconds;
            let days = secs / 86400;
            let (y, m, d) = civil_from_days(days as i64);
            format!("{y:04}-{m:02}-{d:02}")
        };
        let path = dir.join(format!("linux-vulkan-rtx5090-{date}.json"));
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&report).expect("serialize"),
        )
        .expect("write baseline");
        eprintln!("[bench] baseline written: {}", path.display());
    }
}

/// Howard Hinnant 的 civil-from-days 算法。
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
