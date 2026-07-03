#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

const ACTIVE_TEXT_FILES: &[&str] = &[
    "README.md",
    "justfile",
    "docs/OPERATIONS.md",
    "docs/ROUTING.md",
    "docs/RELEASE_READINESS_REPORT.md",
    "docs/RELEASE_READINESS_REPORT_RU.md",
    "docs/REPORT_PACK.md",
    "configs/mesh-node.example.conf",
    "configs/mesh_bootstrap.env.example",
    "crates/chimera-cli/src/main.rs",
    "crates/chimera-lab/src/main.rs",
    "scripts/build_release.sh",
    "scripts/chimera-control.sh",
    "scripts/chimera-runner.sh",
    "scripts/chimera_doctor_contract_smoke.sh",
    "scripts/chimera_start_contract_smoke.sh",
    "scripts/chimera_stop_contract_smoke.sh",
    "scripts/chimera_installer_gate.sh",
    "scripts/install_desktop_control.sh",
    "scripts/release_bundle_install_contract_smoke.sh",
    "scripts/ship_readiness.sh",
    ".github/workflows/release.yml",
];

const ACTIVE_JSON_REPORTS: &[&str] = &[
    "docs/RELEASE_READINESS_REPORT.json",
    "docs/REPORT_PACK.json",
    "docs/MVP_SNAPSHOT.json",
    "docs/release_readiness_audit.json",
];

const SCANNED_JSON_REPORTS: &[&str] = &[
    "docs/SHIP_READINESS_REPORT.json",
    "docs/MVP_VERIFY.json",
    "docs/REALITY_AUDIT_LATEST.json",
];

const FORBIDDEN_PRODUCT_PHRASES: &[&str] = &[
    "gateway/proxy path",
    "proxy path",
    "VPN path",
    "VPN product",
    "VPN-like",
    "as a VPN",
    "as VPN",
    "как VPN",
    "как прокси",
    "прокси-сервис",
    "VPN как продукт",
    "VPN как продукт для обычных приложений",
    "VPN для обычных приложений",
    "VPN для приложений",
    "обычные приложения через VPN",
    "обычные приложения через proxy",
    "обычные приложения через прокси",
    "normal applications through VPN",
    "ordinary applications through VPN",
    "normal applications through proxy",
    "ordinary applications through proxy",
    "manual proxy setup",
    "client/gateway workflow",
    "gateway mode is normal",
    "ordinary VPN product",
    "ordinary proxy product",
    "обычный VPN-продукт",
    "обычный proxy-продукт",
];

const NORMAL_PATH_MARKERS: &[&str] = &[
    "normal product path",
    "default product path",
    "canonical path",
    "release path",
    "mvp path",
    "ordinary application workflow",
    "normal app workflow",
    "browser workflow",
    "ide workflow",
    "нормальный продуктовый путь",
    "продуктовый путь",
    "нормальный путь",
    "обычный путь",
    "штатный путь",
];

const BAD_NORMAL_PATH_MARKERS: &[&str] = &[
    "vpn mode",
    "vpn service",
    "proxy mode",
    "proxy workflow",
    "local proxy",
    "socks",
    "per-app proxy",
    "app-specific proxy",
    "--proxy-server",
    "manual proxy toggle",
    "app relaunch",
    "restart browser",
    "mutate app profile",
    "workaround path",
    "sing-box",
    "third-party runtime bootstrap",
    "vpn-режим",
    "прокси-режим",
    "ручная настройка прокси",
    "перезапуск браузера",
];

const DENIAL_MARKERS: &[&str] = &[
    "must not",
    "do not",
    "does not",
    "cannot",
    "must never",
    "forbidden",
    "prohibited",
    "reject",
    "invalid",
    "fail-closed",
    "not use",
    "not rely",
    "not require",
    "не должен",
    "не должна",
    "не должно",
    "не использовать",
    "не опираться",
    "не требуется",
    "нельзя",
    "запрещ",
    "отклон",
    "недопуст",
];

