//! Static renderer inspection. No gameplay, animation controller or sibling dependency.
mod asset;
mod capture;
mod panel;
mod state;
mod validation;
use asset::Input;
use bevy::{
    asset::{AssetMetaCheck, LoadState},
    camera::{primitives::Aabb, visibility::RenderLayers},
    core_pipeline::tonemapping::Tonemapping,
    gltf::GltfAssetLabel,
    prelude::*,
    render::{RenderPlugin, settings::RenderCreation},
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::{StylizedAssetIntegrationPlugin, StylizedMaterialAdoption, prelude::*};
use golden_harness::{capture as gpu, readiness};
use state::*;
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Default)]
struct Options {
    asset: PathBuf,
    config: Option<PathBuf>,
    out: PathBuf,
    headless: bool,
    once: bool,
    exercise_reload: bool,
    resolution: u32,
    validation: Option<String>,
    frames: usize,
}
impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut o = Self {
            out: "target/stylized/inspector".into(),
            resolution: 1024,
            frames: 120,
            ..Default::default()
        };
        let mut it = args.iter();
        while let Some(a) = it.next() {
            match a.as_str() {
                "--asset" => o.asset = it.next().ok_or("--asset requires a path")?.into(),
                "--config" => o.config = Some(it.next().ok_or("--config requires a path")?.into()),
                "--out" => o.out = it.next().ok_or("--out requires a path")?.into(),
                "--resolution" => {
                    o.resolution = it
                        .next()
                        .ok_or("--resolution requires pixels")?
                        .parse()
                        .map_err(|_| "invalid resolution")?
                }
                "--headless" => {
                    o.headless = true;
                    o.once = true;
                }
                "--capture-once" => o.once = true,
                "--scan" | "--measure" => {
                    if o.validation.is_some() {
                        return Err("choose only one validation mode".into());
                    }
                    o.validation = Some(a.clone());
                    o.headless = true;
                }
                "--frames" => {
                    o.frames = it
                        .next()
                        .ok_or("--frames requires a count")?
                        .parse()
                        .map_err(|_| "invalid frame count")?
                }
                "--exercise-reload" => o.exercise_reload = true,
                _ => return Err(format!("unknown inspect argument: {a}")),
            }
        }
        if o.asset.as_os_str().is_empty() {
            return Err("usage: inspect --asset COMPILED.glb [--config recipe.json] [--out NEW_DIRECTORY] [--headless|--capture-once] [--exercise-reload]".into());
        }
        if o.validation.is_some()
            && (o.once || o.exercise_reload || !(24..=600).contains(&o.frames))
        {
            return Err(
                "scan/measure requires 24–600 frames and cannot combine with capture/reload modes"
                    .into(),
            );
        }
        if (o.once || o.validation.is_some()) && o.out.exists() {
            return Err("capture output exists; choose a new directory".into());
        }
        Ok(o)
    }
}
#[derive(Component)]
struct Installed;
#[derive(Resource)]
struct Runtime {
    input: Input,
    options: Options,
    scene: Handle<bevy::world_serialization::WorldAsset>,
    roots: Vec<Entity>,
    targets: [gpu::CaptureTarget; 2],
    cameras: [Entity; 2],
    clones: HashMap<(AssetId<StylizedHeroMaterial>, usize), Handle<StylizedHeroMaterial>>,
    pending: Option<capture::Pending>,
    retired: Vec<gpu::CaptureRequest>,
    generation: u64,
    applied: Recipe,
    started: Instant,
    last_frame: u64,
    bounds: Option<(Vec3, Vec3)>,
    face_focus: Option<[f32; 3]>,
    auto_fit: bool,
    reload_exercised: bool,
    done: bool,
    window_capture: Option<(Entity, PathBuf, u64, u64)>,
    window_due: Option<(PathBuf, u64)>,
}

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
fn tick(world: &mut World) {
    let mut runtime = world.remove_resource::<Runtime>().expect("runtime");
    let mut state = world.remove_resource::<State>().expect("state");
    if let Err(error) = runtime.update(world, &mut state) {
        state.error = Some(error.clone());
        state.status = error;
        state.ready = false;
        if runtime.options.once && !runtime.options.headless {
            world.write_message(AppExit::error());
        }
    }
    world.insert_resource(runtime);
    world.insert_resource(state);
}
impl Runtime {
    fn reload(&mut self, world: &mut World, state: &mut State) -> Result<(), String> {
        capture::retire(self, state);
        // Compiler outputs are immutable. A same-bundle scene rebuild may use
        // cached GPU assets, but must never attest hashes of newly edited bytes.
        let checked = Input::load(&self.options.asset)?;
        if checked.root != self.input.root || checked.files != self.input.files {
            return Err("compiled bundle changed; restart with the new build to load and verify fresh assets".into());
        }
        state.recipe.files = self.input.files.clone();
        state.reset.files = self.input.files.clone();
        for root in self.roots.drain(..) {
            world.despawn(root);
        }
        self.clones.clear();
        self.generation += 1;
        self.bounds = None;
        self.face_focus = None;
        self.scene = world
            .resource::<AssetServer>()
            .load(GltfAssetLabel::Scene(0).from_asset(self.input.name.clone()));
        for _ in 0..2 {
            self.roots.push(
                world
                    .spawn((
                        WorldAssetRoot(self.scene.clone()),
                        self.input.set.clone(),
                        Transform::default(),
                        Visibility::default(),
                    ))
                    .id(),
            );
        }
        state.error = None;
        state.changed();
        state.status = "Loading scene and materials…".into();
        self.started = Instant::now();
        Ok(())
    }
    fn update(&mut self, world: &mut World, state: &mut State) -> Result<(), String> {
        self.retired
            .retain(|r| gpu::try_take(world, r.clone()).is_none());
        if state.reload {
            state.reload = false;
            self.reload(world, state)?;
        }
        if state.error.is_some() {
            return Ok(());
        }
        if state.load_config {
            state.load_config = false;
            let path = self
                .options
                .config
                .as_ref()
                .ok_or("launch with --config to enable recipe reload")?;
            let recipe = self.input.recipe_from(path)?;
            if recipe.resolution != state.recipe.resolution {
                return Err("restart the inspector to change render-target resolution".into());
            }
            state.recipe = recipe;
            self.auto_fit = false;
        }
        if !self.input.supports_debug()
            && state.recipe.debug.iter().any(|m| *m != DebugMode::Beauty)
        {
            return Err("these diagnostic modes require Hero materials".into());
        }
        if state.recipe != self.applied {
            state.recipe.validate()?;
            state.changed();
            self.started = Instant::now();
            self.applied = state.recipe.clone();
            capture::retire(self, state);
        }
        let mut view = self.input.view.clone();
        view.key_light_direction = Vec3::from_array(state.recipe.light);
        view.exposure = state.recipe.exposure;
        world.resource_mut::<StylizedViewSettings>().0 = view;
        for side in 0..2 {
            world
                .entity_mut(self.cameras[side])
                .insert(state.recipe.cameras[side].transform());
        }
        let mut lights = world.query_filtered::<Entity, With<StylizedKeyLight>>();
        let lights = lights.iter(world).collect::<Vec<_>>();
        for entity in lights {
            let layers = RenderLayers::from_layers(&[1, 2]);
            if world.get::<RenderLayers>(entity) != Some(&layers) {
                world.entity_mut(entity).insert(layers);
            }
        }
        // Route all descendants, including late scene/material adoption.
        let mut q = world.query::<(Entity, &ChildOf)>();
        let parents: qtype::Parents = q.iter(world).map(|(e, p)| (e, p.parent())).collect();
        let side_of = |mut e: Entity| -> Option<usize> {
            loop {
                if let Some(i) = self.roots.iter().position(|r| *r == e) {
                    return Some(i);
                }
                e = *parents.get(&e)?;
            }
        };
        for &entity in parents.keys() {
            if let Some(side) = side_of(entity) {
                let layer = RenderLayers::layer(side + 1);
                if world.get::<RenderLayers>(entity) != Some(&layer) {
                    world.entity_mut(entity).insert(layer);
                }
            }
        }
        let mut q = world.query::<(
            Entity,
            &StylizedMaterialAdoption,
            Option<&MeshMaterial3d<StylizedHeroMaterial>>,
            Option<&Installed>,
        )>();
        let mut count = 0;
        let mut install = Vec::new();
        for (entity, adoption, material, installed) in q.iter(world) {
            if let Some(side) = side_of(entity) {
                count += 1;
                if let Some(e) = &adoption.error {
                    return Err(e.clone());
                }
                if installed.is_none()
                    && let Some(m) = material
                {
                    install.push((entity, side, m.0.clone()));
                }
            }
        }
        for (entity, side, source) in install {
            let key = (source.id(), side);
            let handle = if let Some(h) = self.clones.get(&key) {
                h.clone()
            } else {
                let mut materials = world.resource_mut::<Assets<StylizedHeroMaterial>>();
                let Some(mut material) = materials.get(&source).cloned() else {
                    continue;
                };
                material.debug_view = state.recipe.debug[side].hero();
                let h = materials.add(material);
                self.clones.insert(key, h.clone());
                h
            };
            world
                .entity_mut(entity)
                .insert((MeshMaterial3d(handle), Installed));
        }
        for ((_, side), handle) in &self.clones {
            if let Some(mut m) = world
                .resource_mut::<Assets<StylizedHeroMaterial>>()
                .get_mut(handle)
                && m.debug_view != state.recipe.debug[*side].hero()
            {
                m.debug_view = state.recipe.debug[*side].hero();
            }
        }
        let server = world.resource::<AssetServer>();
        if let Some(LoadState::Failed(e)) = server.get_load_state(self.scene.id()) {
            return Err(format!("scene load failed: {e}"));
        }
        let mut loaded = server.is_loaded_with_dependencies(self.scene.id());
        for uri in self
            .input
            .set
            .materials
            .values()
            .flat_map(|m| m.textures.iter().map(|t| &t.uri))
        {
            if let Some(h) = server.get_handle::<Image>(uri.clone()) {
                if let Some(LoadState::Failed(e)) = server.get_load_state(h.id()) {
                    return Err(format!("texture {uri}: {e}"));
                }
                loaded &= server.is_loaded_with_dependencies(h.id());
            } else {
                loaded = false;
            }
        }
        if loaded && count == self.input.meshes * 2 && self.bounds.is_none() {
            let mut min = Vec3::splat(f32::INFINITY);
            let mut max = Vec3::splat(f32::NEG_INFINITY);
            let face_materials = world
                .resource::<Assets<StylizedHeroMaterial>>()
                .iter()
                .filter(|(_, m)| m.face_sdf_texture.is_some())
                .map(|(id, _)| id)
                .collect::<std::collections::HashSet<_>>();
            let mut face_min = Vec3::splat(f32::INFINITY);
            let mut face_max = Vec3::splat(f32::NEG_INFINITY);
            let mut q = world.query::<(
                Entity,
                &Aabb,
                &GlobalTransform,
                Option<&MeshMaterial3d<StylizedHeroMaterial>>,
            )>();
            for (e, b, t, material) in q.iter(world) {
                if side_of(e) == Some(0) {
                    for x in [-1., 1.] {
                        for y in [-1., 1.] {
                            for z in [-1., 1.] {
                                let p = t.transform_point(
                                    Vec3::from(b.center)
                                        + Vec3::from(b.half_extents) * Vec3::new(x, y, z),
                                );
                                min = min.min(p);
                                max = max.max(p);
                                if material.is_some_and(|m| face_materials.contains(&m.0.id())) {
                                    face_min = face_min.min(p);
                                    face_max = face_max.max(p);
                                }
                            }
                        }
                    }
                }
            }
            if min.is_finite() && max.is_finite() && max.distance(min) > 0.001 {
                self.bounds = Some((min, max));
                self.face_focus = self.input.focus.or_else(|| {
                    (face_min.is_finite() && face_max.is_finite())
                        .then_some(((face_min + face_max) * 0.5).to_array())
                });
                if self.auto_fit {
                    let camera = fit(min, max, false, self.input.focus);
                    state.recipe.cameras = [camera.clone(), camera];
                    state.reset = state.recipe.clone();
                    self.auto_fit = false;
                    state.changed();
                    self.applied = state.recipe.clone();
                }
            }
        }
        let probe = world.resource::<readiness::Probe>().snapshot();
        if let Some(e) = probe.error {
            return Err(e);
        }
        if probe.frame != self.last_frame {
            self.last_frame = probe.frame;
            if loaded && count == self.input.meshes * 2 && self.bounds.is_some() && probe.ready {
                state.stable += 1;
            } else {
                state.stable = 0;
            }
        }
        state.ready = state.stable >= 24;
        if !state.ready && self.started.elapsed() > Duration::from_secs(60) {
            return Err(format!(
                "readiness timeout: {count}/{} adopted meshes; inspect textures and pipelines",
                self.input.meshes * 2
            ));
        }
        if state.ready && self.pending.is_none() {
            state.status = "Ready · live A/B · shared lighting · MSAA 4x".into();
        }
        if self.options.once && state.ready && state.captures == 0 && self.pending.is_none() {
            state.capture = true;
        }
        capture::update(self, world, state)?;
        if self.options.exercise_reload && !self.reload_exercised && self.pending.is_some() {
            self.reload_exercised = true;
            state.reload = true;
        }
        Ok(())
    }
}
mod qtype {
    pub type Parents = std::collections::HashMap<bevy::prelude::Entity, bevy::prelude::Entity>;
}
fn fit(min: Vec3, max: Vec3, face: bool, explicit: Option<[f32; 3]>) -> OrbitCamera {
    let height = (max.y - min.y).max(0.01);
    let center = (min + max) * 0.5;
    let target = if face {
        explicit.map(Vec3::from_array).unwrap_or(Vec3::new(
            center.x,
            min.y + height * 0.87,
            center.z,
        ))
    } else {
        center
    };
    let extent = if face {
        height * 0.28
    } else {
        (max - min).max_element()
    };
    OrbitCamera {
        target: target.to_array(),
        yaw: 0.,
        pitch: 0.,
        distance: (extent / (2. * (std::f32::consts::FRAC_PI_4 / 2.).tan()) * 1.2).max(0.02),
    }
}
