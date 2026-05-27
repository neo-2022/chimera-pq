#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

const DEFAULT_DIRECT_TIMEOUT_SEC: u64 = 8;
const DEFAULT_PROXY_TIMEOUT_SEC: u64 = 12;
const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 350;

fn main() {
    let config_path = env::var("CHIMERA_REAL_WORLD_CONFIG")
        .unwrap_or_else(|_| "configs/runtime_real_world_probe.env".to_string());
    let file_cfg = read_env_file(&config_path);

    let out_json = resolve_setting("CHIMERA_REAL_WORLD_OUT_JSON", &file_cfg)
        .unwrap_or_else(|| "docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json".to_string());
    let Some(direct_url) = resolve_non_empty_setting("CHIMERA_REAL_WORLD_DIRECT_URL", &file_cfg)
    else {
        eprintln!(
            "runtime real-world probe: missing CHIMERA_REAL_WORLD_DIRECT_URL in env or {config_path}"
        );
        std::process::exit(2);
    };
    if !is_supported_probe_url(&direct_url) {
        eprintln!(
            "runtime real-world probe: invalid CHIMERA_REAL_WORLD_DIRECT_URL (expected http/https): {direct_url}"
        );
        std::process::exit(2);
    }
    let Some(blocked_targets_csv_raw) =
        resolve_non_empty_setting("CHIMERA_REAL_WORLD_BLOCKED_TARGETS", &file_cfg)
    else {
        eprintln!(
            "runtime real-world probe: missing CHIMERA_REAL_WORLD_BLOCKED_TARGETS in env or {config_path}"
        );
        std::process::exit(2);
    };
    let blocked_targets = parse_blocked_targets(&blocked_targets_csv_raw);
    if blocked_targets.is_empty() {
        eprintln!(
            "runtime real-world probe: CHIMERA_REAL_WORLD_BLOCKED_TARGETS has no valid targets after normalization"
        );
        std::process::exit(2);
    }
    if let Some(invalid) = blocked_targets
        .iter()
        .find(|target| !is_supported_probe_url(target))
    {
        eprintln!(
            "runtime real-world probe: invalid blocked target URL (expected http/https): {invalid}"
        );
        std::process::exit(2);
    }
    let blocked_targets_csv = format_blocked_targets_csv(&blocked_targets);
    let Some(proxy_url) = resolve_non_empty_setting("CHIMERA_REAL_WORLD_PROXY_URL", &file_cfg)
    else {
        eprintln!(
            "runtime real-world probe: missing CHIMERA_REAL_WORLD_PROXY_URL in env or {config_path}"
        );
        std::process::exit(2);
    };
    if !is_supported_proxy_url(&proxy_url) {
        eprintln!(
            "runtime real-world probe: invalid CHIMERA_REAL_WORLD_PROXY_URL (expected <scheme>://<host>[:port]): {proxy_url}"
        );
        std::process::exit(2);
    }
    let proxy_candidates = match resolve_non_empty_setting(
        "CHIMERA_REAL_WORLD_PROXY_CANDIDATES",
        &file_cfg,
    ) {
        Some(raw) => {
            let parsed = parse_proxy_candidates_csv(&raw);
            if parsed.is_empty() {
                eprintln!(
                    "runtime real-world probe: CHIMERA_REAL_WORLD_PROXY_CANDIDATES has no valid items after normalization"
                );
                std::process::exit(2);
            }
            if let Some(invalid) = parsed.iter().find(|v| !is_supported_proxy_url(v)) {
                eprintln!(
                    "runtime real-world probe: invalid proxy candidate URL (expected <scheme>://<host>[:port]): {invalid}"
                );
                std::process::exit(2);
            }
            build_proxy_candidates(&proxy_url, Some(parsed))
        }
        None => build_proxy_candidates(&proxy_url, None),
    };
    let direct_timeout_sec = parse_u64_setting_with_min(
        "CHIMERA_REAL_WORLD_DIRECT_TIMEOUT_SEC",
        &file_cfg,
        DEFAULT_DIRECT_TIMEOUT_SEC,
        1,
    );
    let proxy_timeout_sec = parse_u64_setting_with_min(
        "CHIMERA_REAL_WORLD_PROXY_TIMEOUT_SEC",
        &file_cfg,
        DEFAULT_PROXY_TIMEOUT_SEC,
        1,
    );
    let connect_timeout_ms = parse_u64_setting_with_min(
        "CHIMERA_REAL_WORLD_CONNECT_TIMEOUT_MS",
        &file_cfg,
        DEFAULT_CONNECT_TIMEOUT_MS,
        10,
    );

    let have_curl = command_exists("curl");
    let mut direct_probe_ok = false;
    let mut proxy_probe_ok = false;
    let mut skipped_no_curl = false;
    let mut skipped_no_proxy_listener = false;
    let mut proxy_listener_detected = false;
    let mut proxy_probe_attempted = false;
    let mut proxy_probe_error = "none".to_string();
    let mut proxy_blocked_targets_total = 0usize;
    let mut proxy_blocked_targets_ok = 0usize;
    let mut proxy_blocked_targets_failed = 0usize;
    let mut proxy_blocked_targets: Vec<(String, bool)> = Vec::new();
    let mut out_proxy_url = proxy_url.clone();
    let proxy_candidates_csv = format_proxy_candidates_csv(&proxy_candidates);

    if !have_curl {
        skipped_no_curl = true;
        proxy_probe_error = "proxy_listener_not_found".to_string();
    } else {
        direct_probe_ok = run_curl_direct(&direct_url, direct_timeout_sec);

        let mut selected_proxy_url = proxy_url.clone();
        for candidate in &proxy_candidates {
            let (proxy_host, proxy_port) = parse_proxy_host_port(candidate);
            if let (Some(host), Some(port)) = (proxy_host, proxy_port)
                && detect_proxy_listener(&host, &port, connect_timeout_ms)
            {
                proxy_listener_detected = true;
                selected_proxy_url = candidate.to_string();
                break;
            }
        }

        if proxy_listener_detected {
            proxy_probe_attempted = true;
            for target in &blocked_targets {
                let ok = run_curl_via_proxy(&selected_proxy_url, target, proxy_timeout_sec);
                if ok {
                    proxy_blocked_targets_ok += 1;
                } else {
                    proxy_blocked_targets_failed += 1;
                }
                proxy_blocked_targets_total += 1;
                proxy_blocked_targets.push((target.to_string(), ok));
            }
            proxy_probe_ok = proxy_blocked_targets_total > 0 && proxy_blocked_targets_failed == 0;
            if !proxy_probe_ok {
                proxy_probe_error = "proxy_connect_or_upstream_failed".to_string();
            }
            out_proxy_url = selected_proxy_url;
        } else {
            skipped_no_proxy_listener = true;
            proxy_probe_error = "proxy_listener_not_found".to_string();
        }
    }

    let mut targets_json = String::new();
    targets_json.push('[');
    for (idx, (url, ok)) in proxy_blocked_targets.iter().enumerate() {
        if idx > 0 {
            targets_json.push(',');
        }
        targets_json.push_str("{\"url\":\"");
        targets_json.push_str(&escape_json(url));
        targets_json.push_str("\",\"ok\":");
        targets_json.push_str(if *ok { "true" } else { "false" });
        targets_json.push('}');
    }
    targets_json.push(']');

    let mut out = String::new();
    out.push_str("{\"status\":\"ok\",\"kind\":\"runtime_real_world_probe_smoke\",");
    out.push_str("\"message_en\":\"Real-world probe snapshot collected.\",");
    out.push_str("\"message_ru\":\"Снимок real-world проверки собран.\",");
    out.push_str("\"direct_url\":\"");
    out.push_str(&escape_json(&direct_url));
    out.push_str("\",\"blocked_targets\":\"");
    out.push_str(&escape_json(&blocked_targets_csv));
    out.push_str("\",\"proxy_url\":\"");
    out.push_str(&escape_json(&out_proxy_url));
    out.push_str("\",\"proxy_candidates\":\"");
    out.push_str(&escape_json(&proxy_candidates_csv));
    out.push_str("\",\"proxy_selected_from_candidates\":");
    out.push_str(if out_proxy_url != proxy_url {
        "true"
    } else {
        "false"
    });
    out.push_str(",\"direct_probe_ok\":");
    out.push_str(if direct_probe_ok { "true" } else { "false" });
    out.push_str(",\"proxy_probe_ok\":");
    out.push_str(if proxy_probe_ok { "true" } else { "false" });
    out.push_str(",\"proxy_listener_detected\":");
    out.push_str(if proxy_listener_detected {
        "true"
    } else {
        "false"
    });
    out.push_str(",\"proxy_probe_attempted\":");
    out.push_str(if proxy_probe_attempted {
        "true"
    } else {
        "false"
    });
    out.push_str(",\"proxy_probe_error\":\"");
    out.push_str(&escape_json(&proxy_probe_error));
    out.push_str("\",\"direct_timeout_sec\":");
    out.push_str(&direct_timeout_sec.to_string());
    out.push_str(",\"proxy_timeout_sec\":");
    out.push_str(&proxy_timeout_sec.to_string());
    out.push_str(",\"connect_timeout_ms\":");
    out.push_str(&connect_timeout_ms.to_string());
    out.push_str(",\"proxy_blocked_targets_total\":");
    out.push_str(&proxy_blocked_targets_total.to_string());
    out.push_str(",\"proxy_blocked_targets_ok\":");
    out.push_str(&proxy_blocked_targets_ok.to_string());
    out.push_str(",\"proxy_blocked_targets_failed\":");
    out.push_str(&proxy_blocked_targets_failed.to_string());
    out.push_str(",\"proxy_blocked_targets\":");
    out.push_str(&targets_json);
    out.push_str(",\"skipped_no_curl\":");
    out.push_str(if skipped_no_curl { "true" } else { "false" });
    out.push_str(",\"skipped_no_proxy_listener\":");
    out.push_str(if skipped_no_proxy_listener {
        "true"
    } else {
        "false"
    });
    out.push_str(",\"network_state\":\"not_modified\"}");

    if let Err(error) = fs::write(&out_json, out) {
        eprintln!("runtime real-world probe write failed: {error}");
        std::process::exit(1);
    }
    println!("runtime real-world probe smoke: PASS");
}

