//! Compare complete GPU skinning and hulls with independently baked geometry.
#[path = "support/render_app.rs"]
mod render_app;
use bevy::{
    mesh::{
        VertexAttributeValues,
        morph::{MeshMorphWeights, MorphAttributes},
        skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    },
    pbr::Material,
    prelude::*,
    render::settings::RenderResources,
};
use bevy_stylized::prelude::*;
use bevy_stylized_render::skinning::{ATTRIBUTE_SECONDARY_JOINTS, ATTRIBUTE_SECONDARY_WEIGHTS};
use golden_harness::{CapturedImage, capture};

fn image<M: Material + Default>(
    resources: RenderResources,
    skinned: bool,
    morph: bool,
    local_outline: bool,
    palette: Option<(u8, u32)>,
    scaled_skin: bool,
) -> CapturedImage {
    let mut app = render_app::headless_app(resources);
    let mut mesh: Mesh = Sphere::new(0.6).into();
    let count = mesh.count_vertices();
    let skin_scale = if scaled_skin {
        Vec3::new(2.0, 0.5, 1.5)
    } else {
        Vec3::ONE
    };
    let displacement = Vec3::new(0.6, 0.0, if morph { 0.15 } else { 0.0 });
    if skinned {
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            VertexAttributeValues::Uint16x4(vec![[0, 0, 0, 0]; count]),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_JOINT_WEIGHT,
            vec![[0.25, 0.0, 0.0, 0.0]; count],
        );
        mesh.insert_attribute(
            ATTRIBUTE_SECONDARY_JOINTS,
            VertexAttributeValues::Uint16x4(vec![[1, 0, 0, 0]; count]),
        );
        mesh.insert_attribute(
            ATTRIBUTE_SECONDARY_WEIGHTS,
            vec![[0.75, 0.0, 0.0, 0.0]; count],
        );
        if morph {
            mesh.set_morph_targets(vec![
                MorphAttributes::new(
                    Vec3::Z * 0.15,
                    Vec3::ZERO,
                    Vec3::ZERO
                );
                count
            ]);
        }
    } else if let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for position in positions {
            *position = (Vec3::from(*position) * skin_scale
                + Vec3::new(
                    displacement.x,
                    displacement.y,
                    displacement.z * skin_scale.z,
                ))
            .to_array();
        }
    }
    if !skinned
        && let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL)
    {
        for normal in normals {
            *normal = (Vec3::from(*normal) / skin_scale).normalize().to_array();
        }
    }
    let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
    let material = app
        .world_mut()
        .resource_mut::<Assets<M>>()
        .add(M::default());
    let mut outline_params = if local_outline {
        StylizedOutlineParams::default().with_source_palette(
            0.06,
            [LinearRgba::RED; 5],
            15,
            false,
            1.0,
        )
    } else {
        StylizedOutlineParams::default()
            .with_color(LinearRgba::RED)
            .with_width(4.0)
    };
    let class_texture = palette.map(|(alpha, mask)| {
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
        outline_params = outline_params.with_source_palette(
            0.06,
            [
                LinearRgba::RED,
                LinearRgba::GREEN,
                LinearRgba::BLUE,
                LinearRgba::new(1., 1., 0., 1.),
                LinearRgba::new(1., 0., 1., 1.),
            ],
            mask,
            false,
            1.0,
        );
        outline_params.source_state.w = 1;
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::new_fill(
                Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &[0, 0, 0, alpha],
                TextureFormat::Rgba8Unorm,
                bevy::asset::RenderAssetUsages::default(),
            ))
    });
    let outline = app
        .world_mut()
        .resource_mut::<Assets<StylizedOutlineMaterial>>()
        .add(StylizedOutlineMaterial {
            params: outline_params,
            class_texture,
        });
    let entity = app
        .world_mut()
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            StylizedOutline::new(outline),
            Transform::IDENTITY,
        ))
        .id();
    if skinned {
        let a = app
            .world_mut()
            .spawn(Transform::from_scale(skin_scale))
            .id();
        let b = app
            .world_mut()
            .spawn(Transform::from_xyz(0.8, 0.0, 0.0).with_scale(skin_scale))
            .id();
        let inverse_bindposes = app
            .world_mut()
            .resource_mut::<Assets<SkinnedMeshInverseBindposes>>()
            .add(SkinnedMeshInverseBindposes::from(vec![Mat4::IDENTITY; 2]));
        app.world_mut().entity_mut(entity).insert(SkinnedMesh {
            inverse_bindposes,
            joints: vec![a, b],
        });
        if morph {
            app.world_mut()
                .entity_mut(entity)
                .insert(MeshMorphWeights::Value { weights: vec![1.0] });
        }
    }
    scene_runtime::capture_in_app(
        &mut app,
        &scenario_model::ident::SessionId::new("extended-skin").expect("session id"),
        &scenes::CameraRecipe::with_attachment(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::ZERO,
            scenes::CameraAttachment::Prepass {
                depth: true,
                normal: true,
                motion_vector: true,
            },
        ),
        (256, 256),
        scene_runtime::Settle::Frames(32),
        false,
        640,
    )
    .expect("extended skin capture timed out")
}

