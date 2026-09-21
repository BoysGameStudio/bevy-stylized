//! Descriptor-owned hulls share materials and follow the surface adoption lifecycle.
use super::*;
use bevy_stylized_render::host_material::HostOutline;
use bevy_stylized_render::{StylizedOutline, StylizedOutlineMaterial, StylizedOutlineParams};

#[derive(Clone)]
enum Material {
    Static(Handle<StylizedOutlineMaterial>),
    Host(Handle<HostMaterial<StylizedOutlineMaterial>>),
}

#[derive(Resource, Default)]
pub(super) struct OutlineCache(HashMap<String, Material>);

#[derive(Component)]
pub(super) struct OwnedOutline {
    generation: u64,
    key: String,
}

pub(super) fn clear(world: &mut World, entity: Entity) {
    if world.get::<OwnedOutline>(entity).is_some() {
        world
            .entity_mut(entity)
            .remove::<(OwnedOutline, StylizedOutline, HostOutline)>();
    }
}

fn parameters(
    d: &StylizedMaterialDescriptor,
) -> Result<Option<(StylizedOutlineParams, Option<&str>)>, String> {
    if !d.static_params.contains_key("outline_source_width") {
        return Ok(None);
    }
    let get = |key: &str| -> Result<f32, String> {
        d.static_params
            .get(key)
            .copied()
            .filter(|v| v.is_finite() && *v >= 0.0)
            .ok_or_else(|| format!("missing/invalid outline parameter: {key}"))
    };
    let width = get("outline_source_width")?;
    let gamma = get("outline_gamma")?;
    let mask = get("outline_enabled_mask")?;
    let first = get("outline_force_first")?;
    if width <= 0.0
        || gamma <= 0.0
        || mask.fract() != 0.0
        || mask > 15.0
        || ![0.0, 1.0].contains(&first)
    {
        return Err("invalid outline width, gamma or class controls".into());
    }
    let mut colors = [LinearRgba::BLACK; 5];
    for (i, color) in colors.iter_mut().enumerate() {
        *color = LinearRgba::new(
            get(&format!("outline_color_{i}_0"))?,
            get(&format!("outline_color_{i}_1"))?,
            get(&format!("outline_color_{i}_2"))?,
            1.0,
        );
    }
    let texture = if let Some(index) = d.static_params.get("outline_class_texture_index") {
        if !index.is_finite() || *index < 0.0 || index.fract() != 0.0 {
            return Err("invalid outline texture index".into());
        }
        let t = d
            .textures
            .get(*index as usize)
            .ok_or("outline texture index out of range")?;
        if t.semantic != StylizedTextureSemantic::DiscreteClassification {
            return Err("outline class texture must be discrete data".into());
        }
        Some(t.uri.as_str())
    } else {
        None
    };
    if first == 0.0 && texture.is_none() {
        return Err("outline class map is required".into());
    }
    let mut params = StylizedOutlineParams::default().with_source_palette(
        width,
        colors,
        mask as u32,
        first == 1.0,
        gamma,
    );
    // Bit 0 binds the class map; bit 1 selects authored cubic reconstruction.
    params.source_state.w = if texture.is_some() { 3 } else { 0 };
    if let Some(value) = d.static_params.get("outline_source_position") {
        if ![0.0, 1.0].contains(value) {
            return Err("invalid source outline position flag".into());
        }
        params.source_params.z = *value;
    }
    Ok(Some((params, texture)))
}

pub(super) fn adopt(
    mut commands: Commands,
    mut entities: Query<(
        Entity,
        &mut StylizedMaterialAdoption,
        Option<&OwnedOutline>,
        Option<&StylizedOutline>,
        Option<&HostOutline>,
        Option<&Mesh3d>,
    )>,
    parents: Query<&ChildOf>,
    sets: Query<(&StylizedMaterialSet, Option<&HostMaterialScope>)>,
    server: Res<AssetServer>,
    meshes: Option<Res<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StylizedOutlineMaterial>>>,
    mut host_materials: Option<ResMut<Assets<HostMaterial<StylizedOutlineMaterial>>>>,
    mut cache: ResMut<OutlineCache>,
) {
    for (entity, mut adoption, owned, external, host_external, mesh) in &mut entities {
        if adoption.error.is_some() || owned.is_some_and(|o| o.generation == adoption.generation) {
            continue;
        }
        let Some((set, scope)) = owner(entity, &parents, &sets) else {
            continue;
        };
        let Some(descriptor) = set.materials.get(&adoption.material_name) else {
            continue;
        };
        let (params, texture) = match parameters(descriptor) {
            Ok(Some(v)) => v,
            Ok(None) => continue,
            Err(e) => {
                adoption.error = Some(e);
                continue;
            }
        };
        if params.source_params.z > 0.5 {
            let Some(mesh) = mesh.and_then(|m| meshes.as_ref().and_then(|assets| assets.get(&m.0)))
            else {
                adoption.error = Some("prepared outline mesh is unavailable".into());
                continue;
            };
            if !mesh.contains_attribute(
                bevy_stylized_render::outline::ATTRIBUTE_OUTLINE_SOURCE_POSITION,
            ) || mesh.contains_attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
                || mesh.has_morph_targets()
            {
                adoption.error =
                    Some("prepared outline requires static CONTROL0 hull positions".into());
                continue;
            }
        }
        if owned.is_none() && (external.is_some() || host_external.is_some()) {
            adoption.error =
                Some("descriptor outline conflicts with an existing outline owner".into());
            continue;
        }
        if materials.is_none() || (scope.is_some() && host_materials.is_none()) {
            adoption.error = Some("outline material plugin is required".into());
            continue;
        }
        let key = adoption.cache_key.clone();
        let material = cache
            .0
            .entry(key.clone())
            .or_insert_with(|| {
                let class_texture = texture.map(|uri| {
                    let path = if set.texture_prefix.is_empty() {
                        uri.to_owned()
                    } else {
                        format!("{}/{uri}", set.texture_prefix.trim_end_matches('/'))
                    };
                    server
                        .load_builder()
                        .with_settings(|s: &mut ImageLoaderSettings| {
                            s.is_srgb = false;
                            let mut sampler = ImageSamplerDescriptor::linear();
                            sampler.set_address_mode(ImageAddressMode::Repeat);
                            s.sampler = ImageSampler::Descriptor(sampler);
                        })
                        .load(path)
                });
                let material = StylizedOutlineMaterial {
                    params,
                    class_texture,
                };
                if scope.is_some() {
                    Material::Host(
                        host_materials
                            .as_mut()
                            .unwrap()
                            .add(HostMaterial::new(material, HostAlphaMode::Inherit)),
                    )
                } else {
                    Material::Static(materials.as_mut().unwrap().add(material))
                }
            })
            .clone();
        let mut target = commands.entity(entity);
        match material {
            Material::Static(h) => {
                target.insert(StylizedOutline::new(h));
            }
            Material::Host(h) => {
                target.insert(HostOutline(h));
            }
        }
        target.insert(OwnedOutline {
            generation: adoption.generation,
            key,
        });
    }
}

