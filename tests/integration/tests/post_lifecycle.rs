//! High-level post controls must create a real pass and remove it when disabled.
#[path = "support/render_app.rs"]
mod render_app;
use bevy::prelude::*;
use bevy_stylized::prelude::*;
use golden_harness::{CapturedImage, capture};
use render_app::headless_app;
const SIZE: u32 = 256;

fn frame(app: &mut App) -> CapturedImage {
    scene_runtime::capture_in_app(
        app,
        &scenario_model::ident::SessionId::new("post-lifecycle").expect("session id"),
        &scenes::CameraRecipe::new(Vec3::new(0., 0., 5.), Vec3::ZERO),
        (SIZE, SIZE),
        scene_runtime::Settle::Frames(32),
        false,
        640,
    )
    .expect("post capture timeout")
}

fn camera(app: &mut App) -> Entity {
    app.world_mut()
        .resource_scope(|_world, cycle: Mut<scene_runtime::CycleResource>| {
            cycle.observation_entity().expect("observation camera")
        })
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
    let original = frame(&mut app);
    let camera = camera(&mut app);
    app.world_mut()
        .entity_mut(camera)
        .insert(StylizedPostProcess {
            vignette_strength: 0.8,
            vignette_radius: 0.5,
            vignette_softness: 0.3,
            chromatic_aberration: 0.,
            display_transform: StylizedDisplayTransform::None,
        });
    let enabled = frame(&mut app);
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
    let disabled = frame(&mut app);
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
    assert_eq!(enabled.data, frame(&mut app).data);
    app.world_mut()
        .entity_mut(camera)
        .remove::<StylizedPostProcess>();
    assert_eq!(original.data, frame(&mut app).data);
    assert!(
        app.world()
            .get::<StylizedFullscreenEffect>(camera)
            .is_none()
    );
}
