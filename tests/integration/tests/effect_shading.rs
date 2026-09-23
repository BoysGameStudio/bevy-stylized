//! Effect light-basis shading evidence on the local compiled effects build
//! (semantics: docs/reference/03 效果网格光照基 decode 节).
//!
//! Renders the compiled `genshin-static-effects-v1` bundle, then flips the
//! effect materials' decoded light azimuth and asserts the pixel change is
//! confined to the effect meshes' screen region (the body must stay on the
//! view-level key light).
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
use bevy_stylized::prelude::{StylizedHeroMaterial, StylizedViewDescriptor, StylizedViewSettings};
use bevy_stylized::{StylizedAssetIntegrationPlugin, StylizedMaterialSet, StylizedPlugin};
use golden_harness::{CapturedImage, capture};
use std::path::PathBuf;
use std::time::Duration;

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

fn effect_region(
    app: &mut App,
    effect_materials: &[String],
    width: u32,
    height: u32,
) -> (i32, i32, i32, i32) {
    let mut corners = Vec::new();
    let mut found = 0;
    let mut query = app.world_mut().query::<(
        &bevy_stylized::StylizedMaterialAdoption,
        &Aabb,
        &GlobalTransform,
    )>();
    for (adoption, aabb, transform) in query.iter(app.world()) {
        if !effect_materials.contains(&adoption.material_name) {
            continue;
        }
        found += 1;
        for mask in 0..8 {
            let h = aabb.half_extents;
            let off = Vec3::new(
                if mask & 1 == 0 { -h.x } else { h.x },
                if mask & 2 == 0 { -h.y } else { h.y },
                if mask & 4 == 0 { -h.z } else { h.z },
            );
            corners.push(transform.transform_point(Vec3::from(aabb.center) + off));
        }
    }
    assert_eq!(found, 2, "both effect meshes must be present with bounds");
    let (camera, camera_transform) = app
        .world_mut()
        .query::<(&Camera, &GlobalTransform)>()
        .single(app.world())
        .expect("single camera");
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for point in &corners {
        let ndc = camera
            .world_to_ndc(camera_transform, *point)
            .expect("effect corner in front of camera");
        let px = ((ndc.x * 0.5 + 0.5) * width as f32) as i32;
        let py = ((0.5 - ndc.y * 0.5) * height as f32) as i32;
        min_x = min_x.min(px);
        min_y = min_y.min(py);
        max_x = max_x.max(px);
        max_y = max_y.max(py);
    }
    // Pad for ribbon curvature beyond the AABB projection and filter spread.
    let pad = 12;
    (
        (min_x - pad).max(0),
        (min_y - pad).max(0),
        (max_x + pad).min(width as i32 - 1),
        (max_y + pad).min(height as i32 - 1),
    )
}

