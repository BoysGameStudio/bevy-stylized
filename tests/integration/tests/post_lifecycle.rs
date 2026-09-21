//! High-level post controls must create a real pass and remove it when disabled.
#[path = "support/render_app.rs"]
mod render_app;
use bevy::prelude::*;
use bevy_stylized::prelude::*;
use golden_harness::{CapturedImage, capture};
use render_app::headless_app;
const SIZE: u32 = 256;

fn frame(app: &mut App, target: &capture::CaptureTarget) -> CapturedImage {
    for _ in 0..32 {
        app.update();
    }
    let request = capture::request_capture(app.world_mut(), target.clone());
    for _ in 0..600 {
        app.update();
        if let Some(image) = capture::try_take(app.world_mut(), request.clone()) {
            return image;
        }
    }
    panic!("post capture timeout");
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn post_controller_renders_and_disable_remove_restore_pixels() {
    let mut app =
        headless_app(capture::probe_render_resources().expect("required GPU unavailable"));
    {
        let mut view = app.world_mut().resource_mut::<StylizedViewSettings>();
        view.0.key_light_direction = Vec3::Z;
        view.0.ambient_override = Some(LinearRgba::WHITE);
    }
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Rectangle::new(8., 8.));
    let mut m = StylizedCelMaterial::default();
    m.params.base_color_factor = Vec4::new(0.7, 0.7, 0.7, 1.);
    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(m);
    app.world_mut()
        .spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::IDENTITY));
    let target = capture::create_capture_target(
        &mut app.world_mut().resource_mut::<Assets<Image>>(),
        SIZE,
        SIZE,
    );
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            capture::capture_target_bundle(&target),
            bevy::core_pipeline::tonemapping::Tonemapping::None,
            Transform::from_xyz(0., 0., 5.).looking_at(Vec3::ZERO, Vec3::Y),
        ))
        .id();
    let original = frame(&mut app, &target);
    app.world_mut()
        .entity_mut(camera)
        .insert(StylizedPostProcess {
            vignette_strength: 0.8,
            vignette_radius: 0.5,
            vignette_softness: 0.3,
            chromatic_aberration: 0.,
            display_transform: StylizedDisplayTransform::None,
        });
    let enabled = frame(&mut app, &target);
    assert!(
        app.world()
            .get::<StylizedFullscreenEffect>(camera)
            .is_some()
    );
    let corner = (8 * SIZE as usize + 8) * 4;
    assert!(
        i32::from(original.data[corner]) - i32::from(enabled.data[corner]) > 30,
        "high-level controller must execute the post pass"
    );
    app.world_mut()
        .resource_mut::<StylizedViewSettings>()
        .0
        .camera
        .post_enabled = false;
    let disabled = frame(&mut app, &target);
    assert!(
        app.world()
            .get::<StylizedFullscreenEffect>(camera)
            .is_none()
    );
    assert_eq!(
        original.data, disabled.data,
        "disabled post must preserve original pixels"
    );
    app.world_mut()
        .resource_mut::<StylizedViewSettings>()
        .0
        .camera
        .post_enabled = true;
    assert_eq!(enabled.data, frame(&mut app, &target).data);
    app.world_mut()
        .entity_mut(camera)
        .remove::<StylizedPostProcess>();
    assert_eq!(original.data, frame(&mut app, &target).data);
    assert!(
        app.world()
            .get::<StylizedFullscreenEffect>(camera)
            .is_none()
    );
}
