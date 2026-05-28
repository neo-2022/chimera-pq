use serde::{Deserialize, Serialize};

use crate::{ContractValidate, ManifestCatalogBundle, validation::validate_non_empty_label};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CatalogPersistenceKind {
    DiscoverySnapshot,
    PublishSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogPersistenceDocument {
    pub schema_version: crate::ContractVersion,
    pub kind: CatalogPersistenceKind,
    pub source: String,
    pub saved_at_unix: u64,
    pub bundle: ManifestCatalogBundle,
}

impl CatalogPersistenceDocument {
    pub fn sample() -> Self {
        Self {
            schema_version: crate::ContractVersion::default(),
            kind: CatalogPersistenceKind::DiscoverySnapshot,
            source: "sample".to_string(),
            saved_at_unix: 1_717_000_000,
            bundle: ManifestCatalogBundle::sample(),
        }
    }
}

impl ContractValidate for CatalogPersistenceDocument {
    fn validate(&self) -> Result<(), String> {
        validate_non_empty_label(&self.source, "catalog_persistence source")?;
        if self.saved_at_unix == 0 {
            return Err("catalog_persistence saved_at_unix must be > 0".to_string());
        }
        self.bundle.validate()
    }
}
