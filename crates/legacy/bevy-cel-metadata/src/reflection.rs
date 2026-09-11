use std::collections::BTreeMap;

use bevy::{
    ecs::{reflect::ReflectComponent, world::EntityWorldMut},
    reflect::{TypeRegistry, serde::TypedReflectDeserializer},
};
use serde::de::DeserializeSeed;
use serde_json::Value;

use crate::{MetadataError, MetadataSource};

/// Shared context for reflected component insertion.
#[derive(Clone, Copy)]
pub struct ReflectionInsertContext<'a> {
    /// Registry containing reflected component registrations.
    pub registry: &'a TypeRegistry,
    /// Source used in path-qualified errors.
    pub origin: &'a MetadataSource,
    /// JSON path to the components map.
    pub json_path: &'a str,
}

/// Parses every reflected value before inserting any component into the entity.
///
/// # Errors
///
/// Returns a path-qualified registration or value-shape error before inserting any component.
pub fn insert_reflected_components(
    entity: &mut EntityWorldMut<'_>,
    components: &BTreeMap<String, Value>,
    context: ReflectionInsertContext<'_>,
) -> Result<(), MetadataError> {
    let mut parsed = Vec::with_capacity(components.len());
    for (type_path, value) in components {
        let path = format!("{}.{type_path}", context.json_path);
        let registration = context
            .registry
            .get_with_type_path(type_path)
            .ok_or_else(|| MetadataError::UnknownReflectedType {
                origin: context.origin.clone(),
                json_path: path.clone(),
                type_path: type_path.clone(),
            })?;
        let reflect_component = registration
            .data::<ReflectComponent>()
            .cloned()
            .ok_or_else(|| MetadataError::MissingReflectComponent {
                origin: context.origin.clone(),
                json_path: path.clone(),
                type_path: type_path.clone(),
            })?;
        let reflected = TypedReflectDeserializer::new(registration, context.registry)
            .deserialize(value.clone())
            .map_err(|error| MetadataError::ReflectedValue {
                origin: context.origin.clone(),
                json_path: path,
                type_path: type_path.clone(),
                error,
            })?;
        parsed.push((reflect_component, reflected));
    }
    for (reflect_component, reflected) in parsed {
        reflect_component.insert(entity, reflected.as_ref(), context.registry);
    }
    Ok(())
}
