use crate::mesh_cli::nodes_inventory::build_discovery_signature_message;
use base64::Engine as _;
use ed25519_dalek::{Signer, SigningKey};
use rand::{RngCore, rngs::OsRng};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

pub(super) fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_else(|err| unreachable!("system time error: {err}"))
}

pub(super) fn random_u64() -> u64 {
    let mut bytes = [0_u8; 8];
    OsRng.fill_bytes(&mut bytes);
    u64::from_le_bytes(bytes)
}

pub(super) fn serve_json_once(body: String) -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind http listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read http listener addr failed: {err}"));
    thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .unwrap_or_else(|err| unreachable!("accept failed: {err}"));
        let mut buffer = [0u8; 1024];
        let _ = stream.read(&mut buffer);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .unwrap_or_else(|err| unreachable!("write response failed: {err}"));
    });
    format!("http://{addr}/nodes")
}

pub(super) fn build_signed_payload(
    signing_key: &SigningKey,
    key_id: &str,
    nonce: &str,
    issued_at_unix: u64,
    expires_at_unix: u64,
    nodes_json: &str,
) -> String {
    let nodes_value: serde_json::Value = serde_json::from_str(nodes_json)
        .unwrap_or_else(|err| unreachable!("valid nodes json: {err}"));
    let msg =
        build_discovery_signature_message(1, issued_at_unix, expires_at_unix, nonce, &nodes_value)
            .unwrap_or_else(|err| unreachable!("build message failed: {err}"));
    let signature = signing_key.sign(&msg);
    let signature_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
    format!(
        "{{\"contract_version\":1,\"key_id\":\"{}\",\"issued_at_unix\":{},\"expires_at_unix\":{},\"nonce\":\"{}\",\"signature\":\"{}\",\"nodes\":{}}}",
        key_id, issued_at_unix, expires_at_unix, nonce, signature_b64, nodes_json
    )
}

pub(super) fn generate_signing_key() -> SigningKey {
    let mut rng = OsRng;
    let mut secret = [0u8; 32];
    rng.fill_bytes(&mut secret);
    SigningKey::from_bytes(&secret)
}
