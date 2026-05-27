pub(super) fn normalize_region_key(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub(super) fn validate_source_name(source: &str, label: &str) -> Result<(), String> {
    validate_runtime_token(source, label, false)
}

pub(super) fn validate_runtime_node_id(node_id: &str, label: &str) -> Result<(), String> {
    validate_runtime_token(node_id, label, true)
}

fn validate_runtime_token(value: &str, label: &str, trim_before_empty: bool) -> Result<(), String> {
    let normalized = if trim_before_empty {
        value.trim()
    } else {
        value
    };
    if normalized.is_empty() {
        return Err(format!("{label} is empty"));
    }
    if normalized.contains(',') {
        return Err(format!("{label} contains comma"));
    }
    if normalized.contains('\n') || normalized.contains('\r') || normalized.contains('\t') {
        return Err(format!("{label} contains control whitespace"));
    }
    Ok(())
}
