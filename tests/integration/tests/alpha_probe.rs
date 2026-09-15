use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    core_pipeline::tonemapping::Tonemapping,
    image::Image,
    math::Vec3,
    mesh::Mesh,
    pbr::MeshMaterial3d,
    prelude::{Camera3d, Mesh3d, Transform},
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
    let Some(resources) = capture::probe_render_resources() else {
        return;
    };
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
    app.finish();
    app.cleanup();

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
    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial {
            params: StylizedCelParams::default().with_alpha_cutoff(0.5),
            base_color_texture: Some(base.clone()),
            ..StylizedCelMaterial::default()
        });
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
    app.world_mut()
        .spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::IDENTITY));
    let target = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        capture::create_capture_target(&mut images, 256, 256)
    };
    app.world_mut().spawn((
        Camera3d::default(),
        capture::capture_target_bundle(&target),
        Tonemapping::None,
        Transform::from_xyz(0.0, 0.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    for _ in 0..24 {
        app.update();
    }
    let request = capture::request_capture(app.world_mut(), target);
    let mut frame = None;
    for _ in 0..600 {
        app.update();
        if let Some(f) = capture::try_take(app.world_mut(), request.clone()) {
            frame = Some(f);
            break;
        }
    }
    let frame = frame.expect("capture");
    // 左半（alpha=255）= 纹理红；右半（alpha=0 < cutoff）= 被 discard 露背景。
    let px = |x: usize, y: usize| {
        let o = (y * 256 + x) * 4;
        [frame.data[o], frame.data[o + 1], frame.data[o + 2]]
    };
    let left = px(80, 128);
    let right = px(180, 128);
    assert!(
        left[0] > 150 && left[1] < 150,
        "left half shows texture: {left:?}"
    );
    assert!(
        right[0] < 80 && right[1] < 80,
        "right half discarded to background: {right:?}"
    );
}
