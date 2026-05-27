use crate::{MeshJoinRequest, MeshRuntime};

#[test]
fn runtime_plan_path_from_dps_payload_rejects_invalid_policy() {
    let runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-client".to_string(),
        invite_token: None,
    };
    assert!(
        runtime
            .plan_path_from_dps_payload(&req, "mesh_max_peers=0")
            .is_err()
    );
    assert!(
        runtime
            .plan_path_from_dps_payload(&req, "allow=mesh")
            .is_err()
    );
}
