//! Standalone native/WebGPU consumer. Inputs are supplied by the local runner.
use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    gltf::GltfAssetLabel,
    prelude::*,
    render::{
        renderer::RenderAdapterInfo,
        view::screenshot::{Screenshot, ScreenshotCaptured},
    },
};
use bevy_stylized::{
    StylizedAssetIntegrationPlugin, StylizedMaterialAdoption, StylizedMaterialSet, prelude::*,
};
use golden_harness::readiness;
use serde_json::json;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(
    inline_js = "export function publish(value) { const result=JSON.parse(value); if(result.status==='LOADING'){globalThis.stylizedProgress=result;return;} globalThis.stylizedResult=result; document.querySelector('#status').textContent=result.status; }"
)]
extern "C" {
    fn publish(value: &str);
}
#[cfg(not(target_arch = "wasm32"))]
fn publish(value: &str) {
    std::fs::write(
        std::env::var("STYLIZED_CAPTURE_OUT").expect("capture path"),
        value,
    )
    .expect("write capture");
}

#[derive(Resource, Default)]
struct Evidence {
    images: Vec<Handle<Image>>,
    scene: Handle<bevy::world_serialization::WorldAsset>,
    stable: u32,
    last_frame: u64,
    sent: bool,
    adopted: usize,
    outlines: usize,
}

fn main() {
    let mut app = App::new();
    let plugins = DefaultPlugins
        .build()
        .set(bevy_stylized::stylized_gltf_plugin())
        .set(AssetPlugin {
            file_path: env!("STYLIZED_BROWSER_ASSETS").into(),
            meta_check: bevy::asset::AssetMetaCheck::Never,
            ..default()
        })
        .set(WindowPlugin {
            primary_window: Some(Window {
                title: "Stylized backend validation".into(),
                resolution: bevy::window::WindowResolution::new(512, 512)
                    .with_scale_factor_override(1.0),
                canvas: Some("#scene".into()),
                fit_canvas_to_parent: false,
                ..default()
            }),
            ..default()
        });
    #[cfg(not(target_arch = "wasm32"))]
    let plugins = {
        let mut plugins = plugins;
        if plugins.contains::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>() {
            plugins =
                plugins.disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>();
        }
        plugins
    };
    app.add_plugins(plugins)
        .add_plugins((StylizedPlugin::default(), StylizedAssetIntegrationPlugin));
    readiness::install(&mut app);
    app.insert_resource(ClearColor(Color::linear_rgb(0.12, 0.12, 0.12)))
        .init_resource::<Evidence>()
        .add_systems(Startup, setup)
        .add_systems(Update, check_ready)
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut evidence: ResMut<Evidence>,
    mut view: ResMut<StylizedViewSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cel: ResMut<Assets<StylizedCelMaterial>>,
    mut character: ResMut<Assets<StylizedCharacterMaterial>>,
    mut hero: ResMut<Assets<StylizedHeroMaterial>>,
    mut outline: ResMut<Assets<StylizedOutlineMaterial>>,
) {
    let sidecar = bevy_stylized::stylized_gltf::parse_sidecar_str(include_str!(env!(
        "STYLIZED_BROWSER_SIDECAR"
    )))
    .expect("verified sidecar");
    view.0 =
        serde_json::from_str(include_str!(env!("STYLIZED_BROWSER_VIEW"))).expect("verified view");
    evidence.scene = server.load(GltfAssetLabel::Scene(0).from_asset("character/character.glb"));
    commands.spawn((
        bevy::world_serialization::WorldAssetRoot(evidence.scene.clone()),
        Visibility::default(),
        Transform::from_xyz(-0.35, 0., 0.),
        StylizedMaterialSet {
            materials: sidecar.materials,
            texture_prefix: "character".into(),
        },
    ));
    commands.spawn((
        Camera3d::default(),
        Tonemapping::None,
        Msaa::Sample4,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 2.1,
            },
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(0., 0.8, 5.).looking_at(Vec3::new(0., 0.8, 0.), Vec3::Y),
    ));
    let mesh = meshes.add(Sphere::new(0.18));
    let mut c = StylizedCelMaterial::default();
    c.params.base_color_factor = Vec4::new(1., 0.02, 0.02, 1.);
    commands.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(cel.add(c)),
        Transform::from_xyz(0.8, 0.3, 0.),
    ));
    let mut c = StylizedCharacterMaterial::default();
    c.params.cel.base_color_factor = Vec4::new(0.02, 1., 0.02, 1.);
    commands.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(character.add(c)),
        Transform::from_xyz(0.8, 0.8, 0.),
    ));
    let mut h = StylizedHeroMaterial::default();
    h.params.character.cel.base_color_factor = Vec4::new(0.02, 0.02, 1., 1.);
    let hull = outline.add(StylizedOutlineMaterial::default());
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(hero.add(h)),
        StylizedOutline::new(hull),
        Transform::from_xyz(0.8, 1.3, 0.),
    ));
}

