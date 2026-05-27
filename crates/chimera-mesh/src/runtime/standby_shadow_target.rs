pub(crate) fn standby_target_for_multipath_mode(
    mode: Option<&str>,
    switch_target: &str,
    selected_peer_ids: &[String],
) -> (String, &'static str) {
    let primary = selected_peer_ids.first().cloned().unwrap_or_default();
    let secondary = selected_peer_ids.get(1).cloned().unwrap_or_default();
    match mode {
        Some("off") | Some("standby_only") => {
            if switch_target != "none" {
                (switch_target.to_string(), "switch_target")
            } else if !primary.is_empty() {
                (primary, "selected_primary")
            } else {
                ("none".to_string(), "none")
            }
        }
        Some("flow_shard") | Some("aggregate_buffered") => {
            if switch_target != "none" {
                (switch_target.to_string(), "switch_target")
            } else if !secondary.is_empty() {
                (secondary, "selected_secondary")
            } else if !primary.is_empty() {
                (primary, "selected_primary")
            } else {
                ("none".to_string(), "none")
            }
        }
        _ => {
            if switch_target != "none" {
                (switch_target.to_string(), "switch_target")
            } else if !secondary.is_empty() {
                (secondary, "selected_secondary")
            } else if !primary.is_empty() {
                (primary, "selected_primary")
            } else {
                ("none".to_string(), "none")
            }
        }
    }
}
