//! regression-lab CLI（V2-g）。
//!
//! 用法：
//!
//! ```text
//! cargo run -p regression-lab -- [--required] [--out FILE] [--gate BASELINE.json] [--golden-dir DIR]
//! ```
//!
//! - 默认：跑完整 sweep，JSON 写 stdout 摘要 +
//!   `target/stylized/sweep-<timestamp>.json`（--out 指定新路径）；
//! - `--gate`：对基线执行回归门禁（mean 帧时 >10% 劣化 → exit 1）；
//! - 无 GPU：required 模式记录 BLOCKED 并 exit 2；普通探索模式可以 skip。

use regression_lab::{BaselineReport, gate_against, run_sweep, validate_required_sweep};

fn main() {
    let mut out: Option<String> = None;
    let mut gate: Option<String> = None;
    let mut required = false;
    let mut golden_dir = "fixtures/golden/regression-lab".to_string();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--required" => required = true,
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

    if required && out.is_none() {
        eprintln!("--required needs an explicit new --out report path");
        std::process::exit(2);
    }
    if out
        .as_ref()
        .is_some_and(|p| std::path::Path::new(p).exists())
    {
        eprintln!("output exists; use a new candidate report");
        std::process::exit(1);
    }

    let Some((mut report, pngs)) = run_sweep(std::path::Path::new(&golden_dir)) else {
        if required {
            let path = out.expect("required output checked");
            write_report(
                &path,
                &serde_json::json!({"schema":regression_lab::SWEEP_SCHEMA,"status":"BLOCKED","collection_status":"BLOCKED","reason":"required GPU unavailable"}),
            );
            eprintln!("BLOCKED: required GPU unavailable");
            std::process::exit(2);
        }
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
    let collection_errors = if required {
        validate_required_sweep(&report)
    } else {
        Vec::new()
    };
    let mut exit_code = i32::from(!collection_errors.is_empty());
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
        format!(
            "target/stylized/sweep-{}.json",
            report.captured_unix_seconds
        )
    });
    let images = std::path::Path::new(&out_path).with_extension("images");
    std::fs::create_dir_all(images.parent().expect("capture parent"))
        .expect("create report parent");
    std::fs::create_dir(&images).expect("new capture directory");
    for (name, bytes) in pngs {
        std::fs::write(
            images.join(format!("{}.png", name.replace('/', "_"))),
            bytes,
        )
        .expect("write candidate capture");
    }
    let mut payload = serde_json::to_value(&report).expect("serialize");
    payload["status"] = serde_json::json!(if exit_code == 0 { "PASS" } else { "FAIL" });
    payload["collection_status"] = serde_json::json!(if collection_errors.is_empty() {
        "PASS"
    } else {
        "FAIL"
    });
    payload["collection_errors"] = serde_json::json!(collection_errors);
    payload["regression_status"] = serde_json::json!(match report.gate {
        Some(ref gate)
            if gate.regressions.is_empty()
                && gate.missing_cells.is_empty()
                && gate.invalid_cells.is_empty() =>
            "PASS",
        Some(_) => "FAIL",
        None => "NOT_EXECUTED",
    });
    payload["absolute_budget_status"] = serde_json::json!("NOT_DEFINED");
    write_report(&out_path, &payload);
    eprintln!("[written] {out_path}");
    std::process::exit(exit_code);
}

fn write_report(out_path: &str, payload: &serde_json::Value) {
    std::fs::create_dir_all(
        std::path::Path::new(&out_path)
            .parent()
            .expect("out path has parent"),
    )
    .expect("create out dir");
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out_path)
        .expect("new report path");
    file.write_all(
        serde_json::to_string_pretty(payload)
            .expect("serialize")
            .as_bytes(),
    )
    .expect("write report");
}
