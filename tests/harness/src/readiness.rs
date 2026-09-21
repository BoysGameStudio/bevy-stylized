//! Shared pipeline readiness for captures and interactive inspection.
use bevy::{
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        render_resource::{CachedPipelineState, PipelineCache},
    },
};
use std::sync::{Arc, Mutex};

pub fn pipeline_status(cache: &PipelineCache) -> Result<bool, String> {
    let mut count = 0;
    for p in cache.pipelines() {
        count += 1;
        if let CachedPipelineState::Err(ref e) = p.state
            && !matches!(
                e,
                bevy::shader::ShaderCacheError::ShaderNotLoaded(_)
                    | bevy::shader::ShaderCacheError::ShaderImportNotYetAvailable
            )
        {
            return Err(format!("shader pipeline failed: {e:?}"));
        }
    }
    Ok(count > 0
        && cache.waiting_pipelines().next().is_none()
        && cache
            .pipelines()
            .all(|p| matches!(p.state, CachedPipelineState::Ok(_))))
}
#[derive(Clone, Default)]
pub struct Snapshot {
    pub frame: u64,
    pub ready: bool,
    pub error: Option<String>,
}
#[derive(Resource, Clone, Default)]
pub struct Probe(Arc<Mutex<Snapshot>>);
impl Probe {
    pub fn snapshot(&self) -> Snapshot {
        self.0.lock().expect("readiness lock").clone()
    }
}
pub fn install(app: &mut App) {
    let probe = Probe::default();
    app.sub_app_mut(RenderApp)
        .insert_resource(probe.clone())
        .add_systems(Render, report.in_set(RenderSystems::Cleanup));
    app.insert_resource(probe);
}

/// Fixed update counts can capture before a newly imported shader is prepared.
/// Wait for consecutive completed render frames, with bounded error reporting.
pub fn wait(app: &mut App, frames: u32) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut stable = 0;
    let mut last = 0;
    while stable < frames {
        assert!(
            std::time::Instant::now() < deadline,
            "pipeline readiness timeout"
        );
        app.update();
        let state = app.world().resource::<Probe>().snapshot();
        assert!(state.error.is_none(), "pipeline error: {:?}", state.error);
        if state.frame != last {
            last = state.frame;
            stable = if state.ready { stable + 1 } else { 0 };
        }
    }
}
fn report(cache: Res<PipelineCache>, probe: Res<Probe>) {
    let result = pipeline_status(&cache);
    let mut state = probe.0.lock().expect("readiness lock");
    state.frame += 1;
    state.ready = result.as_ref().copied().unwrap_or(false);
    state.error = result.err();
}
