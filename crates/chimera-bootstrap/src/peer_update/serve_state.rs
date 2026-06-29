use crate::Result;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

use super::serve_state_publish::{
    PeerUpdateStateAdvertisement, PeerUpdateStatePublishAction, decide_peer_update_state_publish,
    parse_existing_peer_update_state,
};

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
    let existing = read_existing_state(path)?;
    let decision = decide_peer_update_state_publish(
        existing.as_ref(),
        PeerUpdateStateAdvertisement {
            listen,
            base_url,
            update_bootstrap_url,
            version,
            sha256,
            endpoint_epoch,
        },
    )?;
    if decision.action == PeerUpdateStatePublishAction::Noop {
        ensure_state_file_private(path)?;
        return Ok(());
    }
    let body = decision
        .body
        .ok_or("peer update state publish changed without body")?;
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
        ensure_state_parent_not_world_writable(parent)?;
    }
    let tmp = path.with_extension("tmp");
    let _ = fs::remove_file(&tmp);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(&tmp)?;
    file.write_all(format!("{body}\n").as_bytes())?;
    file.sync_all()?;
    #[cfg(unix)]
    fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))?;
    fs::rename(tmp, path)?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

fn ensure_state_parent_not_world_writable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let mode = fs::metadata(path)?.permissions().mode() & 0o777;
        if mode & 0o002 != 0 {
            return Err("peer update state parent directory must not be world-writable".into());
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

fn ensure_state_file_private(path: &Path) -> Result<()> {
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

fn read_existing_state(
    path: &Path,
) -> Result<Option<super::serve_state_publish::ExistingPeerUpdateServeState>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(Box::new(error)),
    };
    parse_existing_peer_update_state(&text)
}

#[cfg(test)]
mod tests {
    use super::write_peer_update_state_file;
    use serde_json::Value;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

    fn temp_dir(prefix: &str) -> TestResult<PathBuf> {
        let mut base = std::env::temp_dir();
        let unique = format!(
            "{}-{}-{}",
            prefix,
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        );
        base.push(unique);
        fs::create_dir_all(&base)?;
        Ok(base)
    }

    fn now_epoch() -> TestResult<u64> {
        Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
    }

    fn state_json(
        listen: &str,
        base_url: &str,
        update_bootstrap_url: &str,
        endpoint_epoch: u64,
        endpoint_generation: u64,
    ) -> String {
        format!(
            "{{\"kind\":\"chimera_peer_update_serve_state\",\"status\":\"ready\",\"listen\":\"{listen}\",\"base_url\":\"{base_url}\",\"update_bootstrap_url\":\"{update_bootstrap_url}\",\"version\":\"1.2.3\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\",\"endpoint_epoch\":{endpoint_epoch},\"endpoint_generation\":{endpoint_generation}}}"
        )
    }

    #[test]
    fn state_file_new_publish_records_endpoint_generation() -> TestResult {
        let dir = temp_dir("chimera-peer-update-state-generation")?;
        let path = dir.join("peer-update-state.json");
        write_peer_update_state_file(
            &path,
            "127.0.0.1:18179",
            Some("http://node.example:18179"),
            Some("http://node.example:18179/chimera.sh"),
            "1.2.3",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )?;
        let state: Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
        assert_eq!(
            state.get("endpoint_generation").and_then(Value::as_u64),
            Some(1)
        );
        let _ = fs::remove_dir_all(dir);
        Ok(())
    }

