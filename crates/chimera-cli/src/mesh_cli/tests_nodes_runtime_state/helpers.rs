use base64::Engine;

pub(super) fn random_u64() -> u64 {
    use rand::RngCore;
    let mut bytes = [0_u8; 8];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    u64::from_le_bytes(bytes)
}

pub(super) fn build_guard_challenge(
    token: &str,
    nonce: &str,
    issued_at_unix: u64,
    expires_at_unix: u64,
) -> serde_json::Value {
    let message = format!(
        "nonce={}\nissued_at_unix={}\nexpires_at_unix={}\n",
        nonce, issued_at_unix, expires_at_unix
    );
    let classic_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, token.as_bytes());
    let classic_payload = format!("chimera-classic-v1\n{message}");
    let classic_sig = base64::engine::general_purpose::STANDARD
        .encode(ring::hmac::sign(&classic_key, classic_payload.as_bytes()).as_ref());
    let pq_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, token.as_bytes());
    let pq_payload = format!("chimera-pq-v1\n{message}");
    let pq_sig = base64::engine::general_purpose::STANDARD
        .encode(ring::hmac::sign(&pq_key, pq_payload.as_bytes()).as_ref());
    serde_json::json!({
        "kind":"mesh_guard_challenge_v1",
        "key_id":"mesh-shared-v1",
        "pq_key_id":"mesh-pq-shared-v1",
        "classic_alg":"hmac-sha256-v1",
        "pq_alg":"hmac-sha256-v1-placeholder",
        "nonce":nonce,
        "issued_at_unix":issued_at_unix,
        "expires_at_unix":expires_at_unix,
        "classic_sig":classic_sig,
        "pq_sig":pq_sig
    })
}

pub(super) fn error_code_eq<T>(result: Result<T, String>, expected: &str) -> bool {
    matches!(result, Err(actual) if actual == expected)
}
