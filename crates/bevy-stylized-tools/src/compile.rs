//! `bevy-stylized compile`：Asset Compiler 骨架（roadmap V2 §Asset Compiler /
//! docs/roadmap/02-iteration-plan.md §8 V2-f）。
//!
//! 输入 contract-conforming `.glb` / `.gltf`（compile 蕴含 validate，
//! fail-closed），输出到 `--out DIR`（默认 `<input-stem>.compiled/`）：
//!
//! - **Runtime Material**：每个 glTF material 一份
//!   `StylizedMaterialDescriptor` JSON（`materials/<name>.json`），schema 固定
//!   为 `bevy_stylized.material/v1`（v1.0）；extras/sidecar 双通道经
//!   [`import_material`] 合并，冲突 fail-closed；无元数据的 material 产出
//!   默认 descriptor（保证每个 material 都有 runtime 产物）；
//! - **Packed Maps**：material extras 的 `bevy_stylized.compile/v1` 键（或
//!   sidecar 顶层 `compile` 键）声明 ILM 打包，把四张单通道 PNG 合成一张
//!   RGBA8 PNG（`textures/<pack>.png`）：**R=metal / G=AO /
//!   B=specular-suppression / A=material-ID**（上游已核实布局，
//!   docs/reference/07 §3.2）；打包结果以
//!   [`StylizedTextureSemantic::ContinuousMask`] 引用写回 descriptor；
//! - **纹理引用改写**：descriptor 已声明的纹理与 glTF baseColorTexture 复制
//!   进 `textures/`，uri 改写为相对输出目录的路径。
//!
//! `bevy_stylized.compile/v1` payload（provisional，随 Asset Compiler 契约
//! 冻结）：
//!
//! ```json
//! {
//!   "schema_version": { "major": 1, "minor": 0 },
//!   "packs": [{
//!     "name": "body_ilm",
//!     "kind": "ilm",
//!     "channels": {
//!       "metal": "textures/body_metal.png",
//!       "ao": "textures/body_ao.png",
//!       "specular_suppression": "textures/body_spec.png",
//!       "material_id": "textures/body_id.png"
//!     }
//!   }]
//! }
//! ```
//!
//! 所有内容错误（缺源文件、尺寸不一致、未知 pack kind、schema major 不符）
//! 都是 error 级诊断：exit 1 且**不写出任何产物**（先规划后落盘）。

use std::{
    collections::BTreeMap,
    io::{self, Cursor, Write},
    path::{Path, PathBuf},
};

use bevy_stylized_core::{
    SCHEMA_ID_MATERIAL_V1, StylizedMaterialDescriptor, StylizedSchemaVersion,
    StylizedTextureSemantic,
};
use bevy_stylized_gltf::{
    StylizedImportSeverity, import_material, load_sidecar, sidecar_path_for_gltf,
};
use sha2::{Digest, Sha256};

use crate::{Diagnostic, Format};

/// material extras 中 compile 指令的挂载键（provisional）。
pub const COMPILE_EXTRAS_KEY: &str = "bevy_stylized.compile/v1";

/// compile 指令 payload 支持的 schema 版本。
const COMPILE_SCHEMA_VERSION: StylizedSchemaVersion = StylizedSchemaVersion::new(1, 0);

// ---------------------------------------------------------------------------
// 报告
// ---------------------------------------------------------------------------

/// 单个编译产物（material descriptor JSON / 纹理文件）。
#[derive(Clone, Debug, serde::Serialize)]
pub struct EmittedFile {
    /// 产物类别：`material` / `texture`。
    pub kind: &'static str,
    /// 相对输出目录的路径（`/` 分隔）。
    pub path: String,
    /// 内容的 SHA-256（hex，与 fixtures provenance 约定一致）。
    pub sha256: String,
    /// 字节数。
    pub bytes: u64,
}

/// 单文件 compile 报告。
#[derive(Clone, Debug, serde::Serialize)]
pub struct CompileReport {
    pub path: String,
    pub command: &'static str,
    /// 是否有 error 级诊断（exit 1）；为 true 时不产生任何输出文件。
    pub violation: bool,
    /// 输出目录。
    pub out_dir: String,
    pub diagnostics: Vec<Diagnostic>,
    /// 已写出的产物清单（含哈希）。
    pub emitted: Vec<EmittedFile>,
}

