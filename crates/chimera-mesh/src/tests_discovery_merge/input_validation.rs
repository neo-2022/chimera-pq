use crate::{MeshDiscoveryRecord, MeshRuntime};

#[test]
fn merge_discovery_rejects_whitespace_source() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![MeshDiscoveryRecord {
        node_id: "node-a".to_string(),
        endpoint: "198.51.100.10:443".to_string(),
        region: "eu".to_string(),
        load_score: 20,
        reliability_score: 90,
    }];
    assert!(runtime.merge_discovery("   ", &records).is_err());
    assert!(runtime.merge_discovery("seed,b", &records).is_err());
    assert!(runtime.merge_discovery("seed\nb", &records).is_err());
}

#[test]
fn merge_discovery_rejects_duplicate_node_ids_in_single_batch() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 90,
        },
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.11:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 95,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_err());
}

#[test]
fn merge_discovery_rejects_invalid_endpoint_format() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let no_port = vec![MeshDiscoveryRecord {
        node_id: "node-a".to_string(),
        endpoint: "198.51.100.10".to_string(),
        region: "eu".to_string(),
        load_score: 20,
        reliability_score: 90,
    }];
    assert!(runtime.merge_discovery("seed-b", &no_port).is_err());

    let bad_port = vec![MeshDiscoveryRecord {
        node_id: "node-a".to_string(),
        endpoint: "198.51.100.10:abc".to_string(),
        region: "eu".to_string(),
        load_score: 20,
        reliability_score: 90,
    }];
    assert!(runtime.merge_discovery("seed-c", &bad_port).is_err());

    let host_space = vec![MeshDiscoveryRecord {
        node_id: "node-a".to_string(),
        endpoint: "198.51.100.10 :443".to_string(),
        region: "eu".to_string(),
        load_score: 20,
        reliability_score: 90,
    }];
    assert!(runtime.merge_discovery("seed-d", &host_space).is_err());

    let port_space = vec![MeshDiscoveryRecord {
        node_id: "node-a".to_string(),
        endpoint: "198.51.100.10: 443".to_string(),
        region: "eu".to_string(),
        load_score: 20,
        reliability_score: 90,
    }];
    assert!(runtime.merge_discovery("seed-e", &port_space).is_err());

    let node_id_bad = vec![MeshDiscoveryRecord {
        node_id: "node,a".to_string(),
        endpoint: "198.51.100.10:443".to_string(),
        region: "eu".to_string(),
        load_score: 20,
        reliability_score: 90,
    }];
    assert!(runtime.merge_discovery("seed-f", &node_id_bad).is_err());

    let region_bad = vec![MeshDiscoveryRecord {
        node_id: "node-a".to_string(),
        endpoint: "198.51.100.10:443".to_string(),
        region: "eu\nwest".to_string(),
        load_score: 20,
        reliability_score: 90,
    }];
    assert!(runtime.merge_discovery("seed-g", &region_bad).is_err());
}

#[test]
fn merge_discovery_accepts_bracketed_ipv6_endpoint() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![MeshDiscoveryRecord {
        node_id: "node-v6".to_string(),
        endpoint: "[2001:db8::1]:443".to_string(),
        region: "eu".to_string(),
        load_score: 20,
        reliability_score: 90,
    }];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());
    assert_eq!(runtime.peer_count(), 1);
}

#[test]
fn merge_discovery_rejects_unbracketed_ipv6_endpoint() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let records = vec![MeshDiscoveryRecord {
        node_id: "node-v6".to_string(),
        endpoint: "2001:db8::1:443".to_string(),
        region: "eu".to_string(),
        load_score: 20,
        reliability_score: 90,
    }];
    assert!(runtime.merge_discovery("seed-b", &records).is_err());
}
