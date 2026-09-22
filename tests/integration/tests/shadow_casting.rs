//! `shadow-casting` feature 的运行时 GPU 证据（ADR-0023）。
//!
//! 场景：白色 StandardMaterial 地面（唯一接收阴影的表面；stylized 着色器
//! 从不采样 shadow map，接收语义不变）+ 两个立方体 caster——Hero 材质
//! （feature 开启时 `enable_shadows() == true`）与 Cel 材质（永不投射，
//! 同场景对照）。侧向 45° 平行光把阴影投向 +x 一侧、与 caster 分离。
//!
//! 方法：同一会话内对同一支平行光做 A/B（`shadow_maps_enabled` 开/关各捕
//! 一张），差分隔离出阴影贡献本身：
//!
//! - feature 开启（`--features shadow-casting`）：Hero 侧地面出现显著
//!   差分（阴影落区），Cel 侧无差分（对照），caster 自身像素不变
//!   （投射不改变受光——stylized shader 不采样 shadow map）；
//! - feature 关闭（默认构建，即"既有行为不变"的对照）：A/B 逐字节一致，
//!   两侧都不产生阴影。

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    color::{Color, LinearRgba},
    light::DirectionalLight,
    math::{
        Vec3,
        primitives::{Cuboid, Rectangle},
    },
    mesh::Mesh,
    pbr::{MeshMaterial3d, StandardMaterial},
    prelude::{Mesh3d, Transform},
    render::{RenderPlugin, settings::RenderCreation},
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::*;
use golden_harness::capture;

/// 本测试二进制编译时的 feature 状态（integration 包的转发 feature）。
const SHADOW_CASTING: bool = cfg!(feature = "shadow-casting");

fn build_app() -> App {
    let resources = capture::probe_render_resources().expect("required GPU unavailable");
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: "assets".into(),
                meta_check: bevy::asset::AssetMetaCheck::Never,
                ..Default::default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Manual(resources),
                synchronous_pipeline_compilation: true,
                ..Default::default()
            })
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..Default::default()
            }),
    );
    app.add_plugins(StylizedPlugin::default());
    capture::install(&mut app);
    golden_harness::readiness::install(&mut app);
    app.finish();
    app.cleanup();
    {
        // 压低环境光使阴影对比最大化；托管 key light 不产阴影
        // （view.rs 固定 shadow_maps_enabled: false），不参与本测试。
        let mut view = app.world_mut().resource_mut::<StylizedViewSettings>();
        view.0.ambient_override = Some(LinearRgba::new(0.02, 0.02, 0.02, 1.0));
    }

    let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
    let ground_mesh = meshes.add(Rectangle::new(16.0, 16.0));
    let cube_mesh = meshes.add(Cuboid::new(1.5, 1.5, 1.5));
    let ground = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 1.0,
            ..Default::default()
        });
    app.world_mut().spawn((
        Mesh3d(ground_mesh),
        MeshMaterial3d(ground),
        Transform::from_rotation(bevy::math::Quat::from_rotation_x(
            -std::f32::consts::FRAC_PI_2,
        )),
    ));
    // Hero caster（feature 门控）在左，Cel 对照（永不投射）在右。
    let hero = app
        .world_mut()
        .resource_mut::<Assets<StylizedHeroMaterial>>()
        .add(StylizedHeroMaterial::default());
    app.world_mut().spawn((
        Mesh3d(cube_mesh.clone()),
        MeshMaterial3d(hero),
        Transform::from_xyz(-3.0, 0.75, 0.0),
    ));
    let cel = app
        .world_mut()
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial::default());
    app.world_mut().spawn((
        Mesh3d(cube_mesh),
        MeshMaterial3d(cel),
        Transform::from_xyz(3.0, 0.75, 0.0),
    ));
    // 侧向 45° 平行光：阴影落向 +x，与 caster 明显分离。
    app.world_mut().spawn((
        DirectionalLight {
            illuminance: 60_000.0,
            shadow_maps_enabled: true,
            ..Default::default()
        },
        Transform::from_xyz(-4.0, 5.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        bevy::light::CascadeShadowConfigBuilder {
            maximum_distance: 30.0,
            first_cascade_far_bound: 10.0,
            ..Default::default()
        }
        .build(),
    ));
    app
}

fn capture_once(app: &mut App, session: &str) -> golden_harness::CapturedImage {
    scene_runtime::capture_in_app(
        app,
        &scenario_model::ident::SessionId::new(session).expect("session id"),
        &scenes::CameraRecipe::new(Vec3::new(0.0, 4.5, 7.0), Vec3::ZERO),
        (512, 512),
        scene_runtime::Settle::ReadinessThenFrames(24),
        true,
        660,
    )
    .expect("capture")
}

/// 各区域的差分像素数（左半 / 右半 / 上三分之一——caster 自身区域）。
fn diff_regions(
    a: &golden_harness::CapturedImage,
    b: &golden_harness::CapturedImage,
) -> (usize, usize, usize) {
    let mut left = 0;
    let mut right = 0;
    let mut top = 0;
    for y in 0..512 {
        for x in 0..512 {
            let o = (y * 512 + x) * 4;
            if a.data[o..o + 4] != b.data[o..o + 4] {
                if y < 170 {
                    top += 1;
                } else if x < 256 {
                    left += 1;
                } else {
                    right += 1;
                }
            }
        }
    }
    (left, right, top)
}

#[test]
#[ignore = "requires GPU; run with/without --features shadow-casting"]
fn hero_casts_shadow_only_with_feature() {
    let mut app = build_app();
    let on = capture_once(&mut app, "shadow-ab-on");
    {
        // 同一会话内关闭阴影贴图（A/B 对照，隔离阴影贡献）。
        let mut lights = app.world_mut().query::<&mut DirectionalLight>();
        for mut light in lights.iter_mut(app.world_mut()) {
            light.shadow_maps_enabled = false;
        }
    }
    let off = capture_once(&mut app, "shadow-ab-off");

    let (left, right, top) = diff_regions(&on, &off);
    let stats = golden_harness::diff_rgba8(&on.data, &off.data).expect("same size");
    eprintln!(
        "shadow A/B (feature={SHADOW_CASTING}): {stats:?}; regions left={left} right={right} top={top}"
    );

    if SHADOW_CASTING {
        assert!(
            left > 500,
            "hero must cast a measurable shadow on the left ground: {left}"
        );
        assert!(
            right == 0,
            "cel control never casts (right side must stay clean): {right}"
        );
        assert_eq!(
            top, 0,
            "the caster's own pixels are unchanged: casting never shades the caster"
        );
    } else {
        assert_eq!(
            stats.differing_pixels, 0,
            "without the feature, toggling shadow maps must be a no-op (byte-identical): {stats:?}"
        );
    }
}
