//! Golden-matrix integration test: skips gracefully without the Odette
//! assets (set `TENACITY_ROOT`) or without a GPU adapter.

use odette_reference_lab::{CaseOutcome, run_matrix};

#[test]
fn odette_golden_matrix_matches_baseline() {
    let Some(outcomes) = run_matrix().expect("capture and IO must succeed") else {
        // Assets or GPU missing: graceful skip keeps CI green.
        return;
    };
    let mut failures = Vec::new();
    for (case, outcome) in outcomes {
        match outcome {
            CaseOutcome::Baselined => eprintln!("[baselined] {}", case.name),
            CaseOutcome::Matched(report) => eprintln!(
                "[matched] {} psnr={:.2}dB mean_delta={:.4}",
                case.name, report.stats.psnr_db, report.stats.mean_absolute_delta
            ),
            CaseOutcome::Exceeded(report) => {
                failures.push(format!("{}: {:?}", case.name, report.violations));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "golden cases exceeded tolerance: {failures:?}"
    );
}
