use serde::{Deserialize, Serialize};

use crate::{ContractValidate, ManifestKind, validation::validate_non_empty_label};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PublicSurfaceVisibility {
    Private,
    Public,
    Unlisted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PublishStatus {
    Draft,
    PendingReview,
    Approved,
    Quarantined,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicSurfaceManifest {
    pub public_id: String,
    pub owner_key_id: String,
    pub title: String,
    pub summary: String,
    pub category: String,
    pub content_reference: String,
    pub visibility: PublicSurfaceVisibility,
    pub moderation_status: PublishStatus,
    pub tags: Vec<String>,
    pub freshness_unix: u64,
    pub ttl_sec: u64,
}

impl PublicSurfaceManifest {
    pub fn sample() -> Self {
        Self {
            public_id: "public-screen-01".to_string(),
            owner_key_id: "owner-key-01".to_string(),
            title: "Public Screen".to_string(),
            summary: "Sample public manifest for owner-controlled publishing.".to_string(),
            category: "demo".to_string(),
            content_reference: "weave://public/public-screen-01".to_string(),
            visibility: PublicSurfaceVisibility::Public,
            moderation_status: PublishStatus::Approved,
            tags: vec!["public".to_string(), "screen".to_string()],
            freshness_unix: 1_717_000_000,
            ttl_sec: 900,
        }
    }
}

impl ContractValidate for PublicSurfaceManifest {
    fn validate(&self) -> Result<(), String> {
        validate_non_empty_label(&self.public_id, "public_surface public_id")?;
        validate_non_empty_label(&self.owner_key_id, "public_surface owner_key_id")?;
        validate_non_empty_label(&self.title, "public_surface title")?;
        validate_non_empty_label(&self.summary, "public_surface summary")?;
        validate_non_empty_label(&self.category, "public_surface category")?;
        validate_non_empty_label(&self.content_reference, "public_surface content_reference")?;
        if self.tags.is_empty() {
            return Err("public_surface tags is empty".to_string());
        }
        for tag in &self.tags {
            validate_non_empty_label(tag, "public_surface tag")?;
        }
        if self.freshness_unix == 0 {
            return Err("public_surface freshness_unix must be > 0".to_string());
        }
        if self.ttl_sec == 0 {
            return Err("public_surface ttl_sec must be > 0".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishEnvelope {
    pub schema_version: crate::ContractVersion,
    pub record_kind: ManifestKind,
    pub record_id: String,
    pub publisher_key_id: String,
    pub shard_key: crate::CatalogShardKey,
    pub issued_at_unix: u64,
    pub expires_at_unix: Option<u64>,
    pub nonce: String,
    pub payload_hash: String,
    pub signature: String,
}

impl PublishEnvelope {
    pub fn sample() -> Self {
        Self {
            schema_version: crate::ContractVersion::default(),
            record_kind: ManifestKind::PublicSurface,
            record_id: "public-screen-01".to_string(),
            publisher_key_id: "publisher-key-01".to_string(),
            shard_key: crate::CatalogShardKey::sample(),
            issued_at_unix: 1_717_000_000,
            expires_at_unix: Some(1_717_000_900),
            nonce: "sample-nonce-01".to_string(),
            payload_hash: "sha256:sample-payload-hash".to_string(),
            signature: "ed25519:sample-signature".to_string(),
        }
    }
}

impl ContractValidate for PublishEnvelope {
    fn validate(&self) -> Result<(), String> {
        validate_non_empty_label(&self.record_id, "publish_envelope record_id")?;
        validate_non_empty_label(&self.publisher_key_id, "publish_envelope publisher_key_id")?;
        self.shard_key.validate()?;
        if self.issued_at_unix == 0 {
            return Err("publish_envelope issued_at_unix must be > 0".to_string());
        }
        if let Some(expires_at_unix) = self.expires_at_unix
            && expires_at_unix <= self.issued_at_unix
        {
            return Err(
                "publish_envelope expires_at_unix must be greater than issued_at_unix".to_string(),
            );
        }
        validate_non_empty_label(&self.nonce, "publish_envelope nonce")?;
        validate_non_empty_label(&self.payload_hash, "publish_envelope payload_hash")?;
        validate_non_empty_label(&self.signature, "publish_envelope signature")?;
        Ok(())
    }
}
