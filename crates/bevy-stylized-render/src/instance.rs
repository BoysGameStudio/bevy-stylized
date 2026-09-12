//! Instance 层数据传输（provisional，pending ADR-0004）。
//!
//! 依据 It-3b 的 spike 证据（`target/tmp/adr-0004-spike/`）：MeshTag 方案
//! 在 100/1000 实例下保持 1 次 draw（自动合批未破坏），且 Portable 不依赖
//! storage buffer。因此 Portable 默认走 **MeshTag 位打包**：
//!
//! ```text
//! bits  0..=7   profile_index（ramp 行 / palette 索引）
//! bits  8..=15  face_threshold（u8 量化 0..=1；0 = 未设置，用材质默认）
//! bit   16      face_side（0 = 自动按光源方向，1 = 强制翻转）
//! bits  17..=23 outline_scale（u7 量化 0..=2）
//! bits  24..=31 per_character_flags（8 位；bit 0..=2 的 LOD 语义见
//!               `crate::lod` 的 `INSTANCE_FLAG_*`，V2-b）
//! ```
//!
//! StorageTable 模式（V2-e，实体带 [`StylizedInstanceSlot`] 时）：
//! bits 0..=23 = 槽位索引（全精度数据在 `crate::instance_table` 的
//! `ShaderBuffer` 表里），bits 24..=31 flags 不变。
//!
//! 位布局是 **provisional**：ADR-0004 关闭前任何外部代码不得依赖它；
//! 读写只能经 [`StylizedInstanceData::to_tag`] / [`StylizedInstanceData::from_tag`]。
//!
//! 数据流：`StylizedInstanceData` 组件 → [`sync_instance_tags`] →
//! `MeshTag`（Bevy mesh uniform 数组的 per-instance 字段，不破坏合批）。
//! **禁止** per-frame 修改材质资产；instance 数据只走这条通道。

use bevy::{
    mesh::MeshTag,
    prelude::{Changed, Commands, Component, Entity, Query, Reflect},
};
use bevy_stylized_core::StylizedInstanceDescriptor;

/// 位段：profile_index。
pub const TAG_PROFILE_INDEX_SHIFT: u32 = 0;
/// 位段：face_threshold（量化）。
pub const TAG_FACE_THRESHOLD_SHIFT: u32 = 8;
/// 位段：face_side。
pub const TAG_FACE_SIDE_SHIFT: u32 = 16;
/// 位段：outline_scale（量化）。
pub const TAG_OUTLINE_SCALE_SHIFT: u32 = 17;
/// 位段：per-character flags。
pub const TAG_FLAGS_SHIFT: u32 = 24;

/// face_threshold 的量化值 0 保留为"未设置"（shader 回退材质默认）。
pub const TAG_FACE_THRESHOLD_UNSET: u8 = 0;

/// StorageTable 模式下的槽位索引掩码（bits 0..=23；bits 24..=31 仍为
/// per-character flags，两通道共享）。
pub const TAG_TABLE_INDEX_MASK: u32 = 0x00FF_FFFF;

/// StorageTable 槽位（由 [`crate::instance_table`] 分配；组件存在即
/// 声明该实例走表通道——`sync_instance_tags` 把 tag 打包为索引布局）。
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub struct StylizedInstanceSlot(pub u32);

/// Instance 数据传输通道（provisional，pending ADR-0004）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum StylizedInstanceTransport {
    /// MeshTag 位打包（Portable 基线；spike 已验证合批不破坏）。
    MeshTagBits,
    /// Storage buffer 实例表（Standard+；V2-e 落地，见
    /// [`crate::instance_table`]）：MeshTag 携带 24-bit 槽位索引，表携带
    /// 全精度记录。WebGL2/GLES3.0 baseline 无 storage buffer，Portable
    /// 永不选择本通道（fallback 语义：
    /// [`bevy_stylized_core::StylizedFallbackStep::InstanceTableToMeshTag`]）。
    StorageTable,
}

/// 当前生效的 instance 传输通道（由 [`select_transport`] 依据 Stage B
/// 解析结果选择）。
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct StylizedInstanceTransportMode(pub StylizedInstanceTransport);

/// 依 Stage B 解析结果选择传输通道（V2-e）。
///
/// `resolved.tier >= Standard` ⇒ StorageTable（Standard 档的 tier 需求表
/// 已要求 `StorageBufferInVertexStage`，故解析到 Standard+ 即蕴含设备
/// storage 能力）；否则 MeshTagBits。`None`（无解析结果）fail-closed
/// 落 MeshTagBits。
#[must_use]
pub fn select_transport(
    resolved: Option<&bevy_stylized_core::StylizedResolvedVariant>,
) -> StylizedInstanceTransport {
    match resolved {
        Some(variant) if variant.tier >= bevy_stylized_core::StylizedExecutionTier::Standard => {
            StylizedInstanceTransport::StorageTable
        }
        _ => StylizedInstanceTransport::MeshTagBits,
    }
}