    #[test]
    fn state_file_skips_fresh_noop_publish() -> TestResult {
        let dir = temp_dir("chimera-peer-update-state-noop")?;
        let path = dir.join("peer-update-state.json");
        let existing = state_json(
            "127.0.0.1:18179",
            "http://node.example:18179",
            "http://node.example:18179/chimera.sh",
            now_epoch()?,
            7,
        );
        fs::write(&path, &existing)?;
        write_peer_update_state_file(
            &path,
            "127.0.0.1:18179",
            Some("http://node.example:18179"),
            Some("http://node.example:18179/chimera.sh"),
            "1.2.3",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )?;
        assert_eq!(fs::read_to_string(&path)?, existing);
        let _ = fs::remove_dir_all(dir);
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn state_file_fresh_noop_preserves_private_permissions() -> TestResult {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_dir("chimera-peer-update-state-noop-private")?;
        let path = dir.join("peer-update-state.json");
        fs::write(
            &path,
            state_json(
                "127.0.0.1:18179",
                "http://node.example:18179",
                "http://node.example:18179/chimera.sh",
                now_epoch()?,
                7,
            ),
        )?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644))?;
        write_peer_update_state_file(
            &path,
            "127.0.0.1:18179",
            Some("http://node.example:18179"),
            Some("http://node.example:18179/chimera.sh"),
            "1.2.3",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )?;
        let mode = fs::metadata(&path)?.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        let _ = fs::remove_dir_all(dir);
        Ok(())
    }

    #[test]
    fn state_file_fresh_legacy_state_is_upgraded_with_generation() -> TestResult {
        let dir = temp_dir("chimera-peer-update-state-legacy-upgrade")?;
        let path = dir.join("peer-update-state.json");
        let now = now_epoch()?;
        fs::write(
            &path,
            format!(
                "{{\"kind\":\"chimera_peer_update_serve_state\",\"status\":\"ready\",\"listen\":\"127.0.0.1:18179\",\"base_url\":\"http://node.example:18179\",\"update_bootstrap_url\":\"http://node.example:18179/chimera.sh\",\"version\":\"1.2.3\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\",\"endpoint_epoch\":{now}}}"
            ),
        )?;
        write_peer_update_state_file(
            &path,
            "127.0.0.1:18179",
            Some("http://node.example:18179"),
            Some("http://node.example:18179/chimera.sh"),
            "1.2.3",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )?;
        let state: Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
        assert_eq!(
            state.get("endpoint_generation").and_then(Value::as_u64),
            Some(1)
        );
        let _ = fs::remove_dir_all(dir);
        Ok(())
    }

    #[test]
    fn state_file_endpoint_change_increments_generation() -> TestResult {
        let dir = temp_dir("chimera-peer-update-state-rebind")?;
        let path = dir.join("peer-update-state.json");
        fs::write(
            &path,
            state_json(
                "127.0.0.1:18179",
                "http://node.example:18179",
                "http://node.example:18179/chimera.sh",
                now_epoch()?,
                7,
            ),
        )?;
        write_peer_update_state_file(
            &path,
            "127.0.0.1:18180",
            Some("http://node.example:18180"),
            Some("http://node.example:18180/chimera.sh"),
            "1.2.3",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )?;
        let state: Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
        assert_eq!(
            state.get("listen").and_then(Value::as_str),
            Some("127.0.0.1:18180")
        );
        assert_eq!(
            state.get("update_bootstrap_url").and_then(Value::as_str),
            Some("http://node.example:18180/chimera.sh")
        );
        assert_eq!(
            state.get("endpoint_generation").and_then(Value::as_u64),
            Some(8)
        );
        let _ = fs::remove_dir_all(dir);
        Ok(())
    }

    #[test]
    fn state_file_malformed_existing_state_fails_without_rewrite() -> TestResult {
        let dir = temp_dir("chimera-peer-update-state-malformed")?;
        let path = dir.join("peer-update-state.json");
        fs::write(&path, "{not-json")?;

        let error = write_peer_update_state_file(
            &path,
            "127.0.0.1:18179",
            Some("http://node.example:18179"),
            Some("http://node.example:18179/chimera.sh"),
            "1.2.3",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .err()
        .ok_or("malformed existing state must fail")?;

        assert!(error.to_string().contains("JSON invalid"));
        assert_eq!(fs::read_to_string(&path)?, "{not-json");
        let _ = fs::remove_dir_all(dir);
        Ok(())
    }

    #[test]
    fn state_file_zero_generation_existing_state_fails_without_rewrite() -> TestResult {
        let dir = temp_dir("chimera-peer-update-state-zero-generation")?;
        let path = dir.join("peer-update-state.json");
        let existing = state_json(
            "127.0.0.1:18179",
            "http://node.example:18179",
            "http://node.example:18179/chimera.sh",
            now_epoch()?,
            0,
        );
        fs::write(&path, &existing)?;

        let error = write_peer_update_state_file(
            &path,
            "127.0.0.1:18179",
            Some("http://node.example:18179"),
            Some("http://node.example:18179/chimera.sh"),
            "1.2.3",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .err()
        .ok_or("zero-generation existing state must fail")?;

        assert!(error.to_string().contains("endpoint_generation"));
        assert_eq!(fs::read_to_string(&path)?, existing);
        let _ = fs::remove_dir_all(dir);
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn state_file_rejects_world_writable_parent_dir() -> TestResult {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_dir("chimera-peer-update-state-public-dir")?;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o777))?;
        let path = dir.join("peer-update-state.json");

        let error = write_peer_update_state_file(
            &path,
            "127.0.0.1:18179",
            Some("http://node.example:18179"),
            Some("http://node.example:18179/chimera.sh"),
            "1.2.3",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .err()
        .ok_or("world-writable parent directory must fail")?;

        assert!(error.to_string().contains("parent directory"));
        assert!(!path.exists());
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
        let _ = fs::remove_dir_all(dir);
        Ok(())
    }
}
