use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use ring::hmac;

use crate::mesh_cli::nodes_inventory::extract_flag_value;

use super::json::escape_json;

const DEFAULT_GUARD_LISTEN_BIND: &str = "0.0.0.0:0";

pub(super) fn guard_listen(args: &[String]) -> i32 {
    let bind = extract_flag_value(args, "--bind").unwrap_or(DEFAULT_GUARD_LISTEN_BIND);
    let pq_strict = proof_pq_strict_enabled(args);
    let proof_key_id = extract_flag_value(args, "--proof-key-id")
        .unwrap_or("mesh-shared-v1")
        .to_string();
    let proof_pq_key_id = extract_flag_value(args, "--proof-pq-key-id")
        .unwrap_or("mesh-pq-shared-v1")
        .to_string();
    let token_classic = extract_flag_value(args, "--proof-token-classic")
        .or_else(|| extract_flag_value(args, "--proof-token"));
    let token_pq = extract_flag_value(args, "--proof-token-pq")
        .or_else(|| extract_flag_value(args, "--proof-token"));
    if pq_strict
        && extract_flag_value(args, "--proof-token").is_some()
        && (extract_flag_value(args, "--proof-token-classic").is_none()
            || extract_flag_value(args, "--proof-token-pq").is_none())
    {
        eprintln!(
            "mesh nodes guard-listen error: pq_strict mode forbids legacy --proof-token; use --proof-token-classic + --proof-token-pq"
        );
        return 2;
    }
    let (Some(token_classic), Some(token_pq)) = (token_classic, token_pq) else {
        eprintln!(
            "mesh nodes guard-listen error: --proof-token-classic and --proof-token-pq are required (or legacy --proof-token)"
        );
        return 2;
    };
    let once = args.iter().any(|v| v == "--once");
    let listener = match TcpListener::bind(bind) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("mesh nodes guard-listen error: bind failed: {error}");
            return 2;
        }
    };
    let resolved_bind = listener
        .local_addr()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|_| bind.to_string());
    println!("guard_listen=ready bind={bind} resolved_bind={resolved_bind} once={once}");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let ok = handle_guard_conn(
                    stream,
                    token_classic,
                    token_pq,
                    proof_key_id.as_str(),
                    proof_pq_key_id.as_str(),
                    3_000,
                );
                if ok {
                    println!("guard_listen=proof_ok");
                    if once {
                        return 0;
                    }
                }
            }
            Err(error) => {
                eprintln!("mesh nodes guard-listen error: accept failed: {error}");
                return 2;
            }
        }
    }
    0
}

fn handle_guard_conn(
    mut stream: TcpStream,
    token_classic: &str,
    token_pq: &str,
    expected_key_id: &str,
    expected_pq_key_id: &str,
    timeout_ms: u64,
) -> bool {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(timeout_ms.max(1))));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(timeout_ms.max(1))));
    let mut reader = BufReader::new(&mut stream);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return false;
    }
    let challenge: serde_json::Value = match serde_json::from_str(line.trim()) {
        Ok(value) => value,
        Err(_) => return false,
    };
    if verify_guard_challenge(
        &challenge,
        token_classic,
        token_pq,
        expected_key_id,
        expected_pq_key_id,
    )
    .is_err()
    {
        return false;
    }
    stream
        .write_all(b"{\"kind\":\"mesh_guard_ack_v1\",\"status\":\"ok\"}\n")
        .is_ok()
}

pub(crate) fn verify_chimera_proof(
    endpoint: &str,
    token_classic: &str,
    token_pq: &str,
    key_id: &str,
    pq_key_id: &str,
    timeout_ms: u64,
) -> Result<(), String> {
    let mut addrs = endpoint
        .to_socket_addrs()
        .map_err(|error| format!("resolve_error:{error}"))?;
    let addr = addrs
        .next()
        .ok_or_else(|| "resolve_error:no_socket_addrs".to_string())?;
    let timeout = Duration::from_millis(timeout_ms.max(1));
    let mut stream = TcpStream::connect_timeout(&addr, timeout)
        .map_err(|error| format!("connect_error:{error}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| format!("set_read_timeout_error:{error}"))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|error| format!("set_write_timeout_error:{error}"))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system_clock_error:{error}"))?
        .as_secs();
    let expires = now.saturating_add(30);
    let nonce = format!("n-{now}-{endpoint}");
    let message = guard_sign_message(&nonce, now, expires);
    let classic_sig = guard_mac(token_classic, "chimera-classic-v1", &message);
    let pq_sig = guard_mac(token_pq, "chimera-pq-v1", &message);
    let hello = format!(
        "{{\"kind\":\"mesh_guard_challenge_v1\",\"key_id\":\"{}\",\"pq_key_id\":\"{}\",\"classic_alg\":\"hmac-sha256-v1\",\"pq_alg\":\"hmac-sha256-v1-placeholder\",\"nonce\":\"{}\",\"issued_at_unix\":{},\"expires_at_unix\":{},\"classic_sig\":\"{}\",\"pq_sig\":\"{}\"}}\n",
        escape_json(key_id),
        escape_json(pq_key_id),
        escape_json(&nonce),
        now,
        expires,
        classic_sig,
        pq_sig
    );
    stream
        .write_all(hello.as_bytes())
        .map_err(|error| format!("write_error:{error}"))?;
    let mut line = String::new();
    let mut reader = BufReader::new(stream);
    reader
        .read_line(&mut line)
        .map_err(|error| format!("read_error:{error}"))?;
    let ack: serde_json::Value =
        serde_json::from_str(line.trim()).map_err(|error| format!("invalid_ack_json:{error}"))?;
    if ack.get("kind").and_then(serde_json::Value::as_str) != Some("mesh_guard_ack_v1")
        || ack.get("status").and_then(serde_json::Value::as_str) != Some("ok")
    {
        return Err("invalid_proof_response".to_string());
    }
    Ok(())
}

