use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    mesh::{Indices, PrimitiveTopology},
    prelude::{Image, Mesh},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};
use bevy_cel::{ATTRIBUTE_CEL_HAIR_MASK, ATTRIBUTE_CEL_HAIR_TANGENT};

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

fn white_ramp() -> Image {
    let mut image = Image::new_fill(
        Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[180, 180, 180, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image
}

fn hair_strip(degenerate: bool) -> Mesh {
    let mut positions = Vec::with_capacity(10);
    let mut uvs = Vec::with_capacity(10);
    let mut masks = Vec::with_capacity(10);
    let mut normals = Vec::with_capacity(10);
    let mut tangents = Vec::with_capacity(10);
    for row in [0.0_f32, 1.0, 2.0, 3.0, 4.0] {
        let height = row.mul_add(0.65, -1.3);
        let depth = 0.12 * (row - 2.0).powi(2);
        let slope = 0.24 * (row - 2.0) / 0.65;
        let inverse_length = 1.0 / (1.0 + slope * slope).sqrt();
        for (side, x) in [(-0.7, 0.0), (0.7, 1.0)] {
            positions.push([side, height, depth]);
            uvs.push([x, row / 4.0]);
            masks.push(row / 4.0);
            normals.push([0.0, -slope * inverse_length, inverse_length]);
            tangents.push(if degenerate {
                [0.0; 3]
            } else {
                [0.0, inverse_length, slope * inverse_length]
            });
        }
    }
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(ATTRIBUTE_CEL_HAIR_TANGENT, tangents);
    mesh.insert_attribute(ATTRIBUTE_CEL_HAIR_MASK, masks);
    mesh.insert_indices(Indices::U32(vec![0,1,3,0,3,2,2,3,5,2,5,4,4,5,7,4,7,6,6,7,9,6,9,8]));
    mesh
}

fn signal_pixel_count(image: &Image) -> usize {
    let mut count = 0;
    for y in 0..SIZE {
        for x in 0..SIZE {
            if let Ok(color) = image.get_color_at(x, y) {
                let [red, green, blue, _] = color.to_srgba().to_f32_array();
                if red.max(green).max(blue) > 0.05 {
                    count += 1;
                }
            }
        }
    }
    count
}
