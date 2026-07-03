#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::BTreeSet;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json");

    let raw = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => fail(&format!(
            "runtime real-world probe schema guard: missing file: {path}"
        )),
    };

    let data: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(err) => fail(&format!(
            "runtime real-world probe schema guard: invalid json: {err}"
        )),
    };

    if let Err(msg) = validate_probe(&data) {
        fail(&msg);
    }

    println!("runtime real-world probe schema guard: PASS");
}

fn validate_probe(data: &Value) -> Result<(), String> {
    let obj = data
        .as_object()
        .ok_or_else(|| "probe envelope mismatch".to_string())?;

    let required: BTreeSet<&str> = [
        "status",
        "kind",
        "message_en",
        "message_ru",
        "evidence_kind",
        "chimera_datapath_evidence",
        "truth_boundary",
        "probe_mode",
        "live_external_probe",
        "ssh_stand_required_for_live_probe",
        "direct_url",
        "datapath_targets",
        "direct_probe_ok",
        "datapath_probe_ok",
        "datapath_probe_attempted",
        "datapath_probe_error",
        "direct_timeout_sec",
        "datapath_timeout_sec",
        "datapath_targets_total",
        "datapath_targets_ok",
        "datapath_targets_failed",
        "datapath_target_results",
        "external_reachability_probe_attempted",
        "external_reachability_probe_ok",
        "external_reachability_targets",
        "external_reachability_targets_total",
        "external_reachability_targets_ok",
        "external_reachability_targets_failed",
        "external_reachability_target_results",
        "skipped_no_curl",
        "network_state",
    ]
    .into_iter()
    .collect();

    let keys: BTreeSet<&str> = obj.keys().map(String::as_str).collect();
    if keys != required {
        let missing: Vec<&str> = required.difference(&keys).copied().collect();
        let extra: Vec<&str> = keys.difference(&required).copied().collect();
        return Err(format!(
            "probe keys mismatch missing={} extra={}",
            missing.join(","),
            extra.join(",")
        ));
    }

    if get_str(obj, "status") != "ok" || get_str(obj, "kind") != "runtime_real_world_probe_smoke" {
        return Err("probe envelope mismatch".to_string());
    }
    if get_str(obj, "network_state") != "not_modified" {
        return Err("probe network_state mismatch".to_string());
    }
    let probe_mode = get_str(obj, "probe_mode");
    if !["live", "ci_snapshot"].contains(&probe_mode) {
        return Err("probe mode invalid".to_string());
    }
    let ci_snapshot = probe_mode == "ci_snapshot";
    if obj.get("live_external_probe").and_then(Value::as_bool) != Some(!ci_snapshot) {
        return Err("probe live_external_probe mismatch".to_string());
    }
    if obj
        .get("ssh_stand_required_for_live_probe")
        .and_then(Value::as_bool)
        != Some(ci_snapshot)
    {
        return Err("probe ssh_stand_required_for_live_probe mismatch".to_string());
    }

    let evidence_kind = get_str(obj, "evidence_kind");
    let chimera_datapath_evidence = get_bool(obj, "chimera_datapath_evidence");
    let allowed_evidence_kinds: BTreeSet<&str> = [
        "external_reachability_without_system_proxy",
        "ci_snapshot_contract",
        "chimera_transparent_datapath",
    ]
    .into_iter()
    .collect();
    if !allowed_evidence_kinds.contains(evidence_kind) {
        return Err("probe evidence_kind invalid".to_string());
    }
    if get_str(obj, "truth_boundary").trim().is_empty() {
        return Err("probe truth_boundary is empty".to_string());
    }
    if !chimera_datapath_evidence && evidence_kind == "chimera_transparent_datapath" {
        return Err("chimera evidence kind requires chimera_datapath_evidence".to_string());
    }
    if chimera_datapath_evidence && evidence_kind != "chimera_transparent_datapath" {
        return Err("chimera_datapath_evidence requires chimera evidence kind".to_string());
    }

    if ci_snapshot {
        for key in [
            "direct_url",
            "datapath_targets",
            "external_reachability_targets",
        ] {
            if !get_str(obj, key).is_empty() {
                return Err(format!("ci snapshot string must be empty: {key}"));
            }
        }
        if evidence_kind != "ci_snapshot_contract" || chimera_datapath_evidence {
            return Err("ci snapshot evidence fields mismatch".to_string());
        }
    } else {
        if !is_redacted_direct_ref(get_str(obj, "direct_url")) {
            return Err("probe direct_url must be redacted direct ref".to_string());
        }
    }
    validate_public_probe_redaction(data)?;

    for key in [
        "direct_probe_ok",
        "datapath_probe_ok",
        "datapath_probe_attempted",
        "skipped_no_curl",
        "live_external_probe",
        "ssh_stand_required_for_live_probe",
        "chimera_datapath_evidence",
        "external_reachability_probe_attempted",
        "external_reachability_probe_ok",
    ] {
        if !obj.get(key).is_some_and(Value::is_boolean) {
            return Err(format!("probe bool type mismatch: {key}"));
        }
    }

    for key in [
        "direct_timeout_sec",
        "datapath_timeout_sec",
        "datapath_targets_total",
        "datapath_targets_ok",
        "datapath_targets_failed",
        "external_reachability_targets_total",
        "external_reachability_targets_ok",
        "external_reachability_targets_failed",
    ] {
        if obj.get(key).and_then(Value::as_i64).is_none_or(|v| v < 0) {
            return Err(format!("probe int type mismatch: {key}"));
        }
    }

    let rows = match obj.get("datapath_target_results").and_then(Value::as_array) {
        Some(v) => v,
        None => return Err("probe datapath_target_results type mismatch".to_string()),
    };
    let external_rows = match obj
        .get("external_reachability_target_results")
        .and_then(Value::as_array)
    {
        Some(v) => v,
        None => {
            return Err("probe external_reachability_target_results type mismatch".to_string());
        }
    };

    for row in rows.iter().chain(external_rows.iter()) {
        let row_obj = match row.as_object() {
            Some(v) => v,
            None => return Err("probe datapath_target_results row schema mismatch".to_string()),
        };
        let row_keys: BTreeSet<&str> = row_obj.keys().map(String::as_str).collect();
        let expected: BTreeSet<&str> = ["url", "ok"].into_iter().collect();
        if row_keys != expected {
            return Err("probe datapath_target_results row schema mismatch".to_string());
        }
        if !row_obj.get("url").is_some_and(Value::is_string)
            || !row_obj.get("ok").is_some_and(Value::is_boolean)
        {
            return Err("probe datapath_target_results row type mismatch".to_string());
        }
        if !is_redacted_target_ref(row_obj.get("url").and_then(Value::as_str).unwrap_or("")) {
            return Err("probe target result row must use redacted target ref".to_string());
        }
    }

    let allowed_errors: BTreeSet<&str> = [
        "none",
        "curl_not_found",
        "datapath_target_failed",
        "chimera_datapath_evidence_missing",
        "ci_snapshot",
        "unknown",
    ]
    .into_iter()
    .collect();
    if !allowed_errors.contains(get_str(obj, "datapath_probe_error")) {
        return Err("probe datapath_probe_error invalid".to_string());
    }

    let datapath_probe_ok = get_bool(obj, "datapath_probe_ok");
    let datapath_probe_attempted = get_bool(obj, "datapath_probe_attempted");
    let skipped_no_curl = get_bool(obj, "skipped_no_curl");
    let external_attempted = get_bool(obj, "external_reachability_probe_attempted");
    let external_ok_flag = get_bool(obj, "external_reachability_probe_ok");

    if datapath_probe_ok && !datapath_probe_attempted {
        return Err("datapath_probe_ok requires datapath_probe_attempted".to_string());
    }
    if (datapath_probe_ok || datapath_probe_attempted) && !chimera_datapath_evidence {
        return Err("datapath proof requires chimera_datapath_evidence".to_string());
    }
    if skipped_no_curl && datapath_probe_attempted {
        return Err("no curl but datapath probe attempted".to_string());
    }
    if skipped_no_curl {
        if get_bool(obj, "direct_probe_ok") || datapath_probe_ok {
            return Err("skipped_no_curl incompatible with successful probes".to_string());
        }
        if get_str(obj, "datapath_probe_error") != "curl_not_found" {
            return Err("skipped_no_curl requires curl_not_found".to_string());
        }
    }
    if ci_snapshot {
        if get_bool(obj, "direct_probe_ok") || datapath_probe_ok || datapath_probe_attempted {
            return Err("ci snapshot cannot report live probe success or attempt".to_string());
        }
        if external_attempted || external_ok_flag {
            return Err("ci snapshot cannot report external reachability attempt".to_string());
        }
        if skipped_no_curl {
            return Err("ci snapshot must not masquerade as missing curl".to_string());
        }
        if get_str(obj, "datapath_probe_error") != "ci_snapshot" {
            return Err("ci snapshot requires ci_snapshot error marker".to_string());
        }
    } else if evidence_kind == "external_reachability_without_system_proxy" {
        if !skipped_no_curl && !external_attempted {
            return Err(
                "external reachability must be attempted when curl is available".to_string(),
            );
        }
        if datapath_probe_attempted || datapath_probe_ok || chimera_datapath_evidence {
            return Err("external reachability must not masquerade as datapath proof".to_string());
        }
        if !skipped_no_curl
            && get_str(obj, "datapath_probe_error") != "chimera_datapath_evidence_missing"
        {
            return Err(
                "external reachability requires missing datapath evidence marker".to_string(),
            );
        }
    } else if !skipped_no_curl && !datapath_probe_attempted {
        return Err(
            "datapath probe must be attempted when CHIMERA evidence is available".to_string(),
        );
    }
    if datapath_probe_attempted && get_str(obj, "datapath_probe_error") == "curl_not_found" {
        return Err("datapath probe attempted with curl_not_found error".to_string());
    }
    if get_str(obj, "datapath_probe_error") == "none" && !datapath_probe_ok {
        return Err("datapath probe failed without error marker".to_string());
    }

    let total = get_i64(obj, "datapath_targets_total");
    let ok = get_i64(obj, "datapath_targets_ok");
    let failed = get_i64(obj, "datapath_targets_failed");

    if ok + failed != total {
        return Err("datapath target totals mismatch".to_string());
    }
    if rows.len() as i64 != total {
        return Err("datapath target list length mismatch".to_string());
    }
    if datapath_probe_attempted && total <= 0 {
        return Err("datapath probe attempted with empty target set".to_string());
    }
    if !datapath_probe_attempted && total != 0 {
        return Err("datapath probe not attempted must have zero target totals".to_string());
    }
    if !datapath_probe_attempted && !rows.is_empty() {
        return Err("datapath probe not attempted must have empty target rows".to_string());
    }
    if datapath_probe_ok && failed != 0 {
        return Err("datapath probe ok requires failed=0".to_string());
    }

    let external_total = get_i64(obj, "external_reachability_targets_total");
    let external_ok = get_i64(obj, "external_reachability_targets_ok");
    let external_failed = get_i64(obj, "external_reachability_targets_failed");
    if external_ok + external_failed != external_total {
        return Err("external reachability target totals mismatch".to_string());
    }
    if external_rows.len() as i64 != external_total {
        return Err("external reachability target list length mismatch".to_string());
    }
    if external_ok_flag && external_failed != 0 {
        return Err("external reachability ok requires failed=0".to_string());
    }
    if external_attempted && external_total <= 0 {
        return Err("external reachability attempted with empty target set".to_string());
    }
    if !external_attempted && external_total != 0 {
        return Err("external reachability not attempted must have zero totals".to_string());
    }
    let external_ok_count = external_rows
        .iter()
        .filter(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false))
        .count() as i64;
    if external_ok_count != external_ok {
        return Err("external reachability target ok mismatch".to_string());
    }
    let external_targets_csv = get_str(obj, "external_reachability_targets");
    if external_targets_csv != normalize_datapath_targets_csv(external_targets_csv) {
        return Err("external_reachability_targets csv is not normalized".to_string());
    }
    if external_attempted {
        let external_targets_list: Vec<&str> = if external_targets_csv.is_empty() {
            Vec::new()
        } else {
            external_targets_csv.split(',').collect()
        };
        if external_targets_list
            .iter()
            .any(|target| !is_redacted_target_ref(target))
        {
            return Err(
                "external_reachability_targets contains non-redacted target ref".to_string(),
            );
        }
        let row_urls: Vec<&str> = external_rows
            .iter()
            .filter_map(|row| row.get("url").and_then(Value::as_str))
            .collect();
        if external_targets_list != row_urls {
            return Err("external reachability csv/row url mismatch".to_string());
        }
    }

    let ok_count = rows
        .iter()
        .filter(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false))
        .count() as i64;
    if ok_count != ok {
        return Err("datapath target ok mismatch".to_string());
    }

    let datapath_targets_csv = get_str(obj, "datapath_targets");
    if datapath_targets_csv != normalize_datapath_targets_csv(datapath_targets_csv) {
        return Err("datapath_targets csv is not normalized".to_string());
    }

    if datapath_probe_attempted {
        let datapath_targets_list: Vec<&str> = if datapath_targets_csv.is_empty() {
            Vec::new()
        } else {
            datapath_targets_csv.split(',').collect()
        };
        if datapath_targets_list
            .iter()
            .any(|target| !is_redacted_target_ref(target))
        {
            return Err("datapath_targets contains non-redacted target ref".to_string());
        }
        let row_urls: Vec<&str> = rows
            .iter()
            .filter_map(|row| row.get("url").and_then(Value::as_str))
            .collect();
        if datapath_targets_list != row_urls {
            return Err("datapath_targets csv/row url mismatch".to_string());
        }
    }

    Ok(())
}