pub(super) fn release_unused(live: Query<&OwnedOutline>, mut cache: ResMut<OutlineCache>) {
    if cache.0.is_empty() {
        return;
    }
    let keys: HashSet<_> = live.iter().map(|o| o.key.as_str()).collect();
    cache.0.retain(|key, _| keys.contains(key.as_str()));
}

#[cfg(test)]
mod tests {
    use super::*;
    fn descriptor() -> StylizedMaterialDescriptor {
        let mut d = StylizedMaterialDescriptor::default();
        d.static_params.extend([
            ("outline_source_width".into(), 0.0025),
            ("outline_gamma".into(), 2.2),
            ("outline_enabled_mask".into(), 15.0),
            ("outline_force_first".into(), 1.0),
        ]);
        for i in 0..5 {
            for c in 0..3 {
                d.static_params
                    .insert(format!("outline_color_{i}_{c}"), 0.2);
            }
        }
        d
    }

    #[test]
    fn shared_hulls_rebind_without_leaking_or_removing_another_owner() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<StandardMaterial>()
            .init_asset::<StylizedCelMaterial>()
            .init_asset::<StylizedCharacterMaterial>()
            .init_asset::<StylizedHeroMaterial>()
            .init_asset::<StylizedOutlineMaterial>()
            .add_plugins(StylizedAssetIntegrationPlugin);
        let source = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let set = StylizedMaterialSet {
            materials: BTreeMap::from([("body".into(), descriptor())]),
            texture_prefix: String::new(),
        };
        let a_root = app.world_mut().spawn(set.clone()).id();
        let b_root = app.world_mut().spawn(set).id();
        let a = app
            .world_mut()
            .spawn((
                ChildOf(a_root),
                GltfMaterialName("body".into()),
                MeshMaterial3d(source.clone()),
            ))
            .id();
        let b = app
            .world_mut()
            .spawn((
                ChildOf(b_root),
                GltfMaterialName("body".into()),
                MeshMaterial3d(source),
            ))
            .id();
        app.update();
        let handle = app
            .world()
            .get::<StylizedOutline>(a)
            .unwrap()
            .material
            .clone();
        assert_eq!(
            handle.id(),
            app.world().get::<StylizedOutline>(b).unwrap().material.id()
        );
        assert_eq!(app.world().resource::<OutlineCache>().0.len(), 1);
        app.world_mut()
            .get_mut::<StylizedMaterialSet>(a_root)
            .unwrap()
            .materials
            .insert("body".into(), StylizedMaterialDescriptor::default());
        assert!(rebind_stylized_material(app.world_mut(), a, 0).is_err());
        assert!(app.world().get::<StylizedOutline>(a).is_some());
        rebind_stylized_material(app.world_mut(), a, 1).unwrap();
        app.update();
        assert!(app.world().get::<StylizedOutline>(a).is_none());
        assert_eq!(
            handle.id(),
            app.world().get::<StylizedOutline>(b).unwrap().material.id()
        );
        app.world_mut().despawn(b);
        app.update();
        assert!(app.world().resource::<OutlineCache>().0.is_empty());
    }

    #[test]
    fn requested_outline_cannot_silently_use_missing_parameters() {
        let mut d = StylizedMaterialDescriptor::default();
        assert!(parameters(&d).unwrap().is_none());
        d.static_params
            .insert("outline_source_width".into(), 0.0025);
        assert!(parameters(&d).is_err());
        d.static_params.extend([
            ("outline_gamma".into(), 2.2),
            ("outline_enabled_mask".into(), 15.0),
            ("outline_force_first".into(), 1.0),
        ]);
        for i in 0..5 {
            for c in 0..3 {
                d.static_params
                    .insert(format!("outline_color_{i}_{c}"), 0.2);
            }
        }
        let (p, t) = parameters(&d).unwrap().unwrap();
        assert_eq!(p.source_params.x, 0.0025);
        assert!(t.is_none());
        d.static_params.insert("outline_force_first".into(), 0.0);
        assert!(parameters(&d).is_err());
    }
}