#[test]
#[ignore = "requires GPU; run the extended_skin suite"]
fn complete_skin_and_morph_match_baked_surface_and_outline() {
    let resources = capture::probe_render_resources().expect("required GPU unavailable");
    fn compare<M: Material + Default>(resources: &RenderResources) {
        for (morph, local_outline, scaled_skin) in [
            (false, false, false),
            (true, false, false),
            (false, true, false),
            (true, true, false),
            (false, false, true),
            (true, false, true),
        ] {
            let actual = image::<M>(
                resources.clone(),
                true,
                morph,
                local_outline,
                None,
                scaled_skin,
            );
            let reference = image::<M>(
                resources.clone(),
                false,
                morph,
                local_outline,
                None,
                scaled_skin,
            );
            let visible = actual
                .data
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| p[0] > 100 && p[1] < 50 && p[2] < 50)
                .count();
            assert!(
                visible > 50,
                "expected visible red outline; empty images cannot pass"
            );
            let differences = actual
                .data
                .iter()
                .zip(&reference.data)
                .map(|(a, b)| a.abs_diff(*b))
                .collect::<Vec<_>>();
            let mean =
                differences.iter().map(|v| f64::from(*v)).sum::<f64>() / differences.len() as f64;
            let max = *differences.iter().max().unwrap();
            assert!(
                mean <= 0.5 && max <= 2,
                "{} morph={morph}: max={max}, mean={mean}",
                std::any::type_name::<M>()
            );
        }
    }
    compare::<StylizedCelMaterial>(&resources);
    compare::<StylizedCharacterMaterial>(&resources);
    compare::<StylizedHeroMaterial>(&resources);
}

#[test]
fn loader_registers_complete_skin_aliases_with_renderer_identity() {
    let plugin = bevy_stylized::stylized_gltf_plugin();
    assert_eq!(
        plugin.custom_vertex_attributes["NPR_JOINTS_1"].id,
        ATTRIBUTE_SECONDARY_JOINTS.id
    );
    assert_eq!(
        plugin.custom_vertex_attributes["NPR_WEIGHTS_1"].id,
        ATTRIBUTE_SECONDARY_WEIGHTS.id
    );
}

#[test]
#[ignore = "requires GPU; run the extended_skin suite"]
fn authored_outline_selects_all_five_alpha_classes_and_disabled_class_falls_back() {
    let resources = capture::probe_render_resources().expect("required GPU unavailable");
    for (alpha, mask, expected) in [
        (0, 15, [255, 0, 0]),
        (77, 15, [255, 255, 0]),
        (128, 15, [0, 0, 255]),
        (179, 15, [255, 0, 255]),
        (230, 15, [0, 255, 0]),
        (230, 0, [255, 0, 0]),
    ] {
        let frame = image::<StylizedCelMaterial>(
            resources.clone(),
            false,
            false,
            true,
            Some((alpha, mask)),
            false,
        );
        let count = frame
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| (0..3).all(|i| p[i].abs_diff(expected[i]) <= 2))
            .count();
        assert!(
            count > 50,
            "missing outline class alpha={alpha} mask={mask}: {count} pixels"
        );
    }
}