fn normalize_datapath_targets_csv(csv: &str) -> String {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for raw in csv.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let key = raw.to_ascii_lowercase();
        if seen.insert(key) {
            out.push(raw.to_string());
        }
    }
    out.join(",")
}

fn is_redacted_direct_ref(value: &str) -> bool {
    value == "direct#1"
}

fn is_redacted_target_ref(value: &str) -> bool {
    let Some(number) = value.strip_prefix("target#") else {
        return false;
    };
    !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()) && number != "0"
}

fn validate_public_probe_redaction(value: &Value) -> Result<(), String> {
    match value {
        Value::String(text) if contains_raw_probe_location(text) => {
            return Err("probe public artifact contains unredacted location".to_string());
        }
        Value::String(_) => {}
        Value::Array(items) => {
            for item in items {
                validate_public_probe_redaction(item)?;
            }
        }
        Value::Object(map) => {
            for value in map.values() {
                validate_public_probe_redaction(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn contains_raw_probe_location(value: &str) -> bool {
    is_supported_probe_url(value)
        || contains_ipv4_literal(value)
        || contains_hostname_literal(value)
        || value.contains("/home/")
        || value.contains("/tmp/chimera")
        || value.contains("BEGIN PRIVATE KEY")
        || value.contains("OPENSSH PRIVATE KEY")
        || value.contains("ssh://")
}

fn contains_ipv4_literal(value: &str) -> bool {
    for token in value.split(|c: char| !(c.is_ascii_digit() || c == '.')) {
        if looks_like_ipv4(token) {
            return true;
        }
    }
    false
}

fn contains_hostname_literal(value: &str) -> bool {
    value
        .split(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':' | '#')))
        .any(looks_like_hostname)
}

fn looks_like_hostname(token: &str) -> bool {
    let token = token.trim_matches('.');
    let token = match token.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => host,
        _ => token,
    };
    if token.is_empty()
        || token.starts_with("target#")
        || token.starts_with("direct#")
        || token.contains("://")
        || token.contains('_')
        || looks_like_ipv4(token)
    {
        return false;
    }
    let labels: Vec<&str> = token.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    labels.iter().all(|label| {
        !label.is_empty()
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    }) && labels
        .last()
        .is_some_and(|tld| tld.bytes().any(|b| b.is_ascii_alphabetic()))
}

fn looks_like_ipv4(token: &str) -> bool {
    let mut parts = token.split('.');
    let Some(a) = parts.next() else {
        return false;
    };
    let Some(b) = parts.next() else {
        return false;
    };
    let Some(c) = parts.next() else {
        return false;
    };
    let Some(d) = parts.next() else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }
    [a, b, c, d]
        .iter()
        .all(|part| !part.is_empty() && part.parse::<u8>().is_ok())
}

fn is_supported_probe_url(value: &str) -> bool {
    let Some((scheme, rest)) = value.split_once("://") else {
        return false;
    };
    let authority = extract_authority(rest);
    let scheme_lc = scheme.to_ascii_lowercase();
    matches!(scheme_lc.as_str(), "http" | "https")
        && is_valid_scheme_token(scheme)
        && !authority.trim().is_empty()
        && !authority.chars().any(char::is_whitespace)
        && authority_has_non_empty_host(authority)
}

fn extract_authority(rest: &str) -> &str {
    rest.split(['/', '?', '#']).next().unwrap_or(rest)
}

fn is_valid_scheme_token(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

fn authority_has_non_empty_host(authority: &str) -> bool {
    let host_port = authority.rsplit('@').next().unwrap_or(authority).trim();
    if host_port.is_empty() {
        return false;
    }
    if let Some(inner) = host_port.strip_prefix('[') {
        let Some((host, _rem)) = inner.split_once(']') else {
            return false;
        };
        return !host.trim().is_empty();
    }
    if let Some((h, p)) = host_port.rsplit_once(':')
        && h.is_empty()
        && !p.is_empty()
    {
        return false;
    }
    let host = match host_port.rsplit_once(':') {
        Some((h, p)) if !h.is_empty() && !p.is_empty() => h,
        _ => host_port,
    };
    !host.trim().is_empty()
}

fn get_bool(obj: &serde_json::Map<String, Value>, key: &str) -> bool {
    obj.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn get_i64(obj: &serde_json::Map<String, Value>, key: &str) -> i64 {
    obj.get(key).and_then(Value::as_i64).unwrap_or(0)
}

fn get_str<'a>(obj: &'a serde_json::Map<String, Value>, key: &str) -> &'a str {
    obj.get(key).and_then(Value::as_str).unwrap_or("")
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::{
        extract_authority, is_supported_probe_url, normalize_datapath_targets_csv, validate_probe,
    };
    use serde_json::json;

    fn base_probe() -> serde_json::Value {
        json!({
            "status":"ok",
            "kind":"runtime_real_world_probe_smoke",
            "message_en":"ok",
            "message_ru":"ok",
            "evidence_kind":"external_reachability_without_system_proxy",
            "chimera_datapath_evidence": false,
            "truth_boundary":"ordinary curl --noproxy proves external reachability only",
            "probe_mode":"live",
            "live_external_probe": true,
            "ssh_stand_required_for_live_probe": false,
            "direct_url":"direct#1",
            "datapath_targets":"",
            "direct_probe_ok": true,
            "datapath_probe_ok": false,
            "datapath_probe_attempted": false,
            "datapath_probe_error":"chimera_datapath_evidence_missing",
            "direct_timeout_sec": 8,
            "datapath_timeout_sec": 12,
            "datapath_targets_total": 0,
            "datapath_targets_ok": 0,
            "datapath_targets_failed": 0,
            "datapath_target_results":[],
            "external_reachability_probe_attempted": true,
            "external_reachability_probe_ok": true,
            "external_reachability_targets":"target#1,target#2",
            "external_reachability_targets_total": 2,
            "external_reachability_targets_ok": 2,
            "external_reachability_targets_failed": 0,
            "external_reachability_target_results":[
                {"url":"target#1","ok":true},
                {"url":"target#2","ok":true}
            ],
            "skipped_no_curl": false,
            "network_state":"not_modified"
        })
    }

    #[test]
    fn validate_probe_accepts_valid_payload() {
        let payload = base_probe();
        assert!(validate_probe(&payload).is_ok());
    }

    #[test]
    fn validate_probe_accepts_ci_snapshot_payload() {
        let payload = json!({
            "status":"ok",
            "kind":"runtime_real_world_probe_smoke",
            "message_en":"ok",
            "message_ru":"ok",
            "evidence_kind":"ci_snapshot_contract",
            "chimera_datapath_evidence": false,
            "truth_boundary":"ci snapshot only",
            "probe_mode":"ci_snapshot",
            "live_external_probe": false,
            "ssh_stand_required_for_live_probe": true,
            "direct_url":"",
            "datapath_targets":"",
            "direct_probe_ok": false,
            "datapath_probe_ok": false,
            "datapath_probe_attempted": false,
            "datapath_probe_error":"ci_snapshot",
            "direct_timeout_sec": 8,
            "datapath_timeout_sec": 12,
            "datapath_targets_total": 0,
            "datapath_targets_ok": 0,
            "datapath_targets_failed": 0,
            "datapath_target_results":[],
            "external_reachability_probe_attempted": false,
            "external_reachability_probe_ok": false,
            "external_reachability_targets":"",
            "external_reachability_targets_total": 0,
            "external_reachability_targets_ok": 0,
            "external_reachability_targets_failed": 0,
            "external_reachability_target_results":[],
            "skipped_no_curl": false,
            "network_state":"not_modified"
        });
        assert!(validate_probe(&payload).is_ok());
    }

    #[test]
    fn validate_probe_rejects_no_curl_with_successful_probes() {
        let mut payload = base_probe();
        payload["skipped_no_curl"] = json!(true);
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_inconsistent_totals() {
        let mut payload = base_probe();
        payload["datapath_targets_total"] = json!(3);
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_accepts_attempted_with_unknown_error() {
        let mut payload = base_probe();
        payload["external_reachability_probe_ok"] = json!(false);
        payload["external_reachability_targets_total"] = json!(2);
        payload["external_reachability_targets_ok"] = json!(0);
        payload["external_reachability_targets_failed"] = json!(2);
        payload["external_reachability_target_results"] = json!([
            {"url":"target#1","ok":false},
            {"url":"target#2","ok":false}
        ]);
        assert!(validate_probe(&payload).is_ok());
    }

    #[test]
    fn validate_probe_rejects_not_attempted_with_non_empty_rows() {
        let mut payload = base_probe();
        payload["external_reachability_probe_attempted"] = json!(false);
        payload["external_reachability_targets_total"] = json!(0);
        payload["external_reachability_targets_ok"] = json!(0);
        payload["external_reachability_targets_failed"] = json!(0);
        payload["external_reachability_target_results"] = json!([{"url":"target#1","ok":false}]);
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_empty_required_strings() {
        let mut payload = base_probe();
        payload["direct_url"] = json!("   ");
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_raw_direct_url() {
        let mut payload = base_probe();
        payload["direct_url"] = json!("https://target.example");
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_raw_hostname_in_public_text() {
        let mut payload = base_probe();
        payload["message_en"] = json!("leaked host stand.example");
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_raw_hostname_with_port_in_public_text() {
        let mut payload = base_probe();
        payload["message_en"] = json!("leaked endpoint stand.example:443");
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_raw_ipv4_in_public_text() {
        let mut payload = base_probe();
        payload["message_en"] = json!("leaked address 203.0.113.10");
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_local_path_in_public_text() {
        let mut payload = base_probe();
        payload["message_en"] = json!("leaked path /home/operator/chimera");
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_raw_datapath_target_url() {
        let mut payload = base_probe();
        payload["external_reachability_targets"] = json!("target#1,https://target2.example");
        payload["external_reachability_targets_total"] = json!(2);
        payload["external_reachability_targets_ok"] = json!(2);
        payload["external_reachability_targets_failed"] = json!(0);
        payload["external_reachability_target_results"] = json!([
            {"url":"target#1","ok":true},
            {"url":"https://target2.example","ok":true}
        ]);
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_invalid_redacted_target_ref() {
        let mut payload = base_probe();
        payload["external_reachability_targets"] = json!("target#0,target#2");
        payload["external_reachability_target_results"] = json!([
            {"url":"target#0","ok":true},
            {"url":"target#2","ok":true}
        ]);
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_non_normalized_datapath_targets_csv() {
        let mut payload = base_probe();
        payload["external_reachability_targets"] = json!(" target#1 ,target#1,target#2 ");
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn normalize_datapath_targets_csv_dedups_and_trims() {
        assert_eq!(
            normalize_datapath_targets_csv(" target#1 ,target#1,target#2 "),
            "target#1,target#2"
        );
    }

    #[test]
    fn supported_probe_url_requires_http_or_https() {
        assert!(is_supported_probe_url("https://target1.example"));
        assert!(is_supported_probe_url("http://target1.example"));
        assert!(is_supported_probe_url("HTTPS://target1.example"));
        assert!(!is_supported_probe_url("h*ttps://target1.example"));
        assert!(!is_supported_probe_url("https://[]"));
        assert!(!is_supported_probe_url("https://?q=1"));
        assert!(!is_supported_probe_url("ws://target1.example"));
        assert!(!is_supported_probe_url("wss://target1.example"));
        assert!(!is_supported_probe_url("https:// "));
        assert!(!is_supported_probe_url("https://bad host"));
        assert!(!is_supported_probe_url("target1.example"));
    }

    #[test]
    fn extract_authority_stops_on_path_query_and_fragment() {
        assert_eq!(extract_authority("host:1234/path"), "host:1234");
        assert_eq!(extract_authority("host:1234?x=1"), "host:1234");
        assert_eq!(extract_authority("host:1234#frag"), "host:1234");
        assert_eq!(extract_authority("host:1234/path?x=1#frag"), "host:1234");
    }
}
