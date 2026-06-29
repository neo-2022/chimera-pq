pub(super) fn count_mesh_policy_fields(payload: &str) -> usize {
    payload
        .split(';')
        .filter_map(|segment| segment.split_once('='))
        .map(|(key, _)| key.trim().to_ascii_lowercase())
        .filter(|key| key.starts_with("mesh_"))
        .count()
}

pub(super) fn ensure_mesh_payload_nonempty(payload: &str) -> Result<(), String> {
    let count = count_mesh_policy_fields(payload);
    if count == 0 {
        return Err("mesh policy payload must include at least one mesh_* field".to_string());
    }
    Ok(())
}
