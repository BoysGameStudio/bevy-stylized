//! Shared host material transport, live buffer writes and alpha routing.
#[path = "support/render_app_builder.rs"]
mod builder;
use bevy::prelude::*;
use bevy_stylized::prelude::*;
use bevy_stylized_render::{host_inputs::*, host_material::*};
use golden_harness::{CapturedImage, capture};

fn capture_image(app: &mut App) -> CapturedImage {
    scene_runtime::capture_in_app(
        app,
        &scenario_model::ident::SessionId::new("host-material").expect("session id"),
        &scenes::CameraRecipe::new(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO),
        (256, 256),
        scene_runtime::Settle::Frames(32),
        false,
        640,
    )
    .expect("host material capture timed out")
}

fn verify_material<M: HostMaterialSource + Default>(
    resources: bevy::render::settings::RenderResources,
) where
    M::Data: Clone + Eq + std::hash::Hash,
{
    let mut app = builder::configured_app(resources, |app| {
        app.add_plugins(HostMaterialPlugin).add_systems(
            PostUpdate,
            publish_host_inputs.after(bevy::transform::TransformSystems::Propagate),
        );
    });
    let material = app
        .world_mut()
        .resource_mut::<Assets<HostMaterial<M>>>()
        .add(HostMaterial::new(M::default(), HostAlphaMode::Opaque));
    let base = app
        .world()
        .resource::<Assets<HostMaterial<M>>>()
        .get(&material)
        .unwrap()
        .base()
        .frame_uniform(&HostFrameInputs::default())
        .unwrap();
    let view = HostViewRecord {
        ambient: Vec4::splat(0.3),
        ..default()
    };
    let a = app.world_mut().spawn(HostViewInput(view)).id();
    let b = app.world_mut().spawn(HostViewInput(view)).id();
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Sphere::new(0.6));
    let left = app
        .world_mut()
        .spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(-0.8, 0.0, 0.0),
            HostInstanceInput {
                view: a,
                record: HostInstanceRecord {
                    tint: Vec4::new(1.0, 0.02, 0.02, 1.0),
                    ..default()
                },
            },
        ))
        .id();
    app.world_mut().spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material.clone()),
        Transform::from_xyz(0.8, 0.0, 0.0),
        HostInstanceInput {
            view: b,
            record: HostInstanceRecord {
                tint: Vec4::new(0.02, 1.0, 0.02, 1.0),
                ..default()
            },
        },
    ));
    let first = capture_image(&mut app);
    let red = first
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[0] > 100 && p[1] < 60 && p[2] < 60)
        .count();
    let green = first
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[1] > 100 && p[0] < 60 && p[2] < 60)
        .count();
    assert!(
        red > 100 && green > 100,
        "required distinct rendered subjects: red={red}, green={green}"
    );
    app.world_mut()
        .get_mut::<HostInstanceInput>(left)
        .unwrap()
        .record
        .tint = Vec4::new(0.02, 0.02, 1.0, 1.0);
    let second = capture_image(&mut app);
    let mut left_diff = 0;
    let mut right_diff = 0;
    for (i, (x, y)) in first
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .zip(second.data.as_chunks::<4>().0.iter())
        .enumerate()
    {
        if x != y {
            if i % 256 < 128 {
                left_diff += 1;
            } else {
                right_diff += 1;
            }
        }
    }
    assert!(left_diff > 100);
    assert_eq!(
        right_diff, 0,
        "one actor update leaked into another view input"
    );
    let materials = app.world().resource::<Assets<HostMaterial<M>>>();
    assert_eq!(materials.len(), 1);
    assert_eq!(
        materials
            .get(&material)
            .unwrap()
            .base()
            .frame_uniform(&HostFrameInputs::default())
            .unwrap(),
        base
    );
    assert!(app.world().resource::<HostFrameStatus>().ready);
    let blend = app
        .world_mut()
        .resource_mut::<Assets<HostMaterial<M>>>()
        .add(HostMaterial::new(M::default(), HostAlphaMode::Blend));
    app.world_mut()
        .entity_mut(left)
        .insert(MeshMaterial3d(blend));
    app.world_mut()
        .get_mut::<HostInstanceInput>(left)
        .unwrap()
        .record
        .tint
        .w = 0.0;
    let transparent = capture_image(&mut app);
    let blue = transparent
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[2] > 100 && p[0] < 60 && p[1] < 60)
        .count();
    assert_eq!(blue, 0, "zero-alpha subject remained visible");
    app.world_mut()
        .entity_mut(left)
        .insert(MeshMaterial3d(material));
    app.world_mut()
        .get_mut::<HostInstanceInput>(left)
        .unwrap()
        .record
        .tint
        .w = 1.0;
    let restored = capture_image(&mut app);
    assert_eq!(
        restored.data, second.data,
        "Opaque restoration changed the accepted image"
    );
}

#[test]
#[ignore = "requires GPU; run the host_material suite"]
fn shared_host_material_updates_instance_data_without_asset_mutation() {
    let resources = capture::probe_render_resources().expect("required GPU unavailable");
    verify_material::<StylizedCelMaterial>(resources.clone());
    verify_material::<StylizedCharacterMaterial>(resources.clone());
    verify_material::<StylizedHeroMaterial>(resources);
}

