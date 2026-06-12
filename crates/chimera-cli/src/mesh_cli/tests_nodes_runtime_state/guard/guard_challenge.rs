use super::super::helpers::{build_guard_challenge, error_code_eq, random_u64};
use crate::mesh_cli::nodes_cmd::verify_guard_challenge;
use base64::Engine;

#[test]
fn nodes_verify_guard_challenge_rejects_unexpected_key_id() {
    let token = "tok-proof-c";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("time error: {err}"))
        .as_secs();
    let expires = now.saturating_add(30);
    let nonce = format!("n-{}-test", random_u64());
    let message = format!(
        "nonce={}\nissued_at_unix={}\nexpires_at_unix={}\n",
        nonce, now, expires
    );
    let classic_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, token.as_bytes());
    let classic_payload = format!("chimera-classic-v1\n{message}");
    let classic_sig = base64::engine::general_purpose::STANDARD
        .encode(ring::hmac::sign(&classic_key, classic_payload.as_bytes()).as_ref());
    let pq_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, token.as_bytes());
    let pq_payload = format!("chimera-pq-v1\n{message}");
    let pq_sig = base64::engine::general_purpose::STANDARD
        .encode(ring::hmac::sign(&pq_key, pq_payload.as_bytes()).as_ref());
    let challenge = serde_json::json!({
        "kind":"mesh_guard_challenge_v1",
        "key_id":"mesh-shared-v2",
        "pq_key_id":"mesh-pq-shared-v1",
        "classic_alg":"hmac-sha256-v1",
        "pq_alg":"hmac-sha256-v1-placeholder",
        "nonce":nonce,
        "issued_at_unix":now,
        "expires_at_unix":expires,
        "classic_sig":classic_sig,
        "pq_sig":pq_sig
    });
    let result = verify_guard_challenge(
        &challenge,
        token,
        token,
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
    );
    assert!(error_code_eq(result, "unexpected_key_id"));
}

#[test]
fn nodes_verify_guard_challenge_rejects_unexpected_pq_key_id() {
    let token = "tok-proof-d";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("time error: {err}"))
        .as_secs();
    let expires = now.saturating_add(30);
    let nonce = format!("n-{}-test", random_u64());
    let message = format!(
        "nonce={}\nissued_at_unix={}\nexpires_at_unix={}\n",
        nonce, now, expires
    );
    let classic_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, token.as_bytes());
    let classic_payload = format!("chimera-classic-v1\n{message}");
    let classic_sig = base64::engine::general_purpose::STANDARD
        .encode(ring::hmac::sign(&classic_key, classic_payload.as_bytes()).as_ref());
    let pq_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, token.as_bytes());
    let pq_payload = format!("chimera-pq-v1\n{message}");
    let pq_sig = base64::engine::general_purpose::STANDARD
        .encode(ring::hmac::sign(&pq_key, pq_payload.as_bytes()).as_ref());
    let challenge = serde_json::json!({
        "kind":"mesh_guard_challenge_v1",
        "key_id":"mesh-shared-v1",
        "pq_key_id":"mesh-pq-shared-v2",
        "classic_alg":"hmac-sha256-v1",
        "pq_alg":"hmac-sha256-v1-placeholder",
        "nonce":nonce,
        "issued_at_unix":now,
        "expires_at_unix":expires,
        "classic_sig":classic_sig,
        "pq_sig":pq_sig
    });
    let result = verify_guard_challenge(
        &challenge,
        token,
        token,
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
    );
    assert!(error_code_eq(result, "unexpected_pq_key_id"));
}

#[test]
fn nodes_verify_guard_challenge_rejects_missing_key_id() {
    let token = "tok-proof-e";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("time error: {err}"))
        .as_secs();
    let expires = now.saturating_add(30);
    let nonce = format!("n-{}-test", random_u64());
    let message = format!(
        "nonce={}\nissued_at_unix={}\nexpires_at_unix={}\n",
        nonce, now, expires
    );
    let classic_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, token.as_bytes());
    let classic_payload = format!("chimera-classic-v1\n{message}");
    let classic_sig = base64::engine::general_purpose::STANDARD
        .encode(ring::hmac::sign(&classic_key, classic_payload.as_bytes()).as_ref());
    let pq_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, token.as_bytes());
    let pq_payload = format!("chimera-pq-v1\n{message}");
    let pq_sig = base64::engine::general_purpose::STANDARD
        .encode(ring::hmac::sign(&pq_key, pq_payload.as_bytes()).as_ref());
    let challenge = serde_json::json!({
        "kind":"mesh_guard_challenge_v1",
        "pq_key_id":"mesh-pq-shared-v1",
        "classic_alg":"hmac-sha256-v1",
        "pq_alg":"hmac-sha256-v1-placeholder",
        "nonce":nonce,
        "issued_at_unix":now,
        "expires_at_unix":expires,
        "classic_sig":classic_sig,
        "pq_sig":pq_sig
    });
    let result = verify_guard_challenge(
        &challenge,
        token,
        token,
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
    );
    assert!(error_code_eq(result, "missing_key_id"));
}