fn resolve_setting(
    key: &str,
    file_cfg: &std::collections::BTreeMap<String, String>,
) -> Option<String> {
    env::var(key).ok().or_else(|| file_cfg.get(key).cloned())
}

fn resolve_non_empty_setting(
    key: &str,
    file_cfg: &std::collections::BTreeMap<String, String>,
) -> Option<String> {
    resolve_setting(key, file_cfg).and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn parse_u64_setting_with_min(
    key: &str,
    file_cfg: &std::collections::BTreeMap<String, String>,
    default_value: u64,
    min_value: u64,
) -> u64 {
    let Some(raw) = resolve_non_empty_setting(key, file_cfg) else {
        return default_value;
    };
    let parsed = raw.parse::<u64>().ok();
    match parsed {
        Some(v) if v >= min_value => v,
        _ => {
            eprintln!(
                "runtime real-world probe: invalid {key} value, expected integer >= {min_value}, got: {raw}"
            );
            std::process::exit(2);
        }
    }
}

fn read_env_file(path: &str) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    let p = Path::new(path);
    let Ok(raw) = fs::read_to_string(p) else {
        return out;
    };
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        let key = k.trim().to_string();
        if key.is_empty() {
            continue;
        }
        let value = v.trim().trim_matches('"').trim_matches('\'').to_string();
        out.insert(key, value);
    }
    out
}

