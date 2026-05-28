use serde::{Deserialize, Serialize};

use crate::{
    ContractValidate,
    validation::{validate_endpoint, validate_non_empty_label},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeRelayRole {
    Node,
    Gateway,
    Relay,
    Publisher,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeRegistryManifest {
    pub node_id: String,
    pub country_code: String,
    pub country_name: String,
    pub region: String,
    pub endpoint: String,
    pub relay_role: NodeRelayRole,
    pub capabilities: Vec<String>,
    pub freshness_unix: u64,
    pub ttl_sec: u64,
}

impl NodeRegistryManifest {
    pub fn sample() -> Self {
        Self {
            node_id: "node-nl-01".to_string(),
            country_code: "NL".to_string(),
            country_name: "Netherlands".to_string(),
            region: "eu-west".to_string(),
            endpoint: "node-nl-01.example.invalid:443".to_string(),
            relay_role: NodeRelayRole::Gateway,
            capabilities: vec!["gateway".to_string(), "mesh".to_string()],
            freshness_unix: 1_717_000_000,
            ttl_sec: 900,
        }
    }
}

impl ContractValidate for NodeRegistryManifest {
    fn validate(&self) -> Result<(), String> {
        validate_non_empty_label(&self.node_id, "node_registry node_id")?;
        validate_non_empty_label(&self.country_code, "node_registry country_code")?;
        validate_non_empty_label(&self.country_name, "node_registry country_name")?;
        validate_non_empty_label(&self.region, "node_registry region")?;
        validate_endpoint(&self.endpoint, "node_registry endpoint")?;
        if self.capabilities.is_empty() {
            return Err("node_registry capabilities is empty".to_string());
        }
        for capability in &self.capabilities {
            validate_non_empty_label(capability, "node_registry capability")?;
        }
        if self.freshness_unix == 0 {
            return Err("node_registry freshness_unix must be > 0".to_string());
        }
        if self.ttl_sec == 0 {
            return Err("node_registry ttl_sec must be > 0".to_string());
        }
        Ok(())
    }
}
