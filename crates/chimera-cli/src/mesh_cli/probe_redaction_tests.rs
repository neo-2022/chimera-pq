#![forbid(unsafe_code)]

use super::probe_redaction::{
    endpoint_label, peer_label, redact_explain_line, selected_peer_labels,
};
use chimera_mesh::{MeshConnectAttempt, MeshConnectProbeReport};

fn report() -> MeshConnectProbeReport {
    MeshConnectProbeReport {
        namespace: "cef-public".to_string(),
        selected_peers: vec!["node-a".to_string(), "node-b".to_string()],
        connected_peer: "node-b".to_string(),
        connected_endpoint: "198.51.100.10:443".to_string(),
        success: true,
        attempts: vec![MeshConnectAttempt {
            peer_id: "node-b".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            success: true,
            error: String::new(),
        }],
        explain: vec![
            "connect_probe_connected_peer=node-b".to_string(),
            "connect_probe_connected_endpoint=198.51.100.10:443".to_string(),
            "selected_peer_ids=node-a,node-b".to_string(),
            "selected_peer_endpoints=198.51.100.9:8443,198.51.100.10:443".to_string(),
            "selected_peer_connect_priority=1:node-a@198.51.100.9:8443,2:node-b@198.51.100.10:443".to_string(),
            "selected_peer_connect_retry_plan=node-b@198.51.100.10:443:try0(connect)|try1(retry_fast)|try2(retry_slow);ports=443|8443;fallback_ports=configured".to_string(),
            "selected_peer_scores=node-b:160".to_string(),
        ],
    }
}

#[test]
fn redacts_probe_identity_and_endpoint() {
    let report = report();
    assert_eq!(peer_label(&report, "node-b"), "peer#2");
    assert_eq!(
        endpoint_label(&report, "198.51.100.10:443"),
        "endpoint#1:<redacted>"
    );
    assert_eq!(
        selected_peer_labels(&report),
        vec!["peer#1".to_string(), "peer#2".to_string()]
    );
}

#[test]
fn redacts_connect_explain_lines() {
    let report = report();
    assert_eq!(
        redact_explain_line("connect_probe_connected_peer=node-b", &report),
        "connect_probe_connected_peer=peer#2"
    );
    assert_eq!(
        redact_explain_line(
            "connect_probe_connected_endpoint=198.51.100.10:443",
            &report
        ),
        "connect_probe_connected_endpoint=endpoint#1:<redacted>"
    );
}

#[test]
fn redacts_selection_explain_lines() {
    let report = report();
    assert_eq!(
        redact_explain_line("selected_peer_ids=node-a,node-b", &report),
        "selected_peer_ids=peer#1,peer#2"
    );
    assert_eq!(
        redact_explain_line(
            "selected_peer_endpoints=198.51.100.9:8443,198.51.100.10:443",
            &report
        ),
        "selected_peer_endpoints=<redacted>,endpoint#1:<redacted>"
    );
    assert_eq!(
        redact_explain_line(
            "selected_peer_connect_priority=1:node-a@198.51.100.9:8443,2:node-b@198.51.100.10:443",
            &report
        ),
        "selected_peer_connect_priority=1:peer#1@<redacted>,2:peer#2@<redacted>"
    );
    assert_eq!(
        redact_explain_line(
            "selected_peer_connect_retry_plan=node-b@198.51.100.10:443:try0(connect)|try1(retry_fast)|try2(retry_slow);ports=443|8443;fallback_ports=configured",
            &report
        ),
        "selected_peer_connect_retry_plan=peer#2@<redacted>:try0(connect)|try1(retry_fast)|try2(retry_slow);ports=<redacted>;fallback_ports=configured"
    );
    assert_eq!(
        redact_explain_line("selected_peer_scores=node-b:160", &report),
        "selected_peer_scores=peer#2:160"
    );
}

#[test]
fn preserves_already_redacted_probe_explain_lines() {
    let report = report();
    assert_eq!(
        redact_explain_line("selected_peer_ids=peer#1,peer#2", &report),
        "selected_peer_ids=peer#1,peer#2"
    );
    assert_eq!(
        redact_explain_line(
            "selected_peer_endpoints=endpoint#1:<redacted>,endpoint#2:<redacted>",
            &report
        ),
        "selected_peer_endpoints=endpoint#1:<redacted>,endpoint#2:<redacted>"
    );
    assert_eq!(
        redact_explain_line(
            "selected_peer_connect_priority=1:peer#1@<redacted>,2:peer#2@<redacted>",
            &report
        ),
        "selected_peer_connect_priority=1:peer#1@<redacted>,2:peer#2@<redacted>"
    );
    assert_eq!(
        redact_explain_line(
            "selected_peer_connect_retry_plan=peer#2@<redacted>:try0(connect)|try1(retry_fast)|try2(retry_slow);ports=<redacted>;fallback_ports=configured",
            &report
        ),
        "selected_peer_connect_retry_plan=peer#2@<redacted>:try0(connect)|try1(retry_fast)|try2(retry_slow);ports=<redacted>;fallback_ports=configured"
    );
}
