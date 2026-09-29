//! Catalog mode: one capture spec per process launch, driven by a control
//! file, captured through the shared capture contract and reported
//! structurally for adb pull. Sibling of the browser host
//! (`apps/browser-showcase`): the same scene bindings, view contract and
//! per-frame CaseDriver pumping (R4: fresh process per capture).
//!
//! 控制通道：runner 将**捕获规格 ID**（05 §9；场景由规格携带）写入应用
//! files 目录的 `catalog-scene.txt`（sdcard FUSE 允许 shell 写入，应用
//! 自身可读）；应用读取后立即删除，过期控制文件绝不驱动下一次启动。
//! 完成后报告与捕获落盘并退出进程。
use bevy::{ecs as bevy_ecs, prelude::*, render::renderer::RenderAdapterInfo};
use bevy_stylized::prelude::*;
use scene_runtime::{CaseDriver, DriverLimits, DriverStatus};
use serde_json::json;

/// 控制/输出目录（应用外部 files 目录；shell 可写、应用可读写、adb pull 可取）。
pub const FILES_DIR: &str = "/sdcard/Android/data/org.bevystylized.smoke/files";
const CONTROL_FILE: &str = "catalog-scene.txt";

/// 读取并删除一次性控制文件（内容 = 捕获规格 ID）；无文件 = 程序化
/// fixture 模式（现状不变）。
pub fn requested_capture() -> Option<String> {
    let path = std::path::Path::new(FILES_DIR).join(CONTROL_FILE);
    let spec = std::fs::read_to_string(path).ok()?.trim().to_owned();
    let _ = std::fs::remove_file(std::path::Path::new(FILES_DIR).join(CONTROL_FILE));
    if spec.is_empty() { None } else { Some(spec) }
}

/// 逐帧泵送状态（资源；R1：宿主拥有事件循环，driver 每帧推进一步）。
#[derive(Resource)]
pub struct CatalogHost {
    driver: Option<Box<CaseDriver>>,
    spec: String,
    sent: bool,
}

pub fn build_driver(world: &mut World) {
    let spec_id = world.resource::<CatalogHost>().spec.clone();
    let Some(spec) = scenario_model::courtyard_capture_specs()
        .into_iter()
        .find(|spec| spec.capture_id == spec_id)
    else {
        report_and_stay(
            &spec_id,
            json!({"status":"FAIL","reason":"unknown capture spec id"}),
        );
        return;
    };
    let binding = scenes::registry::bindings()
        .into_iter()
        .find(|binding| binding.id == spec.scene.as_str())
        .expect("capture spec scene must be registered in scenes");
    // 机位 = 规格的机位 id 对应的相机配方覆盖（与 PC 捕获路径同一规则）。
    let settings = world
        .get_resource::<scenes::CourtyardSettings>()
        .copied()
        .expect("settings inserted in configure");
    let recipe = showcase::capture_spec::camera_recipe_for_spec(&spec, &settings);
    match CaseDriver::new(
        world,
        scenario_model::ident::SessionId::new("android-catalog").expect("session id"),
        binding,
        showcase::capture_profile_graph(),
        DriverLimits {
            implicit_readiness: false,
            ..DriverLimits::default()
        },
    )
    .map(|driver| driver.with_camera_recipe(recipe))
    {
        Ok(driver) => world.resource_mut::<CatalogHost>().driver = Some(Box::new(driver)),
        Err(error) => report_and_stay(
            &spec_id,
            json!({"status":"FAIL","reason":format!("invalid capture plan: {error}")}),
        ),
    }
}

/// 报告写盘后进入驻留（§11.5 的驻留会话模式地基）：应用不退出，窗口
/// 保持运行，触摸事件经既有 input 通道可读（手势翻译与参数面板属交互
/// V3 后续工作；lane 的 per-capture 进程隔离由 runner 的 force-stop
/// 承担，驻留不改变 lane 语义）。
fn report_and_stay(scene: &str, payload: serde_json::Value) {
    write_report(scene, &payload);
    info!("ANDROID_RESIDENT alive scene={scene}");
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
        // 驻留：应用保持运行；触摸事件记录到 logcat（§11.5 手势翻译的
        // 现状梳理——事件已到达共享输入层，分解/面板属交互 V3）。
        let touches = world.resource::<bevy::input::touch::Touches>();
        for touch in touches.iter_just_pressed() {
            info!(
                "ANDROID_RESIDENT touch start id={:?} pos=({:.1},{:.1})",
                touch.id(),
                touch.position().x,
                touch.position().y
            );
        }
        for touch in touches.iter_just_released() {
            info!("ANDROID_RESIDENT touch end id={:?}", touch.id());
        }
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
                let spec = host.spec.clone();
                world.insert_resource(host);
                report_and_stay(
                    &spec,
                    json!({"status":"FAIL","reason":"no observation receipt"}),
                );
                return;
            };
            let dir = std::path::Path::new(FILES_DIR);
            let _ = std::fs::create_dir_all(dir);
            let png_candidate = dir.join(format!("catalog-{}.candidate.png", host.spec));
            let png_final = dir.join(format!("catalog-{}.png", host.spec));
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
            let spec = host.spec.clone();
            report_and_stay(
                &spec,
                json!({
                    "schema":"bevy_stylized.android-catalog/v1",
                    "status":"PASS",
                    "capture":spec,
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
            let spec = host.spec.clone();
            world.insert_resource(host);
            report_and_stay(&spec, json!({"status":"FAIL","reason":error.to_string()}));
            return;
        }
    }
    if !host.sent {
        world.insert_resource(host);
    }
}

/// Catalog mode 的宿主装配：捕获规格的 tier 桶（Standard 桶只用于
/// tier 门控捕获）与参数钉值（CourtyardSettings）。
pub fn configure(app: &mut App, spec_id: &str) {
    let spec = scenario_model::courtyard_capture_specs()
        .into_iter()
        .find(|spec| spec.capture_id == spec_id);
    if let Some(tier) = spec.as_ref().and_then(|spec| spec.tier) {
        // 必须在 StylizedPlugin 之前（Stage B 在 finish 时读该配置）。
        app.insert_resource(StylizedCapabilityConfig {
            requested_tier: tier,
            ..StylizedCapabilityConfig::default()
        });
    }
    if let Some(spec) = &spec {
        app.insert_resource(showcase::capture_spec::settings_from_spec(spec));
    }
    // 与 PC 捕获路径同一背景：程序化 fixture 的 ClearColor 覆盖不进入
    // catalog 模式（Bevy 默认 srgb_u8(43,44,47)）。
    app.insert_resource(ClearColor::default());
    *app.world_mut().resource_mut::<StylizedViewSettings>() =
        StylizedViewSettings(showcase::default_view_descriptor());
    app.insert_resource(CatalogHost {
        driver: None,
        spec: spec_id.to_owned(),
        sent: false,
    })
    .add_systems(Startup, build_driver)
    .add_systems(Update, pump);
}
