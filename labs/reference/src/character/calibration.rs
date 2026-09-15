//! Geometry, camera axes and color calibration before character comparison.
use super::{
    frames::{frame, pipelines_ready, readiness_timeout},
    input::CameraInput,
    io::{read, vec, write},
};
use bevy::{
    camera::ScalingMode, gltf::GltfAssetLabel, prelude::*, world_serialization::WorldAssetRoot,
};
use golden_harness::{CapturedImage, capture};
use serde_json::{Value, json};
use std::{
    path::Path,
    time::{Duration, Instant},
};

pub(super) fn calibrate(
    app: &mut App,
    root: &Path,
    out: &Path,
    target: &capture::CaptureTarget,
    camera: Entity,
) -> Result<Value, String> {
    let record: Value =
        serde_json::from_slice(&read(root.join("calibration.json"))?).map_err(|e| e.to_string())?;
    let input: CameraInput =
        serde_json::from_value(record["camera"].clone()).map_err(|e| e.to_string())?;
    app.world_mut().entity_mut(camera).insert((
        Transform::from_translation(vec(input.position))
            .looking_at(vec(input.target), vec(input.up)),
        Projection::Orthographic(OrthographicProjection {
            near: input.near,
            far: input.far,
            scaling_mode: ScalingMode::Fixed {
                width: input.scale,
                height: input.scale,
            },
            ..OrthographicProjection::default_3d()
        }),
    ));
    let scene: Handle<bevy::world_serialization::WorldAsset> = app
        .world()
        .resource::<AssetServer>()
        .load(GltfAssetLabel::Scene(0).from_asset("calibration.glb"));
    let entity = app
        .world_mut()
        .spawn((WorldAssetRoot(scene.clone()), Transform::default()))
        .id();
    let start = Instant::now();
    let mut stable = 0;
    loop {
        app.update();
        if app
            .world()
            .resource::<AssetServer>()
            .is_loaded_with_dependencies(scene.id())
            && pipelines_ready(app)?
        {
            stable += 1
        } else {
            stable = 0
        }
        if stable >= 24 {
            break;
        }
        if start.elapsed() > readiness_timeout() {
            return Err("calibration readiness timeout".into());
        }
        std::thread::sleep(Duration::from_millis(3));
    }
    let image = frame(app, target)?;
    let reference = golden_harness::decode_png(&read(root.join("calibration-blender.png"))?)
        .map_err(|e| e.to_string())?;
    let mut checks = Vec::new();
    for marker in record["markers"].as_array().ok_or("calibration markers")? {
        let x = marker["pixel"][0].as_u64().ok_or("pixel x")? as usize;
        let y = marker["pixel"][1].as_u64().ok_or("pixel y")? as usize;
        if x >= 512 || y >= 512 {
            return Err("calibration pixel out of bounds".into());
        }
        let color: [f32; 3] =
            serde_json::from_value(marker["linear_color"].clone()).map_err(|e| e.to_string())?;
        let expected = Color::linear_rgb(color[0], color[1], color[2])
            .to_srgba()
            .to_u8_array();
        let offset = (y * 512 + x) * 4;
        let delta = (0..3)
            .map(|c| {
                image.data[offset + c]
                    .abs_diff(expected[c])
                    .max(reference.data[offset + c].abs_diff(expected[c]))
            })
            .max()
            .unwrap_or(255);
        let centroid = |im: &CapturedImage| -> Result<[f64; 2], String> {
            let mut n = 0.;
            let mut sum = [0., 0.];
            for (i, p) in im.data.as_chunks::<4>().0.iter().enumerate() {
                if (0..3).all(|c| p[c].abs_diff(expected[c]) <= 2) {
                    n += 1.;
                    sum[0] += (i % 512) as f64 + 0.5;
                    sum[1] += (i / 512) as f64 + 0.5;
                }
            }
            if n < 100. {
                return Err("calibration marker missing".into());
            }
            Ok([sum[0] / n, sum[1] / n])
        };
        let a = centroid(&image)?;
        let b = centroid(&reference)?;
        let position_error = (a[0] - b[0])
            .abs()
            .max((a[1] - b[1]).abs())
            .max((a[0] - x as f64).abs())
            .max((a[1] - y as f64).abs());
        checks.push(json!({"marker":marker["name"],"max_channel_delta":delta,"position_error_pixels":position_error,"passed":delta<=2&&position_error<=1.}));
    }
    let passed = checks.iter().all(|c| c["passed"] == true);
    std::fs::write(
        out.join("calibration-bevy.png"),
        golden_harness::encode_png(&image).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::copy(
        root.join("calibration-blender.png"),
        out.join("calibration-blender.png"),
    )
    .map_err(|e| e.to_string())?;
    app.world_mut().despawn(entity);
    for _ in 0..8 {
        app.update();
    }
    let result = json!({"passed":passed,"checks":checks,"scope":"geometry, projected axes, color and unit exposure; excludes NPR shading"});
    write(out.join("calibration-report.json"), &result)?;
    if !passed {
        return Err("geometry/color calibration failed; inspect calibration-report.json".into());
    }
    Ok(result)
}
