//! regression-lab CLI（V2-g）。
//!
//! 用法：
//!
//! ```text
//! cargo run -p regression-lab -- [--out FILE] [--gate BASELINE.json] [--golden-dir DIR]
//! ```
//!
//! - 默认：跑完整 sweep，JSON 写 stdout 摘要 +
//!   `docs/performance/baselines/sweep-<gpu>-<date>.json`（--out 覆盖）；
//! - `--gate`：对基线执行回归门禁（mean 帧时 >10% 劣化 → exit 1）；
//! - 无 GPU：打印 skip 并 exit 0（graceful skip 约定）。

use regression_lab::{BaselineReport, gate_against, run_sweep};

fn main() {
    let mut out: Option<String> = None;
    let mut gate: Option<String> = None;
    let mut golden_dir = "fixtures/golden/regression-lab".to_string();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => out = args.next(),
            "--gate" => gate = args.next(),
            "--golden-dir" => {
                golden_dir = args.next().expect("--golden-dir needs a value");
            }
            other => {
                eprintln!("unknown arg {other}");
                std::process::exit(2);
            }
        }
    }

    let Some((mut report, _pngs)) = run_sweep(std::path::Path::new(&golden_dir)) else {
        eprintln!("regression-lab: no GPU adapter available; skipping");
        return;
    };

    // 汇总表（stdout）。
    println!(
        "| {:<44} | {:>9} | {:>9} | {:>9} | {:<9} |",
        "cell", "mean ms", "p95 ms", "resolved", "golden"
    );
    println!("|---|---|---|---|---|");
    for cell in &report.cells {
        println!(
            "| {:<44} | {:>9.3} | {:>9.3} | {:>9} | {:<9} |",
            cell.target_id,
            cell.frame_stats.mean_ms,
            cell.frame_stats.p95_ms,
            cell.resolved_tier,
            cell.golden
                .as_ref()
                .map_or("-".to_string(), |g| g.status.clone()),
        );
    }

    // Gate。
    let mut exit_code = 0;
    if let Some(baseline_path) = &gate {
        let baseline: BaselineReport =
            serde_json::from_str(&std::fs::read_to_string(baseline_path).expect("read baseline"))
                .expect("parse baseline");
        let (failed, mut outcome) = gate_against(&baseline, &report);
        outcome.baseline = baseline_path.clone();
        if failed {
            eprintln!("[gate] REGRESSIONS:");
            for failure in &outcome.regressions {
                eprintln!(
                    "  {} : {:.3}ms → {:.3}ms (+{:.1}%)",
                    failure.target_id,
                    failure.baseline_mean_ms,
                    failure.current_mean_ms,
                    failure.delta_ratio * 100.0
                );
            }
            exit_code = 1;
        } else {
            eprintln!(
                "[gate] pass ({} cells within +{:.0}% threshold; {} missing)",
                report.cells.len() - outcome.missing_cells.len(),
                outcome.threshold * 100.0,
                outcome.missing_cells.len()
            );
        }
        report.gate = Some(outcome);
    }

    // 输出 JSON。
    let out_path = out.unwrap_or_else(|| {
        let date = regression_lab::civil_date_from_unix(report.captured_unix_seconds);
        format!(
            "docs/performance/baselines/sweep-{}-{date}.json",
            report
                .gpu
                .to_lowercase()
                .replace([' ', '(', ')'], "-")
                .trim_end_matches('-')
        )
    });
    std::fs::create_dir_all(
        std::path::Path::new(&out_path)
            .parent()
            .expect("out path has parent"),
    )
    .expect("create out dir");
    std::fs::write(
        &out_path,
        serde_json::to_string_pretty(&report).expect("serialize"),
    )
    .expect("write report");
    eprintln!("[written] {out_path}");
    std::process::exit(exit_code);
}
