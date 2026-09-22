//! 场景所有权与生命周期账本。
//!
//! driver 构建场景前记录宿主基线，构建后把"场景新增的实体"记入
//! [`SceneLedger`]；同 SessionPlan 内的合法场景切换只退役账本内的实体，
//! 有界排空异步工作（资产句柄释放在后续 update 中处理），退役代次晚到
//! 的事件被忽略。运行时 Entity 不进入可保存配方——账本只用于生命周期
//! 管理与证据核对。

use std::collections::HashSet;

use bevy::asset::Assets;
use bevy::prelude::{Camera3d, Entity, Image, Mesh, World};
use bevy_stylized::prelude::{
    StylizedCelMaterial, StylizedCharacterMaterial, StylizedHeroMaterial, StylizedOutlineMaterial,
};

use scenario_model::ident::Generation;

/// 场景自有实体账本（一次内容实例化 = 一代）。
#[derive(Clone, Debug)]
pub struct SceneLedger {
    /// 本账本所属代次。
    pub generation: Generation,
    /// 构建该代场景时新增的实体（含观测相机；子实体随父代递归退役）。
    pub entities: HashSet<Entity>,
}

impl SceneLedger {
    /// 世界实体快照（基线或构建后全集）。
    ///
    /// Bevy 0.19 把资源存放在专用实体上；资源实体不是场景内容，且对其
    /// despawn 会经 discard hook 报错——快照一律排除资源实体。场景构建器
    /// 不插入资源（内容资产经 `Assets<T>` 计数核对，见 [`SceneCounts`]）；
    /// driver 装配的探针资源（readiness Probe 等）属基础设施，跨场景保留。
    #[must_use]
    pub fn snapshot(world: &World) -> HashSet<Entity> {
        let resource_entities: HashSet<Entity> = world
            .resource_entities()
            .iter()
            .map(|(_, entity)| entity)
            .collect();
        world
            .iter_entities()
            .map(|entity| entity.id())
            .filter(|entity| !resource_entities.contains(entity))
            .collect()
    }

    /// 以基线快照为参照建立账本：`after − baseline`。
    #[must_use]
    pub fn record(generation: Generation, baseline: &HashSet<Entity>, world: &World) -> Self {
        let entities = Self::snapshot(world)
            .into_iter()
            .filter(|entity| !baseline.contains(entity))
            .collect();
        Self {
            generation,
            entities,
        }
    }

    /// 退役本代实体：只 despawn 账本内的根实体（父代不在账本中的实体），
    /// 子实体随递归 despawn 退役——避免无序集合先退役子实体后，父实体的
    /// 递归 despawn 再访问已失效实体。
    pub fn retire(&self, world: &mut World) {
        for entity in &self.entities {
            let is_root = world
                .get_entity(*entity)
                .ok()
                .and_then(|entity_ref| {
                    entity_ref
                        .get::<bevy::prelude::ChildOf>()
                        .map(|c| c.parent())
                })
                .is_none_or(|parent| !self.entities.contains(&parent));
            if !is_root {
                continue;
            }
            if let Ok(entity_mut) = world.get_entity_mut(*entity) {
                entity_mut.despawn();
            }
        }
    }
}

/// 生命周期证据用的场景资源计数（与基线逐项比较）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneCounts {
    /// 世界实体总数。
    pub entities: usize,
    /// Camera3d 数量。
    pub cameras: usize,
    /// Mesh 资产数。
    pub meshes: usize,
    /// Image 资产数。
    pub images: usize,
    /// Cel 材质数。
    pub cel_materials: usize,
    /// Character 材质数。
    pub character_materials: usize,
    /// Hero 材质数。
    pub hero_materials: usize,
    /// Outline 材质数。
    pub outline_materials: usize,
}

impl SceneCounts {
    /// 取当前计数。
    #[must_use]
    pub fn capture(world: &World) -> Self {
        Self {
            entities: world.iter_entities().count(),
            cameras: world
                .iter_entities()
                .filter(|entity| entity.get::<Camera3d>().is_some())
                .count(),
            meshes: world.resource::<Assets<Mesh>>().len(),
            images: world.resource::<Assets<Image>>().len(),
            cel_materials: world.resource::<Assets<StylizedCelMaterial>>().len(),
            character_materials: world.resource::<Assets<StylizedCharacterMaterial>>().len(),
            hero_materials: world.resource::<Assets<StylizedHeroMaterial>>().len(),
            outline_materials: world.resource::<Assets<StylizedOutlineMaterial>>().len(),
        }
    }
}