const PROXY_DENIAL_MARKERS: &[&str] = &[
    "without proxy",
    "without `--proxy",
    "without --proxy",
    "no proxy",
    "no app proxy",
    "no per-app proxy",
    "без proxy",
    "без прокси",
];

const FORBIDDEN_RELEASE_EVIDENCE_MARKERS: &[&str] = &[
    "selected_proxy",
    "proxy_url",
    "chimera_proxy_listener",
    "CHIMERA_PATH_PROOF_PROD",
    "PROBE_ACCESS_FOREIGN_FOCUS",
    "CHIMERA_BROWSER_PARALLEL_SOAK",
];

const HARD_HISTORICAL_PROXY_MARKERS: &[&str] = &[
    "selected_proxy",
    "proxy_url",
    "socks5h://",
    "chimera_proxy_listener",
];

const QUARANTINED_HISTORICAL_ARTIFACTS: &[&str] = &[
    "docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md",
    "docs/CHIMERA_PATH_PROOF_PROD.json",
    "docs/CHIMERA_PATH_PROOF_PROD_AFTER_FIX.json",
    "docs/CHIMERA_PATH_PROOF_WITH_AISTUDIO_RUN.json",
    "docs/PROBE_ACCESS_FOREIGN_FOCUS_PROD.json",
    "docs/PROBE_ACCESS_FOREIGN_FOCUS_PROD_AFTER_FIX.json",
    "docs/PROBE_ACCESS_FOREIGN_FOCUS_WITH_AISTUDIO.json",
    "docs/PROBE_ACCESS_FOREIGN_FOCUS_WITH_AISTUDIO_RERUN.json",
    "docs/CHIMERA_BROWSER_PARALLEL_SOAK_5M.json",
    "docs/CHIMERA_BROWSER_PARALLEL_SOAK_5M_STABILIZED2.json",
    "docs/load/CHIMERA_LOAD_60S_SIDE_B_20260522_144925.json",
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        for path in args {
            let text = read(&path);
            reject_forbidden_product_phrases(&path, &text);
            reject_contextual_normal_path_workaround(&path, &text);
            reject_forbidden_release_evidence_markers(&path, &text);
            reject_normal_path_legacy_markers(&path, &text);
        }
        println!("product language guard: PASS");
        return;
    }

    require_file("deploy/systemd-user/chimera-node.service");
    require_file("deploy/systemd-user/chimera-datapath.service");
    forbid_file("deploy/systemd-user/chimera-client.service");
    forbid_file("deploy/systemd-user/chimera-gateway.service");

    for path in ACTIVE_TEXT_FILES {
        let text = read(path);
        reject_forbidden_product_phrases(path, &text);
        reject_contextual_normal_path_workaround(path, &text);
        reject_normal_path_legacy_markers(path, &text);
    }

    for path in ACTIVE_JSON_REPORTS {
        let text = read(path);
        require_contains(path, &text, "mesh_node_runs_linux");
        require_contains(path, &text, "policy_routing_direct_peer_transit_block");
        reject_contains(path, &text, "client_gateway_run_linux");
        reject_contains(path, &text, "policy_routing_direct_gateway_block");
        reject_contextual_normal_path_workaround(path, &text);
        reject_forbidden_release_evidence_markers(path, &text);
    }

    for path in SCANNED_JSON_REPORTS {
        let text = read(path);
        reject_forbidden_product_phrases(path, &text);
        reject_contextual_normal_path_workaround(path, &text);
        reject_forbidden_release_evidence_markers(path, &text);
    }

    validate_normal_path_contracts();

    let quarantine = read("docs/HISTORICAL_PROXY_ARTIFACTS_NOT_RELEASE_EVIDENCE.md");
    require_contains(
        "docs/HISTORICAL_PROXY_ARTIFACTS_NOT_RELEASE_EVIDENCE.md",
        &quarantine,
        "must not be used as",
    );
    for artifact in QUARANTINED_HISTORICAL_ARTIFACTS {
        require_contains(
            "docs/HISTORICAL_PROXY_ARTIFACTS_NOT_RELEASE_EVIDENCE.md",
            &quarantine,
            artifact,
        );
    }
    validate_historical_proxy_artifact_quarantine(&quarantine);

    let first_launch = read("docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md");
    require_contains(
        "docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md",
        &first_launch,
        "Release evidence status: invalid_for_release",
    );
    require_contains(
        "docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md",
        &first_launch,
        "Evidence class: historical_proxy_evidence_only",
    );

    println!("product language guard: PASS");
}

