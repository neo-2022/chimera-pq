use crate::{MeshJoinMode, MeshJoinRequest, evaluate_join_mode};

#[test]
fn invitation_token_selects_invitation_mode() {
    let req = MeshJoinRequest {
        namespace: "cef-ru-west".to_string(),
        node_name: "node-a".to_string(),
        invite_token: Some("inv-123456".to_string()),
    };

    let mode = match evaluate_join_mode(&req) {
        Ok(value) => value,
        Err(error) => unreachable!("join mode should resolve: {error}"),
    };
    assert_eq!(mode, MeshJoinMode::InvitationOnly);
}

#[test]
fn missing_token_selects_public_mode() {
    let req = MeshJoinRequest {
        namespace: "cef-ru-west".to_string(),
        node_name: "node-a".to_string(),
        invite_token: None,
    };

    let mode = match evaluate_join_mode(&req) {
        Ok(value) => value,
        Err(error) => unreachable!("join mode should resolve: {error}"),
    };
    assert_eq!(mode, MeshJoinMode::PublicDiscovery);
}

#[test]
fn whitespace_token_selects_public_mode() {
    let req = MeshJoinRequest {
        namespace: "cef-ru-west".to_string(),
        node_name: "node-a".to_string(),
        invite_token: Some("   ".to_string()),
    };

    let mode = match evaluate_join_mode(&req) {
        Ok(value) => value,
        Err(error) => unreachable!("join mode should resolve: {error}"),
    };
    assert_eq!(mode, MeshJoinMode::PublicDiscovery);
}

#[test]
fn empty_namespace_is_rejected() {
    let req = MeshJoinRequest {
        namespace: String::new(),
        node_name: "node-a".to_string(),
        invite_token: None,
    };

    assert!(evaluate_join_mode(&req).is_err());
}
