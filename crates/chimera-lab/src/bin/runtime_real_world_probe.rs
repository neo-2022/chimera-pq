#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

const DEFAULT_DIRECT_TIMEOUT_SEC: u64 = 8;
const DEFAULT_DATAPATH_TIMEOUT_SEC: u64 = 12;
const DEFAULT_FLOW_MAX_AGE_SEC: u64 = 300;
const DEFAULT_STATE_PATH: &str = "docs/runtime_state_latest.json";
const MODE_LIVE: &str = "live";
const MODE_CI_SNAPSHOT: &str = "ci_snapshot";
const EVIDENCE_EXTERNAL_REACHABILITY: &str = "external_reachability_without_system_proxy";
const EVIDENCE_CI_SNAPSHOT: &str = "ci_snapshot_contract";
const EVIDENCE_CHIMERA_DATAPATH: &str = "chimera_transparent_datapath";
const DATAPATH_EVIDENCE_MISSING: &str = "chimera_datapath_evidence_missing";
const TRUTH_BOUNDARY_EXTERNAL: &str = "ordinary curl --noproxy proves external reachability without system proxy only; it is not CHIMERA/WEAVE datapath evidence";
const TRUTH_BOUNDARY_DATAPATH: &str = "strict CHIMERA flow-proof plus ordinary target outcomes provide CHIMERA/WEAVE datapath evidence; direct curl remains an external baseline only";