fn read(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|_| fail(&format!("product language guard: missing file: {path}")))
}

fn require_file(path: &str) {
    if !Path::new(path).is_file() {
        fail(&format!(
            "product language guard: required file missing: {path}"
        ));
    }
}

fn forbid_file(path: &str) {
    if Path::new(path).exists() {
        fail(&format!(
            "product language guard: forbidden legacy product file present: {path}"
        ));
    }
}

fn reject_forbidden_product_phrases(path: &str, text: &str) {
    let lower = text.to_lowercase();
    for phrase in FORBIDDEN_PRODUCT_PHRASES {
        if lower.contains(&phrase.to_lowercase()) {
            fail(&format!(
                "product language guard: forbidden VPN/proxy product wording in {path}: {phrase}"
            ));
        }
    }
}

fn reject_forbidden_release_evidence_markers(path: &str, text: &str) {
    for marker in FORBIDDEN_RELEASE_EVIDENCE_MARKERS {
        reject_contains(path, text, marker);
    }
}

fn reject_contextual_normal_path_workaround(path: &str, text: &str) {
    for (index, line) in text.lines().enumerate() {
        let lower = line.to_lowercase();
        let normal_marker = NORMAL_PATH_MARKERS
            .iter()
            .find(|marker| lower.contains(**marker));
        let bad_marker = BAD_NORMAL_PATH_MARKERS
            .iter()
            .find(|marker| lower.contains(**marker));
        let (Some(normal_marker), Some(bad_marker)) = (normal_marker, bad_marker) else {
            continue;
        };
        if contextual_denial_allows(&lower, bad_marker) {
            continue;
        }
        fail(&format!(
            "product language guard: normal product path is tied to VPN/proxy/workaround in {path}:{}: {normal_marker} + {bad_marker}",
            index + 1
        ));
    }
}

fn contextual_denial_allows(line: &str, bad_marker: &str) -> bool {
    if DENIAL_MARKERS.iter().any(|marker| line.contains(marker)) {
        return true;
    }
    is_proxy_marker(bad_marker)
        && PROXY_DENIAL_MARKERS
            .iter()
            .any(|marker| line.contains(marker))
}

fn is_proxy_marker(marker: &str) -> bool {
    marker.contains("proxy")
        || marker.contains("socks")
        || marker.contains("--proxy")
        || marker.contains("прокси")
}

fn reject_normal_path_legacy_markers(path: &str, text: &str) {
    if path.contains("tests/fixtures/product_language_guard/fail/") {
        for marker in [
            "CLIENT_CONFIG_FILE",
            "configs/client.conf",
            "configs/client.example.conf",
            "configs/gateway.example.conf",
            "chimera-release/bin/chimera-gateway",
            "client_config_ready",
            "ensure_upstream_env_bootstrapped",
            "UPSTREAM_AUTOBOOTSTRAP_SCRIPT",
            "RUNTIME_BOOTSTRAP_SCRIPT",
            "ensure-singbox",
            "CHIMERA_SINGBOX_",
            "sing-box",
            "chimera_runtime_bootstrap.sh",
            "normal_app_path_uses_third_party_runtime",
        ] {
            reject_contains(path, text, marker);
        }
    }
}

