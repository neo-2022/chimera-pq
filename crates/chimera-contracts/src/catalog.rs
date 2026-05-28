use serde::{Deserialize, Serialize};

use crate::{ContractValidate, ManifestKind, validation::validate_non_empty_label};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogShardKey {
    pub country_code: String,
    pub region: String,
    pub topic: String,
    pub namespace: String,
}

impl CatalogShardKey {
    pub fn sample() -> Self {
        Self {
            country_code: "NL".to_string(),
            region: "eu-west".to_string(),
            topic: "public-screen".to_string(),
            namespace: "weave-public".to_string(),
        }
    }
}

impl ContractValidate for CatalogShardKey {
    fn validate(&self) -> Result<(), String> {
        validate_non_empty_label(&self.country_code, "catalog_shard country_code")?;
        validate_non_empty_label(&self.region, "catalog_shard region")?;
        validate_non_empty_label(&self.topic, "catalog_shard topic")?;
        validate_non_empty_label(&self.namespace, "catalog_shard namespace")?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogIndexRecord {
    pub record_kind: ManifestKind,
    pub record_id: String,
    pub title: String,
    pub country_code: String,
    pub region: String,
    pub topic: String,
    pub endpoint: String,
    pub visibility: crate::PublicSurfaceVisibility,
    pub freshness_unix: u64,
    pub ttl_sec: u64,
}

impl CatalogIndexRecord {
    pub fn sample() -> Self {
        Self {
            record_kind: ManifestKind::PublicSurface,
            record_id: "public-screen-01".to_string(),
            title: "Public Screen".to_string(),
            country_code: "NL".to_string(),
            region: "eu-west".to_string(),
            topic: "public-screen".to_string(),
            endpoint: "public-screen-01.example.invalid:443".to_string(),
            visibility: crate::PublicSurfaceVisibility::Public,
            freshness_unix: 1_717_000_000,
            ttl_sec: 900,
        }
    }
}

impl ContractValidate for CatalogIndexRecord {
    fn validate(&self) -> Result<(), String> {
        validate_non_empty_label(&self.record_id, "catalog_index record_id")?;
        validate_non_empty_label(&self.title, "catalog_index title")?;
        validate_non_empty_label(&self.country_code, "catalog_index country_code")?;
        validate_non_empty_label(&self.region, "catalog_index region")?;
        validate_non_empty_label(&self.topic, "catalog_index topic")?;
        crate::validation::validate_endpoint(&self.endpoint, "catalog_index endpoint")?;
        if self.freshness_unix == 0 {
            return Err("catalog_index freshness_unix must be > 0".to_string());
        }
        if self.ttl_sec == 0 {
            return Err("catalog_index ttl_sec must be > 0".to_string());
        }
        Ok(())
    }
}