fn main() {
    let config_path = env::var("CHIMERA_REAL_WORLD_CONFIG")
        .unwrap_or_else(|_| "configs/runtime_real_world_probe.env".to_string());
    let file_cfg = read_env_file(&config_path);

    let out_json = resolve_setting("CHIMERA_REAL_WORLD_OUT_JSON", &file_cfg)
        .unwrap_or_else(|| "docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json".to_string());
    let probe_mode = resolve_probe_mode(&file_cfg);
    let direct_url = if probe_mode == MODE_CI_SNAPSHOT {
        String::new()
    } else {
        let Some(value) = resolve_non_empty_setting("CHIMERA_REAL_WORLD_DIRECT_URL", &file_cfg)
        else {
            eprintln!(
                "runtime real-world probe: missing CHIMERA_REAL_WORLD_DIRECT_URL in env or {config_path}"
            );
            std::process::exit(2);
        };
        if !is_supported_probe_url(&value) {
            eprintln!(
                "runtime real-world probe: invalid CHIMERA_REAL_WORLD_DIRECT_URL (expected http/https)"
            );
            std::process::exit(2);
        }
        value
    };

    let datapath_targets = if probe_mode == MODE_CI_SNAPSHOT {
        Vec::new()
    } else {
        let Some(datapath_targets_csv_raw) =
            resolve_non_empty_setting("CHIMERA_REAL_WORLD_DATAPATH_TARGETS", &file_cfg)
        else {
            eprintln!(
                "runtime real-world probe: missing CHIMERA_REAL_WORLD_DATAPATH_TARGETS in env or {config_path}"
            );
            std::process::exit(2);
        };
        let values = parse_datapath_targets(&datapath_targets_csv_raw);
        if values.is_empty() {
            eprintln!(
                "runtime real-world probe: CHIMERA_REAL_WORLD_DATAPATH_TARGETS has no valid targets after normalization"
            );
            std::process::exit(2);
        }
        if values.iter().any(|target| !is_supported_probe_url(target)) {
            eprintln!(
                "runtime real-world probe: invalid datapath target URL (expected http/https)"
            );
            std::process::exit(2);
        }
        values
    };
    let direct_timeout_sec = parse_u64_setting_with_min(
        "CHIMERA_REAL_WORLD_DIRECT_TIMEOUT_SEC",
        &file_cfg,
        DEFAULT_DIRECT_TIMEOUT_SEC,
        1,
    );
    let datapath_timeout_sec = parse_u64_setting_with_min(
        "CHIMERA_REAL_WORLD_DATAPATH_TIMEOUT_SEC",
        &file_cfg,
        DEFAULT_DATAPATH_TIMEOUT_SEC,
        1,
    );
    let have_curl = command_exists("curl");
    let mut direct_probe_ok = false;
    let mut datapath_probe_ok = false;
    let mut skipped_no_curl = false;
    let mut datapath_probe_attempted = false;
    let datapath_probe_error: String;
    let mut datapath_targets_total = 0usize;
    let mut datapath_targets_ok = 0usize;
    let mut datapath_targets_failed = 0usize;
    let mut datapath_target_rows: Vec<(String, bool)> = Vec::new();
    let mut external_reachability_probe_attempted = false;
    let mut external_reachability_probe_ok = false;
    let mut external_reachability_targets_total = 0usize;
    let mut external_reachability_targets_ok = 0usize;
    let mut external_reachability_targets_failed = 0usize;
    let mut external_reachability_target_rows: Vec<(String, bool)> = Vec::new();
    let mut evidence_kind = if probe_mode == MODE_CI_SNAPSHOT {
        EVIDENCE_CI_SNAPSHOT
    } else {
        EVIDENCE_EXTERNAL_REACHABILITY
    };
    let mut chimera_datapath_evidence = false;
    let mut truth_boundary = TRUTH_BOUNDARY_EXTERNAL;
    let mut message_en =
        "External reachability snapshot collected; CHIMERA datapath evidence not collected.";
    let mut message_ru =
        "Снимок внешней доступности собран; доказательство CHIMERA datapath не собрано.";

    if probe_mode == MODE_CI_SNAPSHOT {
        datapath_probe_error = MODE_CI_SNAPSHOT.to_string();
    } else if !have_curl {
        skipped_no_curl = true;
        datapath_probe_error = "curl_not_found".to_string();
    } else {
        external_reachability_probe_attempted = true;
        direct_probe_ok = run_curl_plain(&direct_url, direct_timeout_sec);
        for target in &datapath_targets {
            let redacted_target_ref = target_ref(external_reachability_targets_total);
            let ok = run_curl_plain(target, datapath_timeout_sec);
            if ok {
                external_reachability_targets_ok += 1;
            } else {
                external_reachability_targets_failed += 1;
            }
            external_reachability_targets_total += 1;
            external_reachability_target_rows.push((redacted_target_ref, ok));
        }
        external_reachability_probe_ok =
            external_reachability_targets_total > 0 && external_reachability_targets_failed == 0;
        if strict_flow_proof_ok(&file_cfg) {
            truth_boundary = TRUTH_BOUNDARY_DATAPATH;
            evidence_kind = EVIDENCE_CHIMERA_DATAPATH;
            chimera_datapath_evidence = true;
            datapath_probe_attempted = true;
            datapath_targets_total = external_reachability_targets_total;
            datapath_targets_ok = external_reachability_targets_ok;
            datapath_targets_failed = external_reachability_targets_failed;
            datapath_probe_ok = datapath_targets_total > 0 && datapath_targets_failed == 0;
            datapath_target_rows = external_reachability_target_rows.clone();
            if datapath_probe_ok {
                datapath_probe_error = "none".to_string();
                message_en = "Strict CHIMERA datapath evidence collected.";
                message_ru = "Собрано строгое доказательство CHIMERA datapath.";
            } else {
                datapath_probe_error = "datapath_target_failed".to_string();
                message_en = "Strict CHIMERA datapath evidence collected; one or more ordinary target flows failed.";
                message_ru = "Собрано строгое доказательство CHIMERA datapath; один или несколько ordinary target flow не прошли.";
            }
        } else {
            datapath_probe_error = DATAPATH_EVIDENCE_MISSING.to_string();
        }
    }

    let mut targets_json = String::new();
    targets_json.push('[');
    for (idx, (redacted_target_ref, ok)) in datapath_target_rows.iter().enumerate() {
        if idx > 0 {
            targets_json.push(',');
        }
        targets_json.push_str("{\"url\":\"");
        targets_json.push_str(&escape_json(redacted_target_ref));
        targets_json.push_str("\",\"ok\":");
        targets_json.push_str(if *ok { "true" } else { "false" });
        targets_json.push('}');
    }
    targets_json.push(']');

    let mut external_targets_json = String::new();
    external_targets_json.push('[');
    for (idx, (redacted_target_ref, ok)) in external_reachability_target_rows.iter().enumerate() {
        if idx > 0 {
            external_targets_json.push(',');
        }
        external_targets_json.push_str("{\"url\":\"");
        external_targets_json.push_str(&escape_json(redacted_target_ref));
        external_targets_json.push_str("\",\"ok\":");
        external_targets_json.push_str(if *ok { "true" } else { "false" });
        external_targets_json.push('}');
    }
    external_targets_json.push(']');

    let mut out = String::new();
    out.push_str("{\"status\":\"ok\",\"kind\":\"runtime_real_world_probe_smoke\",");
    out.push_str("\"message_en\":\"");
    out.push_str(&escape_json(message_en));
    out.push_str("\",");
    out.push_str("\"message_ru\":\"");
    out.push_str(&escape_json(message_ru));
    out.push_str("\",");
    out.push_str("\"evidence_kind\":\"");
    out.push_str(evidence_kind);
    out.push_str("\",\"chimera_datapath_evidence\":");
    out.push_str(if chimera_datapath_evidence {
        "true"
    } else {
        "false"
    });
    out.push_str(",\"truth_boundary\":\"");
    out.push_str(&escape_json(truth_boundary));
    out.push_str("\",");
    out.push_str("\"probe_mode\":\"");
    out.push_str(&escape_json(&probe_mode));
    out.push_str("\",");
    out.push_str("\"live_external_probe\":");
    out.push_str(if probe_mode == MODE_LIVE {
        "true"
    } else {
        "false"
    });
    out.push_str(",\"ssh_stand_required_for_live_probe\":");
    out.push_str(if probe_mode == MODE_LIVE {
        "false"
    } else {
        "true"
    });
    out.push(',');
    let datapath_target_refs_csv = format_target_refs_csv(datapath_targets.len());
    let external_target_refs_csv = format_target_refs_csv(external_reachability_targets_total);
    out.push_str("\"direct_url\":\"");
    out.push_str(if probe_mode == MODE_LIVE {
        "direct#1"
    } else {
        ""
    });
    out.push_str("\",\"datapath_targets\":\"");
    out.push_str(&escape_json(if chimera_datapath_evidence {
        &datapath_target_refs_csv
    } else {
        ""
    }));
    out.push_str("\",\"direct_probe_ok\":");
    out.push_str(if direct_probe_ok { "true" } else { "false" });
    out.push_str(",\"datapath_probe_ok\":");
    out.push_str(if datapath_probe_ok { "true" } else { "false" });
    out.push_str(",\"datapath_probe_attempted\":");
    out.push_str(if datapath_probe_attempted {
        "true"
    } else {
        "false"
    });
    out.push_str(",\"datapath_probe_error\":\"");
    out.push_str(&escape_json(&datapath_probe_error));
    out.push_str("\",\"direct_timeout_sec\":");
    out.push_str(&direct_timeout_sec.to_string());
    out.push_str(",\"datapath_timeout_sec\":");
    out.push_str(&datapath_timeout_sec.to_string());
    out.push_str(",\"datapath_targets_total\":");
    out.push_str(&datapath_targets_total.to_string());
    out.push_str(",\"datapath_targets_ok\":");
    out.push_str(&datapath_targets_ok.to_string());
    out.push_str(",\"datapath_targets_failed\":");
    out.push_str(&datapath_targets_failed.to_string());
    out.push_str(",\"datapath_target_results\":");
    out.push_str(&targets_json);
    out.push_str(",\"external_reachability_probe_attempted\":");
    out.push_str(if external_reachability_probe_attempted {
        "true"
    } else {
        "false"
    });
    out.push_str(",\"external_reachability_probe_ok\":");
    out.push_str(if external_reachability_probe_ok {
        "true"
    } else {
        "false"
    });
    out.push_str(",\"external_reachability_targets\":\"");
    out.push_str(&escape_json(&external_target_refs_csv));
    out.push_str("\",\"external_reachability_targets_total\":");
    out.push_str(&external_reachability_targets_total.to_string());
    out.push_str(",\"external_reachability_targets_ok\":");
    out.push_str(&external_reachability_targets_ok.to_string());
    out.push_str(",\"external_reachability_targets_failed\":");
    out.push_str(&external_reachability_targets_failed.to_string());
    out.push_str(",\"external_reachability_target_results\":");
    out.push_str(&external_targets_json);
    out.push_str(",\"skipped_no_curl\":");
    out.push_str(if skipped_no_curl { "true" } else { "false" });
    out.push_str(",\"network_state\":\"not_modified\"}");

    if let Err(error) = fs::write(&out_json, out) {
        eprintln!("runtime real-world probe write failed: {error}");
        std::process::exit(1);
    }
    println!("runtime real-world probe smoke: PASS");
}

