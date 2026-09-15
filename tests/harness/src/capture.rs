//! Headless offscreen capture: render a camera into an [`Image`] target and
//! read the pixels back to the CPU. No window is ever created.

use std::collections::{HashMap, HashSet};

use bevy::{
    asset::{Assets, RenderAssetUsages},
    camera::RenderTarget,
    prelude::{App, Bundle, Camera, Commands, Entity, Handle, Image, On, Resource, World},
    render::{
        gpu_readback::{Readback, ReadbackComplete},
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        renderer::initialize_renderer,
        settings::{RenderResources, WgpuSettings},
    },
};

/// RGBA8 bytes per pixel for the fixed capture format.
pub const CAPTURE_BYTES_PER_PIXEL: usize = 4;

/// An offscreen render target image plus its capture resolution.
#[derive(Clone, Debug)]
pub struct CaptureTarget {
    /// The render-target image handle.
    pub image: Handle<Image>,
    /// Capture width in pixels.
    pub width: u32,
    /// Capture height in pixels.
    pub height: u32,
}

/// A CPU copy of one captured frame in tightly packed RGBA8 row-major order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapturedImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Tightly packed RGBA8 pixels, `width * height * 4` bytes.
    pub data: Vec<u8>,
}

impl CapturedImage {
    /// Builds a capture from raw bytes, rejecting inconsistent sizes.
    #[must_use]
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Option<Self> {
        (data.len() == width as usize * height as usize * CAPTURE_BYTES_PER_PIXEL).then_some(Self {
            width,
            height,
            data,
        })
    }
}

/// Creates the offscreen target image every capture camera renders into.
pub fn create_capture_target(images: &mut Assets<Image>, width: u32, height: u32) -> CaptureTarget {
    let mut image = Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC;
    CaptureTarget {
        image: images.add(image),
        width,
        height,
    }
}

/// Camera components binding one camera to the offscreen target.
pub fn capture_target_bundle(target: &CaptureTarget) -> impl Bundle {
    (
        Camera::default(),
        RenderTarget::Image(target.image.clone().into()),
    )
}

#[derive(Resource, Default)]
struct CaptureInbox {
    pending: HashSet<Entity>,
    frames: HashMap<Entity, Vec<u8>>,
}

/// Installs the readback-completed observer. Call once per app before
/// requesting captures.
pub fn install(app: &mut App) {
    app.init_resource::<CaptureInbox>();
    app.add_observer(
        |event: On<ReadbackComplete>,
         mut commands: Commands,
         mut inbox: bevy::prelude::ResMut<CaptureInbox>| {
            // Several GPU copies may already be in flight when the first one
            // completes. Only the first completion belongs to this request;
            // late events must neither despawn again nor recreate consumed data.
            if !inbox.pending.remove(&event.entity) {
                return;
            }
            inbox.frames.insert(event.entity, event.data.clone());
            // `Readback` re-copies every frame while attached; one shot is enough.
            commands.entity(event.entity).despawn();
        },
    );
}

/// A pending frame capture request.
#[derive(Clone, Debug)]
pub struct CaptureRequest {
    entity: Entity,
    target: CaptureTarget,
}

/// Requests a texture readback starting on the next rendered frame.
pub fn request_capture(world: &mut World, target: CaptureTarget) -> CaptureRequest {
    let entity = world.spawn(Readback::texture(target.image.clone())).id();
    world.resource_mut::<CaptureInbox>().pending.insert(entity);
    CaptureRequest { entity, target }
}

/// Returns the captured frame once the readback has landed.
///
/// The GPU copy buffer is row-padded to 256 bytes; the returned image is
/// always tightly packed RGBA8.
pub fn try_take(world: &mut World, request: CaptureRequest) -> Option<CapturedImage> {
    let data = world
        .resource_mut::<CaptureInbox>()
        .frames
        .remove(&request.entity)?;
    CapturedImage::new(
        request.target.width,
        request.target.height,
        depad_rgba8(
            &data,
            request.target.width as usize,
            request.target.height as usize,
        ),
    )
}

/// Strips the 256-byte row alignment wgpu applies to texture copy buffers.
#[must_use]
pub fn depad_rgba8(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    let tight = width * CAPTURE_BYTES_PER_PIXEL;
    let padded = tight.div_ceil(256) * 256;
    if padded == tight {
        return data.to_vec();
    }
    let mut out = Vec::with_capacity(tight * height);
    for row in 0..height {
        out.extend_from_slice(&data[row * padded..row * padded + tight]);
    }
    out
}

