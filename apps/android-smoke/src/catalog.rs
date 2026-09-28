//! Catalog mode: one catalog scene per process launch, driven by a control
//! file, captured through the shared capture contract and reported
//! structurally for adb pull. Sibling of the browser host
//! (`apps/browser-showcase`): the same scene bindings, view contract and
//! per-frame CaseDriver pumping (R4: fresh process per scene).
//!
//! 控制通道：runner 将场景 ID 写入应用 files 目录的 `catalog-scene.txt`
//! （sdcard FUSE 允许 shell 写入，应用自身可读）；应用读取后立即删除，
//! 过期控制文件绝不驱动下一次启动。完成后报告与捕获落盘并退出进程。
use bevy::{ecs as bevy_ecs, prelude::*, render::renderer::RenderAdapterInfo};
use bevy_stylized::prelude::*;
use scene_runtime::{CaseDriver, DriverLimits, DriverStatus};
use serde_json::json;

/// 控制/输出目录（应用外部 files 目录；shell 可写、应用可读写、adb pull 可取）。
pub const FILES_DIR: &str = "/sdcard/Android/data/org.bevystylized.smoke/files";
const CONTROL_FILE: &str = "catalog-scene.txt";

/// 读取并删除一次性控制文件；无文件 = 程序化 fixture 模式（现状不变）。
pub fn requested_scene() -> Option<String> {
    let path = std::path::Path::new(FILES_DIR).join(CONTROL_FILE);
    let scene = std::fs::read_to_string(path).ok()?.trim().to_owned();
    let _ = std::fs::remove_file(std::path::Path::new(FILES_DIR).join(CONTROL_FILE));
    if scene.is_empty() { None } else { Some(scene) }
}

/// 逐帧泵送状态（资源；R1：宿主拥有事件循环，driver 每帧推进一步）。
#[derive(Resource)]
pub struct CatalogHost {
    driver: Option<Box<CaseDriver>>,
    scene: String,
    sent: bool,
}

pub fn build_driver(world: &mut World) {
    let scene = world.resource::<CatalogHost>().scene.clone();
    let Some(case) = showcase::all_cases()
        .into_iter()
        .find(|case| case.name == scene.as_str())
    else {
        report_and_exit(
            &scene,
            json!({"status":"FAIL","reason":"scene is not a registered catalog case"}),
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
        .expect("catalog case must be registered in scenes");
    match CaseDriver::new(
        world,
        scenario_model::ident::SessionId::new("android-catalog").expect("session id"),
        binding,
        showcase::capture_profile_graph(),
        DriverLimits {
            implicit_readiness: false,
            ..DriverLimits::default()
        },
    ) {
        Ok(driver) => world.resource_mut::<CatalogHost>().driver = Some(Box::new(driver)),
        Err(error) => report_and_exit(
            &scene,
            json!({"status":"FAIL","reason":format!("invalid capture plan: {error}")}),
        ),
    }
}

fn report_and_exit(scene: &str, payload: serde_json::Value) -> ! {
    write_report(scene, &payload);
    std::process::exit(0);
}

fn write_report(scene: &str, payload: &serde_json::Value) {
    let dir = std::path::Path::new(FILES_DIR);
    let _ = std::fs::create_dir_all(dir);
    let candidate = dir.join(format!("catalog-{scene}.candidate.json"));
    let final_path = dir.join(format!("catalog-{scene}.json"));
    let bytes = serde_json::to_vec_pretty(payload).expect("report serializes");
    // 候选 → 原子发布（部分输出不算完成）。
    std::fs::write(&candidate, &bytes).expect("write report candidate");
    std::fs::rename(&candidate, &final_path).expect("publish report");
    info!("ANDROID_CATALOG report_ready scene={scene}");
}

pub fn pump(world: &mut World) {
    let mut host = world
        .remove_resource::<CatalogHost>()
        .expect("catalog host");
    if host.sent {
        world.insert_resource(host);
        return;
    }
    let Some(mut driver) = host.driver.take() else {
        world.insert_resource(host);
        return;
    };
    match driver.update(world) {
        DriverStatus::Running => host.driver = Some(driver),
        DriverStatus::Done => {
            let Some(receipt) = driver.drain_observations().pop() else {
                host.sent = true;
                let scene = host.scene.clone();
                world.insert_resource(host);
                report_and_exit(
                    &scene,
                    json!({"status":"FAIL","reason":"no observation receipt"}),
                );
            };
            let dir = std::path::Path::new(FILES_DIR);
            let _ = std::fs::create_dir_all(dir);
            let png_candidate = dir.join(format!("catalog-{}.candidate.png", host.scene));
            let png_final = dir.join(format!("catalog-{}.png", host.scene));
            if let Ok(png) = golden_harness::encode_png(&receipt.image) {
                std::fs::write(&png_candidate, png).expect("write capture candidate");
                std::fs::rename(&png_candidate, &png_final).expect("publish capture");
            }
            let adapter = world.resource::<RenderAdapterInfo>();
            let tier = world
                .get_resource::<StylizedResolvedCapabilities>()
                .and_then(|caps| caps.resolved.as_ref().ok().map(|v| format!("{:?}", v.tier)))
                .unwrap_or_else(|| "unknown".into());
            let stamp = receipt.stamp;
            let png_bytes = std::fs::read(&png_final).expect("published capture");
            let scene = host.scene.clone();
            report_and_exit(
                &scene,
                json!({
                    "schema":"bevy_stylized.android-catalog/v1",
                    "status":"PASS",
                    "scene":scene,
                    "backend":format!("{:?}",adapter.backend),
                    "device":format!("{:?}",adapter.0),
                    "driver":format!("{} {}",adapter.driver,adapter.driver_info),
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
                    "capture_fnv1a64":golden_harness::fnv1a64_hex(&png_bytes),
                }),
            );
        }
        DriverStatus::Failed(error) => {
            host.sent = true;
            let scene = host.scene.clone();
            world.insert_resource(host);
            report_and_exit(&scene, json!({"status":"FAIL","reason":error.to_string()}));
        }
    }
    if !host.sent {
        world.insert_resource(host);
    }
}

/// Catalog mode 的宿主装配：目录场景的 tier 覆盖（与 apps/showcase
/// `all_cases` 的 outline-advanced 请求同一档位）。
pub fn configure(app: &mut App, scene: &str) {
    if scene == "outline-advanced" {
        // 必须在 StylizedPlugin 之前（Stage B 在 finish 时读该配置）。
        app.insert_resource(StylizedCapabilityConfig {
            requested_tier: StylizedExecutionTier::Standard,
            ..StylizedCapabilityConfig::default()
        });
    }
    // 与 PC 捕获路径同一背景：程序化 fixture 的 ClearColor 覆盖不进入
    // catalog 模式（Bevy 默认 srgb_u8(43,44,47)）。
    app.insert_resource(ClearColor::default());
    *app.world_mut().resource_mut::<StylizedViewSettings>() =
        StylizedViewSettings(showcase::default_view_descriptor());
    app.insert_resource(CatalogHost {
        driver: None,
        scene: scene.to_owned(),
        sent: false,
    })
    .add_systems(Startup, build_driver)
    .add_systems(Update, pump);
}