fn resolve_probe_mode(file_cfg: &std::collections::BTreeMap<String, String>) -> String {
    let raw = resolve_non_empty_setting("CHIMERA_REAL_WORLD_PROBE_MODE", file_cfg);
    match select_probe_mode(raw) {
        Ok(mode) => mode,
        Err(raw) => {
            eprintln!(
                "runtime real-world probe: invalid CHIMERA_REAL_WORLD_PROBE_MODE value, expected live or ci_snapshot, got: {raw}"
            );
            std::process::exit(2);
        }
    }
}

fn select_probe_mode(raw: Option<String>) -> Result<String, String> {
    let mode = raw.unwrap_or_else(|| MODE_LIVE.to_string());
    match mode.as_str() {
        MODE_LIVE | MODE_CI_SNAPSHOT => Ok(mode),
        _ => Err(mode),
    }
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

fn run_curl_plain(url: &str, timeout_sec: u64) -> bool {
    let mut cmd = Command::new("curl");
    cmd.arg("--disable")
        .arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--noproxy")
        .arg("*")
        .arg("--max-time")
        .arg(timeout_sec.to_string())
        .arg("--output")
        .arg("/dev/null")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for key in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "NO_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
        "no_proxy",
    ] {
        cmd.env_remove(key);
    }
    cmd.arg(url).status().map(|s| s.success()).unwrap_or(false)
}

