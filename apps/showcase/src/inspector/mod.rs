//! Static renderer inspection. No gameplay, animation controller or sibling dependency.
//!
//! 共享运行时对齐的处置（H4b；docs/showcase/01 line 135/194-195）：
//!
//! - **读回身份**：capture.json 携带 harness `CaptureStamp`（additive
//!   `stamps` 字段）；generation/revision 与 scenario-model 的
//!   Generation/Revision 语义一一对应；
//! - **保留在 Inspector 本地（各自有明确理由）**：编译产物 bundle 的输入
//!   验收（不可变字节核验，line 137——不与 labs/reference 合并）；
//!   A/B 双侧同步捕获 + retire-排空语义（与 `cancel_capture` 的丢弃语义
//!   不同，`retired_captures` 验收依赖它）；就绪谓词（adoption 计数 +
//!   bounds + 连续稳定帧，超出 driver `WaitReady` 的语义）；相机/调试
//!   控制（R12 边界：A/B 克隆材质与 RenderLayers 分区是诊断面，不触碰
//!   共享实验状态——唯一全局写入是 Inspector 独占应用的
//!   `StylizedViewSettings`）；
//! - egui 面板只存在于本 crate（桌面宿主）；共享层无 UI 依赖。
mod asset;
mod capture;
mod options;
mod panel;
mod runtime;
mod state;
pub(crate) mod validation;
use asset::Input;
use bevy::{
    asset::AssetMetaCheck,
    camera::visibility::RenderLayers,
    core_pipeline::tonemapping::Tonemapping,
    gltf::GltfAssetLabel,
    prelude::*,
    render::{RenderPlugin, settings::RenderCreation},
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::{StylizedAssetIntegrationPlugin, prelude::*};
use golden_harness::{capture as gpu, readiness};
use options::Options;
use runtime::{Runtime, fit, tick};
use state::*;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

pub fn run(args: &[String]) -> Result<(), String> {
    if args.iter().any(|a| a == "--help") {
        println!(
            "inspect --asset COMPILED.glb [--config recipe.json] [--out NEW_DIRECTORY] [--resolution 1024] [--headless|--capture-once|--scan|--measure] [--frames 120] [--exercise-reload]"
        );
        return Ok(());
    }
    let options = Options::parse(args)?;
    let input = Input::load(&options.asset)?;
    let recipe = if let Some(p) = &options.config {
        input.recipe_from(p)?
    } else {
        input.recipe(options.resolution)
    };
    recipe.validate()?;
    let headless = options.headless;
    let validation_mode = options.validation.clone();
    let resources = std::panic::catch_unwind(gpu::probe_render_resources)
        .map_err(|_| "required GPU unavailable: initialization failed")?
        .ok_or("required GPU unavailable for inspection")?;
    let mut app = App::new();
    let mut plugins = DefaultPlugins
        .build()
        .set(bevy_stylized::stylized_gltf_plugin())
        .set(AssetPlugin {
            file_path: input.root.to_string_lossy().into_owned(),
            meta_check: AssetMetaCheck::Never,
            ..default()
        })
        .set(RenderPlugin {
            render_creation: RenderCreation::Manual(resources),
            ..default()
        });
    if headless {
        plugins = plugins
            .disable::<bevy::winit::WinitPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            });
    } else {
        plugins = plugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Stylized — Character Inspector".into(),
                resolution: (1360, 880).into(),
                ..default()
            }),
            ..default()
        });
    }
    if validation_mode.is_some()
        && plugins.contains::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>()
    {
        plugins = plugins.disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>();
    }
    app.add_plugins(plugins)
        .add_plugins((StylizedPlugin::default(), StylizedAssetIntegrationPlugin));
    if validation_mode.as_deref() == Some("--measure") {
        validation::install_measurement(&mut app);
    }
    gpu::install(&mut app);
    readiness::install(&mut app);
    if !headless {
        // The two offscreen cameras are not UI contexts. A dedicated Camera2d
        // below owns the single primary context; automatic creation races it.
        app.insert_resource(bevy_egui::EguiGlobalSettings {
            auto_create_primary_context: false,
            ..default()
        });
        app.add_plugins(bevy_egui::EguiPlugin::default())
            .add_systems(bevy_egui::EguiPrimaryContextPass, panel::ui);
    }
    app.insert_resource(ClearColor(Color::linear_rgb(0.12, 0.12, 0.12)));
    let targets = std::array::from_fn(|_| {
        gpu::create_capture_target(
            &mut app.world_mut().resource_mut::<Assets<Image>>(),
            recipe.resolution,
            recipe.resolution,
        )
    });
    let cameras = std::array::from_fn(|side| {
        app.world_mut()
            .spawn((
                Camera3d::default(),
                gpu::capture_target_bundle(&targets[side]),
                Tonemapping::None,
                Msaa::Sample4,
                Projection::Perspective(PerspectiveProjection {
                    fov: std::f32::consts::FRAC_PI_4,
                    near: 0.001,
                    far: 10000.,
                    ..default()
                }),
                RenderLayers::layer(side + 1),
                recipe.cameras[side].transform(),
            ))
            .id()
    });
    if !headless {
        app.world_mut().spawn((
            Camera2d,
            Camera {
                order: 10,
                ..default()
            },
            bevy_egui::PrimaryEguiContext,
        ));
    }
    let scene = app
        .world()
        .resource::<AssetServer>()
        .load(GltfAssetLabel::Scene(0).from_asset(input.name.clone()));
    let runtime = Runtime {
        auto_fit: options.config.is_none(),
        input,
        options,
        scene,
        roots: Vec::new(),
        targets,
        cameras,
        clones: HashMap::new(),
        pending: None,
        retired: Vec::new(),
        generation: 0,
        applied: recipe.clone(),
        started: Instant::now(),
        last_frame: 0,
        bounds: None,
        face_focus: None,
        reload_exercised: false,
        done: false,
        window_capture: None,
        window_due: None,
    };
    app.insert_resource(State {
        reset: recipe.clone(),
        recipe,
        revision: 0,
        stable: 0,
        status: "Loading compiled asset…".into(),
        error: None,
        ready: false,
        capture: false,
        reload: true,
        load_config: false,
        captures: 0,
        retired: 0,
    });
    app.insert_resource(runtime).add_systems(
        Update,
        tick.before(bevy_stylized::stylized_render::view::sync_stylized_view),
    );
    if headless {
        app.finish();
        app.cleanup();
        if let Some(mode) = validation_mode {
            return validation::run(&mut app, &mode);
        }
        let started = Instant::now();
        loop {
            app.update();
            if let Some(error) = &app.world().resource::<State>().error {
                return Err(error.clone());
            }
            if app.world().resource::<Runtime>().done {
                break;
            }
            if started.elapsed() > Duration::from_secs(120) {
                return Err("inspector timed out".into());
            }
        }
    } else {
        if !app.run().is_success() {
            return Err("interactive inspector failed; inspect its status/log".into());
        }
    }
    Ok(())
}

