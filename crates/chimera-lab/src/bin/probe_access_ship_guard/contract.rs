use super::redaction::validate_public_probe_text_fields;
use serde_json::Value;

pub(crate) fn validate_probe_contract(
    probe: &serde_json::Map<String, Value>,
    ci_snapshot_host: &str,
) -> Result<(), String> {
    if get_str(probe, "status") != "ok" {
        return Err("probe access ship guard: probe status mismatch".to_string());
    }
    if get_str(probe, "kind") != "probe_access" {
        return Err("probe access ship guard: probe kind mismatch".to_string());
    }
    if get_str(probe, "redaction") != "raw_targets_redacted" {
        return Err("probe access ship guard: probe redaction marker missing".to_string());
    }
    if !matches!(get_str(probe, "target_profile"), "live" | "ci_snapshot") {
        return Err("probe access ship guard: invalid probe target_profile".to_string());
    }
    if get_str(probe, "network_state") != "not_modified" {
        return Err("probe access ship guard: probe network_state mismatch".to_string());
    }
    let targets = probe
        .get("targets")
        .and_then(Value::as_array)
        .ok_or_else(|| "probe access ship guard: probe targets missing".to_string())?;
    if targets.is_empty() {
        return Err("probe access ship guard: probe targets empty".to_string());
    }
    let target_profile = get_str(probe, "target_profile");
    let has_snapshot_refs = targets_have_snapshot_refs(targets, ci_snapshot_host)?;
    if has_snapshot_refs && target_profile != "ci_snapshot" {
        return Err(
            "probe access ship guard: mixed ci_snapshot and live probe targets".to_string(),
        );
    }
    let totals = probe
        .get("totals")
        .and_then(Value::as_object)
        .ok_or_else(|| "probe access ship guard: probe totals missing".to_string())?;
    let all = get_i64(totals, "all")?;
    let direct_ok = get_i64(totals, "direct_ok")?;
    let unreachable = get_i64(totals, "unreachable")?;
    let policy_apply_failed = get_i64(totals, "policy_apply_failed")?;
    let failed_total = get_i64(totals, "failed_total")?;
    let fail_threshold = get_i64(totals, "fail_threshold")?;
    let threshold_exceeded = totals
        .get("threshold_exceeded")
        .and_then(Value::as_bool)
        .ok_or_else(|| "probe access ship guard: invalid threshold_exceeded".to_string())?;
    if all <= 0 || all != targets.len() as i64 {
        return Err("probe access ship guard: probe all total mismatch".to_string());
    }
    if direct_ok < 0 || unreachable < 0 || policy_apply_failed < 0 || failed_total < 0 {
        return Err("probe access ship guard: negative probe totals".to_string());
    }
    let reachability_total = checked_total(direct_ok, unreachable, "probe reachability totals")?;
    if reachability_total != all {
        return Err("probe access ship guard: probe reachability totals mismatch".to_string());
    }
    let computed_failed_total =
        checked_total(unreachable, policy_apply_failed, "probe failed totals")?;
    if computed_failed_total != failed_total {
        return Err("probe access ship guard: probe failed_total mismatch".to_string());
    }
    if fail_threshold < 0 {
        return Err("probe access ship guard: negative fail_threshold".to_string());
    }
    if threshold_exceeded != (failed_total > fail_threshold) {
        return Err("probe access ship guard: threshold_exceeded mismatch".to_string());
    }
    if threshold_exceeded {
        return Err("probe access ship guard: threshold_exceeded cannot ship".to_string());
    }
    let row_counts = count_target_rows(targets)?;
    if direct_ok != row_counts.direct_ok {
        return Err("probe access ship guard: target direct_ok total mismatch".to_string());
    }
    if unreachable != row_counts.unreachable {
        return Err("probe access ship guard: target unreachable total mismatch".to_string());
    }
    if policy_apply_failed != row_counts.policy_apply_failed {
        return Err(
            "probe access ship guard: target policy_apply_failed total mismatch".to_string(),
        );
    }
    Ok(())
}

struct TargetRowCounts {
    direct_ok: i64,
    unreachable: i64,
    policy_apply_failed: i64,
}

