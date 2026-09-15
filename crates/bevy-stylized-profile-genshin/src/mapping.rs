//! Pure-data conversion for the explicitly scoped Genshin static recipe.
use bevy_stylized_core::{
    StylizedMaterialDescriptor, StylizedSurfaceClass, StylizedTextureRef, StylizedTextureSemantic,
};
use std::collections::BTreeMap;

/// Roles declared by an authoring recipe, never inferred from runtime object names.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenshinMaterialRole {
    Body,
    Hair,
    Face,
    Brow,
    Pupil,
}

/// Evaluated Genshin body lighting inputs; independent of animation controls.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenshinLightingInput {
    pub use_lightmap_ao: f32,
    #[serde(default)]
    pub use_vertex_ao: f32,
    pub use_shadow_ramp: f32,
    pub lightmap_constant: [f32; 4],
    pub specular_color: [f32; 4],
    pub specular_rows: [[f32; 4]; 5],
    pub shadow_rows: [[f32; 4]; 5],
    pub metal_params: [f32; 4],
    pub metal_color: [f32; 4],
    pub metal_sharp_color: [f32; 4],
}

/// UV-driven emissive highlight layer outside the Genshin body shader.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenshinHighlightInput {
    pub texture: String,
    pub primary: [f32; 4],
    pub secondary: [f32; 4],
    pub curve: [f32; 4],
}

/// Face tint colors selected by the authored face-control alpha bands.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenshinFaceTints {
    pub day: [[f32; 4]; 3],
    pub night: [[f32; 4]; 3],
}

/// One exported material with explicit source texture semantics.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenshinMaterialInput {
    pub name: String,
    pub role: GenshinMaterialRole,
    pub base_color: String,
    #[serde(default)]
    pub cubic_base_color: bool,
    pub ilm: Option<String>,
    pub face_sdf: Option<String>,
    #[serde(default)]
    pub face_shadow: Option<String>,
    #[serde(default)]
    pub lighting: Option<GenshinLightingInput>,
    #[serde(default)]
    pub highlight: Option<GenshinHighlightInput>,
    #[serde(default)]
    pub face_tints: Option<GenshinFaceTints>,
    pub ramp: Option<String>,
    pub shadow_position: f32,
    pub ramp_width: f32,
    pub face_softness: f32,
    pub face_forward: [f32; 3],
    pub face_up: [f32; 3],
    pub metallic: bool,
    pub specular_ramp: bool,
    pub packed_normal: bool,
}

/// Versioned authoring-to-runtime handoff. This is not a runtime sidecar.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenshinBundleInput {
    pub schema: String,
    pub variant: String,
    pub materials: Vec<GenshinMaterialInput>,
    pub palette: BTreeMap<String, [f32; 4]>,
    /// Upstream convention: zero is day, one is night.
    #[serde(default)]
    pub day_night: f32,
}

