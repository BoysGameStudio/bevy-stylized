//! Disabled-feature 成本测试（provisional，pending ADR-0026）：
//! 组件缺失时对应系统必须零工作（cheap query early-out），不产生实体、
//! 不写资产。

use bevy::prelude::*;
use bevy_stylized::prelude::*;
use bevy_stylized::stylized_render::{
    StylizedOutlineReplica,
    outline::sync_outline_replicas,
    post::StylizedFullscreenEffect,
    screen_outline::{StylizedScreenOutlineOverlay, sync_screen_outline},
};

/// 无 `StylizedOutline` 组件时，outline 副本系统不得产生任何副本实体。
#[test]
fn outline_sync_is_noop_without_outline_components() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::asset::AssetPlugin {
            meta_check: bevy::asset::AssetMetaCheck::Never,
            ..Default::default()
        })
        .add_plugins(bevy::mesh::MeshPlugin)
        .add_systems(PostUpdate, sync_outline_replicas);

    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(bevy::math::primitives::Sphere::new(0.5));
    for i in 0..10 {
        app.world_mut().spawn((
            Mesh3d(mesh.clone()),
            Transform::from_xyz(i as f32, 0.0, 0.0),
        ));
    }
    let entity_count_before = app.world().entities().len();
    for _ in 0..5 {
        app.update();
    }
    let mut replicas = app.world_mut().query::<&StylizedOutlineReplica>();
    assert_eq!(replicas.iter(app.world()).count(), 0);
    assert_eq!(app.world().entities().len(), entity_count_before);
}

/// 无材质资产事件时，flag 同步系统不得写入任何材质资产。
#[test]
fn flag_sync_is_noop_without_material_events() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::asset::AssetPlugin {
            meta_check: bevy::asset::AssetMetaCheck::Never,
            ..Default::default()
        })
        .add_plugins(StylizedPlugin::default());
    // 创建一个材质后跑若干帧：初次 Added 触发一次同步写，之后必须静默。
    app.world_mut()
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial::default());
    for _ in 0..3 {
        app.update();
    }
    // 清空事件流后再跑——不得再出现 Modified 事件。
    app.world_mut()
        .resource_mut::<Messages<AssetEvent<StylizedCelMaterial>>>()
        .clear();
    for _ in 0..5 {
        app.update();
    }
    let events = app
        .world()
        .resource::<Messages<AssetEvent<StylizedCelMaterial>>>();
    let modified = events
        .get_cursor()
        .read(events)
        .filter(|e| matches!(e, AssetEvent::Modified { .. }))
        .count();
    assert_eq!(modified, 0, "flag sync must not write without real changes");
}

/// 相机无 `StylizedPostProcess` 组件时，post 同步系统不得创建 effect。
#[test]
fn post_sync_is_noop_without_post_component() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::asset::AssetPlugin {
            meta_check: bevy::asset::AssetMetaCheck::Never,
            ..Default::default()
        })
        .add_plugins(StylizedPlugin::default());
    app.world_mut().spawn(Camera3d::default());
    for _ in 0..3 {
        app.update();
    }
    let mut effects = app.world_mut().query::<&StylizedFullscreenEffect>();
    assert_eq!(effects.iter(app.world()).count(), 0);
}

/// 无 `StylizedScreenOutline` 组件时：无 overlay、无 prepass 组件插入
///（prepass 成本零）。
#[test]
fn screen_outline_absent_means_zero_prepass_work() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::asset::AssetPlugin {
            meta_check: bevy::asset::AssetMetaCheck::Never,
            ..Default::default()
        })
        .add_plugins(bevy::mesh::MeshPlugin);
    app.init_resource::<Assets<
        bevy_stylized::stylized_render::screen_outline::StylizedScreenOutlineMaterial,
    >>();
    app.add_systems(PostUpdate, sync_screen_outline);
    app.world_mut().spawn(Camera3d::default());
    for _ in 0..3 {
        app.update();
    }
    let mut overlays = app.world_mut().query::<&StylizedScreenOutlineOverlay>();
    assert_eq!(overlays.iter(app.world()).count(), 0);
    let mut prepass = app
        .world_mut()
        .query::<&bevy::core_pipeline::prepass::DepthPrepass>();
    assert_eq!(prepass.iter(app.world()).count(), 0);
}
