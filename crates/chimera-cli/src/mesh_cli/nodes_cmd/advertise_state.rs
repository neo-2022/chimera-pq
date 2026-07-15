use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use chimera_mesh::validate_update_bootstrap_url;
use serde_json::Value;
use url::Url;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const PEER_UPDATE_STATE_KIND: &str = "chimera_peer_update_serve_state";
const PEER_UPDATE_STATE_STATUS_READY: &str = "ready";
const PEER_UPDATE_STATE_MAX_AGE_SEC: u64 = 3600;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PeerUpdateAdvertiseState {
    pub(super) update_bootstrap_url: String,
    pub(super) endpoint_generation: Option<u64>,
}

pub(super) fn read_resolved_peer_listen_from_state(path: &str) -> Result<Option<String>, String> {
    let path = Path::new(path);
    match fs::metadata(path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("read peer egress state metadata failed: {error}")),
    }
    validate_state_file_permissions(path)?;
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("read peer egress state failed: {error}")),
    };
    if text.trim().is_empty() {
        return Ok(None);
    }
    let mut resolved_peer_listen = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("resolved_peer_listen=") {
            let endpoint = rest.trim();
            if endpoint.is_empty() {
                return Err("peer egress state resolved_peer_listen is empty".to_string());
            }
            validate_resolved_peer_listen(endpoint)?;
            resolved_peer_listen = Some(endpoint.to_string());
            break;
        }
    }
    Ok(resolved_peer_listen)
}

pub(super) fn read_resolved_node_id_from_state(path: &str) -> Result<Option<String>, String> {
    let path = Path::new(path);
    match fs::metadata(path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("read peer egress state metadata failed: {error}")),
    }
    validate_state_file_permissions(path)?;
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("read peer egress state failed: {error}")),
    };
    if text.trim().is_empty() {
        return Ok(None);
    }
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("node_id=") {
            let node_id = rest.trim();
            if node_id.is_empty() {
                return Err("peer egress state node_id is empty".to_string());
            }
            return Ok(Some(node_id.to_string()));
        }
    }
    Ok(None)
}

#[cfg(test)]
pub(super) fn read_update_bootstrap_url_from_state(path: &str) -> Result<Option<String>, String> {
    Ok(read_peer_update_advertise_state(path)?.map(|state| state.update_bootstrap_url))
}

pub(super) fn read_peer_update_advertise_state(
    path: &str,
) -> Result<Option<PeerUpdateAdvertiseState>, String> {
    let path = Path::new(path);
    match fs::metadata(path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("read update state metadata failed: {error}")),
    }
    validate_state_file_permissions(path)?;
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("read update state failed: {error}")),
    };
    if text.trim().is_empty() {
        return Ok(None);
    }
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| format!("peer update state JSON invalid: {error}"))?;
    validate_string_field(&value, "kind", PEER_UPDATE_STATE_KIND)?;
    validate_string_field(&value, "status", PEER_UPDATE_STATE_STATUS_READY)?;
    validate_nonempty_string_field(&value, "listen")?;
    validate_endpoint_epoch(&value)?;
    validate_optional_endpoint_generation(&value)?;
    validate_semver_field(&value, "version")?;
    validate_sha256_field(&value, "sha256")?;
    let base_url = match value.get("base_url").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => Some(value),
        Some(_) => return Err("peer update state base_url is empty".to_string()),
        None => None,
    };
    let update_bootstrap_url = match value.get("update_bootstrap_url").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => Some(value),
        Some(_) => return Err("peer update state update_bootstrap_url is empty".to_string()),
        None => None,
    };
    match (base_url, update_bootstrap_url) {
        (None, None) => Ok(None),
        (Some(base_url), Some(update_bootstrap_url)) => {
            validate_bootstrap_state_contract(&value, base_url, update_bootstrap_url)?;
            Ok(Some(PeerUpdateAdvertiseState {
                update_bootstrap_url: update_bootstrap_url.to_string(),
                endpoint_generation: value.get("endpoint_generation").and_then(Value::as_u64),
            }))
        }
        _ => Err("peer update state base_url/update_bootstrap_url mismatch".to_string()),
    }
}

