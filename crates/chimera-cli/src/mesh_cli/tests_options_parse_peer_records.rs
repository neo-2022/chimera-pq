use crate::mesh_cli::options::parse_mesh_peer_records;

#[test]
fn parse_mesh_peer_records_rejects_duplicate_node_ids() {
    let peers = vec![
        "n1@198.51.100.1:443@eu@20@90".to_string(),
        "n1@198.51.100.2:443@eu@25@88".to_string(),
    ];
    assert_eq!(
        parse_mesh_peer_records(&peers).err(),
        Some("duplicate peer node_id 'n1' in --peer set".to_string())
    );
}

#[test]
fn parse_mesh_peer_records_accepts_unique_node_ids() {
    let peers = vec![
        "n1@198.51.100.1:443@eu@20@90".to_string(),
        "n2@198.51.100.2:443@eu@25@88".to_string(),
    ];
    let parsed = parse_mesh_peer_records(&peers)
        .unwrap_or_else(|e| unreachable!("unique peers should parse: {e}"));
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].node_id, "n1");
    assert_eq!(parsed[1].node_id, "n2");
}
