//! `bevy-stylized` CLI：validate / inspect / compile（release gate §Tools、
//! roadmap V2-f Asset Compiler 骨架）。
//!
//! 纯 glTF 解析（`gltf` crate）+ PNG 打包（`image` crate），
//! 不需要 Bevy App / GPU。

mod compile;

use std::{
    env,
    ffi::OsString,
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use bevy_stylized_gltf::{
    LEGACY_CEL_ALIASES, MATERIAL_EXTRAS_KEY, NPR_ATTRIBUTE_SPECS, NprAttributeSpec,
    StylizedImportError, extract_materials_from_document, load_sidecar, sidecar_path_for_gltf,
};

const HELP: &str = "bevy-stylized - bevy-stylized asset tooling\n\n\
Usage:\n  \
bevy-stylized validate [--json] <FILE.glb|FILE.gltf>...\n  \
bevy-stylized inspect  [--json] <FILE.glb|FILE.gltf>...\n  \
bevy-stylized compile  [--json] [--out DIR] <FILE.glb|FILE.gltf>...\n  \
bevy-stylized --help\n\n\
compile: Asset Compiler 骨架 — 先 validate（fail-closed），再输出 runtime\n\
material descriptors（materials/*.json，bevy_stylized.material/v1）与 packed\n\
maps（textures/*.png；ILM: R=metal G=AO B=specular-suppression A=material-ID）。\n\
默认输出目录：<input-stem>.compiled/\n\n\
Exit codes: 0 = ok / clean report, 1 = contract violations, 2 = usage or IO error\n";

// ---------------------------------------------------------------------------
// CLI 骨架
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Format {
    #[default]
    Human,
    Json,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Command {
    Validate,
    Inspect,
    Compile,
}

#[derive(Debug)]
enum Action {
    Help,
    Run {
        command: Command,
        format: Format,
        out: Option<PathBuf>,
        profile: Option<String>,
        paths: Vec<PathBuf>,
    },
}

fn parse(arguments: &[OsString]) -> Result<Action, String> {
    let mut format = Format::default();
    let mut command: Option<Command> = None;
    let mut out: Option<PathBuf> = None;
    let mut profile = None;
    let mut strict = false;
    let mut paths = Vec::new();
    let mut args = arguments.iter();
    while let Some(arg) = args.next() {
        let arg = arg.to_str().ok_or("non-UTF8 argument")?;
        match arg {
            "--json" => format = Format::Json,
            "--strict" => strict = true,
            "--profile" => {
                profile = Some(
                    args.next()
                        .ok_or("--profile requires a value")?
                        .to_str()
                        .ok_or("non-UTF8 profile")?
                        .to_owned(),
                );
            }
            "-h" | "--help" => return Ok(Action::Help),
            "--out" => {
                let value = args
                    .next()
                    .ok_or("--out requires a directory argument")?
                    .to_str()
                    .ok_or("non-UTF8 --out argument")?;
                out = Some(PathBuf::from(value));
            }
            _ if let Some(value) = arg.strip_prefix("--out=") => {
                out = Some(PathBuf::from(value));
            }
            "validate" | "inspect" | "compile" if command.is_none() => {
                command = Some(match arg {
                    "validate" => Command::Validate,
                    "inspect" => Command::Inspect,
                    _ => Command::Compile,
                });
            }
            _ if arg.starts_with('-') => return Err(format!("unknown flag: {arg}")),
            _ => paths.push(PathBuf::from(arg)),
        }
    }
    let Some(command) = command else {
        return Err("missing command (validate|inspect|compile)".to_string());
    };
    if out.is_some() && command != Command::Compile {
        return Err("--out is only valid for compile".to_string());
    }
    if profile.is_some() || strict {
        if command != Command::Compile || profile.as_deref() != Some("genshin") || !strict {
            return Err("use compile --profile genshin --strict".into());
        }
        if !cfg!(feature = "profile-genshin") {
            return Err("rebuild bevy-stylized-tools with --features profile-genshin".into());
        }
    }
    if paths.is_empty() {
        return Err("missing input file".to_string());
    }
    Ok(Action::Run {
        command,
        format,
        out,
        profile,
        paths,
    })
}

fn main() -> ExitCode {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    match parse(&args) {
        Ok(Action::Help) => {
            let _ = io::stdout().lock().write_all(HELP.as_bytes());
            ExitCode::SUCCESS
        }
        Ok(Action::Run {
            command,
            format,
            out,
            profile,
            paths,
        }) => {
            let mut violations = false;
            for path in &paths {
                let result = match command {
                    Command::Validate | Command::Inspect => {
                        run_one(path, command == Command::Validate).map(|report| {
                            violations |= report.violation;
                            write_report(&report, format)
                        })
                    }
                    Command::Compile => {
                        compile::compile_path(path, out.as_deref(), profile.as_deref()).map(
                            |report| {
                                violations |= report.violation;
                                compile::write_report(&report, format)
                            },
                        )
                    }
                };
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        let _ = writeln!(
                            io::stderr().lock(),
                            "bevy-stylized: {}: report write failed: {error}",
                            path.display()
                        );
                        return ExitCode::from(2);
                    }
                    Err(error) => {
                        let _ = writeln!(
                            io::stderr().lock(),
                            "bevy-stylized: {}: {error}",
                            path.display()
                        );
                        return ExitCode::from(2);
                    }
                }
            }
            if violations {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "bevy-stylized: {error}\n{HELP}");
            ExitCode::from(2)
        }
    }
}

