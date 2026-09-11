use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::{Image, Mesh, Sphere},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};
use bevy_cel::ATTRIBUTE_CEL_EMISSION;

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

fn emission_mask_texture() -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255],
        TextureFormat::R8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image
}

fn sphere_with_emission(radius: f32, mask: Option<f32>) -> Mesh {
    let mut mesh = Sphere::new(radius).mesh().uv(48, 24);
    if let Some(mask) = mask {
        mesh.insert_attribute(ATTRIBUTE_CEL_EMISSION, vec![mask; mesh.count_vertices()]);
    }
    mesh
}

fn pixel_luma(image: &Image, x: u32, y: u32) -> Option<f32> {
    image.get_color_at(x, y).ok().map(|color| {
        let [red, green, blue, _] = color.to_srgba().to_f32_array();
        red.mul_add(0.2126, green.mul_add(0.7152, blue * 0.0722))
    })
}
