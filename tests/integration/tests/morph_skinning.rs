//! Morph / skinning 渲染语义测试（GPU-gated；release gate "Skinning/morph
//! semantics tested" 行）。
//!
//! - morph：合成 quad + 一个把整体平移的 morph target；weight 0 vs 1 的
//!   捕获必须在物体区域有实质差异（且都非黑）；
//! - skinning：双关节蒙皮条带，关节旋转后形变可见。

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    color::LinearRgba,
    image::Image,
    math::{Mat4, Quat, Vec3},
    mesh::{Mesh, skinning::SkinnedMesh},
    pbr::MeshMaterial3d,
    prelude::{Camera3d, Mesh3d, Transform},
    render::{
        RenderPlugin,
        settings::{RenderCreation, RenderResources},
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::*;
use golden_harness::{CapturedImage, capture};

const SIZE: u32 = 256;

fn headless_app(resources: RenderResources) -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: "assets".to_string(),
                meta_check: bevy::asset::AssetMetaCheck::Never,
                ..Default::default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Manual(resources),
                synchronous_pipeline_compilation: true,
                ..Default::default()
            })
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..Default::default()
            }),
    );
    app.add_plugins(StylizedPlugin::default());
    capture::install(&mut app);
    *app.world_mut().resource_mut::<StylizedViewSettings>() =
        StylizedViewSettings(StylizedViewDescriptor {
            key_light_direction: Vec3::new(0.3, 0.8, 0.6).normalize(),
            key_light_color: LinearRgba::WHITE,
            exposure: 1.0,
            day_night_factor: 1.0,
            ..StylizedViewDescriptor::default()
        });
    app.finish();
    app.cleanup();
    app
}

fn capture_frame(app: &mut App) -> CapturedImage {
    capture_frame_with_projection(app, bevy::camera::Projection::default())
}

