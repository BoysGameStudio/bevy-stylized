//! Browser (WebGPU) showcase host：courtyard 捕获规格经 CaseDriver 逐帧泵送
//! 执行（R1：事件循环归宿主；每帧一次 `driver.update`，无忙等），产出与 PC
//! 捕获路径（`showcase::capture_spec`）同一动作图（`fixed_step{22} →
//! observe`）的离屏捕获，供 lane 与 canonical golden 基线逐捕获判定
//! （exact 优先；分歧走 docs/reference/08 §5 envelope，仅此域）。
//!
//! 捕获选择经 URL 查询参数（`?spec=<capture_id>`，捕获规格唯一键，场景由
//! 规格携带）由驱动脚本指定；结果经 `globalThis.stylizedResult` 发布
//! （与 tests/browser smoke 同一模式），RGBA 载荷由 lane 分块取回。
//!
//! 资产边界：courtyard 的 reference-scene 依赖由 lane 经 HTTP 提供
//! （`assets/reference-scene/`）。第三方编译产物不进入本宿主。
use bevy::{
    DefaultPlugins,
    app::PluginGroup,
    asset::AssetPlugin,
    prelude::*,
    render::{RenderPlugin, renderer::RenderAdapterInfo},
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::*;
use golden_harness::capture;
use scene_runtime::{CaseDriver, DriverLimits, DriverStatus};
use serde_json::json;

/// 捕获分辨率（与 PC 捕获路径一致）。
const CAPTURE_SIZE: u32 = showcase::CAPTURE_SIZE;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(
    inline_js = "export function publish(value) { const result=JSON.parse(value); if(result.status==='LOADING'){globalThis.stylizedProgress=result;return;} globalThis.stylizedResult=result; document.querySelector('#status').textContent=result.status; } export function current_spec() { return new URLSearchParams(location.search).get('spec') || ''; }"
)]
extern "C" {
    fn publish(value: &str);
    fn current_spec() -> String;
}
#[cfg(not(target_arch = "wasm32"))]
fn publish(value: &str) {
    std::fs::write(
        std::env::var("STYLIZED_CAPTURE_OUT").expect("capture path"),
        value,
    )
    .expect("write capture");
}
#[cfg(not(target_arch = "wasm32"))]
fn current_spec() -> String {
    std::env::var("STYLIZED_BROWSER_SPEC").unwrap_or_default()
}

/// 宿主侧资产根：courtyard（捕获规格的 scene）依赖 reference-scene
/// fixture；wasm 侧经 HTTP 提供，本机侧直接读仓库 fixture。
fn host_asset_root(scene: &str) -> Option<&'static str> {
    #[cfg(target_arch = "wasm32")]
    if scene == "courtyard" {
        return Some("assets/reference-scene");
    }
    #[cfg(not(target_arch = "wasm32"))]
    if scene == "courtyard" {
        return Some(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/synthetic/reference-scene"
        ));
    }
    let _ = scene;
    None
}

/// 逐帧泵送状态（R1：宿主拥有事件循环；driver 每帧最多推进一步）。
#[derive(Resource)]
struct Host {
    driver: Option<Box<CaseDriver>>,
    spec: String,
    sent: bool,
}

fn main() {
    let spec_id = current_spec();
    let Some(spec) = scenario_model::courtyard_capture_specs()
        .into_iter()
        .find(|spec| spec.capture_id == spec_id)
    else {
        publish(
            &json!({"status":"FAIL","reason":"unknown or missing ?spec= capture id","spec":spec_id})
                .to_string(),
        );
        return;
    };
    let mut app = App::new();
    if let Some(tier) = spec.tier {
        app.insert_resource(StylizedCapabilityConfig {
            requested_tier: tier,
            ..StylizedCapabilityConfig::default()
        });
    }
    app.add_plugins(
        DefaultPlugins
            .build()
            .set(AssetPlugin {
                file_path: host_asset_root(spec.scene.as_str())
                    .unwrap_or("assets")
                    .to_string(),
                meta_check: bevy::asset::AssetMetaCheck::Never,
                ..Default::default()
            })
            .set(RenderPlugin {
                synchronous_pipeline_compilation: true,
                ..Default::default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Stylized browser showcase".into(),
                    resolution: bevy::window::WindowResolution::new(CAPTURE_SIZE, CAPTURE_SIZE)
                        .with_scale_factor_override(1.0),
                    canvas: Some("#scene".into()),
                    fit_canvas_to_parent: false,
                    ..Default::default()
                }),
                exit_condition: ExitCondition::DontExit,
                ..Default::default()
            }),
    );
    app.add_plugins((
        StylizedPlugin::default(),
        bevy_stylized::StylizedAssetIntegrationPlugin,
    ));
    capture::install(&mut app);
    *app.world_mut().resource_mut::<StylizedViewSettings>() =
        StylizedViewSettings(showcase::default_view_descriptor());
    // 捕获规格的参数钉值在构建前落入 CourtyardSettings（builder 读取）。
    app.insert_resource(showcase::capture_spec::settings_from_spec(&spec));
    app.insert_resource(Host {
        driver: None,
        spec: spec.capture_id.clone(),
        sent: false,
    });
    // No explicit finish/cleanup: on wasm the render device initializes
    // asynchronously and the winit runner finishes the app at the right time
    // (an explicit finish would take the resource mutex across the async
    // boundary and panic under no_threads).
    app.add_systems(Startup, build_driver);
    app.add_systems(Update, pump);
    app.run();
}