// Keep compile-time provenance explicit; the test checks it against the module tree.
const SOURCES: &[(&str, &str)] = &[
    ("asset.rs", include_str!("asset.rs")),
    ("capture.rs", include_str!("capture.rs")),
    ("mod.rs", include_str!("mod.rs")),
    ("options.rs", include_str!("options.rs")),
    ("panel.rs", include_str!("panel.rs")),
    ("runtime.rs", include_str!("runtime.rs")),
    ("state.rs", include_str!("state.rs")),
    ("validation.rs", include_str!("validation.rs")),
];

fn source_sha256() -> String {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    for (name, source) in SOURCES {
        hash.update(name.as_bytes());
        hash.update(b"\0");
        hash.update(source.as_bytes());
        hash.update(b"\0");
    }
    format!("{:x}", hash.finalize())
}

#[cfg(test)]
mod provenance_tests {
    use super::*;

    #[test]
    fn source_provenance_covers_each_inspector_module() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/inspector");
        fn collect(root: &std::path::Path, modules: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(root).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    collect(&path, modules);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    modules.push(path);
                }
            }
        }
        let mut modules = Vec::new();
        collect(&root, &mut modules);
        modules.sort();
        let names: Vec<_> = modules
            .iter()
            .map(|p| p.strip_prefix(&root).unwrap().to_str().unwrap())
            .collect();
        assert_eq!(
            names,
            SOURCES.iter().map(|(name, _)| *name).collect::<Vec<_>>()
        );
        for ((_, embedded), path) in SOURCES.iter().zip(modules) {
            assert_eq!(*embedded, std::fs::read_to_string(path).unwrap());
        }
    }
}
