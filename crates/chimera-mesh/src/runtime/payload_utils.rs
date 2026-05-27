use std::collections::BTreeSet;

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

pub(super) fn mesh_policy_keys_fingerprint(payload: &str) -> String {
    let mut keys: BTreeSet<String> = BTreeSet::new();
    for (key, _) in payload
        .split(';')
        .filter_map(|segment| segment.split_once('='))
    {
        let normalized = key.trim().to_ascii_lowercase();
        if normalized.starts_with("mesh_") {
            keys.insert(normalized);
        }
    }
    if keys.is_empty() {
        "none".to_string()
    } else {
        keys.into_iter().collect::<Vec<_>>().join(",")
    }
}

pub(super) fn has_mesh_policy_key(payload: &str, expected_key: &str) -> bool {
    let expected = expected_key.trim().to_ascii_lowercase();
    payload
        .split(';')
        .filter_map(|segment| segment.split_once('='))
        .map(|(key, _)| key.trim().to_ascii_lowercase())
        .any(|key| key == expected)
}