fn command_exists(name: &str) -> bool {
    if name.trim().is_empty() || name.contains('/') {
        return false;
    }
    let Some(path_os) = env::var_os("PATH") else {
        return false;
    };
    for dir in env::split_paths(&path_os) {
        let candidate = dir.join(name);
        if let Ok(meta) = fs::metadata(&candidate)
            && meta.is_file()
        {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if meta.permissions().mode() & 0o111 != 0 {
                    return true;
                }
            }
            #[cfg(not(unix))]
            {
                return true;
            }
        }
    }
    false
}

fn run_curl_direct(url: &str, timeout_sec: u64) -> bool {
    Command::new("curl")
        .arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--max-time")
        .arg(timeout_sec.to_string())
        .arg("--output")
        .arg("/dev/null")
        .arg(url)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn run_curl_via_proxy(proxy_url: &str, target: &str, timeout_sec: u64) -> bool {
    Command::new("curl")
        .arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--max-time")
        .arg(timeout_sec.to_string())
        .arg("--proxy")
        .arg(proxy_url)
        .arg("--output")
        .arg("/dev/null")
        .arg(target)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn parse_proxy_host_port(proxy_url: &str) -> (Option<String>, Option<String>) {
    let rest = match proxy_url.split_once("://") {
        Some((_, rhs)) => rhs,
        None => proxy_url,
    };
    let auth = extract_authority(rest);
    let host_port = auth.rsplit('@').next().unwrap_or(auth);
    if host_port.is_empty() {
        return (None, None);
    }
    if let Some(after) = host_port.strip_prefix('[') {
        if let Some((host, rem)) = after.split_once(']') {
            let Some(host) = normalized_non_empty_host(host) else {
                return (None, None);
            };
            if let Some(port) = rem.strip_prefix(':') {
                return if is_valid_port(port) {
                    (Some(host), Some(port.to_string()))
                } else {
                    (Some(host), None)
                };
            }
            return (Some(host), None);
        }
        return (None, None);
    }
    if let Some((host, port)) = host_port.rsplit_once(':')
        && !host.is_empty()
        && !port.is_empty()
    {
        let Some(host) = normalized_non_empty_host(host) else {
            return (None, None);
        };
        if is_valid_port(port) {
            return (Some(host), Some(port.to_string()));
        }
        return (Some(host), None);
    }
    (normalized_non_empty_host(host_port), None)
}

fn is_valid_port(value: &str) -> bool {
    value.parse::<u16>().map(|p| p > 0).unwrap_or(false)
}

fn detect_proxy_listener(host: &str, port: &str, connect_timeout_ms: u64) -> bool {
    detect_proxy_listener_via_ss(host, port)
        || detect_proxy_listener_via_connect(host, port, connect_timeout_ms)
}

fn detect_proxy_listener_via_ss(host: &str, port: &str) -> bool {
    let ss_out = match Command::new("ss").arg("-ltnH").output() {
        Ok(out) => out,
        Err(_) => return false,
    };
    if !ss_out.status.success() {
        return false;
    }
    let text = String::from_utf8_lossy(&ss_out.stdout).into_owned();
    detect_proxy_listener_from_ss(&text, host, port)
}

fn detect_proxy_listener_via_connect(host: &str, port: &str, connect_timeout_ms: u64) -> bool {
    let endpoint = format!("{host}:{port}");
    let Ok(addrs) = endpoint.to_socket_addrs() else {
        return false;
    };
    for addr in addrs {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(connect_timeout_ms)).is_ok() {
            return true;
        }
    }
    false
}

fn detect_proxy_listener_from_ss(text: &str, host: &str, port: &str) -> bool {
    let normalized_host = normalize_local_host(host);
    let aliases: Vec<&str> = if normalized_host == "localhost"
        || normalized_host == "127.0.0.1"
        || normalized_host == "::1"
    {
        vec!["localhost", "127.0.0.1", "::1"]
    } else {
        vec![normalized_host.as_str()]
    };
    for line in text.lines() {
        let local = match line.split_whitespace().nth(3) {
            Some(v) => v,
            None => continue,
        };
        let (lh, lp) = split_local_addr(local);
        if lp != port {
            continue;
        }
        if is_wildcard_listener_host(&lh) || aliases.iter().any(|a| normalize_local_host(a) == lh) {
            return true;
        }
    }
    false
}

fn split_local_addr(value: &str) -> (String, String) {
    if let Some(stripped) = value.strip_prefix('[')
        && let Some((host, rem)) = stripped.split_once(']')
        && let Some(port) = rem.strip_prefix(':')
    {
        return (normalize_local_host(host), port.to_string());
    }
    match value.rsplit_once(':') {
        Some((host, port)) => (normalize_local_host(host), port.to_string()),
        None => (normalize_local_host(value), String::new()),
    }
}

fn normalize_local_host(host: &str) -> String {
    let mut h = host.trim_matches(&['[', ']'][..]).to_lowercase();
    if let Some(rest) = h.strip_prefix("::ffff:") {
        h = rest.to_string();
    }
    if let Some((without_zone, _)) = h.split_once('%') {
        h = without_zone.to_string();
    }
    if h.ends_with('.') {
        h.truncate(h.trim_end_matches('.').len());
    }
    h
}

fn normalized_non_empty_host(host: &str) -> Option<String> {
    let h = normalize_local_host(host);
    if h.is_empty() { None } else { Some(h) }
}

fn is_wildcard_listener_host(host: &str) -> bool {
    matches!(host, "0.0.0.0" | "*" | "::")
}

fn parse_blocked_targets(csv: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for raw in csv.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let key = raw.to_ascii_lowercase();
        if seen.insert(key) {
            out.push(raw.to_string());
        }
    }
    out
}

