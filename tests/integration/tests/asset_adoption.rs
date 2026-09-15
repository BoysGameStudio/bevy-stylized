//! Required GPU material-adoption and scene-lifecycle checks on an explicit asset.
use bevy::{
    asset::{AssetMetaCheck, AssetPlugin},
    camera::primitives::Aabb,
    core_pipeline::tonemapping::Tonemapping,
    gltf::GltfAssetLabel,
    prelude::*,
    render::{RenderPlugin, settings::RenderCreation},
    window::ExitCondition,
    world_serialization::WorldAssetRoot,
};
use bevy_stylized::prelude::{
    StylizedCelMaterial, StylizedCharacterMaterial, StylizedHeroMaterial, StylizedViewDescriptor,
    StylizedViewSettings,
};
use bevy_stylized::{
    StylizedAssetIntegrationPlugin, StylizedMaterialAdoption, StylizedMaterialSet, StylizedPlugin,
};
use golden_harness::{CapturedImage, capture};
use std::{collections::HashSet, path::PathBuf, time::Duration};

fn expected_primitives(path: &std::path::Path) -> usize {
    let bytes = std::fs::read(path).unwrap();
    let n = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let json: serde_json::Value = serde_json::from_slice(&bytes[20..20 + n]).unwrap();
    json["meshes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["primitives"].as_array().unwrap().len())
        .sum()
}
fn wait_adoption(app: &mut App, expected: usize) {
    for _ in 0..600 {
        app.update();
        let mut query = app.world_mut().query::<&StylizedMaterialAdoption>();
        let values: Vec<_> = query.iter(app.world()).collect();
        for value in &values {
            assert!(value.error.is_none(), "{:?}", value.error);
        }
        if values.len() == expected {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("asset adoption timed out or coverage incomplete");
}
fn frame(app: &mut App, target: &capture::CaptureTarget) -> CapturedImage {
    for _ in 0..30 {
        app.update();
        std::thread::sleep(Duration::from_millis(2));
    }
    let request = capture::request_capture(app.world_mut(), target.clone());
    for _ in 0..180 {
        app.update();
        if let Some(frame) = capture::try_take(app.world_mut(), request.clone()) {
            return frame;
        }
    }
    panic!("GPU capture timed out");
}

fn check_asset(asset: PathBuf, textured: bool) {
    let asset = asset.canonicalize().unwrap();
    let sidecar = bevy_stylized_gltf::load_sidecar(asset.with_extension("npr.json"))
        .expect("typed sidecar required");
    let count = expected_primitives(&asset);
    assert!(count > 0 && !sidecar.materials.is_empty());
    let resources = capture::probe_render_resources().expect("required GPU unavailable");
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .build()
            .set(bevy_stylized::stylized_gltf_plugin())
            .set(AssetPlugin {
                file_path: asset.parent().unwrap().to_string_lossy().into_owned(),
                meta_check: AssetMetaCheck::Never,
                ..default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Manual(resources),
                synchronous_pipeline_compilation: true,
                ..default()
            })
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            }),
    );
    app.add_plugins((StylizedPlugin::default(), StylizedAssetIntegrationPlugin));
    capture::install(&mut app);
    let target = capture::create_capture_target(
        &mut app.world_mut().resource_mut::<Assets<Image>>(),
        512,
        512,
    );
    app.insert_resource(ClearColor(Color::srgb(0.12, 0.12, 0.12)));
    *app.world_mut().resource_mut::<StylizedViewSettings>() =
        StylizedViewSettings(StylizedViewDescriptor {
            key_light_direction: Vec3::new(0.3, 0.6, 1.).normalize(),
            key_light_color: bevy::color::LinearRgba::WHITE,
            ..default()
        });
    let authored_view = asset.with_extension("view.json");
    if authored_view.exists() {
        let descriptor: StylizedViewDescriptor =
            serde_json::from_slice(&std::fs::read(authored_view).unwrap()).unwrap();
        *app.world_mut().resource_mut::<StylizedViewSettings>() = StylizedViewSettings(descriptor);
    }
    app.finish();
    app.cleanup();
    let scene = app.world().resource::<AssetServer>().load(
        GltfAssetLabel::Scene(0)
            .from_asset(asset.file_name().unwrap().to_string_lossy().into_owned()),
    );
    let materials = StylizedMaterialSet {
        materials: sidecar.materials,
        texture_prefix: String::new(),
    };
    let root = app
        .world_mut()
        .spawn((
            WorldAssetRoot(scene.clone()),
            materials.clone(),
            Transform::default(),
        ))
        .id();
    let second = app
        .world_mut()
        .spawn((
            WorldAssetRoot(scene.clone()),
            materials.clone(),
            Transform::default(),
        ))
        .id();
    app.world_mut().spawn((
        Camera3d::default(),
        capture::capture_target_bundle(&target),
        Tonemapping::None,
        Transform::default(),
    ));
    wait_adoption(&mut app, count * 2);
    if !textured {
        let mut skins = app
            .world_mut()
            .query::<&bevy::mesh::skinning::SkinnedMesh>();
        assert_eq!(
            skins.iter(app.world()).count(),
            count * 2,
            "imported skin components missing"
        );
        let mut morphs = app
            .world_mut()
            .query::<&bevy::mesh::morph::MeshMorphWeights>();
        assert!(
            morphs.iter(app.world()).count() >= 2,
            "imported face morph components missing"
        );
    }
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    let mut bounds = app.world_mut().query::<(&Aabb, &GlobalTransform)>();
    for (aabb, transform) in bounds.iter(app.world()) {
        for mask in 0..8 {
            let h = aabb.half_extents;
            let off = Vec3::new(
                if mask & 1 == 0 { -h.x } else { h.x },
                if mask & 2 == 0 { -h.y } else { h.y },
                if mask & 4 == 0 { -h.z } else { h.z },
            );
            let point = transform.transform_point(Vec3::from(aabb.center) + off);
            min = min.min(point);
            max = max.max(point);
        }
    }
    assert!(min.is_finite() && max.is_finite() && max.y > min.y);
    let center = (min + max) * 0.5;
    let radius = (max - min).length() * 0.5;
    let mut camera = app
        .world_mut()
        .query_filtered::<&mut Transform, With<Camera3d>>();
    for mut transform in camera.iter_mut(app.world_mut()) {
        *transform =
            Transform::from_translation(center + Vec3::Z * radius * 3.).looking_at(center, Vec3::Y);
    }
    let mut handles = app.world_mut().query::<(
        Option<&MeshMaterial3d<StylizedCelMaterial>>,
        Option<&MeshMaterial3d<StylizedCharacterMaterial>>,
        Option<&MeshMaterial3d<StylizedHeroMaterial>>,
    )>();
    let ids: HashSet<_> = handles
        .iter(app.world())
        .filter_map(|(c, ch, h)| {
            c.map(|x| x.0.id().untyped())
                .or_else(|| ch.map(|x| x.0.id().untyped()))
                .or_else(|| h.map(|x| x.0.id().untyped()))
        })
        .collect();
    assert_eq!(
        ids.len(),
        materials.materials.len(),
        "instances must share descriptor-equivalent materials"
    );
    // Inspect a single instance; overlapping duplicates would create z-fighting.
    app.world_mut().despawn(second);
    wait_adoption(&mut app, count);
    let captured = frame(&mut app, &target);
    let colors: HashSet<_> = captured
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| [p[0], p[1], p[2]])
        .collect();
    if textured {
        assert!(
            colors.len() > 64,
            "textured character capture has no expected color variation"
        );
    } else {
        // Flat first-party panels intentionally contain only a few solid colors.
        // Require both independently authored cool and warm surfaces, not texture noise.
        let cool = captured
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| u16::from(p[2]) > u16::from(p[0]) * 12 / 10 && p[2] > p[1])
            .count();
        let warm = captured
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] > p[1] && p[1] > p[2])
            .count();
        assert!(
            cool > 100 && warm > 100,
            "authored color surfaces missing: cool={cool} warm={warm}"
        );
    }
    if let Ok(out) = std::env::var("STYLIZED_CAPTURE_OUT") {
        let path = PathBuf::from(out);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        use sha2::{Digest, Sha256};
        let png = golden_harness::encode_png(&captured).unwrap();
        let info = app
            .world()
            .get_resource::<bevy::render::renderer::RenderAdapterInfo>();
        let gpu = info.map_or_else(
            || "unknown".into(),
            |i| format!("{} ({:?})", i.name, i.backend),
        );
        let driver = info.map_or_else(|| "unknown".into(), |i| i.driver_info.clone());
        let mut provenance = golden_harness::GoldenProvenance::for_capture(
            "explicit-runtime-asset",
            None,
            "not-recorded-at-runtime",
            None,
            "bevy_stylized.runtime-export/v1",
            "bevy_stylized.material/v1",
            "asset-integration",
            "bevy-stylized",
            &gpu,
            &driver,
            &captured,
        );
        provenance.png_fnv1a64 = golden_harness::fnv1a64_hex(&png);
        let mut record = serde_json::to_value(provenance).unwrap();
        record["image_sha256"] = format!("{:x}", Sha256::digest(&png)).into();
        record["input_asset_sha256"] =
            format!("{:x}", Sha256::digest(std::fs::read(&asset).unwrap())).into();
        record["asset_ownership"] = if textured { "external" } else { "first-party" }.into();
        record["view_settings"] =
            serde_json::to_value(&app.world().resource::<StylizedViewSettings>().0).unwrap();
        record["camera"] = serde_json::json!({"look_at":center.to_array(),"position":(center+Vec3::Z*radius*3.0).to_array(),"projection":"perspective default"});
        std::fs::write(&path, png).unwrap();
        std::fs::write(
            path.with_extension("golden.json"),
            serde_json::to_vec_pretty(&record).unwrap(),
        )
        .unwrap();
    }
    app.world_mut().despawn(root);
    wait_adoption(&mut app, 0);
    app.world_mut()
        .spawn((WorldAssetRoot(scene), materials, Transform::default()));
    wait_adoption(&mut app, count);
    eprintln!(
        "ASSET_INTEGRATION_PASS primitives={count} shared_materials={} reload=passed",
        ids.len()
    );
}

#[test]
#[ignore = "requires a GPU and an explicitly prepared STYLIZED_TEST_ASSET"]
fn declared_asset_adopts_shares_and_reloads() {
    check_asset(
        PathBuf::from(std::env::var("STYLIZED_TEST_ASSET").expect("explicit asset required")),
        true,
    );
}
#[test]
#[ignore = "requires GPU; first-party input is part of this repository"]
fn synthetic_character_imports_skin_morph_and_materials() {
    check_asset(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/synthetic/character/character.glb"),
        false,
    );
}
