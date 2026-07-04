use crate::mesh_cli::nodes_cmd::mesh_nodes_command;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

#[test]
fn nodes_probe_all_proof_uses_resolved_endpoint_instead_of_redacted_label() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| unreachable!("bind listener failed: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| unreachable!("read addr failed: {err}"));
    let token = "tok-proof-nodes-probe".to_string();
    let handle = thread::spawn(move || {
        let mut proof_seen = false;
        for _ in 0..4 {
            let (mut stream, _) = listener
                .accept()
                .unwrap_or_else(|err| unreachable!("accept failed: {err}"));
            stream
                .set_read_timeout(Some(Duration::from_millis(500)))
                .unwrap_or_else(|err| unreachable!("set_read_timeout failed: {err}"));
            let mut line = String::new();
            let mut reader = BufReader::new(
                stream
                    .try_clone()
                    .unwrap_or_else(|err| unreachable!("clone failed: {err}")),
            );
            let read = reader
                .read_line(&mut line)
                .unwrap_or_else(|err| unreachable!("read_line failed: {err}"));
            if read == 0 || line.trim().is_empty() {
                continue;
            }
            let parsed: serde_json::Value =
                serde_json::from_str(line.trim()).unwrap_or_else(|err| unreachable!("{err}"));
            assert_eq!(parsed["kind"], "mesh_guard_challenge_v1");
            assert_eq!(parsed["key_id"], "mesh-shared-v1");
            assert_eq!(parsed["pq_key_id"], "mesh-pq-shared-v1");
            assert!(parsed["classic_sig"].as_str().unwrap_or("").len() > 10);
            assert!(parsed["pq_sig"].as_str().unwrap_or("").len() > 10);
            stream
                .write_all(b"{\"kind\":\"mesh_guard_ack_v1\",\"status\":\"ok\"}\n")
                .unwrap_or_else(|err| unreachable!("write failed: {err}"));
            proof_seen = true;
            break;
        }
        assert!(proof_seen, "proof connection was not observed");
    });

    let node = format!("n1@{addr}@US@United States@healthy@10@1@0@99@99@0@1");
    let args = vec![
        "probe".to_string(),
        "--all".to_string(),
        "--node".to_string(),
        node,
        "--json".to_string(),
        "--proof-token-classic".to_string(),
        token.clone(),
        "--proof-token-pq".to_string(),
        token,
        "--probe-timeout-ms".to_string(),
        "500".to_string(),
    ];

    let rc = mesh_nodes_command(&args);
    handle
        .join()
        .unwrap_or_else(|_| unreachable!("join failed"));
    assert_eq!(rc, 0);
}
