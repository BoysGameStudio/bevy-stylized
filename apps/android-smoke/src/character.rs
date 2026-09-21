//! Explicit adoption of a compiled character; raw glTF PBR is not a substitute.
use std::collections::BTreeSet;

use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    gltf::GltfAssetLabel,
    prelude::*,
    render::renderer::RenderAdapterInfo,
    world_serialization::{WorldAsset, WorldAssetRoot},
};
use bevy_stylized::{StylizedMaterialAdoption, StylizedMaterialSet, prelude::*};

#[derive(Resource, Default)]
pub struct CharacterState {
    scene: Handle<WorldAsset>,
    material_names: BTreeSet<String>,
    reported: bool,
}

pub fn setup(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut state: ResMut<CharacterState>,
    mut view: ResMut<StylizedViewSettings>,
    adapter: Res<RenderAdapterInfo>,
    caps: Res<StylizedResolvedCapabilities>,
) {
    let sidecar = bevy_stylized::stylized_gltf::parse_sidecar_str(include_str!(concat!(
        env!("STYLIZED_ANDROID_CHARACTER_DIR"),
        "/character.npr.json"
    )))
    .expect("compiled character NPR sidecar");
    view.0 = serde_json::from_str(include_str!(concat!(
        env!("STYLIZED_ANDROID_CHARACTER_DIR"),
        "/character.view.json"
    )))
    .expect("compiled character view");
    state.material_names = sidecar.materials.keys().cloned().collect();
    state.scene = server.load(GltfAssetLabel::Scene(0).from_asset("character.glb"));
    commands.spawn((
        super::Sample,
        WorldAssetRoot(state.scene.clone()),
        Visibility::default(),
        Transform::default(),
        StylizedMaterialSet {
            materials: sidecar.materials,
            texture_prefix: String::new(),
        },
    ));
    commands.spawn((
        Camera3d::default(),
        Tonemapping::None,
        Msaa::Off,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: bevy::camera::ScalingMode::AutoMin {
                min_width: 1.2,
                min_height: 2.1,
            },
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(0.0, 0.8, 5.0).looking_at(Vec3::new(0.0, 0.8, 0.0), Vec3::Y),
    ));
    info!(
        "ANDROID_CHARACTER loading material_names={:?} adapter={:?} capabilities={:?}",
        state.material_names, adapter.0, caps
    );
}

pub fn check_loaded(
    server: Res<AssetServer>,
    mut state: ResMut<CharacterState>,
    adopted: Query<&StylizedMaterialAdoption>,
    outlines: Query<(), With<StylizedOutlineReplica>>,
    pending: Query<(), With<MeshMaterial3d<StandardMaterial>>>,
    materials: Res<Assets<StylizedHeroMaterial>>,
) {
    if state.reported {
        return;
    }
    for item in &adopted {
        if let Some(error) = &item.error {
            error!(
                "ANDROID_CHARACTER adoption_failed name={} error={error}",
                item.material_name
            );
            state.reported = true;
            return;
        }
    }
    let names: BTreeSet<_> = adopted
        .iter()
        .map(|item| item.material_name.clone())
        .collect();
    let images: Vec<_> = materials
        .iter()
        .flat_map(|(_, material)| {
            [
                material.base_color_texture.clone(),
                material.ilm_texture.clone(),
                material.face_sdf_texture.clone(),
                material.ramp_texture.clone(),
                material.packed_normal_texture.clone(),
                material.matcap_texture.clone(),
                material.spec_ramp_texture.clone(),
                material.surface_control_texture.clone(),
            ]
            .into_iter()
            .flatten()
        })
        .collect();
    if !state.material_names.is_empty()
        && names == state.material_names
        && pending.is_empty()
        && server.is_loaded_with_dependencies(state.scene.id())
        && images
            .iter()
            .all(|image| server.is_loaded_with_dependencies(image.id()))
    {
        info!(
            "ANDROID_CHARACTER assets_adopted primitives={} material_names={} images={} hull_entities={} (visual verification still required)",
            adopted.iter().count(),
            names.len(),
            images.len(),
            outlines.iter().count()
        );
        state.reported = true;
    }
}