fn parse_datapath_targets(csv: &str) -> Vec<String> {
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

fn resolve_cli_bin(file_cfg: &std::collections::BTreeMap<String, String>) -> Option<String> {
    let mut candidates = Vec::new();
    if let Some(value) = resolve_non_empty_setting("CHIMERA_REAL_WORLD_CLI", file_cfg) {
        candidates.push(value);
    }
    if let Some(value) = resolve_non_empty_setting("CHIMERA_CLI_BIN", file_cfg) {
        candidates.push(value);
    }
    candidates.push("bin/chimera-cli".to_string());
    candidates.push("target/debug/chimera-cli".to_string());
    for candidate in candidates {
        let path_candidate = Path::new(&candidate);
        if candidate.contains('/') {
            if is_executable_file(path_candidate) {
                return Some(candidate);
            }
        } else if command_exists(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn resolve_state_path(file_cfg: &std::collections::BTreeMap<String, String>) -> String {
    resolve_non_empty_setting("CHIMERA_REAL_WORLD_STATE_FILE", file_cfg)
        .unwrap_or_else(|| DEFAULT_STATE_PATH.to_string())
}

fn strict_flow_proof_ok(file_cfg: &std::collections::BTreeMap<String, String>) -> bool {
    let Some(cli_bin) = resolve_cli_bin(file_cfg) else {
        return false;
    };
    let state_path = resolve_state_path(file_cfg);
    let max_flow_age_sec = parse_u64_setting_with_min(
        "CHIMERA_REAL_WORLD_FLOW_MAX_AGE_SEC",
        file_cfg,
        DEFAULT_FLOW_MAX_AGE_SEC,
        1,
    );
    let output = Command::new(cli_bin)
        .arg("state")
        .arg("proof")
        .arg("--state-file")
        .arg(state_path)
        .arg("--require-flow")
        .arg("true")
        .arg("--max-flow-age-sec")
        .arg(max_flow_age_sec.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();
    let Ok(output) = output else {
        return false;
    };
    output.status.success() && strict_flow_proof_stdout_ok(&output.stdout)
}

fn strict_flow_proof_stdout_ok(stdout: &[u8]) -> bool {
    std::str::from_utf8(stdout)
        .ok()
        .map(|text| text.lines().any(|line| line.trim() == "datapath_proof=ok"))
        .unwrap_or(false)
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
fn format_datapath_targets_csv(values: &[String]) -> String {
    values.join(",")
}

fn target_ref(index_zero_based: usize) -> String {
    format!("target#{}", index_zero_based + 1)
}

fn format_target_refs_csv(count: usize) -> String {
    (0..count)
        .map(target_ref)
        .collect::<Vec<String>>()
        .join(",")
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
        DEFAULT_STATE_PATH, extract_authority, format_datapath_targets_csv, format_target_refs_csv,
        is_supported_probe_url, parse_datapath_targets, resolve_cli_bin, resolve_non_empty_setting,
        resolve_state_path, select_probe_mode, strict_flow_proof_ok, strict_flow_proof_stdout_ok,
        target_ref,
    };
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    #[cfg(unix)]
    use std::time::{SystemTime, UNIX_EPOCH};

    fn probe_url(scheme: &str, authority: &str) -> String {
        format!("{scheme}://{authority}")
    }

    #[test]
    fn extract_authority_stops_on_path_query_and_fragment() {
        assert_eq!(extract_authority("host:1234/path"), "host:1234");
        assert_eq!(extract_authority("host:1234?x=1"), "host:1234");
        assert_eq!(extract_authority("host:1234#frag"), "host:1234");
        assert_eq!(extract_authority("host:1234/path?x=1#frag"), "host:1234");
    }

    #[test]
    fn parse_datapath_targets_dedups_case_insensitive_and_trims() {
        let first = probe_url("https", "example.org");
        let duplicate = probe_url("HTTPS", "EXAMPLE.ORG");
        let second = probe_url("https", "second.example");
        let raw = format!(" {first} ,{duplicate},{second} ");
        assert_eq!(
            parse_datapath_targets(&raw),
            vec![first.to_string(), second.to_string()]
        );
    }

    #[test]
    fn format_datapath_targets_csv_preserves_normalized_order() {
        let values = vec![
            probe_url("https", "one.example"),
            probe_url("https", "two.example"),
        ];
        let expected = format!("{},{}", values[0], values[1]);
        assert_eq!(format_datapath_targets_csv(&values), expected);
    }

    #[test]
    fn redacted_target_refs_are_stable_and_one_based() {
        assert_eq!(target_ref(0), "target#1");
        assert_eq!(target_ref(2), "target#3");
        assert_eq!(format_target_refs_csv(3), "target#1,target#2,target#3");
        assert_eq!(format_target_refs_csv(0), "");
    }

    #[test]
    fn resolve_non_empty_setting_trims_and_rejects_empty() {
        let mut cfg = BTreeMap::new();
        cfg.insert("A".to_string(), "  value  ".to_string());
        cfg.insert("B".to_string(), "   ".to_string());
        assert_eq!(
            resolve_non_empty_setting("A", &cfg),
            Some("value".to_string())
        );
        assert_eq!(resolve_non_empty_setting("B", &cfg), None);
        assert_eq!(resolve_non_empty_setting("C", &cfg), None);
    }

    #[test]
    fn resolve_probe_mode_defaults_to_live() {
        assert_eq!(select_probe_mode(None), Ok("live".to_string()));
    }

    #[test]
    fn resolve_probe_mode_accepts_ci_snapshot() {
        assert_eq!(
            select_probe_mode(Some("ci_snapshot".to_string())),
            Ok("ci_snapshot".to_string())
        );
    }

    #[test]
    fn supported_probe_url_requires_http_or_https() {
        assert!(is_supported_probe_url(&probe_url(
            "https",
            "target.example"
        )));
        assert!(is_supported_probe_url(&probe_url("http", "target.example")));
        assert!(is_supported_probe_url(&probe_url(
            "HTTPS",
            "target.example"
        )));
        assert!(!is_supported_probe_url(&probe_url(
            "h*ttps",
            "target.example"
        )));
        assert!(!is_supported_probe_url(&probe_url("https", "[]")));
        assert!(!is_supported_probe_url(&format!("{}://?q=1", "https")));
        let ws_target = format!("{}://{}", "ws", "example.invalid");
        let wss_target = format!("{}://{}", "wss", "example.invalid");
        assert!(!is_supported_probe_url(&ws_target));
        assert!(!is_supported_probe_url(&wss_target));
        assert!(!is_supported_probe_url(&probe_url("https", " ")));
        assert!(!is_supported_probe_url(&probe_url("https", "bad host")));
        assert!(!is_supported_probe_url("target.example"));
    }

    #[test]
    fn resolve_state_path_defaults_to_runtime_state_file() {
        let cfg = BTreeMap::new();
        assert_eq!(resolve_state_path(&cfg), DEFAULT_STATE_PATH);
    }

    #[test]
    fn resolve_state_path_prefers_config_value() {
        let mut cfg = BTreeMap::new();
        cfg.insert(
            "CHIMERA_REAL_WORLD_STATE_FILE".to_string(),
            "docs/custom_state.json".to_string(),
        );
        assert_eq!(resolve_state_path(&cfg), "docs/custom_state.json");
    }

    #[test]
    fn strict_flow_proof_stdout_requires_exact_ok_marker() {
        assert!(strict_flow_proof_stdout_ok(b"datapath_proof=ok\n"));
        assert!(strict_flow_proof_stdout_ok(
            b"other=value\ndatapath_proof=ok\n"
        ));
        assert!(!strict_flow_proof_stdout_ok(
            b"datapath_proof=missing_flow_proof\n"
        ));
    }

    #[cfg(unix)]
    fn temp_test_dir(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "runtime_real_world_probe_{name}_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    #[cfg(unix)]
    fn write_executable_script(
        path: &PathBuf,
        body: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(path, body)?;
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)?;
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn resolve_cli_bin_accepts_explicit_executable_path() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = temp_test_dir("resolve_cli_bin")?;
        let cli_path = dir.join("chimera-cli");
        write_executable_script(&cli_path, "#!/bin/sh\nexit 0\n")?;
        let mut cfg = BTreeMap::new();
        cfg.insert(
            "CHIMERA_REAL_WORLD_CLI".to_string(),
            cli_path.display().to_string(),
        );
        assert_eq!(resolve_cli_bin(&cfg), Some(cli_path.display().to_string()));
        fs::remove_dir_all(dir)?;
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn strict_flow_proof_ok_uses_cli_contract() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_test_dir("strict_flow_proof_ok")?;
        let cli_path = dir.join("chimera-cli");
        let state_path = dir.join("runtime_state.json");
        fs::write(&state_path, "{}")?;
        write_executable_script(
            &cli_path,
            "#!/bin/sh\nprintf 'datapath_proof=ok\\n'\nexit 0\n",
        )?;
        let mut cfg = BTreeMap::new();
        cfg.insert(
            "CHIMERA_REAL_WORLD_CLI".to_string(),
            cli_path.display().to_string(),
        );
        cfg.insert(
            "CHIMERA_REAL_WORLD_STATE_FILE".to_string(),
            state_path.display().to_string(),
        );
        cfg.insert(
            "CHIMERA_REAL_WORLD_FLOW_MAX_AGE_SEC".to_string(),
            "120".to_string(),
        );
        assert!(strict_flow_proof_ok(&cfg));
        fs::remove_dir_all(dir)?;
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn strict_flow_proof_ok_rejects_non_ok_cli_result() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_test_dir("strict_flow_proof_fail")?;
        let cli_path = dir.join("chimera-cli");
        let state_path = dir.join("runtime_state.json");
        fs::write(&state_path, "{}")?;
        write_executable_script(
            &cli_path,
            "#!/bin/sh\nprintf 'datapath_proof=missing_flow_proof\\n'\nexit 1\n",
        )?;
        let mut cfg = BTreeMap::new();
        cfg.insert(
            "CHIMERA_REAL_WORLD_CLI".to_string(),
            cli_path.display().to_string(),
        );
        cfg.insert(
            "CHIMERA_REAL_WORLD_STATE_FILE".to_string(),
            state_path.display().to_string(),
        );
        assert!(!strict_flow_proof_ok(&cfg));
        fs::remove_dir_all(dir)?;
        Ok(())
    }
}