#[test]
#[ignore = "requires GPU; run the host_material suite"]
fn hero_blend_uses_diffuse_alpha_while_opaque_keeps_it_as_control_data() {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let resources = capture::probe_render_resources().expect("required GPU unavailable");
    let mut app = builder::configured_app(resources, |app| {
        app.add_plugins(HostMaterialPlugin)
            .add_systems(PostUpdate, publish_host_inputs);
    });
    let texture = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_fill(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[255, 255, 255, 0],
            TextureFormat::Rgba8Unorm,
            bevy::asset::RenderAssetUsages::default(),
        ));
    let material = StylizedHeroMaterial {
        base_color_texture: Some(texture),
        ..default()
    };
    let opaque = app
        .world_mut()
        .resource_mut::<Assets<HostMaterial<StylizedHeroMaterial>>>()
        .add(HostMaterial::new(material.clone(), HostAlphaMode::Opaque));
    let blend = app
        .world_mut()
        .resource_mut::<Assets<HostMaterial<StylizedHeroMaterial>>>()
        .add(HostMaterial::new(material, HostAlphaMode::Blend));
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Sphere::new(0.6));
    let view = app
        .world_mut()
        .spawn(HostViewInput(HostViewRecord {
            ambient: Vec4::ONE,
            ..default()
        }))
        .id();
    let entity = app
        .world_mut()
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(opaque),
            Transform::IDENTITY,
            HostInstanceInput {
                view,
                record: HostInstanceRecord {
                    tint: Vec4::new(1., 0., 0., 1.),
                    ..default()
                },
            },
        ))
        .id();
    let visible = capture_image(&mut app);
    assert!(
        visible
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] > 100 && p[1] < 20)
            .count()
            > 100
    );
    app.world_mut()
        .entity_mut(entity)
        .insert(MeshMaterial3d(blend));
    let transparent = capture_image(&mut app);
    app.world_mut()
        .entity_mut(entity)
        .insert(Visibility::Hidden);
    let empty = capture_image(&mut app);
    assert_eq!(
        transparent.data, empty.data,
        "diffuse alpha must reach the ordinary Blend pipeline"
    );
}

#[test]
#[ignore = "requires GPU; run the host_material suite"]
fn opaque_priorities_are_independent_of_material_allocation_order() {
    let resources = capture::probe_render_resources().expect("required GPU unavailable");
    let mut app = builder::configured_app(resources, |app| {
        app.add_plugins(HostMaterialPlugin)
            .add_systems(PostUpdate, publish_host_inputs);
    });
    let view = app
        .world_mut()
        .spawn(HostViewInput(HostViewRecord {
            ambient: Vec4::ONE,
            ..default()
        }))
        .id();
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Rectangle::new(1.2, 1.2));
    let material = |priority| {
        HostMaterial::new(StylizedCelMaterial::default(), HostAlphaMode::Opaque)
            .with_opaque_priority(priority)
    };
    let red = app
        .world_mut()
        .resource_mut::<Assets<HostMaterial<StylizedCelMaterial>>>()
        .add(material(10));
    let green = app
        .world_mut()
        .resource_mut::<Assets<HostMaterial<StylizedCelMaterial>>>()
        .add(material(20));
    let a = app
        .world_mut()
        .spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(red),
            Transform::IDENTITY,
            HostInstanceInput {
                view,
                record: HostInstanceRecord {
                    tint: Vec4::new(1., 0., 0., 1.),
                    ..default()
                },
            },
        ))
        .id();
    let b = app
        .world_mut()
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(green),
            Transform::IDENTITY,
            HostInstanceInput {
                view,
                record: HostInstanceRecord {
                    tint: Vec4::new(0., 1., 0., 1.),
                    ..default()
                },
            },
        ))
        .id();
    let first = capture_image(&mut app);
    let green = app
        .world_mut()
        .resource_mut::<Assets<HostMaterial<StylizedCelMaterial>>>()
        .add(material(20));
    let red = app
        .world_mut()
        .resource_mut::<Assets<HostMaterial<StylizedCelMaterial>>>()
        .add(material(10));
    app.world_mut().entity_mut(a).insert(MeshMaterial3d(red));
    app.world_mut().entity_mut(b).insert(MeshMaterial3d(green));
    let reordered = capture_image(&mut app);
    assert_eq!(
        first.data, reordered.data,
        "GPU allocation order must not choose an opaque layer winner"
    );
    let reverse = app
        .world_mut()
        .resource_mut::<Assets<HostMaterial<StylizedCelMaterial>>>()
        .add(material(30));
    app.world_mut()
        .entity_mut(a)
        .insert(MeshMaterial3d(reverse));
    let reversed = capture_image(&mut app);
    let center = |frame: &CapturedImage| frame.data[(128 * 256 + 128) * 4..][..3].to_vec();
    assert_ne!(
        center(&first),
        center(&reversed),
        "the priority control must actually select a different layer"
    );
}
