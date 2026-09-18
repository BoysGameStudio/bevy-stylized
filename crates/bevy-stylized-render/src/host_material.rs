//! Opt-in shared materials with bounded frame data in their existing uniform.
//! Material assets remain immutable during frame updates. An input record binds
//! one view explicitly; hosts must validate visibility when using that mode.
use crate::{
    character::StylizedCharacterMaterial,
    hero::StylizedHeroMaterial,
    host_inputs::{
        HOST_INSTANCE_CAPACITY, HOST_VIEW_CAPACITY, HostInstanceRecord, HostViewRecord,
        pack_host_material_uniform,
    },
    material::StylizedCelMaterial,
    outline::StylizedOutlineMaterial,
};
use bevy::{
    asset::Asset,
    ecs::system::{StaticSystemParam, SystemParamItem},
    mesh::MeshTag,
    pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin},
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{
            AsBindGroup, AsBindGroupError, BindGroupLayout, BindGroupLayoutEntry, BindingType,
            Buffer, BufferBindingType, BufferInitDescriptor, BufferSize, BufferUsages,
            OwnedBindingResource, RenderPipelineDescriptor, SpecializedMeshPipelineError,
            UnpreparedBindGroup,
        },
        renderer::{RenderDevice, RenderQueue},
    },
    shader::ShaderRef,
};
use std::{
    collections::{BTreeMap, HashMap},
    hash::Hash,
    sync::{Arc, Weak},
};

#[derive(Resource, Clone, Default, ExtractResource)]
pub struct HostFrameInputs {
    pub version: u64,
    pub instances: Vec<HostInstanceRecord>,
    pub views: Vec<HostViewRecord>,
}
#[derive(Resource, Default, Clone, Debug)]
pub struct HostFrameStatus {
    pub ready: bool,
    pub error: Option<String>,
}
#[derive(Component, Clone)]
pub struct HostViewInput(pub HostViewRecord);
#[derive(Component, Clone)]
pub struct HostInstanceInput {
    pub view: Entity,
    pub record: HostInstanceRecord,
}

/// A hull using the same host-frame transport as its surface.
#[derive(Component, Clone)]
pub struct HostOutline(pub Handle<HostMaterial<StylizedOutlineMaterial>>);

fn record_key(record: &HostInstanceRecord) -> [u32; 24] {
    let mut result = [0; 24];
    for (i, lane) in [
        record.tint,
        record.emission,
        record.rim,
        record.face_forward,
        record.face_up,
    ]
    .iter()
    .enumerate()
    {
        result[i * 4..i * 4 + 4].copy_from_slice(&lane.to_array().map(f32::to_bits));
    }
    result[20..24].copy_from_slice(&record.indices.to_array());
    result
}