fn validate_resolved_peer_listen(endpoint: &str) -> Result<(), String> {
    let socket_addr: SocketAddr = endpoint
        .parse()
        .map_err(|_| "peer egress state resolved_peer_listen must be host:port".to_string())?;
    if socket_addr.port() == 0 {
        return Err("peer egress state resolved_peer_listen port must be > 0".to_string());
    }
    Ok(())
}

#[cfg(unix)]
fn validate_state_file_permissions(path: &Path) -> Result<(), String> {
    let mode = fs::metadata(path)
        .map_err(|error| format!("read update state metadata failed: {error}"))?
        .permissions()
        .mode()
        & 0o777;
    if mode & 0o077 != 0 {
        return Err("peer update state permissions must be private".to_string());
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_state_file_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn validate_bootstrap_state_contract(
    value: &Value,
    base_url: &str,
    update_bootstrap_url: &str,
) -> Result<(), String> {
    validate_update_bootstrap_url(update_bootstrap_url)?;
    let update_url = parse_http_url(update_bootstrap_url, "peer update bootstrap URL")?;
    let base_url = parse_http_url(base_url, "peer update base URL")?;
    require_canonical_base_url(&base_url)?;
    require_canonical_update_url(&update_url)?;
    require_same_origin(&base_url, &update_url, "peer update bootstrap URL")?;
    let listen = value
        .get("listen")
        .and_then(Value::as_str)
        .ok_or_else(|| "peer update state missing listen".to_string())?;
    parse_listen_port(listen)?;
    Ok(())
}

fn parse_http_url(raw: &str, label: &str) -> Result<Url, String> {
    let parsed = Url::parse(raw).map_err(|error| format!("{label} parse failed: {error}"))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(format!("{label} must be http(s)"));
    }
    if parsed.host_str().is_none() {
        return Err(format!("{label} missing host"));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(format!("{label} must not contain userinfo"));
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err(format!("{label} must not contain query or fragment"));
    }
    Ok(parsed)
}

fn require_canonical_base_url(url: &Url) -> Result<(), String> {
    if !matches!(url.path(), "" | "/") {
        return Err("peer update base URL must not include a path".to_string());
    }
    Ok(())
}

fn require_canonical_update_url(url: &Url) -> Result<(), String> {
    if url.path() != "/chimera.sh" {
        return Err("peer update bootstrap URL must end with /chimera.sh".to_string());
    }
    let host = url
        .host_str()
        .ok_or_else(|| "peer update bootstrap URL missing host".to_string())?;
    let host_lower = host.to_ascii_lowercase();
    if host_lower == "localhost"
        || host_lower == "0.0.0.0"
        || host_lower == "::"
        || host_lower == "::1"
        || host_lower.starts_with("127.")
    {
        return Err(
            "peer update bootstrap URL must not use loopback or unspecified host".to_string(),
        );
    }
    Ok(())
}

fn require_same_origin(base: &Url, candidate: &Url, label: &str) -> Result<(), String> {
    if base.scheme() != candidate.scheme()
        || base.host_str() != candidate.host_str()
        || base.port_or_known_default() != candidate.port_or_known_default()
    {
        return Err(format!("{label} origin differs from peer update base URL"));
    }
    Ok(())
}

fn parse_listen_port(listen: &str) -> Result<u16, String> {
    let listen = listen.trim();
    if listen.is_empty() {
        return Err("peer update state listen is empty".to_string());
    }
    if listen.starts_with('[') {
        let close = listen
            .find(']')
            .ok_or_else(|| "peer update state listen invalid IPv6 host".to_string())?;
        let tail = listen[(close + 1)..].trim();
        let port = tail
            .strip_prefix(':')
            .ok_or_else(|| "peer update state listen must be host:port".to_string())?;
        return parse_port_value(port);
    }
    let (_, port) = listen
        .rsplit_once(':')
        .ok_or_else(|| "peer update state listen must be host:port".to_string())?;
    parse_port_value(port)
}

fn parse_port_value(raw: &str) -> Result<u16, String> {
    let port = raw
        .trim()
        .parse::<u16>()
        .map_err(|_| "peer update state listen port is invalid".to_string())?;
    if port == 0 {
        return Err("peer update state listen port must be > 0".to_string());
    }
    Ok(port)
}

fn validate_string_field(value: &Value, key: &str, expected: &str) -> Result<(), String> {
    let actual = value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("peer update state missing {key}"))?;
    if actual != expected {
        return Err(format!("peer update state {key} mismatch"));
    }
    Ok(())
}

