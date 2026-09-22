use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    image::Image,
    math::Vec3,
    mesh::Mesh,
    pbr::MeshMaterial3d,
    prelude::{Mesh3d, Transform},
    render::{
        RenderPlugin,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        settings::RenderCreation,
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::*;
use golden_harness::capture;

/// Alpha mask 契约的 GPU 证据：quad 上左半 alpha=255 / 右半 0，
/// cutoff 0.5 → 右半必须被 discard（露背景）。
#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn alpha_mask_cutout_probe() {
    render_mask(0, 0);
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn native_mask_prepass_preserves_cutouts_for_all_classes() {
    for class in 0..3 {
        let forward = render_mask(class, 0);
        for mode in [1, 2] {
            let prepass = render_mask(class, mode);
            assert_eq!(
                forward.data, prepass.data,
                "class {class}: prepass {mode} must preserve cutouts and shading"
            );
        }
    }
}

fn render_mask(class: u8, prepass: u8) -> golden_harness::CapturedImage {
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
        let mut view = app.world_mut().resource_mut::<StylizedViewSettings>();
        view.0.key_light_direction = Vec3::Z;
        view.0.ambient_override = Some(bevy::color::LinearRgba::WHITE);
    }

    // 64x64: alpha=255 左半, 0 右半
    let mut data = Vec::new();
    for _y in 0..64 {
        for x in 0..64 {
            let a = if x < 32 { 255u8 } else { 0u8 };
            data.extend_from_slice(&[200, 100, 100, a]);
        }
    }
    let mut image = Image::new_fill(
        Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    );
    image.data = Some(data);
    let base = app.world_mut().resource_mut::<Assets<Image>>().add(image);

    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(bevy::math::primitives::Rectangle::new(2.0, 2.0));
    let params = StylizedCelParams::default().with_alpha_cutoff(0.5);
    let entity = app
        .world_mut()
        .spawn((Mesh3d(mesh.clone()), Transform::IDENTITY))
        .id();
    match class {
        0 => {
            let material = app
                .world_mut()
                .resource_mut::<Assets<StylizedCelMaterial>>()
                .add(StylizedCelMaterial {
                    params,
                    base_color_texture: Some(base.clone()),
                    ..Default::default()
                });
            app.world_mut()
                .entity_mut(entity)
                .insert(MeshMaterial3d(material));
        }
        1 => {
            let mut material = StylizedCharacterMaterial {
                base_color_texture: Some(base.clone()),
                ..Default::default()
            };
            material.params.cel = params;
            let material = app
                .world_mut()
                .resource_mut::<Assets<StylizedCharacterMaterial>>()
                .add(material);
            app.world_mut()
                .entity_mut(entity)
                .insert(MeshMaterial3d(material));
        }
        _ => {
            let mut material = StylizedHeroMaterial {
                base_color_texture: Some(base.clone()),
                ..Default::default()
            };
            material.params.character.cel = params;
            let material = app
                .world_mut()
                .resource_mut::<Assets<StylizedHeroMaterial>>()
                .add(material);
            app.world_mut()
                .entity_mut(entity)
                .insert(MeshMaterial3d(material));
        }
    }
    let mut background = StylizedCelMaterial::default();
    background.params.base_color_factor = bevy::math::Vec4::new(0., 1., 0., 1.);
    let background = app
        .world_mut()
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(background);
    app.world_mut().spawn((
        Mesh3d(mesh),
        MeshMaterial3d(background),
        Transform::from_xyz(0., 0., -0.1),
    ));
    // 读回：确认 alpha 数据在资产内正确。
    {
        let img = app
            .world()
            .resource::<Assets<Image>>()
            .get(&base)
            .expect("image");
        let data = img.data.as_ref().expect("data");
        let right_alpha = data[(32 * 64 + 40) * 4 + 3];
        assert_eq!(right_alpha, 0, "texture alpha must be 0 on the right half");
    }
    // 经 CaseDriver 观测周期捕获（readiness 就绪 + 24 帧暖场；prepass 变体
    // 经相机配方的 Prepass 附件声明，与旧逐相机 insert 等价）。
    let attachment = scenes::CameraAttachment::Prepass {
        depth: prepass >= 1,
        normal: prepass >= 2,
        motion_vector: false,
    };
    let frame = scene_runtime::capture_in_app(
        &mut app,
        &scenario_model::ident::SessionId::new("alpha-probe").expect("session id"),
        &scenes::CameraRecipe::with_attachment(Vec3::new(0.0, 0.0, 3.0), Vec3::ZERO, attachment),
        (256, 256),
        scene_runtime::Settle::ReadinessThenFrames(24),
        true,
        660,
    )
    .expect("capture");
    // 左半（alpha=255）= 纹理红；右半（alpha=0 < cutoff）= 被 discard 露背景。
    let px = |x: usize, y: usize| {
        let o = (y * 256 + x) * 4;
        [frame.data[o], frame.data[o + 1], frame.data[o + 2]]
    };
    let left = px(80, 128);
    let right = px(180, 128);
    assert!(
        left[0] > 150 && left[1] < 150,
        "class {class}, prepass {prepass}: left half shows texture: {left:?}"
    );
    assert!(
        right[0] < 80 && right[1] > 150,
        "class {class}, prepass {prepass}: right half must reveal the green surface behind it: {right:?}"
    );
    frame
}