/// Validate the whole frame before publishing it or changing any instance tag.
/// Deduplication is by exact input values, not actor count. Rebuilt bounded
/// slots have no tombstone growth and are extracted together with this frame.
pub fn publish_host_inputs(
    mut commands: Commands,
    views: Query<(Entity, &HostViewInput)>,
    instances: Query<(Entity, &HostInstanceInput, Option<&MeshTag>)>,
    mut frame: ResMut<HostFrameInputs>,
    mut status: ResMut<HostFrameStatus>,
) {
    let mut ordered = views.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|(e, _)| e.to_bits());
    let mut view_indices = HashMap::new();
    let mut view_data = Vec::new();
    for (entity, input) in ordered {
        // Identical lighting records may be shared by arbitrarily many actors.
        // Entity identity establishes routing, not uniform-table occupancy.
        let slot = view_data
            .iter()
            .position(|record| record == &input.0)
            .unwrap_or_else(|| {
                view_data.push(input.0);
                view_data.len() - 1
            });
        view_indices.insert(entity, slot as u32);
    }
    if view_data.len() > HOST_VIEW_CAPACITY {
        *status = HostFrameStatus {
            ready: false,
            error: Some("distinct host view capacity exceeded".into()),
        };
        return;
    }
    let mut ordered = instances.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|(e, _, _)| e.to_bits());
    let mut slots = BTreeMap::new();
    let mut records = Vec::new();
    let mut tags = Vec::new();
    for (entity, input, tag) in ordered {
        let Some(&view) = view_indices.get(&input.view) else {
            *status = HostFrameStatus {
                ready: false,
                error: Some("host instance references a missing view".into()),
            };
            return;
        };
        let mut record = input.record;
        record.indices.y = view;
        let key = record_key(&record);
        let slot = *slots.entry(key).or_insert_with(|| {
            let id = records.len();
            records.push(record);
            id
        });
        if tag.is_none_or(|tag| tag.0 != slot as u32) {
            tags.push((entity, slot as u32));
        }
    }
    if records.len() > HOST_INSTANCE_CAPACITY {
        *status = HostFrameStatus {
            ready: false,
            error: Some("distinct host input capacity exceeded".into()),
        };
        return;
    }
    if let Err(error) = pack_host_material_uniform(&Vec4::ZERO, &records, &view_data) {
        *status = HostFrameStatus {
            ready: false,
            error: Some(format!("invalid host frame: {error:?}")),
        };
        return;
    }
    if frame.instances != records || frame.views != view_data {
        let Some(next) = frame.version.checked_add(1) else {
            *status = HostFrameStatus {
                ready: false,
                error: Some("host frame version exhausted".into()),
            };
            return;
        };
        *frame = HostFrameInputs {
            version: next,
            instances: records,
            views: view_data,
        };
    }
    for (entity, slot) in tags {
        commands.entity(entity).insert(MeshTag(slot));
    }
    *status = HostFrameStatus {
        ready: true,
        error: None,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HostAlphaMode {
    Inherit,
    Opaque,
    Blend,
}

pub trait HostMaterialSource: Material {
    fn frame_uniform(&self, frame: &HostFrameInputs) -> Result<Vec<u8>, String>;
    /// Binding containing this material's prefix and the shared frame tables.
    /// Extensions can retain their base material's binding zero unchanged.
    fn frame_uniform_binding() -> u32 {
        0
    }
    /// Normalize immutable shader flags when selecting a cached alpha class.
    fn set_host_alpha_mode(&mut self, _alpha: HostAlphaMode) {}
}
macro_rules! host_source {
    ($($ty:ty),*)=>{$(impl HostMaterialSource for $ty {
        fn frame_uniform(&self,frame:&HostFrameInputs)->Result<Vec<u8>,String> {
            pack_host_material_uniform(&self.params,&frame.instances,&frame.views).map_err(|e|format!("{e:?}"))
        }
    })*};
}
host_source!(
    StylizedCelMaterial,
    StylizedCharacterMaterial,
    StylizedHeroMaterial,
    StylizedOutlineMaterial
);

#[derive(Asset, TypePath, Clone)]
pub struct HostMaterial<M: HostMaterialSource> {
    #[dependency]
    base: M,
    alpha: HostAlphaMode,
    lease: Arc<()>,
    opaque_priority: i32,
}
impl<M: HostMaterialSource> HostMaterial<M> {
    pub fn new(mut base: M, alpha: HostAlphaMode) -> Self {
        base.set_host_alpha_mode(alpha);
        Self {
            base,
            alpha,
            lease: Arc::new(()),
            opaque_priority: 0,
        }
    }
    /// Stable opaque layer ordering, independent of GPU bind-group allocation.
    pub fn with_opaque_priority(mut self, priority: i32) -> Self {
        self.opaque_priority = priority;
        self
    }
    pub fn opaque_priority(&self) -> i32 {
        self.opaque_priority
    }
    pub fn base(&self) -> &M {
        &self.base
    }
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct HostMaterialKey<D> {
    base: D,
    alpha: HostAlphaMode,
}

struct GpuUniform {
    lease: Weak<()>,
    buffer: Buffer,
    offset: u64,
}
#[derive(Resource, Default)]
pub struct HostGpuUniforms {
    buffers: HashMap<usize, GpuUniform>,
    version: Option<u64>,
}

impl<M: HostMaterialSource> AsBindGroup for HostMaterial<M> {
    type Data = HostMaterialKey<M::Data>;
    type Param = (
        StaticSystemParam<'static, 'static, M::Param>,
        ResMut<'static, HostGpuUniforms>,
        Res<'static, HostFrameInputs>,
    );
    fn label() -> &'static str {
        "stylized-host-material"
    }
    fn bind_group_data(&self) -> Self::Data {
        HostMaterialKey {
            base: self.base.bind_group_data(),
            alpha: self.alpha,
        }
    }
    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        device: &RenderDevice,
        param: &mut SystemParamItem<'_, '_, Self::Param>,
        force_no_bindless: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        let mut bindings =
            self.base
                .unprepared_bind_group(layout, device, &mut param.0, force_no_bindless)?;
        let key = Arc::as_ptr(&self.lease) as usize;
        let bytes = self
            .base
            .frame_uniform(&param.2)
            .map_err(|_| AsBindGroupError::RetryNextUpdate)?;
        let entry = param.1.buffers.entry(key).or_insert_with(|| GpuUniform {
            lease: Arc::downgrade(&self.lease),
            offset: (bytes.len() - 14_848) as u64,
            buffer: device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("stylized-host-uniform"),
                contents: &bytes,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            }),
        });
        for (binding, resource) in bindings.bindings.iter_mut() {
            if *binding == M::frame_uniform_binding() {
                *resource = OwnedBindingResource::Buffer(entry.buffer.clone());
            }
        }
        Ok(bindings)
    }
    fn bind_group_layout_entries(
        device: &RenderDevice,
        force_no_bindless: bool,
    ) -> Vec<BindGroupLayoutEntry> {
        let mut entries = M::bind_group_layout_entries(device, force_no_bindless);
        for entry in &mut entries {
            if entry.binding == M::frame_uniform_binding()
                && let BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    min_binding_size,
                    ..
                } = &mut entry.ty
            {
                let prefix = min_binding_size
                    .expect("material prefix")
                    .get()
                    .div_ceil(16)
                    * 16;
                *min_binding_size = BufferSize::new(prefix + 14_848);
            }
        }
        entries
    }
}

