//! Build a complete candidate in memory; validation errors prevent publication.
use super::{
    COMPILE_EXTRAS_KEY,
    input::{MaterialInputs, parse_directives},
    textures::{TextureCopies, append_base_color, compose_ilm, sanitize_file_stem},
};
use crate::Diagnostic;
use bevy_stylized_core::{
    SCHEMA_ID_MATERIAL_V1, StylizedMaterialDescriptor, StylizedTextureSemantic,
};
use bevy_stylized_gltf::{StylizedImportSeverity, import_material};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// 待写出的文件（先规划后落盘：有 error 时整批不写）。
pub(super) struct PendingWrite {
    pub(super) kind: &'static str,
    /// 相对输出目录。
    pub(super) rel_path: PathBuf,
    pub(super) bytes: Vec<u8>,
}

pub(super) struct CompilePlan {
    pub(super) files: Vec<PendingWrite>,
}

pub(super) fn build(
    path: &Path,
    gltf: &gltf::Gltf,
    bytes: &[u8],
    stem: &str,
    profile: Option<&str>,
    inputs: MaterialInputs,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<CompilePlan, String> {
    let document = &gltf.document;
    let input_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut runtime_materials = BTreeMap::new();
    let mut pending = Vec::new();
    let mut copies = TextureCopies::default();
    let mut queue_copy =
        |source: &Path, pending: &mut Vec<PendingWrite>, diagnostics: &mut Vec<Diagnostic>| {
            copies.queue(input_dir, source, pending, diagnostics)
        };
    for (index, material) in document.materials().enumerate() {
        let name = material
            .name()
            .map(str::to_string)
            .unwrap_or_else(|| format!("material_{index}"));
        let extras_json: Option<serde_json::Value> = material
            .extras()
            .as_ref()
            .and_then(|raw| serde_json::from_str(raw.get()).ok());
        // -- descriptor 合并（extras + sidecar 双通道） ---------------------
        let import = match import_material(&name, extras_json.as_ref(), inputs.materials.get(&name))
        {
            Ok(import) => import,
            Err(error) => {
                diagnostics.push(Diagnostic {
                    severity: "error",
                    message: format!("material {name:?}: 元数据解析失败: {error}"),
                });
                continue;
            }
        };
        let mut conflict = false;
        for diagnostic in &import.diagnostics {
            // 冲突 = 拒绝静默取舍（ADR-0020 Provisional）；compile 升级为 error。
            let is_conflict = matches!(
                diagnostic.code,
                bevy_stylized_gltf::StylizedImportDiagnosticCode::ExtrasSidecarConflict
            );
            conflict |= is_conflict;
            diagnostics.push(Diagnostic {
                severity: if is_conflict {
                    "error"
                } else {
                    match diagnostic.severity {
                        StylizedImportSeverity::Warning => "warning",
                        StylizedImportSeverity::Error => "error",
                    }
                },
                message: diagnostic.message.clone(),
            });
        }
        if conflict {
            continue;
        }
        // 无元数据：产出默认 descriptor（每个 material 都有 runtime 产物）。
        let mut descriptor = import.material.unwrap_or_default();
        descriptor.name.clone_from(&name);
        if !SCHEMA_ID_MATERIAL_V1
            .version
            .can_read(descriptor.schema_version)
        {
            diagnostics.push(Diagnostic {
                severity: "error",
                message: format!(
                    "material {name:?}: schema 版本 {} 超出读取能力 {}（fail-closed）",
                    descriptor.schema_version, SCHEMA_ID_MATERIAL_V1.version
                ),
            });
            continue;
        }
        descriptor.schema_version = SCHEMA_ID_MATERIAL_V1.version;

        // -- 纹理引用改写 ---------------------------------------------------
        for texture in &mut descriptor.textures {
            if texture.uri.is_empty() || texture.uri.starts_with("data:") {
                diagnostics.push(Diagnostic {
                    severity: "warning",
                    message: format!(
                        "material {name:?}: 跳过无法外部化的纹理引用 {:?}",
                        texture.uri
                    ),
                });
                continue;
            }
            if let Some(uri) = queue_copy(Path::new(&texture.uri), &mut pending, diagnostics) {
                texture.uri = uri;
            }
        }

        // glTF baseColorTexture：descriptor 未声明 Color 纹理时补上。
        append_base_color(
            gltf,
            &material,
            index,
            &mut descriptor,
            &mut queue_copy,
            &mut pending,
            diagnostics,
        );

        // -- compile 指令（extras + sidecar） -------------------------------
        let mut packs = Vec::new();
        if let Some(extras) = &extras_json
            && let Some(value) = extras.get(COMPILE_EXTRAS_KEY)
        {
            let context = format!("material {name:?} extras");
            if let Some(directives) = parse_directives(value, &context, diagnostics) {
                packs.extend(directives.packs);
            }
        }
        if let Some(directives) = inputs.directives.get(&name) {
            packs.extend(directives.packs.iter().cloned());
        }
        for pack in &packs {
            if pack.kind != "ilm" {
                diagnostics.push(Diagnostic {
                    severity: "error",
                    message: format!(
                        "material {name:?}: 未知 pack kind {:?}（v1 仅支持 \"ilm\"，fail-closed）",
                        pack.kind
                    ),
                });
                continue;
            }
            let out_name = format!("{}.png", sanitize_file_stem(&pack.name));
            let rel = PathBuf::from("textures").join(&out_name);
            if pending.iter().any(|w| w.rel_path == rel) {
                diagnostics.push(Diagnostic {
                    severity: "error",
                    message: format!("material {name:?}: pack 名 {:?} 重复", pack.name),
                });
                continue;
            }
            match compose_ilm(input_dir, &pack.channels) {
                Ok(bytes) => {
                    pending.push(PendingWrite {
                        kind: "texture",
                        rel_path: rel,
                        bytes,
                    });
                    let uri = format!("textures/{out_name}");
                    if !descriptor.textures.iter().any(|t| t.uri == uri) {
                        descriptor
                            .textures
                            .push(bevy_stylized_core::StylizedTextureRef {
                                semantic: StylizedTextureSemantic::ContinuousMask,
                                uri,
                            });
                    }
                }
                Err(error) => diagnostics.push(Diagnostic {
                    severity: "error",
                    message: format!("material {name:?} pack {:?}: {error}", pack.name),
                }),
            }
        }

        runtime_materials.insert(name, descriptor.clone());
        pending_descriptor(&mut pending, &descriptor, diagnostics);
    }

    if let Some(view) = inputs.view {
        pending.push(PendingWrite {
            kind: "view",
            rel_path: PathBuf::from(format!("{stem}.view.json")),
            bytes: serde_json::to_vec_pretty(&view).map_err(|e| e.to_string())?,
        });
    }
    if profile.is_some() {
        let asset_name = path.file_name().ok_or("missing asset name")?;
        pending.push(PendingWrite {
            kind: "asset",
            rel_path: PathBuf::from(asset_name),
            bytes: bytes.to_vec(),
        });
        let sidecar = bevy_stylized_gltf::StylizedSidecar {
            schema: bevy_stylized_gltf::STYLIZED_SIDECAR_SCHEMA.into(),
            materials: runtime_materials,
        };
        pending.push(PendingWrite {
            kind: "sidecar",
            rel_path: PathBuf::from(format!("{stem}.npr.json")),
            bytes: serde_json::to_vec_pretty(&sidecar).map_err(|e| e.to_string())?,
        });
    }
    Ok(CompilePlan { files: pending })
}

/// descriptor → `materials/<name>.json` 规划项。
fn pending_descriptor(
    pending: &mut Vec<PendingWrite>,
    descriptor: &StylizedMaterialDescriptor,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let file_name = format!("{}.json", sanitize_file_stem(&descriptor.name));
    match serde_json::to_vec_pretty(descriptor) {
        Ok(bytes) => pending.push(PendingWrite {
            kind: "material",
            rel_path: PathBuf::from("materials").join(file_name),
            bytes,
        }),
        Err(error) => diagnostics.push(Diagnostic {
            severity: "error",
            message: format!(
                "material {:?}: descriptor 序列化失败: {error}",
                descriptor.name
            ),
        }),
    }
}