/// Probes for a headless GPU render device without creating a window.
///
/// Returns `None` when no adapter is available (e.g. GPU-less CI). The probe
/// runs on a separate thread because Bevy's renderer initialization panics
/// when no adapter is found; the thread boundary turns that panic into a
/// graceful skip.
#[must_use]
pub fn probe_render_resources() -> Option<RenderResources> {
    let resources = if std::env::var("STYLIZED_GPU").as_deref() == Ok("disabled") {
        None
    } else {
        std::thread::spawn(|| {
            let settings = WgpuSettings::default();
            let backends = settings.backends.unwrap_or_default();
            bevy::tasks::block_on(initialize_renderer(backends, None, &settings))
        })
        .join()
        .ok()
    };
    assert!(
        resources.is_some() || std::env::var("STYLIZED_REQUIRE_GPU").as_deref() != Ok("1"),
        "required GPU unavailable; this is not a passed test"
    );
    resources
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_completions_cannot_replace_or_recreate_consumed_frames() {
        let mut app = App::new();
        install(&mut app);
        let request = request_capture(
            app.world_mut(),
            CaptureTarget {
                image: Handle::default(),
                width: 64,
                height: 1,
            },
        );
        let entity = request.entity;
        app.world_mut().trigger(ReadbackComplete {
            entity,
            data: vec![1; 256],
        });
        app.world_mut().trigger(ReadbackComplete {
            entity,
            data: vec![2; 256],
        });
        assert_eq!(
            try_take(app.world_mut(), request.clone()).unwrap().data,
            vec![1; 256]
        );
        app.world_mut().trigger(ReadbackComplete {
            entity,
            data: vec![3; 256],
        });
        assert!(try_take(app.world_mut(), request).is_none());
        let inbox = app.world().resource::<CaptureInbox>();
        assert!(inbox.pending.is_empty() && inbox.frames.is_empty());
    }

    #[test]
    fn depad_strips_row_alignment_padding() {
        // Given a 2x2 image whose 8-byte rows are padded to 256 bytes
        let width = 2;
        let height = 2;
        let padded_row = 256;
        let mut padded = vec![0_u8; padded_row * height];
        for (i, byte) in padded.iter_mut().enumerate() {
            *byte = i as u8;
        }

        // When
        let tight = depad_rgba8(&padded, width, height);

        // Then
        assert_eq!(tight.len(), width * height * CAPTURE_BYTES_PER_PIXEL);
        assert_eq!(tight[..8], padded[..8]);
        assert_eq!(tight[8..], padded[padded_row..padded_row + 8]);
    }

    #[test]
    fn depad_keeps_aligned_rows_untouched() {
        // Given
        let data: Vec<u8> = (0..64 * 4 * 3).map(|i| i as u8).collect();

        // When
        let out = depad_rgba8(&data, 64, 3);

        // Then
        assert_eq!(out, data);
    }

    #[test]
    fn captured_image_rejects_inconsistent_byte_count() {
        assert!(CapturedImage::new(2, 2, vec![0; 16]).is_some());
        assert!(CapturedImage::new(2, 2, vec![0; 15]).is_none());
    }
}

/// Linear floating-point target for matched offline sampling. This is not a
/// requirement of the production renderer's portable path.
pub fn create_linear_capture_target(
    images: &mut Assets<Image>,
    width: u32,
    height: u32,
) -> CaptureTarget {
    let mut image = Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0; 8],
        TextureFormat::Rgba16Float,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC;
    CaptureTarget {
        image: images.add(image),
        width,
        height,
    }
}

/// Resolve a linear HDR readback using a regular box in linear light, then encode
/// sRGB once. Each renderer must use the same declared grid and resolve policy.
pub fn try_take_linear_resolved(
    world: &mut World,
    request: CaptureRequest,
    scale: u32,
) -> Result<Option<CapturedImage>, String> {
    let Some(data) = world
        .resource_mut::<CaptureInbox>()
        .frames
        .remove(&request.entity)
    else {
        return Ok(None);
    };
    let (width, height) = (
        request.target.width as usize,
        request.target.height as usize,
    );
    let scale = scale as usize;
    if scale == 0 || !width.is_multiple_of(scale) || !height.is_multiple_of(scale) {
        return Err("invalid resolve grid".into());
    }
    let row = (width * 8).div_ceil(256) * 256;
    if data.len() != row * height {
        return Err("invalid linear readback size".into());
    }
    let mut result = Vec::with_capacity(width * height * 4 / (scale * scale));
    for y in 0..height / scale {
        for x in 0..width / scale {
            let mut sum = [0.0f64; 4];
            for dy in 0..scale {
                for dx in 0..scale {
                    let offset = (y * scale + dy) * row + (x * scale + dx) * 8;
                    for c in 0..4 {
                        let v = half::f16::from_le_bytes([
                            data[offset + c * 2],
                            data[offset + c * 2 + 1],
                        ])
                        .to_f32();
                        if !v.is_finite() {
                            return Err("nonfinite linear capture".into());
                        }
                        sum[c] += f64::from(v);
                    }
                }
            }
            for (c, v) in sum.into_iter().enumerate() {
                let linear = (v / (scale * scale) as f64).clamp(0.0, 1.0);
                let encoded = if c == 3 {
                    linear
                } else if linear <= 0.0031308 {
                    linear * 12.92
                } else {
                    1.055 * linear.powf(1.0 / 2.4) - 0.055
                };
                result.push((encoded * 255.).round() as u8);
            }
        }
    }
    Ok(CapturedImage::new(
        (width / scale) as u32,
        (height / scale) as u32,
        result,
    ))
}

#[cfg(test)]
mod linear_resolve_tests {
    use super::*;
    #[test]
    fn hdr_samples_are_averaged_before_display_clipping() {
        let mut app = App::new();
        install(&mut app);
        let request = request_capture(
            app.world_mut(),
            CaptureTarget {
                image: Handle::default(),
                width: 2,
                height: 2,
            },
        );
        let mut data = vec![0; 512];
        for y in 0..2 {
            for x in 0..2 {
                for (c, v) in [if x == 0 { 0.0 } else { 2.0 }, 0.0, 0.0, 1.0]
                    .into_iter()
                    .enumerate()
                {
                    let bytes = half::f16::from_f32(v).to_le_bytes();
                    let offset = y * 256 + x * 8 + c * 2;
                    data[offset..offset + 2].copy_from_slice(&bytes);
                }
            }
        }
        app.world_mut().trigger(ReadbackComplete {
            entity: request.entity,
            data,
        });
        let image = try_take_linear_resolved(app.world_mut(), request, 2)
            .unwrap()
            .unwrap();
        assert_eq!((image.width, image.height), (1, 1));
        assert_eq!(image.data, [255, 0, 0, 255]);
    }
}
