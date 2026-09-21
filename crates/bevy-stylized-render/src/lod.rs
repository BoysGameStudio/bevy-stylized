//! Per-entity NPR LOD（V2-b，docs/roadmap/02 §8；provisional pending
//! ADR-0018 的 LOD 落地条款）。
//!
//! 不变量：**LOD 只削减 tier/feature/几何，永不降低 Surface Class**
//! （Hero 远处仍是 Hero；ADR-0018 三轴不变量）。
//!
//! 数据流（零 per-frame 材质资产变异）：
//!
//! ```text
//! StylizedLodPolicyResource（默认政策）+ 相机距离
//!   → core resolve_lod（确定性分带）
//!   → effective tier = min(band tier_cap, StylizedTierOverride)
//!   → StylizedInstanceData.flags（MeshTag 位，hero shader 逐实例消费）
//!   → StylizedLodMeshes 存在时按 band 换 Mesh3d 句柄（几何 LOD）
//! ```
//!
//! 写入只发生在数值变化时（guard write），`sync_instance_tags` 经
//! `Changed<StylizedInstanceData>` 拾取；材质资产从不被本模块触碰。

use bevy::prelude::{
    Camera, Component, GlobalTransform, Handle, Mesh, Mesh3d, Query, Reflect, Res, Resource, With,
};
use bevy_stylized_core::{
    StylizedExecutionTier, StylizedFallbackStep, StylizedLodPolicy, StylizedLodResolution,
    resolve_lod,
};

use crate::instance::StylizedInstanceData;

/// 动态质量给出的 tier 覆盖（`None` = 与 Stage A 请求档一致，无动态调整）。
///
/// 由 governor（facade `dynamic_quality`）或调用方写入；LOD 解析与
/// variant 选择共同消费。**定义在 render crate**：facade 经 re-export
/// 保持 `bevy_stylized::StylizedTierOverride` 公共路径不变。
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StylizedTierOverride(pub Option<StylizedExecutionTier>);

impl StylizedTierOverride {
    /// 有效请求档：`min(requested, override)`（动态调整只降不升于请求档）。
    #[must_use]
    pub fn effective_tier(&self, requested: StylizedExecutionTier) -> StylizedExecutionTier {
        match self.0 {
            Some(override_tier) => requested.min(override_tier),
            None => requested,
        }
    }
}

/// LOD 政策资源（core [`StylizedLodPolicy`] 的 ECS 包装；默认政策见 core）。
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct StylizedLodPolicyResource(pub StylizedLodPolicy);

/// 几何 LOD：band → mesh 句柄映射（只有声明了映射才换 `Mesh3d`）。
///
/// `band_meshes[i]` 是带 i 的网格；`None` 表示沿用更低带的第一个
/// `Some`（直到 band 0 = base）。恢复近距离时系统自动回到 base。
#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct StylizedLodMeshes {
    /// 按 band 索引的网格（0 = near/base）。
    pub band_meshes: [Option<Handle<Mesh>>; 4],
}

impl StylizedLodMeshes {
    /// 以 base（near 带）网格构造。
    #[must_use]
    pub fn new(base: Handle<Mesh>) -> Self {
        Self {
            band_meshes: [Some(base), None, None, None],
        }
    }

    /// band `band` 的有效网格：向下找到第一个声明的句柄。
    #[must_use]
    pub fn mesh_for_band(&self, band: u32) -> Option<&Handle<Mesh>> {
        (0..=band.min(3))
            .rev()
            .find_map(|b| self.band_meshes[b as usize].as_ref())
    }
}

/// Instance flags bit 0：削减 hero 额外特性（matcap / spec ramp /
/// packed normal）。hero shader 经 MeshTag bits 24..31 逐实例读取。
pub const INSTANCE_FLAG_DROP_HERO_EXTRAS: u8 = 1 << 0;
/// Instance flags bit 1：face SDF 降为硬 step 阈值。
pub const INSTANCE_FLAG_FACE_HARD_STEP: u8 = 1 << 1;
/// Instance flags bit 2：hair 高光关闭（rim 补偿属材质层，不在逐实例通道）。
pub const INSTANCE_FLAG_HAIR_SPECULAR_OFF: u8 = 1 << 2;

