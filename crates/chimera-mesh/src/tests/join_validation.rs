use crate::{MeshJoinRequest, evaluate_join_mode};

#[test]
fn whitespace_namespace_or_node_is_rejected() {
    let req_namespace = MeshJoinRequest {
        namespace: "   ".to_string(),
        node_name: "node-a".to_string(),
        invite_token: None,
    };
    assert!(evaluate_join_mode(&req_namespace).is_err());

    let req_node = MeshJoinRequest {
        namespace: "cef-ru-west".to_string(),
        node_name: "   ".to_string(),
        invite_token: None,
    };
    assert!(evaluate_join_mode(&req_node).is_err());
}

#[test]
fn invalid_node_name_with_comma_is_rejected() {
    let req = MeshJoinRequest {
        namespace: "cef-ru-west".to_string(),
        node_name: "node,a".to_string(),
        invite_token: None,
    };
    assert!(evaluate_join_mode(&req).is_err());
}
