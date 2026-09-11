use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    mesh::{Indices, PrimitiveTopology},
    prelude::{Image, Mesh},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};
use bevy_cel::ATTRIBUTE_CEL_MATERIAL_ID;

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
            width: 1,
            height: 2,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[224, 64, 48, 255, 48, 112, 240, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image
}

fn ilm_texture() -> Image {
    Image::new_fill(
        Extent3d {
            width: 2,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[64, 255, 0, 32, 192, 128, 1, 224],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn two_region_mesh() -> Mesh {
    let positions = vec![
        [-2.0, -1.5, 0.0],
        [0.0, -1.5, 0.0],
        [0.0, 1.5, 0.0],
        [-2.0, 1.5, 0.0],
        [0.0, -1.5, 0.0],
        [2.0, -1.5, 0.0],
        [2.0, 1.5, 0.0],
        [0.0, 1.5, 0.0],
    ];
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 8]);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.25, 0.0]; 4]
            .into_iter()
            .chain(vec![[0.75, 0.0]; 4])
            .collect::<Vec<_>>(),
    );
    mesh.insert_attribute(
        ATTRIBUTE_CEL_MATERIAL_ID,
        vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0],
    );
    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]));
    mesh
}

fn validate_regions(left: [f32; 4], right: [f32; 4], mode: FixtureMode) -> bool {
    let grayscale =
        |color: [f32; 4]| (color[0] - color[1]).abs() < 0.02 && (color[1] - color[2]).abs() < 0.02;
    match mode {
        FixtureMode::Final | FixtureMode::Absent => {
            (left[0] - right[0]).abs() + (left[2] - right[2]).abs() > 0.2
        }
        FixtureMode::Specular | FixtureMode::Emission | FixtureMode::RampRow => {
            grayscale(left) && grayscale(right) && right[0] - left[0] > 0.1
        }
        FixtureMode::Ao => grayscale(left) && grayscale(right) && left[0] - right[0] > 0.05,
        FixtureMode::MaterialId => left[0] < 0.05 && right[0] > 0.5,
    }
}
