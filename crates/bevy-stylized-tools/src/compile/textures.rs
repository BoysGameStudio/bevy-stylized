//! Texture copy deduplication, base-color extraction and ILM conversion.
use super::{input::IlmChannels, plan::PendingWrite};
use crate::Diagnostic;
use bevy_stylized_core::{StylizedMaterialDescriptor, StylizedTextureSemantic};
use std::{
    collections::BTreeMap,
    io::Cursor,
    path::{Path, PathBuf},
};

pub(super) fn sanitize_file_stem(name: &str) -> String {
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

/// ILM 合成：R=metal / G=AO / B=specular-suppression / A=material-ID。
/// 源图按 luma 取单通道（L8 恒等），四源尺寸必须一致，否则 fail-closed。
pub(super) fn compose_ilm(input_dir: &Path, channels: &IlmChannels) -> Result<Vec<u8>, String> {
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

/// glTF baseColorTexture → Color 纹理引用（descriptor 已声明 Color 时跳过）。
pub(super) fn append_base_color(
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

#[derive(Default)]
pub(super) struct TextureCopies {
    copies: BTreeMap<PathBuf, String>,
    dest_names: BTreeMap<String, PathBuf>,
}
impl TextureCopies {
    pub(super) fn queue(
        &mut self,
        input_dir: &Path,
        source: &Path,
        pending: &mut Vec<PendingWrite>,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<String> {
        let source_abs = input_dir.join(source);
        if let Some(dest) = self.copies.get(&source_abs) {
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
        if let Some(other) = self.dest_names.get(&dest)
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
        self.copies.insert(source_abs.clone(), dest.clone());
        self.dest_names.insert(dest.clone(), source_abs);
        Some(format!("textures/{dest}"))
    }
}
