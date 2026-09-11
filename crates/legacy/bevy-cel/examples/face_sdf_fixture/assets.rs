use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    mesh::{Indices, PrimitiveTopology},
    prelude::{Image, Mesh},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};

fn render_target(size: u32) -> Image {
    let mut image = Image::new_uninit(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
    image
}

fn ramp_atlas() -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: 2,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[62, 42, 40, 255, 244, 224, 210, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image
}

fn face_threshold_map(flipped: bool) -> Image {
    const WIDTH: u32 = 256;
    const HEIGHT: u32 = 64;
    let mut pixels = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for _ in 0..HEIGHT {
        for x in 0..=u8::MAX {
            let value = if flipped { u8::MAX - x } else { x };
            pixels.extend_from_slice(&[value, value, value, 255]);
        }
    }
    let mut image = Image::new_fill(
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &pixels,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::linear();
    image
}

fn face_mesh() -> Mesh {
    const SEGMENTS: usize = 64;
    let mut positions = Vec::with_capacity(SEGMENTS + 1);
    let mut normals = Vec::with_capacity(SEGMENTS + 1);
    let mut uvs = Vec::with_capacity(SEGMENTS + 1);
    let mut indices = Vec::with_capacity(SEGMENTS * 3);
    positions.push([0.0, 0.0, 0.12]);
    normals.push([0.0, 0.0, 1.0]);
    uvs.push([0.5, 0.5]);
    let step = std::f32::consts::TAU / 64.0;
    let (step_sine, step_cosine) = step.sin_cos();
    let mut cosine = 1.0_f32;
    let mut sine = 0.0_f32;
    for ring_index in (1_u32..).take(SEGMENTS) {
        positions.push([1.65 * cosine, 1.8 * sine, 0.0]);
        normals.push([0.0, 0.0, 1.0]);
        uvs.push([cosine.mul_add(0.5, 0.5), sine.mul_add(0.5, 0.5)]);
        let next_index = if ring_index == 64 { 1 } else { ring_index + 1 };
        indices.extend_from_slice(&[0, ring_index, next_index]);
        let next_cosine = cosine.mul_add(step_cosine, -sine * step_sine);
        sine = sine.mul_add(step_cosine, cosine * step_sine);
        cosine = next_cosine;
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn validate_face_pixels(left: [f32; 4], right: [f32; 4], mode: FixtureMode) -> bool {
    let luma = |color: [f32; 4]| {
        color[2].mul_add(0.0722, color[1].mul_add(0.7152, color[0] * 0.2126))
    };
    let grayscale =
        |color: [f32; 4]| (color[0] - color[1]).abs() < 0.03 && (color[1] - color[2]).abs() < 0.03;
    match mode {
        FixtureMode::Light0 => luma(left) - luma(right) > 0.12,
        FixtureMode::Light180 => luma(right) - luma(left) > 0.12,
        FixtureMode::Light90 | FixtureMode::Vertical | FixtureMode::Fallback => {
            luma(left) > 0.12 && luma(right) > 0.12 && (luma(left) - luma(right)).abs() < 0.08
        }
        FixtureMode::DebugSelected => left[2] > 0.7 && right[2] > 0.7,
        FixtureMode::DebugThreshold => {
            grayscale(left) && grayscale(right) && (left[0] - 0.67).abs() < 0.08
        }
        FixtureMode::DebugLight => {
            left[0] > 0.78 && left[1] > 0.55 && left[1] < 0.72 && left[2] < 0.3
        }
    }
}

#[cfg(test)]
fn validate_cheek_swap(
    light_from_right: ([f32; 4], [f32; 4]),
    light_from_left: ([f32; 4], [f32; 4]),
) -> bool {
    let luma = |color: [f32; 4]| {
        color[2].mul_add(0.0722, color[1].mul_add(0.7152, color[0] * 0.2126))
    };
    luma(light_from_right.0) > luma(light_from_right.1)
        && luma(light_from_left.0) < luma(light_from_left.1)
}
