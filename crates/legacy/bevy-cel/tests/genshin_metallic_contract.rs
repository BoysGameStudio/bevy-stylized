use bevy::{
    asset::RenderAssetUsages,
    image::Image,
    prelude::Handle,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_cel::{
    CharacterExtension, GENSHIN_METAL_MAP_SAMPLER_BINDING, GENSHIN_METAL_MAP_TEXTURE_BINDING,
    GENSHIN_METALLIC_UNIFORM_BINDING, GENSHIN_SPECULAR_RAMP_SAMPLER_BINDING,
    GENSHIN_SPECULAR_RAMP_TEXTURE_BINDING, GenshinMetallicSettings,
};

fn image(width: u32, height: u32, format: TextureFormat) -> Image {
    Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[128, 128, 128, 255],
        format,
        RenderAssetUsages::default(),
    )
}

#[test]
fn source_defaults_are_explicit_and_disabled_until_requested() {
    let settings = GenshinMetallicSettings::default();
    assert!(!settings.enabled());
    assert!(!settings.has_textures());
    assert_eq!(settings.matcap_shape(), (3.0, 1.0));
    assert_eq!(settings.specular_shape(), [4.0, 1.2, 0.2, 0.95]);
    let (dark, light, shadow) = settings.matcap_colors();
    assert_eq!(dark[..3], [0.647_058_8, 0.574_871_7, 0.542_387_5]);
    assert_eq!(light[..3], [0.985_293_9; 3]);
    assert_eq!(shadow[..3], [0.935_831_1, 0.885_813_2, 0.941_176_5]);
}

#[test]
fn texture_pair_requires_srgb_matcap_and_linear_specular_ramp() {
    let metal = image(256, 256, TextureFormat::Rgba8UnormSrgb);
    let specular = image(256, 2, TextureFormat::Rgba8Unorm);
    let extension = CharacterExtension::default()
        .with_genshin_metallic_textures((Handle::default(), &metal), (Handle::default(), &specular))
        .with_genshin_metallic_settings(GenshinMetallicSettings::default().with_enabled(true));
    assert!(extension.genshin_metallic_textures().is_some());
    assert!(extension.genshin_metallic_settings().enabled());
    assert!(extension.genshin_metallic_settings().has_textures());

    let invalid = CharacterExtension::default()
        .with_genshin_metallic_textures(
            (
                Handle::default(),
                &image(256, 256, TextureFormat::Rgba8Unorm),
            ),
            (Handle::default(), &specular),
        )
        .with_genshin_metallic_settings(GenshinMetallicSettings::default().with_enabled(true));
    assert!(invalid.genshin_metallic_textures().is_none());
    assert!(!invalid.genshin_metallic_settings().has_textures());
}

#[test]
fn shader_bindings_and_strict_metallic_split_are_locked() {
    assert_eq!(GENSHIN_METAL_MAP_TEXTURE_BINDING, 118);
    assert_eq!(GENSHIN_METAL_MAP_SAMPLER_BINDING, 119);
    assert_eq!(GENSHIN_SPECULAR_RAMP_TEXTURE_BINDING, 120);
    assert_eq!(GENSHIN_SPECULAR_RAMP_SAMPLER_BINDING, 121);
    assert_eq!(GENSHIN_METALLIC_UNIFORM_BINDING, 122);

    let source = include_str!("../src/shaders/character_forward.wgsl");
    for contract in [
        "@binding(118)",
        "@binding(119)",
        "@binding(120)",
        "@binding(121)",
        "@binding(122)",
        "ilm.specular_strength > 0.9",
        "let source_ray = resolved_genshin_source_direction(-source_normal)",
        "response > genshin_metallic.specular.w",
        "sampled * genshin_metallic.matcap.x",
        "cross(view_direction, -camera_up)",
    ] {
        assert!(
            source.contains(contract),
            "missing shader contract: {contract}"
        );
    }
}