fn build_driver(world: &mut World) {
    let spec_id = world.resource::<Host>().spec.clone();
    let spec = scenario_model::courtyard_capture_specs()
        .into_iter()
        .find(|spec| spec.capture_id == spec_id)
        .expect("validated in main");
    let binding = scenes::registry::bindings()
        .into_iter()
        .find(|binding| binding.id == spec.scene.as_str())
        .expect("capture spec scene must be registered in scenes");
    // 机位 = 规格的机位 id 对应的相机配方覆盖（附件由参数钉值驱动，
    // 与 PC 捕获路径同一规则）。
    let settings = world
        .get_resource::<scenes::CourtyardSettings>()
        .copied()
        .expect("settings inserted in main");
    let recipe = showcase::capture_spec::camera_recipe_for_spec(&spec, &settings);
    let driver = CaseDriver::new(
        world,
        scenario_model::ident::SessionId::new("browser-showcase").expect("session id"),
        binding,
        showcase::capture_profile_graph(),
        DriverLimits {
            implicit_readiness: false,
            ..DriverLimits::default()
        },
    )
    .expect("capture profile must be a valid plan")
    .with_camera_recipe(recipe);
    world.resource_mut::<Host>().driver = Some(Box::new(driver));
}

fn pump(world: &mut World) {
    let mut host = world.remove_resource::<Host>().expect("host resource");
    if host.sent {
        world.insert_resource(host);
        return;
    }
    let Some(mut driver) = host.driver.take() else {
        world.insert_resource(host);
        return;
    };
    match driver.update(world) {
        DriverStatus::Running => {
            host.driver = Some(driver);
        }
        DriverStatus::Done => {
            let Some(receipt) = driver.drain_observations().pop() else {
                publish(
                    &json!({"status":"FAIL","spec":host.spec,"reason":"no observation receipt"})
                        .to_string(),
                );
                host.sent = true;
                world.insert_resource(host);
                return;
            };
            let adapter = world.resource::<RenderAdapterInfo>();
            let tier = world
                .get_resource::<StylizedResolvedCapabilities>()
                .and_then(|caps| caps.resolved.as_ref().ok().map(|v| format!("{:?}", v.tier)))
                .unwrap_or_else(|| "unknown".into());
            let stamp = receipt.stamp;
            publish(
                &json!({
                    "schema":"bevy_stylized.browser-showcase/v1",
                    "status":"PASS",
                    "spec":host.spec.clone(),
                    "backend":format!("{:?}",adapter.backend),
                    "device":format!("{:?}",adapter.0),
                    "resolved_tier":tier,
                    "session_id":receipt.session.to_string(),
                    "generation":receipt.generation.get(),
                    "experiment_tick":receipt.experiment_tick,
                    "stamp":{
                        "sequence":stamp.sequence,
                        "issued_update_index":stamp.issued_update_index,
                        "completed_update_index":stamp.completed_update_index,
                        "completed_render_frame":stamp.completed_render_frame,
                    },
                    "engine_git_commit":golden_harness::engine_git_commit(),
                    "width":receipt.image.width,
                    "height":receipt.image.height,
                    "rgba":receipt.image.data,
                })
                .to_string(),
            );
            host.sent = true;
            #[cfg(not(target_arch = "wasm32"))]
            world.write_message(bevy::app::AppExit::Success);
        }
        DriverStatus::Failed(error) => {
            publish(
                &json!({"status":"FAIL","spec":host.spec.clone(),"reason":error.to_string()})
                    .to_string(),
            );
            host.sent = true;
        }
    }
    world.insert_resource(host);
}
