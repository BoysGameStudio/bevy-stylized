//! Effect-mesh handoff: the authored "Light Vectors" GN outputs must reach the
//! typed sidecar, and the compiled GLB must carry the effect meshes. Data
//! integrity only; effect shading is a separate contract.

use std::path::{Path, PathBuf};

use bevy_stylized_core::StylizedEffectLightBasis;

// Locked upstream values for the genshin-odette Eff_001/Eff_002 source build
// (azimuth-packed light direction; basis exported with the [x, z, -y] conversion).
const LIGHT_DIRECTION: [f32; 3] = [-std::f32::consts::FRAC_PI_2, 0.0, 0.0];
const HEAD_FORWARD: [f32; 3] = [0.0, 0.0, 0.7827];
const HEAD_UP: [f32; 3] = [0.0, 0.5384, 0.0];

fn assert_basis(mesh: &str, basis: &StylizedEffectLightBasis) {
    for (label, actual, expected) in [
        ("light_direction", basis.light_direction, LIGHT_DIRECTION),
        ("head_forward", basis.head_forward, HEAD_FORWARD),
        ("head_up", basis.head_up, HEAD_UP),
    ] {
        for (a, e) in actual.iter().zip(expected) {
            assert!(
                (a - e).abs() < 1e-4,
                "{mesh}: {label} mismatch: {actual:?} vs {expected:?}"
            );
        }
    }
}

fn glb_node_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let n = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let json: serde_json::Value = serde_json::from_slice(&bytes[20..20 + n]).unwrap();
    json["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["name"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
#[ignore = "requires the local genshin-odette effects build: STYLIZED_EFFECTS_ASSET=<build>/compiled/character.glb"]
fn effect_light_basis_reaches_typed_sidecar() {
    let asset = PathBuf::from(
        std::env::var("STYLIZED_EFFECTS_ASSET").expect("explicit effects build asset required"),
    );
    let sidecar = bevy_stylized_gltf::load_sidecar(asset.with_extension("npr.json"))
        .expect("typed sidecar required");
    assert_eq!(sidecar.effect_lighting.len(), 2);
    for mesh in ["Eff_001", "Eff_002"] {
        let basis = sidecar
            .effect_lighting
            .get(mesh)
            .unwrap_or_else(|| panic!("missing effect_lighting entry for {mesh}"));
        assert_basis(mesh, basis);
    }
    let nodes = glb_node_names(&asset);
    for node in ["runtime_Eff_001", "runtime_Eff_002"] {
        assert!(
            nodes.iter().any(|n| n == node),
            "compiled GLB missing effect node {node}"
        );
    }
}
