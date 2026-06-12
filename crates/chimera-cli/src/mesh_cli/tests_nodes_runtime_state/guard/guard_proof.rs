use crate::mesh_cli::nodes_cmd::{proof_pq_strict_enabled, verify_chimera_proof};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn nodes_verify_chimera_proof_accepts_valid_guard_response() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read addr failed: {err}"));
    let token = "tok-proof-a".to_string();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .unwrap_or_else(|err| unreachable!("accept failed: {err}"));
        let mut line = String::new();
        let mut reader = BufReader::new(
            stream
                .try_clone()
                .unwrap_or_else(|err| unreachable!("clone failed: {err}")),
        );
        reader
            .read_line(&mut line)
            .unwrap_or_else(|err| unreachable!("read_line failed: {err}"));
        let parsed: serde_json::Value =
            serde_json::from_str(line.trim()).unwrap_or_else(|err| unreachable!("{err}"));
        assert_eq!(parsed["kind"], "mesh_guard_challenge_v1");
        assert_eq!(parsed["key_id"], "mesh-shared-v1");
        assert_eq!(parsed["pq_key_id"], "mesh-pq-shared-v1");
        assert_eq!(parsed["classic_alg"], "hmac-sha256-v1");
        assert_eq!(parsed["pq_alg"], "hmac-sha256-v1-placeholder");
        assert!(parsed["classic_sig"].as_str().unwrap_or("").len() > 10);
        assert!(parsed["pq_sig"].as_str().unwrap_or("").len() > 10);
        stream
            .write_all(b"{\"kind\":\"mesh_guard_ack_v1\",\"status\":\"ok\"}\n")
            .unwrap_or_else(|err| unreachable!("write failed: {err}"));
    });
    let endpoint = format!("{addr}");
    let result = verify_chimera_proof(
        &endpoint,
        &token,
        &token,
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
        500,
    );
    handle
        .join()
        .unwrap_or_else(|_| unreachable!("join failed"));
    assert!(result.is_ok());
}

#[test]
fn nodes_verify_chimera_proof_sends_custom_key_ids() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read addr failed: {err}"));
    let token = "tok-proof-custom".to_string();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .unwrap_or_else(|err| unreachable!("accept failed: {err}"));
        let mut line = String::new();
        let mut reader = BufReader::new(
            stream
                .try_clone()
                .unwrap_or_else(|err| unreachable!("clone failed: {err}")),
        );
        reader
            .read_line(&mut line)
            .unwrap_or_else(|err| unreachable!("read_line failed: {err}"));
        let parsed: serde_json::Value =
            serde_json::from_str(line.trim()).unwrap_or_else(|err| unreachable!("{err}"));
        assert_eq!(parsed["key_id"], "mesh-shared-v9");
        assert_eq!(parsed["pq_key_id"], "mesh-pq-shared-v9");
        stream
            .write_all(b"{\"kind\":\"mesh_guard_ack_v1\",\"status\":\"ok\"}\n")
            .unwrap_or_else(|err| unreachable!("write failed: {err}"));
    });
    let endpoint = format!("{addr}");
    let result = verify_chimera_proof(
        &endpoint,
        &token,
        &token,
        "mesh-shared-v9",
        "mesh-pq-shared-v9",
        500,
    );
    handle
        .join()
        .unwrap_or_else(|_| unreachable!("join failed"));
    assert!(result.is_ok());
}

#[test]
fn nodes_verify_chimera_proof_rejects_invalid_guard_response() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read addr failed: {err}"));
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .unwrap_or_else(|err| unreachable!("accept failed: {err}"));
        let mut line = String::new();
        let mut reader = BufReader::new(
            stream
                .try_clone()
                .unwrap_or_else(|err| unreachable!("clone failed: {err}")),
        );
        reader
            .read_line(&mut line)
            .unwrap_or_else(|err| unreachable!("read_line failed: {err}"));
        stream
            .write_all(b"NOPE\n")
            .unwrap_or_else(|err| unreachable!("write failed: {err}"));
    });
    let endpoint = format!("{addr}");
    let result = verify_chimera_proof(
        &endpoint,
        "tok-proof-b",
        "tok-proof-b",
        "mesh-shared-v1",
        "mesh-pq-shared-v1",
        500,
    );
    handle
        .join()
        .unwrap_or_else(|_| unreachable!("join failed"));
    assert!(result.is_err());
}

#[test]
fn nodes_proof_pq_strict_defaults_to_enabled() {
    let args: Vec<String> = Vec::new();
    assert!(proof_pq_strict_enabled(&args));
}

#[test]
fn nodes_proof_pq_strict_can_be_disabled_by_flag() {
    let args = vec!["--no-pq-strict".to_string()];
    assert!(!proof_pq_strict_enabled(&args));
}