// ---------------------------------------------------------------------------
// 诊断模型
// ---------------------------------------------------------------------------

/// 单条诊断。
#[derive(Clone, Debug, serde::Serialize)]
pub(crate) struct Diagnostic {
    pub severity: &'static str, // "error" | "warning" | "info"
    pub message: String,
}

/// 单文件报告。
#[derive(Clone, Debug, serde::Serialize)]
struct FileReport {
    path: String,
    command: &'static str,
    /// validate：是否有 error 级诊断（exit 1）。
    violation: bool,
    diagnostics: Vec<Diagnostic>,
    /// inspect：属性发现（语义名 → (类型, 覆盖 primitive 数)）。
    #[serde(skip_serializing_if = "Vec::is_empty")]
    attributes: Vec<AttributeFinding>,
    /// inspect：场景计数。
    #[serde(skip_serializing_if = "Option::is_none")]
    counts: Option<SceneCounts>,
    /// inspect：可驱动的 surface class（启发式，见 inspect() 文档）。
    #[serde(skip_serializing_if = "Vec::is_empty")]
    classes: Vec<String>,
    /// inspect：namespaced extras 摘要。
    #[serde(skip_serializing_if = "Vec::is_empty")]
    extras: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize)]
struct AttributeFinding {
    semantic: String,
    data_type: String,
    accessor_type: String,
    primitives: usize,
}

#[derive(Clone, Debug, serde::Serialize)]
struct SceneCounts {
    meshes: usize,
    primitives: usize,
    skins: usize,
    animations: usize,
    materials: usize,
}

fn write_report(report: &FileReport, format: Format) -> io::Result<()> {
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
            writeln!(out, "== {} ({}) ==", report.path, report.command)?;
            for diagnostic in &report.diagnostics {
                writeln!(out, "  [{}] {}", diagnostic.severity, diagnostic.message)?;
            }
            for finding in &report.attributes {
                writeln!(
                    out,
                    "  attr {}: {}/{} ({} primitives)",
                    finding.semantic, finding.accessor_type, finding.data_type, finding.primitives
                )?;
            }
            if let Some(counts) = &report.counts {
                writeln!(
                    out,
                    "  meshes={} primitives={} skins={} animations={} materials={}",
                    counts.meshes,
                    counts.primitives,
                    counts.skins,
                    counts.animations,
                    counts.materials
                )?;
            }
            if !report.classes.is_empty() {
                writeln!(out, "  drivable classes: {}", report.classes.join(", "))?;
            }
            for extra in &report.extras {
                writeln!(out, "  extras: {extra}")?;
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// validate / inspect 核心
// ---------------------------------------------------------------------------

/// 契约条目查找（gltf-json 语义名剥掉首下划线；注册表键即剥线名）。
fn contract_entry(loader_key: &str) -> Option<&'static NprAttributeSpec> {
    NPR_ATTRIBUTE_SPECS
        .iter()
        .find(|registration| registration.loader_key == loader_key)
}

/// legacy `_CEL_*` 语义 → 规范化 `_NPR_*`（别名表数据；仅提示，不注册）。
fn legacy_alias_note(loader_key: &str) -> Option<&'static str> {
    let cel = format!("_{loader_key}");
    LEGACY_CEL_ALIASES
        .iter()
        .find(|alias| alias.cel_semantic == Some(cel.as_str()))
        .map(|alias| alias.npr_semantic)
}

pub(crate) fn run_one(path: &Path, validate: bool) -> Result<FileReport, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read: {e}"))?;
    let gltf =
        gltf::Gltf::from_slice_without_validation(&bytes).map_err(|e| format!("parse: {e}"))?;
    inspect_document(path, &gltf.document, validate)
}

