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
        "blocked_targets",
        "proxy_url",
        "proxy_candidates",
        "proxy_selected_from_candidates",
        "direct_probe_ok",
        "proxy_probe_ok",
        "proxy_listener_detected",
        "proxy_probe_attempted",
        "proxy_probe_error",
        "direct_timeout_sec",
        "proxy_timeout_sec",
        "connect_timeout_ms",
        "proxy_blocked_targets_total",
        "proxy_blocked_targets_ok",
        "proxy_blocked_targets_failed",
        "proxy_blocked_targets",
        "skipped_no_curl",
        "skipped_no_proxy_listener",
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
    for key in [
        "direct_url",
        "proxy_url",
        "blocked_targets",
        "proxy_candidates",
    ] {
        if get_str(obj, key).trim().is_empty() {
            return Err(format!("probe string is empty: {key}"));
        }
    }
    if !is_supported_probe_url(get_str(obj, "direct_url")) {
        return Err("probe direct_url must use http/https".to_string());
    }
    if !is_supported_proxy_url(get_str(obj, "proxy_url")) {
        return Err("probe proxy_url format mismatch".to_string());
    }
    let proxy_candidates_csv = get_str(obj, "proxy_candidates");
    if proxy_candidates_csv != normalize_blocked_targets_csv(proxy_candidates_csv) {
        return Err("proxy_candidates csv is not normalized".to_string());
    }
    if proxy_candidates_csv
        .split(',')
        .any(|candidate| !candidate.is_empty() && !is_supported_proxy_url(candidate))
    {
        return Err("proxy_candidates contains invalid proxy url".to_string());
    }
    if !proxy_candidates_csv
        .split(',')
        .any(|candidate| candidate == get_str(obj, "proxy_url"))
    {
        return Err("proxy_url must be part of proxy_candidates".to_string());
    }

    for key in [
        "direct_probe_ok",
        "proxy_probe_ok",
        "proxy_listener_detected",
        "proxy_probe_attempted",
        "proxy_selected_from_candidates",
        "skipped_no_curl",
        "skipped_no_proxy_listener",
    ] {
        if !obj.get(key).is_some_and(Value::is_boolean) {
            return Err(format!("probe bool type mismatch: {key}"));
        }
    }

    for key in [
        "direct_timeout_sec",
        "proxy_timeout_sec",
        "connect_timeout_ms",
        "proxy_blocked_targets_total",
        "proxy_blocked_targets_ok",
        "proxy_blocked_targets_failed",
    ] {
        if obj.get(key).and_then(Value::as_i64).is_none_or(|v| v < 0) {
            return Err(format!("probe int type mismatch: {key}"));
        }
    }

    let rows = match obj.get("proxy_blocked_targets").and_then(Value::as_array) {
        Some(v) => v,
        None => return Err("probe proxy_blocked_targets type mismatch".to_string()),
    };

    for row in rows {
        let row_obj = match row.as_object() {
            Some(v) => v,
            None => return Err("probe proxy_blocked_targets row schema mismatch".to_string()),
        };
        let row_keys: BTreeSet<&str> = row_obj.keys().map(String::as_str).collect();
        let expected: BTreeSet<&str> = ["url", "ok"].into_iter().collect();
        if row_keys != expected {
            return Err("probe proxy_blocked_targets row schema mismatch".to_string());
        }
        if !row_obj.get("url").is_some_and(Value::is_string)
            || !row_obj.get("ok").is_some_and(Value::is_boolean)
        {
            return Err("probe proxy_blocked_targets row type mismatch".to_string());
        }
    }

    let allowed_errors: BTreeSet<&str> = [
        "none",
        "proxy_listener_not_found",
        "proxy_connect_or_upstream_failed",
        "unknown",
    ]
    .into_iter()
    .collect();
    if !allowed_errors.contains(get_str(obj, "proxy_probe_error")) {
        return Err("probe proxy_probe_error invalid".to_string());
    }

    let proxy_probe_ok = get_bool(obj, "proxy_probe_ok");
    let proxy_probe_attempted = get_bool(obj, "proxy_probe_attempted");
    let proxy_listener_detected = get_bool(obj, "proxy_listener_detected");
    let skipped_no_proxy_listener = get_bool(obj, "skipped_no_proxy_listener");
    let skipped_no_curl = get_bool(obj, "skipped_no_curl");

    if proxy_probe_ok && !proxy_probe_attempted {
        return Err("proxy_probe_ok requires proxy_probe_attempted".to_string());
    }
    if proxy_probe_attempted && !proxy_listener_detected {
        return Err("proxy probe attempted without listener".to_string());
    }
    if proxy_listener_detected && skipped_no_proxy_listener {
        return Err("listener detected but skipped_no_proxy_listener=true".to_string());
    }
    if skipped_no_proxy_listener && proxy_probe_attempted {
        return Err("skipped_no_proxy_listener=true but proxy probe attempted".to_string());
    }
    if skipped_no_curl && proxy_probe_attempted {
        return Err("no curl but proxy probe attempted".to_string());
    }
    if get_str(obj, "proxy_probe_error") == "none" && !proxy_probe_ok && proxy_probe_attempted {
        return Err("proxy probe attempted without error marker".to_string());
    }
    if !proxy_probe_attempted && get_str(obj, "proxy_probe_error") != "proxy_listener_not_found" {
        return Err("proxy probe not attempted must report proxy_listener_not_found".to_string());
    }
    if proxy_probe_attempted && get_str(obj, "proxy_probe_error") == "proxy_listener_not_found" {
        return Err("proxy probe attempted with listener_not_found error".to_string());
    }
    if proxy_listener_detected && get_str(obj, "proxy_probe_error") == "proxy_listener_not_found" {
        return Err("listener detected but proxy_probe_error=proxy_listener_not_found".to_string());
    }
    if !proxy_probe_attempted && total_or_zero(obj) != 0 {
        return Err("proxy probe not attempted must have zero target totals".to_string());
    }
    if !proxy_probe_attempted && !rows.is_empty() {
        return Err("proxy probe not attempted must have empty target rows".to_string());
    }
    if !skipped_no_curl && !proxy_probe_attempted && !skipped_no_proxy_listener {
        return Err(
            "proxy probe not attempted must set skipped_no_proxy_listener=true".to_string(),
        );
    }
    if proxy_probe_attempted && skipped_no_proxy_listener {
        return Err("proxy probe attempted requires skipped_no_proxy_listener=false".to_string());
    }

    if skipped_no_curl {
        if get_bool(obj, "direct_probe_ok") || proxy_probe_ok {
            return Err("skipped_no_curl incompatible with successful probes".to_string());
        }
        if proxy_listener_detected || skipped_no_proxy_listener {
            return Err("skipped_no_curl incompatible with proxy listener branch".to_string());
        }
    }

    let total = get_i64(obj, "proxy_blocked_targets_total");
    let ok = get_i64(obj, "proxy_blocked_targets_ok");
    let failed = get_i64(obj, "proxy_blocked_targets_failed");

    if ok + failed != total {
        return Err("proxy blocked totals mismatch".to_string());
    }
    if rows.len() as i64 != total {
        return Err("proxy blocked list length mismatch".to_string());
    }
    if proxy_probe_attempted && total <= 0 {
        return Err("proxy probe attempted with empty target set".to_string());
    }

    let ok_count = rows
        .iter()
        .filter(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false))
        .count() as i64;
    if ok_count != ok {
        return Err("proxy blocked ok mismatch".to_string());
    }

    let blocked_targets_csv = get_str(obj, "blocked_targets");
    if blocked_targets_csv != normalize_blocked_targets_csv(blocked_targets_csv) {
        return Err("blocked_targets csv is not normalized".to_string());
    }

    if proxy_probe_attempted {
        let blocked_targets_list: Vec<&str> = if blocked_targets_csv.is_empty() {
            Vec::new()
        } else {
            blocked_targets_csv.split(',').collect()
        };
        if blocked_targets_list
            .iter()
            .any(|target| !is_supported_probe_url(target))
        {
            return Err("blocked_targets contains non-http/https url".to_string());
        }
        let row_urls: Vec<&str> = rows
            .iter()
            .filter_map(|row| row.get("url").and_then(Value::as_str))
            .collect();
        if blocked_targets_list != row_urls {
            return Err("blocked_targets csv/row url mismatch".to_string());
        }
    }

    Ok(())
}

