//! Sidecar directives and optional profile interpretation. No output writes.
use super::COMPILE_EXTRAS_KEY;
use crate::Diagnostic;
use bevy_stylized_core::{
    StylizedMaterialDescriptor, StylizedSchemaVersion, StylizedViewDescriptor,
};
use bevy_stylized_gltf::{load_sidecar, sidecar_path_for_gltf};
use std::{collections::BTreeMap, path::Path};
const COMPILE_SCHEMA_VERSION: StylizedSchemaVersion = StylizedSchemaVersion::new(1, 0);

/// `bevy_stylized.compile/v1` payload。
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub(super) struct CompileDirectives {
    #[serde(default)]
    pub(super) schema_version: Option<StylizedSchemaVersion>,
    #[serde(default)]
    pub(super) packs: Vec<PackDirective>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub(super) struct PackDirective {
    pub(super) name: String,
    pub(super) kind: String,
    pub(super) channels: IlmChannels,
}

/// ILM 四通道源（单通道 PNG，相对输入 glTF 目录）。
#[derive(Clone, Debug, serde::Deserialize)]
pub(super) struct IlmChannels {
    pub(super) metal: String,
    pub(super) ao: String,
    pub(super) specular_suppression: String,
    pub(super) material_id: String,
}

pub(super) fn parse_directives(
    value: &serde_json::Value,
    context: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CompileDirectives> {
    let directives: CompileDirectives = match serde_json::from_value(value.clone()) {
        Ok(directives) => directives,
        Err(error) => {
            diagnostics.push(Diagnostic {
                severity: "error",
                message: format!("{context}: {COMPILE_EXTRAS_KEY} payload 解析失败: {error}"),
            });
            return None;
        }
    };
    if let Some(version) = directives.schema_version
        && !COMPILE_SCHEMA_VERSION.can_read(version)
    {
        diagnostics.push(Diagnostic {
            severity: "error",
            message: format!(
                "{context}: compile schema 版本 {version} 超出读取能力 \
                 {COMPILE_SCHEMA_VERSION}（fail-closed）"
            ),
        });
        return None;
    }
    Some(directives)
}

pub(super) struct MaterialInputs {
    pub(super) materials: BTreeMap<String, StylizedMaterialDescriptor>,
    pub(super) directives: BTreeMap<String, CompileDirectives>,
    pub(super) view: Option<StylizedViewDescriptor>,
    /// Profile handoff 的效果网格光照基（additive optional；无 profile 或无
    /// 效果数据时为空）。
    pub(super) effect_lighting: BTreeMap<String, bevy_stylized_core::StylizedEffectLightBasis>,
}

pub(super) fn material_inputs(
    path: &Path,
    document: &gltf::Document,
    profile: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> MaterialInputs {
    // sidecar：typed 通道（descriptor）+ 原始 JSON 通道（compile 指令）。
    let sidecar_path = sidecar_path_for_gltf(path);
    let mut sidecar_materials = BTreeMap::new();
    let mut sidecar_directives: BTreeMap<String, CompileDirectives> = BTreeMap::new();
    if sidecar_path.is_file() {
        match load_sidecar(&sidecar_path) {
            Ok(sidecar) => sidecar_materials = sidecar.materials,
            Err(error) => diagnostics.push(Diagnostic {
                severity: "error",
                message: format!("sidecar {} 解析失败: {error}", sidecar_path.display()),
            }),
        }
        let raw = std::fs::read_to_string(&sidecar_path)
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
        if let Some(raw) = raw
            && let Some(compile) = raw.get("compile")
        {
            match serde_json::from_value::<BTreeMap<String, serde_json::Value>>(compile.clone()) {
                Ok(per_material) => {
                    for (name, value) in per_material {
                        let context = format!("sidecar compile[{name:?}]");
                        if let Some(directives) = parse_directives(&value, &context, diagnostics) {
                            sidecar_directives.insert(name, directives);
                        }
                    }
                }
                Err(error) => diagnostics.push(Diagnostic {
                    severity: "error",
                    message: format!(
                        "sidecar {} 的 compile 键解析失败: {error}",
                        sidecar_path.display()
                    ),
                }),
            }
        }
    }

    let mut profile_view = None;
    let mut profile_effect_lighting = BTreeMap::new();
    if let Some(profile) = profile {
        if path.extension().and_then(|x| x.to_str()) != Some("glb") {
            diagnostics.push(Diagnostic {
                severity: "error",
                message: "strict profile packaging currently requires GLB".into(),
            });
        }
        let mapped = load_profile_materials(path, profile);
        match mapped {
            Ok((materials, view, effect_lighting)) => {
                profile_effect_lighting = effect_lighting;
                profile_view = Some(view);
                let expected: std::collections::BTreeSet<_> = document
                    .materials()
                    .enumerate()
                    .map(|(i, m)| {
                        m.name()
                            .map(str::to_owned)
                            .unwrap_or_else(|| format!("material_{i}"))
                    })
                    .collect();
                if expected.len() != document.materials().len()
                    || expected != materials.keys().cloned().collect()
                {
                    diagnostics.push(Diagnostic {
                        severity: "error",
                        message: "profile material coverage differs from GLB materials".into(),
                    });
                }
                for (name, descriptor) in materials {
                    if let Some(previous) = sidecar_materials.get(&name)
                        && previous != &descriptor
                    {
                        diagnostics.push(Diagnostic {
                            severity: "error",
                            message: format!("{name}: profile/sidecar conflict"),
                        });
                    }
                    sidecar_materials.insert(name, descriptor);
                }
            }
            Err(message) => diagnostics.push(Diagnostic {
                severity: "error",
                message,
            }),
        }
    }
    MaterialInputs {
        materials: sidecar_materials,
        directives: sidecar_directives,
        view: profile_view,
        effect_lighting: profile_effect_lighting,
    }
}

fn load_profile_materials(
    path: &Path,
    profile: &str,
) -> Result<
    (
        BTreeMap<String, StylizedMaterialDescriptor>,
        bevy_stylized_core::StylizedViewDescriptor,
        BTreeMap<String, bevy_stylized_core::StylizedEffectLightBasis>,
    ),
    String,
> {
    #[cfg(feature = "profile-genshin")]
    {
        if profile != "genshin" {
            return Err(format!("unsupported profile: {profile}"));
        }
        let source = path.with_extension("genshin.json");
        let data = std::fs::read(&source).map_err(|e| format!("{}: {e}", source.display()))?;
        let input: bevy_stylized_profile_genshin::GenshinBundleInput =
            serde_json::from_slice(&data).map_err(|e| e.to_string())?;
        let materials = bevy_stylized_profile_genshin::map_bundle(&input)?;
        let scene = gltf::Gltf::open(path).map_err(|e| e.to_string())?;
        for primitive in scene.meshes().flat_map(|m| m.primitives()) {
            let Some(material) = primitive.material().name().and_then(|n| materials.get(n)) else {
                continue;
            };
            if material.static_params.get("outline_source_position") == Some(&1.0) {
                if scene.skins().next().is_some() || primitive.morph_targets().next().is_some() {
                    return Err("prepared source outline positions require a static asset".into());
                }
                let present = primitive.attributes().any(|(semantic, accessor)| {
                    matches!(semantic,gltf::Semantic::Extras(ref name) if name=="NPR_CONTROL0")
                        && accessor.dimensions() == gltf::accessor::Dimensions::Vec3
                        && accessor.data_type() == gltf::accessor::DataType::F32
                });
                if !present {
                    return Err("prepared source outline requires Float32 VEC3 _NPR_CONTROL0 on every outlined primitive".into());
                }
            }
        }
        Ok((
            materials,
            bevy_stylized_profile_genshin::map_view(&input)?,
            input.effect_lighting,
        ))
    }
    #[cfg(not(feature = "profile-genshin"))]
    {
        let _ = (path, profile);
        Err("profile-genshin feature unavailable".into())
    }
}