#[test]
#[ignore = "requires a GPU and the local effects build: STYLIZED_EFFECTS_ASSET=<build>/compiled/character.glb"]
fn effect_light_basis_drives_scoped_shading() {
    let asset = PathBuf::from(
        std::env::var("STYLIZED_EFFECTS_ASSET").expect("explicit effects build asset required"),
    )
    .canonicalize()
    .unwrap();
    let sidecar = bevy_stylized_gltf::load_sidecar(asset.with_extension("npr.json"))
        .expect("typed sidecar required");
    let effect_materials: Vec<String> = sidecar
        .materials
        .iter()
        .filter(|(_, d)| d.static_params.contains_key("effect_light_dir_0"))
        .map(|(name, _)| name.clone())
        .collect();
    assert_eq!(
        effect_materials.len(),
        2,
        "exactly the two effect materials carry a decoded light basis"
    );
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
            // View light deliberately different from the baked effect basis
            // ([0,0,1]) so the override is observable.
            key_light_direction: Vec3::new(0.3, 0.6, 1.).normalize(),
            key_light_color: bevy::color::LinearRgba::WHITE,
            ..default()
        });
    app.finish();
    app.cleanup();
    let scene = app.world().resource::<AssetServer>().load(
        GltfAssetLabel::Scene(0)
            .from_asset(asset.file_name().unwrap().to_string_lossy().into_owned()),
    );
    let materials = StylizedMaterialSet {
        materials: sidecar.materials.clone(),
        texture_prefix: String::new(),
    };
    app.world_mut()
        .spawn((WorldAssetRoot(scene), materials, Transform::default()));
    app.world_mut().spawn((
        Camera3d::default(),
        capture::capture_target_bundle(&target),
        Tonemapping::None,
        Transform::default(),
    ));
    // Wait for adoption: every primitive adopted without error.
    for _ in 0..600 {
        app.update();
        let mut query = app
            .world_mut()
            .query::<&bevy_stylized::StylizedMaterialAdoption>();
        let values: Vec<_> = query.iter(app.world()).collect();
        for value in &values {
            assert!(value.error.is_none(), "{:?}", value.error);
        }
        if !values.is_empty() && values.len() >= 12 {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    // Frame the whole character.
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
    let mut cameras = app
        .world_mut()
        .query_filtered::<&mut Transform, With<Camera3d>>();
    for mut transform in cameras.iter_mut(app.world_mut()) {
        *transform =
            Transform::from_translation(center + Vec3::Z * radius * 3.).looking_at(center, Vec3::Y);
    }
    // The adopted effect materials must carry the enabled decoded basis.
    let mut hero = app.world_mut().query::<(
        &bevy_stylized::StylizedMaterialAdoption,
        &MeshMaterial3d<StylizedHeroMaterial>,
    )>();
    let effect_handles: Vec<_> = hero
        .iter(app.world())
        .filter(|(adoption, _)| effect_materials.contains(&adoption.material_name))
        .map(|(_, handle)| handle.0.clone())
        .collect();
    assert_eq!(
        effect_handles.len(),
        2,
        "effect meshes adopted hero materials"
    );
    {
        let assets = app.world().resource::<Assets<StylizedHeroMaterial>>();
        for handle in &effect_handles {
            let material = assets.get(handle).expect("hero material");
            let dir = material.params.character.effect_light_dir;
            assert!(dir.w > 0.5, "effect light basis must be enabled");
            assert!(
                dir.z > 0.99,
                "locked upstream basis decodes to +Z, got {dir:?}"
            );
        }
    }
    // Transforms propagate during the first capture; compute the region on
    // settled global transforms, not the pre-update camera.
    let baseline = frame(&mut app, &target);
    let region = effect_region(&mut app, &effect_materials, 512, 512);
    assert!(
        region.2 - region.0 < 500 && region.3 - region.1 < 500,
        "effect region must be a sub-region of the frame, got {region:?}"
    );
    // Visibility: the effect region is non-trivially lit (not black, not
    // the clear color).
    let (x0, y0, x1, y1) = region;
    let mut lit = 0usize;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let p = &baseline.data[(y as usize * 512 + x as usize) * 4..][..4];
            if u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2]) > 140 {
                lit += 1;
            }
        }
    }
    assert!(
        lit > 2000,
        "effect meshes must render visibly lit (lit={lit})"
    );
    // Flip the effect light azimuth: (x,y,z) -> (-x,y,-z).
    {
        let mut assets = app
            .world_mut()
            .resource_mut::<Assets<StylizedHeroMaterial>>();
        let handles = effect_handles.clone();
        for handle in &handles {
            let mut material = assets.get_mut(handle).expect("hero material");
            let dir = material.params.character.effect_light_dir;
            material.params.character.effect_light_dir =
                bevy::math::Vec4::new(-dir.x, dir.y, -dir.z, dir.w);
        }
    }
    let flipped = frame(&mut app, &target);
    // The change must be visible inside the effect region and absent outside
    // it (body/face stay on the view-level key light).
    let mut inside = 0usize;
    let mut outside = 0usize;
    for y in 0..512i32 {
        for x in 0..512i32 {
            let i = (y as usize * 512 + x as usize) * 4;
            let a = &baseline.data[i..i + 4];
            let b = &flipped.data[i..i + 4];
            let changed = (0..4).any(|c| a[c].abs_diff(b[c]) > 8);
            if changed {
                if x >= x0 && x <= x1 && y >= y0 && y <= y1 {
                    inside += 1;
                } else {
                    outside += 1;
                }
            }
        }
    }
    assert!(
        inside > 500,
        "azimuth flip must visibly reshape effect shading (inside={inside})"
    );
    assert_eq!(
        outside, 0,
        "effect light override leaked outside the effect meshes ({outside} px)"
    );
    eprintln!("EFFECT_SHADING_PASS lit={lit} inside={inside} region=({x0},{y0})-({x1},{y1})");
}