fn total_or_zero(obj: &serde_json::Map<String, Value>) -> i64 {
    get_i64(obj, "proxy_blocked_targets_total")
}

fn normalize_blocked_targets_csv(csv: &str) -> String {
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

fn is_supported_proxy_url(value: &str) -> bool {
    let Some((scheme, rest)) = value.split_once("://") else {
        return false;
    };
    if scheme.is_empty() || rest.is_empty() || !is_valid_scheme_token(scheme) {
        return false;
    }
    let authority = extract_authority(rest);
    !authority.trim().is_empty()
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
        extract_authority, is_supported_probe_url, is_supported_proxy_url,
        normalize_blocked_targets_csv, validate_probe,
    };
    use serde_json::json;

    fn base_probe() -> serde_json::Value {
        json!({
            "status":"ok",
            "kind":"runtime_real_world_probe_smoke",
            "message_en":"ok",
            "message_ru":"ok",
            "direct_url":"https://direct.example",
            "blocked_targets":"https://blocked1.example,https://blocked2.example",
            "proxy_url":"socks5h://proxy.local:11080",
            "proxy_candidates":"socks5h://proxy.local:11080,http://proxy.local:1080",
            "proxy_selected_from_candidates": false,
            "direct_probe_ok": true,
            "proxy_probe_ok": true,
            "proxy_listener_detected": true,
            "proxy_probe_attempted": true,
            "proxy_probe_error":"none",
            "direct_timeout_sec": 8,
            "proxy_timeout_sec": 12,
            "connect_timeout_ms": 350,
            "proxy_blocked_targets_total": 2,
            "proxy_blocked_targets_ok": 2,
            "proxy_blocked_targets_failed": 0,
            "proxy_blocked_targets":[
                {"url":"https://blocked1.example","ok":true},
                {"url":"https://blocked2.example","ok":true}
            ],
            "skipped_no_curl": false,
            "skipped_no_proxy_listener": false,
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
        payload["proxy_blocked_targets_total"] = json!(3);
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_listener_detected_with_not_found_error() {
        let mut payload = base_probe();
        payload["proxy_probe_attempted"] = json!(true);
        payload["proxy_listener_detected"] = json!(true);
        payload["proxy_probe_error"] = json!("proxy_listener_not_found");
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_accepts_attempted_with_unknown_error() {
        let mut payload = base_probe();
        payload["proxy_probe_attempted"] = json!(true);
        payload["proxy_listener_detected"] = json!(true);
        payload["skipped_no_proxy_listener"] = json!(false);
        payload["proxy_probe_ok"] = json!(false);
        payload["proxy_probe_error"] = json!("unknown");
        payload["proxy_blocked_targets_total"] = json!(2);
        payload["proxy_blocked_targets_ok"] = json!(0);
        payload["proxy_blocked_targets_failed"] = json!(2);
        payload["proxy_blocked_targets"] = json!([
            {"url":"https://blocked1.example","ok":false},
            {"url":"https://blocked2.example","ok":false}
        ]);
        assert!(validate_probe(&payload).is_ok());
    }

    #[test]
    fn validate_probe_rejects_not_attempted_with_non_not_found_error() {
        let mut payload = base_probe();
        payload["proxy_probe_attempted"] = json!(false);
        payload["proxy_listener_detected"] = json!(false);
        payload["skipped_no_proxy_listener"] = json!(true);
        payload["proxy_probe_ok"] = json!(false);
        payload["proxy_probe_error"] = json!("none");
        payload["proxy_blocked_targets_total"] = json!(0);
        payload["proxy_blocked_targets_ok"] = json!(0);
        payload["proxy_blocked_targets_failed"] = json!(0);
        payload["proxy_blocked_targets"] = json!([]);
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_not_attempted_without_skipped_no_proxy_listener() {
        let mut payload = base_probe();
        payload["proxy_probe_attempted"] = json!(false);
        payload["proxy_listener_detected"] = json!(false);
        payload["skipped_no_proxy_listener"] = json!(false);
        payload["proxy_probe_ok"] = json!(false);
        payload["proxy_probe_error"] = json!("proxy_listener_not_found");
        payload["proxy_blocked_targets_total"] = json!(0);
        payload["proxy_blocked_targets_ok"] = json!(0);
        payload["proxy_blocked_targets_failed"] = json!(0);
        payload["proxy_blocked_targets"] = json!([]);
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_attempted_with_skipped_no_proxy_listener() {
        let mut payload = base_probe();
        payload["proxy_probe_attempted"] = json!(true);
        payload["proxy_listener_detected"] = json!(true);
        payload["skipped_no_proxy_listener"] = json!(true);
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_blocked_targets_csv_row_mismatch() {
        let mut payload = base_probe();
        payload["blocked_targets"] = json!("https://blocked1.example");
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_non_normalized_blocked_targets_csv() {
        let mut payload = base_probe();
        payload["blocked_targets"] =
            json!(" https://blocked1.example ,https://blocked1.example,https://blocked2.example ");
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn normalize_blocked_targets_csv_dedups_and_trims() {
        assert_eq!(
            normalize_blocked_targets_csv(
                " https://blocked1.example ,https://blocked1.example,https://blocked2.example "
            ),
            "https://blocked1.example,https://blocked2.example"
        );
    }

    #[test]
    fn validate_probe_rejects_not_attempted_with_non_empty_rows() {
        let mut payload = base_probe();
        payload["proxy_probe_attempted"] = json!(false);
        payload["proxy_listener_detected"] = json!(false);
        payload["skipped_no_proxy_listener"] = json!(true);
        payload["proxy_probe_ok"] = json!(false);
        payload["proxy_probe_error"] = json!("proxy_listener_not_found");
        payload["proxy_blocked_targets_total"] = json!(0);
        payload["proxy_blocked_targets_ok"] = json!(0);
        payload["proxy_blocked_targets_failed"] = json!(0);
        payload["proxy_blocked_targets"] = json!([{"url":"https://blocked1.example","ok":false}]);
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_empty_required_strings() {
        let mut payload = base_probe();
        payload["direct_url"] = json!("   ");
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_proxy_url_without_scheme() {
        let mut payload = base_probe();
        payload["proxy_url"] = json!("127.0.0.1:11080");
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_proxy_url_with_empty_authority() {
        let mut payload = base_probe();
        payload["proxy_url"] = json!("socks5h://");
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_non_http_direct_url() {
        let mut payload = base_probe();
        payload["direct_url"] = json!("socks5h://127.0.0.1:11080");
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn validate_probe_rejects_non_http_blocked_target_url() {
        let mut payload = base_probe();
        payload["blocked_targets"] = json!("https://blocked1.example,socks5h://proxy.example");
        payload["proxy_blocked_targets_total"] = json!(2);
        payload["proxy_blocked_targets_ok"] = json!(2);
        payload["proxy_blocked_targets_failed"] = json!(0);
        payload["proxy_blocked_targets"] = json!([
            {"url":"https://blocked1.example","ok":true},
            {"url":"socks5h://proxy.example","ok":true}
        ]);
        assert!(validate_probe(&payload).is_err());
    }

    #[test]
    fn supported_probe_url_requires_http_or_https() {
        assert!(is_supported_probe_url("https://blocked1.example"));
        assert!(is_supported_probe_url("http://blocked1.example"));
        assert!(is_supported_probe_url("HTTPS://blocked1.example"));
        assert!(!is_supported_probe_url("h*ttps://blocked1.example"));
        assert!(!is_supported_probe_url("https://[]"));
        assert!(!is_supported_probe_url("https://?q=1"));
        assert!(!is_supported_probe_url("ws://blocked1.example"));
        assert!(!is_supported_probe_url("wss://blocked1.example"));
        assert!(!is_supported_probe_url("socks5h://127.0.0.1:11080"));
        assert!(!is_supported_probe_url("https:// "));
        assert!(!is_supported_probe_url("https://bad host"));
        assert!(!is_supported_probe_url("blocked1.example"));
    }

    #[test]
    fn supported_proxy_url_requires_non_blank_non_spaced_authority() {
        assert!(is_supported_proxy_url("socks5h://127.0.0.1:11080"));
        assert!(is_supported_proxy_url("http://localhost:8080"));
        assert!(is_supported_proxy_url("SOCKS5H://127.0.0.1:11080"));
        assert!(!is_supported_proxy_url("127.0.0.1:11080"));
        assert!(!is_supported_proxy_url("1socks://127.0.0.1:11080"));
        assert!(!is_supported_proxy_url("so*cks://127.0.0.1:11080"));
        assert!(!is_supported_proxy_url("socks5h://[]:11080"));
        assert!(!is_supported_proxy_url("socks5h://[::1"));
        assert!(!is_supported_proxy_url("socks5h://:1080"));
        assert!(!is_supported_proxy_url("socks5h://user@:1080"));
        assert!(!is_supported_proxy_url("socks5h://user:pass@?q=1"));
        assert!(!is_supported_proxy_url("socks5h://#frag"));
        assert!(!is_supported_proxy_url("socks5h://"));
        assert!(!is_supported_proxy_url("socks5h:// "));
        assert!(!is_supported_proxy_url("socks5h://bad host:1080"));
    }

    #[test]
    fn extract_authority_stops_on_path_query_and_fragment() {
        assert_eq!(extract_authority("host:1234/path"), "host:1234");
        assert_eq!(extract_authority("host:1234?x=1"), "host:1234");
        assert_eq!(extract_authority("host:1234#frag"), "host:1234");
        assert_eq!(extract_authority("host:1234/path?x=1#frag"), "host:1234");
    }
}
