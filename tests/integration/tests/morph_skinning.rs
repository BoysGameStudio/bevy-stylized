//! External deformation, outline tracking and head-pose input.
#[path = "support/render_scene.rs"]
mod render_scene;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::prelude::*;
use bevy_stylized::prelude::*;
use golden_harness::{CapturedImage, capture};
use render_scene::*;

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
            ..Default::default()
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
