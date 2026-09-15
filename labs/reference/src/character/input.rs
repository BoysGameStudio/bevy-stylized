//! Reference-only case versions, fixed matrix and actual-settings validation.
use super::io::{read, sha, vec};
use bevy::prelude::*;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::BTreeMap, path::Path};

#[derive(Deserialize)]
pub(super) struct CameraInput {
    pub(super) position: [f32; 3],
    pub(super) target: [f32; 3],
    pub(super) up: [f32; 3],
    pub(super) projection: String,
    pub(super) scale: f32,
    pub(super) near: f32,
    pub(super) far: f32,
}
#[derive(Deserialize)]
pub(super) struct Case {
    pub(super) id: String,
    pub(super) camera: CameraInput,
    pub(super) direction_to_light: [f32; 3],
    pub(super) reference: Vec<String>,
    pub(super) actual: String,
    pub(super) silhouette: String,
    #[serde(default)]
    pub(super) factor_reference: Option<String>,
    #[serde(default)]
    pub(super) center_reference: Option<String>,
}
#[derive(Deserialize)]
pub(super) struct Package {
    pub(super) schema: String,
    pub(super) coordinates: String,
    pub(super) resolution: [u32; 2],
    pub(super) background_linear: [f32; 4],
    pub(super) required_cases: Vec<String>,
    pub(super) cases: Vec<Case>,
    pub(super) files: BTreeMap<String, String>,
    #[serde(default)]
    pub(super) diagnostics: Value,
    #[serde(default)]
    pub(super) supersample: Option<u32>,
    #[serde(default)]
    pub(super) sampling: Value,
}
pub(super) fn package(path: &Path) -> Result<Package, String> {
    let p: Package = serde_json::from_slice(&read(path)?).map_err(|e| e.to_string())?;
    if !matches!(
        p.schema.as_str(),
        "bevy_stylized.character-capture/v1"
            | "bevy_stylized.character-capture/v2"
            | "bevy_stylized.character-capture/v3"
    ) || p.coordinates != "gltf-y-up-metres"
        || p.resolution != [512, 512]
    {
        return Err("unsupported capture domain".into());
    }
    if p.schema.ends_with("/v3") && !p.supersample.is_some_and(|v| [2, 4, 8].contains(&v)) {
        return Err("matched case requires a supported sampling grid".into());
    }
    if p.schema.ends_with("/v3")
        && (p.diagnostics["policy"] != "lighting-factors/v1"
            || p.sampling["matched"]["policy"] != "matched-box/v1"
            || p.sampling["matched"]["grid"].as_u64() != p.supersample.map(u64::from))
    {
        return Err(
            "matched qualification requires lighting factors and matching sampling policy".into(),
        );
    }
    let ids: Vec<_> = ["front", "left", "right"]
        .into_iter()
        .flat_map(|l| [format!("full_body-{l}"), format!("face_closeup-{l}")])
        .collect();
    if p.required_cases != ids || p.cases.iter().map(|c| c.id.clone()).collect::<Vec<_>>() != ids {
        return Err("required cases missing/duplicated".into());
    }
    let root = path
        .parent()
        .ok_or("case parent")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    for (name, digest) in &p.files {
        if Path::new(name).is_absolute() {
            return Err("absolute input".into());
        }
        let file = root.join(name).canonicalize().map_err(|e| e.to_string())?;
        if !file.starts_with(&root) || sha(&read(file)?) != *digest {
            return Err(format!("input identity mismatch: {name}"));
        }
    }
    for name in [
        "runtime/character.glb",
        "runtime/character.npr.json",
        "runtime/character.view.json",
        "authoring.json",
        "audit.json",
        "calibration.json",
        "calibration-blender.png",
        "runtime/calibration.glb",
    ] {
        if !p.files.contains_key(name) {
            return Err(format!("input not inventoried: {name}"));
        }
    }
    for c in &p.cases {
        if (p.schema.ends_with("/v2") || p.schema.ends_with("/v3"))
            && (c.factor_reference.is_none() || c.center_reference.is_none())
        {
            return Err("missing diagnostic reference".into());
        }
        let a = &c.camera;
        if a.projection != "orthographic"
            || !a.scale.is_finite()
            || a.scale <= 0.
            || !(0. < a.near && a.near < a.far)
            || !a.far.is_finite()
            || !vec(a.position).is_finite()
            || !vec(a.target).is_finite()
            || !vec(a.up).is_finite()
            || (vec(a.target) - vec(a.position))
                .cross(vec(a.up))
                .length_squared()
                < 1e-12
            || !vec(c.direction_to_light).is_finite()
            || (vec(c.direction_to_light).length_squared() - 1.).abs() > 1e-5
        {
            return Err("invalid camera/light".into());
        }
        if c.reference.len() != 2
            || c.reference
                .iter()
                .chain([&c.actual, &c.silhouette])
                .chain(c.factor_reference.iter())
                .chain(c.center_reference.iter())
                .any(|f| !p.files.contains_key(f))
        {
            return Err("reference not inventoried".into());
        }
    }
    for c in &p.cases {
        let actual: Value =
            serde_json::from_slice(&read(root.join(&c.actual))?).map_err(|e| e.to_string())?;
        let matrix: [[f32; 4]; 4] =
            serde_json::from_value(actual["camera"]["matrix_world"].clone())
                .map_err(|e| e.to_string())?;
        let position = Vec3::new(matrix[0][3], matrix[2][3], -matrix[1][3]);
        let forward = Vec3::new(-matrix[0][2], -matrix[2][2], matrix[1][2]);
        let body: [f32; 3] = serde_json::from_value(actual["body_direction_blender"].clone())
            .map_err(|e| e.to_string())?;
        let light = Vec3::new(body[0], body[2], -body[1]).normalize();
        if position.distance(vec(c.camera.position)) > 1e-5
            || forward.distance((vec(c.camera.target) - position).normalize()) > 1e-5
            || light.distance(vec(c.direction_to_light)) > 1e-5
            || actual["exposure_ev"] != 0.
            || actual["gamma"] != 1.
            || actual["view_transform"] != "Standard"
            || (actual["camera"]["ortho_scale"]
                .as_f64()
                .ok_or("actual scale")? as f32
                - c.camera.scale)
                .abs()
                > 1e-5
        {
            return Err("upstream actual settings mismatch".into());
        }
    }
    Ok(p)
}
