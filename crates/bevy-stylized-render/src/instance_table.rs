//! GPU Instance Database（V2-e；ADR-0004 Accepted）：
//! StorageTable 传输通道。
//!
//! 形态（ADR-0004 的"MeshTag 索引表"方案，spike 证据见
//! `target/tmp/adr-0004-spike/`）：
//!
//! ```text
//! StylizedInstanceTableEntry（core 全精度记录，组件）
//!   → sync_instance_table（PostUpdate）分配槽位 + 写 ShaderBuffer
//!   → sync_instance_tags 把 MeshTag 打包为 24-bit 槽位索引
//!   → shader（INSTANCE_STORAGE_TABLE def）按索引读全精度记录
//! ```
//!
//! - **能力门控**：仅 Standard+（tier 需求表已含 storage buffer 特性；
//!   选择逻辑见 [`crate::instance::select_transport`]）。Portable 永不
//!   触碰本通道——三个主材质类型的 layout 保持零 storage binding
//!   （`tests/instance_table.rs` 钉死），表通道由独立的
//!   [`StylizedCharacterTableMaterial`] 承载；
//! - **保真契约**：见 core [`StylizedInstanceGpuRecord`] 文档
//!   （MeshTag = 粗粒度子集；表 = 全字段全精度）；
//! - **数据资产纪律**：表是 [`ShaderBuffer`] 数据资产（不是材质资产），
//!   仅在记录变更/槽位增长时整体重写，无 per-frame 写；
//! - Slots are dense within a published snapshot. Membership changes rebuild the
//!   snapshot and tags together; every membership change uses a fresh buffer asset and rebinds
//!   dependent materials before extraction. In-flight draws retain their old buffer.

use crate::{character::StylizedCharacterTableMaterial, instance::StylizedInstanceData};
use bevy::{
    asset::{Assets, RenderAssetUsages},
    mesh::MeshTag,
    prelude::{
        Commands, Component, Entity, Handle, MeshMaterial3d, Or, PostUpdate, Query, Res, ResMut,
        Resource, With, Without,
    },
    render::{renderer::RenderDevice, storage::ShaderBuffer},
};
use bevy_stylized_core::StylizedInstanceGpuRecord;

use crate::instance::StylizedInstanceSlot;

/// 单条实例记录的 GPU 字节步长（3×vec4 = 48B；与
/// `character_forward.wgsl` 的 `StylizedInstanceRecord` 布局一致）。
pub const INSTANCE_RECORD_STRIDE: usize = 48;

/// 表初始容量（槽位数；不足时倍增重建）。
pub const INSTANCE_TABLE_INITIAL_CAPACITY: u32 = 64;

