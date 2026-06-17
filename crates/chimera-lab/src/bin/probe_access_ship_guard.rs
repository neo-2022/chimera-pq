#![forbid(unsafe_code)]

use serde_json::Value;
use std::env;
use std::fs;

#[path = "probe_access_ship_guard/contract.rs"]
mod probe_access_contract;

use probe_access_contract::{url_has_host, validate_probe_contract};

const CI_SNAPSHOT_HOST: &str = "chimera-ci-snapshot.local";

fn main() {
    let args: Vec<String> = env::args().collect();
    let ship_json = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("docs/SHIP_READINESS_REPORT.json");
    let probe_json = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("docs/probe_access_latest.json");
    let ship = read_obj(ship_json, "ship");
    let probe = read_obj(probe_json, "probe access");

    require_bool(&ship, "runtime_probe_access_smoke_ok", true);
    validate_probe_contract(&probe, CI_SNAPSHOT_HOST).unwrap_or_else(|msg| fail(&msg));
    let mode = get_str(&ship, "runtime_probe_access_mode");
    if !["live", "ci_snapshot"].contains(&mode) {
        fail("probe access ship guard: invalid runtime_probe_access_mode");
    }

    let probe_mode = infer_probe_mode(&probe);
    if mode != probe_mode {
        fail("probe access ship guard: probe access mode mismatch");
    }
    validate_mode_flags(&ship, &probe, mode).unwrap_or_else(|msg| fail(&msg));
    println!("probe access ship guard: PASS");
}

fn read_obj(path: &str, label: &str) -> serde_json::Map<String, Value> {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|_| fail(&format!("probe access ship guard: missing {label}: {path}")));
    let value: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|_| fail(&format!("probe access ship guard: invalid {label} json")));
    value
        .as_object()
        .cloned()
        .unwrap_or_else(|| fail(&format!("probe access ship guard: {label} root not object")))
}

fn validate_mode_flags(
    ship: &serde_json::Map<String, Value>,
    probe: &serde_json::Map<String, Value>,
    mode: &str,
) -> Result<(), String> {
    let live_external = get_bool(ship, "runtime_probe_access_live_external_probe")?;
    let ssh_stand_required = get_bool(
        ship,
        "runtime_probe_access_ssh_stand_required_for_live_probe",
    )?;
    let ci_snapshot_targets_ok = get_bool(ship, "runtime_probe_access_ci_snapshot_targets_ok")?;
    match mode {
        "live" => {
            if !live_external || ssh_stand_required || ci_snapshot_targets_ok {
                return Err("probe access ship guard: live probe access flag mismatch".to_string());
            }
        }
        "ci_snapshot" => {
            if live_external || !ssh_stand_required || !ci_snapshot_targets_ok {
                return Err(
                    "probe access ship guard: ci_snapshot probe access requires snapshot-safe targets"
                        .to_string(),
                );
            }
            if !targets_all_match_host(probe, CI_SNAPSHOT_HOST) {
                return Err(
                    "probe access ship guard: ci_snapshot probe access target mismatch".to_string(),
                );
            }
        }
        _ => return Err("probe access ship guard: invalid runtime_probe_access_mode".to_string()),
    }
    Ok(())
}

fn infer_probe_mode(probe: &serde_json::Map<String, Value>) -> &'static str {
    if targets_all_match_host_obj(probe, CI_SNAPSHOT_HOST) {
        "ci_snapshot"
    } else {
        "live"
    }
}

fn targets_all_match_host_obj(probe: &serde_json::Map<String, Value>, host: &str) -> bool {
    probe
        .get("targets")
        .and_then(Value::as_array)
        .map(|targets| {
            !targets.is_empty()
                && targets.iter().all(|target| {
                    target
                        .get("url")
                        .and_then(Value::as_str)
                        .is_some_and(|url| url_has_host(url, host))
                })
        })
        .unwrap_or(false)
}

fn targets_all_match_host(probe: &serde_json::Map<String, Value>, host: &str) -> bool {
    targets_all_match_host_obj(probe, host)
}

fn get_str<'a>(obj: &'a serde_json::Map<String, Value>, key: &str) -> &'a str {
    obj.get(key).and_then(Value::as_str).unwrap_or("")
}

fn get_bool(obj: &serde_json::Map<String, Value>, key: &str) -> Result<bool, String> {
    obj.get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("probe access ship guard: invalid bool field: {key}"))
}

fn require_bool(obj: &serde_json::Map<String, Value>, key: &str, expected: bool) {
    if obj.get(key).and_then(Value::as_bool) != Some(expected) {
        fail(&format!("probe access ship guard: {key} mismatch"));
    }
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::{infer_probe_mode, validate_mode_flags};
    use serde_json::{Map, Value, json};

    fn probe_with(url: &str) -> Map<String, Value> {
        let mut probe = Map::new();
        probe.insert("targets".to_string(), json!([{ "url": url }]));
        probe
    }

    fn ship_flags(live: bool, ssh_required: bool, snapshot_targets: bool) -> Map<String, Value> {
        let mut ship = Map::new();
        ship.insert(
            "runtime_probe_access_live_external_probe".to_string(),
            json!(live),
        );
        ship.insert(
            "runtime_probe_access_ssh_stand_required_for_live_probe".to_string(),
            json!(ssh_required),
        );
        ship.insert(
            "runtime_probe_access_ci_snapshot_targets_ok".to_string(),
            json!(snapshot_targets),
        );
        ship
    }

    #[test]
    fn infers_ci_snapshot_from_snapshot_host() {
        let probe = probe_with("https://chimera-ci-snapshot.local/ok");
        assert_eq!(infer_probe_mode(&probe), "ci_snapshot");
    }

    #[test]
    fn accepts_live_contract() {
        let ship = ship_flags(true, false, false);
        let probe = probe_with("https://example.org");
        assert!(validate_mode_flags(&ship, &probe, "live").is_ok());
    }

    #[test]
    fn accepts_ci_snapshot_contract() {
        let ship = ship_flags(false, true, true);
        let probe = probe_with("https://chimera-ci-snapshot.local/ok");
        assert!(validate_mode_flags(&ship, &probe, "ci_snapshot").is_ok());
    }

    #[test]
    fn rejects_ci_snapshot_without_safe_targets() {
        let ship = ship_flags(false, true, false);
        let probe = probe_with("https://chimera-ci-snapshot.local/ok");
        let res = validate_mode_flags(&ship, &probe, "ci_snapshot");
        assert!(res.is_err());
        assert!(
            res.err()
                .is_some_and(|e| e.contains("snapshot-safe targets"))
        );
    }

    #[test]
    fn rejects_ci_snapshot_with_external_targets() {
        let ship = ship_flags(false, true, true);
        let probe = probe_with("https://example.org");
        let res = validate_mode_flags(&ship, &probe, "ci_snapshot");
        assert!(res.is_err());
        assert!(res.err().is_some_and(|e| e.contains("target mismatch")));
    }
}
