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
    for key in ["direct_url", "datapath_targets"] {
        if get_str(obj, key).trim().is_empty() {
            return Err(format!("probe string is empty: {key}"));
        }
    }
    if !is_supported_probe_url(get_str(obj, "direct_url")) {
        return Err("probe direct_url must use http/https".to_string());
    }

    for key in [
        "direct_probe_ok",
        "datapath_probe_ok",
        "datapath_probe_attempted",
        "skipped_no_curl",
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
    ] {
        if obj.get(key).and_then(Value::as_i64).is_none_or(|v| v < 0) {
            return Err(format!("probe int type mismatch: {key}"));
        }
    }

    let rows = match obj.get("datapath_target_results").and_then(Value::as_array) {
        Some(v) => v,
        None => return Err("probe datapath_target_results type mismatch".to_string()),
    };

    for row in rows {
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
    }

    let allowed_errors: BTreeSet<&str> = [
        "none",
        "curl_not_found",
        "datapath_target_failed",
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

    if datapath_probe_ok && !datapath_probe_attempted {
        return Err("datapath_probe_ok requires datapath_probe_attempted".to_string());
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
    if !skipped_no_curl && !datapath_probe_attempted {
        return Err("datapath probe must be attempted when curl is available".to_string());
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
            .any(|target| !is_supported_probe_url(target))
        {
            return Err("datapath_targets contains non-http/https url".to_string());
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
            "direct_url":"https://direct.example",
            "datapath_targets":"https://target1.example,https://target2.example",
            "direct_probe_ok": true,
            "datapath_probe_ok": true,
            "datapath_probe_attempted": true,
            "datapath_probe_error":"none",
            "direct_timeout_sec": 8,
            "datapath_timeout_sec": 12,
            "datapath_targets_total": 2,
            "datapath_targets_ok": 2,
            "datapath_targets_failed": 0,
            "datapath_target_results":[
                {"url":"https://target1.example","ok":true},
                {"url":"https://target2.example","ok":true}
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
        payload["datapath_probe_ok"] = json!(false);
        payload["datapath_probe_error"] = json!("unknown");
        payload["datapath_targets_total"] = json!(2);
        payload["datapath_targets_ok"] = json!(0);
        payload["datapath_targets_failed"] = json!(2);
        payload["datapath_target_results"] = json!([
            {"url":"https://target1.example","ok":false},
            {"url":"https://target2.example","ok":false}
        ]);
        assert!(validate_probe(&payload).is_ok());
    }

    #[test]
    fn validate_probe_rejects_not_attempted_with_non_empty_rows() {
        let mut payload = base_probe();
        payload["datapath_probe_attempted"] = json!(false);
        payload["datapath_probe_ok"] = json!(false);
        payload["datapath_probe_error"] = json!("curl_not_found");
        payload["datapath_targets_total"] = json!(0);
        payload["datapath_targets_ok"] = json!(0);
        payload["datapath_targets_failed"] = json!(0);
        payload["datapath_target_results"] = json!([{"url":"https://target1.example","ok":false}]);
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_empty_required_strings() {
        let mut payload = base_probe();
        payload["direct_url"] = json!("   ");
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_non_http_direct_url() {
        let mut payload = base_probe();
        payload["direct_url"] = json!("ws://target.example");
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_non_http_datapath_target_url() {
        let mut payload = base_probe();
        payload["datapath_targets"] = json!("https://target1.example,ws://target2.example");
        payload["datapath_targets_total"] = json!(2);
        payload["datapath_targets_ok"] = json!(2);
        payload["datapath_targets_failed"] = json!(0);
        payload["datapath_target_results"] = json!([
            {"url":"https://target1.example","ok":true},
            {"url":"ws://target2.example","ok":true}
        ]);
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_non_normalized_datapath_targets_csv() {
        let mut payload = base_probe();
        payload["datapath_targets"] =
            json!(" https://target1.example ,https://target1.example,https://target2.example ");
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn normalize_datapath_targets_csv_dedups_and_trims() {
        assert_eq!(
            normalize_datapath_targets_csv(
                " https://target1.example ,https://target1.example,https://target2.example "
            ),
            "https://target1.example,https://target2.example"
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
