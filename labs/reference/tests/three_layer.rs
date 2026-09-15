//! GPU reference comparison is explicit and never a graceful skip.
#[test]
#[ignore = "requires GPU and STYLIZED_REFERENCE_CASE"]
fn upstream_comparison_reports_actual_criteria() {
    let reports =
        reference_comparison::run_reference_comparison().expect("reference capture failed");
    assert_eq!(reports.len(), 1);
    assert!(
        reports[0].tier2.passed,
        "upstream parity criteria failed; inspect the report"
    );
}
