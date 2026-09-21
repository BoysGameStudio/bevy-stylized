//! Live scene lifecycle, linked-view synchronization and capture readiness.
use super::{asset::Input, capture, options::Options, state::*};
use bevy::{
    asset::LoadState,
    camera::{primitives::Aabb, visibility::RenderLayers},
    gltf::GltfAssetLabel,
    prelude::*,
    world_serialization::WorldAssetRoot,
};
use bevy_stylized::{StylizedMaterialAdoption, prelude::*};
use golden_harness::{capture as gpu, readiness};
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Component)]
struct Installed;
#[derive(Resource)]
pub(super) struct Runtime {
    pub(super) input: Input,
    pub(super) options: Options,
    pub(super) scene: Handle<bevy::world_serialization::WorldAsset>,
    pub(super) roots: Vec<Entity>,
    pub(super) targets: [gpu::CaptureTarget; 2],
    pub(super) cameras: [Entity; 2],
    pub(super) clones:
        HashMap<(AssetId<StylizedHeroMaterial>, usize), Handle<StylizedHeroMaterial>>,
    pub(super) pending: Option<capture::Pending>,
    pub(super) retired: Vec<gpu::CaptureRequest>,
    pub(super) generation: u64,
    pub(super) applied: Recipe,
    pub(super) started: Instant,
    pub(super) last_frame: u64,
    pub(super) bounds: Option<(Vec3, Vec3)>,
    pub(super) face_focus: Option<[f32; 3]>,
    pub(super) auto_fit: bool,
    pub(super) reload_exercised: bool,
    pub(super) done: bool,
    pub(super) window_capture: Option<(Entity, PathBuf, u64, u64)>,
    pub(super) window_due: Option<(PathBuf, u64)>,
}

pub(super) fn tick(world: &mut World) {
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
                // Hull colors are not lighting-factor data. Keep visibility
                // local to this view rather than mutating shared materials.
                if world
                    .get::<bevy_stylized::stylized_render::outline::StylizedOutlineReplica>(entity)
                    .is_some()
                {
                    let visibility = if state.recipe.debug[side] == DebugMode::Beauty {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    };
                    if world.get::<Visibility>(entity) != Some(&visibility) {
                        world.entity_mut(entity).insert(visibility);
                    }
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
pub(super) fn fit(min: Vec3, max: Vec3, face: bool, explicit: Option<[f32; 3]>) -> OrbitCamera {
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