/// Shared inspection of an already parsed document; compile reuses its input.
pub(crate) fn inspect_document(
    path: &Path,
    document: &gltf::Document,
    validate: bool,
) -> Result<FileReport, String> {
    let mut report = FileReport {
        path: path.display().to_string(),
        command: if validate { "validate" } else { "inspect" },
        violation: false,
        diagnostics: Vec::new(),
        attributes: Vec::new(),
        counts: None,
        classes: Vec::new(),
        extras: Vec::new(),
    };

    // -- attributes ------------------------------------------------------
    let mut found: Vec<(String, String, String, usize)> = Vec::new(); // (key, dtype, atype, count)
    let mut primitives_total = 0usize;
    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            primitives_total += 1;
            for (semantic, accessor) in primitive.attributes() {
                let gltf::Semantic::Extras(name) = semantic else {
                    continue;
                };
                let data_type = format!("{:?}", accessor.data_type());
                let accessor_type = format!("{:?}", accessor.dimensions());
                if let Some(entry) = contract_entry(&name) {
                    // 契约：存在的 _NPR_* 若 accessor 形态不符 = fail-closed。
                    let expected = match entry.blender_data_type {
                        "FLOAT" => ("F32", "Scalar"),
                        "FLOAT_VECTOR" => ("F32", "Vec3"),
                        other => panic!("unknown contract type {other}"),
                    };
                    if (data_type.as_str(), accessor_type.as_str()) != expected {
                        report.diagnostics.push(Diagnostic {
                            severity: "error",
                            message: format!(
                                "{}: accessor 形态 {data_type}x{accessor_type} 与契约 {} 不符",
                                entry.authored_semantic, entry.blender_data_type
                            ),
                        });
                    }
                } else if let Some(npr) = legacy_alias_note(&name) {
                    report.diagnostics.push(Diagnostic {
                        severity: "warning",
                        message: format!(
                            "发现 legacy 语义 _{name}（对应 {npr}）；迁移期 shim 未注册，建议改用 {npr}"
                        ),
                    });
                }
                if let Some(existing) = found.iter_mut().find(|(k, ..)| *k == name) {
                    existing.3 += 1;
                } else {
                    found.push((name, data_type, accessor_type, 1));
                }
            }
        }
    }
    // warn-if-missing（契约 v1：全部 warn 级）。
    for entry in NPR_ATTRIBUTE_SPECS {
        if !found.iter().any(|(k, ..)| k == entry.loader_key) {
            report.diagnostics.push(Diagnostic {
                severity: "warning",
                message: format!(
                    "缺少可选属性 {}（fallback: {}）",
                    entry.authored_semantic, entry.missing_fallback
                ),
            });
        }
    }
    report.attributes = found
        .into_iter()
        .map(
            |(semantic, data_type, accessor_type, primitives)| AttributeFinding {
                semantic: format!("_{semantic}"),
                data_type,
                accessor_type,
                primitives,
            },
        )
        .collect();

    // -- extras / sidecar ------------------------------------------------
    match extract_materials_from_document(document) {
        Ok(materials) => {
            for (name, descriptor) in &materials {
                report.extras.push(format!(
                    "material {name}: {} (class {:?})",
                    MATERIAL_EXTRAS_KEY, descriptor.surface_class
                ));
            }
        }
        Err(StylizedImportError::UnsupportedSchemaVersion {
            namespace,
            found,
            supported,
        }) => {
            report.diagnostics.push(Diagnostic {
                severity: "error",
                message: format!(
                    "extras schema 版本 {found} 超出读取能力 {supported}（{namespace}，fail-closed）"
                ),
            });
        }
        Err(error) => {
            report.diagnostics.push(Diagnostic {
                severity: "error",
                message: format!("extras 解析失败: {error}"),
            });
        }
    }
    let sidecar = sidecar_path_for_gltf(path);
    if sidecar.is_file() {
        match load_sidecar(&sidecar) {
            Ok(parsed) => {
                report.extras.push(format!(
                    "sidecar {}: {} materials",
                    sidecar.display(),
                    parsed.materials.len()
                ));
            }
            Err(error) => {
                report.diagnostics.push(Diagnostic {
                    severity: "error",
                    message: format!("sidecar {} 解析失败: {error}", sidecar.display()),
                });
            }
        }
    }

    // -- inspect 补充 ----------------------------------------------------
    if !validate {
        report.counts = Some(SceneCounts {
            meshes: document.meshes().count(),
            primitives: primitives_total,
            skins: document.skins().count(),
            animations: document.animations().count(),
            materials: document.materials().count(),
        });
        // class 启发式（诚实标注）：
        // - Cel：任何 mesh 资产都能驱动（base color + 光照即可）；
        // - Character：存在 face SDF 输入（_NPR_FACE_SDF 属性或 extras 声明）；
        // - Hero：存在 ILM/控制输入（_NPR_MASK 或 extras 声明 mask 纹理语义）。
        report.classes.push("Cel".to_string());
        let has_face = report
            .attributes
            .iter()
            .any(|a| a.semantic == "_NPR_FACE_SDF")
            || report.extras.iter().any(|e| e.contains("face"));
        if has_face {
            report.classes.push("Character".to_string());
        }
        let has_ilm = report
            .attributes
            .iter()
            .any(|a| a.semantic == "_NPR_MASK" || a.semantic == "_NPR_HAIR_TANGENT");
        if has_ilm {
            report.classes.push("Hero".to_string());
        }
    }

    report.violation = report.diagnostics.iter().any(|d| d.severity == "error");
    Ok(report)
}
