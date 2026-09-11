#![expect(
    clippy::redundant_pub_crate,
    reason = "support functions are shared with the parent integration test"
)]

use std::{
    io,
    path::{Path, PathBuf},
    process::{Command, Output},
};

pub(super) fn asset_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets/test")
        .join(name)
}

pub(super) fn run(arguments: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_cel_gltf_validate"))
        .args(arguments)
        .output()
}