impl<M: HostMaterialSource> Material for HostMaterial<M>
where
    M::Data: Clone + Eq + Hash,
{
    fn vertex_shader() -> ShaderRef {
        M::vertex_shader()
    }
    fn fragment_shader() -> ShaderRef {
        M::fragment_shader()
    }
    fn prepass_vertex_shader() -> ShaderRef {
        M::prepass_vertex_shader()
    }
    fn prepass_fragment_shader() -> ShaderRef {
        M::prepass_fragment_shader()
    }
    fn enable_prepass() -> bool {
        M::enable_prepass()
    }
    fn enable_shadows() -> bool {
        M::enable_shadows()
    }
    fn alpha_mode(&self) -> AlphaMode {
        match self.alpha {
            HostAlphaMode::Inherit => self.base.alpha_mode(),
            HostAlphaMode::Opaque => AlphaMode::Opaque,
            HostAlphaMode::Blend => AlphaMode::Blend,
        }
    }
    fn specialize(
        pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        M::specialize(
            pipeline,
            descriptor,
            layout,
            MaterialPipelineKey::<M> {
                mesh_key: key.mesh_key,
                bind_group_data: key.bind_group_data.base,
            },
        )?;
        descriptor.vertex.shader_defs.push("NPR_HOST_INPUTS".into());
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader_defs.push("NPR_HOST_INPUTS".into());
            if key.bind_group_data.alpha == HostAlphaMode::Blend {
                fragment.shader_defs.push("NPR_HOST_BLEND".into());
            }
        }
        Ok(())
    }
}

fn upload_host_uniforms(
    frame: Res<HostFrameInputs>,
    queue: Res<RenderQueue>,
    mut buffers: ResMut<HostGpuUniforms>,
) {
    buffers.buffers.retain(|_, b| b.lease.strong_count() > 0);
    if buffers.version == Some(frame.version) {
        return;
    }
    let Ok(bytes) = pack_host_material_uniform(&Vec4::ZERO, &frame.instances, &frame.views) else {
        return;
    };
    for entry in buffers.buffers.values() {
        queue.write_buffer(&entry.buffer, entry.offset, &bytes[16..]);
    }
    buffers.version = Some(frame.version);
}

/// Register the opt-in material types and extraction; the host explicitly
/// schedules `publish_host_inputs` at its final-pose boundary.
pub struct HostMaterialPlugin;
impl Plugin for HostMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<crate::outline_order::MaterialPriorities>()
            .add_systems(Last, collect_priorities);
        app.init_resource::<HostFrameInputs>()
            .init_resource::<HostFrameStatus>()
            .add_plugins(ExtractResourcePlugin::<HostFrameInputs>::default())
            .add_plugins((
                MaterialPlugin::<HostMaterial<StylizedCelMaterial>>::default(),
                MaterialPlugin::<HostMaterial<StylizedCharacterMaterial>>::default(),
                MaterialPlugin::<HostMaterial<StylizedHeroMaterial>>::default(),
                MaterialPlugin::<HostMaterial<StylizedOutlineMaterial>>::default(),
            ));
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render.init_resource::<HostGpuUniforms>().add_systems(
                Render,
                upload_host_uniforms.in_set(RenderSystems::PrepareResources),
            );
        }
    }
}

