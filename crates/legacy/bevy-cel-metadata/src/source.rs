use std::{collections::BTreeMap, fmt, fs, path::PathBuf};

use serde::Deserialize;

use crate::{
    CEL_SCHEMA_VERSION, CelMetadataEnvelope, CelNodeMetadata, CelSchemaVersion, MetadataError,
};

#[derive(Deserialize)]
struct WireEnvelope {
    schema_version: u32,
    #[serde(default)]
    nodes: BTreeMap<String, CelNodeMetadata>,
}

/// Origin of a metadata document.
#[derive(bevy::prelude::Reflect, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MetadataSource {
    /// A named extension embedded in a glTF asset.
    Extension {
        /// glTF asset path.
        asset: PathBuf,
        /// Extension identifier.
        extension: String,
    },
    /// A standalone JSON sidecar.
    Sidecar(PathBuf),
}

impl fmt::Display for MetadataSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Extension { asset, extension } => {
                write!(formatter, "{} extension {extension}", asset.display())
            }
            Self::Sidecar(path) => path.display().fmt(formatter),
        }
    }
}

/// Borrowed metadata bytes and their diagnostic origin.
#[derive(Clone, Debug)]
pub struct MetadataInput<'a> {
    bytes: &'a [u8],
    source: MetadataSource,
}

impl<'a> MetadataInput<'a> {
    /// Creates one metadata input.
    #[must_use]
    pub const fn new(bytes: &'a [u8], source: MetadataSource) -> Self {
        Self { bytes, source }
    }
}

/// Merged metadata plus the selected source for every domain and component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedMetadata {
    /// Deterministically merged metadata.
    pub metadata: CelMetadataEnvelope,
    /// JSON-path keys mapped to the source that supplied the selected value.
    pub sources: BTreeMap<String, MetadataSource>,
}

struct DecodedMetadata {
    metadata: CelMetadataEnvelope,
    source: MetadataSource,
}

/// Decodes and validates one complete metadata document.
///
/// # Errors
///
/// Returns [`MetadataError::Json`] for malformed JSON or shape errors and
/// [`MetadataError::UnsupportedVersion`] for any schema other than version 1.
pub fn decode_metadata(
    bytes: &[u8],
    source: MetadataSource,
) -> Result<CelMetadataEnvelope, MetadataError> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let wire: WireEnvelope =
        serde_path_to_error::deserialize(&mut deserializer).map_err(|error| {
            MetadataError::Json {
                origin: source.clone(),
                json_path: json_path(error.path()),
                error: error.into_inner(),
            }
        })?;
    if wire.schema_version != CEL_SCHEMA_VERSION {
        return Err(MetadataError::UnsupportedVersion {
            origin: source,
            json_path: "$.schema_version".to_owned(),
            found: wire.schema_version,
            supported: CEL_SCHEMA_VERSION,
        });
    }
    Ok(CelMetadataEnvelope {
        schema_version: CelSchemaVersion::V1,
        nodes: wire.nodes,
    })
}

/// Reads a whole sidecar before decoding it, so callers never observe partial state.
///
/// # Errors
///
/// Returns [`MetadataError::Io`] when the file cannot be read in full, plus the decode errors
/// documented by [`decode_metadata`].
pub fn load_sidecar(path: impl Into<PathBuf>) -> Result<CelMetadataEnvelope, MetadataError> {
    let path = path.into();
    let bytes = fs::read(&path).map_err(|error| MetadataError::Io {
        path: path.clone(),
        error,
    })?;
    decode_metadata(&bytes, MetadataSource::Sidecar(path))
}