/// Convert a complete supported variant, rejecting guessed/default material roles.
pub fn map_bundle(
    input: &GenshinBundleInput,
) -> Result<BTreeMap<String, StylizedMaterialDescriptor>, String> {
    if !matches!(
        input.schema.as_str(),
        "bevy_stylized.genshin-input/v1" | "bevy_stylized.genshin-input/v2"
    ) || input.variant.is_empty()
        || input.materials.is_empty()
    {
        return Err("unsupported or empty Genshin handoff".into());
    }
    if !(0.0..=1.0).contains(&input.day_night) {
        return Err("invalid day/night factor".into());
    }
    let mut result = BTreeMap::new();
    let palette_keys = [
        ("Soft Lit Colour", "palette_soft_lit"),
        ("Soft Shadow Color", "palette_soft_shadow"),
        ("Sharp Lit Colour", "palette_sharp_lit"),
        ("Sharp Shadow Color", "palette_sharp_shadow"),
    ];
    for m in &input.materials {
        if m.name.is_empty() || result.contains_key(&m.name) {
            return Err("empty/duplicate material name".into());
        }
        if m.metallic || m.specular_ramp || m.packed_normal {
            return Err(format!(
                "{}: unsupported active feature in static recipe",
                m.name
            ));
        }
        if !m.shadow_position.is_finite()
            || !m.ramp_width.is_finite()
            || m.ramp_width <= 0.0
            || !m.face_softness.is_finite()
            || m.face_softness <= 0.0
        {
            return Err(format!("{}: invalid shader parameters", m.name));
        }
        if !(0.0..=1.0).contains(&m.shadow_position)
            || m.face_forward
                .iter()
                .chain(&m.face_up)
                .any(|v| !v.is_finite())
        {
            return Err(format!("{}: invalid face basis or shadow position", m.name));
        }
        let f = m.face_forward;
        let u = m.face_up;
        let cross = [
            f[1] * u[2] - f[2] * u[1],
            f[2] * u[0] - f[0] * u[2],
            f[0] * u[1] - f[1] * u[0],
        ];
        if cross.iter().map(|v| v * v).sum::<f32>() < 1.0e-8 {
            return Err(format!("{}: degenerate face basis", m.name));
        }
        let mut descriptor = StylizedMaterialDescriptor {
            name: m.name.clone(),
            surface_class: StylizedSurfaceClass::Hero,
            ..Default::default()
        };
        descriptor.static_flags.double_sided = true;
        if m.cubic_base_color {
            descriptor
                .static_params
                .insert("cubic_base_color".into(), 1.0);
        }
        let mut texture = |uri: &str, semantic| -> Result<usize, String> {
            if uri.is_empty()
                || uri.starts_with('/')
                || uri.contains('\\')
                || uri.contains(':')
                || uri.split('/').any(|p| p == "..")
            {
                return Err(format!("{}: texture must be a relative bundle URI", m.name));
            }
            descriptor.textures.push(StylizedTextureRef {
                semantic,
                uri: uri.to_owned(),
            });
            Ok(descriptor.textures.len() - 1)
        };
        texture(&m.base_color, StylizedTextureSemantic::Color)?;
        match m.role {
            GenshinMaterialRole::Body | GenshinMaterialRole::Hair => {
                texture(
                    m.ilm
                        .as_deref()
                        .ok_or_else(|| format!("{}: ILM required", m.name))?,
                    StylizedTextureSemantic::ContinuousMask,
                )?;
                texture(
                    m.ramp
                        .as_deref()
                        .ok_or_else(|| format!("{}: ramp required", m.name))?,
                    StylizedTextureSemantic::Ramp,
                )?;
                descriptor.static_params.insert("genshin_ramp".into(), 1.0);
            }
            GenshinMaterialRole::Face => {
                texture(
                    m.face_sdf
                        .as_deref()
                        .ok_or_else(|| format!("{}: face SDF required", m.name))?,
                    StylizedTextureSemantic::DiscreteClassification,
                )?;
                if let Some(shadow) = &m.face_shadow {
                    let control_index = texture(shadow, StylizedTextureSemantic::ContinuousMask)?;
                    descriptor.static_params.insert("genshin_face".into(), 1.0);
                    descriptor
                        .static_params
                        .insert("surface_control_index".into(), control_index as f32);
                } else if input.schema.ends_with("/v2") {
                    return Err(format!("{}: face shadow control required for v2", m.name));
                }
                descriptor.static_params.extend([
                    ("face_forward_x".into(), m.face_forward[0]),
                    ("face_forward_y".into(), m.face_forward[1]),
                    ("face_forward_z".into(), m.face_forward[2]),
                    ("face_up_x".into(), m.face_up[0]),
                    ("face_up_y".into(), m.face_up[1]),
                    ("face_up_z".into(), m.face_up[2]),
                    ("face_softness".into(), m.face_softness),
                ]);
            }
            GenshinMaterialRole::Brow | GenshinMaterialRole::Pupil => {}
        }
        if let Some(lighting) = &m.lighting {
            if matches!(
                m.role,
                GenshinMaterialRole::Brow | GenshinMaterialRole::Pupil
            ) && let Some(ilm) = &m.ilm
            {
                texture(ilm, StylizedTextureSemantic::ContinuousMask)?;
            }
            let mut fields = vec![
                ("use_lightmap_ao".to_owned(), lighting.use_lightmap_ao),
                ("use_vertex_ao".to_owned(), lighting.use_vertex_ao),
                ("use_shadow_ramp".to_owned(), lighting.use_shadow_ramp),
            ];
            for (key, values) in [
                ("lightmap_constant", lighting.lightmap_constant),
                ("specular_color", lighting.specular_color),
                ("metal_params", lighting.metal_params),
                ("metal_color", lighting.metal_color),
                ("metal_sharp_color", lighting.metal_sharp_color),
            ] {
                for (i, v) in values.into_iter().enumerate() {
                    fields.push((format!("{key}_{i}"), v));
                }
            }
            for (key, rows) in [
                ("specular_rows", lighting.specular_rows),
                ("shadow_rows", lighting.shadow_rows),
            ] {
                for (r, values) in rows.into_iter().enumerate() {
                    for (c, v) in values.into_iter().enumerate() {
                        fields.push((format!("{key}_{r}_{c}"), v));
                    }
                }
            }
            if fields.iter().any(|(_, v)| !v.is_finite() || *v < 0.0) {
                return Err(format!("{}: invalid lighting metadata", m.name));
            }
            descriptor
                .static_params
                .insert("genshin_lighting".into(), 1.0);
            descriptor.static_params.extend(fields);
        }
        if let Some(tints) = &m.face_tints {
            if !matches!(m.role, GenshinMaterialRole::Face) {
                return Err("face tints require face role".into());
            }
            descriptor.static_params.insert("face_tints".into(), 1.0);
            for (key, rows) in [("face_day", tints.day), ("face_night", tints.night)] {
                for (r, values) in rows.into_iter().enumerate() {
                    for (c, value) in values.into_iter().enumerate() {
                        if !value.is_finite() || value < 0.0 {
                            return Err("invalid face tint".into());
                        }
                        descriptor
                            .static_params
                            .insert(format!("{key}_{r}_{c}"), value);
                    }
                }
            }
        }
        if let Some(highlight) = &m.highlight {
            if !matches!(m.role, GenshinMaterialRole::Pupil) {
                return Err("highlight requires the declared pupil role".into());
            }
            let control_index = texture(&highlight.texture, StylizedTextureSemantic::Color)?;
            descriptor
                .static_params
                .insert("pupil_highlight".into(), 1.0);
            descriptor
                .static_params
                .insert("surface_control_index".into(), control_index as f32);
            for (key, values) in [
                ("highlight_primary", highlight.primary),
                ("highlight_secondary", highlight.secondary),
                ("highlight_curve", highlight.curve),
            ] {
                for (i, value) in values.into_iter().enumerate() {
                    if !value.is_finite() || value < 0.0 {
                        return Err("invalid highlight data".into());
                    }
                    descriptor.static_params.insert(format!("{key}_{i}"), value);
                }
            }
        }
        descriptor.static_params.extend([
            ("shadow_position".into(), m.shadow_position),
            ("ramp_width".into(), m.ramp_width),
            ("day_night".into(), 1.0 - input.day_night),
        ]);
        for (key, prefix) in palette_keys {
            let color = input
                .palette
                .get(key)
                .ok_or_else(|| format!("missing palette: {key}"))?;
            for (channel, value) in ["r", "g", "b"].into_iter().zip(color) {
                if !value.is_finite() || *value < 0.0 {
                    return Err(format!("invalid palette: {key}"));
                }
                descriptor
                    .static_params
                    .insert(format!("{prefix}_{channel}"), *value);
            }
        }
        result.insert(m.name.clone(), descriptor);
    }
    Ok(result)
}

