//! Texture reconstruction and projection-dependent shading semantics.
#[path = "support/render_scene.rs"]
mod render_scene;
use bevy::prelude::*;
use bevy_stylized::prelude::*;
use render_scene::*;

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn authored_cubic_diffuse_reconstructs_texel_neighborhood() {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let Some(resources) = golden_harness::probe_render_resources() else {
        return;
    };
    let mut app = headless_app(resources);
    let mut pixels = vec![0_u8; 4 * 4 * 4];
    for pixel in pixels.as_chunks_mut::<4>().0 {
        pixel[3] = 255;
    }
    pixels[20..24].copy_from_slice(&[255; 4]);
    let mut image = Image::new(
        Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8Unorm,
        bevy::asset::RenderAssetUsages::default(),
    );
    image.sampler = bevy::image::ImageSampler::linear();
    let handle = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    let mut descriptor = bevy_stylized::stylized_core::StylizedMaterialDescriptor::default();
    descriptor
        .static_params
        .insert("cubic_base_color".into(), 1.0);
    descriptor
        .textures
        .push(bevy_stylized::stylized_core::StylizedTextureRef {
            uri: "impulse.png".into(),
            semantic: bevy_stylized::stylized_core::StylizedTextureSemantic::Color,
        });
    let mut material = StylizedHeroMaterial::from_descriptor(&descriptor, |_| handle.clone());
    material.debug_view = StylizedHeroDebugView::BaseColor;
    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedHeroMaterial>>()
        .add(material);
    let mut mesh = quad_mesh();
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.375_f32, 0.375]; 4]);
    let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
    app.world_mut()
        .spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::default()));
    let image = capture_frame(&mut app);
    let value = image.data[((SIZE / 2 * SIZE + SIZE / 2) * 4) as usize];
    // A unit impulse at its texel center has tensor-product B-spline weight
    // (2/3)^2, displayed as sRGB 178. Bilinear reconstruction would give 255.
    assert!(
        (176..=180).contains(&value),
        "cubic impulse reconstruction: {value}"
    );
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn orthographic_specular_uses_constant_camera_basis() {
    let Some(resources) = golden_harness::probe_render_resources() else {
        return;
    };
    let mut app = headless_app(resources);
    app.world_mut()
        .resource_mut::<StylizedViewSettings>()
        .0
        .key_light_direction = Vec3::new(1., 0., 1.).normalize();
    for x in [-0.9, 0.9] {
        let mut material = StylizedHeroMaterial {
            debug_view: StylizedHeroDebugView::NormalHalfDot,
            ..Default::default()
        };
        material.params.genshin.controls.z = 1.;
        let material = app
            .world_mut()
            .resource_mut::<Assets<StylizedHeroMaterial>>()
            .add(material);
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(quad_mesh());
        app.world_mut().spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(x, 0., 0.),
        ));
    }
    let projection = bevy::camera::Projection::Orthographic(bevy::camera::OrthographicProjection {
        scaling_mode: bevy::camera::ScalingMode::FixedVertical {
            viewport_height: 3.8,
        },
        ..bevy::camera::OrthographicProjection::default_3d()
    });
    let image = capture_frame_with_projection(&mut app, projection);
    let values = [70, 186].map(|x| image.data[(128 * SIZE as usize + x) * 4]);
    assert!(
        values.iter().all(|v| (245..=247).contains(v)),
        "constant orthographic NdotH: {values:?}"
    );
    assert_eq!(values[0], values[1]);
}
