//! Headless offscreen capture: render a camera into an [`Image`] target and
//! read the pixels back to the CPU. No window is ever created.

use std::collections::HashMap;

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
struct CaptureInbox(HashMap<Entity, Vec<u8>>);

/// Installs the readback-completed observer. Call once per app before
/// requesting captures.
pub fn install(app: &mut App) {
    app.init_resource::<CaptureInbox>();
    app.add_observer(
        |event: On<ReadbackComplete>,
         mut commands: Commands,
         mut inbox: bevy::prelude::ResMut<CaptureInbox>| {
            inbox.0.insert(event.entity, event.data.clone());
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
    CaptureRequest {
        entity: world.spawn(Readback::texture(target.image.clone())).id(),
        target,
    }
}

/// Returns the captured frame once the readback has landed.
///
/// The GPU copy buffer is row-padded to 256 bytes; the returned image is
/// always tightly packed RGBA8.
pub fn try_take(world: &mut World, request: CaptureRequest) -> Option<CapturedImage> {
    let data = world
        .resource_mut::<CaptureInbox>()
        .0
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
    std::thread::spawn(|| {
        let settings = WgpuSettings::default();
        let backends = settings.backends.unwrap_or_default();
        bevy::tasks::block_on(initialize_renderer(backends, None, &settings))
    })
    .join()
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

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
