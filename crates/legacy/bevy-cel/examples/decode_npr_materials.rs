#![expect(missing_docs, reason = "focused Task 26 multi-material roundtrip tool")]

use std::{io, path::PathBuf};

use bevy_cel::CelNprAuthoring;
use bevy_gltf_metadata::load_sidecar;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "expected metadata path"))?;
    let metadata = load_sidecar(path)?;
    let node = metadata
        .nodes
        .get("PortableHero")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "PortableHero node missing"))?;
    let authored = CelNprAuthoring::from_node(node)?;
    let entries = authored.materials();
    assert_eq!(entries.len(), 2);
    assert_eq!((entries[0].slot, entries[0].name.as_str()), (0, "B_Second"));
    assert_eq!((entries[1].slot, entries[1].name.as_str()), (1, "A_First"));
    assert_eq!(entries[0].settings.ramp.to_settings().ramp_row(), 1);
    assert_eq!(entries[1].settings.ramp.to_settings().ramp_row(), 0);
    assert_eq!(
        entries[0].settings.hair.to_settings().intensity().to_bits(),
        2.0_f32.to_bits()
    );
    assert_eq!(
        entries[1].settings.hair.to_settings().intensity().to_bits(),
        1.0_f32.to_bits()
    );
    assert_eq!(authored.default_material(), &entries[0].settings);
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "materials_version": authored.materials_version,
            "materials": entries.iter().map(|entry| json!({"slot": entry.slot, "name": entry.name, "row": entry.settings.ramp.to_settings().ramp_row(), "hair_intensity": entry.settings.hair.to_settings().intensity()})).collect::<Vec<_>>()
        }))?
    );
    Ok(())
}