/// Interpret the authored view separately from static per-material colors.
pub fn map_view(
    input: &GenshinBundleInput,
) -> Result<bevy_stylized_core::StylizedViewDescriptor, String> {
    if !matches!(
        input.schema.as_str(),
        "bevy_stylized.genshin-input/v1" | "bevy_stylized.genshin-input/v2"
    ) || !(0.0..=1.0).contains(&input.day_night)
    {
        return Err("invalid view handoff".into());
    }
    let color = input
        .palette
        .get("Ambient Colour")
        .ok_or("missing authored ambient color")?;
    if color.iter().any(|v| !v.is_finite() || *v < 0.0) {
        return Err("invalid authored ambient color".into());
    }
    let mut view = bevy_stylized_core::StylizedViewDescriptor {
        day_night_factor: 1.0 - input.day_night,
        ..Default::default()
    };
    let mut ambient = view.key_light_color;
    ambient.red = color[0];
    ambient.green = color[1];
    ambient.blue = color[2];
    ambient.alpha = color[3];
    view.ambient_override = Some(ambient);
    Ok(view)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> GenshinBundleInput {
        GenshinBundleInput {
            schema: "bevy_stylized.genshin-input/v1".into(),
            variant: "static".into(),
            day_night: 0.0,
            palette: [
                "Soft Lit Colour",
                "Soft Shadow Color",
                "Sharp Lit Colour",
                "Sharp Shadow Color",
            ]
            .into_iter()
            .map(|s| (s.into(), [1.; 4]))
            .collect(),
            materials: vec![GenshinMaterialInput {
                name: "face".into(),
                role: GenshinMaterialRole::Face,
                base_color: "textures/color.png".into(),
                cubic_base_color: false,
                ilm: None,
                ramp: None,
                face_shadow: None,
                lighting: None,
                highlight: None,
                face_tints: None,
                face_sdf: Some("textures/sdf.png".into()),
                shadow_position: 0.55,
                ramp_width: 1.,
                face_softness: 0.001,
                face_forward: [0., 0., 1.],
                face_up: [0., 1., 0.],
                metallic: false,
                specular_ramp: false,
                packed_normal: false,
            }],
        }
    }
    #[test]
    fn missing_face_data_is_not_a_default_material() {
        let mut i = input();
        i.materials[0].face_sdf = None;
        assert!(map_bundle(&i).is_err());
    }
    #[test]
    fn roles_preserve_typed_channels_and_reject_unmapped_features() {
        let mut i = input();
        let d = map_bundle(&i).unwrap();
        assert_eq!(
            d["face"].textures[1].semantic,
            StylizedTextureSemantic::DiscreteClassification
        );
        i.materials[0].metallic = true;
        assert!(map_bundle(&i).is_err());
    }
    #[test]
    fn duplicate_roles_and_external_paths_fail() {
        let mut i = input();
        i.materials.push(i.materials[0].clone());
        assert!(map_bundle(&i).is_err());
        i.materials.pop();
        i.materials[0].base_color = "../escape.png".into();
        assert!(map_bundle(&i).is_err());
    }
}