#[cfg(test)]
mod frame_tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<HostFrameInputs>()
            .init_resource::<HostFrameStatus>()
            .add_systems(Update, publish_host_inputs);
        app
    }
    #[test]
    fn shared_inputs_do_not_consume_per_actor_slots_and_despawn_reclaims_slots() {
        let mut app = app();
        let mut actors = Vec::new();
        for _ in 0..256 {
            let view = app
                .world_mut()
                .spawn(HostViewInput(HostViewRecord::default()))
                .id();
            let actor = app
                .world_mut()
                .spawn(HostInstanceInput {
                    view,
                    record: HostInstanceRecord::default(),
                })
                .id();
            actors.push((actor, view));
        }
        app.update();
        assert!(app.world().resource::<HostFrameStatus>().ready);
        let frame = app.world().resource::<HostFrameInputs>();
        assert_eq!((frame.instances.len(), frame.views.len()), (1, 1));
        let version = frame.version;
        app.update();
        assert_eq!(app.world().resource::<HostFrameInputs>().version, version);
        for (actor, view) in actors {
            app.world_mut().despawn(actor);
            app.world_mut().despawn(view);
        }
        app.update();
        let frame = app.world().resource::<HostFrameInputs>();
        assert!(frame.instances.is_empty() && frame.views.is_empty());
    }
    #[test]
    fn invalid_frame_preserves_published_data_and_tags_then_recovers() {
        let mut app = app();
        let view = app
            .world_mut()
            .spawn(HostViewInput(HostViewRecord::default()))
            .id();
        let actor = app
            .world_mut()
            .spawn(HostInstanceInput {
                view,
                record: HostInstanceRecord::default(),
            })
            .id();
        app.update();
        let before = app.world().resource::<HostFrameInputs>().clone();
        let tag = app.world().get::<MeshTag>(actor).unwrap().0;
        app.world_mut()
            .get_mut::<HostInstanceInput>(actor)
            .unwrap()
            .record
            .tint
            .x = f32::NAN;
        app.update();
        assert!(!app.world().resource::<HostFrameStatus>().ready);
        let frame = app.world().resource::<HostFrameInputs>();
        assert_eq!(frame.version, before.version);
        assert_eq!(frame.instances, before.instances);
        assert_eq!(app.world().get::<MeshTag>(actor).unwrap().0, tag);
        app.world_mut()
            .get_mut::<HostInstanceInput>(actor)
            .unwrap()
            .record
            .tint
            .x = 0.25;
        app.update();
        assert!(app.world().resource::<HostFrameStatus>().ready);
        assert!(app.world().resource::<HostFrameInputs>().version > before.version);
        app.world_mut().despawn(view);
        app.update();
        assert!(
            app.world()
                .resource::<HostFrameStatus>()
                .error
                .as_ref()
                .unwrap()
                .contains("missing view")
        );
    }
    #[test]
    fn distinct_view_capacity_is_rejected_without_partial_publication() {
        let mut app = app();
        for index in 0..=HOST_VIEW_CAPACITY {
            app.world_mut().spawn(HostViewInput(HostViewRecord {
                ambient: Vec4::splat(index as f32),
                ..default()
            }));
        }
        app.update();
        assert!(!app.world().resource::<HostFrameStatus>().ready);
        assert_eq!(app.world().resource::<HostFrameInputs>().version, 0);
    }
}

fn collect_priorities(
    cel: Res<Assets<HostMaterial<StylizedCelMaterial>>>,
    character: Res<Assets<HostMaterial<StylizedCharacterMaterial>>>,
    hero: Res<Assets<HostMaterial<StylizedHeroMaterial>>>,
    outline: Res<Assets<HostMaterial<StylizedOutlineMaterial>>>,
    physical: Option<Res<Assets<HostMaterial<crate::physical::PhysicalCelMaterial>>>>,
    mut priorities: ResMut<crate::outline_order::MaterialPriorities>,
) {
    if !cel.is_changed()
        && !character.is_changed()
        && !hero.is_changed()
        && !outline.is_changed()
        && physical
            .as_ref()
            .is_none_or(|materials| !materials.is_changed())
    {
        return;
    }
    priorities.0 = cel
        .iter()
        .map(|(id, m)| (id.untyped(), m.opaque_priority()))
        .chain(
            character
                .iter()
                .map(|(id, m)| (id.untyped(), m.opaque_priority())),
        )
        .chain(
            hero.iter()
                .map(|(id, m)| (id.untyped(), m.opaque_priority())),
        )
        .chain(
            outline
                .iter()
                .map(|(id, m)| (id.untyped(), m.opaque_priority())),
        )
        .filter(|(_, p)| *p != 0)
        .collect();
    if let Some(materials) = physical {
        priorities.0.extend(materials.iter().filter_map(|(id, m)| {
            (m.opaque_priority() != 0).then_some((id.untyped(), m.opaque_priority()))
        }));
    }
}
