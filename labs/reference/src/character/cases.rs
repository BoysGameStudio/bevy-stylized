//! Fixed six-case execution and its report; no interactive recipe policy.
use super::diagnostics::{coverage, lighting_diagnostics};
use super::frames::{frame, frame_linear, pipelines_ready, readiness_timeout};
use super::io::{read, sha, vec, write};
use super::metrics::{coverage_iou, repeat};
use super::{input::Package, scene};
use bevy::{
    asset::LoadState,
    camera::{Exposure, ScalingMode},
    core_pipeline::tonemapping::Tonemapping,
    prelude::*,
};
use bevy_stylized::{StylizedMaterialAdoption, prelude::*};
use golden_harness::{Tier2Thresholds, capture, evaluate_tier2};
use serde_json::{Value, json};
use std::{
    path::Path,
    time::{Duration, Instant},
};

pub(super) fn compare(
    path: &Path,
    out: &Path,
    root: &Path,
    p: Package,
    prepared: scene::Scene,
) -> Result<Value, String> {
    let scene::Scene {
        mut app,
        target,
        linear_target,
        mut camera,
        sidecar,
        view,
        scene,
        expected,
        textures,
        calibration,
    } = prepared;
    let mut reports = Vec::new();
    for case in p.cases {
        let c = &case.camera;
        if let Some(linear) = &linear_target {
            app.world_mut().despawn(camera);
            camera = app
                .world_mut()
                .spawn((
                    Camera3d::default(),
                    Camera::default(),
                    bevy::camera::Hdr,
                    bevy::camera::RenderTarget::Image(linear.image.clone().into()),
                    Msaa::Off,
                    Tonemapping::None,
                    Transform::default(),
                ))
                .id();
        }

        app.world_mut().entity_mut(camera).insert((
            Transform::from_translation(vec(c.position)).looking_at(vec(c.target), vec(c.up)),
            Projection::Orthographic(OrthographicProjection {
                near: c.near,
                far: c.far,
                scaling_mode: ScalingMode::Fixed {
                    width: c.scale,
                    height: c.scale,
                },
                ..OrthographicProjection::default_3d()
            }),
        ));
        let mut settings = view.clone();
        settings.key_light_direction = vec(case.direction_to_light);
        settings.exposure = 1.;
        *app.world_mut().resource_mut::<StylizedViewSettings>() = StylizedViewSettings(settings);
        let start = Instant::now();
        let mut stable = 0;
        loop {
            app.update();
            let mut q = app.world_mut().query::<&StylizedMaterialAdoption>();
            let adopted: Vec<_> = q.iter(app.world()).collect();
            if let Some(e) = adopted.iter().find_map(|a| a.error.as_ref()) {
                return Err(e.clone());
            }
            let server = app.world().resource::<AssetServer>();
            if let Some(LoadState::Failed(e)) = server.get_load_state(scene.id()) {
                return Err(format!("scene load failed: {e}"));
            }
            let mut loaded = server.is_loaded_with_dependencies(scene.id());
            for uri in &textures {
                if let Some(handle) = server.get_handle::<Image>(uri) {
                    if let Some(LoadState::Failed(e)) = server.get_load_state(handle.id()) {
                        return Err(format!("texture load failed: {e}"));
                    }
                    loaded &= server.is_loaded_with_dependencies(handle.id());
                } else {
                    loaded = false;
                }
            }
            if loaded && adopted.len() == expected && pipelines_ready(&app)? {
                stable += 1
            } else {
                stable = 0
            }
            if stable >= 24 {
                break;
            }
            if start.elapsed() > readiness_timeout() {
                return Err(format!("asset/pipeline readiness timeout: {}", case.id));
            }
            std::thread::sleep(Duration::from_millis(3));
        }
        let a = if let (Some(linear), Some(scale)) = (&linear_target, p.supersample) {
            frame_linear(&mut app, linear, scale)?
        } else {
            frame(&mut app, &target)?
        };
        let b = if let (Some(linear), Some(scale)) = (&linear_target, p.supersample) {
            frame_linear(&mut app, linear, scale)?
        } else {
            frame(&mut app, &target)?
        };
        if linear_target.is_some() {
            let transform = *app
                .world()
                .get::<Transform>(camera)
                .ok_or("camera transform")?;
            let projection = app
                .world()
                .get::<Projection>(camera)
                .ok_or("camera projection")?
                .clone();
            app.world_mut().despawn(camera);
            camera = app
                .world_mut()
                .spawn((
                    Camera3d::default(),
                    capture::capture_target_bundle(&target),
                    Tonemapping::None,
                    Msaa::Sample4,
                    transform,
                    projection,
                ))
                .id();
            for _ in 0..24 {
                app.update();
            }
        }
        let ra = golden_harness::decode_png(&read(root.join(&case.reference[0]))?)
            .map_err(|e| e.to_string())?;
        let rb = golden_harness::decode_png(&read(root.join(&case.reference[1]))?)
            .map_err(|e| e.to_string())?;
        let repeat_up = repeat(&ra, &rb)?;
        let repeat_bevy = repeat(&a, &b)?;
        if a.width != ra.width || a.height != ra.height {
            return Err("cross-renderer dimensions mismatch".into());
        }
        let color = Color::linear_rgb(
            p.background_linear[0],
            p.background_linear[1],
            p.background_linear[2],
        )
        .to_srgba()
        .to_u8_array();
        let tier = evaluate_tier2(
            &case.id,
            &ra,
            &a,
            [color[0], color[1], color[2]],
            Tier2Thresholds::default(),
        )
        .map_err(|e| e.to_string())?;
        let mut difference = a.clone();
        for ((pixel, left), right) in difference
            .data
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(ra.data.as_chunks::<4>().0.iter())
            .zip(a.data.as_chunks::<4>().0.iter())
        {
            for c in 0..3 {
                pixel[c] = left[c].abs_diff(right[c]).saturating_mul(4);
            }
            pixel[3] = 255;
        }
        std::fs::write(
            out.join(format!("{}-diff.png", case.id)),
            golden_harness::encode_png(&difference).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::write(
            out.join(format!("{}-bevy-repeat.png", case.id)),
            golden_harness::encode_png(&b).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let name = format!("{}-bevy.png", case.id);
        let png = golden_harness::encode_png(&a).map_err(|e| e.to_string())?;
        std::fs::write(out.join(&name), &png).map_err(|e| e.to_string())?;
        std::fs::copy(
            root.join(&case.reference[0]),
            out.join(format!("{}-blender.png", case.id)),
        )
        .map_err(|e| e.to_string())?;
        let mask = coverage(&mut app, &target)?;
        let reference_mask = golden_harness::decode_png(&read(root.join(&case.silhouette))?)
            .map_err(|e| e.to_string())?;
        let silhouette = coverage_iou(&reference_mask, &mask)?;
        std::fs::write(
            out.join(format!("{}-coverage-bevy.png", case.id)),
            golden_harness::encode_png(&mask).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::copy(
            root.join(&case.silhouette),
            out.join(format!("{}-coverage-blender.png", case.id)),
        )
        .map_err(|e| e.to_string())?;
        let mut light_query = app
            .world_mut()
            .query_filtered::<(&Transform, &DirectionalLight), With<StylizedKeyLight>>();
        let (light_transform, light_component) =
            light_query.single(app.world()).map_err(|e| e.to_string())?;
        let actual_light = -light_transform.forward().as_vec3();
        if actual_light.distance(vec(case.direction_to_light)) > 1e-5 {
            return Err("actual key light direction mismatch".into());
        }
        let actual_illuminance = light_component.illuminance;
        let entity = app.world().entity(camera);
        let transform = entity.get::<Transform>().ok_or("camera transform")?;
        let projection = entity.get::<Projection>().ok_or("camera projection")?;
        let exposure = entity
            .get::<Exposure>()
            .ok_or("camera exposure")?
            .exposure();
        let forward = transform.forward().as_vec3();
        let expected_forward = (vec(c.target) - vec(c.position)).normalize();
        if transform.translation.distance(vec(c.position)) > 1e-5
            || forward.distance(expected_forward) > 1e-5
            || (exposure - 1.).abs() > 1e-5
        {
            return Err("runtime camera/exposure mismatch".into());
        }
        let Projection::Orthographic(ortho) = projection else {
            return Err("runtime projection mismatch".into());
        };
        if (ortho.area.width() - c.scale).abs() > 1e-5
            || (ortho.area.height() - c.scale).abs() > 1e-5
        {
            return Err("runtime orthographic dimensions mismatch".into());
        }
        let actual = json!({"key_light_direction":actual_light.to_array(),"key_light_illuminance":actual_illuminance,"position":transform.translation.to_array(),"rotation":transform.rotation.to_array(),"projection":format!("{projection:?}"),"orthographic_width":ortho.area.width(),"orthographic_height":ortho.area.height(),"exposure_multiplier":exposure,
            "view":app.world().resource::<StylizedViewSettings>().0,"background":format!("{:?}",app.world().resource::<ClearColor>().0),"adopted_primitives":expected});
        write(out.join(format!("{}-bevy-actual.json", case.id)), &actual)?;
        let semantic_shadow = if let (Some(factor), Some(center)) =
            (&case.factor_reference, &case.center_reference)
        {
            lighting_diagnostics(
                &mut app, &target, camera, root, out, &case.id, factor, center,
            )?
        } else {
            json!({"status":"UNQUALIFIED","reason":"No lighting-factor references in this v1 case"})
        };
        let color_passed = tier.max_abs_delta_nonedge <= 16.0 / 255.0
            && tier.mean_abs_delta <= 4.0 / 255.0
            && tier.psnr_db >= 30.0;
        let passed = if p.schema.ends_with("/v3") {
            color_passed
                && repeat_up["passed"] == true
                && repeat_bevy["passed"] == true
                && semantic_shadow["status"] == "PASS"
        } else {
            tier.passed
                && repeat_up["passed"] == true
                && repeat_bevy["passed"] == true
                && silhouette["passed"] == true
                && semantic_shadow["status"] == "PASS"
        };
        reports.push(json!({"case":case.id,"image_sha256":sha(&png),"color_criteria_passed":color_passed,"sampling_grid":p.supersample,"silhouette":silhouette,"repeat_blender":repeat_up,"repeat_bevy":repeat_bevy,"legacy_tier2":tier,
            "input_alignment":{"status":"PASS","scope":"calibrated camera/color and declared NPR direction; not shader formula equivalence"},"semantic_shadow":semantic_shadow,"parity_status":if passed {"PASS"}else{"FAIL"}}));
    }
    let gpu = app
        .world()
        .resource::<bevy::render::renderer::RenderAdapterInfo>();
    let device = json!({"name":gpu.name,"backend":format!("{:?}",gpu.backend),"driver":gpu.driver,"driver_info":gpu.driver_info});
    let capabilities = format!(
        "{:?}",
        app.world().get_resource::<StylizedRenderCapabilities>()
    );
    let report = json!({"schema":"bevy_stylized.character-comparison/v1","input_sha256":sha(&read(path)?),"device":device,"capabilities":capabilities,"material_classes":sidecar.materials.values().map(|m|format!("{:?}",m.surface_class)).collect::<Vec<_>>(),"capture_status":"PASS","parity_status":if reports.iter().all(|r|r["parity_status"]=="PASS") {"PASS"}else{"FAIL"},"threshold_policy":"RGB thresholds from docs/reference/08 unchanged; matched v3 uses center coverage IoU>=0.99 and sharp lighting-factor IoU>=0.95; legacy AA and luminance proxies remain diagnostic","calibration":calibration,"reports":reports});
    write(out.join("report.json"), &report)?;
    Ok(report)
}
