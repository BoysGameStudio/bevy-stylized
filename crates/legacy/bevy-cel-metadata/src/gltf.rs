use std::path::PathBuf;

use bevy::{
    app::{App, Plugin},
    asset::LoadContext,
    ecs::{reflect::AppTypeRegistry, world::EntityWorldMut},
    gltf::{
        GltfLoaderSettings,
        extensions::{ErasedGltfExtensionHandler, GltfExtensionHandler, GltfExtensionHandlers},
        gltf::Gltf,
    },
    reflect::TypeRegistryArc,
};
use gltf::Node;

use crate::{
    CelMetadataEnvelope, MetadataDiagnostic, MetadataError, MetadataSource,
    ReflectionInsertContext, decode_metadata, insert_reflected_components,
};

/// Identifier for the versioned CEL metadata glTF extension.
pub const CEL_METADATA_EXTENSION: &str = "CEL_metadata";

/// Registers the CEL extension handler with Bevy's official glTF loader seam.
#[derive(Clone, Copy, Debug, Default)]
pub struct CelMetadataPlugin;

impl Plugin for CelMetadataPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<MetadataDiagnostic>()
            .register_type::<crate::CelDomainMetadata>()
            .init_resource::<GltfExtensionHandlers>();
        let registry = app.world().resource::<AppTypeRegistry>().0.clone();
        let handler = CelMetadataExtensionHandler::new(registry);
        #[cfg(target_family = "wasm")]
        bevy::tasks::block_on(async {
            app.world_mut()
                .resource_mut::<GltfExtensionHandlers>()
                .0
                .write()
                .await
                .push(Box::new(handler));
        });
        #[cfg(not(target_family = "wasm"))]
        app.world_mut()
            .resource_mut::<GltfExtensionHandlers>()
            .0
            .write_blocking()
            .push(Box::new(handler));
    }
}

/// Per-load handler for the `CEL_metadata` root extension.
pub struct CelMetadataExtensionHandler {
    registry: TypeRegistryArc,
    metadata: Option<CelMetadataEnvelope>,
    error: Option<MetadataError>,
}

impl CelMetadataExtensionHandler {
    /// Creates a handler backed by the application's type registry.
    #[must_use]
    pub const fn new(registry: TypeRegistryArc) -> Self {
        Self {
            registry,
            metadata: None,
            error: None,
        }
    }
}

impl Clone for CelMetadataExtensionHandler {
    fn clone(&self) -> Self {
        Self::new(self.registry.clone())
    }
}

impl GltfExtensionHandler for CelMetadataExtensionHandler {
    fn dyn_clone(&self) -> Box<dyn ErasedGltfExtensionHandler> {
        Box::new(self.clone())
    }

    fn on_root(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf: &Gltf,
        _settings: &GltfLoaderSettings,
    ) {
        let Some(value) = gltf.extension_value(CEL_METADATA_EXTENSION) else {
            return;
        };
        let origin = MetadataSource::Extension {
            asset: PathBuf::from(load_context.path().path()),
            extension: CEL_METADATA_EXTENSION.to_owned(),
        };
        match serde_json::to_vec(value)
            .map_err(|error| MetadataError::Json {
                origin: origin.clone(),
                json_path: "$".to_owned(),
                error,
            })
            .and_then(|bytes| decode_metadata(&bytes, origin))
        {
            Ok(metadata) => self.metadata = Some(metadata),
            Err(error) => self.error = Some(error),
        }
    }

    fn on_gltf_node(
        &mut self,
        load_context: &mut LoadContext<'_>,
        gltf_node: &Node,
        entity: &mut EntityWorldMut<'_>,
    ) {
        if let Some(diagnostic) = self.error.as_ref().and_then(MetadataError::diagnostic) {
            entity.insert(diagnostic);
            return;
        }
        let Some(node_name) = gltf_node.name() else {
            return;
        };
        let Some(metadata) = self
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.nodes.get(node_name))
            .cloned()
        else {
            return;
        };
        let origin = MetadataSource::Extension {
            asset: PathBuf::from(load_context.path().path()),
            extension: CEL_METADATA_EXTENSION.to_owned(),
        };
        let registry = self.registry.read();
        let result = insert_reflected_components(
            entity,
            &metadata.components,
            ReflectionInsertContext {
                registry: &registry,
                origin: &origin,
                json_path: &format!("$.nodes.{node_name}.components"),
            },
        );
        drop(registry);
        entity.insert(crate::CelDomainMetadata {
            domains: metadata
                .domains
                .into_iter()
                .map(|(name, value)| (name, value.to_string()))
                .collect(),
        });
        if let Err(error) = result
            && let Some(diagnostic) = error.diagnostic()
        {
            entity.insert(diagnostic);
        }
    }
}
