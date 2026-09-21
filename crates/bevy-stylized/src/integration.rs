//! Explicit scene material adoption, shared across instances and released on despawn.
use bevy::gltf::GltfMaterialName;
use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy_stylized_core::{
    StylizedMaterialDescriptor, StylizedSurfaceClass, StylizedTextureSemantic,
};
use bevy_stylized_render::{
    StylizedCelMaterial, StylizedCharacterMaterial, StylizedHeroMaterial, StylizedInstanceData,
};
use bevy_stylized_render::{
    host_inputs::HostInstanceRecord,
    host_material::{HostAlphaMode, HostInstanceInput, HostMaterial},
};
use std::collections::{BTreeMap, HashMap, HashSet};

mod outline;

/// Attach to the application-owned scene root before loading its descendants.
/// This is a one-time adoption configuration, not a live material editor.
/// Use `rebind_stylized_material` for an explicit generation-checked retry/rebind;
/// per-instance rendering state is separate.
#[derive(Component, Clone, Debug)]
pub struct StylizedMaterialSet {
    /// Exact glTF material names mapped to runtime descriptors.
    pub materials: BTreeMap<String, StylizedMaterialDescriptor>,
    /// Asset-server-relative directory containing descriptor texture paths.
    pub texture_prefix: String,
}

/// Opt-in bounded host-frame adoption. View and instance state are not part of
/// the shared static-material cache key. Requires HostMaterialPlugin.
#[derive(Component, Clone, Debug)]
pub struct HostMaterialScope {
    pub view: Entity,
    pub instance: HostInstanceRecord,
}

/// Per-entity adoption outcome. Errors are inspectable instead of silent defaults.
#[derive(Component, Clone, Debug)]
pub struct StylizedMaterialAdoption {
    pub material_name: String,
    pub error: Option<String>,
    cache_key: String,
    source: Handle<StandardMaterial>,
    pub generation: u64,
}

#[derive(Component)]
struct NextAdoptionGeneration(u64);

/// Restore the original material and request one new adoption on the next update.
/// A stale expected generation, missing entity or unavailable source changes nothing.
/// Per-instance data is preserved because it may belong to the host.
pub fn rebind_stylized_material(
    world: &mut World,
    entity: Entity,
    expected_generation: u64,
) -> Result<(), &'static str> {
    let adoption = world
        .get::<StylizedMaterialAdoption>(entity)
        .ok_or("no completed adoption")?;
    if adoption.generation != expected_generation {
        return Err("stale adoption generation");
    }
    let next = expected_generation
        .checked_add(1)
        .ok_or("adoption generation exhausted")?;
    let source = adoption.source.clone();
    if world
        .get_resource::<Assets<StandardMaterial>>()
        .is_none_or(|assets| assets.get(&source).is_none())
    {
        return Err("original material is unavailable");
    }
    outline::clear(world, entity);
    world
        .entity_mut(entity)
        .remove::<(
            StylizedMaterialAdoption,
            MeshMaterial3d<StylizedCelMaterial>,
            MeshMaterial3d<StylizedCharacterMaterial>,
            MeshMaterial3d<StylizedHeroMaterial>,
            MeshMaterial3d<HostMaterial<StylizedCelMaterial>>,
            MeshMaterial3d<HostMaterial<StylizedCharacterMaterial>>,
            MeshMaterial3d<HostMaterial<StylizedHeroMaterial>>,
            HostInstanceInput,
        )>()
        .insert((MeshMaterial3d(source), NextAdoptionGeneration(next)));
    Ok(())
}

#[derive(Clone)]
enum CachedMaterial {
    Cel(Handle<StylizedCelMaterial>),
    Character(Handle<StylizedCharacterMaterial>),
    Hero(Handle<StylizedHeroMaterial>),
    HostCel(Handle<HostMaterial<StylizedCelMaterial>>, Vec4),
    HostCharacter(Handle<HostMaterial<StylizedCharacterMaterial>>, Vec4),
    HostHero(Handle<HostMaterial<StylizedHeroMaterial>>, Vec4),
}
#[derive(Resource, Default)]
struct MaterialCache(HashMap<String, CachedMaterial>);

/// Optional integration plugin; does not load manifests or fetch assets itself.
pub struct StylizedAssetIntegrationPlugin;
impl Plugin for StylizedAssetIntegrationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MaterialCache>()
            .init_resource::<outline::OutlineCache>()
            .add_systems(
                Update,
                (
                    adopt,
                    outline::adopt,
                    release_unused,
                    outline::release_unused,
                )
                    .chain(),
            );
    }
}