pub fn write_report(report: &CompileReport, format: Format) -> io::Result<()> {
    let mut out = io::stdout().lock();
    match format {
        Format::Json => {
            writeln!(
                out,
                "{}",
                serde_json::to_string_pretty(report).expect("json")
            )?;
        }
        Format::Human => {
            writeln!(out, "== {} (compile) ==", report.path)?;
            writeln!(out, "  out: {}", report.out_dir)?;
            for diagnostic in &report.diagnostics {
                writeln!(out, "  [{}] {}", diagnostic.severity, diagnostic.message)?;
            }
            for emitted in &report.emitted {
                writeln!(
                    out,
                    "  emitted {} {} ({} B, sha256 {})",
                    emitted.kind, emitted.path, emitted.bytes, emitted.sha256
                )?;
            }
            if report.violation {
                writeln!(out, "  failed closed: no artifacts written")?;
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// compile 指令（provisional schema）
// ---------------------------------------------------------------------------

/// `bevy_stylized.compile/v1` payload。
#[derive(Clone, Debug, Default, serde::Deserialize)]
struct CompileDirectives {
    #[serde(default)]
    schema_version: Option<StylizedSchemaVersion>,
    #[serde(default)]
    packs: Vec<PackDirective>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct PackDirective {
    name: String,
    kind: String,
    channels: IlmChannels,
}

/// ILM 四通道源（单通道 PNG，相对输入 glTF 目录）。
#[derive(Clone, Debug, serde::Deserialize)]
struct IlmChannels {
    metal: String,
    ao: String,
    specular_suppression: String,
    material_id: String,
}

fn parse_directives(
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

// ---------------------------------------------------------------------------
// 打包 / 文件规划
// ---------------------------------------------------------------------------

/// 待写出的文件（先规划后落盘：有 error 时整批不写）。
struct PendingWrite {
    kind: &'static str,
    /// 相对输出目录。
    rel_path: PathBuf,
    bytes: Vec<u8>,
}

fn sanitize_file_stem(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "unnamed".to_string()
    } else {
        cleaned
    }
}

fn path_to_slash(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// ILM 合成：R=metal / G=AO / B=specular-suppression / A=material-ID。
/// 源图按 luma 取单通道（L8 恒等），四源尺寸必须一致，否则 fail-closed。
fn compose_ilm(input_dir: &Path, channels: &IlmChannels) -> Result<Vec<u8>, String> {
    let load = |rel: &str, semantic: &str| -> Result<image::GrayImage, String> {
        let path = input_dir.join(rel);
        let image = image::open(&path)
            .map_err(|e| format!("ILM {semantic} 源 {} 读取失败: {e}", path.display()))?;
        Ok(image.to_luma8())
    };
    let metal = load(&channels.metal, "metal(R)")?;
    let ao = load(&channels.ao, "AO(G)")?;
    let specular = load(&channels.specular_suppression, "specular-suppression(B)")?;
    let material_id = load(&channels.material_id, "material-ID(A)")?;

    let dims = metal.dimensions();
    for (semantic, image) in [
        ("AO(G)", &ao),
        ("specular-suppression(B)", &specular),
        ("material-ID(A)", &material_id),
    ] {
        if image.dimensions() != dims {
            return Err(format!(
                "ILM 通道尺寸不一致：metal(R) {}x{} vs {semantic} {}x{}",
                dims.0,
                dims.1,
                image.dimensions().0,
                image.dimensions().1
            ));
        }
    }

    let (width, height) = dims;
    let mut packed = image::RgbaImage::new(width, height);
    for (x, y, pixel) in packed.enumerate_pixels_mut() {
        *pixel = image::Rgba([
            metal.get_pixel(x, y)[0],
            ao.get_pixel(x, y)[0],
            specular.get_pixel(x, y)[0],
            material_id.get_pixel(x, y)[0],
        ]);
    }
    let mut bytes = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(packed)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|e| format!("ILM PNG 编码失败: {e}"))?;
    Ok(bytes.into_inner())
}

// ---------------------------------------------------------------------------
// 主流程
// ---------------------------------------------------------------------------

/// 编译单个输入文件。`Err` = usage/IO 级失败（exit 2）；诊断级失败通过
/// `report.violation` 表达（exit 1，且不写出任何产物）。
pub fn compile_path(
    path: &Path,
    out_override: Option<&Path>,
    profile: Option<&str>,
) -> Result<CompileReport, String> {
    // compile 蕴含 validate：契约违规 fail-closed，直接不产出。
    let validation = crate::run_one(path, true)?;
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "asset".to_string());
    let out_dir = out_override
        .map(Path::to_path_buf)
        .unwrap_or_else(|| path.with_file_name(format!("{stem}.compiled")));
    let mut report = CompileReport {
        path: path.display().to_string(),
        command: "compile",
        violation: validation.violation,
        out_dir: out_dir.display().to_string(),
        diagnostics: validation.diagnostics.clone(),
        emitted: Vec::new(),
    };
    if report.violation {
        return Ok(report);
    }

    let input_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let bytes = std::fs::read(path).map_err(|e| format!("read: {e}"))?;
    let gltf =
        gltf::Gltf::from_slice_without_validation(&bytes).map_err(|e| format!("parse: {e}"))?;
    let document = &gltf.document;

    // sidecar：typed 通道（descriptor）+ 原始 JSON 通道（compile 指令）。
    let sidecar_path = sidecar_path_for_gltf(path);
    let mut sidecar_materials = BTreeMap::new();
    let mut sidecar_directives: BTreeMap<String, CompileDirectives> = BTreeMap::new();
    if sidecar_path.is_file() {
        match load_sidecar(&sidecar_path) {
            Ok(sidecar) => sidecar_materials = sidecar.materials,
            Err(error) => report.diagnostics.push(Diagnostic {
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
                        if let Some(directives) =
                            parse_directives(&value, &context, &mut report.diagnostics)
                        {
                            sidecar_directives.insert(name, directives);
                        }
                    }
                }
                Err(error) => report.diagnostics.push(Diagnostic {
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
    if let Some(profile) = profile {
        if path.extension().and_then(|x| x.to_str()) != Some("glb") {
            report.diagnostics.push(Diagnostic {
                severity: "error",
                message: "strict profile packaging currently requires GLB".into(),
            });
        }
        let mapped = load_profile_materials(path, profile);
        match mapped {
            Ok((materials, view)) => {
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
                    report.diagnostics.push(Diagnostic {
                        severity: "error",
                        message: "profile material coverage differs from GLB materials".into(),
                    });
                }
                for (name, descriptor) in materials {
                    if let Some(previous) = sidecar_materials.get(&name)
                        && previous != &descriptor
                    {
                        report.diagnostics.push(Diagnostic {
                            severity: "error",
                            message: format!("{name}: profile/sidecar conflict"),
                        });
                    }
                    sidecar_materials.insert(name, descriptor);
                }
            }
            Err(message) => report.diagnostics.push(Diagnostic {
                severity: "error",
                message,
            }),
        }
    }
    let mut runtime_materials = BTreeMap::new();
    let mut pending: Vec<PendingWrite> = Vec::new();
    // 纹理复制去重：源路径 → textures/ 下的目标文件名。
    let mut copies: BTreeMap<PathBuf, String> = BTreeMap::new();
    let mut dest_names: BTreeMap<String, PathBuf> = BTreeMap::new();

    // 把一张源纹理排队复制到 textures/，返回改写后的相对 uri。
    let mut queue_copy = |source: &Path,
                          pending: &mut Vec<PendingWrite>,
                          diagnostics: &mut Vec<Diagnostic>|
     -> Option<String> {
        let source_abs = input_dir.join(source);
        if let Some(dest) = copies.get(&source_abs) {
            return Some(format!("textures/{dest}"));
        }
        if !source_abs.is_file() {
            diagnostics.push(Diagnostic {
                severity: "error",
                message: format!("纹理源 {} 不存在", source_abs.display()),
            });
            return None;
        }
        let file_name = source_abs
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "texture.png".to_string());
        let dest = sanitize_file_stem(&file_name);
        if let Some(other) = dest_names.get(&dest)
            && *other != source_abs
        {
            diagnostics.push(Diagnostic {
                severity: "error",
                message: format!(
                    "纹理目标名冲突：{} 与 {} 都映射到 textures/{dest}",
                    other.display(),
                    source_abs.display()
                ),
            });
            return None;
        }
        let bytes = match std::fs::read(&source_abs) {
            Ok(bytes) => bytes,
            Err(error) => {
                diagnostics.push(Diagnostic {
                    severity: "error",
                    message: format!("纹理源 {} 读取失败: {error}", source_abs.display()),
                });
                return None;
            }
        };
        pending.push(PendingWrite {
            kind: "texture",
            rel_path: PathBuf::from("textures").join(&dest),
            bytes,
        });
        copies.insert(source_abs.clone(), dest.clone());
        dest_names.insert(dest.clone(), source_abs);
        Some(format!("textures/{dest}"))
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
        let import =
            match import_material(&name, extras_json.as_ref(), sidecar_materials.get(&name)) {
                Ok(import) => import,
                Err(error) => {
                    report.diagnostics.push(Diagnostic {
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
            report.diagnostics.push(Diagnostic {
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
            report.diagnostics.push(Diagnostic {
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
                report.diagnostics.push(Diagnostic {
                    severity: "warning",
                    message: format!(
                        "material {name:?}: 跳过无法外部化的纹理引用 {:?}",
                        texture.uri
                    ),
                });
                continue;
            }
            if let Some(uri) = queue_copy(
                Path::new(&texture.uri),
                &mut pending,
                &mut report.diagnostics,
            ) {
                texture.uri = uri;
            }
        }

        // glTF baseColorTexture：descriptor 未声明 Color 纹理时补上。
        append_base_color(
            &gltf,
            &material,
            index,
            &mut descriptor,
            &mut queue_copy,
            &mut pending,
            &mut report.diagnostics,
        );

        // -- compile 指令（extras + sidecar） -------------------------------
        let mut packs = Vec::new();
        if let Some(extras) = &extras_json
            && let Some(value) = extras.get(COMPILE_EXTRAS_KEY)
        {
            let context = format!("material {name:?} extras");
            if let Some(directives) = parse_directives(value, &context, &mut report.diagnostics) {
                packs.extend(directives.packs);
            }
        }
        if let Some(directives) = sidecar_directives.get(&name) {
            packs.extend(directives.packs.iter().cloned());
        }
        for pack in &packs {
            if pack.kind != "ilm" {
                report.diagnostics.push(Diagnostic {
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
                report.diagnostics.push(Diagnostic {
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
                Err(error) => report.diagnostics.push(Diagnostic {
                    severity: "error",
                    message: format!("material {name:?} pack {:?}: {error}", pack.name),
                }),
            }
        }

        runtime_materials.insert(name, descriptor.clone());
        pending_descriptor(&mut pending, &descriptor, &mut report.diagnostics);
    }

    if let Some(view) = profile_view {
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
            bytes: bytes.clone(),
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
    // -- fail-closed：有 error 不落盘 ----------------------------------------
    report.violation = report.diagnostics.iter().any(|d| d.severity == "error");
    if report.violation {
        return Ok(report);
    }

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
    for write in pending {
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
        "input_sha256":format!("{:x}",Sha256::digest(&bytes)), "emitted":report.emitted});
    std::fs::write(
        staging.path().join("compile-manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(staging.path(), &out_dir)
        .map_err(|e| format!("commit compiled candidate: {e}"))?;
    Ok(report)
}

/// glTF baseColorTexture → Color 纹理引用（descriptor 已声明 Color 时跳过）。
fn append_base_color(
    gltf: &gltf::Gltf,
    material: &gltf::Material<'_>,
    index: usize,
    descriptor: &mut StylizedMaterialDescriptor,
    queue_copy: &mut impl FnMut(&Path, &mut Vec<PendingWrite>, &mut Vec<Diagnostic>) -> Option<String>,
    pending: &mut Vec<PendingWrite>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let name = descriptor.name.clone();
    if descriptor
        .textures
        .iter()
        .any(|t| t.semantic == StylizedTextureSemantic::Color)
    {
        return;
    }
    let Some(texture) = material.pbr_metallic_roughness().base_color_texture() else {
        return;
    };
    let image = texture.texture().source();
    match image.source() {
        gltf::image::Source::Uri { uri, .. } => {
            if uri.starts_with("data:") {
                diagnostics.push(Diagnostic {
                    severity: "warning",
                    message: format!(
                        "material {name:?}: baseColorTexture 是 data: URI，compile 骨架不内联解码，跳过"
                    ),
                });
            } else if let Some(uri) = queue_copy(Path::new(uri), pending, diagnostics) {
                descriptor
                    .textures
                    .push(bevy_stylized_core::StylizedTextureRef {
                        semantic: StylizedTextureSemantic::Color,
                        uri,
                    });
            }
        }
        gltf::image::Source::View { view, mime_type } => {
            let Some(blob) = gltf.blob.as_ref() else {
                diagnostics.push(Diagnostic {
                    severity: "error",
                    message: format!(
                        "material {name:?}: baseColorTexture 引用嵌入 buffer view 但输入无 GLB blob"
                    ),
                });
                return;
            };
            let start = view.offset();
            let Some(bytes) = blob.get(start..start + view.length()) else {
                diagnostics.push(Diagnostic {
                    severity: "error",
                    message: format!("material {name:?}: baseColorTexture buffer view 越界"),
                });
                return;
            };
            let extension = match mime_type {
                "image/png" => "png",
                "image/jpeg" => "jpg",
                _ => "bin",
            };
            let stem = image
                .name()
                .map(sanitize_file_stem)
                .unwrap_or_else(|| format!("image_{index}"));
            let file_name = format!("{stem}.{extension}");
            pending.push(PendingWrite {
                kind: "texture",
                rel_path: PathBuf::from("textures").join(&file_name),
                bytes: bytes.to_vec(),
            });
            descriptor
                .textures
                .push(bevy_stylized_core::StylizedTextureRef {
                    semantic: StylizedTextureSemantic::Color,
                    uri: format!("textures/{file_name}"),
                });
        }
    }
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

fn load_profile_materials(
    path: &Path,
    profile: &str,
) -> Result<
    (
        BTreeMap<String, StylizedMaterialDescriptor>,
        bevy_stylized_core::StylizedViewDescriptor,
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
        Ok((
            bevy_stylized_profile_genshin::map_bundle(&input)?,
            bevy_stylized_profile_genshin::map_view(&input)?,
        ))
    }
    #[cfg(not(feature = "profile-genshin"))]
    {
        let _ = (path, profile);
        Err("profile-genshin feature unavailable".into())
    }
}
