#![expect(
    missing_docs,
    reason = "serde field names are the versioned CEL wire contract"
)]

use bevy_gltf_metadata::CelNodeMetadata;
use serde::{Deserialize, Serialize};

use crate::{CelNprMaterialAuthoring, CelNprMaterialEntry, CelNprObjectAuthoring};

pub const CEL_NPR_DOMAIN: &str = "bevy_cel.npr";
pub const CEL_NPR_MATERIALS_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CelNprAuthoring {
    pub material: CelNprMaterialAuthoring,
    #[serde(default)]
    pub materials_version: u32,
    #[serde(default)]
    pub materials: Vec<CelNprMaterialEntry>,
    pub object: CelNprObjectAuthoring,
}

#[derive(Debug, thiserror::Error)]
pub enum CelNprMetadataError {
    #[error("node has no {CEL_NPR_DOMAIN} domain")]
    MissingDomain,
    #[error("{CEL_NPR_DOMAIN} domain is malformed: {0}")]
    Malformed(#[from] serde_json::Error),
    #[error("{CEL_NPR_DOMAIN} material list version {found} is unsupported; expected {supported}")]
    UnsupportedMaterialsVersion { found: u32, supported: u32 },
}

impl CelNprAuthoring {
    /// Decodes the stable NPR domain from one schema-v1 CEL node.
    ///
    /// # Errors
    /// Returns [`CelNprMetadataError`] when the domain is absent or malformed.
    pub fn from_node(node: &CelNodeMetadata) -> Result<Self, CelNprMetadataError> {
        let value = node
            .domains
            .get(CEL_NPR_DOMAIN)
            .ok_or(CelNprMetadataError::MissingDomain)?;
        let authored: Self = serde_json::from_value(value.clone())?;
        if authored.materials_version > CEL_NPR_MATERIALS_VERSION {
            return Err(CelNprMetadataError::UnsupportedMaterialsVersion {
                found: authored.materials_version,
                supported: CEL_NPR_MATERIALS_VERSION,
            });
        }
        Ok(authored)
    }

    #[must_use]
    pub fn materials(&self) -> &[CelNprMaterialEntry] {
        &self.materials
    }

    #[must_use]
    pub fn default_material(&self) -> &CelNprMaterialAuthoring {
        self.materials
            .first()
            .map_or(&self.material, |entry| &entry.settings)
    }

    #[must_use]
    pub fn material_for_slot(&self, slot: u32) -> Option<&CelNprMaterialAuthoring> {
        if self.materials.is_empty() && slot == 0 {
            return Some(&self.material);
        }
        self.materials
            .iter()
            .find(|entry| entry.slot == slot)
            .map(|entry| &entry.settings)
    }
}
