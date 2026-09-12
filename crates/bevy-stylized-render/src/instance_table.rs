//! GPU Instance Database（V2-e，provisional pending ADR-0004 关闭）：
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
//! - **槽位只增不减**：despawn 的实体留下 tombstone 槽位（无人读取），
//!   回收/压缩属后续迭代。

use bevy::{
    asset::{Assets, RenderAssetUsages},
    prelude::{
        Changed, Commands, Component, Entity, Or, PostUpdate, Query, ResMut, Resource, Without,
    },
    render::storage::ShaderBuffer,
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

/// 实例表（资源）：GPU buffer 句柄 + 槽位分配状态。
#[derive(Resource)]
pub struct StylizedInstanceTable {
    /// 底层存储 buffer（数据资产；记录变更时整体重写）。
    pub buffer: bevy::prelude::Handle<ShaderBuffer>,
    /// 当前容量（槽位数）。
    pub capacity: u32,
    /// 下一个待分配槽位。
    pub next_slot: u32,
}

/// 表同步：分配槽位（只增不减）+ 记录变更时整体重写 buffer。
///
/// 先于 `sync_instance_tags`（同一 PostUpdate 内完成槽位 → 索引打包）。
/// 注意 commands 插入的槽位本帧不可见，新实体的记录经本地分配表直接
/// 写入（不等下一帧）。
pub fn sync_instance_table(
    mut commands: Commands,
    table: Option<ResMut<StylizedInstanceTable>>,
    mut buffers: Option<ResMut<Assets<ShaderBuffer>>>,
    new_entries: Query<(Entity, &StylizedInstanceTableEntry), Without<StylizedInstanceSlot>>,
    changed: Query<
        (&StylizedInstanceSlot, &StylizedInstanceTableEntry),
        Or<(
            Changed<StylizedInstanceTableEntry>,
            Changed<StylizedInstanceSlot>,
        )>,
    >,
    all: Query<(&StylizedInstanceSlot, &StylizedInstanceTableEntry)>,
) {
    // 懒初始化：Assets<ShaderBuffer> 仅在渲染栈就位时存在（headless 无
    // RenderPlugin 的 App 跳过——纯逻辑测试可手动 init_asset）。
    let Some(buffers) = buffers.as_deref_mut() else {
        return;
    };
    let mut table = match table {
        Some(t) => t,
        None => {
            let buffer = buffers.add(ShaderBuffer::with_size(
                INSTANCE_TABLE_INITIAL_CAPACITY as usize * INSTANCE_RECORD_STRIDE,
                RenderAssetUsages::default(),
            ));
            commands.insert_resource(StylizedInstanceTable {
                buffer,
                capacity: INSTANCE_TABLE_INITIAL_CAPACITY,
                next_slot: 0,
            });
            return; // 资源下一帧生效
        }
    };

    // 槽位分配（确定性：按 query 迭代序 = 生成序）。commands 插入本帧
    // 不可见，本地记录 (entity → slot) 供本帧写 buffer。
    let mut assigned: Vec<(Entity, u32, StylizedInstanceGpuRecord)> = Vec::new();
    for (entity, entry) in &new_entries {
        let slot = table.next_slot;
        table.next_slot += 1;
        commands.entity(entity).insert(StylizedInstanceSlot(slot));
        assigned.push((entity, slot, entry.0));
    }
    let dirty = !assigned.is_empty() || !changed.is_empty();
    if !dirty {
        return;
    }

    // 容量不足 → 倍增重建（只增不减；旧槽位内容随全量重写保留）。
    if table.next_slot > table.capacity {
        let mut new_capacity = table.capacity.max(1);
        while new_capacity < table.next_slot {
            new_capacity *= 2; // 指数增长保证摊销
        }
        let fresh = buffers.add(ShaderBuffer::with_size(
            new_capacity as usize * INSTANCE_RECORD_STRIDE,
            RenderAssetUsages::default(),
        ));
        table.buffer = fresh;
        table.capacity = new_capacity;
    }

    // 全量重写（N × 48B；仅在脏时发生）。
    let mut bytes = vec![0u8; table.capacity as usize * INSTANCE_RECORD_STRIDE];
    for (slot, entry) in &all {
        if slot.0 >= table.capacity {
            continue; // 防御：容量刚重建的同一帧内 all 查询已含新槽位
        }
        let offset = slot.0 as usize * INSTANCE_RECORD_STRIDE;
        bytes[offset..offset + INSTANCE_RECORD_STRIDE]
            .copy_from_slice(&pack_instance_record(&entry.0));
    }
    for (_, slot, record) in &assigned {
        if *slot >= table.capacity {
            continue;
        }
        let offset = *slot as usize * INSTANCE_RECORD_STRIDE;
        bytes[offset..offset + INSTANCE_RECORD_STRIDE]
            .copy_from_slice(&pack_instance_record(record));
    }
    let len = bytes.len() as u64;
    if let Some(mut buffer) = buffers.get_mut(&table.buffer) {
        buffer.data = Some(bytes);
        buffer.buffer_description.size = len;
    }
}

/// 注册表通道（资源懒初始化 + PostUpdate 系统，先于 instance tag 同步）。
pub(crate) fn register(app: &mut bevy::app::App) {
    use bevy::prelude::IntoScheduleConfigs;
    app.add_systems(
        PostUpdate,
        sync_instance_table.before(crate::instance::sync_instance_tags),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    fn table_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::asset::AssetPlugin::default())
            .init_asset::<ShaderBuffer>();
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
        let entities: Vec<Entity> = (0..3)
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
        assert_eq!(record.face_threshold, 0.2);
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
    }
}
