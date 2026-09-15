//! Frame-indexed scans and separate instrumented performance measurements.
use super::{Recipe, Runtime, State, asset, fit, state::DebugMode};
use bevy::{
    diagnostic::DiagnosticsStore,
    prelude::*,
    render::{
        RenderApp,
        diagnostic::{RecordDiagnostics, RenderDiagnosticsPlugin},
        renderer::{RenderDevice, RenderGraph, RenderGraphSystems},
    },
};
use golden_harness::{CapturedImage, capture as gpu};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    path::Path,
    time::{Duration, Instant},
};

pub(super) fn install_measurement(app: &mut App) {
    app.add_plugins(RenderDiagnosticsPlugin);
    app.sub_app_mut(RenderApp).add_systems(
        RenderGraph,
        (
            begin
                .after(RenderGraphSystems::Begin)
                .before(RenderGraphSystems::Render),
            end.after(RenderGraphSystems::Render)
                .before(bevy::render::diagnostic::resolve_encoder),
        ),
    );
}
fn begin(mut context: bevy::render::renderer::RenderContext) {
    let recorder = context
        .diagnostic_recorder()
        .expect("measurement plugin installed");
    recorder.begin_time_span(context.command_encoder(), "inspector_frame".into());
}
fn end(mut context: bevy::render::renderer::RenderContext) {
    let recorder = context
        .diagnostic_recorder()
        .expect("measurement plugin installed");
    recorder.end_time_span(context.command_encoder());
}
fn update(app: &mut App) -> Result<(), String> {
    app.update();
    if let Some(e) = &app.world().resource::<State>().error {
        return Err(e.clone());
    }
    Ok(())
}
fn ready(app: &mut App) -> Result<(), String> {
    let start = Instant::now();
    loop {
        update(app)?;
        if app.world().resource::<State>().ready {
            return Ok(());
        }
        if start.elapsed() > Duration::from_secs(65) {
            return Err("validation readiness timeout".into());
        }
    }
}
fn write(path: &Path, value: &Value) -> Result<(), String> {
    std::fs::write(
        path,
        serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
fn sample(base: &Recipe, phase: &str, frame: usize, frames: usize) -> Recipe {
    let t = frame as f32 / (frames - 1) as f32;
    let mut r = base.clone();
    match phase {
        "orbit" => {
            for c in &mut r.cameras {
                c.yaw += std::f32::consts::TAU * t;
            }
        }
        "light" => {
            let angle = std::f32::consts::TAU * t;
            let y = 0.35_f32;
            let h = (1. - y * y).sqrt();
            r.light = if frame == frames - 1 {
                [0., y, h]
            } else {
                [angle.sin() * h, y, angle.cos() * h]
            };
        }
        "zoom" => {
            let distance =
                base.cameras[0].distance * (0.4 + 0.6 * (std::f32::consts::PI * t).cos().abs());
            for c in &mut r.cameras {
                c.distance = distance;
            }
        }
        _ => unreachable!(),
    }
    if frame == frames - 1 && phase == "orbit" {
        r = base.clone();
    }
    r
}
fn apply(app: &mut App, recipe: Recipe) -> Result<(), String> {
    app.world_mut().resource_mut::<State>().recipe = recipe.clone();
    update(app)?;
    // Demand current-frame light propagation; a one-frame controller lag must
    // not masquerade as a shader discontinuity during the scan.
    let mut q = app
        .world_mut()
        .query_filtered::<&Transform, With<bevy_stylized::prelude::StylizedKeyLight>>();
    let actual = q
        .iter(app.world())
        .next()
        .ok_or("missing managed light")?
        .back()
        .as_vec3();
    if actual.distance(Vec3::from_array(recipe.light)) > 1e-5 {
        return Err("light transform lags the declared scan frame".into());
    }
    Ok(())
}
struct Readback {
    phase: String,
    repeat: usize,
    frame: usize,
    recipe: Recipe,
    requests: [Option<gpu::CaptureRequest>; 2],
    images: [Option<CapturedImage>; 2],
}
fn drain(
    app: &mut App,
    queue: &mut VecDeque<Readback>,
    out: &Path,
    rows: &mut Vec<Value>,
    pixels: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    for item in queue.iter_mut() {
        for i in 0..2 {
            if let Some(r) = &item.requests[i]
                && let Some(image) = gpu::try_take(app.world_mut(), r.clone())
            {
                item.images[i] = Some(image);
                item.requests[i] = None;
            }
        }
    }
    while queue
        .front()
        .is_some_and(|r| r.images.iter().all(Option::is_some))
    {
        let item = queue.pop_front().unwrap();
        let mut images = Vec::new();
        for (side, image) in item.images.into_iter().flatten().enumerate() {
            let key = format!("{}-{:03}-{side}", item.phase, item.frame);
            let digest = asset::sha(&image.data);
            let equal = if item.repeat == 0 {
                pixels.insert(key, digest.clone());
                true
            } else {
                pixels.get(&key) == Some(&digest)
            };
            let name = format!(
                "{}-{}-{:03}-{side}.png",
                item.phase, item.repeat, item.frame
            );
            let png = golden_harness::encode_png(&image).map_err(|e| e.to_string())?;
            std::fs::write(out.join(&name), png).map_err(|e| e.to_string())?;
            images.push(json!({"file":name,"pixel_sha256":digest,"repeat_identical":equal}));
        }
        rows.push(json!({"phase":item.phase,"repeat":item.repeat,"frame":item.frame,"recipe":item.recipe,"images":images}));
    }
    Ok(())
}
pub(super) fn run(app: &mut App, mode: &str) -> Result<(), String> {
    ready(app)?;
    if mode == "--measure" {
        use bevy::render::render_resource::WgpuFeatures;
        if !app
            .world()
            .resource::<RenderDevice>()
            .features()
            .contains(WgpuFeatures::TIMESTAMP_QUERY | WgpuFeatures::TIMESTAMP_QUERY_INSIDE_ENCODERS)
        {
            return Err("required GPU timestamps unavailable".into());
        }
    }
    let runtime = app.world().resource::<Runtime>();
    let out = runtime.options.out.clone();
    let frames = runtime.options.frames;
    std::fs::create_dir_all(out.parent().ok_or("output parent")?).map_err(|e| e.to_string())?;
    std::fs::create_dir(&out).map_err(|e| e.to_string())?;
    let base = app.world().resource::<State>().recipe.clone();
    write(
        &out.join("recipe.json"),
        &serde_json::to_value(&base).map_err(|e| e.to_string())?,
    )?;
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_secs_f64(1. / 60.),
    ));
    let report = if mode == "--scan" {
        scan(app, &out, base, frames)?
    } else {
        measure(app, base, frames)?
    };
    write(&out.join("report.json"), &report)?;
    if report["status"] != "PASS" {
        return Err("validation failed; inspect report.json".into());
    }
    Ok(())
}
fn scan(app: &mut App, out: &Path, base: Recipe, frames: usize) -> Result<Value, String> {
    let targets = app.world().resource::<Runtime>().targets.clone();
    let mut rows = Vec::new();
    let mut pixels = BTreeMap::new();
    for phase in ["orbit", "light", "zoom"] {
        let mut phase_base = base.clone();
        if phase == "zoom" {
            let runtime = app.world().resource::<Runtime>();
            let (min, max) = runtime.bounds.ok_or("bounds")?;
            let c = fit(min, max, true, runtime.face_focus);
            phase_base.cameras = [c.clone(), c];
        }
        for repeat in 0..2 {
            apply(app, sample(&phase_base, phase, 0, frames))?;
            ready(app)?;
            let mut queue = VecDeque::new();
            for frame in 0..frames {
                let recipe = sample(&phase_base, phase, frame, frames);
                // Readback is submitted before update so it captures the same
                // serial render frame in which this recipe is applied.
                let requests = std::array::from_fn(|i| {
                    Some(gpu::request_capture(app.world_mut(), targets[i].clone()))
                });
                app.world_mut().resource_mut::<State>().recipe = recipe.clone();
                apply(app, recipe.clone())?;
                queue.push_back(Readback {
                    phase: phase.into(),
                    repeat,
                    frame,
                    recipe,
                    requests,
                    images: [None, None],
                });
                drain(app, &mut queue, out, &mut rows, &mut pixels)?;
                if queue.len() > 32 {
                    return Err("readback backlog exceeds bounded scan queue".into());
                }
            }
            let start = Instant::now();
            while !queue.is_empty() {
                update(app)?;
                drain(app, &mut queue, out, &mut rows, &mut pixels)?;
                if start.elapsed() > Duration::from_secs(30) {
                    return Err("scan drain timed out".into());
                }
            }
        }
    }
    let closures:Vec<_>=["orbit","light","zoom"].into_iter().map(|phase|json!({"phase":phase,"identical":(0..2).all(|side|pixels.get(&format!("{phase}-000-{side}"))==pixels.get(&format!("{phase}-{:03}-{side}",frames-1)))})).collect();
    let passed = closures.iter().all(|r| r["identical"] == true)
        && rows.iter().all(|r| {
            r["images"]
                .as_array()
                .unwrap()
                .iter()
                .all(|i| i["repeat_identical"] == true)
        });
    Ok(
        json!({"schema":"bevy_stylized.realtime-scan/v1","status":if passed{"PASS"}else{"FAIL"},"scope":"frame-indexed replay equality and review frames; repeatability alone does not qualify absence of temporal artifacts","msaa":4,"frames_per_phase":frames,"virtual_fps":60,"pipelined_rendering":false,"phases":["orbit","light","zoom"],"repeats":2,"loop_closure":closures,"readback_perturbs_wall_clock":true,"rows":rows}),
    )
}
fn stats(values: &[f64]) -> Value {
    if values.is_empty() {
        return Value::Null;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    json!({"count":n,"mean":values.iter().sum::<f64>()/n as f64,"median":if n.is_multiple_of(2){(sorted[n/2-1]+sorted[n/2])*0.5}else{sorted[n/2]},"p95":sorted[((n as f64*0.95).ceil() as usize).saturating_sub(1)],"min":sorted[0],"max":sorted[n-1]})
}
fn measure(app: &mut App, base: Recipe, frames: usize) -> Result<Value, String> {
    let mut reports = Vec::new();
    for mode in [DebugMode::Beauty, DebugMode::Lighting] {
        let mut base = base.clone();
        base.debug = [mode; 2];
        apply(app, base.clone())?;
        ready(app)?;
        for frame in 0..60 {
            apply(app, sample(&base, "orbit", frame, 60))?;
        }
        let mut wall = Vec::new();
        let mut diagnostics: BTreeMap<String, Vec<f64>> = BTreeMap::new();
        let mut seen = BTreeMap::new();
        // Prime the timestamp identities so no previous-mode samples are admitted.
        for d in app.world().resource::<DiagnosticsStore>().iter() {
            if let Some(m) = d.measurement() {
                seen.insert(d.path().to_string(), format!("{:?}", m.time));
            }
        }
        for frame in 0..frames {
            let start = Instant::now();
            apply(app, sample(&base, "orbit", frame, frames))?;
            wall.push(start.elapsed().as_secs_f64() * 1000.);
            for d in app.world().resource::<DiagnosticsStore>().iter() {
                if let Some(m) = d.measurement() {
                    let key = d.path().to_string();
                    let time = format!("{:?}", m.time);
                    if seen.get(&key) != Some(&time) {
                        if !m.value.is_finite() {
                            return Err("nonfinite render diagnostic".into());
                        }
                        seen.insert(key.clone(), time);
                        diagnostics.entry(key).or_default().push(m.value);
                    }
                }
            }
        }
        let device = app.world().resource::<RenderDevice>();
        let memory=device.wgpu_device().generate_allocator_report().map(|r|json!({"allocated_bytes":r.total_allocated_bytes,"reserved_bytes":r.total_reserved_bytes,"scope":"wgpu device allocator, not total process VRAM"}));
        let outer = diagnostics.get("render/inspector_frame/elapsed_gpu");
        let outer_mean = outer
            .map(|v| v.iter().sum::<f64>() / v.len() as f64)
            .unwrap_or(0.);
        for (path, values) in &diagnostics {
            if path.ends_with("elapsed_gpu")
                && path != "render/inspector_frame/elapsed_gpu"
                && values.iter().sum::<f64>() / values.len() as f64 > outer_mean + 0.01
            {
                return Err("GPU timing bookends do not enclose child render passes".into());
            }
        }
        let aggregate: BTreeMap<_, _> = diagnostics
            .iter()
            .map(|(k, v)| (k.clone(), stats(v)))
            .collect();
        reports.push(json!({"mode":mode,"warmup_render_frames":60,"requested_samples":frames,"app_update_wall_ms":stats(&wall),"diagnostics":aggregate,"raw_diagnostics":diagnostics,"memory":memory,"device_features":format!("{:?}",device.features()),"draw_calls":{"status":"UNAVAILABLE","reason":"stock Bevy diagnostics expose pipeline invocations, not submitted draw-command counts; no mesh-count substitution"}}));
    }
    let gpu = reports.iter().all(|r| {
        r["diagnostics"]["render/inspector_frame/elapsed_gpu"]["count"]
            .as_u64()
            .unwrap_or(0)
            >= (frames as f64 * 0.9).ceil() as u64
    });
    Ok(
        json!({"schema":"bevy_stylized.realtime-performance/v1","status":if gpu{"PASS"}else{"FAIL"},"scope":"instrumented two-view inspector baseline, not a shipping-frame budget or accepted regression baseline","optimized":!cfg!(debug_assertions),"resolution_per_view":[base.resolution,base.resolution],"view_count":2,"msaa":4,"capture_readbacks":false,"diagnostic_readbacks":true,"pipelined_rendering":false,"device":app.world().get_resource::<bevy::render::renderer::RenderAdapterInfo>().map(|v|format!("{:?}",v.0)),"files":base.files,"reports":reports}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn percentiles_do_not_drop_the_tail() {
        let s = stats(&[1., 2., 3., 100.]);
        assert_eq!(s["p95"], 100.);
        assert_eq!(s["mean"], 26.5);
        assert_eq!(s["median"], 2.5);
        assert!(stats(&[]).is_null());
    }
}