fn validate_historical_proxy_artifact_quarantine(quarantine: &str) {
    let mut paths = Vec::new();
    collect_files(Path::new("docs"), &mut paths);
    for path in paths {
        let path_str = path.to_string_lossy().replace('\\', "/");
        if path_str == "docs/HISTORICAL_PROXY_ARTIFACTS_NOT_RELEASE_EVIDENCE.md" {
            continue;
        }
        let text = read(&path_str);
        if HARD_HISTORICAL_PROXY_MARKERS
            .iter()
            .any(|marker| text.contains(marker))
            && !quarantine.contains(&path_str)
        {
            fail(&format!(
                "product language guard: historical proxy artifact is not quarantined: {path_str}"
            ));
        }
    }
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|_| {
        fail(&format!(
            "product language guard: cannot read dir: {}",
            dir.display()
        ))
    });
    for entry in entries {
        let path = entry
            .unwrap_or_else(|_| fail("product language guard: cannot read dir entry"))
            .path();
        if path.is_dir() {
            collect_files(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

fn validate_normal_path_contracts() {
    let control = read("scripts/chimera-control.sh");
    require_contains("scripts/chimera-control.sh", &control, "NODE_CONFIG_FILE");
    require_contains("scripts/chimera-control.sh", &control, "node_config_ready");
    require_contains(
        "scripts/chimera-control.sh",
        &control,
        "ensure_mesh_bootstrap_env",
    );
    reject_contains("scripts/chimera-control.sh", &control, "CLIENT_CONFIG_FILE");
    reject_contains(
        "scripts/chimera-control.sh",
        &control,
        "client_config_ready",
    );
    reject_contains(
        "scripts/chimera-control.sh",
        &control,
        "ensure_upstream_env_bootstrapped",
    );
    reject_contains(
        "scripts/chimera-control.sh",
        &control,
        "UPSTREAM_AUTOBOOTSTRAP_SCRIPT",
    );
    reject_contains(
        "scripts/chimera-control.sh",
        &control,
        "RUNTIME_BOOTSTRAP_SCRIPT",
    );
    reject_contains("scripts/chimera-control.sh", &control, "ensure-singbox");
    reject_contains("scripts/chimera-control.sh", &control, "SINGBOX_BIN");
    reject_contains("scripts/chimera-control.sh", &control, "singbox-split.json");
    reject_contains(
        "scripts/chimera-control.sh",
        &control,
        "chimera-singbox.pid",
    );
    require_contains(
        "scripts/chimera-control.sh",
        &control,
        "reason=datapath_unconfigured",
    );
    require_contains(
        "scripts/chimera-control.sh",
        &control,
        "legacy_lab_only_not_datapath_evidence",
    );
    require_contains(
        "scripts/chimera-control.sh",
        &control,
        "upstream_truth_boundary=legacy_lab_only_not_datapath_evidence",
    );
    require_contains(
        "scripts/chimera-control.sh",
        &control,
        "legacy_upstream_probe.env",
    );
    require_contains(
        "scripts/chimera-control.sh",
        &control,
        "LEGACY_NODE_COMPAT_SERVICE_UNIT",
    );
    require_contains(
        "scripts/chimera-control.sh",
        &control,
        "LEGACY_DATAPATH_COMPAT_SERVICE_UNIT",
    );
    require_contains(
        "scripts/chimera-control.sh",
        &control,
        "LEGACY_MANUAL_COMPAT_DOMAINS_FILE",
    );
    require_contains(
        "scripts/chimera-control.sh",
        &control,
        "legacy_manual_compat_domains_file",
    );
    require_contains(
        "scripts/chimera-control.sh",
        &control,
        "legacy_manual_compat_domains_count",
    );
    reject_contains(
        "scripts/chimera-control.sh",
        &control,
        "legacy_manual_gateway_domains_file",
    );
    reject_contains(
        "scripts/chimera-control.sh",
        &control,
        "legacy_manual_gateway_domains_count",
    );
    reject_contains(
        "scripts/chimera-control.sh",
        &control,
        "legacy_manual_gateway_domains:",
    );
    reject_contains(
        "scripts/chimera-control.sh",
        &control,
        "LEGACY_NODE_SERVICE_UNIT=",
    );
    reject_contains(
        "scripts/chimera-control.sh",
        &control,
        "LEGACY_DATAPATH_SERVICE_UNIT=",
    );
    reject_contains(
        "scripts/chimera-control.sh",
        &control,
        "MANUAL_GATEWAY_DOMAINS_FILE=",
    );
    reject_contains(
        "scripts/chimera-control.sh",
        &control,
        "endpoint=unconfigured",
    );
    reject_contains(
        "scripts/chimera-control.sh",
        &control,
        "verify_app_status=pass",
    );
    reject_contains(
        "scripts/chimera-control.sh",
        &control,
        "verify_cmd_status=pass",
    );
    reject_contains(
        "scripts/chimera-control.sh",
        &control,
        "service_route_enable_status=ok",
    );
    reject_contains(
        "scripts/chimera-control.sh",
        &control,
        "CHIMERA_UPSTREAM_NODE_ID",
    );
    reject_contains(
        "scripts/chimera-control.sh",
        &control,
        "upstream_failover_smoke=transparent_runtime",
    );

    let legacy_upstream_autobootstrap = read("scripts/chimera_upstream_autobootstrap.sh");
    require_contains(
        "scripts/chimera_upstream_autobootstrap.sh",
        &legacy_upstream_autobootstrap,
        "legacy_upstream_probe.env",
    );
    require_contains(
        "scripts/chimera_upstream_autobootstrap.sh",
        &legacy_upstream_autobootstrap,
        "legacy_upstream_pool.list",
    );
    require_contains(
        "scripts/chimera_upstream_autobootstrap.sh",
        &legacy_upstream_autobootstrap,
        "mode=legacy_probe_only",
    );
    require_contains(
        "scripts/chimera_upstream_autobootstrap.sh",
        &legacy_upstream_autobootstrap,
        "CHIMERA_UPSTREAM_ENDPOINTS_CSV",
    );
    reject_contains(
        "scripts/chimera_upstream_autobootstrap.sh",
        &legacy_upstream_autobootstrap,
        "printf 'CHIMERA_UPSTREAM_USER=%s",
    );
    reject_contains(
        "scripts/chimera_upstream_autobootstrap.sh",
        &legacy_upstream_autobootstrap,
        "printf 'CHIMERA_UPSTREAM_PASS=%s",
    );

    let inventory_bootstrap = read("crates/chimera-cli/src/mesh_cli/nodes_inventory/bootstrap.rs");
    require_contains(
        "crates/chimera-cli/src/mesh_cli/nodes_inventory/bootstrap.rs",
        &inventory_bootstrap,
        "mesh_bootstrap.env",
    );
    require_contains(
        "crates/chimera-cli/src/mesh_cli/nodes_inventory/bootstrap.rs",
        &inventory_bootstrap,
        "CHIMERA_MESH_REMOTE_ENDPOINT",
    );
    reject_contains(
        "crates/chimera-cli/src/mesh_cli/nodes_inventory/bootstrap.rs",
        &inventory_bootstrap,
        "upstream_proxy.env",
    );
    reject_contains(
        "crates/chimera-cli/src/mesh_cli/nodes_inventory/bootstrap.rs",
        &inventory_bootstrap,
        "last_upstream_endpoint",
    );

    let installer = read("scripts/install_desktop_control.sh");
    require_contains(
        "scripts/install_desktop_control.sh",
        &installer,
        "configs/mesh-node.conf",
    );
    require_contains(
        "scripts/install_desktop_control.sh",
        &installer,
        "configs/mesh-node.example.conf",
    );
    reject_contains(
        "scripts/install_desktop_control.sh",
        &installer,
        "configs/client.conf",
    );
    reject_contains(
        "scripts/install_desktop_control.sh",
        &installer,
        "configs/client.example.conf",
    );
    reject_contains(
        "scripts/install_desktop_control.sh",
        &installer,
        "LEGACY_UPSTREAM_ENV_FILE",
    );
    reject_contains(
        "scripts/install_desktop_control.sh",
        &installer,
        "chimera_runtime_bootstrap.sh",
    );

    for path in [
        "scripts/chimera_doctor_contract_smoke.sh",
        "scripts/chimera_start_contract_smoke.sh",
        "scripts/chimera_stop_contract_smoke.sh",
    ] {
        let text = read(path);
        require_contains(path, &text, "mesh-node.example.conf");
        reject_contains(path, &text, "CLIENT_CONFIG_FILE");
        reject_contains(path, &text, "client_config_ready");
        reject_contains(path, &text, "client.example.conf");
        reject_contains(path, &text, "gateway.example.conf");
    }
    for path in [
        "scripts/chimera_doctor_contract_smoke.sh",
        "scripts/chimera_start_contract_smoke.sh",
    ] {
        let text = read(path);
        require_contains(path, &text, "NODE_CONFIG_FILE");
    }

    let cli = read("crates/chimera-cli/src/main.rs");
    require_contains("crates/chimera-cli/src/main.rs", &cli, "node_config_file");
    require_contains("crates/chimera-cli/src/main.rs", &cli, "файл_node_config");
    require_contains(
        "crates/chimera-cli/src/main.rs",
        &cli,
        "configs/mesh-node.example.conf",
    );
    reject_contains(
        "crates/chimera-cli/src/main.rs",
        &cli,
        "[--config <client_config_file>]",
    );
    reject_contains(
        "crates/chimera-cli/src/main.rs",
        &cli,
        "[--config <файл_client_config>]",
    );
    reject_contains("crates/chimera-cli/src/main.rs", &cli, "client.toml");
    reject_contains("crates/chimera-cli/src/main.rs", &cli, "client.conf");
    reject_contains(
        "crates/chimera-cli/src/main.rs",
        &cli,
        "configs/client.example.conf",
    );
    require_contains("crates/chimera-cli/src/main.rs", &cli, "Node check: ok");
    require_contains("crates/chimera-cli/src/main.rs", &cli, "Проверка узла: ok");
    reject_contains("crates/chimera-cli/src/main.rs", &cli, "Client check:");
    reject_contains("crates/chimera-cli/src/main.rs", &cli, "Проверка клиента:");

    let lab = read("crates/chimera-lab/src/main.rs");
    require_contains("crates/chimera-lab/src/main.rs", &lab, "--node-config");
    require_contains(
        "crates/chimera-lab/src/main.rs",
        &lab,
        "configs/mesh-node.example.conf",
    );
    require_contains("crates/chimera-lab/src/main.rs", &lab, "node_config_ok");
    require_contains(
        "crates/chimera-lab/src/main.rs",
        &lab,
        "peer_ingress_config_ok",
    );
    reject_contains("crates/chimera-lab/src/main.rs", &lab, "--client");
    reject_contains("crates/chimera-lab/src/main.rs", &lab, "--gateway");
    reject_contains(
        "crates/chimera-lab/src/main.rs",
        &lab,
        "configs/client.example.conf",
    );
    reject_contains(
        "crates/chimera-lab/src/main.rs",
        &lab,
        "configs/gateway.example.conf",
    );

    let lab_doctor = read("docs/lab_doctor_latest.json");
    require_contains("docs/lab_doctor_latest.json", &lab_doctor, "node_config_ok");
    require_contains(
        "docs/lab_doctor_latest.json",
        &lab_doctor,
        "peer_ingress_config_ok",
    );
    reject_contains(
        "docs/lab_doctor_latest.json",
        &lab_doctor,
        "client_config_ok",
    );
    reject_contains(
        "docs/lab_doctor_latest.json",
        &lab_doctor,
        "gateway_config_ok",
    );

    let build_release = read("scripts/build_release.sh");
    require_contains(
        "scripts/build_release.sh",
        &build_release,
        "bin/chimera-node",
    );
    require_contains(
        "scripts/build_release.sh",
        &build_release,
        "build_bin chimera-gateway chimera-node",
    );
    require_contains(
        "scripts/build_release.sh",
        &build_release,
        "target/release/chimera-node",
    );
    require_contains(
        "scripts/build_release.sh",
        &build_release,
        "chimera-release/bin/chimera-node",
    );
    reject_contains(
        "scripts/build_release.sh",
        &build_release,
        "target/release/chimera-gateway",
    );
    reject_contains(
        "scripts/build_release.sh",
        &build_release,
        "cp -p \"${ROOT_DIR}/bin/chimera-gateway\"",
    );
    reject_contains(
        "scripts/build_release.sh",
        &build_release,
        "cp -p \"${ROOT_DIR}/scripts/chimera_runtime_bootstrap.sh\"",
    );
    require_contains(
        "scripts/build_release.sh",
        &build_release,
        "chimera-release/scripts/chimera_runtime_bootstrap",
    );

    let node_bin = read("crates/chimera-gateway/src/bin/chimera-node.rs");
    require_contains(
        "crates/chimera-gateway/src/bin/chimera-node.rs",
        &node_bin,
        "chimera-node commands",
    );
    require_contains(
        "crates/chimera-gateway/src/bin/chimera-node.rs",
        &node_bin,
        "Node doctor: ready for MVP checks",
    );
    require_contains(
        "crates/chimera-gateway/src/bin/chimera-node.rs",
        &node_bin,
        "peer_ingress",
    );
    reject_contains(
        "crates/chimera-gateway/src/bin/chimera-node.rs",
        &node_bin,
        "chimera-gateway",
    );
    reject_contains(
        "crates/chimera-gateway/src/bin/chimera-node.rs",
        &node_bin,
        "Gateway",
    );
    reject_contains(
        "crates/chimera-gateway/src/bin/chimera-node.rs",
        &node_bin,
        "gateway",
    );

    let release_smoke = read("scripts/release_bundle_install_contract_smoke.sh");
    require_contains(
        "scripts/release_bundle_install_contract_smoke.sh",
        &release_smoke,
        "chimera-release/bin/chimera-node",
    );
    require_contains(
        "scripts/release_bundle_install_contract_smoke.sh",
        &release_smoke,
        "installed_home/bin/chimera-node",
    );
    require_contains(
        "scripts/release_bundle_install_contract_smoke.sh",
        &release_smoke,
        "installed_legacy_third_party_runtime_bootstrap_present",
    );

    let release_workflow = read(".github/workflows/release.yml");
    require_contains(
        ".github/workflows/release.yml",
        &release_workflow,
        "chimera-release/bin/chimera-node",
    );

    let runner = read("scripts/chimera-runner.sh");
    require_contains(
        "scripts/chimera-runner.sh",
        &runner,
        "legacy target 'gateway' is retired; use target 'node'",
    );
    reject_contains(
        "scripts/chimera-runner.sh",
        &runner,
        "gateway)\n    run_with_fallback",
    );

    for path in [
        "configs/client.example.conf",
        "configs/gateway.example.conf",
    ] {
        let text = read(path);
        require_contains(path, &text, "Legacy compatibility fixture only.");
        require_contains(path, &text, "not a release config");
        reject_contains(path, &text, "MVP example");
    }

    let justfile = read("justfile");
    require_contains("justfile", &justfile, "node-doctor:");
    reject_contains("justfile", &justfile, "client-doctor:");
    reject_contains("justfile", &justfile, "gateway-doctor:");
    reject_contains("justfile", &justfile, "configs/client.example.conf");

    let readme = read("README.md");
    require_contains("README.md", &readme, "configs/mesh-node.example.conf");
    reject_contains("README.md", &readme, "configs/client.example.conf");
    reject_contains("README.md", &readme, "`gateway run`");
}

fn require_contains(path: &str, text: &str, needle: &str) {
    if !text.contains(needle) {
        fail(&format!(
            "product language guard: required marker missing in {path}: {needle}"
        ));
    }
}

fn reject_contains(path: &str, text: &str, needle: &str) {
    if text.contains(needle) {
        fail(&format!(
            "product language guard: forbidden marker found in {path}: {needle}"
        ));
    }
}

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1)
}