/// Resolves extension metadata over sidecar fallback at domain/component granularity.
///
/// # Errors
///
/// Returns the first input decode error, or [`MetadataError::MissingMetadata`] when both inputs
/// are absent.
pub fn resolve_metadata(
    extension: Option<MetadataInput<'_>>,
    sidecar: Option<MetadataInput<'_>>,
) -> Result<ResolvedMetadata, MetadataError> {
    let extension = extension.map(|input| decode_input(&input)).transpose()?;
    let sidecar = sidecar.map(|input| decode_input(&input)).transpose()?;
    match (extension, sidecar) {
        (None, None) => Err(MetadataError::MissingMetadata),
        (Some(decoded), None) | (None, Some(decoded)) => {
            Ok(with_sources(decoded.metadata, &decoded.source))
        }
        (Some(extension), Some(sidecar)) => Ok(merge(extension, sidecar)),
    }
}

fn decode_input(input: &MetadataInput<'_>) -> Result<DecodedMetadata, MetadataError> {
    decode_metadata(input.bytes, input.source.clone()).map(|metadata| DecodedMetadata {
        metadata,
        source: input.source.clone(),
    })
}

fn with_sources(metadata: CelMetadataEnvelope, source: &MetadataSource) -> ResolvedMetadata {
    let mut sources = BTreeMap::new();
    record_sources(&metadata, source, &mut sources);
    ResolvedMetadata { metadata, sources }
}

fn merge(extension: DecodedMetadata, mut sidecar: DecodedMetadata) -> ResolvedMetadata {
    let mut sources = BTreeMap::new();
    record_sources(&sidecar.metadata, &sidecar.source, &mut sources);
    for (node_name, extension_node) in extension.metadata.nodes {
        let node = sidecar.metadata.nodes.entry(node_name.clone()).or_default();
        for (domain, value) in extension_node.domains {
            sources.insert(
                domain_path(&node_name, "domains", &domain),
                extension.source.clone(),
            );
            node.domains.insert(domain, value);
        }
        for (type_path, value) in extension_node.components {
            sources.insert(
                domain_path(&node_name, "components", &type_path),
                extension.source.clone(),
            );
            node.components.insert(type_path, value);
        }
        if !extension_node.references.bones.is_empty() {
            sources.insert(
                domain_path(&node_name, "references", "bones"),
                extension.source.clone(),
            );
            node.references.bones = extension_node.references.bones;
        }
        if !extension_node.references.prefabs.is_empty() {
            sources.insert(
                domain_path(&node_name, "references", "prefabs"),
                extension.source.clone(),
            );
            node.references.prefabs = extension_node.references.prefabs;
        }
        if !extension_node.references.colliders.is_empty() {
            sources.insert(
                domain_path(&node_name, "references", "colliders"),
                extension.source.clone(),
            );
            node.references.colliders = extension_node.references.colliders;
        }
    }
    ResolvedMetadata {
        metadata: sidecar.metadata,
        sources,
    }
}

fn record_sources(
    metadata: &CelMetadataEnvelope,
    source: &MetadataSource,
    output: &mut BTreeMap<String, MetadataSource>,
) {
    for (node_name, node) in &metadata.nodes {
        for domain in node.domains.keys() {
            output.insert(domain_path(node_name, "domains", domain), source.clone());
        }
        for type_path in node.components.keys() {
            output.insert(
                domain_path(node_name, "components", type_path),
                source.clone(),
            );
        }
        if !node.references.bones.is_empty() {
            output.insert(
                domain_path(node_name, "references", "bones"),
                source.clone(),
            );
        }
        if !node.references.prefabs.is_empty() {
            output.insert(
                domain_path(node_name, "references", "prefabs"),
                source.clone(),
            );
        }
        if !node.references.colliders.is_empty() {
            output.insert(
                domain_path(node_name, "references", "colliders"),
                source.clone(),
            );
        }
    }
}

fn domain_path(node: &str, collection: &str, key: &str) -> String {
    format!("$.nodes.{node}.{collection}.{key}")
}

fn json_path(path: &serde_path_to_error::Path) -> String {
    let path = path.to_string();
    if path == "." {
        "$".to_owned()
    } else {
        format!("$.{path}")
    }
}
