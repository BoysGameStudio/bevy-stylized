//! Browser (WebGPU) showcase host：全部 18 个注册场景经 CaseDriver 逐帧泵送
//! 执行（R1：事件循环归宿主；每帧一次 `driver.update`，无忙等），产出与 PC
//! 捕获路径（`showcase::capture_case`）同一动作图（`fixed_step{22} →
//! observe`）的离屏捕获，供 lane 与 canonical golden 基线逐场景判定
//! （exact 优先；分歧走 docs/reference/08 §5 envelope，仅此域）。
//!
//! 场景选择经 URL 查询参数（`?scene=<id>`）由驱动脚本指定；结果经
//! `globalThis.stylizedResult` 发布（与 tests/browser smoke 同一模式），
//! RGBA 载荷由 lane 分块取回。
//!
//! 资产边界：`reference` 场景的第一方 fixture 由 lane 经 HTTP 提供
//! （`assets/reference-scene/`）；其余场景为程序化内容。第三方编译产物
//! 不进入本宿主。
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
    inline_js = "export function publish(value) { const result=JSON.parse(value); if(result.status==='LOADING'){globalThis.stylizedProgress=result;return;} globalThis.stylizedResult=result; document.querySelector('#status').textContent=result.status; } export function current_scene() { return new URLSearchParams(location.search).get('scene') || ''; }"
)]
extern "C" {
    fn publish(value: &str);
    fn current_scene() -> String;
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
fn current_scene() -> String {
    std::env::var("STYLIZED_BROWSER_SCENE").unwrap_or_default()
}

/// 宿主侧场景选择：沿用 showcase `all_cases` 的注册绑定；浏览器侧的资产根
/// 覆盖（reference 的第一方 fixture 经 HTTP 提供）归本宿主记录。
fn host_asset_root(scene: &str) -> Option<&'static str> {
    // 与 apps/showcase `all_cases` 的 reference 资产根同一内容，经 HTTP 提供。
    if scene == "reference" {
        Some("assets/reference-scene")
    } else {
        None
    }
}

/// 逐帧泵送状态（R1：宿主拥有事件循环；driver 每帧最多推进一步）。
#[derive(Resource)]
struct Host {
    driver: Option<Box<CaseDriver>>,
    scene: String,
    sent: bool,
}

fn main() {
    let scene = current_scene();
    if scene.is_empty() {
        publish(&json!({"status":"FAIL","reason":"missing ?scene= query parameter"}).to_string());
        return;
    }
    let tier = registry_tier(&scene);
    let mut app = App::new();
    if let Some(tier) = tier {
        app.insert_resource(StylizedCapabilityConfig {
            requested_tier: tier,
            ..StylizedCapabilityConfig::default()
        });
    }
    app.add_plugins(
        DefaultPlugins
            .build()
            .set(AssetPlugin {
                file_path: host_asset_root(&scene).unwrap_or("assets").to_string(),
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
    app.add_plugins(StylizedPlugin::default());
    capture::install(&mut app);
    *app.world_mut().resource_mut::<StylizedViewSettings>() =
        StylizedViewSettings(showcase::default_view_descriptor());
    app.insert_resource(Host {
        driver: None,
        scene,
        sent: false,
    });
    app.finish();
    app.cleanup();
    app.add_systems(Startup, build_driver);
    app.add_systems(Update, pump);
    app.run();
}

/// `all_cases` 的 tier 覆盖（outline-advanced = Standard）在本宿主的记录。
fn registry_tier(scene: &str) -> Option<StylizedExecutionTier> {
    if scene == "outline-advanced" {
        Some(StylizedExecutionTier::Standard)
    } else {
        None
    }
}

fn build_driver(world: &mut World) {
    let scene = world.resource::<Host>().scene.clone();
    let Some(case) = showcase::all_cases()
        .into_iter()
        .find(|case| case.name == scene.as_str())
    else {
        publish(
            &json!({"status":"FAIL","scene":scene,"reason":"scene is not a registered catalog case"})
                .to_string(),
        );
        return;
    };
    if let Some(view) = case.view {
        let mut settings = world.resource_mut::<StylizedViewSettings>();
        view(&mut settings.0);
    }
    let binding = scenes::registry::bindings()
        .into_iter()
        .find(|binding| binding.id == case.name)
        .expect("showcase case must be registered in scenes");
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
    .expect("capture profile must be a valid plan");
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
                    &json!({"status":"FAIL","scene":host.scene,"reason":"no observation receipt"})
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
                    "scene":host.scene,
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
        }
        DriverStatus::Failed(error) => {
            publish(
                &json!({"status":"FAIL","scene":host.scene,"reason":error.to_string()})
                    .to_string(),
            );
            host.sent = true;
        }
    }
    world.insert_resource(host);
}