/// 由 LOD 解析结果与 governor override 计算 per-instance flags 与有效
/// tier（纯函数，确定性；base tier 恒为 High——authored 全特性起点，
/// LOD/override 只降不升）。
#[must_use]
pub fn lod_flags_for(
    resolution: &StylizedLodResolution,
    tier_override: Option<StylizedExecutionTier>,
) -> (u8, StylizedExecutionTier) {
    let mut effective = StylizedExecutionTier::High;
    if let Some(cap) = resolution.tier_cap {
        effective = effective.min(cap);
    }
    if let Some(override_tier) = tier_override {
        effective = effective.min(override_tier);
    }
    let forced_portable = effective == StylizedExecutionTier::Portable;
    let drops = |step: StylizedFallbackStep| resolution.drop_features.contains(&step);

    let mut flags = 0u8;
    if forced_portable
        || drops(StylizedFallbackStep::DropMatcap)
        || drops(StylizedFallbackStep::DropSpecRamp)
        || drops(StylizedFallbackStep::DropPackedNormal)
    {
        flags |= INSTANCE_FLAG_DROP_HERO_EXTRAS;
    }
    if forced_portable || drops(StylizedFallbackStep::FaceSdfHardStep) {
        flags |= INSTANCE_FLAG_FACE_HARD_STEP;
    }
    if forced_portable || drops(StylizedFallbackStep::HairSpecularToRim) {
        flags |= INSTANCE_FLAG_HAIR_SPECULAR_OFF;
    }
    (flags, effective)
}

/// Per-entity LOD 解析系统（PostUpdate，先于 `sync_instance_tags`）。
///
/// 对每个带 [`StylizedInstanceData`] 的实体：按到**最近相机**的距离分带
/// → [`resolve_lod`] → 与 [`StylizedTierOverride`] 取 min → guard-write
/// instance flags（MeshTag 位）；带 [`StylizedLodMeshes`] 的实体按 band
/// 换 `Mesh3d` 句柄。无相机时整体空转。
pub fn sync_lod(
    policy: Option<Res<StylizedLodPolicyResource>>,
    tier_override: Option<Res<StylizedTierOverride>>,
    cameras: Query<&GlobalTransform, With<Camera>>,
    mut instances: Query<(
        &GlobalTransform,
        &mut StylizedInstanceData,
        Option<&StylizedLodMeshes>,
        Option<&mut Mesh3d>,
    )>,
) {
    let Some(policy) = policy else { return };
    // 最近相机距离决定带（多相机取 min：最近的观察者决定质量）。
    let camera_positions: Vec<bevy::math::Vec3> = cameras.iter().map(|t| t.translation()).collect();
    if camera_positions.is_empty() {
        return;
    }
    let override_tier = tier_override.and_then(|o| o.0);

    for (transform, mut data, lod_meshes, mesh) in &mut instances {
        let position = transform.translation();
        let distance = camera_positions
            .iter()
            .map(|c| c.distance(position))
            .fold(f32::INFINITY, f32::min);
        let resolution = resolve_lod(distance, &policy.0);
        let (lod_flags, _effective_tier) = lod_flags_for(&resolution, override_tier);

        // 保留非 LOD 管理的位，只重写 bit 0..=2；值未变则不触碰
        // （避免触发 Changed → sync_instance_tags 的无效重打包）。
        let new_flags = (data.flags
            & !(INSTANCE_FLAG_DROP_HERO_EXTRAS
                | INSTANCE_FLAG_FACE_HARD_STEP
                | INSTANCE_FLAG_HAIR_SPECULAR_OFF))
            | lod_flags;
        if new_flags != data.flags {
            data.flags = new_flags;
        }

        // 几何 LOD：只有政策（组件）声明了 band→mesh 映射才换句柄。
        if let (Some(lod_meshes), Some(mut mesh)) = (lod_meshes, mesh)
            && let Some(target) = lod_meshes.mesh_for_band(resolution.band)
            && mesh.0 != *target
        {
            mesh.0 = target.clone();
        }
    }
}