fn check_ready(
    mut commands: Commands,
    server: Res<AssetServer>,
    probe: Res<readiness::Probe>,
    mut state: ResMut<Evidence>,
    adopted: Query<&StylizedMaterialAdoption>,
    outlines: Query<&StylizedOutlineReplica>,
    materials: Res<Assets<StylizedHeroMaterial>>,
) {
    if state.sent {
        return;
    }
    let snapshot = probe.snapshot();
    if let Some(error) = snapshot.error {
        publish(&json!({"status":"FAIL","reason":error}).to_string());
        state.sent = true;
        return;
    }
    for row in &adopted {
        if let Some(error) = &row.error {
            publish(&json!({"status":"FAIL","reason":error}).to_string());
            state.sent = true;
            return;
        }
    }
    let expected: usize = env!("STYLIZED_BROWSER_PRIMITIVES")
        .parse()
        .expect("primitive count");
    if state.images.is_empty() && adopted.iter().count() == expected {
        // Respect the descriptor factory's linear/sRGB loader settings. A plain
        // preload before adoption can claim the URI with the wrong colorspace.
        state.images = materials
            .iter()
            .flat_map(|(_, m)| {
                [
                    m.base_color_texture.clone(),
                    m.ilm_texture.clone(),
                    m.face_sdf_texture.clone(),
                    m.ramp_texture.clone(),
                    m.packed_normal_texture.clone(),
                    m.matcap_texture.clone(),
                    m.spec_ramp_texture.clone(),
                    m.surface_control_texture.clone(),
                ]
                .into_iter()
                .flatten()
            })
            .collect();
    }
    let ready = snapshot.ready
        && adopted.iter().count() == expected
        && server.is_loaded_with_dependencies(state.scene.id())
        && state
            .images
            .iter()
            .all(|h| server.is_loaded_with_dependencies(h.id()));
    if snapshot.frame != state.last_frame {
        state.last_frame = snapshot.frame;
        state.stable = if ready { state.stable + 1 } else { 0 };
    }
    if snapshot.frame % 30 == 0 {
        publish(&json!({"status":"LOADING","frame":snapshot.frame,"pipelines":snapshot.ready,"adopted":adopted.iter().count(),"expected":expected,"scene_loaded":server.is_loaded_with_dependencies(state.scene.id()),"images":state.images.len(),"images_loaded":state.images.iter().filter(|h|server.is_loaded_with_dependencies(h.id())).count(),"stable":state.stable}).to_string());
    }
    if state.stable >= 32 {
        state.sent = true;
        state.adopted = adopted.iter().count();
        state.outlines = outlines.iter().count();
        commands
            .spawn(Screenshot::primary_window())
            .observe(captured);
    }
}
fn captured(
    event: On<ScreenshotCaptured>,
    state: Res<Evidence>,
    adapter: Res<RenderAdapterInfo>,
    mut exit: MessageWriter<AppExit>,
) {
    let image = event
        .image
        .clone()
        .try_into_dynamic()
        .expect("RGBA screenshot")
        .to_rgba8();
    let value = json!({"schema":"bevy_stylized.browser-smoke/v1","status":"PASS","backend":format!("{:?}",adapter.backend),
        "device":format!("{:?}",adapter.0),"width":image.width(),"height":image.height(),"rgba":image.as_raw(),
        "adopted_primitives":state.adopted,"outline_primitives":state.outlines,"stable_render_frames":state.stable,
        "classes":["Cel","Character","Hero"],"scope":"same fixture native/browser pipeline and screenshot evidence"});
    publish(&value.to_string());
    #[cfg(not(target_arch = "wasm32"))]
    exit.write(AppExit::Success);
    #[cfg(target_arch = "wasm32")]
    let _ = &mut exit;
}