fn owner<'a>(
    mut entity: Entity,
    parents: &Query<&ChildOf>,
    sets: &'a Query<(&StylizedMaterialSet, Option<&HostMaterialScope>)>,
) -> Option<(&'a StylizedMaterialSet, Option<&'a HostMaterialScope>)> {
    loop {
        if let Ok(set) = sets.get(entity) {
            return Some(set);
        }
        entity = parents.get(entity).ok()?.parent();
    }
}

fn adopt(
    mut commands: Commands,
    entities: Query<
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            Option<&GltfMaterialName>,
            Option<&NextAdoptionGeneration>,
        ),
        Without<StylizedMaterialAdoption>,
    >,
    parents: Query<&ChildOf>,
    sets: Query<(&StylizedMaterialSet, Option<&HostMaterialScope>)>,
    server: Res<AssetServer>,
    standard: Res<Assets<StandardMaterial>>,
    mut cel: ResMut<Assets<StylizedCelMaterial>>,
    mut character: ResMut<Assets<StylizedCharacterMaterial>>,
    mut hero: ResMut<Assets<StylizedHeroMaterial>>,
    mut cache: ResMut<MaterialCache>,
    mut host_cel: Option<ResMut<Assets<HostMaterial<StylizedCelMaterial>>>>,
    mut host_character: Option<ResMut<Assets<HostMaterial<StylizedCharacterMaterial>>>>,
    mut host_hero: Option<ResMut<Assets<HostMaterial<StylizedHeroMaterial>>>>,
) {
    for (entity, source, name, requested_generation) in &entities {
        let generation = requested_generation.map_or(1, |requested| requested.0);
        let Some((set, scope)) = owner(entity, &parents, &sets) else {
            continue;
        };
        if standard.get(&source.0).is_none() {
            continue;
        }
        let name = name.map(|n| n.0.clone()).unwrap_or_default();
        let Some(descriptor) = set.materials.get(&name) else {
            commands
                .entity(entity)
                .insert(StylizedMaterialAdoption {
                    material_name: name.clone(),
                    error: Some(format!("unmapped material: {name}")),
                    cache_key: String::new(),
                    source: source.0.clone(),
                    generation,
                })
                .remove::<NextAdoptionGeneration>();
            continue;
        };
        // Built only once at adoption. Identical descriptors share material assets
        // across scene roots; dynamic instance state is kept out of this key.
        if scope.is_some()
            && (host_cel.is_none() || host_character.is_none() || host_hero.is_none())
        {
            commands
                .entity(entity)
                .insert(StylizedMaterialAdoption {
                    material_name: name,
                    error: Some("HostMaterialPlugin is required for host material scope".into()),
                    cache_key: String::new(),
                    source: source.0.clone(),
                    generation,
                })
                .remove::<NextAdoptionGeneration>();
            continue;
        }
        let key = format!("{}:{}:{descriptor:?}", scope.is_some(), set.texture_prefix);
        let material = cache
            .0
            .entry(key.clone())
            .or_insert_with(|| {
                let load = |uri: &str| {
                    let semantic = descriptor
                        .textures
                        .iter()
                        .find(|t| t.uri == uri)
                        .map(|t| t.semantic)
                        .unwrap_or_default();
                    let source_linear_ramp = descriptor.static_params.get("specular_ramp_linear")
                        == Some(&1.0)
                        && descriptor
                            .static_params
                            .get("specular_ramp_index")
                            .and_then(|i| descriptor.textures.get(*i as usize))
                            .is_some_and(|t| t.uri == uri);
                    let srgb = !source_linear_ramp
                        && matches!(
                            semantic,
                            StylizedTextureSemantic::Color
                                | StylizedTextureSemantic::Ramp
                                | StylizedTextureSemantic::MatCap
                        );
                    let authored_sampling =
                        ["genshin_lighting", "genshin_face", "cubic_base_color"]
                            .iter()
                            .any(|key| {
                                descriptor.static_params.get(*key).is_some_and(|v| *v > 0.5)
                            });
                    let path = if set.texture_prefix.is_empty() {
                        uri.to_owned()
                    } else {
                        format!("{}/{uri}", set.texture_prefix.trim_end_matches('/'))
                    };
                    server
                        .load_builder()
                        .with_settings(move |settings: &mut ImageLoaderSettings| {
                            settings.is_srgb = srgb;
                            if authored_sampling {
                                let mut sampler = ImageSamplerDescriptor::linear();
                                sampler.set_address_mode(ImageAddressMode::Repeat);
                                settings.sampler = ImageSampler::Descriptor(sampler);
                            }
                        })
                        .load(path)
                };
                match descriptor.surface_class {
                    StylizedSurfaceClass::Cel => {
                        let material = StylizedCelMaterial::from_descriptor(descriptor, load);
                        if scope.is_some() {
                            let tint = material.params.base_color_factor;
                            CachedMaterial::HostCel(
                                host_cel
                                    .as_mut()
                                    .unwrap()
                                    .add(HostMaterial::new(material, HostAlphaMode::Inherit)),
                                tint,
                            )
                        } else {
                            CachedMaterial::Cel(cel.add(material))
                        }
                    }
                    StylizedSurfaceClass::Character => {
                        let material = StylizedCharacterMaterial::from_descriptor(descriptor, load);
                        if scope.is_some() {
                            let tint = material.params.cel.base_color_factor;
                            CachedMaterial::HostCharacter(
                                host_character
                                    .as_mut()
                                    .unwrap()
                                    .add(HostMaterial::new(material, HostAlphaMode::Inherit)),
                                tint,
                            )
                        } else {
                            CachedMaterial::Character(character.add(material))
                        }
                    }
                    StylizedSurfaceClass::Hero => {
                        let material = StylizedHeroMaterial::from_descriptor(descriptor, load);
                        if scope.is_some() {
                            let tint = material.params.character.cel.base_color_factor;
                            CachedMaterial::HostHero(
                                host_hero
                                    .as_mut()
                                    .unwrap()
                                    .add(HostMaterial::new(material, HostAlphaMode::Inherit)),
                                tint,
                            )
                        } else {
                            CachedMaterial::Hero(hero.add(material))
                        }
                    }
                }
            })
            .clone();
        let mut target = commands.entity(entity);
        target.remove::<MeshMaterial3d<StandardMaterial>>();
        match material {
            CachedMaterial::Cel(handle) => {
                target.insert(MeshMaterial3d(handle));
            }
            CachedMaterial::Character(handle) => {
                target.insert(MeshMaterial3d(handle));
            }
            CachedMaterial::Hero(handle) => {
                target.insert(MeshMaterial3d(handle));
            }
            CachedMaterial::HostCel(handle, tint) => {
                let scope = scope.unwrap();
                let mut record = scope.instance;
                record.tint *= tint;
                target.insert((
                    MeshMaterial3d(handle),
                    HostInstanceInput {
                        view: scope.view,
                        record,
                    },
                ));
            }
            CachedMaterial::HostCharacter(handle, tint) => {
                let scope = scope.unwrap();
                let mut record = scope.instance;
                record.tint *= tint;
                target.insert((
                    MeshMaterial3d(handle),
                    HostInstanceInput {
                        view: scope.view,
                        record,
                    },
                ));
            }
            CachedMaterial::HostHero(handle, tint) => {
                let scope = scope.unwrap();
                let mut record = scope.instance;
                record.tint *= tint;
                target.insert((
                    MeshMaterial3d(handle),
                    HostInstanceInput {
                        view: scope.view,
                        record,
                    },
                ));
            }
        }
        if scope.is_none() {
            target.insert_if_new(StylizedInstanceData::default());
        }
        target
            .insert(StylizedMaterialAdoption {
                material_name: name,
                error: None,
                cache_key: key,
                source: source.0.clone(),
                generation,
            })
            .remove::<NextAdoptionGeneration>();
    }
}

