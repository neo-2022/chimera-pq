use crate::Result;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use url::Url;

use super::{RELEASE_ARCHIVE_ROUTE, RELEASE_CHECKSUM_ROUTE, RELEASE_METADATA_ROUTE};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PeerUpdateMetadata {
    status: String,
    kind: String,
    pub(crate) version: String,
    pub(crate) archive: String,
    pub(crate) checksum: String,
    pub(crate) sha256: String,
}

pub(crate) fn parse_peer_metadata_file(
    metadata_file: &Path,
    metadata_url: &str,
) -> Result<PeerUpdateMetadata> {
    let raw = fs::read_to_string(metadata_file)?;
    let metadata: PeerUpdateMetadata = serde_json::from_str(&raw)?;
    validate_peer_update_metadata(metadata, metadata_url)
}

fn validate_peer_update_metadata(
    metadata: PeerUpdateMetadata,
    metadata_url: &str,
) -> Result<PeerUpdateMetadata> {
    if metadata.status != "ok" {
        return Err("peer update metadata status is not ok".into());
    }
    if metadata.kind != "chimera_peer_update_metadata" {
        return Err("peer update metadata kind mismatch".into());
    }
    validate_release_version(&metadata.version)?;
    validate_sha256_hex(&metadata.sha256)?;

    let metadata_url = parse_update_url(metadata_url, "peer metadata URL")?;
    let archive_url = parse_update_url(&metadata.archive, "peer archive URL")?;
    let checksum_url = parse_update_url(&metadata.checksum, "peer checksum URL")?;
    require_exact_path(&metadata_url, RELEASE_METADATA_ROUTE, "peer metadata URL")?;
    require_exact_path(&archive_url, RELEASE_ARCHIVE_ROUTE, "peer archive URL")?;
    require_exact_path(&checksum_url, RELEASE_CHECKSUM_ROUTE, "peer checksum URL")?;
    require_same_origin(&metadata_url, &archive_url, "peer archive URL")?;
    require_same_origin(&metadata_url, &checksum_url, "peer checksum URL")?;
    Ok(metadata)
}

fn parse_update_url(raw: &str, label: &str) -> Result<Url> {
    if raw != raw.trim()
        || raw.contains('@')
        || raw.contains('"')
        || raw.contains('\'')
        || raw.contains('`')
        || raw.contains('$')
        || raw.contains('\\')
        || raw.contains('\r')
        || raw.contains('\n')
        || raw.contains('\t')
        || raw.chars().any(char::is_whitespace)
    {
        return Err(format!("{label} contains invalid characters").into());
    }
    let parsed = Url::parse(raw).map_err(|err| format!("{label} parse failed: {err}"))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(format!("{label} must be http(s)").into());
    }
    if parsed.host_str().is_none() {
        return Err(format!("{label} missing host").into());
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(format!("{label} must not contain userinfo").into());
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err(format!("{label} must not contain query or fragment").into());
    }
    Ok(parsed)
}

fn require_exact_path(url: &Url, expected_path: &str, label: &str) -> Result<()> {
    if url.path() != expected_path {
        return Err(format!("{label} path must be {expected_path}").into());
    }
    Ok(())
}

fn require_same_origin(base: &Url, candidate: &Url, label: &str) -> Result<()> {
    if base.scheme() != candidate.scheme()
        || base.host_str() != candidate.host_str()
        || base.port_or_known_default() != candidate.port_or_known_default()
    {
        return Err(format!("{label} origin differs from peer metadata URL").into());
    }
    Ok(())
}

fn validate_release_version(version: &str) -> Result<()> {
    let mut parts = version.split('.');
    for _ in 0..3 {
        let part = parts.next().ok_or("release version must be semver X.Y.Z")?;
        if part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()) {
            return Err("release version must be semver X.Y.Z".into());
        }
    }
    if parts.next().is_some() {
        return Err("release version must be semver X.Y.Z".into());
    }
    Ok(())
}

fn validate_sha256_hex(value: &str) -> Result<()> {
    if value.len() != 64 || !value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err("peer update metadata sha256 is invalid".into());
    }
    Ok(())
}
