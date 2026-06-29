use super::MeshDiscoveryRecord;

#[test]
fn mesh_discovery_endpoint_rejects_comma() {
    let record = MeshDiscoveryRecord {
        node_id: "node-a".to_string(),
        endpoint: "bad,host.example:443".to_string(),
        region: "eu".to_string(),
        load_score: 10,
        reliability_score: 90,
    };

    let result = record.validate();
    assert!(result.is_err());
    let error = result.err().unwrap_or_default();

    assert!(error.contains("comma"));
}