fn release_unused(
    mut removed: RemovedComponents<StylizedMaterialAdoption>,
    live: Query<&StylizedMaterialAdoption>,
    mut cache: ResMut<MaterialCache>,
) {
    if removed.read().next().is_none() {
        return;
    }
    let keys: HashSet<_> = live.iter().map(|m| m.cache_key.as_str()).collect();
    cache.0.retain(|key, _| keys.contains(key.as_str()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_materials_share_survive_and_release_the_last_cache_owner() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<StandardMaterial>()
            .init_asset::<StylizedCelMaterial>()
            .init_asset::<StylizedCharacterMaterial>()
            .init_asset::<StylizedHeroMaterial>()
            .add_plugins(StylizedAssetIntegrationPlugin);
        let standard = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let set = StylizedMaterialSet {
            materials: BTreeMap::from([("body".into(), StylizedMaterialDescriptor::default())]),
            texture_prefix: String::new(),
        };
        let first = app.world_mut().spawn(set.clone()).id();
        let second = app.world_mut().spawn(set).id();
        let a = app
            .world_mut()
            .spawn((
                ChildOf(first),
                GltfMaterialName("body".into()),
                MeshMaterial3d(standard.clone()),
            ))
            .id();
        let b = app
            .world_mut()
            .spawn((
                ChildOf(second),
                GltfMaterialName("body".into()),
                MeshMaterial3d(standard),
            ))
            .id();
        app.update();
        let handle_a = app
            .world()
            .get::<MeshMaterial3d<StylizedCelMaterial>>(a)
            .unwrap()
            .0
            .id();
        let handle_b = app
            .world()
            .get::<MeshMaterial3d<StylizedCelMaterial>>(b)
            .unwrap()
            .0
            .id();
        assert_eq!(handle_a, handle_b);
        assert_eq!(app.world().resource::<MaterialCache>().0.len(), 1);
        app.world_mut().despawn(first);
        app.update();
        assert_eq!(app.world().resource::<MaterialCache>().0.len(), 1);
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StylizedCelMaterial>>(b)
                .unwrap()
                .0
                .id(),
            handle_b
        );
        app.world_mut().despawn(second);
        app.update();
        assert!(app.world().resource::<MaterialCache>().0.is_empty());
        for _ in 0..5 {
            app.update();
        }
        assert!(
            app.world()
                .resource::<Assets<StylizedCelMaterial>>()
                .get(handle_a)
                .is_none()
        );
    }
    #[test]
    fn explicit_retry_recovers_errors_and_rebinds_without_stale_generations() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<StandardMaterial>()
            .init_asset::<StylizedCelMaterial>()
            .init_asset::<StylizedCharacterMaterial>()
            .init_asset::<StylizedHeroMaterial>()
            .add_plugins(StylizedAssetIntegrationPlugin);
        let source = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let root = app
            .world_mut()
            .spawn(StylizedMaterialSet {
                materials: BTreeMap::new(),
                texture_prefix: String::new(),
            })
            .id();
        let entity = app
            .world_mut()
            .spawn((
                ChildOf(root),
                GltfMaterialName("body".into()),
                MeshMaterial3d(source.clone()),
            ))
            .id();
        app.update();
        assert!(
            app.world()
                .get::<StylizedMaterialAdoption>(entity)
                .unwrap()
                .error
                .is_some()
        );
        app.world_mut()
            .get_mut::<StylizedMaterialSet>(root)
            .unwrap()
            .materials
            .insert("body".into(), Default::default());
        app.update(); // Configuration changes alone do not silently retry.
        assert!(
            app.world()
                .get::<StylizedMaterialAdoption>(entity)
                .unwrap()
                .error
                .is_some()
        );
        assert!(rebind_stylized_material(app.world_mut(), entity, 0).is_err());
        rebind_stylized_material(app.world_mut(), entity, 1).unwrap();
        app.update();
        let outcome = app.world().get::<StylizedMaterialAdoption>(entity).unwrap();
        assert_eq!(outcome.generation, 2);
        assert!(outcome.error.is_none());
        let old = app
            .world()
            .get::<MeshMaterial3d<StylizedCelMaterial>>(entity)
            .unwrap()
            .0
            .id();
        app.world_mut()
            .get_mut::<StylizedMaterialSet>(root)
            .unwrap()
            .materials
            .get_mut("body")
            .unwrap()
            .surface_class = StylizedSurfaceClass::Hero;
        assert!(rebind_stylized_material(app.world_mut(), entity, 1).is_err());
        rebind_stylized_material(app.world_mut(), entity, 2).unwrap();
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(entity)
                .unwrap()
                .0,
            source
        );
        assert!(
            !app.world()
                .entity(entity)
                .contains::<MeshMaterial3d<StylizedCelMaterial>>()
        );
        app.update();
        assert!(
            app.world()
                .entity(entity)
                .contains::<MeshMaterial3d<StylizedHeroMaterial>>()
        );
        assert_eq!(
            app.world()
                .get::<StylizedMaterialAdoption>(entity)
                .unwrap()
                .generation,
            3
        );
        assert_eq!(app.world().resource::<MaterialCache>().0.len(), 1);
        for _ in 0..5 {
            app.update();
        }
        assert!(
            app.world()
                .resource::<Assets<StylizedCelMaterial>>()
                .get(old)
                .is_none()
        );
    }
}
