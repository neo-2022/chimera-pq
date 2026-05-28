use serde::{Deserialize, Serialize};

use crate::ContractValidate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractVersion {
    pub major: u16,
    pub minor: u16,
}

impl Default for ContractVersion {
    fn default() -> Self {
        Self { major: 1, minor: 0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManifestKind {
    NodeRegistry,
    PublicSurface,
    Miniapp,
    Commerce,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestCatalogBundle {
    pub schema_version: ContractVersion,
    pub node_registry: crate::NodeRegistryManifest,
    pub public_surface: crate::PublicSurfaceManifest,
    pub miniapp: crate::MiniappManifest,
    pub publish_envelope: crate::PublishEnvelope,
    pub index_record: crate::CatalogIndexRecord,
}

impl ManifestCatalogBundle {
    pub fn sample() -> Self {
        Self {
            schema_version: ContractVersion::default(),
            node_registry: crate::NodeRegistryManifest::sample(),
            public_surface: crate::PublicSurfaceManifest::sample(),
            miniapp: crate::MiniappManifest::sample(),
            publish_envelope: crate::PublishEnvelope::sample(),
            index_record: crate::CatalogIndexRecord::sample(),
        }
    }
}

impl ContractValidate for ManifestCatalogBundle {
    fn validate(&self) -> Result<(), String> {
        self.node_registry.validate()?;
        self.public_surface.validate()?;
        self.miniapp.validate()?;
        self.publish_envelope.validate()?;
        self.index_record.validate()?;
        Ok(())
    }
}
