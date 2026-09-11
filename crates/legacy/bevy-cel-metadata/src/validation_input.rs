#![expect(
    clippy::redundant_pub_crate,
    reason = "validator internals are shared across private sibling modules"
)]

use std::{fs, path::Path};

use serde_json::Value;

use crate::{ValidationCode, ValidationDiagnostic};

const GLB_HEADER_LENGTH: usize = 12;
const GLB_CHUNK_HEADER_LENGTH: usize = 8;
const GLB_JSON_CHUNK: u32 = 0x4e4f_534a;
const GLB_BINARY_CHUNK: u32 = 0x004e_4942;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AssetKind {
    Glb,
    Gltf,
    Sidecar,
}

#[derive(Debug)]
pub(crate) struct ParsedInput {
    pub(crate) document: Value,
    pub(crate) kind: AssetKind,
    pub(crate) has_binary_chunk: bool,
}

struct JsonParseContext<'a> {
    path: &'a Path,
    kind: AssetKind,
    diagnostics: &'a mut Vec<ValidationDiagnostic>,
}

pub(crate) fn parse_input(
    path: &Path,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) -> Option<ParsedInput> {
    let Some(kind) = input_kind(path) else {
        diagnostics.push(ValidationDiagnostic::error(
            ValidationCode::InputExtensionUnsupported,
            path,
            "$file.extension",
        ));
        return None;
    };
    let Ok(bytes) = fs::read(path) else {
        diagnostics.push(ValidationDiagnostic::error(
            ValidationCode::InputUnreadable,
            path,
            "$file",
        ));
        return None;
    };
    match kind {
        AssetKind::Glb => parse_glb(path, &bytes, diagnostics),
        AssetKind::Gltf | AssetKind::Sidecar => parse_json(
            &bytes,
            &mut JsonParseContext {
                path,
                kind,
                diagnostics,
            },
        ),
    }
}

fn input_kind(path: &Path) -> Option<AssetKind> {
    let name = path.file_name()?.to_string_lossy();
    if name.ends_with(".cel.json") {
        Some(AssetKind::Sidecar)
    } else {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("glb") => Some(AssetKind::Glb),
            Some("gltf") => Some(AssetKind::Gltf),
            Some(_) | None => None,
        }
    }
}

fn parse_json(bytes: &[u8], context: &mut JsonParseContext<'_>) -> Option<ParsedInput> {
    let Ok(document) = serde_json::from_slice(bytes) else {
        context.diagnostics.push(ValidationDiagnostic::error(
            ValidationCode::JsonSyntaxInvalid,
            context.path,
            "$",
        ));
        return None;
    };
    Some(ParsedInput {
        document,
        kind: context.kind,
        has_binary_chunk: false,
    })
}

fn parse_glb(
    path: &Path,
    bytes: &[u8],
    diagnostics: &mut Vec<ValidationDiagnostic>,
) -> Option<ParsedInput> {
    if bytes.len() < GLB_HEADER_LENGTH {
        diagnostics.push(ValidationDiagnostic::error(
            ValidationCode::GlbHeaderTruncated,
            path,
            "$glb.header",
        ));
        return None;
    }
    if bytes.get(0..4) != Some(b"glTF") {
        diagnostics.push(ValidationDiagnostic::error(
            ValidationCode::GlbMagicInvalid,
            path,
            "$glb.header.magic",
        ));
        return None;
    }
    if read_u32(bytes, 4) != Some(2) {
        diagnostics.push(ValidationDiagnostic::error(
            ValidationCode::GlbVersionUnsupported,
            path,
            "$glb.header.version",
        ));
        return None;
    }
    if read_u32(bytes, 8).and_then(|length| usize::try_from(length).ok()) != Some(bytes.len()) {
        diagnostics.push(ValidationDiagnostic::error(
            ValidationCode::GlbLengthInvalid,
            path,
            "$glb.header.length",
        ));
        return None;
    }
    parse_chunks(path, bytes, diagnostics)
}

fn parse_chunks(
    path: &Path,
    bytes: &[u8],
    diagnostics: &mut Vec<ValidationDiagnostic>,
) -> Option<ParsedInput> {
    let mut offset = GLB_HEADER_LENGTH;
    let mut json_chunk: Option<&[u8]> = None;
    let mut has_binary_chunk = false;
    while offset < bytes.len() {
        let header_end = offset.checked_add(GLB_CHUNK_HEADER_LENGTH)?;
        if header_end > bytes.len() {
            diagnostics.push(ValidationDiagnostic::error(
                ValidationCode::GlbChunkHeaderTruncated,
                path,
                format!("$glb.chunks@{offset}"),
            ));
            return None;
        }
        let chunk_length = read_u32(bytes, offset).and_then(|value| usize::try_from(value).ok())?;
        let chunk_end = header_end.checked_add(chunk_length)?;
        if chunk_end > bytes.len() {
            diagnostics.push(ValidationDiagnostic::error(
                ValidationCode::GlbChunkLengthInvalid,
                path,
                format!("$glb.chunks@{offset}.length"),
            ));
            return None;
        }
        if chunk_length % 4 != 0 {
            diagnostics.push(ValidationDiagnostic::error(
                ValidationCode::GlbChunkAlignmentInvalid,
                path,
                format!("$glb.chunks@{offset}.length"),
            ));
        }
        let chunk = bytes.get(header_end..chunk_end)?;
        match read_u32(bytes, offset + 4) {
            Some(GLB_JSON_CHUNK) if json_chunk.is_some() => diagnostics.push(
                ValidationDiagnostic::error(ValidationCode::GlbJsonDuplicate, path, "$glb.json"),
            ),
            Some(GLB_JSON_CHUNK) => json_chunk = Some(chunk),
            Some(GLB_BINARY_CHUNK) if has_binary_chunk => diagnostics.push(
                ValidationDiagnostic::error(ValidationCode::GlbBinaryDuplicate, path, "$glb.bin"),
            ),
            Some(GLB_BINARY_CHUNK) => has_binary_chunk = true,
            Some(_) | None => {}
        }
        offset = chunk_end;
    }
    let Some(json_chunk) = json_chunk else {
        diagnostics.push(ValidationDiagnostic::error(
            ValidationCode::GlbJsonMissing,
            path,
            "$glb.json",
        ));
        return None;
    };
    parse_json(
        json_chunk.trim_ascii_end(),
        &mut JsonParseContext {
            path,
            kind: AssetKind::Glb,
            diagnostics,
        },
    )
    .map(|mut input| {
        input.has_binary_chunk = has_binary_chunk;
        input
    })
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    let array = bytes.get(offset..end)?.try_into().ok()?;
    Some(u32::from_le_bytes(array))
}
