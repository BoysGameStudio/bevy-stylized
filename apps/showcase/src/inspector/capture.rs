//! 捕获生命周期：双目标同步读回 + 代次/修订身份（generation/revision
//! 与 scenario-model 的 Generation/Revision 一一对应）+ 晚到结果排空。
//!
//! 共享运行时对齐（H4b）：读回一律经 harness 的 stamp 回执
//! （`try_take_stamped`，phase E 的 CaptureStamp——请求序列/主世界 update
//! 序号/渲染帧），capture.json 以 additive `stamps` 字段携带两侧 stamp。
//! 保留本模块语义的明确理由：
//!
//! - 双目标（A/B 两侧）同步捕获不是单相机 CaptureCycle 的形态；
//! - retire + 排空（晚到结果**可取回并计数**）是 Inspector 的既有验收
//!   合同（`retired_captures` 度量依赖它），与 harness `cancel_capture`
//!   的丢弃语义不同——不替换；
//! - capture.json 的既有身份键是 dev.py inspector lane 的验收产物，
//!   只做 additive 扩展。
use super::{Recipe, Runtime, State, asset};
use bevy::prelude::*;
use golden_harness::{CaptureStamp, CapturedImage, capture as gpu};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(super) struct Pending {
    generation: u64,
    revision: u64,
    recipe: Recipe,
    requests: [Option<gpu::CaptureRequest>; 2],
    images: [Option<CapturedImage>; 2],
    stamps: [Option<CaptureStamp>; 2],
    started: Instant,
}
fn retire_requests(pending: &mut Option<Pending>, retired: &mut Vec<gpu::CaptureRequest>) -> bool {
    if let Some(p) = pending.take() {
        retired.extend(p.requests.into_iter().flatten());
        true
    } else {
        false
    }
}
pub(super) fn retire(runtime: &mut Runtime, state: &mut State) {
    if retire_requests(&mut runtime.pending, &mut runtime.retired) {
        state.retired += 1;
        state.status = "Capture cancelled: inspection state changed".into();
    }
}
pub(super) fn update(
    runtime: &mut Runtime,
    world: &mut World,
    state: &mut State,
) -> Result<(), String> {
    if runtime
        .window_due
        .as_ref()
        .is_some_and(|(_, frame)| runtime.last_frame >= *frame)
    {
        use bevy::render::view::window::screenshot::Screenshot;
        let (path, _) = runtime.window_due.take().unwrap();
        let entity = world
            .spawn(Screenshot::primary_window())
            .observe(window_captured)
            .id();
        runtime.window_capture = Some((entity, path, runtime.generation, state.revision));
    }
    if runtime
        .pending
        .as_ref()
        .is_some_and(|p| p.generation != runtime.generation || p.revision != state.revision)
    {
        retire(runtime, state);
    }
    if state.capture && state.ready && runtime.pending.is_none() {
        state.capture = false;
        let requests =
            std::array::from_fn(|i| Some(gpu::request_capture(world, runtime.targets[i].clone())));
        runtime.pending = Some(Pending {
            generation: runtime.generation,
            revision: state.revision,
            recipe: state.recipe.clone(),
            requests,
            images: [None, None],
            stamps: [None, None],
            started: Instant::now(),
        });
        state.status = "Capturing current A/B generation…".into();
    }
    let Some(p) = runtime.pending.as_mut() else {
        return Ok(());
    };
    if p.started.elapsed() > Duration::from_secs(30) {
        return Err("GPU readback timed out".into());
    }
    for i in 0..2 {
        if let Some(request) = &p.requests[i]
            && let Some(capture) = gpu::try_take_stamped(world, request.clone())
        {
            p.images[i] = Some(capture.image);
            p.stamps[i] = Some(capture.stamp);
            p.requests[i] = None;
        }
    }
    if p.images.iter().any(Option::is_none) {
        return Ok(());
    }
    let p = runtime.pending.take().unwrap();
    let directory = if runtime.options.once {
        runtime.options.out.clone()
    } else {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        runtime.options.out.join(format!("take-{stamp}"))
    };
    if let Some(parent) = directory.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir(&directory)
        .map_err(|e| format!("new capture directory {}: {e}", directory.display()))?;
    let mut image_hashes = std::collections::BTreeMap::new();
    for (name, image) in ["a.png", "b.png"]
        .into_iter()
        .zip(p.images.into_iter().flatten())
    {
        let png = golden_harness::encode_png(&image).map_err(|e| e.to_string())?;
        image_hashes.insert(name, asset::sha(&png));
        std::fs::write(directory.join(name), png).map_err(|e| e.to_string())?;
    }
    std::fs::write(
        directory.join("recipe.json"),
        serde_json::to_vec_pretty(&p.recipe).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let stamps: Vec<serde_json::Value> = p
        .stamps
        .into_iter()
        .flatten()
        .map(|stamp| {
            serde_json::json!({
                "sequence": stamp.sequence,
                "issued_update_index": stamp.issued_update_index,
                "completed_update_index": stamp.completed_update_index,
                "completed_render_frame": stamp.completed_render_frame,
            })
        })
        .collect();
    let report = serde_json::json!({"schema":"bevy_stylized.inspector-capture/v1","status":"PASS","scope":"current live A/B capture, not upstream parity or an accepted baseline","generation":p.generation,"revision":p.revision,"retired_captures":state.retired,"ready_render_frames":state.stable,"material_primitives_per_view":runtime.input.meshes,"msaa":4,"supersampling":false,"images":image_hashes,"files":p.recipe.files,"renderer_version":env!("CARGO_PKG_VERSION"),"device":world.get_resource::<bevy::render::renderer::RenderAdapterInfo>().map(|info|format!("{:?}",info.0)),"inspector_source_sha256":super::source_sha256(),"inspector_source_policy":"module-sources/v2","stamps":stamps});
    std::fs::write(
        directory.join("capture.json"),
        serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    state.captures += 1;
    state.status = format!("Saved {}", directory.display());
    if runtime.options.once {
        if runtime.options.headless {
            runtime.done = true;
        } else {
            runtime.window_due = Some((directory.join("window.png"), runtime.last_frame + 8));
        }
    }
    Ok(())
}

fn window_captured(
    event: On<bevy::render::view::window::screenshot::ScreenshotCaptured>,
    mut runtime: ResMut<Runtime>,
    mut state: ResMut<State>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some((entity, path, generation, revision)) = runtime.window_capture.take() else {
        return;
    };
    if event.entity != entity {
        runtime.window_capture = Some((entity, path, generation, revision));
        return;
    }
    if generation != runtime.generation || revision != state.revision {
        state.error = Some("UI capture invalidated by inspection change".into());
        exit.write(AppExit::error());
        return;
    }
    let result = event
        .image
        .clone()
        .try_into_dynamic()
        .map_err(|e| e.to_string())
        .and_then(|i| i.save(path).map_err(|e| e.to_string()));
    if let Err(error) = result {
        state.error = Some(error);
        exit.write(AppExit::error());
    } else {
        runtime.done = true;
        exit.write(AppExit::Success);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retiring_requests_preserves_gpu_entities_and_drains_late_results() {
        use crate::inspector::state::{DebugMode, OrbitCamera};
        use bevy::render::gpu_readback::{Readback, ReadbackComplete};
        let mut app = App::new();
        gpu::install(&mut app);
        let target = gpu::CaptureTarget {
            image: Handle::default(),
            width: 1,
            height: 1,
        };
        let requests =
            std::array::from_fn(|_| Some(gpu::request_capture(app.world_mut(), target.clone())));
        let entities = app
            .world_mut()
            .query_filtered::<Entity, With<Readback>>()
            .iter(app.world())
            .collect::<Vec<_>>();
        let c = OrbitCamera {
            target: [0.; 3],
            yaw: 0.,
            pitch: 0.,
            distance: 1.,
        };
        let recipe = Recipe {
            schema: "bevy_stylized.inspector/v1".into(),
            files: Default::default(),
            cameras: [c.clone(), c],
            linked: true,
            debug: [DebugMode::Beauty; 2],
            light: [0., 1., 0.],
            exposure: 1.,
            resolution: 512,
        };
        let mut pending = Some(Pending {
            generation: 1,
            revision: 0,
            recipe,
            requests,
            images: [None, None],
            stamps: [None, None],
            started: Instant::now(),
        });
        let mut retired = Vec::new();
        assert!(retire_requests(&mut pending, &mut retired));
        assert!(pending.is_none());
        assert_eq!(entities.len(), 2);
        assert_eq!(retired.len(), 2);
        for entity in entities {
            assert!(app.world().get_entity(entity).is_ok());
            let mut bytes = vec![0; 256];
            bytes[..4].copy_from_slice(&[12, 34, 56, 255]);
            app.world_mut().trigger(ReadbackComplete {
                entity,
                data: bytes,
            });
        }
        app.world_mut().flush();
        for request in retired {
            let frame = gpu::try_take(app.world_mut(), request.clone()).unwrap();
            assert_eq!(frame.data, [12, 34, 56, 255]);
            assert!(gpu::try_take(app.world_mut(), request).is_none());
        }
    }
}
