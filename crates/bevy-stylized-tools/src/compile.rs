//! `bevy-stylized compile`：Asset Compiler：输入解释、内存规划与候选输出。
//!
//! 输入 contract-conforming `.glb` / `.gltf`（compile 蕴含 validate，
//! fail-closed），输出到 `--out DIR`（默认 `<input-stem>.compiled/`）：
//!
//! - **Runtime Material**：每个 glTF material 一份
//!   `StylizedMaterialDescriptor` JSON（`materials/<name>.json`），schema 固定
//!   为 `bevy_stylized.material/v1`（v1.0）；extras/sidecar 双通道经
//!   [`bevy_stylized_gltf::import_material`] 合并，冲突 fail-closed；无元数据的 material 产出
//!   默认 descriptor（保证每个 material 都有 runtime 产物）；
//! - **Packed Maps**：material extras 的 `bevy_stylized.compile/v1` 键（或
//!   sidecar 顶层 `compile` 键）声明 ILM 打包，把四张单通道 PNG 合成一张
//!   RGBA8 PNG（`textures/<pack>.png`）：**R=metal / G=AO /
//!   B=specular-suppression / A=material-ID**（上游已核实布局，
//!   docs/reference/07 §3.2）；打包结果以
//!   [`bevy_stylized_core::StylizedTextureSemantic::ContinuousMask`] 引用写回 descriptor；
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

use crate::{Diagnostic, Format};
use std::{
    io::{self, Write},
    path::Path,
};
mod input;
mod output;
mod plan;
mod textures;

/// material extras 中 compile 指令的挂载键（provisional）。
pub const COMPILE_EXTRAS_KEY: &str = "bevy_stylized.compile/v1";

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

/// Compile from one frozen glTF byte buffer. Validation precedes planning;
/// diagnostics prevent any candidate writes. Err retains the CLI usage/I/O code.
pub fn compile_path(
    path: &Path,
    out_override: Option<&Path>,
    profile: Option<&str>,
) -> Result<CompileReport, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read: {e}"))?;
    let gltf =
        gltf::Gltf::from_slice_without_validation(&bytes).map_err(|e| format!("parse: {e}"))?;
    let validation = crate::inspect_document(path, &gltf.document, true)?;
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
        diagnostics: validation.diagnostics,
        emitted: Vec::new(),
    };
    if report.violation {
        return Ok(report);
    }
    let inputs = input::material_inputs(path, &gltf.document, profile, &mut report.diagnostics);
    let plan = plan::build(
        path,
        &gltf,
        &bytes,
        &stem,
        profile,
        inputs,
        &mut report.diagnostics,
    )?;
    report.violation = report.diagnostics.iter().any(|d| d.severity == "error");
    if !report.violation {
        output::publish(plan, &bytes, profile, &out_dir, &mut report)?;
    }
    Ok(report)
}
