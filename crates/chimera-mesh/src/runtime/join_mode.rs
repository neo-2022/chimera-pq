use super::*;

pub fn evaluate_join_mode(request: &MeshJoinRequest) -> Result<MeshJoinMode, String> {
    request.validate()?;

    match &request.invite_token {
        Some(token) if !token.trim().is_empty() => Ok(MeshJoinMode::InvitationOnly),
        _ => Ok(MeshJoinMode::PublicDiscovery),
    }
}