/// Instance 层 NPR 数据（对应 core 的 [`StylizedInstanceDescriptor`]；
/// 组件化 + MeshTag 打包传输）。
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
pub struct StylizedInstanceData {
    /// Profile / ramp 行索引（0..=255）。
    pub profile_index: u32,
    /// Face SDF 阈值；`None` 表示用材质默认值。
    pub face_threshold: Option<f32>,
    /// Face 侧选择：false = 自动（按光源方向），true = 强制翻转。
    pub face_side_flip: bool,
    /// 描边宽度缩放（0..=2）。
    pub outline_scale: f32,
    /// Per-character 开关位（8 位）。
    pub flags: u8,
}

impl Default for StylizedInstanceData {
    fn default() -> Self {
        Self {
            profile_index: 0,
            face_threshold: None,
            face_side_flip: false,
            outline_scale: 1.0,
            flags: 0,
        }
    }
}

impl StylizedInstanceData {
    /// StorageTable 模式的 tag 打包：bits 0..=23 = 槽位索引，bits 24..=31
    /// = flags（与位打包模式共享）。
    #[must_use]
    pub fn index_tag(slot: u32, flags: u8) -> MeshTag {
        MeshTag((slot & TAG_TABLE_INDEX_MASK) | (u32::from(flags) << TAG_FLAGS_SHIFT))
    }

    /// 打包为 `MeshTag` 位段。
    #[must_use]
    pub fn to_tag(&self) -> MeshTag {
        let threshold = match self.face_threshold {
            Some(value) => {
                // 1..=255 → 0..=1；0 保留为"未设置"。
                (value.clamp(0.0, 1.0) * 254.0).round() as u32 + 1
            }
            None => u32::from(TAG_FACE_THRESHOLD_UNSET),
        };
        let scale = (self.outline_scale.clamp(0.0, 2.0) * 63.5).round() as u32;
        MeshTag(
            (self.profile_index.min(0xFF) << TAG_PROFILE_INDEX_SHIFT)
                | (threshold << TAG_FACE_THRESHOLD_SHIFT)
                | (u32::from(self.face_side_flip) << TAG_FACE_SIDE_SHIFT)
                | (scale << TAG_OUTLINE_SCALE_SHIFT)
                | (u32::from(self.flags) << TAG_FLAGS_SHIFT),
        )
    }

    /// 从 `MeshTag` 位段解包（`to_tag` 的逆运算，量化精度内一致）。
    #[must_use]
    pub fn from_tag(tag: MeshTag) -> Self {
        let raw = tag.0;
        let threshold = (raw >> TAG_FACE_THRESHOLD_SHIFT) & 0xFF;
        Self {
            profile_index: (raw >> TAG_PROFILE_INDEX_SHIFT) & 0xFF,
            face_threshold: (threshold != u32::from(TAG_FACE_THRESHOLD_UNSET))
                .then(|| (threshold - 1) as f32 / 254.0),
            face_side_flip: (raw >> TAG_FACE_SIDE_SHIFT) & 1 != 0,
            outline_scale: ((raw >> TAG_OUTLINE_SCALE_SHIFT) & 0x7F) as f32 / 63.5,
            flags: (raw >> TAG_FLAGS_SHIFT) as u8,
        }
    }
}

impl From<StylizedInstanceDescriptor> for StylizedInstanceData {
    fn from(descriptor: StylizedInstanceDescriptor) -> Self {
        Self {
            profile_index: descriptor.profile_index,
            face_threshold: Some(descriptor.face_threshold),
            face_side_flip: descriptor.face_side != 0,
            outline_scale: descriptor.outline_scale,
            flags: descriptor.per_character_flags as u8,
        }
    }
}

