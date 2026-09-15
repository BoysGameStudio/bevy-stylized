//! Local capture-package I/O and coordinate helpers.
use bevy::prelude::Vec3;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;

pub(super) fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn read(path: impl AsRef<Path>) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| e.to_string())
}
pub(super) fn write(path: impl AsRef<Path>, data: &Value) -> Result<(), String> {
    std::fs::write(
        path,
        serde_json::to_vec_pretty(data).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
pub(super) fn vec(v: [f32; 3]) -> Vec3 {
    Vec3::from_array(v)
}
