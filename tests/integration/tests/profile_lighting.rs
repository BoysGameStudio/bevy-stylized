//! Authored Genshin lighting and control-map semantics.
#[path = "support/render_scene.rs"]
mod render_scene;
use bevy::prelude::*;
use bevy_stylized::prelude::*;
use golden_harness::capture;
use render_scene::*;

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn source_emission_mixes_tinted_radiance_and_zero_strength_stays_enabled() {
    let mut app =
        headless_app(capture::probe_render_resources().expect("required GPU unavailable"));
    {
        let mut view = app.world_mut().resource_mut::<StylizedViewSettings>();
        view.0.key_light_direction = Vec3::Z;
        view.0.ambient_override = Some(LinearRgba::WHITE);
    }
    for (x, alpha, strength, enabled) in [(-1.2, 1., 2., 0.), (0., 0.5, 2., 1.), (1.2, 1., 0., 1.)]
    {
        let mut m = StylizedHeroMaterial::default();
        m.params.character.cel.base_color_factor = Vec4::new(0.6, 0.4, 0.2, alpha);
        m.params.character.palette_soft_lit = Vec4::ONE;
        m.params.character.palette_soft_shadow = Vec4::ONE;
        m.params.character.palette_sharp_lit = Vec4::ONE;
        m.params.character.palette_sharp_shadow = Vec4::ONE;
        m.params.source_emission = Vec4::new(
            0.5_f32.powf(2.2) * strength,
            0.7_f32.powf(2.2) * strength,
            0.9_f32.powf(2.2) * strength,
            enabled,
        );
        let material = app
            .world_mut()
            .resource_mut::<Assets<StylizedHeroMaterial>>()
            .add(m);
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(quad_mesh());
        app.world_mut().spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(x, 0., 0.).with_scale(Vec3::new(0.4, 1., 1.)),
        ));
    }
    let image = capture_frame_with_projection(
        &mut app,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 3.8,
            },
            ..OrthographicProjection::default_3d()
        }),
    );
    // Native source Alpha closure: C*(1-m) + base*tint^2.2*strength*m^2.
    // The third panel proves that zero gain must still attenuate the lit closure.
    for (x, mask, gain) in [(47, 0., 2.), (128, 0.47, 2.), (209, 0.97, 0.)] {
        let rgb = &image.data[(128 * SIZE as usize + x) * 4..][..3];
        for (channel, (base, tint)) in [(0.6_f32, 0.5_f32), (0.4, 0.7), (0.2, 0.9)]
            .into_iter()
            .enumerate()
        {
            let linear = base * (1. - mask) + base * tint.powf(2.2) * gain * mask * mask;
            let expected = (linear.powf(1. / 2.4) * 1.055 - 0.055) * 255.;
            assert!(
                (f32::from(rgb[channel]) - expected).abs() <= 2.,
                "source emission x={x} channel={channel}: {rgb:?}, expected {expected}"
            );
        }
    }
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn genshin_ilm_zero_ao_preserves_shadow_gate() {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let resources = capture::probe_render_resources().expect("required GPU unavailable");
    let mut app = headless_app(resources);
    app.world_mut()
        .resource_mut::<StylizedViewSettings>()
        .0
        .key_light_direction = Vec3::Z;
    app.world_mut()
        .resource_mut::<StylizedViewSettings>()
        .0
        .ambient_override = Some(LinearRgba::WHITE);
    for (x, green) in [(-0.9, 0u8), (0.9, 255u8)] {
        let image = Image::new_fill(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, green, 255, 0],
            TextureFormat::Rgba8Unorm,
            bevy::asset::RenderAssetUsages::default(),
        );
        let ilm = app.world_mut().resource_mut::<Assets<Image>>().add(image);
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(quad_mesh());
        let mut material = StylizedHeroMaterial::default();
        material.params = material.params.with_genshin_ramp(0.55, 1.0);
        material.params.character.palette_soft_shadow = bevy::math::Vec4::splat(0.1);
        material.params.character.palette_sharp_shadow = bevy::math::Vec4::splat(0.1);
        material.ilm_texture = Some(ilm);
        let material = app
            .world_mut()
            .resource_mut::<Assets<StylizedHeroMaterial>>()
            .add(material);
        app.world_mut().spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(x, 0., 0.),
        ));
    }
    let image = capture_frame(&mut app);
    let at = |x: usize| image.data[(128 * SIZE as usize + x) * 4] as i32;
    assert!(at(176) > 150, "lit control missing: {}", at(176));
    assert!(
        at(176) - at(80) > 80,
        "AO 0 must visibly gate light instead of being remapped above the smoothstep range: dark={} lit={}",
        at(80),
        at(176)
    );
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn genshin_ramp_day_night_uses_image_row_orientation() {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let resources = capture::probe_render_resources().expect("required GPU unavailable");
    let mut app = headless_app(resources);
    app.world_mut()
        .resource_mut::<StylizedViewSettings>()
        .0
        .key_light_direction = -Vec3::Z;
    app.world_mut()
        .resource_mut::<StylizedViewSettings>()
        .0
        .ambient_override = Some(LinearRgba::WHITE);
    let mut pixels = Vec::new();
    for row in 0..10 {
        pixels.extend_from_slice(if row < 5 {
            &[255, 0, 0, 255]
        } else {
            &[0, 0, 255, 255]
        });
    }
    let ramp = Image::new(
        Extent3d {
            width: 1,
            height: 10,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    );
    let ramp = app.world_mut().resource_mut::<Assets<Image>>().add(ramp);
    let ilm = Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 128, 255, 0],
        TextureFormat::Rgba8Unorm,
        bevy::asset::RenderAssetUsages::default(),
    );
    let ilm = app.world_mut().resource_mut::<Assets<Image>>().add(ilm);
    for (x, day, prepared) in [(-0.9, 1.0, false), (0.9, 0.0, false), (0.0, 0.7, true)] {
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(quad_mesh());
        let mut material = StylizedHeroMaterial::default();
        material.params = material.params.with_genshin_ramp(0.55, 1.0);
        material.params.character = material.params.character.with_day_night(day);
        material.params.source_conventions.y = f32::from(prepared);
        material.ramp_texture = Some(ramp.clone());
        material.ilm_texture = Some(ilm.clone());
        let material = app
            .world_mut()
            .resource_mut::<Assets<StylizedHeroMaterial>>()
            .add(material);
        app.world_mut().spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            // Three coplanar 1.6-wide quads spaced by 0.9 overlap at the
            // sampled pixels. Narrow them so depth ties cannot select the
            // center blend material instead of the day/night material.
            Transform::from_xyz(x, 0., 0.).with_scale(Vec3::new(0.4, 1.0, 1.0)),
        ));
    }
    let image = capture_frame(&mut app);
    let at =
        |x: usize| &image.data[(128 * SIZE as usize + x) * 4..(128 * SIZE as usize + x) * 4 + 3];
    let day = at(80);
    let night = at(176);
    assert!(
        day[0] > 100 && day[2] < 10,
        "day must sample the authored PNG top half: {day:?}"
    );
    assert!(
        night[2] > 100 && night[0] < 10,
        "night must sample the authored PNG bottom half: {night:?}"
    );
    let mixed = at(128);
    assert!(
        mixed[0] > 100 && mixed[2] > 50 && mixed[0] < day[0] && mixed[2] < night[2],
        "prepared partial page blend must interpolate both page colors: {mixed:?}"
    );
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn genshin_face_factor_uses_projected_light_and_control_map() {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let resources = capture::probe_render_resources().expect("required GPU unavailable");
    let mut app = headless_app(resources);
    app.world_mut()
        .resource_mut::<StylizedViewSettings>()
        .0
        .key_light_direction = Vec3::new(-0.65_f32.sin(), 0.6, 0.65_f32.cos()).normalize();
    let mut texture = |color: &[u8]| {
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::new_fill(
                Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                color,
                TextureFormat::Rgba8Unorm,
                bevy::asset::RenderAssetUsages::default(),
            ))
    };
    let sdf = texture(&[43, 43, 43, 255]);
    let control = texture(&[0, 255, 0, 0]);
    let mut material = StylizedHeroMaterial {
        debug_view: StylizedHeroDebugView::LightingFactors,
        ..Default::default()
    };
    material.params.hero_a.z = 0.55;
    material.params.character = material
        .params
        .character
        .with_face_basis(Vec3::Z, Vec3::Y, 0.001);
    material.face_sdf_texture = Some(sdf);
    material.surface_control_texture = Some(control);
    material.surface_control = StylizedHeroSurfaceControl::GenshinFace;
    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedHeroMaterial>>()
        .add(material);
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(quad_mesh());
    app.world_mut()
        .spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::IDENTITY));
    let image = capture_frame(&mut app);
    let pixel = &image.data[(128 * SIZE as usize + 128) * 4..(128 * SIZE as usize + 128) * 4 + 4];
    assert!(
        pixel[0] > 250 && pixel[1] > 250 && pixel[2] > 250,
        "projected Genshin threshold (~0.14) must light SDF 43/255; angular (~0.21) would shadow it: {pixel:?}"
    );
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn genshin_material_bands_follow_authored_nonsequential_order() {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let resources = capture::probe_render_resources().expect("required GPU unavailable");
    let mut app = headless_app(resources);
    app.world_mut()
        .resource_mut::<StylizedViewSettings>()
        .0
        .key_light_direction = -Vec3::Z;
    app.world_mut()
        .resource_mut::<StylizedViewSettings>()
        .0
        .ambient_override = Some(LinearRgba::WHITE);
    let colors = [
        [255u8, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 0, 255],
        [0, 255, 255, 255],
    ];
    let pixels = colors
        .iter()
        .chain(colors.iter())
        .flatten()
        .copied()
        .collect();
    let ramp = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new(
            Extent3d {
                width: 1,
                height: 10,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixels,
            TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::default(),
        ));
    for (i, alpha) in [26, 77, 128, 179, 230].into_iter().enumerate() {
        let ilm = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::new_fill(
                Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &[0, 128, 255, alpha],
                TextureFormat::Rgba8Unorm,
                bevy::asset::RenderAssetUsages::default(),
            ));
        let mut material = StylizedHeroMaterial::default();
        material.params = material.params.with_genshin_ramp(0.55, 1.);
        material.params.character = material.params.character.with_day_night(1.);
        material.params.character.palette_soft_lit = bevy::math::Vec4::ONE;
        material.params.character.palette_soft_shadow = bevy::math::Vec4::ONE;
        material.params.character.palette_sharp_lit = bevy::math::Vec4::ONE;
        material.params.character.palette_sharp_shadow = bevy::math::Vec4::ONE;
        material.ilm_texture = Some(ilm);
        material.ramp_texture = Some(ramp.clone());
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
            Transform::from_xyz(-1.6 + i as f32 * 0.8, 0., 0.).with_scale(Vec3::new(0.35, 0.6, 1.)),
        ));
    }
    let image = capture_frame(&mut app);
    // Independently observed from the locked upstream material-ID node group.
    for (x, band) in [29usize, 79, 128, 177, 227]
        .into_iter()
        .zip([1usize, 4, 3, 5, 2])
    {
        let pixel = &image.data[(128 * SIZE as usize + x) * 4..(128 * SIZE as usize + x) * 4 + 3];
        assert!(
            pixel
                .iter()
                .zip(colors[band - 1])
                .all(|(a, b)| a.abs_diff(b) <= 1),
            "wrong authored band {band}: {pixel:?}"
        );
    }
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn genshin_zero_vertex_ramp_width_uses_authored_safe_divide() {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let resources = capture::probe_render_resources().expect("required GPU unavailable");
    let mut app = headless_app(resources);
    let image = |color: &[u8]| {
        Image::new_fill(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            color,
            TextureFormat::Rgba8Unorm,
            bevy::asset::RenderAssetUsages::default(),
        )
    };
    let ilm = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(image(&[0, 0, 255, 0]));
    let ramp = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(image(&[255; 4]));
    let mut mesh = quad_mesh();
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1., 0., 0., 1.]; 4]);
    let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
    let mut material = StylizedHeroMaterial::default();
    material.params = material.params.with_genshin_ramp(0.55, 1.);
    material.debug_view = StylizedHeroDebugView::LightingFactors;
    material.ilm_texture = Some(ilm);
    material.ramp_texture = Some(ramp);
    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedHeroMaterial>>()
        .add(material);
    app.world_mut()
        .spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::IDENTITY));
    let image = capture_frame(&mut app);
    let pixel = &image.data[(128 * SIZE as usize + 128) * 4..(128 * SIZE as usize + 128) * 4 + 3];
    assert!(
        pixel.iter().all(|v| *v > 250),
        "zero width divides to zero upstream, producing ramp coordinate 1: {pixel:?}"
    );
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn prepared_ramp_zero_vertex_width_uses_explicit_denominator_floor() {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let mut app =
        headless_app(capture::probe_render_resources().expect("required GPU unavailable"));
    {
        let mut settings = app.world_mut().resource_mut::<StylizedViewSettings>();
        settings.0.key_light_direction = Vec3::NEG_Z;
        settings.0.ambient_override = Some(LinearRgba::WHITE);
    }
    let texture = |width, height, data| {
        Image::new(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8Unorm,
            bevy::asset::RenderAssetUsages::default(),
        )
    };
    let ramp = app.world_mut().resource_mut::<Assets<Image>>().add(texture(
        2,
        1,
        vec![0, 0, 255, 255, 255, 0, 0, 255],
    ));
    let ilm =
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .add(texture(1, 1, vec![0, 128, 0, 0]));
    for (x, prepared) in [(-0.9, false), (0.9, true)] {
        let mut mesh = quad_mesh();
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_COLOR,
            vec![[1., 0., 1., 1.]; mesh.count_vertices()],
        );
        let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
        let mut material = StylizedHeroMaterial::default();
        material.params = material.params.with_genshin_ramp(0.55, 1.0);
        material.params.source_conventions.y = f32::from(prepared);
        material.params.character.palette_soft_lit = Vec4::ONE;
        material.params.character.palette_soft_shadow = Vec4::ONE;
        material.params.character.palette_sharp_lit = Vec4::ONE;
        material.params.character.palette_sharp_shadow = Vec4::ONE;
        material.ilm_texture = Some(ilm.clone());
        material.ramp_texture = Some(ramp.clone());
        let handle = app
            .world_mut()
            .resource_mut::<Assets<StylizedHeroMaterial>>()
            .add(material);
        app.world_mut().spawn((
            Mesh3d(mesh),
            MeshMaterial3d(handle),
            Transform::from_xyz(x, 0., 0.),
        ));
    }
    let image = capture_frame(&mut app);
    let at = |x: usize| &image.data[(128 * SIZE as usize + x) * 4..][..3];
    assert!(
        at(80)[0] > 240 && at(80)[2] < 5,
        "raw Divide-zero convention must remain red: {:?}",
        at(80)
    );
    assert!(
        at(176)[2] > 240 && at(176)[0] < 5,
        "prepared denominator-floor convention must select blue: {:?}",
        at(176)
    );
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn source_metal_requires_explicit_enable_and_uses_authored_color() {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let mut app =
        headless_app(capture::probe_render_resources().expect("required GPU unavailable"));
    {
        let mut v = app.world_mut().resource_mut::<StylizedViewSettings>();
        v.0.key_light_direction = Vec3::Z;
        v.0.ambient_override = Some(LinearRgba::WHITE);
    }
    let tex = |rgba: &[u8]| {
        Image::new_fill(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            rgba,
            TextureFormat::Rgba8Unorm,
            bevy::asset::RenderAssetUsages::default(),
        )
    };
    let ilm = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(tex(&[255, 255, 0, 0]));
    let matcap = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(tex(&[255, 255, 255, 255]));
    let spec = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(tex(&[0, 0, 0, 255]));
    for (x, enabled) in [(-0.9, false), (0.9, true)] {
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(quad_mesh());
        let mut m = StylizedHeroMaterial {
            ilm_texture: Some(ilm.clone()),
            matcap_texture: Some(matcap.clone()),
            spec_ramp_texture: Some(spec.clone()),
            ..Default::default()
        };
        m.params.genshin.controls = Vec4::new(1., 1., 1., 0.);
        for row in &mut m.params.genshin.specular_rows {
            row.y = 0.;
        }
        m.params.character.palette_soft_lit = Vec4::ONE;
        m.params.character.palette_soft_shadow = Vec4::ONE;
        m.params.character.palette_sharp_lit = Vec4::ONE;
        m.params.character.palette_sharp_shadow = Vec4::ONE;
        m.params.metal.controls.x = f32::from(enabled);
        m.params.metal.light = Vec4::new(1., 0., 0., 1.);
        m.params.metal.specular.w = 2.;
        let material = app
            .world_mut()
            .resource_mut::<Assets<StylizedHeroMaterial>>()
            .add(m);
        app.world_mut().spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(x, 0., 0.),
        ));
    }
    let image = capture_frame(&mut app);
    let at = |x: usize| &image.data[(128 * SIZE as usize + x) * 4..][..3];
    assert!(
        at(80).iter().all(|v| *v > 240),
        "disabled source branch must remain neutral: {:?}",
        at(80)
    );
    assert!(
        at(176)[0] > 240 && at(176)[1] < 5 && at(176)[2] < 5,
        "authored matcap tint missing: {:?}",
        at(176)
    );
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn source_metal_ramp_uses_linear_data_blue_mask_and_zero_half_vector() {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let mut app =
        headless_app(capture::probe_render_resources().expect("required GPU unavailable"));
    {
        let mut view = app.world_mut().resource_mut::<StylizedViewSettings>();
        view.0.key_light_direction = -Vec3::Z;
        view.0.ambient_override = Some(LinearRgba::WHITE);
    }
    let mut tex = |color: &[u8]| {
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::new_fill(
                Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                color,
                TextureFormat::Rgba8Unorm,
                bevy::asset::RenderAssetUsages::default(),
            ))
    };
    let ilm = tex(&[255, 255, 128, 0]);
    let matcap = tex(&[0, 0, 0, 255]);
    let ramp = tex(&[0, 0, 255, 255]);
    for (x, enabled) in [(-0.9, false), (0.9, true)] {
        let mut material = StylizedHeroMaterial {
            ilm_texture: Some(ilm.clone()),
            matcap_texture: Some(matcap.clone()),
            spec_ramp_texture: Some(ramp.clone()),
            ..Default::default()
        };
        material.params.genshin.controls = Vec4::new(1., 0., 1., 0.);
        material.params.character.palette_soft_lit = Vec4::ONE;
        material.params.character.palette_soft_shadow = Vec4::ONE;
        material.params.character.palette_sharp_lit = Vec4::ONE;
        material.params.character.palette_sharp_shadow = Vec4::ONE;
        material.params.metal.controls = Vec4::new(1., 1., 1., f32::from(enabled));
        material.params.metal.dark = Vec4::ZERO;
        material.params.metal.light = Vec4::ZERO;
        material.params.metal.specular = Vec4::new(4., 1.2, 1., 0.95);
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
    let image = capture_frame_with_projection(
        &mut app,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 3.8,
            },
            ..OrthographicProjection::default_3d()
        }),
    );
    let at = |x: usize| &image.data[(128 * SIZE as usize + x) * 4..][..3];
    assert!(
        at(70).iter().all(|v| *v <= 2),
        "zero half-vector must not create a sharp highlight: {:?}",
        at(70)
    );
    assert!(
        at(186)[0] <= 2 && at(186)[1] <= 2 && (186..=189).contains(&at(186)[2]),
        "ramp replaces scalar response and is multiplied by linear ILM blue 128/255: {:?}",
        at(186)
    );
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn hero_material_and_instance_fallback_disable_source_extras_after_sync() {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    use bevy_stylized_render::{
        hero::apply_hero_portable_fallback, lod::INSTANCE_FLAG_DROP_HERO_EXTRAS,
    };
    let mut app =
        headless_app(capture::probe_render_resources().expect("required GPU unavailable"));
    // Hold explicit quality inputs; the distance-policy system owns these same
    // bits and is exercised independently by the LOD tests.
    app.world_mut()
        .remove_resource::<bevy_stylized_render::lod::StylizedLodPolicyResource>();
    {
        let mut view = app.world_mut().resource_mut::<StylizedViewSettings>();
        view.0.key_light_direction = Vec3::Z;
        view.0.ambient_override = Some(LinearRgba::WHITE);
    }
    let mut texture = |rgba: &[u8]| {
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::new_fill(
                Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                rgba,
                TextureFormat::Rgba8Unorm,
                bevy::asset::RenderAssetUsages::default(),
            ))
    };
    let ilm = texture(&[255, 255, 255, 0]);
    let matcap = texture(&[255; 4]);
    let ramp = texture(&[0, 0, 255, 255]);
    let normal = texture(&[255, 128, 0, 255]);
    for row in 0..3 {
        for column in 0..3 {
            let mut m = StylizedHeroMaterial {
                ilm_texture: Some(ilm.clone()),
                matcap_texture: Some(matcap.clone()),
                spec_ramp_texture: Some(ramp.clone()),
                ..Default::default()
            };
            m.params.genshin.controls = Vec4::new(1., 1., 1., 0.);
            for band in &mut m.params.genshin.specular_rows {
                band.y = 0.;
            }
            m.params.character.palette_soft_lit = Vec4::ONE;
            m.params.character.palette_soft_shadow = Vec4::ONE;
            m.params.character.palette_sharp_lit = Vec4::ONE;
            m.params.character.palette_sharp_shadow = Vec4::ONE;
            m.params.metal.controls = Vec4::new(1., 1., 1., f32::from(row == 1));
            m.params.metal.light = Vec4::new(1., 0., 0., 1.);
            m.params.metal.specular = Vec4::new(4., 0., 1., 2.);
            if row == 2 {
                m.packed_normal_texture = Some(normal.clone());
                m.params.source_conventions.x = 1.;
                m.debug_view = StylizedHeroDebugView::NormalHalfDot;
            }
            if column == 1 {
                m.params = apply_hero_portable_fallback(m.params);
            }
            // Asset event synchronization must not accidentally restore expensive paths.
            m.sync_flags();
            let material = app
                .world_mut()
                .resource_mut::<Assets<StylizedHeroMaterial>>()
                .add(m);
            let mesh = app
                .world_mut()
                .resource_mut::<Assets<Mesh>>()
                .add(quad_mesh());
            app.world_mut().spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                StylizedInstanceData {
                    flags: if column == 2 {
                        INSTANCE_FLAG_DROP_HERO_EXTRAS
                    } else {
                        0
                    },
                    ..default()
                },
                Transform::from_xyz((column as f32 - 1.) * 1.2, 1. - row as f32, 0.)
                    .with_scale(Vec3::splat(0.35)),
            ));
        }
    }
    let image = capture_frame_with_projection(
        &mut app,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 4.8,
            },
            ..OrthographicProjection::default_3d()
        }),
    );
    let at = |x: usize, y: usize| &image.data[(y * SIZE as usize + x) * 4..][..3];
    assert!(
        at(64, 75)[0] > 240 && at(64, 75)[1] < 5 && at(64, 75)[2] < 5,
        "full source metal control: {:?}",
        at(64, 75)
    );
    assert!(
        at(64, 128)[0] > 240 && at(64, 128)[1] < 5 && at(64, 128)[2] > 240,
        "full source ramp control: {:?}",
        at(64, 128)
    );
    assert!(
        (60..240).contains(&at(64, 181)[0]),
        "full authored normal control: {:?}",
        at(64, 181)
    );
    for x in [128, 192] {
        for y in [75, 128, 181] {
            assert!(
                at(x, y).iter().all(|v| *v >= 250),
                "fallback must retain neutral base/geometric normal, x={x} y={y}: {:?}",
                at(x, y)
            );
        }
    }
}
