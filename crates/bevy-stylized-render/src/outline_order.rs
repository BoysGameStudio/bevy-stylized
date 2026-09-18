//! Explicit ordering for opaque hulls that write depth on touching surfaces.
use bevy::{
    core_pipeline::core_3d::Opaque3d,
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_phase::ViewBinnedRenderPhases,
        render_resource::{PipelineCache, PipelineDescriptor},
    },
};
use std::collections::{HashMap, HashSet};

#[derive(Resource, Clone, Default, ExtractResource)]
pub(crate) struct MaterialPriorities(pub Vec<(bevy::asset::UntypedAssetId, i32)>);

/// Opt-in composition policy. Other opaque batches retain their relative order.
#[derive(Resource, Clone, Copy, Debug, Default, ExtractResource)]
pub enum OutlineOrder {
    #[default]
    Engine,
    BeforeSurfaces,
    AfterSurfaces,
}

pub(crate) fn register(app: &mut App) {
    app.init_resource::<OutlineOrder>()
        .init_resource::<MaterialPriorities>()
        .add_plugins((
            ExtractResourcePlugin::<OutlineOrder>::default(),
            ExtractResourcePlugin::<MaterialPriorities>::default(),
        ));
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render.add_systems(
            Render,
            order_hulls
                .in_set(RenderSystems::PhaseSort)
                .after(bevy::render::batching::sort_binned_render_phase::<Opaque3d>),
        );
    }
}
fn order_hulls(
    order: Res<OutlineOrder>,
    cache: Res<PipelineCache>,
    phases: Option<ResMut<ViewBinnedRenderPhases<Opaque3d>>>,
    priorities: Res<MaterialPriorities>,
    materials: Res<
        bevy::render::erased_render_asset::ErasedRenderAssets<bevy::pbr::PreparedMaterial>,
    >,
) {
    if matches!(*order, OutlineOrder::Engine) && priorities.0.is_empty() {
        return;
    }
    let Some(mut phases) = phases else {
        return;
    };
    // Only processed descriptors are visible here; newly queued pipelines are
    // not yet drawable. Never index an unprocessed pipeline descriptor.
    let hulls = cache
        .pipelines()
        .enumerate()
        .filter_map(|(index, p)| match &p.descriptor {
            PipelineDescriptor::RenderPipelineDescriptor(d)
                if d.label.as_deref() == Some("stylized opaque hull") =>
            {
                Some(index)
            }
            _ => None,
        })
        .collect::<HashSet<_>>();
    let mut groups = HashMap::new();
    for &(asset, priority) in &priorities.0 {
        if let Some(m) = materials.get(asset)
            && let Some(shader) = m.properties.get_shader(bevy::pbr::MaterialFragmentShader)
        {
            groups.insert((m.binding.group.0, shader.id()), priority);
        }
    }
    let descriptors = cache.pipelines().collect::<Vec<_>>();
    let rank = |key: &bevy::core_pipeline::core_3d::Opaque3dBatchSetKey| {
        let id = key.pipeline.id();
        let layer = if matches!(*order, OutlineOrder::Engine) {
            0
        } else {
            u8::from(hulls.contains(&id) == matches!(*order, OutlineOrder::AfterSurfaces))
        };
        let priority = descriptors
            .get(id)
            .and_then(|p| match &p.descriptor {
                PipelineDescriptor::RenderPipelineDescriptor(d)
                    if d.vertex.shader_defs.iter().any(|def|matches!(def,bevy::shader::ShaderDefVal::Bool(name,true) if name=="NPR_HOST_INPUTS")) => d
                    .fragment
                    .as_ref()
                    .and_then(|f| {
                        key.material_bind_group_index
                            .and_then(|g| groups.get(&(g, f.shader.id())))
                    })
                    .copied(),
                _ => None,
            })
            .unwrap_or(0);
        (layer, priority)
    };
    for phase in phases.0.values_mut() {
        // Sort before batching creates its matching GPU batch/index buffers.
        phase
            .multidrawable_meshes
            .sort_by(|a, _, b, _| rank(a).cmp(&rank(b)));
        phase
            .batchable_meshes
            .sort_by(|a, _, b, _| rank(&a.0).cmp(&rank(&b.0)));
        phase
            .unbatchable_meshes
            .sort_by(|a, _, b, _| rank(&a.0).cmp(&rank(&b.0)));
    }
}