#[test]
fn nodes_verify_guard_challenge_rejects_missing_pq_key_id() {
    let token = "tok-proof-f";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("time error: {err}"))
        .as_secs();
    let expires = now.saturating_add(30);
    let nonce = format!("n-{}-test", random_u64());
    let message = format!(
        "nonce={}\nissued_at_unix={}\nexpires_at_unix={}\n",
        nonce, now, expires
    );
    let classic_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, token.as_bytes());
    let classic_payload = format!("chimera-classic-v1\n{message}");
    let classic_sig = base64::engine::general_purpose::STANDARD
        .encode(ring::hmac::sign(&classic_key, classic_payload.as_bytes()).as_ref());
    let pq_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, token.as_bytes());
    let pq_payload = format!("chimera-pq-v1\n{message}");
    let pq_sig = base64::engine::general_purpose::STANDARD
        .encode(ring::hmac::sign(&pq_key, pq_payload.as_bytes()).as_ref());
    let challenge = serde_json::json!({
        "kind":"mesh_guard_challenge_v1",
        "key_id":"mesh-shared-v1",
        "classic_alg":"hmac-sha256-v1",
        "pq_alg":"hmac-sha256-v1-placeholder",
        "nonce":nonce,
        "issued_at_unix":now,
        "expires_at_unix":expires,
        "classic_sig":classic_sig,
        "pq_sig":pq_sig
    });
    let result = verify_guard_challenge(
        &challenge,
        token,
        token,
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
    );
    assert!(error_code_eq(result, "missing_pq_key_id"));
}

#[test]
fn nodes_verify_guard_challenge_rejects_invalid_ttl_window() {
    let token = "tok-proof-g";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("time error: {err}"))
        .as_secs();
    let nonce = format!("n-{}-test", random_u64());
    let challenge = build_guard_challenge(token, &nonce, now, now);
    let result = verify_guard_challenge(
        &challenge,
        token,
        token,
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
    );
    assert!(error_code_eq(result, "invalid_ttl_window"));
}

#[test]
fn nodes_verify_guard_challenge_rejects_issued_at_too_far_in_future() {
    let token = "tok-proof-h";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("time error: {err}"))
        .as_secs();
    let nonce = format!("n-{}-test", random_u64());
    let issued_at = now.saturating_add(3600);
    let challenge = build_guard_challenge(token, &nonce, issued_at, issued_at.saturating_add(30));
    let result = verify_guard_challenge(
        &challenge,
        token,
        token,
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
    );
    assert!(error_code_eq(result, "issued_at_too_far_in_future"));
}

#[test]
fn nodes_verify_guard_challenge_rejects_expired_challenge() {
    let token = "tok-proof-i";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("time error: {err}"))
        .as_secs();
    let nonce = format!("n-{}-test", random_u64());
    let issued_at = now.saturating_sub(120);
    let challenge = build_guard_challenge(token, &nonce, issued_at, now.saturating_sub(1));
    let result = verify_guard_challenge(
        &challenge,
        token,
        token,
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
    );
    assert!(error_code_eq(result, "challenge_expired"));
}

#[test]
fn nodes_verify_guard_challenge_rejects_replay_nonce() {
    let token = "tok-proof-j";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|err| unreachable!("time error: {err}"))
        .as_secs();
    let nonce = format!("n-{}-test", random_u64());
    let challenge = build_guard_challenge(token, &nonce, now, now.saturating_add(30));
    let first = verify_guard_challenge(
        &challenge,
        token,
        token,
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
    );
    assert!(first.is_ok());
    let second = verify_guard_challenge(
        &challenge,
        token,
        token,
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
    );
    assert!(error_code_eq(second, "guard_replay_nonce"));
}
