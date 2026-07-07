use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use ed25519_dalek::{Signer, SigningKey};

use super::parse_discovery_nodes_json;

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn signed_envelope(nodes: &serde_json::Value) -> (String, String) {
    let seed: [u8; 32] = rand::random();
    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();
    let pubkey_b64 = base64::engine::general_purpose::STANDARD.encode(verifying_key.to_bytes());

    let issued_at = now_unix();
    let expires_at = issued_at + 120;
    let nonce = format!("test-nonce-{}-{}", issued_at, rand::random::<u64>());
    let nodes_compact = serde_json::to_string(nodes).unwrap();
    let message = format!(
        "contract_version=1\nissued_at_unix={issued_at}\nexpires_at_unix={expires_at}\nnonce={nonce}\nnodes={nodes_compact}\n"
    );
    let signature = signing_key.sign(message.as_bytes());
    let signature_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());

    let envelope = serde_json::json!({
        "contract_version": 1,
        "issued_at_unix": issued_at,
        "expires_at_unix": expires_at,
        "nonce": nonce,
        "key_id": "default",
        "nodes": nodes,
        "signature": signature_b64,
    });

    (serde_json::to_string(&envelope).unwrap(), pubkey_b64)
}

#[test]
fn parse_discovery_nodes_json_accepts_valid_signed_snapshot() {
    let nodes = serde_json::json!([
        {
            "node_id": "node-a",
            "endpoint": "198.51.100.10:18142",
            "endpoint_generation": 3,
            "country_code": "nl",
            "success_rate_1h": 95.0,
            "loss_pct": 2.0
        }
    ]);
    let (json, pubkey) = signed_envelope(&nodes);
    let mut keyring = BTreeMap::new();
    keyring.insert("default".to_string(), pubkey);
    let parsed =
        parse_discovery_nodes_json(&json, &keyring, &BTreeSet::new(), &BTreeSet::new()).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].node_id, "node-a");
    assert_eq!(parsed[0].endpoint, "198.51.100.10:18142");
    assert_eq!(parsed[0].endpoint_generation, Some(3));
    assert_eq!(parsed[0].region(), "NL");
    assert_eq!(parsed[0].reliability_score(), 95);
    assert_eq!(parsed[0].load_score(), 2);
}

#[test]
fn parse_discovery_nodes_json_rejects_bad_signature() {
    let nodes = serde_json::json!([{ "node_id": "node-a", "endpoint": "198.51.100.10:18142" }]);
    let (mut json, pubkey) = signed_envelope(&nodes);
    // Corrupt signature by changing a few characters.
    if let Some(pos) = json.rfind('"') {
        json.replace_range(pos - 4..pos, "AAAA");
    }
    let mut keyring = BTreeMap::new();
    keyring.insert("default".to_string(), pubkey);
    let result = parse_discovery_nodes_json(&json, &keyring, &BTreeSet::new(), &BTreeSet::new());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("signature verification failed")
    );
}

#[test]
fn parse_discovery_nodes_json_rejects_expired_snapshot() {
    let nodes = serde_json::json!([{ "node_id": "node-a", "endpoint": "198.51.100.10:18142" }]);
    let (json, pubkey) = signed_envelope(&nodes);
    // Patch expires_at to the past without re-signing; should fail before signature.
    let expired_json = json.replace("expires_at_unix", "x_expires_at_unix");
    let mut keyring = BTreeMap::new();
    keyring.insert("default".to_string(), pubkey);
    let result =
        parse_discovery_nodes_json(&expired_json, &keyring, &BTreeSet::new(), &BTreeSet::new());
    assert!(result.is_err());
}
