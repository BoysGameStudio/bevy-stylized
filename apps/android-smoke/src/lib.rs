//! Android Vulkan fixture with procedural and compiled-character modes.
#![cfg(target_os = "android")]

#[cfg(feature = "character")]
mod character;

mod catalog;
mod driver_host;

#[cfg(not(feature = "character"))]
use bevy::{
    camera::ScalingMode, core_pipeline::tonemapping::Tonemapping,
    render::renderer::RenderAdapterInfo,
};
use bevy::{
    // Bevy derives do not discover dependencies in Cargo target tables.
    ecs as bevy_ecs,
    prelude::*,
    render::{
        RenderPlugin,
        settings::{Backends, RenderCreation, WgpuSettings},
    },
    window::AppLifecycle,
};
use bevy_stylized::prelude::*;

#[bevy_main]
pub fn main() {
    let plugins = DefaultPlugins.build().set(RenderPlugin {
        render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
            backends: Some(Backends::VULKAN),
            ..default()
        })),
        ..default()
    });
    #[cfg(feature = "character")]
    let plugins = plugins
        .set(bevy_stylized::stylized_gltf_plugin())
        .set(AssetPlugin {
            meta_check: bevy::asset::AssetMetaCheck::Never,
            ..default()
        });
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.08, 0.10, 0.15)))
        .add_plugins(plugins)
        // courtyard 角色台的 NPR 材质 adoption（05 §4；程序化 fixture 不
        // 使用 adoption，加挂无副作用）。
        .add_plugins((
            StylizedPlugin::default(),
            bevy_stylized::StylizedAssetIntegrationPlugin,
        ));
    // harness 中性探针（driver 的捕获/就绪前置；R1：平台事件循环不变）。
    golden_harness::capture::install(&mut app);
    golden_harness::readiness::install(&mut app);
    // Catalog mode（runner 控制文件驱动）：每进程一个目录场景，共享捕获
    // 合同；缺省 = 既有程序化/角色 fixture 与重放合同不变。
    // 控制文件读取一次性消耗（读后删除）；只读一次并复用结果。
    let catalog_scene = catalog::requested_capture();
    let catalog_active = catalog_scene.is_some();
    if let Some(scene) = catalog_scene {
        catalog::configure(&mut app, &scene);
    } else {
        app.init_resource::<driver_host::ReplayHost>().add_systems(
            Update,
            (input_and_lifecycle, heartbeat, driver_host::pump_replay),
        );
    }
    #[cfg(not(feature = "character"))]
    if !catalog_active {
        app.add_systems(Startup, setup);
    }
    #[cfg(feature = "character")]
    app.init_resource::<character::CharacterState>()
        .add_systems(Startup, character::setup)
        .add_systems(Update, character::check_loaded);
    app.run();
}

#[derive(Component)]
struct Sample;

#[cfg(not(feature = "character"))]
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard: ResMut<Assets<StandardMaterial>>,
    mut cel: ResMut<Assets<StylizedCelMaterial>>,
    mut character: ResMut<Assets<StylizedCharacterMaterial>>,
    mut hero: ResMut<Assets<StylizedHeroMaterial>>,
    mut outlines: ResMut<Assets<StylizedOutlineMaterial>>,
    adapter: Res<RenderAdapterInfo>,
    caps: Res<StylizedResolvedCapabilities>,
) {
    info!(
        "ANDROID_SMOKE adapter={:?} capabilities={:?}",
        adapter.0, caps
    );
    let sphere = meshes.add(Sphere::new(0.65));
    let outline = outlines.add(StylizedOutlineMaterial::default());
    commands.spawn((
        Sample,
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(standard.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.8, 0.8),
            ..default()
        })),
        Transform::from_xyz(-0.9, 1.1, 0.0).with_rotation(Quat::from_rotation_y(0.4)),
    ));
    let cel_params = StylizedCelParams {
        base_color_factor: Vec4::new(0.15, 0.5, 0.95, 1.0),
        ..default()
    };
    commands.spawn((
        Sample,
        Mesh3d(sphere.clone()),
        MeshMaterial3d(cel.add(StylizedCelMaterial {
            params: cel_params,
            ..default()
        })),
        StylizedOutline::new(outline.clone()),
        Transform::from_xyz(0.9, 1.1, 0.0),
    ));
    let character_params = StylizedCharacterParams {
        cel: StylizedCelParams {
            base_color_factor: Vec4::new(0.9, 0.25, 0.12, 1.0),
            ..default()
        },
        ..default()
    };
    commands.spawn((
        Sample,
        Mesh3d(sphere.clone()),
        MeshMaterial3d(character.add(StylizedCharacterMaterial {
            params: character_params,
            ..default()
        })),
        StylizedOutline::new(outline.clone()),
        Transform::from_xyz(-0.9, -1.1, 0.0),
    ));
    let hero_params = StylizedHeroParams {
        character: StylizedCharacterParams {
            cel: StylizedCelParams {
                base_color_factor: Vec4::new(0.2, 0.8, 0.3, 1.0),
                ..default()
            },
            ..default()
        },
        ..default()
    };
    commands.spawn((
        Sample,
        Mesh3d(sphere),
        MeshMaterial3d(hero.add(StylizedHeroMaterial {
            params: hero_params,
            ..default()
        })),
        StylizedOutline::new(outline),
        Transform::from_xyz(0.9, -1.1, 0.0),
    ));
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 5.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 8.0,
            },
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(0.0, 0.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        Msaa::Off,
        Tonemapping::None,
    ));
    info!(
        "ANDROID_SMOKE scene_spawned standard=1 cel=1 character=1 hero=1 hull_requested=3 msaa=off"
    );
}

fn input_and_lifecycle(
    touches: Res<Touches>,
    mut samples: Query<&mut Transform, With<Sample>>,
    mut lifecycle: MessageReader<AppLifecycle>,
) {
    for event in lifecycle.read() {
        info!("ANDROID_SMOKE lifecycle={event:?}");
    }
    if touches.any_just_pressed() {
        for mut transform in &mut samples {
            #[cfg(not(feature = "character"))]
            transform.rotate_z(0.2);
            #[cfg(feature = "character")]
            transform.rotate_y(0.2);
        }
        info!("ANDROID_SMOKE touch_received");
    }
}

fn heartbeat(mut frame: Local<u64>, outlines: Query<(), With<StylizedOutlineReplica>>) {
    *frame += 1;
    if *frame == 120 || (*frame).is_multiple_of(600) {
        // Update progress is deliberately not a GPU timing or a visual PASS.
        info!(
            "ANDROID_SMOKE update_frames={} hull_entities={}",
            *frame,
            outlines.iter().count()
        );
    }
}