/// LOD 系统注册（资源 + PostUpdate 系统，先于 instance tag 同步）。
///
/// `StylizedTierOverride` 有意**不**在此 init：它的存在性是 governor
/// （facade `dynamic_quality`，opt-in）已安装的可观测信号；未安装时
/// `sync_lod` 以 `Option<Res>` 读取，缺失即无 override。
pub(crate) fn register(app: &mut bevy::app::App) {
    use bevy::prelude::{IntoScheduleConfigs, PostUpdate};
    app.init_resource::<StylizedLodPolicyResource>()
        .add_systems(
            PostUpdate,
            sync_lod
                .before(crate::instance::sync_instance_tags)
                .in_set(crate::presentation::NativePresentationSystems),
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    /// 最小 App：Transform/GlobalTransform 直插（不经传播系统，值即设定值）。
    fn lod_app(policy: StylizedLodPolicy) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(StylizedLodPolicyResource(policy))
            .init_resource::<StylizedTierOverride>()
            .add_systems(PostUpdate, sync_lod);
        app
    }

    fn spawn_camera(app: &mut App, z: f32) {
        app.world_mut().spawn((
            Camera::default(),
            Transform::from_xyz(0.0, 0.0, z),
            GlobalTransform::from_xyz(0.0, 0.0, z),
        ));
    }

    fn spawn_instance(app: &mut App, z: f32) -> Entity {
        app.world_mut()
            .spawn((
                StylizedInstanceData::default(),
                Transform::from_xyz(0.0, 0.0, z),
                GlobalTransform::from_xyz(0.0, 0.0, z),
            ))
            .id()
    }

    /// band tier_cap = Portable ⇒ 该带即 portable variant（matcap/spec
    /// ramp/packed normal + face 硬 step + hair 高光全削减，与
    /// `hero_params_for_variant` 的 tier_reduced 路径语义一致）；
    /// `drop_features` 用于不做整带降级时的细粒度削减。
    #[test]
    fn portable_cap_implies_full_instance_fallback() {
        let policy = StylizedLodPolicy::default();
        let far = resolve_lod(30.0, &policy);
        let (flags, effective) = lod_flags_for(&far, None);
        assert_eq!(effective, StylizedExecutionTier::Portable);
        assert_eq!(
            flags,
            INSTANCE_FLAG_DROP_HERO_EXTRAS
                | INSTANCE_FLAG_FACE_HARD_STEP
                | INSTANCE_FLAG_HAIR_SPECULAR_OFF
        );
        // mid 仅 cap Standard：无 per-instance 特性削减。
        let mid = resolve_lod(15.0, &policy);
        let (mid_flags, mid_tier) = lod_flags_for(&mid, None);
        assert_eq!(mid_tier, StylizedExecutionTier::Standard);
        assert_eq!(mid_flags, 0);
    }

    #[test]
    fn lod_flags_track_distance_band_deterministically() {
        let mut app = lod_app(StylizedLodPolicy::default());
        spawn_camera(&mut app, 0.0);
        // 相机在 z=0，实体沿 -Z：距离 2（near）/ 15（mid）/ 30（far）/ 60（very_far）。
        let near = spawn_instance(&mut app, -2.0);
        let mid = spawn_instance(&mut app, -15.0);
        let far = spawn_instance(&mut app, -30.0);
        let very_far = spawn_instance(&mut app, -60.0);
        app.update();

        fn flags(app: &App, e: Entity) -> u8 {
            app.world()
                .entity(e)
                .get::<StylizedInstanceData>()
                .unwrap()
                .flags
        }
        assert_eq!(
            flags(&app, near) & INSTANCE_FLAG_DROP_HERO_EXTRAS,
            0,
            "near 不削减"
        );
        assert_eq!(
            flags(&app, mid) & INSTANCE_FLAG_DROP_HERO_EXTRAS,
            0,
            "mid 仅 cap tier"
        );
        assert_ne!(
            flags(&app, far) & INSTANCE_FLAG_DROP_HERO_EXTRAS,
            0,
            "far 削 hero 特性"
        );
        assert_ne!(
            flags(&app, very_far) & INSTANCE_FLAG_FACE_HARD_STEP,
            0,
            "very_far 加 face 硬 step"
        );
        assert_ne!(
            flags(&app, very_far) & INSTANCE_FLAG_HAIR_SPECULAR_OFF,
            0,
            "very_far 关 hair 高光"
        );

        // 确定性：同距离重跑 flags 稳定，且不产生新的变更（guard write）。
        let before = flags(&app, far);
        app.update();
        app.update();
        assert_eq!(flags(&app, far), before);
    }

    #[test]
    fn governor_override_is_respected_by_lod() {
        let mut app = lod_app(StylizedLodPolicy::default());
        spawn_camera(&mut app, 0.0);
        let near = spawn_instance(&mut app, -2.0); // near 带：LOD 自身不削减
        app.world_mut().resource_mut::<StylizedTierOverride>().0 =
            Some(StylizedExecutionTier::Portable);
        app.update();

        let flags = app
            .world()
            .entity(near)
            .get::<StylizedInstanceData>()
            .unwrap()
            .flags;
        assert_ne!(
            flags & INSTANCE_FLAG_DROP_HERO_EXTRAS,
            0,
            "override=Portable 时 near 实例也被压低（min 语义）"
        );
        assert_ne!(flags & INSTANCE_FLAG_FACE_HARD_STEP, 0);
        assert_ne!(flags & INSTANCE_FLAG_HAIR_SPECULAR_OFF, 0);
    }

    #[test]
    fn non_lod_flag_bits_are_preserved() {
        let mut app = lod_app(StylizedLodPolicy::default());
        spawn_camera(&mut app, 0.0);
        let entity = app
            .world_mut()
            .spawn((
                StylizedInstanceData {
                    flags: 0b1000_0000, // 非 LOD 管理位
                    ..StylizedInstanceData::default()
                },
                Transform::from_xyz(0.0, 0.0, -30.0),
                GlobalTransform::from_xyz(0.0, 0.0, -30.0),
            ))
            .id();
        app.update();
        let flags = app
            .world()
            .entity(entity)
            .get::<StylizedInstanceData>()
            .unwrap()
            .flags;
        assert_eq!(flags & 0b1000_0000, 0b1000_0000, "高位不被 LOD 覆写");
        assert_ne!(flags & INSTANCE_FLAG_DROP_HERO_EXTRAS, 0);
    }

    #[test]
    fn geometry_lod_swaps_and_restores_mesh_handles() {
        let mut app = lod_app(StylizedLodPolicy::default());
        spawn_camera(&mut app, 0.0);

        let mut meshes = Assets::<Mesh>::default();
        let base = meshes.add(Sphere::new(1.0));
        let low = meshes.add(Cuboid::new(0.5, 0.5, 0.5));
        let mut lod_meshes = StylizedLodMeshes::new(base.clone());
        lod_meshes.band_meshes[2] = Some(low.clone()); // far 带换低模

        let entity = app
            .world_mut()
            .spawn((
                StylizedInstanceData::default(),
                Mesh3d(base.clone()),
                lod_meshes,
                Transform::from_xyz(0.0, 0.0, -30.0), // far 带
                GlobalTransform::from_xyz(0.0, 0.0, -30.0),
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().entity(entity).get::<Mesh3d>().unwrap().0,
            low,
            "far 带换成低模"
        );

        // 相机靠近 → 回到 base。
        app.world_mut()
            .entity_mut(entity)
            .insert(GlobalTransform::from_xyz(0.0, 0.0, -1.0));
        app.update();
        assert_eq!(
            app.world().entity(entity).get::<Mesh3d>().unwrap().0,
            base,
            "近距恢复 base 网格"
        );
    }

    #[test]
    fn sync_lod_never_mutates_material_assets() {
        use crate::hero::StylizedHeroMaterial;
        let mut app = lod_app(StylizedLodPolicy::default());
        spawn_camera(&mut app, 0.0);
        spawn_instance(&mut app, -60.0);

        app.init_resource::<Assets<StylizedHeroMaterial>>();
        let handle = app
            .world_mut()
            .resource_mut::<Assets<StylizedHeroMaterial>>()
            .add(StylizedHeroMaterial::default());
        let before = app
            .world()
            .resource::<Assets<StylizedHeroMaterial>>()
            .get(&handle)
            .unwrap()
            .clone();

        app.update();
        app.update();

        let after = app
            .world()
            .resource::<Assets<StylizedHeroMaterial>>()
            .get(&handle)
            .unwrap();
        assert_eq!(
            format!("{before:?}"),
            format!("{after:?}"),
            "LOD 不得触碰材质资产（per-instance 效果只走 MeshTag 位）"
        );
        // MeshTag 由 sync_instance_tags 写入；本测试 App 未注册它，
        // 只验证 LOD 自身零材质写。
    }
}
