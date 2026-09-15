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
fn report(cache: Res<PipelineCache>, probe: Res<Probe>) {
    let result = pipeline_status(&cache);
    let mut state = probe.0.lock().expect("readiness lock");
    state.frame += 1;
    state.ready = result.as_ref().copied().unwrap_or(false);
    state.error = result.err();
}
