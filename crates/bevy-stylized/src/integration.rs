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
use std::collections::{BTreeMap, HashMap, HashSet};

/// Attach to the application-owned scene root before loading its descendants.
/// This is a one-time adoption configuration, not a live material editor.
/// Recreate the scene to apply a new set; per-instance rendering state is separate.
#[derive(Component, Clone, Debug)]
pub struct StylizedMaterialSet {
    /// Exact glTF material names mapped to runtime descriptors.
    pub materials: BTreeMap<String, StylizedMaterialDescriptor>,
    /// Asset-server-relative directory containing descriptor texture paths.
    pub texture_prefix: String,
}

/// Per-entity adoption outcome. Errors are inspectable instead of silent defaults.
#[derive(Component, Clone, Debug)]
pub struct StylizedMaterialAdoption {
    pub material_name: String,
    pub error: Option<String>,
    cache_key: String,
}

#[derive(Clone)]
enum CachedMaterial {
    Cel(Handle<StylizedCelMaterial>),
    Character(Handle<StylizedCharacterMaterial>),
    Hero(Handle<StylizedHeroMaterial>),
}
#[derive(Resource, Default)]
struct MaterialCache(HashMap<String, CachedMaterial>);

/// Optional integration plugin; does not load manifests or fetch assets itself.
pub struct StylizedAssetIntegrationPlugin;
impl Plugin for StylizedAssetIntegrationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MaterialCache>()
            .add_systems(Update, (adopt, release_unused).chain());
    }
}

fn owner<'a>(
    mut entity: Entity,
    parents: &Query<&ChildOf>,
    sets: &'a Query<&StylizedMaterialSet>,
) -> Option<&'a StylizedMaterialSet> {
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
        ),
        Without<StylizedMaterialAdoption>,
    >,
    parents: Query<&ChildOf>,
    sets: Query<&StylizedMaterialSet>,
    server: Res<AssetServer>,
    standard: Res<Assets<StandardMaterial>>,
    mut cel: ResMut<Assets<StylizedCelMaterial>>,
    mut character: ResMut<Assets<StylizedCharacterMaterial>>,
    mut hero: ResMut<Assets<StylizedHeroMaterial>>,
    mut cache: ResMut<MaterialCache>,
) {
    for (entity, source, name) in &entities {
        let Some(set) = owner(entity, &parents, &sets) else {
            continue;
        };
        if standard.get(&source.0).is_none() {
            continue;
        }
        let name = name.map(|n| n.0.clone()).unwrap_or_default();
        let Some(descriptor) = set.materials.get(&name) else {
            commands.entity(entity).insert(StylizedMaterialAdoption {
                material_name: name.clone(),
                error: Some(format!("unmapped material: {name}")),
                cache_key: String::new(),
            });
            continue;
        };
        // Built only once at adoption. Identical descriptors share material assets
        // across scene roots; dynamic instance state is kept out of this key.
        let key = format!("{}:{descriptor:?}", set.texture_prefix);
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
                    let srgb = matches!(
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
                    StylizedSurfaceClass::Cel => CachedMaterial::Cel(
                        cel.add(StylizedCelMaterial::from_descriptor(descriptor, load)),
                    ),
                    StylizedSurfaceClass::Character => CachedMaterial::Character(
                        character.add(StylizedCharacterMaterial::from_descriptor(descriptor, load)),
                    ),
                    StylizedSurfaceClass::Hero => CachedMaterial::Hero(
                        hero.add(StylizedHeroMaterial::from_descriptor(descriptor, load)),
                    ),
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
        }
        target.insert_if_new(StylizedInstanceData::default());
        target.insert(StylizedMaterialAdoption {
            material_name: name,
            error: None,
            cache_key: key,
        });
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
}
