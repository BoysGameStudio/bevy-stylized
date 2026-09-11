#![expect(missing_docs, reason = "focused Task 26 roundtrip tool")]

use std::{io, path::PathBuf};

use bevy_cel::CelNprAuthoring;
use bevy_gltf_metadata::load_sidecar;
use serde_json::json;

fn bits<const N: usize>(values: [f32; N]) -> [u32; N] {
    values.map(f32::to_bits)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "expected metadata path"))?;
    let metadata = load_sidecar(path)?;
    let node = metadata
        .nodes
        .get("Hero")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Hero node missing"))?;
    let authored = CelNprAuthoring::from_node(node)?;
    let ramp = authored.material.ramp.to_settings();
    let lighting = authored.material.to_lighting_settings();
    let eye = authored.material.eye_emission.to_settings();
    let outline = authored.material.outline.to_settings();
    let outline_color = outline.color();
    let post = authored.object.post.to_profile();
    assert_eq!(
        (ramp.ramp_row(), ramp.ramp_row_count(), ramp.band_count()),
        (2, 4, 5)
    );
    assert_eq!(
        bits([ramp.band_smoothing(), ramp.ramp_scale(), ramp.ramp_offset()]),
        bits([0.125, 1.5, -0.25])
    );
    assert_eq!(
        authored.material.ilm.to_settings().material_id_source(),
        bevy_cel::IlmMaterialIdSource::VertexThenTexture
    );
    assert_eq!(bits(lighting.material_masks()), bits([0.9, 0.7, 0.6]));
    let emission = eye.emission();
    assert_eq!(bits(emission.0), bits([1.0, 0.5, 0.25]));
    assert_eq!(emission.1.to_bits(), 12.0_f32.to_bits());
    assert_eq!(outline.width().to_bits(), 4.0_f32.to_bits());
    assert_eq!(
        post.settings().bloom_intensity().to_bits(),
        0.3_f32.to_bits()
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ramp": {"row": ramp.ramp_row(), "rows": ramp.ramp_row_count(), "bands": ramp.band_count(), "smoothing": ramp.band_smoothing(), "scale": ramp.ramp_scale(), "offset": ramp.ramp_offset()},
            "lighting_masks": lighting.material_masks(),
            "eye_emission": {"emission": eye.emission(), "floors": eye.eye_floor(), "class": eye.eye_class(), "masks": eye.masks()},
            "outline": {"width": outline.width(), "color": [outline_color.red, outline_color.green, outline_color.blue, outline_color.alpha]},
            "post": {"bloom": post.settings().bloom_intensity(), "vignette": post.settings().vignette(), "distortion": post.settings().lens_distortion(), "chromatic": post.settings().chromatic_aberration()},
        }))?
    );
    Ok(())
}
