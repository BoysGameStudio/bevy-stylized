//! Required collection cannot skip absent devices or overwrite prior evidence.
use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn required_gpu_absence_is_blocked_and_existing_output_is_preserved() {
    let root = std::env::temp_dir().join(format!(
        "stylized-required-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let path = root.join("report.json");
    let mut command = Command::new(env!("CARGO_BIN_EXE_regression-lab"));
    command
        .env("STYLIZED_GPU", "disabled")
        .env_remove("STYLIZED_REQUIRE_GPU")
        .args(["--required", "--out"])
        .arg(&path);
    let output = command.output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = fs::read(&path).unwrap();
    let record: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(record["status"], "BLOCKED");
    assert_eq!(record["collection_status"], "BLOCKED");
    let repeated = command.output().unwrap();
    assert_eq!(repeated.status.code(), Some(1));
    assert_eq!(fs::read(&path).unwrap(), bytes);
    fs::remove_dir_all(root).unwrap();
}
