use serde::{Deserialize, Serialize};

use crate::{ContractValidate, validation::validate_non_empty_label};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MiniappVisibility {
    Private,
    Public,
    Unlisted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MiniappManifest {
    pub app_id: String,
    pub owner_key_id: String,
    pub name: String,
    pub version: String,
    pub entrypoint: String,
    pub description: String,
    pub icon_uri: Option<String>,
    pub visibility: MiniappVisibility,
    pub tags: Vec<String>,
    pub freshness_unix: u64,
    pub ttl_sec: u64,
}

impl MiniappManifest {
    pub fn sample() -> Self {
        Self {
            app_id: "miniapp-weather-demo".to_string(),
            owner_key_id: "owner-key-01".to_string(),
            name: "Weather Demo".to_string(),
            version: "1.0.0".to_string(),
            entrypoint: "weave://miniapp/weather-demo".to_string(),
            description: "Sample miniapp manifest for the shared public surface.".to_string(),
            icon_uri: Some("weave://assets/weather-demo/icon.png".to_string()),
            visibility: MiniappVisibility::Public,
            tags: vec!["demo".to_string(), "miniapp".to_string()],
            freshness_unix: 1_717_000_000,
            ttl_sec: 900,
        }
    }
}

impl ContractValidate for MiniappManifest {
    fn validate(&self) -> Result<(), String> {
        validate_non_empty_label(&self.app_id, "miniapp app_id")?;
        validate_non_empty_label(&self.owner_key_id, "miniapp owner_key_id")?;
        validate_non_empty_label(&self.name, "miniapp name")?;
        validate_non_empty_label(&self.version, "miniapp version")?;
        validate_non_empty_label(&self.entrypoint, "miniapp entrypoint")?;
        validate_non_empty_label(&self.description, "miniapp description")?;
        if let Some(icon_uri) = self.icon_uri.as_ref() {
            validate_non_empty_label(icon_uri, "miniapp icon_uri")?;
        }
        if self.tags.is_empty() {
            return Err("miniapp tags is empty".to_string());
        }
        for tag in &self.tags {
            validate_non_empty_label(tag, "miniapp tag")?;
        }
        if self.freshness_unix == 0 {
            return Err("miniapp freshness_unix must be > 0".to_string());
        }
        if self.ttl_sec == 0 {
            return Err("miniapp ttl_sec must be > 0".to_string());
        }
        Ok(())
    }
}