fn count_target_rows(targets: &[Value]) -> Result<TargetRowCounts, String> {
    let mut direct_ok = 0i64;
    let mut unreachable = 0i64;
    let mut policy_apply_failed = 0i64;
    for (idx, target) in targets.iter().enumerate() {
        let obj = target
            .as_object()
            .ok_or_else(|| "probe access ship guard: target row is not object".to_string())?;
        let row_url = obj
            .get("url")
            .and_then(Value::as_str)
            .ok_or_else(|| "probe access ship guard: target url missing".to_string())?;
        let expected_ref = redacted_target_ref(idx + 1);
        if row_url != expected_ref {
            return Err("probe access ship guard: target url must be redacted ref".to_string());
        }
        let target_ref = obj
            .get("target_ref")
            .and_then(Value::as_str)
            .ok_or_else(|| "probe access ship guard: target target_ref missing".to_string())?;
        if target_ref != expected_ref {
            return Err("probe access ship guard: target_ref mismatch".to_string());
        }
        let row_direct_ok = obj
            .get("direct_ok")
            .and_then(Value::as_bool)
            .ok_or_else(|| "probe access ship guard: target direct_ok missing".to_string())?;
        let recommended_route = obj
            .get("recommended_route")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                "probe access ship guard: target recommended_route missing".to_string()
            })?;
        let expected_route = if row_direct_ok { "direct" } else { "transit" };
        if recommended_route != expected_route {
            return Err("probe access ship guard: target recommended_route mismatch".to_string());
        }
        let policy_apply_result = obj
            .get("policy_apply_result")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                "probe access ship guard: target policy_apply_result missing".to_string()
            })?;
        if !matches!(
            policy_apply_result,
            "not_requested" | "applied" | "failed" | "skipped_unknown_recommendation"
        ) {
            return Err("probe access ship guard: target policy_apply_result mismatch".to_string());
        }
        let policy_verify_ok = obj
            .get("policy_verify_ok")
            .and_then(Value::as_bool)
            .ok_or_else(|| {
                "probe access ship guard: target policy_verify_ok missing".to_string()
            })?;
        if obj.contains_key("policy_rule_id") {
            return Err("probe access ship guard: raw policy_rule_id field forbidden".to_string());
        }
        let policy_rule_ref = get_target_str(obj, "policy_rule_ref")?;
        let policy_verify_outbound = get_target_str(obj, "policy_verify_outbound")?;
        let target_error = get_target_str(obj, "target_error")?;
        let policy_hint = get_target_str(obj, "policy_hint")?;
        validate_public_probe_text_fields(&[
            ("target_ref", target_ref),
            ("policy_hint", policy_hint),
            ("policy_rule_ref", policy_rule_ref),
            ("policy_verify_outbound", policy_verify_outbound),
            ("target_error", target_error),
        ])?;
        validate_redacted_policy_hint(policy_hint, recommended_route)?;
        if policy_apply_result != "failed" && !target_error.is_empty() {
            return Err("probe access ship guard: target_error requires failed policy".to_string());
        }
        match policy_apply_result {
            "applied" => {
                if !policy_verify_ok {
                    return Err(
                        "probe access ship guard: applied policy must verify ok".to_string()
                    );
                }
                if policy_rule_ref.is_empty()
                    || !is_redacted_rule_ref(policy_rule_ref)
                    || policy_verify_outbound != recommended_route
                {
                    return Err(
                        "probe access ship guard: applied policy verification mismatch".to_string(),
                    );
                }
            }
            "failed" => {
                if policy_verify_ok || target_error.is_empty() {
                    return Err(
                        "probe access ship guard: failed policy must carry error".to_string()
                    );
                }
            }
            "not_requested" | "skipped_unknown_recommendation" => {
                if policy_verify_ok
                    || !policy_rule_ref.is_empty()
                    || !policy_verify_outbound.is_empty()
                {
                    return Err(
                        "probe access ship guard: unapplied policy must not claim verification"
                            .to_string(),
                    );
                }
            }
            _ => return Err("probe access ship guard: target policy_apply_result mismatch".into()),
        }
        if row_direct_ok {
            direct_ok += 1;
        } else {
            unreachable += 1;
        }
        if policy_apply_result == "failed" {
            policy_apply_failed += 1;
        }
    }
    Ok(TargetRowCounts {
        direct_ok,
        unreachable,
        policy_apply_failed,
    })
}

fn targets_have_snapshot_refs(targets: &[Value], _host: &str) -> Result<bool, String> {
    for target in targets {
        let obj = target
            .as_object()
            .ok_or_else(|| "probe access ship guard: target row is not object".to_string())?;
        let target_ref = obj
            .get("target_ref")
            .and_then(Value::as_str)
            .ok_or_else(|| "probe access ship guard: target target_ref missing".to_string())?;
        if target_ref.starts_with("ci_snapshot#") {
            return Ok(true);
        }
    }
    Ok(false)
}

fn redacted_target_ref(index: usize) -> String {
    format!("target#{index}")
}

fn is_redacted_rule_ref(value: &str) -> bool {
    value
        .strip_prefix("rule#")
        .and_then(|raw| raw.parse::<usize>().ok())
        .is_some_and(|index| index > 0)
}

fn validate_redacted_policy_hint(value: &str, recommended_route: &str) -> Result<(), String> {
    let valid_kind = [
        "target_kind=domain_exact_present",
        "target_kind=ip_literal",
        "target_kind=domain_absent",
    ]
    .iter()
    .any(|prefix| value.starts_with(prefix));
    let valid_route = value.ends_with(&format!(" outbound={recommended_route}"));
    if !valid_kind || !valid_route {
        return Err("probe access ship guard: target policy_hint must be redacted".to_string());
    }
    Ok(())
}

fn checked_total(left: i64, right: i64, label: &str) -> Result<i64, String> {
    left.checked_add(right)
        .ok_or_else(|| format!("probe access ship guard: {label} overflow"))
}

fn get_str<'a>(obj: &'a serde_json::Map<String, Value>, key: &str) -> &'a str {
    obj.get(key).and_then(Value::as_str).unwrap_or("")
}

fn get_i64(obj: &serde_json::Map<String, Value>, key: &str) -> Result<i64, String> {
    obj.get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("probe access ship guard: invalid integer field: {key}"))
}

fn get_target_str<'a>(
    obj: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a str, String> {
    obj.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("probe access ship guard: target {key} missing"))
}

#[cfg(test)]
#[path = "contract_tests.rs"]
mod tests;
