//! Authored Genshin lighting and control-map semantics.
#[path = "support/render_scene.rs"]
mod render_scene;
use bevy::prelude::*;
use bevy_stylized::prelude::*;
use golden_harness::capture;
use render_scene::*;

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
            Transform::from_xyz(x, 0., 0.),
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
        let mut m = StylizedHeroMaterial::default();
        m.ilm_texture = Some(ilm.clone());
        m.matcap_texture = Some(matcap.clone());
        m.spec_ramp_texture = Some(spec.clone());
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
