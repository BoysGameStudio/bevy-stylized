//! Declared reference pipeline: validate, build calibrated scene, compare cases.
mod calibration;
mod cases;
mod diagnostics;
mod frames;
mod input;
mod io;
mod metrics;
mod scene;
use input::package;
use serde_json::Value;
use std::path::PathBuf;

/// Run a declared six-case capture package, retaining truthful failed parity.
pub fn run(path: PathBuf, out: PathBuf) -> Result<Value, String> {
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    let p = package(&path)?;
    let root = path.parent().ok_or("case parent")?;
    if out.exists() {
        return Err("candidate output already exists".into());
    }
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let prepared = scene::setup(root, &out, &p)?;
    cases::compare(&path, &out, root, p, prepared)
}