fn validate_nonempty_string_field(value: &Value, key: &str) -> Result<(), String> {
    let actual = value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("peer update state missing {key}"))?;
    if actual.trim().is_empty() {
        return Err(format!("peer update state {key} is empty"));
    }
    if actual != actual.trim() || actual.chars().any(char::is_whitespace) {
        return Err(format!(
            "peer update state {key} contains invalid whitespace"
        ));
    }
    Ok(())
}

fn validate_semver_field(value: &Value, key: &str) -> Result<(), String> {
    let version = value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("peer update state missing {key}"))?;
    let mut parts = version.split('.');
    for _ in 0..3 {
        let part = parts
            .next()
            .ok_or_else(|| format!("peer update state {key} must be semver X.Y.Z"))?;
        if part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(format!("peer update state {key} must be semver X.Y.Z"));
        }
    }
    if parts.next().is_some() {
        return Err(format!("peer update state {key} must be semver X.Y.Z"));
    }
    Ok(())
}

fn validate_sha256_field(value: &Value, key: &str) -> Result<(), String> {
    let sha = value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("peer update state missing {key}"))?;
    if sha.len() != 64 || !sha.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(format!("peer update state {key} is invalid"));
    }
    Ok(())
}

fn validate_endpoint_epoch(value: &Value) -> Result<(), String> {
    let epoch = value
        .get("endpoint_epoch")
        .and_then(Value::as_u64)
        .ok_or_else(|| "peer update state missing endpoint_epoch".to_string())?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock error: {error}"))?
        .as_secs();
    if epoch > now.saturating_add(60) {
        return Err("peer update state endpoint_epoch is in the future".to_string());
    }
    if now.saturating_sub(epoch) > PEER_UPDATE_STATE_MAX_AGE_SEC {
        return Err("peer update state is stale".to_string());
    }
    Ok(())
}

