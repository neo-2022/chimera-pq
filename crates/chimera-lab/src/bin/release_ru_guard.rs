#![forbid(unsafe_code)]

use serde_json::Value;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let release_json = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("docs/RELEASE_READINESS_REPORT.json");
    let release_ru_md = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("docs/RELEASE_READINESS_REPORT_RU.md");
    let reality_json = args
        .get(3)
        .map(String::as_str)
        .unwrap_or("docs/REALITY_AUDIT_LATEST.json");

    let release = read_obj(release_json);
    let md = read_text(release_ru_md);
    let reality = read_obj(reality_json);

    require_str(&release, "status", "ok");
    require_str(&release, "kind", "release_readiness_report");
    require_bool(&release, "release_ok", true);
    require_str(&release, "network_state", "not_modified");
    let gate = get_obj(
        &release,
        "release_gate",
        "release ru guard: missing release_gate",
    );
    require_bool(gate, "runtime_forced_stop_rollback_verified", true);

    let truth_expected = serde_json::json!({
        "lab_scope_only": true,
        "real_world_datapath_closed": reality
            .get("real_world_datapath_closed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    });
    if release.get("truth_boundary") != Some(&truth_expected) {
        fail("release ru guard: truth boundary mismatch");
    }

    for line in [
        "# Отчет Готовности Релиза",
        "Статус: **PASS**",
        "Release gate (раздел 11 спеки):",
        "Этапы:",
        "Артефакты:",
        "Граница истины:",
        "Безопасность сети: в этом отчете мы не меняем маршруты/DNS/firewall/proxy ОС.",
    ] {
        require_line_once(&md, line);
    }
    require_contains(&md, "MVP готов только к расширенным лабораторным тестам");
    require_contains(&md, "не означает закрытие real-world datapath");
    require_line_once(&md, "- Контур lab/proof/report: `true`");
    require_line_once(
        &md,
        &format!(
            "- Real OS-level datapath closure (strict M4/M5): `{}`",
            if truth_expected["real_world_datapath_closed"] == Value::Bool(true) {
                "true"
            } else {
                "false"
            }
        ),
    );

    let line_gate_header = line_no(&md, "Release gate (раздел 11 спеки):");
    let line_first_gate = line_no(&md, "- Чистая копия репозитория собирается: `true`");
    let line_last_gate = line_no(
        &md,
        "- Runtime rollback после forced-stop подтвержден: `true`",
    );
    if !(line_gate_header < line_first_gate && line_first_gate < line_last_gate) {
        fail("release ru guard: release gate order mismatch");
    }

    let runtime_items = [
        "- Runtime DNS apply подтвержден: `true`",
        "- Runtime route apply подтвержден: `true`",
        "- Runtime route-policy validation подтвержден: `true`",
        "- Runtime TUN-name validation подтвержден: `true`",
        "- Runtime rollback после forced-stop подтвержден: `true`",
    ];
    strict_order(
        &md,
        &runtime_items,
        "release ru guard: runtime order mismatch",
    );

    for m in 0..=6 {
        let pref = format!("- M{m} ");
        let matches = md
            .lines()
            .filter(|l| l.starts_with(&pref) && l.ends_with(": `true`"))
            .count();
        if matches != 1 {
            fail("release ru guard: milestone lines mismatch");
        }
    }

    let artifacts = [
        "- Отчет артефактов M5: `true` (`docs/M5_ARTIFACTS_REPORT.md`)",
        "- Отчет артефактов M6: `true` (`docs/M6_ARTIFACTS_REPORT.md`)",
        "- Артефакт benchmark: `true` (`docs/benchmark_latest.json`)",
        "- CEF phase1 smoke: `true` (`docs/CEF_PHASE1_SMOKE.json`)",
        "- Mesh route explain: `true` (`docs/MESH_ROUTE_EXPLAIN.json`)",
    ];
    strict_order(
        &md,
        &artifacts,
        "release ru guard: artifacts order mismatch",
    );

    println!("release ru guard: PASS");
}

fn read_text(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|_| fail(&format!("release ru guard: missing file: {path}")))
}

fn read_obj(path: &str) -> serde_json::Map<String, Value> {
    let raw = read_text(path);
    let v: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|_| fail(&format!("release ru guard: invalid json: {path}")));
    v.as_object()
        .cloned()
        .unwrap_or_else(|| fail(&format!("release ru guard: root not object: {path}")))
}

fn require_str(obj: &serde_json::Map<String, Value>, key: &str, expected: &str) {
    if obj.get(key).and_then(Value::as_str) != Some(expected) {
        fail(&format!("release ru guard: {key} mismatch"));
    }
}
fn require_bool(obj: &serde_json::Map<String, Value>, key: &str, expected: bool) {
    if obj.get(key).and_then(Value::as_bool) != Some(expected) {
        fail(&format!("release ru guard: {key} mismatch"));
    }
}
fn get_obj<'a>(
    root: &'a serde_json::Map<String, Value>,
    key: &str,
    err: &str,
) -> &'a serde_json::Map<String, Value> {
    root.get(key)
        .and_then(Value::as_object)
        .unwrap_or_else(|| fail(err))
}
fn require_line_once(md: &str, line: &str) {
    if md.lines().filter(|l| *l == line).count() != 1 {
        fail(&format!("release ru guard: markdown line mismatch: {line}"));
    }
}
fn require_contains(md: &str, needle: &str) {
    if !md.contains(needle) {
        fail(&format!("release ru guard: markdown missing: {needle}"));
    }
}
fn line_no(md: &str, expected: &str) -> usize {
    md.lines()
        .enumerate()
        .find_map(|(i, l)| if l == expected { Some(i + 1) } else { None })
        .unwrap_or_else(|| {
            fail(&format!(
                "release ru guard: missing markdown line: {expected}"
            ))
        })
}
fn strict_order(md: &str, items: &[&str], err: &str) {
    let mut prev = 0usize;
    for item in items {
        let ln = line_no(md, item);
        if ln <= prev {
            fail(err);
        }
        prev = ln;
    }
}
fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}
