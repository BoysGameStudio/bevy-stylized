//! Packed HoYoverse normal-map decode contract.

use bevy::{
    asset::RenderAssetUsages,
    image::Image,
    prelude::Handle,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_cel::{
    CharacterExtension, GENSHIN_PACKED_NORMAL_SAMPLER_BINDING,
    GENSHIN_PACKED_NORMAL_TEXTURE_BINDING,
};

fn image(format: TextureFormat) -> Image {
    Image::new_fill(
        Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[128, 128, 0, 255],
        format,
        RenderAssetUsages::default(),
    )
}

#[test]
fn packed_normal_requires_a_non_srgb_2d_texture() {
    let handle = Handle::default();
    let valid = CharacterExtension::default()
        .with_genshin_packed_normal_texture(handle.clone(), &image(TextureFormat::Rgba8Unorm));
    let invalid = CharacterExtension::default()
        .with_genshin_packed_normal_texture(handle, &image(TextureFormat::Rgba8UnormSrgb));
    assert!(valid.genshin_packed_normal_texture().is_some());
    assert!(invalid.genshin_packed_normal_texture().is_none());
}

#[test]
fn shader_reconstructs_source_z_instead_of_decoding_packed_blue() {
    assert_eq!(GENSHIN_PACKED_NORMAL_TEXTURE_BINDING, 123);
    assert_eq!(GENSHIN_PACKED_NORMAL_SAMPLER_BINDING, 124);
    let source = include_str!("../src/shaders/character_forward.wgsl");
    for token in [
        "@binding(123)",
        "@binding(124)",
        "1.0 - tangent_x * tangent_x + tangent_y * tangent_y",
        "let r1 = cross(sigma_y, in.world_normal)",
        "let r2 = cross(in.world_normal, sigma_x)",
        "v_gradient * 1000.0",
        "tangent_normal.x * tangent_u",
        "pbr_input.N = source_normal",
    ] {
        assert!(
            source.contains(token),
            "missing packed-normal contract: {token}"
        );
    }
}
