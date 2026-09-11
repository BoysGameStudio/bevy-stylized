//! Runs the Odette golden capture matrix: captures when no baseline exists,
//! diffs against the stored baseline otherwise, exits non-zero on exceedance.
//!
//! Set `GOLDEN_REBASELINE=1` to accept the current render as the new baseline.

use std::process::ExitCode;

fn main() -> ExitCode {
    odette_reference_lab::run_cli()
}