/// 打包为 GPU 记录字节（host 镜像；布局注释见 WGSL 侧）。
#[must_use]
pub fn pack_instance_record(record: &StylizedInstanceGpuRecord) -> [u8; INSTANCE_RECORD_STRIDE] {
    let mut out = [0u8; INSTANCE_RECORD_STRIDE];
    let f32s = [
        record.face_threshold,
        record.outline_scale,
        0.0,
        0.0,
        record.light_tint.red,
        record.light_tint.green,
        record.light_tint.blue,
        record.light_tint.alpha,
    ];
    let u32s = [
        record.face_side,
        record.profile_index,
        record.palette_index,
        record.quality_flags,
    ];
    // scalars(16B) | indices(16B) | light_tint(16B)
    for (i, v) in f32s[..4].iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    for (i, v) in u32s.iter().enumerate() {
        out[16 + i * 4..16 + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    for (i, v) in f32s[4..].iter().enumerate() {
        out[32 + i * 4..32 + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    out
}

/// 解包 GPU 记录字节（`pack_instance_record` 的逆；测试用）。
#[must_use]
pub fn unpack_instance_record(bytes: &[u8; INSTANCE_RECORD_STRIDE]) -> StylizedInstanceGpuRecord {
    let f = |i: usize| f32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().expect("4 bytes"));
    let u = |i: usize| {
        u32::from_le_bytes(
            bytes[16 + i * 4..16 + i * 4 + 4]
                .try_into()
                .expect("4 bytes"),
        )
    };
    StylizedInstanceGpuRecord {
        face_threshold: f(0),
        outline_scale: f(1),
        face_side: u(0),
        profile_index: u(1),
        palette_index: u(2),
        quality_flags: u(3),
        light_tint: bevy::color::LinearRgba::new(f(8), f(9), f(10), f(11)),
        ..StylizedInstanceGpuRecord::default()
    }
}

/// StorageTable 的实例记录组件（携带即声明该实体走表通道）。
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct StylizedInstanceTableEntry(pub StylizedInstanceGpuRecord);

/// CPU and GPU budget for the provisional table transport. May be lowered by
/// the host; actual RenderDevice limits and the 24-bit tag are always enforced.
#[derive(Resource, Clone, Copy, Debug)]
pub struct InstanceTableBudget {
    pub max_bytes: u64,
}
impl Default for InstanceTableBudget {
    fn default() -> Self {
        Self {
            max_bytes: 16 * 1024 * 1024,
        }
    }
}
impl InstanceTableBudget {
    fn slots(self, device: Option<&RenderDevice>) -> u32 {
        let bytes = device.map_or(self.max_bytes, |device| {
            let limits = device.limits();
            if limits.max_storage_buffers_per_shader_stage == 0 {
                return 0;
            }
            self.max_bytes
                .min(limits.max_buffer_size)
                .min(limits.max_storage_buffer_binding_size)
        });
        (bytes / INSTANCE_RECORD_STRIDE as u64)
            .min(u64::from(crate::instance::TAG_TABLE_INDEX_MASK) + 1) as u32
    }
}

/// Admission failed; the table draw binding is suspended until capacity returns.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstanceTableCapacityExceeded {
    pub max_slots: u32,
}

#[derive(Component)]
pub struct SuspendedTableMaterial(Handle<StylizedCharacterTableMaterial>);

/// A complete table snapshot. Slot numbers are transient, not persistent handles.
#[derive(Resource)]
pub struct StylizedInstanceTable {
    pub buffer: Handle<ShaderBuffer>,
    pub capacity: u32,
    /// Live record count (and the next dense slot), not historical allocations.
    pub next_slot: u32,
    records: Vec<(Entity, StylizedInstanceGpuRecord)>,
}
impl StylizedInstanceTable {
    /// Seed a table with an existing buffer. The next upload replaces it and
    /// rebinds all table materials that use it; callers must not cache slot IDs.
    pub fn new(buffer: Handle<ShaderBuffer>) -> Self {
        Self {
            buffer,
            capacity: 0,
            next_slot: 0,
            records: Vec::new(),
        }
    }
}

/// Publish records, tags and material dependencies before asset extraction.
/// Fresh buffer identity on layout changes prevents a delayed upload from exposing new indices
/// through an old prepared material. Bevy retains resources used by queued draws.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn sync_instance_table(
    mut commands: Commands,
    mut table: Option<ResMut<StylizedInstanceTable>>,
    mut buffers: Option<ResMut<Assets<ShaderBuffer>>>,
    mut materials: Option<ResMut<Assets<StylizedCharacterTableMaterial>>>,
    device: Option<Res<RenderDevice>>,
    budget: Option<Res<InstanceTableBudget>>,
    entries: Query<(
        Entity,
        &StylizedInstanceTableEntry,
        Option<&StylizedInstanceSlot>,
        Option<&MeshTag>,
        Option<&StylizedInstanceData>,
        Option<&MeshMaterial3d<StylizedCharacterTableMaterial>>,
        Option<&SuspendedTableMaterial>,
    )>,
    orphans: Query<
        (
            Entity,
            Option<&MeshMaterial3d<StylizedCharacterTableMaterial>>,
        ),
        (
            Or<(
                With<StylizedInstanceSlot>,
                With<InstanceTableCapacityExceeded>,
            )>,
            Without<StylizedInstanceTableEntry>,
        ),
    >,
) {
    let Some(buffers) = buffers.as_deref_mut() else {
        return;
    };
    // Keep the public lazy resource available to material setup, but do not
    // allocate a storage asset until an entity explicitly requests this channel.
    if entries.is_empty()
        && orphans.is_empty()
        && table
            .as_deref()
            .is_none_or(|table| table.records.is_empty() && buffers.get(&table.buffer).is_none())
    {
        if table.is_none() {
            commands.insert_resource(StylizedInstanceTable::new(Handle::default()));
        }
        return;
    }
    let max_slots = budget
        .as_deref()
        .copied()
        .unwrap_or_default()
        .slots(device.as_deref());
    // Removing an entry revokes the draw binding as well as its index. Re-adding
    // the entry can restore the saved material after admission into a new snapshot.
    for (entity, material) in &orphans {
        let mut entity = commands.entity(entity);
        entity.remove::<(
            StylizedInstanceSlot,
            InstanceTableCapacityExceeded,
            MeshTag,
            MeshMaterial3d<StylizedCharacterTableMaterial>,
        )>();
        if let Some(material) = material {
            entity.insert(SuspendedTableMaterial(material.0.clone()));
        }
    }
    let mut ordered = entries.iter().collect::<Vec<_>>();
    ordered.sort_unstable_by_key(|entry| entry.0.to_bits());
    let mut records = Vec::with_capacity(ordered.len().min(max_slots as usize));
    for (index, (entity, entry, slot, tag, data, material, suspended)) in
        ordered.into_iter().enumerate()
    {
        let mut entity_commands = commands.entity(entity);
        if index >= max_slots as usize {
            entity_commands
                .remove::<(
                    StylizedInstanceSlot,
                    MeshTag,
                    MeshMaterial3d<StylizedCharacterTableMaterial>,
                )>()
                .insert(InstanceTableCapacityExceeded { max_slots });
            if let Some(material) = material {
                entity_commands.insert(SuspendedTableMaterial(material.0.clone()));
            }
            continue;
        }
        let index = index as u32;
        records.push((entity, entry.0));
        entity_commands.remove::<InstanceTableCapacityExceeded>();
        if slot.is_none_or(|slot| slot.0 != index) {
            entity_commands.insert(StylizedInstanceSlot(index));
        }
        let packed = StylizedInstanceData::index_tag(index, data.map_or(0, |data| data.flags));
        if tag.is_none_or(|tag| tag.0 != packed.0) {
            entity_commands.insert(packed);
        }
        if material.is_none()
            && let Some(suspended) = suspended
        {
            entity_commands.insert(MeshMaterial3d(suspended.0.clone()));
        }
        entity_commands.remove::<SuspendedTableMaterial>();
    }
    if max_slots == 0 {
        if let Some(table) = table.as_deref() {
            if let Some(materials) = materials.as_deref_mut() {
                for (_, material) in materials.iter_mut() {
                    if material.instance_table == table.buffer {
                        material.instance_table = Handle::default();
                    }
                }
            }
            commands.remove_resource::<StylizedInstanceTable>();
        }
        return;
    }
    let initializing = table.is_none();
    let mut initial = StylizedInstanceTable::new(Handle::default());
    let table = table.as_deref_mut().unwrap_or(&mut initial);
    let capacity = (records.len().max(1) as u32)
        .max(INSTANCE_TABLE_INITIAL_CAPACITY.min(max_slots))
        .next_power_of_two()
        .min(max_slots);
    if table.records == records
        && table.capacity == capacity
        && buffers.get(&table.buffer).is_some()
    {
        return;
    }
    let mut bytes = vec![0u8; capacity as usize * INSTANCE_RECORD_STRIDE];
    for (slot, (_, record)) in records.iter().enumerate() {
        let offset = slot * INSTANCE_RECORD_STRIDE;
        bytes[offset..offset + INSTANCE_RECORD_STRIDE]
            .copy_from_slice(&pack_instance_record(record));
    }
    let same_layout = table.capacity == capacity
        && table
            .records
            .iter()
            .map(|record| record.0)
            .eq(records.iter().map(|record| record.0));
    if same_layout && let Some(mut buffer) = buffers.get_mut(&table.buffer) {
        // Every index still names the same entity. A delayed numeric upload can
        // retain an older value, but cannot expose a recycled entity's record.
        buffer.data = Some(bytes);
        table.records = records;
        return;
    }
    let mut snapshot = ShaderBuffer::with_size(bytes.len(), RenderAssetUsages::default());
    snapshot.data = Some(bytes);
    let buffer = buffers.add(snapshot);
    if let Some(materials) = materials.as_deref_mut() {
        for (_, material) in materials.iter_mut() {
            if material.instance_table == table.buffer {
                material.instance_table = buffer.clone();
            }
        }
    }
    table.buffer = buffer;
    table.capacity = capacity;
    table.next_slot = records.len() as u32;
    table.records = records;
    if initializing {
        commands.insert_resource(initial);
    }
}

/// 注册表通道（资源懒初始化 + PostUpdate 系统，先于 instance tag 同步）。
pub(crate) fn register(app: &mut bevy::app::App) {
    use bevy::prelude::IntoScheduleConfigs;
    // Material preparation must see this frame's newly uploaded buffer, including
    // continuous updates. The barrier orders existing Bevy systems without adding
    // a second copy of either preparation system.
    if let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
        render_app.add_systems(
            bevy::render::Render,
            table_upload_barrier
                .after(
                    bevy::render::render_asset::prepare_assets::<
                        bevy::render::storage::GpuShaderBuffer,
                    >,
                )
                .before(
                    bevy::render::erased_render_asset::prepare_erased_assets::<
                        MeshMaterial3d<StylizedCharacterTableMaterial>,
                    >,
                ),
        );
    }
    app.add_systems(
        PostUpdate,
        sync_instance_table
            .before(bevy::asset::AssetEventSystems)
            .before(crate::instance::sync_instance_tags)
            .in_set(crate::presentation::NativePresentationSystems),
    );
}