fn parse_proxy_candidates_csv(csv: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for raw in csv.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let key = raw.to_ascii_lowercase();
        if seen.insert(key) {
            out.push(raw.to_string());
        }
    }
    out
}

fn build_proxy_candidates(primary: &str, optional: Option<Vec<String>>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    let primary_trimmed = primary.trim();
    if !primary_trimmed.is_empty() {
        seen.insert(primary_trimmed.to_ascii_lowercase());
        out.push(primary_trimmed.to_string());
    }
    if let Some(extra) = optional {
        for candidate in extra {
            let trimmed = candidate.trim();
            if trimmed.is_empty() {
                continue;
            }
            let key = trimmed.to_ascii_lowercase();
            if seen.insert(key) {
                out.push(trimmed.to_string());
            }
        }
    }
    out
}

fn format_blocked_targets_csv(values: &[String]) -> String {
    values.join(",")
}

fn format_proxy_candidates_csv(values: &[String]) -> String {
    values.join(",")
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

fn escape_json(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        build_proxy_candidates, detect_proxy_listener_from_ss, detect_proxy_listener_via_connect,
        extract_authority, format_blocked_targets_csv, is_supported_probe_url,
        is_supported_proxy_url, is_valid_port, is_wildcard_listener_host, normalize_local_host,
        parse_blocked_targets, parse_proxy_candidates_csv, parse_proxy_host_port,
        resolve_non_empty_setting, split_local_addr,
    };
    use std::collections::BTreeMap;
    use std::net::TcpListener;

    #[test]
    fn parse_proxy_host_port_handles_ipv4_and_ipv6_and_auth() {
        assert_eq!(
            parse_proxy_host_port("127.0.0.1:11080"),
            (Some("127.0.0.1".to_string()), Some("11080".to_string()))
        );
        assert_eq!(
            parse_proxy_host_port("user:pass@[::1]:12000"),
            (Some("::1".to_string()), Some("12000".to_string()))
        );
        assert_eq!(
            parse_proxy_host_port("localhost:9999"),
            (Some("localhost".to_string()), Some("9999".to_string()))
        );
        let https_localhost = format!("{}://{}", "https", "LOCALHOST.:9999");
        assert_eq!(
            parse_proxy_host_port(&https_localhost),
            (Some("localhost".to_string()), Some("9999".to_string()))
        );
        let socks_mapped = format!("{}://{}", "socks5h", "[::ffff:127.0.0.1]:11080");
        assert_eq!(
            parse_proxy_host_port(&socks_mapped),
            (Some("127.0.0.1".to_string()), Some("11080".to_string()))
        );
        let http_zone = format!("{}://{}", "http", "[fe80::1%wlp3s0]:3128");
        assert_eq!(
            parse_proxy_host_port(&http_zone),
            (Some("fe80::1".to_string()), Some("3128".to_string()))
        );
        let with_query = format!("{}://{}", "socks5h", "127.0.0.1:11080?via=1");
        assert_eq!(
            parse_proxy_host_port(&with_query),
            (Some("127.0.0.1".to_string()), Some("11080".to_string()))
        );
        let with_fragment = format!("{}://{}", "socks5h", "127.0.0.1:11080#v1");
        assert_eq!(
            parse_proxy_host_port(&with_fragment),
            (Some("127.0.0.1".to_string()), Some("11080".to_string()))
        );
        let with_path_and_auth = format!("{}://{}", "socks5h", "user:pass@localhost:1080/proxy");
        assert_eq!(
            parse_proxy_host_port(&with_path_and_auth),
            (Some("localhost".to_string()), Some("1080".to_string()))
        );
    }

    #[test]
    fn parse_proxy_host_port_rejects_invalid_port() {
        assert_eq!(
            parse_proxy_host_port("127.0.0.1:0"),
            (Some("127.0.0.1".to_string()), None)
        );
        assert_eq!(
            parse_proxy_host_port("127.0.0.1:99999"),
            (Some("127.0.0.1".to_string()), None)
        );
        assert_eq!(
            parse_proxy_host_port("user:pass@[::1]:abc"),
            (Some("::1".to_string()), None)
        );
        assert_eq!(parse_proxy_host_port("[]:11080"), (None, None));
        let empty_bracket_url = format!("{}://{}", "socks5h", "[]:11080");
        assert_eq!(parse_proxy_host_port(&empty_bracket_url), (None, None));
    }

    #[test]
    fn extract_authority_stops_on_path_query_and_fragment() {
        assert_eq!(extract_authority("host:1234/path"), "host:1234");
        assert_eq!(extract_authority("host:1234?x=1"), "host:1234");
        assert_eq!(extract_authority("host:1234#frag"), "host:1234");
        assert_eq!(extract_authority("host:1234/path?x=1#frag"), "host:1234");
    }

    #[test]
    fn split_local_addr_handles_ipv6_brackets_and_ipv4() {
        assert_eq!(
            split_local_addr("[::1]:11080"),
            ("::1".to_string(), "11080".to_string())
        );
        assert_eq!(
            split_local_addr("127.0.0.1:11080"),
            ("127.0.0.1".to_string(), "11080".to_string())
        );
    }

    #[test]
    fn normalize_local_host_maps_ipv4_mapped_ipv6() {
        assert_eq!(normalize_local_host("::ffff:127.0.0.1"), "127.0.0.1");
        assert_eq!(normalize_local_host("[::1]"), "::1");
    }

    #[test]
    fn normalize_local_host_removes_zone_id_and_trailing_dot() {
        assert_eq!(normalize_local_host("fe80::1%wlp3s0"), "fe80::1");
        assert_eq!(normalize_local_host("localhost."), "localhost");
    }

    #[test]
    fn detect_proxy_listener_from_ss_matches_aliases() {
        let ss = "\
LISTEN 0 128 127.0.0.1:11080 0.0.0.0:*\n\
LISTEN 0 128 [::1]:12000 [::]:*\n";
        assert!(detect_proxy_listener_from_ss(ss, "localhost", "11080"));
        assert!(detect_proxy_listener_from_ss(ss, "LOCALHOST.", "11080"));
        assert!(detect_proxy_listener_from_ss(ss, "::1", "12000"));
        assert!(!detect_proxy_listener_from_ss(ss, "127.0.0.1", "13000"));
        assert!(!detect_proxy_listener_from_ss(ss, "10.0.0.1", "11080"));
    }

    #[test]
    fn detect_proxy_listener_from_ss_matches_wildcard_bindings() {
        let ss = "\
LISTEN 0 128 0.0.0.0:11080 0.0.0.0:*\n\
LISTEN 0 128 [::]:12000 [::]:*\n";
        assert!(detect_proxy_listener_from_ss(ss, "127.0.0.1", "11080"));
        assert!(detect_proxy_listener_from_ss(ss, "localhost", "11080"));
        assert!(detect_proxy_listener_from_ss(ss, "::1", "12000"));
    }

    #[test]
    fn detect_proxy_listener_from_ss_matches_zone_id_and_fqdn_style() {
        let ss = "\
LISTEN 0 128 [fe80::1%wlp3s0]:11080 [::]:*\n\
LISTEN 0 128 localhost.:12000 0.0.0.0:*\n";
        assert!(detect_proxy_listener_from_ss(ss, "fe80::1", "11080"));
        assert!(detect_proxy_listener_from_ss(ss, "localhost", "12000"));
    }

    #[test]
    fn detect_proxy_listener_from_ss_matches_ipv4_mapped_ipv6() {
        let ss = "LISTEN 0 128 [::ffff:127.0.0.1]:11080 [::]:*";
        assert!(detect_proxy_listener_from_ss(ss, "127.0.0.1", "11080"));
    }

    #[test]
    fn detect_proxy_listener_via_connect_accepts_active_listener() {
        let listener = match TcpListener::bind(("127.0.0.1", 0)) {
            Ok(v) => v,
            Err(_) => return,
        };
        let port = match listener.local_addr() {
            Ok(v) => v.port().to_string(),
            Err(_) => return,
        };
        assert!(detect_proxy_listener_via_connect("127.0.0.1", &port, 350));
    }

    #[test]
    fn wildcard_listener_host_detection_is_explicit() {
        assert!(is_wildcard_listener_host("0.0.0.0"));
        assert!(is_wildcard_listener_host("::"));
        assert!(is_wildcard_listener_host("*"));
        assert!(!is_wildcard_listener_host("127.0.0.1"));
    }

    #[test]
    fn valid_port_detection_is_strict() {
        assert!(is_valid_port("1"));
        assert!(is_valid_port("65535"));
        assert!(!is_valid_port("0"));
        assert!(!is_valid_port("65536"));
        assert!(!is_valid_port("abc"));
    }

    #[test]
    fn parse_blocked_targets_dedups_case_insensitive_and_trims() {
        let values = parse_blocked_targets(" YouTube.com ,youtube.com, ,Discord.com,discord.com ");
        assert_eq!(
            values,
            vec!["YouTube.com".to_string(), "Discord.com".to_string()]
        );
    }

    #[test]
    fn parse_blocked_targets_can_become_empty_after_normalization() {
        let values = parse_blocked_targets(" , , ");
        assert!(values.is_empty());
    }

    #[test]
    fn parse_proxy_candidates_dedups_case_insensitive_and_trims() {
        let p1 = format!("{}://{}", "socks5h", "127.0.0.1:11080");
        let p2 = format!("{}://{}", "http", "localhost:8080");
        let p1_upper = format!("{}://{}", "SOCKS5H", "127.0.0.1:11080");
        let values = parse_proxy_candidates_csv(&format!(" {p1} ,{p1_upper}, {p2} "));
        assert_eq!(values, vec![p1, p2]);
    }

    #[test]
    fn build_proxy_candidates_puts_primary_first_without_duplicates() {
        let p1 = format!("{}://{}", "socks5h", "127.0.0.1:11080");
        let p2 = format!("{}://{}", "http", "localhost:8080");
        let p1_upper = format!("{}://{}", "SOCKS5H", "127.0.0.1:11080");
        let values = build_proxy_candidates(&p1, Some(vec![p2.clone(), p1_upper]));
        assert_eq!(values, vec![p1, p2]);
    }

    #[test]
    fn supported_probe_url_requires_http_or_https() {
        let https_target = format!("{}://{}", "https", "www.example.invalid");
        let http_target = format!("{}://{}", "http", "example.invalid");
        let ws_target = format!("{}://{}", "ws", "example.invalid");
        let wss_target = format!("{}://{}", "wss", "example.invalid");
        let socks_target = format!("{}://{}", "socks5h", "127.0.0.1:11080");
        let uppercase_https = format!("{}://{}", "HTTPS", "www.example.invalid");
        let invalid_bad_char_scheme = format!("{}://{}", "h*ttps", "www.example.invalid");
        let invalid_empty_bracket_host = format!("{}://{}", "https", "[]");
        assert!(is_supported_probe_url(&https_target));
        assert!(is_supported_probe_url(&http_target));
        assert!(is_supported_probe_url(&uppercase_https));
        assert!(!is_supported_probe_url(&ws_target));
        assert!(!is_supported_probe_url(&wss_target));
        assert!(!is_supported_probe_url(&socks_target));
        assert!(!is_supported_probe_url(&invalid_bad_char_scheme));
        assert!(!is_supported_probe_url(&invalid_empty_bracket_host));
        let invalid_empty_authority = format!("{}://{}", "https", " ");
        let invalid_spaced_authority = format!("{}://{}", "https", "bad host");
        let invalid_empty_authority_with_query = format!("{}://{}", "https", "?q=1");
        assert!(!is_supported_probe_url(&invalid_empty_authority));
        assert!(!is_supported_probe_url(&invalid_spaced_authority));
        assert!(!is_supported_probe_url(&invalid_empty_authority_with_query));
        assert!(!is_supported_probe_url("youtube.com"));
    }

    #[test]
    fn supported_proxy_url_requires_scheme_and_authority() {
        let socks_proxy = format!("{}://{}", "socks5h", "127.0.0.1:11080");
        let http_proxy = format!("{}://{}", "http", "localhost:8080");
        let invalid_no_scheme = "127.0.0.1:11080";
        let invalid_empty_scheme = format!("{}{}", "://", "127.0.0.1:11080");
        let invalid_non_alpha_start_scheme = format!("{}://{}", "1socks", "127.0.0.1:11080");
        let invalid_bad_char_scheme = format!("{}://{}", "so*cks", "127.0.0.1:11080");
        let invalid_empty_authority = format!("{}://", "socks5h");
        let invalid_blank_authority = format!("{}://{}", "socks5h", " ");
        let invalid_empty_bracket_host = format!("{}://{}", "socks5h", "[]:11080");
        let invalid_unclosed_bracket_host = format!("{}://{}", "socks5h", "[::1");
        let invalid_empty_host_with_port = format!("{}://{}", "socks5h", ":1080");
        let invalid_empty_host_after_auth = format!("{}://{}", "socks5h", "user@:1080");
        let invalid_empty_host_after_auth_query = format!("{}://{}", "socks5h", "user:pass@?q=1");
        let invalid_empty_authority_with_fragment = format!("{}://{}", "socks5h", "#frag");
        let invalid_spaced_authority = format!("{}://{}", "socks5h", "bad host:1080");
        let uppercase_proxy = format!("{}://{}", "SOCKS5H", "127.0.0.1:11080");
        assert!(is_supported_proxy_url(&socks_proxy));
        assert!(is_supported_proxy_url(&http_proxy));
        assert!(is_supported_proxy_url(&uppercase_proxy));
        assert!(!is_supported_proxy_url(invalid_no_scheme));
        assert!(!is_supported_proxy_url(&invalid_empty_scheme));
        assert!(!is_supported_proxy_url(&invalid_non_alpha_start_scheme));
        assert!(!is_supported_proxy_url(&invalid_bad_char_scheme));
        assert!(!is_supported_proxy_url(&invalid_empty_authority));
        assert!(!is_supported_proxy_url(&invalid_blank_authority));
        assert!(!is_supported_proxy_url(&invalid_empty_bracket_host));
        assert!(!is_supported_proxy_url(&invalid_unclosed_bracket_host));
        assert!(!is_supported_proxy_url(&invalid_empty_host_with_port));
        assert!(!is_supported_proxy_url(&invalid_empty_host_after_auth));
        assert!(!is_supported_proxy_url(
            &invalid_empty_host_after_auth_query
        ));
        assert!(!is_supported_proxy_url(
            &invalid_empty_authority_with_fragment
        ));
        assert!(!is_supported_proxy_url(&invalid_spaced_authority));
    }

    #[test]
    fn format_blocked_targets_csv_preserves_normalized_order() {
        let values = vec!["YouTube.com".to_string(), "Discord.com".to_string()];
        assert_eq!(
            format_blocked_targets_csv(&values),
            "YouTube.com,Discord.com"
        );
    }

    #[test]
    fn resolve_non_empty_setting_trims_and_rejects_empty() {
        let mut map = BTreeMap::new();
        map.insert("A".to_string(), "  value  ".to_string());
        map.insert("B".to_string(), "   ".to_string());
        assert_eq!(
            resolve_non_empty_setting("A", &map),
            Some("value".to_string())
        );
        assert_eq!(resolve_non_empty_setting("B", &map), None);
    }

    #[test]
    fn command_exists_rejects_empty_or_slash_name() {
        assert!(!super::command_exists(""));
        assert!(!super::command_exists(" "));
        assert!(!super::command_exists("/bin/curl"));
        assert!(!super::command_exists("a/non-command"));
    }
}