fn capture_frame_with_projection(
    app: &mut App,
    projection: bevy::camera::Projection,
) -> CapturedImage {
    let old_cameras: Vec<_> = app
        .world_mut()
        .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<Camera3d>>()
        .iter(app.world())
        .collect();
    for camera in old_cameras {
        app.world_mut().despawn(camera);
    }
    let target = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        capture::create_capture_target(&mut images, SIZE, SIZE)
    };
    app.world_mut().spawn((
        Camera3d::default(),
        projection,
        capture::capture_target_bundle(&target),
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    for _ in 0..24 {
        app.update();
    }
    let request = capture::request_capture(app.world_mut(), target);
    for _ in 0..600 {
        app.update();
        if let Some(frame) = capture::try_take(app.world_mut(), request.clone()) {
            return frame;
        }
    }
    panic!("capture timed out");
}

fn quad_mesh() -> Mesh {
    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-0.8, -0.8, 0.0],
            [0.8, -0.8, 0.0],
            [0.8, 0.8, 0.0],
            [-0.8, 0.8, 0.0],
        ],
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4]);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );
    mesh.insert_indices(bevy::mesh::Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    mesh
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn morph_target_deformation_renders() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("morph test: no GPU adapter; skipping");
        return;
    };
    // morph target：整体向左平移 1.0（per-vertex delta）。
    let mut mesh = quad_mesh();
    let deltas: Vec<bevy::mesh::morph::MorphAttributes> = (0..4)
        .map(|_| {
            bevy::mesh::morph::MorphAttributes::new(
                Vec3::new(-1.0, 0.0, 0.0),
                Vec3::ZERO,
                Vec3::ZERO,
            )
        })
        .collect();
    mesh.set_morph_targets(deltas);

    let materialize = |app: &mut App, weight: f32| {
        let mesh_handle = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(mesh.clone());
        let material = app
            .world_mut()
            .resource_mut::<Assets<StylizedCelMaterial>>()
            .add(StylizedCelMaterial::default());
        app.world_mut().spawn((
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(material),
            bevy::mesh::morph::MeshMorphWeights::Value {
                weights: vec![weight],
            },
            Transform::IDENTITY,
        ));
    };

    let mut app = headless_app(resources.clone());
    materialize(&mut app, 0.0);
    let rest = capture_frame(&mut app);

    let mut app = headless_app(resources);
    materialize(&mut app, 1.0);
    let morphed = capture_frame(&mut app);

    let stats = golden_harness::diff_rgba8(&rest.data, &morphed.data).expect("same size");
    assert!(
        stats.differing_pixels > 500,
        "morph weight 0 vs 1 must visibly differ (differing={})",
        stats.differing_pixels
    );
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn skinned_deformation_renders() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("skinning test: no GPU adapter; skipping");
        return;
    };

    let build = |app: &mut App, bent: bool| {
        // 竖直条带，两段关节。
        let (cols, rows) = (2usize, 16usize);
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut joints: Vec<[u16; 4]> = Vec::new();
        let mut weights = Vec::new();
        for r in 0..=rows {
            let v = r as f32 / rows as f32;
            for c in 0..=cols {
                let u = c as f32 / cols as f32;
                positions.push([(u - 0.5) * 0.8, v * 2.4 - 1.2, 0.0]);
                normals.push([0.0, 0.0, 1.0]);
                uvs.push([u, v]);
                joints.push([u16::from(v > 0.5), 0, 0, 0]);
                weights.push([1.0, 0.0, 0.0, 0.0]);
            }
        }
        let mut indices = Vec::new();
        for r in 0..rows {
            for c in 0..cols {
                let i = (r * (cols + 1) + c) as u32;
                let w = cols as u32 + 1;
                indices.extend_from_slice(&[i, i + 1, i + w, i + 1, i + w + 1, i + w]);
            }
        }
        let mut mesh = Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            bevy::mesh::VertexAttributeValues::Uint16x4(joints),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_JOINT_WEIGHT,
            bevy::mesh::VertexAttributeValues::Float32x4(weights),
        );
        mesh.insert_indices(bevy::mesh::Indices::U32(indices));
        let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);

        let root = app
            .world_mut()
            .spawn(Transform::from_xyz(0.0, -1.2, 0.0))
            .id();
        let tip = app.world_mut().spawn(Transform::default()).id();
        app.world_mut().entity_mut(root).add_child(tip);
        if bent {
            app.world_mut()
                .entity_mut(root)
                .get_mut::<Transform>()
                .expect("transform")
                .rotation = Quat::from_rotation_z(0.6);
        }
        let inverse_bindposes = app
            .world_mut()
            .resource_mut::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
            .add(bevy::mesh::skinning::SkinnedMeshInverseBindposes::from(
                vec![
                    Mat4::from_translation(Vec3::new(0.0, 1.2, 0.0)),
                    Mat4::IDENTITY,
                ],
            ));
        let material = app
            .world_mut()
            .resource_mut::<Assets<StylizedCelMaterial>>()
            .add(StylizedCelMaterial::default());
        app.world_mut().spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            SkinnedMesh {
                inverse_bindposes,
                joints: vec![root, tip],
            },
            Transform::IDENTITY,
        ));
    };

    let mut app = headless_app(resources.clone());
    build(&mut app, false);
    let straight = capture_frame(&mut app);

    let mut app = headless_app(resources);
    build(&mut app, true);
    let bent = capture_frame(&mut app);

    let stats = golden_harness::diff_rgba8(&straight.data, &bent.data).expect("same size");
    assert!(
        stats.differing_pixels > 200,
        "skinning deformation must be visible (differing={})",
        stats.differing_pixels
    );
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn live_morph_outline_tracks_surface_and_returns_to_rest() {
    use bevy::mesh::morph::{MeshMorphWeights, MorphAttributes};
    let resources = capture::probe_render_resources().expect("required GPU unavailable");
    let mut app = headless_app(resources);
    let mut mesh: Mesh = bevy::prelude::Sphere::new(0.8).into();
    mesh.set_morph_targets(vec![
        MorphAttributes::new(
            Vec3::X * 0.8,
            Vec3::ZERO,
            Vec3::ZERO
        );
        mesh.count_vertices()
    ]);
    let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial::default());
    let outline = app
        .world_mut()
        .resource_mut::<Assets<StylizedOutlineMaterial>>()
        .add(StylizedOutlineMaterial {
            params: StylizedOutlineParams::default()
                .with_color(LinearRgba::RED)
                .with_width(4.0),
        });
    let source = app
        .world_mut()
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            StylizedOutline::new(outline),
            MeshMorphWeights::Value { weights: vec![0.0] },
            Transform::IDENTITY,
        ))
        .id();
    let centroid = |image: &CapturedImage| {
        let mut sum = 0usize;
        let mut count = 0usize;
        for (i, p) in image.data.as_chunks::<4>().0.iter().enumerate() {
            if p[0] > 100 && p[1] < 40 && p[2] < 40 {
                sum += i % SIZE as usize;
                count += 1;
            }
        }
        assert!(
            count > 100,
            "expected a visible red hull outline, got {count} pixels"
        );
        sum as f32 / count as f32
    };
    let rest = capture_frame(&mut app);
    app.world_mut()
        .entity_mut(source)
        .insert(MeshMorphWeights::Value { weights: vec![1.0] });
    let moved = capture_frame(&mut app);
    let shift = centroid(&moved) - centroid(&rest);
    assert!(
        shift > 20.0,
        "outline must move with live morph input: shift={shift}"
    );
    app.world_mut()
        .entity_mut(source)
        .insert(MeshMorphWeights::Value { weights: vec![0.0] });
    let returned = capture_frame(&mut app);
    assert!(
        (centroid(&returned) - centroid(&rest)).abs() < 1.0,
        "outline did not return to rest"
    );
    let stats = golden_harness::diff_rgba8(&rest.data, &returned.data).unwrap();
    assert!(
        stats.differing_pixels < 20,
        "restoring input must restore the rendered surface and outline"
    );
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
    for (x, day) in [(-0.9, 1.0), (0.9, 0.0)] {
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(quad_mesh());
        let mut material = StylizedHeroMaterial::default();
        material.params = material.params.with_genshin_ramp(0.55, 1.0);
        material.params.character = material.params.character.with_day_night(day);
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

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn external_head_pose_drives_face_shading_without_material_clones() {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let resources = capture::probe_render_resources().expect("required GPU unavailable");
    let mut app = headless_app(resources);
    app.world_mut()
        .resource_mut::<StylizedViewSettings>()
        .0
        .key_light_direction = Vec3::Z;
    let mut tex = |value: &[u8]| {
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::new_fill(
                Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                value,
                TextureFormat::Rgba8Unorm,
                bevy::asset::RenderAssetUsages::default(),
            ))
    };
    let sdf = tex(&[128, 128, 128, 255]);
    let control = tex(&[0, 255, 0, 0]);
    let mut material = StylizedHeroMaterial {
        debug_view: StylizedHeroDebugView::LightingFactors,
        face_sdf_texture: Some(sdf),
        surface_control_texture: Some(control),
        surface_control: StylizedHeroSurfaceControl::GenshinFace,
        ..Default::default()
    };
    material.params.character = material
        .params
        .character
        .with_face_basis(Vec3::Z, Vec3::Y, 0.01);
    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedHeroMaterial>>()
        .add(material);
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(quad_mesh());
    let head = app.world_mut().spawn(Transform::default()).id();
    let face = app
        .world_mut()
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::default(),
            StylizedFaceInput {
                head,
                forward: Vec3::Z,
                up: Vec3::Y,
            },
        ))
        .id();
    let front = capture_frame(&mut app);
    app.world_mut().get_mut::<Transform>(head).unwrap().rotation =
        Quat::from_rotation_y(std::f32::consts::PI);
    let back = capture_frame(&mut app);
    let at = ((SIZE / 2 * SIZE + SIZE / 2) * 4) as usize;
    assert!(
        front.data[at] > 250 && back.data[at] < 5,
        "external face thresholds must reach the shader"
    );
    assert!(
        app.world()
            .get::<StylizedFaceInputStatus>(face)
            .unwrap()
            .error
            .is_none()
    );
    assert_eq!(
        app.world().resource::<Assets<StylizedHeroMaterial>>().len(),
        1
    );
    app.world_mut().get_mut::<Transform>(head).unwrap().rotation = Quat::IDENTITY;
    assert_eq!(capture_frame(&mut app).data, front.data);
    app.world_mut().get_mut::<Transform>(head).unwrap().rotation = Quat::from_rotation_y(0.65);
    let _ = capture_frame(&mut app);
    let left = *app.world().get::<StylizedInstanceData>(face).unwrap();
    app.world_mut().get_mut::<Transform>(head).unwrap().rotation = Quat::from_rotation_y(-0.65);
    let _ = capture_frame(&mut app);
    let right = *app.world().get::<StylizedInstanceData>(face).unwrap();
    assert!((left.face_threshold.unwrap() - right.face_threshold.unwrap()).abs() < 1e-6);
    assert_ne!(
        left.face_side_flip, right.face_side_flip,
        "symmetric head yaw must select opposite SDF halves"
    );
}