fn table_upload_barrier() {}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    fn table_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::asset::AssetPlugin::default())
            .init_asset::<ShaderBuffer>()
            .init_asset::<StylizedCharacterTableMaterial>();
        app.add_systems(
            PostUpdate,
            sync_instance_table.before(crate::instance::sync_instance_tags),
        );
        app.add_systems(PostUpdate, crate::instance::sync_instance_tags);
        app
    }

    #[test]
    fn record_pack_roundtrip_full_precision() {
        let record = StylizedInstanceGpuRecord {
            face_threshold: 0.5001, // 1/254 量化网格外的值
            outline_scale: 1.37,
            face_side: 1,
            profile_index: 300, // 超出 MeshTag u8
            palette_index: 5,
            quality_flags: 0x0102_0304,
            light_tint: bevy::color::LinearRgba::new(0.9, 0.95, 1.1, 0.8),
            ..StylizedInstanceGpuRecord::default()
        };
        let bytes = pack_instance_record(&record);
        let back = unpack_instance_record(&bytes);
        assert_eq!(back.face_threshold, 0.5001);
        assert_eq!(back.profile_index, 300);
        assert_eq!(back.palette_index, 5);
        assert_eq!(back.quality_flags, 0x0102_0304);
        assert!((back.light_tint.blue - 1.1).abs() < 1e-6);

        // 对照：MeshTag 量化丢失的信息（两档保真契约的可执行钉死）。
        let tag = crate::instance::StylizedInstanceData {
            face_threshold: Some(0.5001),
            profile_index: 300,
            ..crate::instance::StylizedInstanceData::default()
        }
        .to_tag();
        let back_tag = crate::instance::StylizedInstanceData::from_tag(tag);
        assert!((back_tag.face_threshold.unwrap() - 0.5001).abs() > 1e-4);
        assert_eq!(back_tag.profile_index, 255, "u8 钳制（300 → 255）");
    }

    #[test]
    fn slot_assignment_and_buffer_upload() {
        let mut app = table_app();
        let mut entities: Vec<Entity> = (0..3)
            .map(|i| {
                app.world_mut()
                    .spawn((
                        StylizedInstanceTableEntry(StylizedInstanceGpuRecord {
                            face_threshold: 0.1 * (i + 1) as f32,
                            ..StylizedInstanceGpuRecord::default()
                        }),
                        crate::instance::StylizedInstanceData::default(),
                    ))
                    .id()
            })
            .collect();
        app.update(); // 懒初始化帧
        app.update(); // 槽位分配 + 上传

        entities.sort_unstable_by_key(|entity| entity.to_bits());
        let table = app.world().resource::<StylizedInstanceTable>();
        assert_eq!(table.next_slot, 3);
        for (i, entity) in entities.iter().enumerate() {
            let slot = app
                .world()
                .entity(*entity)
                .get::<StylizedInstanceSlot>()
                .expect("slot assigned");
            assert_eq!(slot.0, i as u32);
            // tag 打包为索引布局（bits 0..=23 = 槽位）。
            let tag = app
                .world()
                .entity(*entity)
                .get::<bevy::mesh::MeshTag>()
                .expect("tag written");
            assert_eq!(tag.0 & crate::instance::TAG_TABLE_INDEX_MASK, i as u32);
        }

        // buffer 内容：slot 1 的 face_threshold 应为 0.2。
        let table = app.world().resource::<StylizedInstanceTable>();
        let buffers = app.world().resource::<Assets<ShaderBuffer>>();
        let data = buffers
            .get(&table.buffer)
            .and_then(|b| b.data.as_ref())
            .expect("buffer data");
        let mut record_bytes = [0u8; INSTANCE_RECORD_STRIDE];
        record_bytes.copy_from_slice(&data[INSTANCE_RECORD_STRIDE..INSTANCE_RECORD_STRIDE * 2]);
        let record = unpack_instance_record(&record_bytes);
        assert_eq!(
            record.face_threshold,
            app.world()
                .entity(entities[1])
                .get::<StylizedInstanceTableEntry>()
                .unwrap()
                .0
                .face_threshold
        );
    }

    #[test]
    fn no_change_no_rewrite() {
        let mut app = table_app();
        app.world_mut().spawn((
            StylizedInstanceTableEntry(StylizedInstanceGpuRecord::default()),
            crate::instance::StylizedInstanceData::default(),
        ));
        app.update();
        app.update();
        let original_buffer = app
            .world()
            .resource::<StylizedInstanceTable>()
            .buffer
            .clone();
        let generation = {
            let table = app.world().resource::<StylizedInstanceTable>();
            let buffers = app.world().resource::<Assets<ShaderBuffer>>();
            let buffer = buffers.get(&table.buffer).expect("buffer");
            buffer.data.clone()
        };
        app.update();
        app.update();
        let table = app.world().resource::<StylizedInstanceTable>();
        let buffers = app.world().resource::<Assets<ShaderBuffer>>();
        let buffer = buffers.get(&table.buffer).expect("buffer");
        assert_eq!(buffer.data, generation, "无变更时不得重写 buffer");
        assert_eq!(table.buffer, original_buffer);
    }
    #[test]
    fn churn_is_bounded_by_live_instances() {
        let mut app = table_app();
        for _ in 0..160 {
            let entity = app
                .world_mut()
                .spawn((
                    StylizedInstanceTableEntry(StylizedInstanceGpuRecord::default()),
                    crate::instance::StylizedInstanceData::default(),
                ))
                .id();
            app.update();
            app.update();
            let table = app.world().resource::<StylizedInstanceTable>();
            assert!(table.capacity <= INSTANCE_TABLE_INITIAL_CAPACITY);
            assert_eq!(table.next_slot, 1);
            app.world_mut().despawn(entity);
        }
    }
    #[test]
    fn removal_rebind_and_budget_rejection_revoke_the_draw_binding() {
        let mut app = table_app();
        app.insert_resource(InstanceTableBudget {
            max_bytes: INSTANCE_RECORD_STRIDE as u64,
        });
        app.update();
        let buffer = app
            .world()
            .resource::<StylizedInstanceTable>()
            .buffer
            .clone();
        let material = app
            .world_mut()
            .resource_mut::<Assets<StylizedCharacterTableMaterial>>()
            .add(StylizedCharacterTableMaterial {
                instance_table: buffer,
                ..Default::default()
            });
        let a = app
            .world_mut()
            .spawn((
                StylizedInstanceTableEntry(Default::default()),
                MeshMaterial3d(material.clone()),
                StylizedInstanceData::default(),
            ))
            .id();
        app.update();
        app.world_mut()
            .entity_mut(a)
            .remove::<StylizedInstanceTableEntry>();
        app.update();
        assert!(
            app.world()
                .entity(a)
                .get::<StylizedInstanceSlot>()
                .is_none()
        );
        assert!(
            app.world()
                .entity(a)
                .get::<MeshMaterial3d<StylizedCharacterTableMaterial>>()
                .is_none()
        );
        app.world_mut()
            .entity_mut(a)
            .insert(StylizedInstanceTableEntry(Default::default()));
        app.update();
        assert!(
            app.world()
                .entity(a)
                .get::<MeshMaterial3d<StylizedCharacterTableMaterial>>()
                .is_some()
        );
        let b = app
            .world_mut()
            .spawn((
                StylizedInstanceTableEntry(Default::default()),
                MeshMaterial3d(material),
                StylizedInstanceData::default(),
            ))
            .id();
        app.update();
        let rejected = [a, b]
            .into_iter()
            .find(|entity| {
                app.world()
                    .entity(*entity)
                    .contains::<InstanceTableCapacityExceeded>()
            })
            .unwrap();
        assert!(
            !app.world()
                .entity(rejected)
                .contains::<StylizedInstanceSlot>()
        );
        assert!(
            !app.world()
                .entity(rejected)
                .contains::<MeshMaterial3d<StylizedCharacterTableMaterial>>()
        );
        app.insert_resource(InstanceTableBudget {
            max_bytes: 2 * INSTANCE_RECORD_STRIDE as u64,
        });
        app.update();
        assert!(
            !app.world()
                .entity(rejected)
                .contains::<InstanceTableCapacityExceeded>()
        );
        assert!(
            app.world()
                .entity(rejected)
                .contains::<MeshMaterial3d<StylizedCharacterTableMaterial>>()
        );
        assert_eq!(app.world().resource::<StylizedInstanceTable>().capacity, 2);
        app.insert_resource(InstanceTableBudget { max_bytes: 0 });
        app.update();
        assert!(!app.world().contains_resource::<StylizedInstanceTable>());
        assert!(!app.world().entity(a).contains::<StylizedInstanceSlot>());
        assert!(
            !app.world()
                .entity(b)
                .contains::<MeshMaterial3d<StylizedCharacterTableMaterial>>()
        );
    }

    #[test]
    fn growth_publishes_a_new_buffer_and_rebinds_existing_materials() {
        let mut app = table_app();
        app.update();
        let old = app
            .world()
            .resource::<StylizedInstanceTable>()
            .buffer
            .clone();
        let material = app
            .world_mut()
            .resource_mut::<Assets<StylizedCharacterTableMaterial>>()
            .add(StylizedCharacterTableMaterial {
                instance_table: old.clone(),
                ..Default::default()
            });
        for _ in 0..65 {
            app.world_mut()
                .spawn(StylizedInstanceTableEntry(Default::default()));
        }
        app.update();
        let table = app.world().resource::<StylizedInstanceTable>();
        assert_eq!(table.capacity, 128);
        assert_ne!(table.buffer, old);
        assert_eq!(
            app.world()
                .resource::<Assets<StylizedCharacterTableMaterial>>()
                .get(&material)
                .unwrap()
                .instance_table,
            table.buffer
        );
        assert_eq!(
            InstanceTableBudget {
                max_bytes: u64::MAX
            }
            .slots(None),
            1 << 24
        );
        assert_eq!(InstanceTableBudget { max_bytes: 47 }.slots(None), 0);
    }
    #[test]
    fn numeric_updates_preserve_the_buffer_and_material_binding() {
        let mut app = table_app();
        let entity = app
            .world_mut()
            .spawn(StylizedInstanceTableEntry(Default::default()))
            .id();
        app.update();
        let buffer = app
            .world()
            .resource::<StylizedInstanceTable>()
            .buffer
            .clone();
        app.world_mut()
            .entity_mut(entity)
            .get_mut::<StylizedInstanceTableEntry>()
            .unwrap()
            .0
            .face_threshold = 0.75;
        app.update();
        assert_eq!(
            app.world().resource::<StylizedInstanceTable>().buffer,
            buffer
        );
        let bytes = app
            .world()
            .resource::<Assets<ShaderBuffer>>()
            .get(&buffer)
            .unwrap()
            .data
            .as_ref()
            .unwrap();
        let record = unpack_instance_record(bytes[..INSTANCE_RECORD_STRIDE].try_into().unwrap());
        assert_eq!(record.face_threshold, 0.75);
    }
    #[test]
    fn removing_a_rejected_entry_clears_admission_failure_and_can_rejoin() {
        let mut app = table_app();
        app.insert_resource(InstanceTableBudget { max_bytes: 0 });
        let material = app
            .world_mut()
            .resource_mut::<Assets<StylizedCharacterTableMaterial>>()
            .add(StylizedCharacterTableMaterial::default());
        let entity = app
            .world_mut()
            .spawn((
                StylizedInstanceTableEntry(Default::default()),
                MeshMaterial3d(material.clone()),
            ))
            .id();
        app.update();
        assert!(
            app.world()
                .entity(entity)
                .contains::<InstanceTableCapacityExceeded>()
        );
        assert!(
            !app.world()
                .entity(entity)
                .contains::<StylizedInstanceSlot>()
        );
        app.world_mut()
            .entity_mut(entity)
            .remove::<StylizedInstanceTableEntry>();
        app.update();
        assert!(
            !app.world()
                .entity(entity)
                .contains::<InstanceTableCapacityExceeded>()
        );
        assert!(
            app.world()
                .entity(entity)
                .contains::<SuspendedTableMaterial>()
        );
        assert!(
            !app.world()
                .entity(entity)
                .contains::<MeshMaterial3d<StylizedCharacterTableMaterial>>()
        );
        app.insert_resource(InstanceTableBudget {
            max_bytes: INSTANCE_RECORD_STRIDE as u64,
        });
        app.world_mut()
            .entity_mut(entity)
            .insert(StylizedInstanceTableEntry(Default::default()));
        app.update();
        assert_eq!(
            app.world()
                .entity(entity)
                .get::<MeshMaterial3d<StylizedCharacterTableMaterial>>()
                .unwrap()
                .0,
            material
        );
        assert!(
            !app.world()
                .entity(entity)
                .contains::<SuspendedTableMaterial>()
        );
        assert_eq!(
            app.world()
                .entity(entity)
                .get::<StylizedInstanceSlot>()
                .unwrap()
                .0,
            0
        );
    }

    #[test]
    fn idle_native_app_does_not_allocate_a_storage_asset() {
        let mut app = table_app();
        app.update();
        app.update();
        assert!(app.world().resource::<Assets<ShaderBuffer>>().is_empty());
        assert_eq!(app.world().resource::<StylizedInstanceTable>().next_slot, 0);
        assert_eq!(app.world().resource::<StylizedInstanceTable>().capacity, 0);
    }
}
