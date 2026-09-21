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

/// Source-unit hull settings evaluated from the supported static outline graph.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenshinOutlineInput {
    pub width: f32,
    pub colors: [[f32; 4]; 5],
    pub enabled_mask: u32,
    pub force_first: bool,
    pub gamma: f32,
    pub class_texture: Option<String>,
    #[serde(default)]
    pub prepared_position: bool,
}

/// Authored Genshin metal tint and highlight inputs, before source gamma nodes.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenshinMetalInput {
    pub matcap: String,
    pub specular_ramp: Option<String>,
    pub brightness: f32,
    pub tile_scale: f32,
    pub dark: [f32; 4],
    pub light: [f32; 4],
    pub shadow: [f32; 4],
    pub specular: [f32; 4],
    pub specular_color: [f32; 4],
    pub sharp_color: [f32; 4],
}

/// Normal-map reconstruction is a source policy, not an ordinary tangent XYZ map.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenshinNormalInput {
    pub texture: String,
    pub method: String,
    pub strength: f32,
}

/// Enabled source Alpha emission closure; zero strength still mixes toward black.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenshinEmissionInput {
    pub method: String,
    pub tint: [f32; 4],
    pub strength: f32,
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
    #[serde(default)]
    pub outline: Option<GenshinOutlineInput>,
    #[serde(default)]
    pub metal: Option<GenshinMetalInput>,
    #[serde(default)]
    pub normal: Option<GenshinNormalInput>,
    #[serde(default)]
    pub emission: Option<GenshinEmissionInput>,
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
        if m.packed_normal != m.normal.is_some()
            || (m.packed_normal
                && (m.lighting.is_none() || matches!(m.role, GenshinMaterialRole::Face)))
        {
            return Err(format!(
                "{}: enabled normals require complete body-shader normal inputs",
                m.name
            ));
        }
        if m.metallic != m.metal.is_some() || (m.metallic && m.lighting.is_none()) {
            return Err(format!(
                "{}: enabled metal requires complete source lighting/metal inputs",
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
        if let Some(emission) = &m.emission {
            if emission.method != "genshin-alpha-emission/v1"
                || !m.cubic_base_color
                || m.lighting.is_none()
                || matches!(m.role, GenshinMaterialRole::Face)
                || emission
                    .tint
                    .iter()
                    .chain([&emission.strength])
                    .any(|v| !v.is_finite() || *v < 0.0)
            {
                return Err(format!("{}: unsupported source emission inputs", m.name));
            }
            for (i, tint) in emission.tint.iter().take(3).enumerate() {
                let value = tint.powf(2.2) * emission.strength;
                if !value.is_finite() {
                    return Err("source emission radiance overflow".into());
                }
                descriptor
                    .static_params
                    .insert(format!("source_emission_{i}"), value);
            }
            descriptor
                .static_params
                .insert("source_emission_3".into(), 1.0);
        }
        if let Some(normal) = &m.normal {
            if normal.method != "genshin-optimized-bump/v1"
                || !normal.strength.is_finite()
                || normal.strength <= 0.0
                || normal.strength > 1.0
            {
                return Err(format!(
                    "{}: unsupported normal reconstruction or strength",
                    m.name
                ));
            }
            texture(&normal.texture, StylizedTextureSemantic::Normal)?;
            descriptor
                .static_params
                .insert("source_conventions_0".into(), normal.strength);
        }
        if let Some(metal) = &m.metal {
            let mut fields = vec![
                ("metal_controls_0".to_owned(), 1.0),
                ("metal_controls_1".to_owned(), metal.brightness),
                ("metal_controls_2".to_owned(), metal.tile_scale),
                ("metal_controls_3".to_owned(), f32::from(m.specular_ramp)),
            ];
            for (key, values) in [
                ("metal_dark", metal.dark),
                ("metal_light", metal.light),
                ("metal_shadow", metal.shadow),
                ("metal_specular", metal.specular),
                ("metal_specular_color", metal.specular_color),
                ("metal_sharp_color", metal.sharp_color),
            ] {
                for (i, value) in values.into_iter().enumerate() {
                    fields.push((format!("{key}_{i}"), value));
                }
            }
            if fields.iter().any(|(_, v)| !v.is_finite() || *v < 0.0) || metal.specular[0] <= 0.0 {
                return Err(format!("{}: invalid source metal parameters", m.name));
            }
            texture(&metal.matcap, StylizedTextureSemantic::MatCap)?;
            if m.specular_ramp {
                let index = texture(
                    metal
                        .specular_ramp
                        .as_deref()
                        .ok_or("enabled metal ramp requires its source texture")?,
                    StylizedTextureSemantic::Ramp,
                )?;
                descriptor
                    .static_params
                    .insert("specular_ramp_index".into(), index as f32);
                descriptor
                    .static_params
                    .insert("specular_ramp_linear".into(), 1.0);
            } else if metal.specular_ramp.is_some() {
                return Err("inactive metal ramp must not bind a texture".into());
            }
            descriptor.static_params.extend(fields);
        }
        if let Some(outline) = &m.outline {
            if !outline.width.is_finite()
                || outline.width <= 0.0
                || !outline.gamma.is_finite()
                || outline.gamma <= 0.0
                || outline.enabled_mask > 15
                || outline
                    .colors
                    .iter()
                    .flatten()
                    .any(|v| !v.is_finite() || *v < 0.0)
                || (!outline.force_first && outline.class_texture.is_none())
            {
                return Err(format!("{}: invalid or incomplete outline input", m.name));
            }
            if let Some(uri) = &outline.class_texture {
                let index = texture(uri, StylizedTextureSemantic::DiscreteClassification)?;
                descriptor
                    .static_params
                    .insert("outline_class_texture_index".into(), index as f32);
            }
            descriptor.static_params.extend([
                ("outline_source_width".into(), outline.width),
                ("outline_gamma".into(), outline.gamma),
                ("outline_enabled_mask".into(), outline.enabled_mask as f32),
                ("outline_force_first".into(), f32::from(outline.force_first)),
                (
                    "outline_source_position".into(),
                    f32::from(outline.prepared_position),
                ),
            ]);
            for (row, color) in outline.colors.iter().enumerate() {
                for (channel, value) in color.iter().take(3).enumerate() {
                    descriptor
                        .static_params
                        .insert(format!("outline_color_{row}_{channel}"), *value);
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
                outline: None,
                metal: None,
                normal: None,
                emission: None,
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
    fn source_outline_colors_width_and_disabled_classes_are_preserved() {
        let mut i = input();
        i.materials[0].outline = Some(GenshinOutlineInput {
            width: 0.0025,
            colors: [[0.2, 0.3, 0.4, 1.0]; 5],
            enabled_mask: 14,
            force_first: false,
            gamma: 2.2,
            class_texture: Some("textures/outline.png".into()),
            prepared_position: true,
        });
        let mapped = map_bundle(&i).unwrap();
        let d = &mapped["face"];
        assert_eq!(d.static_params["outline_source_width"], 0.0025);
        assert_eq!(d.static_params["outline_enabled_mask"], 14.0);
        assert_eq!(d.static_params["outline_color_4_2"], 0.4);
        let index = d.static_params["outline_class_texture_index"] as usize;
        assert_eq!(
            d.textures[index].semantic,
            StylizedTextureSemantic::DiscreteClassification
        );
        i.materials[0].outline.as_mut().unwrap().class_texture = Some("../outside.png".into());
        assert!(map_bundle(&i).is_err());
        i.materials[0].outline.as_mut().unwrap().class_texture = None;
        assert!(map_bundle(&i).is_err());
    }

    fn body_input() -> GenshinBundleInput {
        let mut input = input();
        let material = &mut input.materials[0];
        material.name = "body".into();
        material.role = GenshinMaterialRole::Body;
        material.ilm = Some("ilm.png".into());
        material.ramp = Some("shadow.png".into());
        material.lighting = Some(GenshinLightingInput {
            use_lightmap_ao: 1.,
            use_vertex_ao: 0.,
            use_shadow_ramp: 1.,
            lightmap_constant: [0., 0., 0., 1.],
            specular_color: [1.; 4],
            specular_rows: [[4., 0.1, 1., 0.]; 5],
            shadow_rows: [[0.01, 0.5, 0., 0.]; 5],
            metal_params: [4., 1.2, 0.2, 0.95],
            metal_color: [1.; 4],
            metal_sharp_color: [1.; 4],
        });
        input
    }

    #[test]
    fn active_metal_and_ramp_require_complete_traceable_inputs() {
        let mut input = body_input();
        input.materials[0].metallic = true;
        assert!(map_bundle(&input).is_err());
        input.materials[0].metal = Some(GenshinMetalInput {
            matcap: "metal.png".into(),
            specular_ramp: None,
            brightness: 3.,
            tile_scale: 1.,
            dark: [0.2; 4],
            light: [1.; 4],
            shadow: [1.; 4],
            specular: [4., 1.2, 0.2, 0.95],
            specular_color: [1.; 4],
            sharp_color: [1.; 4],
        });
        let mapped = map_bundle(&input).unwrap();
        assert_eq!(mapped["body"].static_params["metal_controls_0"], 1.);
        assert_eq!(mapped["body"].static_params["metal_controls_3"], 0.);
        assert!(
            !mapped["body"]
                .static_params
                .contains_key("specular_ramp_index")
        );
        input.materials[0].specular_ramp = true;
        assert!(map_bundle(&input).is_err());
        input.materials[0].metal.as_mut().unwrap().specular_ramp = Some("specular.png".into());
        let mapped = map_bundle(&input).unwrap();
        assert_eq!(mapped["body"].static_params["metal_controls_3"], 1.);
        assert_eq!(mapped["body"].static_params["specular_ramp_linear"], 1.);
        input.materials[0].metal.as_mut().unwrap().matcap = "../outside.png".into();
        assert!(map_bundle(&input).is_err());
    }
    #[test]
    fn source_emission_requires_raw_alpha_and_preserves_zero_strength() {
        let mut input = body_input();
        input.materials[0].emission = Some(GenshinEmissionInput {
            method: "genshin-alpha-emission/v1".into(),
            tint: [0.5, 0.7, 0.9, 1.],
            strength: 2.,
        });
        assert!(map_bundle(&input).is_err());
        input.materials[0].cubic_base_color = true;
        let mapped = map_bundle(&input).unwrap();
        assert!((mapped["body"].static_params["source_emission_0"] - 0.4352753).abs() < 1e-6);
        assert!(!mapped["body"].static_flags.alpha_test);
        input.materials[0].emission.as_mut().unwrap().strength = 0.;
        let mapped = map_bundle(&input).unwrap();
        assert_eq!(mapped["body"].static_params["source_emission_0"], 0.);
        assert_eq!(mapped["body"].static_params["source_emission_3"], 1.);
        input.materials[0].emission.as_mut().unwrap().strength = -1.;
        assert!(map_bundle(&input).is_err());
        input.materials[0].emission.as_mut().unwrap().strength = 2.;
        input.materials[0].emission.as_mut().unwrap().method = "additive".into();
        assert!(map_bundle(&input).is_err());
    }

    #[test]
    fn normals_require_the_verified_method_texture_and_strength() {
        let mut input = body_input();
        input.materials[0].packed_normal = true;
        assert!(map_bundle(&input).is_err());
        input.materials[0].normal = Some(GenshinNormalInput {
            texture: "normal.png".into(),
            method: "genshin-optimized-bump/v1".into(),
            strength: 1.,
        });
        let mapped = map_bundle(&input).unwrap();
        assert_eq!(mapped["body"].static_params["source_conventions_0"], 1.);
        assert!(
            mapped["body"]
                .textures
                .iter()
                .any(|t| t.uri == "normal.png" && t.semantic == StylizedTextureSemantic::Normal)
        );
        input.materials[0].normal.as_mut().unwrap().method = "ordinary-xyz".into();
        assert!(map_bundle(&input).is_err());
        input.materials[0].normal.as_mut().unwrap().method = "genshin-optimized-bump/v1".into();
        input.materials[0].normal.as_mut().unwrap().strength = f32::NAN;
        assert!(map_bundle(&input).is_err());
        input.materials[0].normal.as_mut().unwrap().strength = 1.;
        input.materials[0].packed_normal = false;
        assert!(map_bundle(&input).is_err());
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