/// 把变更过的 [`StylizedInstanceData`] 同步到 `MeshTag`（缺则补插）。
///
/// 只在组件变更（`Added`/`Changed`）时写入，无 per-frame 资产写。
/// 带 [`StylizedInstanceSlot`] 的实体打包为 StorageTable 索引布局
/// （bits 0..=23 = 槽位）；其余保持位打包布局。
pub fn sync_instance_tags(
    mut commands: Commands,
    mut changed: Query<
        (
            Entity,
            &StylizedInstanceData,
            Option<&StylizedInstanceSlot>,
            Option<&mut MeshTag>,
        ),
        bevy::prelude::Or<(Changed<StylizedInstanceData>, Changed<StylizedInstanceSlot>)>,
    >,
    missing: Query<
        (Entity, &StylizedInstanceData, Option<&StylizedInstanceSlot>),
        bevy::prelude::Without<MeshTag>,
    >,
) {
    let pack = |data: &StylizedInstanceData, slot: Option<&StylizedInstanceSlot>| match slot {
        Some(slot) => StylizedInstanceData::index_tag(slot.0, data.flags),
        None => data.to_tag(),
    };
    for (entity, data, slot, tag) in &mut changed {
        let packed = pack(data, slot);
        if let Some(mut tag) = tag {
            *tag = packed;
        } else {
            commands.entity(entity).insert(packed);
        }
    }
    for (entity, data, slot) in &missing {
        commands.entity(entity).insert(pack(data, slot));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_roundtrip_within_quantization() {
        let data = StylizedInstanceData {
            profile_index: 42,
            face_threshold: Some(0.5),
            face_side_flip: true,
            outline_scale: 1.5,
            flags: 0b1010_0101,
        };
        let back = StylizedInstanceData::from_tag(data.to_tag());
        assert_eq!(back.profile_index, 42);
        let threshold = back.face_threshold.expect("threshold set");
        assert!((threshold - 0.5).abs() < 0.01, "threshold {threshold}");
        assert!(back.face_side_flip);
        assert!((back.outline_scale - 1.5).abs() < 0.02);
        assert_eq!(back.flags, 0b1010_0101);
    }

    #[test]
    fn unset_threshold_encodes_as_reserved_zero() {
        let data = StylizedInstanceData::default();
        assert_eq!(data.face_threshold, None);
        let back = StylizedInstanceData::from_tag(data.to_tag());
        assert_eq!(back.face_threshold, None);
    }

    #[test]
    fn descriptor_conversion_preserves_fields() {
        let descriptor = StylizedInstanceDescriptor {
            face_threshold: 0.75,
            face_side: 1,
            outline_scale: 1.2,
            profile_index: 7,
            per_character_flags: 3,
            ..StylizedInstanceDescriptor::default()
        };
        let data = StylizedInstanceData::from(descriptor);
        assert_eq!(data.profile_index, 7);
        assert_eq!(data.face_threshold, Some(0.75));
        assert!(data.face_side_flip);
        assert!((data.outline_scale - 1.2).abs() < f32::EPSILON);
        assert_eq!(data.flags, 3);
    }

    #[test]
    fn transport_selector_stays_on_mesh_tag() {
        assert_eq!(
            select_transport(None),
            StylizedInstanceTransport::MeshTagBits
        );
    }

    #[test]
    fn transport_selection_matrix_caps_by_tier() {
        use bevy_stylized_core::{
            StylizedExecutionTier, StylizedResolvedVariant, StylizedSurfaceClass,
        };
        let variant = |tier| StylizedResolvedVariant {
            class: StylizedSurfaceClass::Character,
            requested_tier: tier,
            tier,
            tier_reduced: false,
            missing_features: Vec::new(),
        };
        // Portable（含 tier_reduced 降级）→ MeshTagBits；Standard/High
        // ⇒ StorageTable（Standard 的 tier 需求表已要求 storage buffer
        // 特性，解析到该档即蕴含设备能力）。
        assert_eq!(
            select_transport(Some(&variant(StylizedExecutionTier::Portable))),
            StylizedInstanceTransport::MeshTagBits
        );
        assert_eq!(
            select_transport(Some(&variant(StylizedExecutionTier::Standard))),
            StylizedInstanceTransport::StorageTable
        );
        assert_eq!(
            select_transport(Some(&variant(StylizedExecutionTier::High))),
            StylizedInstanceTransport::StorageTable
        );
        // 降档到 Portable 的 High 请求 → 仍 MeshTagBits（按实际档）。
        let mut reduced = variant(StylizedExecutionTier::Portable);
        reduced.requested_tier = StylizedExecutionTier::High;
        reduced.tier_reduced = true;
        assert_eq!(
            select_transport(Some(&reduced)),
            StylizedInstanceTransport::MeshTagBits
        );
    }

    #[test]
    fn index_tag_layout_carries_slot_and_flags() {
        let tag = StylizedInstanceData::index_tag(0x00AB_CDEF, 0b1010_0101);
        assert_eq!(tag.0 & TAG_TABLE_INDEX_MASK, 0x00AB_CDEF);
        assert_eq!((tag.0 >> TAG_FLAGS_SHIFT) & 0xFF, 0b1010_0101);
        // 槽位越界截断到 24 bit。
        let clamped = StylizedInstanceData::index_tag(0xFFFF_FFFF, 0);
        assert_eq!(clamped.0 & TAG_TABLE_INDEX_MASK, 0x00FF_FFFF);
    }

    #[test]
    fn sync_writes_tag_only_on_change() {
        use bevy::prelude::*;
        let mut app = App::new();
        app.add_systems(Update, sync_instance_tags);
        let entity = app
            .world_mut()
            .spawn((
                Mesh3d::default(),
                StylizedInstanceData {
                    profile_index: 5,
                    ..StylizedInstanceData::default()
                },
            ))
            .id();
        app.update();
        let tag = app.world().entity(entity).get::<MeshTag>().map(|t| t.0);
        assert_eq!(tag.map(|t| t & 0xFF), Some(5));

        // 不变更组件时不再写入（tag 保持同值且无 Changed 触发）。
        app.update();
        let tag2 = app.world().entity(entity).get::<MeshTag>().map(|t| t.0);
        assert_eq!(tag, tag2);

        // 变更组件 → 重新打包。
        app.world_mut()
            .entity_mut(entity)
            .get_mut::<StylizedInstanceData>()
            .expect("component")
            .profile_index = 9;
        app.update();
        let tag3 = app.world().entity(entity).get::<MeshTag>().map(|t| t.0);
        assert_eq!(tag3.map(|t| t & 0xFF), Some(9));
    }
}
