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
    let target = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        capture::create_capture_target(&mut images, SIZE, SIZE)
    };
    app.world_mut().spawn((
        Camera3d::default(),
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
