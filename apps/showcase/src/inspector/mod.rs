//! Static renderer inspection. No gameplay, animation controller or sibling dependency.
mod asset;
mod capture;
mod options;
mod panel;
mod runtime;
mod state;
mod validation;
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
