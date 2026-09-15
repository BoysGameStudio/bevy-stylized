//! Write a staged candidate and atomically rename its directory; never rebaseline.
use super::{CompileReport, EmittedFile, plan::CompilePlan};
use sha2::{Digest, Sha256};
use std::path::Path;

pub(super) fn publish(
    plan: CompilePlan,
    bytes: &[u8],
    profile: Option<&str>,
    out_dir: &Path,
    report: &mut CompileReport,
) -> Result<(), String> {
    // -- 落盘 + 哈希 ---------------------------------------------------------
    if out_dir.exists() {
        return Err(format!(
            "output already exists: {}; choose a new candidate directory",
            out_dir.display()
        ));
    }
    let parent = out_dir
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| format!("create output parent: {e}"))?;
    let staging = tempfile::Builder::new()
        .prefix(".compile-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    for write in plan.files {
        let target = staging.path().join(&write.rel_path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("create {}: {e}", parent.display()))?;
        }
        std::fs::write(&target, &write.bytes)
            .map_err(|e| format!("write {}: {e}", target.display()))?;
        report.emitted.push(EmittedFile {
            kind: write.kind,
            path: path_to_slash(&write.rel_path),
            sha256: format!("{:x}", Sha256::digest(&write.bytes)),
            bytes: write.bytes.len() as u64,
        });
    }
    let manifest = serde_json::json!({"schema":"bevy_stylized.compiled/v1", "profile":profile,
        "input_sha256":format!("{:x}",Sha256::digest(bytes)), "emitted":report.emitted});
    std::fs::write(
        staging.path().join("compile-manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(staging.path(), out_dir)
        .map_err(|e| format!("commit compiled candidate: {e}"))?;
    Ok(())
}

fn path_to_slash(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
