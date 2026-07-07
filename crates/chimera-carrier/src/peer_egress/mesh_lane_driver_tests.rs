use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use ed25519_dalek::{Signer, SigningKey};

use crate::peer_egress::lane_binding::load_transit_lane_document;
use crate::peer_egress::mesh_lane_driver::{MeshLaneDriverOptions, run_mesh_lane_driver_once};

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn signed_discovery_body(node_id: &str, node_endpoint: &str) -> (String, String) {
    let seed: [u8; 32] = rand::random();
    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();
    let pubkey_b64 = base64::engine::general_purpose::STANDARD.encode(verifying_key.to_bytes());

    let issued_at = now_unix();
    let expires_at = issued_at + 120;
    let nonce = format!("lane-driver-test-{}-{}", issued_at, rand::random::<u64>());
    let nodes = serde_json::json!([
        {
            "node_id": node_id,
            "endpoint": node_endpoint,
            "endpoint_generation": 1,
            "country_code": "eu",
            "success_rate_1h": 95.0,
            "loss_pct": 1.0
        }
    ]);
    let nodes_compact = serde_json::to_string(&nodes).unwrap();
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

fn serve_one_json(body: String) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(&mut stream);
        let mut headers = String::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" || line.is_empty() {
                break;
            }
            headers.push_str(&line);
        }
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).unwrap();
    });
    format!("http://{}", addr)
}

#[test]
fn mesh_lane_driver_plans_carrier_lanes_from_discovery_snapshot() {
    let endpoint = "198.51.100.31:18143";
    let (body, pubkey) = signed_discovery_body("remote-node", endpoint);
    let url = serve_one_json(body);

    let mut keyring = BTreeMap::new();
    keyring.insert("default".to_string(), pubkey);

    let lane_document_path = std::env::temp_dir()
        .join(format!(
            "chimera-lane-driver-{}-{}.v1",
            std::process::id(),
            now_unix()
        ))
        .to_string_lossy()
        .to_string();

    let options = MeshLaneDriverOptions {
        namespace: "chimera-mesh".to_string(),
        self_node_id: "local-node".to_string(),
        policy_payload: concat!(
            "allow=mesh;",
            "mesh_allowed_regions=eu;",
            "mesh_max_peers=1;",
            "mesh_max_selected_per_region=1;",
            "mesh_multipath_mode=flow_shard;",
            "mesh_route_binding_id=7001"
        )
        .to_string(),
        lane_document_path: lane_document_path.clone(),
        discovery_urls: vec![url],
        discovery_keyring: keyring,
        discovery_timeout_ms: 5_000,
        poll_interval_ms: 30_000,
    };

    run_mesh_lane_driver_once(&options).unwrap();

    let document = load_transit_lane_document(&lane_document_path).unwrap();
    let plan = document.require_mesh_path_plan_ref().unwrap();
    assert_eq!(plan.multipath_schedule.active_lane_count, 1);
    assert_eq!(plan.multipath_schedule.carrier_lane_bindings.len(), 1);
    assert_eq!(
        plan.multipath_schedule.carrier_lane_bindings[0].carrier_endpoint,
        endpoint
    );
}

#[test]
fn mesh_lane_driver_filters_self_node() {
    let endpoint = "198.51.100.31:18143";
    let (body, pubkey) = signed_discovery_body("local-node", endpoint);
    let url = serve_one_json(body);

    let mut keyring = BTreeMap::new();
    keyring.insert("default".to_string(), pubkey);

    let lane_document_path = std::env::temp_dir()
        .join(format!(
            "chimera-lane-driver-self-{}-{}.v1",
            std::process::id(),
            now_unix()
        ))
        .to_string_lossy()
        .to_string();

    let options = MeshLaneDriverOptions {
        namespace: "chimera-mesh".to_string(),
        self_node_id: "local-node".to_string(),
        policy_payload: concat!(
            "allow=mesh;",
            "mesh_allowed_regions=eu;",
            "mesh_max_peers=1;",
            "mesh_max_selected_per_region=1;",
            "mesh_multipath_mode=flow_shard;",
            "mesh_route_binding_id=7001"
        )
        .to_string(),
        lane_document_path,
        discovery_urls: vec![url],
        discovery_keyring: keyring,
        discovery_timeout_ms: 5_000,
        poll_interval_ms: 30_000,
    };

    let result = run_mesh_lane_driver_once(&options);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no remote peers"));
}