fn guard_sign_message(nonce: &str, issued_at: u64, expires_at: u64) -> String {
    format!("nonce={nonce}\nissued_at_unix={issued_at}\nexpires_at_unix={expires_at}\n")
}

fn guard_mac(token: &str, domain: &str, message: &str) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, token.as_bytes());
    let payload = format!("{domain}\n{message}");
    let sig = hmac::sign(&key, payload.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(sig.as_ref())
}

pub(crate) fn verify_guard_challenge(
    challenge: &serde_json::Value,
    token_classic: &str,
    token_pq: &str,
    expected_key_id: &str,
    expected_pq_key_id: &str,
) -> Result<(), String> {
    const MAX_CLOCK_SKEW_SEC: u64 = 120;
    let kind = challenge
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "missing_kind".to_string())?;
    if kind != "mesh_guard_challenge_v1" {
        return Err("invalid_kind".to_string());
    }
    let classic_alg = challenge
        .get("classic_alg")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "missing_classic_alg".to_string())?;
    if classic_alg != "hmac-sha256-v1" {
        return Err("unsupported_classic_alg".to_string());
    }
    let pq_alg = challenge
        .get("pq_alg")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "missing_pq_alg".to_string())?;
    if pq_alg != "hmac-sha256-v1-placeholder" {
        return Err("unsupported_pq_alg".to_string());
    }
    let key_id = challenge
        .get("key_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "missing_key_id".to_string())?;
    if key_id != expected_key_id {
        return Err("unexpected_key_id".to_string());
    }
    let pq_key_id = challenge
        .get("pq_key_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "missing_pq_key_id".to_string())?;
    if pq_key_id != expected_pq_key_id {
        return Err("unexpected_pq_key_id".to_string());
    }
    let nonce = challenge
        .get("nonce")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "missing_nonce".to_string())?;
    if nonce.trim().is_empty() {
        return Err("blank_nonce".to_string());
    }
    let issued_at = challenge
        .get("issued_at_unix")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "missing_issued_at".to_string())?;
    let expires_at = challenge
        .get("expires_at_unix")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "missing_expires_at".to_string())?;
    if expires_at <= issued_at {
        return Err("invalid_ttl_window".to_string());
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system_clock_error:{error}"))?
        .as_secs();
    if issued_at > now.saturating_add(MAX_CLOCK_SKEW_SEC) {
        return Err("issued_at_too_far_in_future".to_string());
    }
    if expires_at < now {
        return Err("challenge_expired".to_string());
    }
    remember_guard_nonce(nonce, expires_at)?;
    let classic_sig = challenge
        .get("classic_sig")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "missing_classic_sig".to_string())?;
    let pq_sig = challenge
        .get("pq_sig")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "missing_pq_sig".to_string())?;
    let message = guard_sign_message(nonce, issued_at, expires_at);
    let expected_classic = guard_mac(token_classic, "chimera-classic-v1", &message);
    let expected_pq = guard_mac(token_pq, "chimera-pq-v1", &message);
    if classic_sig != expected_classic {
        return Err("classic_sig_mismatch".to_string());
    }
    if pq_sig != expected_pq {
        return Err("pq_sig_mismatch".to_string());
    }
    Ok(())
}

fn remember_guard_nonce(nonce: &str, expires_at: u64) -> Result<(), String> {
    static CACHE: OnceLock<Mutex<std::collections::BTreeMap<String, u64>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(std::collections::BTreeMap::new()));
    let mut guard = cache
        .lock()
        .map_err(|_| "guard_nonce_cache_lock_poisoned".to_string())?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system_clock_error:{error}"))?
        .as_secs();
    guard.retain(|_, expiry| *expiry >= now);
    if guard.contains_key(nonce) {
        return Err("guard_replay_nonce".to_string());
    }
    if guard.len() >= 4096
        && let Some(first) = guard.keys().next().cloned()
    {
        guard.remove(&first);
    }
    guard.insert(nonce.to_string(), expires_at);
    Ok(())
}

pub(crate) fn proof_pq_strict_enabled(args: &[String]) -> bool {
    if args.iter().any(|v| v == "--no-pq-strict") {
        return false;
    }
    if args.iter().any(|v| v == "--pq-strict") {
        return true;
    }
    match std::env::var("CHIMERA_MESH_PROOF_PQ_STRICT") {
        Ok(value) => {
            let v = value.trim().to_ascii_lowercase();
            !(v.is_empty() || v == "0" || v == "false" || v == "off" || v == "no")
        }
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::DEFAULT_GUARD_LISTEN_BIND;

    #[test]
    fn guard_listen_default_uses_os_selected_port() {
        assert_eq!(DEFAULT_GUARD_LISTEN_BIND, "0.0.0.0:0");
    }
}
