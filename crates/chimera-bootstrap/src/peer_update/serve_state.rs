use crate::Result;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
struct PeerUpdateServeState<'a> {
    kind: &'static str,
    status: &'static str,
    listen: &'a str,
    base_url: Option<&'a str>,
    update_bootstrap_url: Option<&'a str>,
    version: &'a str,
    sha256: &'a str,
    endpoint_epoch: u64,
}

pub(super) fn write_peer_update_state_file(
    path: &Path,
    listen: &str,
    base_url: Option<&str>,
    update_bootstrap_url: Option<&str>,
    version: &str,
    sha256: &str,
) -> Result<()> {
    let endpoint_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock error: {error}"))?
        .as_secs();
    let state = PeerUpdateServeState {
        kind: "chimera_peer_update_serve_state",
        status: "ready",
        listen,
        base_url,
        update_bootstrap_url,
        version,
        sha256,
        endpoint_epoch,
    };
    let body = serde_json::to_string_pretty(&state)?;
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, format!("{body}\n"))?;
    fs::rename(tmp, path)?;
    Ok(())
}