fn validate_optional_endpoint_generation(value: &Value) -> Result<(), String> {
    let Some(generation) = value.get("endpoint_generation") else {
        return Ok(());
    };
    let generation = generation
        .as_u64()
        .ok_or_else(|| "peer update state endpoint_generation is invalid".to_string())?;
    if generation == 0 {
        return Err("peer update state endpoint_generation must be > 0".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        read_resolved_node_id_from_state, read_resolved_peer_listen_from_state,
        read_update_bootstrap_url_from_state,
    };
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> Result<PathBuf, String> {
        let mut path = std::env::temp_dir();
        let unique = format!(
            "{}-{}-{}",
            prefix,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| format!("system clock error: {error}"))?
                .as_nanos()
        );
        path.push(unique);
        fs::create_dir_all(&path).map_err(|error| format!("create temp dir failed: {error}"))?;
        Ok(path)
    }

    #[test]
    fn read_state_missing_file_returns_none() -> Result<(), String> {
        let dir = temp_dir("chimera-peer-state-missing")?;
        let peer_path = dir.join("peer-egress.state");
        let update_path = dir.join("peer-update.state.json");
        let peer_path = peer_path
            .to_str()
            .ok_or_else(|| "peer path utf8".to_string())?;
        let update_path = update_path
            .to_str()
            .ok_or_else(|| "update path utf8".to_string())?;
        assert_eq!(read_resolved_peer_listen_from_state(peer_path)?, None);
        assert_eq!(read_update_bootstrap_url_from_state(update_path)?, None);
        let _ = fs::remove_dir_all(dir);
        Ok(())
    }

    #[test]
    fn read_peer_egress_state_rejects_invalid_endpoint() -> Result<(), String> {
        let dir = temp_dir("chimera-peer-egress-state-invalid")?;
        let state_path = dir.join("peer-egress.state");
        fs::write(
            &state_path,
            "mode=peer\nresolved_local_listen=127.0.0.1:11111\nresolved_peer_listen=not-an-endpoint\n",
        )
        .map_err(|error| format!("write state failed: {error}"))?;
        #[cfg(unix)]
        {
            fs::set_permissions(&state_path, fs::Permissions::from_mode(0o600))
                .map_err(|error| format!("chmod state failed: {error}"))?;
        }
        let state_path = state_path
            .to_str()
            .ok_or_else(|| "state path utf8".to_string())?;
        let error = match read_resolved_peer_listen_from_state(state_path) {
            Ok(_) => return Err("invalid peer endpoint must fail".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("host:port"));
        let _ = fs::remove_dir_all(dir);
        Ok(())
    }

    #[test]
    fn read_update_state_accepts_fresh_private_state() -> Result<(), String> {
        let dir = temp_dir("chimera-peer-update-state-ok")?;
        let state_path = dir.join("peer-update.state.json");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock error: {error}"))?
            .as_secs();
        fs::write(
            &state_path,
            format!(
                "{{\"kind\":\"chimera_peer_update_serve_state\",\"status\":\"ready\",\"listen\":\"127.0.0.1:18179\",\"base_url\":\"http://node.example:18179\",\"update_bootstrap_url\":\"http://node.example:18179/chimera.sh\",\"version\":\"1.2.3\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\",\"endpoint_epoch\":{now}}}"
            ),
        )
        .map_err(|error| format!("write state failed: {error}"))?;
        #[cfg(unix)]
        {
            fs::set_permissions(&state_path, fs::Permissions::from_mode(0o600))
                .map_err(|error| format!("chmod state failed: {error}"))?;
        }
        let state_path = state_path
            .to_str()
            .ok_or_else(|| "state path utf8".to_string())?;
        let url = read_update_bootstrap_url_from_state(state_path)?;
        assert_eq!(url.as_deref(), Some("http://node.example:18179/chimera.sh"));
        let _ = fs::remove_dir_all(dir);
        Ok(())
    }

    #[test]
    fn read_update_state_rejects_stale_or_public_state() -> Result<(), String> {
        let dir = temp_dir("chimera-peer-update-state-bad")?;
        let state_path = dir.join("peer-update.state.json");
        fs::write(
            &state_path,
            "{\"kind\":\"chimera_peer_update_serve_state\",\"status\":\"ready\",\"listen\":\"127.0.0.1:18179\",\"base_url\":\"http://127.0.0.1:18179\",\"update_bootstrap_url\":\"http://127.0.0.1:18179/chimera.sh\",\"version\":\"1.2.3\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\",\"endpoint_epoch\":1}",
        )
        .map_err(|error| format!("write state failed: {error}"))?;
        #[cfg(unix)]
        {
            fs::set_permissions(&state_path, fs::Permissions::from_mode(0o644))
                .map_err(|error| format!("chmod state failed: {error}"))?;
        }
        let state_path = state_path
            .to_str()
            .ok_or_else(|| "state path utf8".to_string())?;
        let error = match read_update_bootstrap_url_from_state(state_path) {
            Ok(_) => return Err("bad state must fail".to_string()),
            Err(error) => error,
        };
        assert!(
            error.contains("permissions") || error.contains("loopback") || error.contains("stale")
        );
        let _ = fs::remove_dir_all(dir);
        Ok(())
    }

    #[test]
    fn read_update_state_accepts_listen_port_mismatch() -> Result<(), String> {
        let dir = temp_dir("chimera-peer-update-state-mismatch")?;
        let state_path = dir.join("peer-update.state.json");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock error: {error}"))?
            .as_secs();
        fs::write(
            &state_path,
            format!(
                "{{\"kind\":\"chimera_peer_update_serve_state\",\"status\":\"ready\",\"listen\":\"127.0.0.1:18180\",\"base_url\":\"http://node.example:18179\",\"update_bootstrap_url\":\"http://node.example:18179/chimera.sh\",\"version\":\"1.2.3\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\",\"endpoint_epoch\":{now}}}"
            ),
        )
        .map_err(|error| format!("write state failed: {error}"))?;
        #[cfg(unix)]
        {
            fs::set_permissions(&state_path, fs::Permissions::from_mode(0o600))
                .map_err(|error| format!("chmod state failed: {error}"))?;
        }
        let state_path = state_path
            .to_str()
            .ok_or_else(|| "state path utf8".to_string())?;
        let url = read_update_bootstrap_url_from_state(state_path)?;
        assert_eq!(url.as_deref(), Some("http://node.example:18179/chimera.sh"));
        let _ = fs::remove_dir_all(dir);
        Ok(())
    }

    #[test]
    fn read_update_state_rejects_zero_endpoint_generation() -> Result<(), String> {
        let dir = temp_dir("chimera-peer-update-state-zero-generation")?;
        let state_path = dir.join("peer-update.state.json");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock error: {error}"))?
            .as_secs();
        fs::write(
            &state_path,
            format!(
                "{{\"kind\":\"chimera_peer_update_serve_state\",\"status\":\"ready\",\"listen\":\"127.0.0.1:18179\",\"base_url\":\"http://node.example:18179\",\"update_bootstrap_url\":\"http://node.example:18179/chimera.sh\",\"version\":\"1.2.3\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\",\"endpoint_epoch\":{now},\"endpoint_generation\":0}}"
            ),
        )
        .map_err(|error| format!("write state failed: {error}"))?;
        #[cfg(unix)]
        {
            fs::set_permissions(&state_path, fs::Permissions::from_mode(0o600))
                .map_err(|error| format!("chmod state failed: {error}"))?;
        }
        let state_path = state_path
            .to_str()
            .ok_or_else(|| "state path utf8".to_string())?;
        let error = match read_update_bootstrap_url_from_state(state_path) {
            Ok(_) => return Err("zero endpoint_generation must fail".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("endpoint_generation"));
        let _ = fs::remove_dir_all(dir);
        Ok(())
    }

    #[test]
    fn read_peer_egress_state_extracts_node_id() -> Result<(), String> {
        let dir = temp_dir("chimera-peer-egress-state-node-id")?;
        let state_path = dir.join("peer-egress.state");
        fs::write(
            &state_path,
            "mode=peer\nnode_id=remote-node\nresolved_local_listen=127.0.0.1:11111\nresolved_peer_listen=198.51.100.44:45678\n",
        )
        .map_err(|error| format!("write state failed: {error}"))?;
        #[cfg(unix)]
        {
            fs::set_permissions(&state_path, fs::Permissions::from_mode(0o600))
                .map_err(|error| format!("chmod state failed: {error}"))?;
        }
        let state_path = state_path
            .to_str()
            .ok_or_else(|| "state path utf8".to_string())?;
        assert_eq!(
            read_resolved_node_id_from_state(state_path)?,
            Some("remote-node".to_string())
        );
        assert_eq!(
            read_resolved_peer_listen_from_state(state_path)?,
            Some("198.51.100.44:45678".to_string())
        );
        let _ = fs::remove_dir_all(dir);
        Ok(())
    }
}
