#![forbid(unsafe_code)]

mod catalog;
mod manifest;
mod miniapp;
mod node_registry;
mod persistence;
mod publish;
mod validation;

pub use catalog::{CatalogIndexRecord, CatalogShardKey};
pub use manifest::{ContractVersion, ManifestCatalogBundle, ManifestKind};
pub use miniapp::{MiniappManifest, MiniappVisibility};
pub use node_registry::{NodeRegistryManifest, NodeRelayRole};
pub use persistence::{CatalogPersistenceDocument, CatalogPersistenceKind};
pub use publish::{PublicSurfaceManifest, PublicSurfaceVisibility, PublishEnvelope, PublishStatus};

pub trait ContractValidate {
    fn validate(&self) -> Result<(), String>;
}

#[cfg(test)]
mod tests;
