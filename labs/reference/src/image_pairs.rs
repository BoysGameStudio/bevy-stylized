//! Read-only comparison of externally captured images using the shared criteria.
use golden_harness::{
    Tier2Thresholds, decode_png, evaluate_tier2, evaluate_tier2_region,
};
use serde::Deserialize;
use std::{collections::HashSet, path::Path};

#[derive(Deserialize)]
struct Region {
    name: String,
    mask: String,
}

#[derive(Deserialize)]
struct Pair {
    case: String,
    baseline: String,
    candidate: String,
    /// Optional region masks (renderer-produced isolation captures): each
    /// entry adds a region-restricted Tier-2 report named `case@region`.
    #[serde(default)]
    regions: Vec<Region>,
}
#[derive(Deserialize)]
struct Manifest {
    background: [u8; 3],
    pairs: Vec<Pair>,
}

pub fn run(manifest: &Path, output: &Path) -> Result<bool, String> {
    if output.exists() {
        return Err("comparison output already exists".into());
    }
    let source = std::fs::read(manifest).map_err(|e| e.to_string())?;
    let input: Manifest = serde_json::from_slice(&source).map_err(|e| e.to_string())?;
    if input.pairs.is_empty() {
        return Err("image-pair manifest is empty".into());
    }
    let parent = manifest.parent().ok_or("manifest parent missing")?;
    let mut names = HashSet::new();
    let mut reports = Vec::new();
    let mut region_reports = Vec::new();
    for pair in input.pairs {
        if !names.insert(pair.case.clone()) {
            return Err(format!("duplicate image-pair case {}", pair.case));
        }
        let load = |path: &str| {
            let bytes = std::fs::read(parent.join(path)).map_err(|e| e.to_string())?;
            decode_png(&bytes).map_err(|e| e.to_string())
        };
        let baseline = load(&pair.baseline)?;
        let candidate = load(&pair.candidate)?;
        let report = evaluate_tier2(
            &pair.case,
            &baseline,
            &candidate,
            input.background,
            Tier2Thresholds::default(),
        )
        .map_err(|e| e.to_string())?;
        reports.push(report);
        for region in pair.regions {
            let mask_image = load(&region.mask)?;
            if mask_image.width != baseline.width || mask_image.height != baseline.height {
                return Err(format!("region mask dimensions differ: {}", region.name));
            }
            let mask: Vec<bool> = mask_image
                .data
                .as_chunks::<4>()
                .0
                .iter()
                .map(|px| px[..3].iter().any(|&c| c > 128))
                .collect();
            let report = evaluate_tier2_region(
                &pair.case,
                &region.name,
                &baseline,
                &candidate,
                &mask,
                Tier2Thresholds::default(),
            )
            .map_err(|e| e.to_string())?;
            region_reports.push(report);
        }
    }
    let passed = reports.iter().all(|r| r.passed) && region_reports.iter().all(|r| r.passed);
    let value = serde_json::json!({
        "schema":"bevy_stylized.image-pairs/v1", "status":if passed {"PASS"}else{"FAIL"},
        "expected":reports.len(), "executed":reports.len(),
        "failed":reports.iter().filter(|r|!r.passed).count(),
        "region_failed":region_reports.iter().filter(|r|!r.passed).count(),
        "reports":reports, "region_reports":region_reports,
        "scope":"image criteria only; input identity, state and motion review are separate gates"
    });
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(
        output,
        serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::json!({"status":value["status"],"executed":value["executed"],"failed":value["failed"],"report":output})
    );
    Ok(passed)
}
