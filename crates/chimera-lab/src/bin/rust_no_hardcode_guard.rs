#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}

fn read_to_string(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|e| fail(&format!("guard: failed to read {}: {e}", path.display())))
}

fn try_read_text(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    String::from_utf8(bytes).ok()
}

fn collect_files(root: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).unwrap_or_else(|e| {
        fail(&format!(
            "guard: failed to read dir {}: {e}",
            root.display()
        ))
    });
    for entry in entries {
        let entry = entry.unwrap_or_else(|e| fail(&format!("guard: dir entry error: {e}")));
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|v| v.to_str()) == Some("target") {
                continue;
            }
            collect_files(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn path_has_component(path: &Path, component: &str) -> bool {
    path.components()
        .any(|c| c.as_os_str().to_string_lossy() == component)
}

fn contains_word(line: &str, needle: &str) -> bool {
    let mut start = 0usize;
    while let Some(idx) = line[start..].find(needle) {
        let abs = start + idx;
        let before_ok = abs == 0
            || !line[..abs]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        let after_idx = abs + needle.len();
        let after_ok = after_idx >= line.len()
            || !line[after_idx..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        if before_ok && after_ok {
            return true;
        }
        start = abs + needle.len();
    }
    false
}

fn line_has_python(line: &str) -> bool {
    contains_word(line, "python") || contains_word(line, "python3")
}

fn starts_with_python_command(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("python ")
        || trimmed == "python"
        || trimmed.starts_with("python3 ")
        || trimmed == "python3"
}

fn has_banned_runtime_probe_endpoint(text: &str) -> bool {
    [
        "https://example.org",
        "http://youtube.com",
        "https://youtube.com",
        "http://www.youtube.com",
        "https://www.youtube.com",
        "http://discord.com",
        "https://discord.com",
        "http://chat.openai.com",
        "https://chat.openai.com",
        "socks5h://127.0.0.1:11080",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn has_banned_runtime_bin_url_literal(text: &str) -> bool {
    [
        "http://",
        "https://",
        "ws://",
        "wss://",
        "socks5://",
        "socks5h://",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn has_banned_transparent_tcp_payload_destination(text: &str) -> bool {
    [
        "parse_proxy_destination",
        "GET http://",
        "GET https://",
        "POST http://",
        "POST https://",
        "strip_prefix(\"CONNECT \")",
        "strip_prefix(\"GET http://\")",
        "strip_prefix(\"POST http://\")",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn has_banned_machine_resource_literal(text: &str) -> bool {
    let text_without_private_cidr = text.replace("192.168.0.0/16", "");
    if text_without_private_cidr.contains("192.168.") {
        return true;
    }
    let banned_literals = [
        ["CHIMERA_", "V", "PS_ENDPOINT"].concat(),
        ["CHIMERA_", "LAP", "TOP_"].concat(),
        ["v", "ps_host"].concat(),
        ["l", "aptop_host"].concat(),
        ["note", "book"].concat(),
        "gosuslugi".to_string(),
        "ozon.ru".to_string(),
        "mos.ru".to_string(),
        "jB5@".to_string(),
        "chimera-peer-egress-20260526".to_string(),
        "127.0.0.1:12090".to_string(),
        "0.0.0.0:12091".to_string(),
    ];
    banned_literals
        .iter()
        .any(|needle| text.contains(needle.as_str()))
}

fn has_banned_product_doc_device_marker(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("notebook")
        || lower.contains("laptop")
        || lower.contains("vps")
        || text.contains("91.124.")
        || text.contains("192.168.31.")
        || text.contains("root@91.")
        || text.contains("art@192.")
}

fn has_banned_active_product_doc_stand_marker(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("<stand-")
        || lower.contains("stand-host")
        || lower.contains("stand verification")
        || lower.contains("stand proof")
        || lower.contains("stand install")
        || text.contains("CHIMERA_SIDE_A")
        || text.contains("CHIMERA_SIDE_B")
        || text.contains("SIDE_A +")
        || text.contains("Side B/SIDE_A")
}

fn has_ambiguous_chimera_lab_cargo_run(line: &str) -> bool {
    let cleaned = line.split('#').next().unwrap_or_default().trim();
    if cleaned.is_empty() {
        return false;
    }
    let tokens: Vec<&str> = cleaned.split_whitespace().collect();
    if tokens.len() < 2 || tokens[0] != "cargo" || tokens[1] != "run" {
        return false;
    }
    let mut has_chimera_lab_pkg = false;
    let mut has_double_dash = false;
    let mut has_bin = false;
    let mut i = 2usize;
    while i < tokens.len() {
        let t = tokens[i];
        if t == "-p" && i + 1 < tokens.len() && tokens[i + 1] == "chimera-lab" {
            has_chimera_lab_pkg = true;
            i += 2;
            continue;
        }
        if t == "--package" && i + 1 < tokens.len() && tokens[i + 1] == "chimera-lab" {
            has_chimera_lab_pkg = true;
            i += 2;
            continue;
        }
        if t == "--" {
            has_double_dash = true;
        }
        if t == "--bin" && i + 1 < tokens.len() && !tokens[i + 1].starts_with('-') {
            has_bin = true;
            i += 2;
            continue;
        }
        if t.starts_with("--bin=") && t.len() > "--bin=".len() {
            has_bin = true;
        }
        i += 1;
    }
    has_chimera_lab_pkg && has_double_dash && !has_bin
}

fn main() {
    let root = Path::new(".");
    let mut all_files = Vec::new();
    collect_files(root, &mut all_files);

    let mut py_sources: Vec<String> = all_files
        .iter()
        .filter(|p| p.extension().and_then(|v| v.to_str()) == Some("py"))
        .map(|p| p.display().to_string())
        .collect();
    py_sources.sort();
    if !py_sources.is_empty() {
        eprintln!("rust/no-hardcode guard: python source found in project tree");
        for p in py_sources {
            eprintln!("{p}");
        }
        std::process::exit(1);
    }

    let mut runtime_script_hits = Vec::new();
    for path in &all_files {
        let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
            continue;
        };
        if !name.starts_with("runtime")
            || !name.ends_with("_smoke.sh")
            || !path_has_component(path, "scripts")
        {
            continue;
        }
        let Some(text) = try_read_text(path) else {
            continue;
        };
        for (i, line) in text.lines().enumerate() {
            if line_has_python(line) {
                runtime_script_hits.push(format!("{}:{}", path.display(), i + 1));
            }
        }
    }
    if !runtime_script_hits.is_empty() {
        eprintln!("rust/no-hardcode guard: python usage found in runtime smoke scripts");
        for hit in runtime_script_hits {
            eprintln!("{hit}");
        }
        std::process::exit(1);
    }

    let mut script_hits = Vec::new();
    for path in &all_files {
        if !path_has_component(path, "scripts")
            || path.extension().and_then(|v| v.to_str()) != Some("sh")
        {
            continue;
        }
        if path.file_name().and_then(|v| v.to_str()) == Some("rust_no_hardcode_guard.sh") {
            continue;
        }
        let Some(text) = try_read_text(path) else {
            continue;
        };
        for (i, line) in text.lines().enumerate() {
            if line_has_python(line) {
                script_hits.push(format!("{}:{}", path.display(), i + 1));
            }
        }
    }
    if !script_hits.is_empty() {
        eprintln!("rust/no-hardcode guard: python usage found in scripts/*.sh");
        for hit in script_hits {
            eprintln!("{hit}");
        }
        std::process::exit(1);
    }

    let mut machine_resource_hits = Vec::new();
    for path in &all_files {
        let is_target = path_has_component(path, "crates")
            || path_has_component(path, "scripts")
            || path.file_name().and_then(|v| v.to_str()) == Some("justfile");
        if !is_target {
            continue;
        }
        if path.file_name().and_then(|v| v.to_str()) == Some("rust_no_hardcode_guard.rs") {
            continue;
        }
        if matches!(
            path.file_name().and_then(|v| v.to_str()),
            Some(
                "mesh_launch_preflight_report_guard.rs"
                    | "mesh_route_explain_error_guard.rs"
                    | "mesh_route_explain_guard.rs"
            )
        ) {
            continue;
        }
        let Some(text) = try_read_text(path) else {
            continue;
        };
        if has_banned_machine_resource_literal(&text) {
            machine_resource_hits.push(path.display().to_string());
        }
    }
    if !machine_resource_hits.is_empty() {
        eprintln!("rust/no-hardcode guard: machine/resource-specific literal found");
        for hit in machine_resource_hits {
            eprintln!("{hit}");
        }
        std::process::exit(1);
    }

    let justfile = read_to_string(Path::new("justfile"));
    let mut just_hits = Vec::new();
    for (i, line) in justfile.lines().enumerate() {
        if starts_with_python_command(line) {
            just_hits.push(i + 1);
        }
    }
    if !just_hits.is_empty() {
        eprintln!("rust/no-hardcode guard: python execution found in justfile");
        for line in just_hits {
            eprintln!("justfile:{line}");
        }
        std::process::exit(1);
    }

    let mut doc_hits = Vec::new();
    for doc in [Path::new("README.md"), Path::new("docs")] {
        if doc.is_dir() {
            let mut doc_files = Vec::new();
            collect_files(doc, &mut doc_files);
            for file in doc_files {
                let Some(text) = try_read_text(&file) else {
                    continue;
                };
                for (i, line) in text.lines().enumerate() {
                    if starts_with_python_command(line) {
                        doc_hits.push(format!("{}:{}", file.display(), i + 1));
                    }
                }
            }
        } else if doc.is_file() {
            let text = read_to_string(doc);
            for (i, line) in text.lines().enumerate() {
                if starts_with_python_command(line) {
                    doc_hits.push(format!("{}:{}", doc.display(), i + 1));
                }
            }
        }
    }
    if !doc_hits.is_empty() {
        eprintln!("rust/no-hardcode guard: python command found in docs/README");
        for hit in doc_hits {
            eprintln!("{hit}");
        }
        std::process::exit(1);
    }

    let mut product_doc_device_hits = Vec::new();
    for doc in [Path::new("README.md"), Path::new("docs")] {
        if doc.is_dir() {
            let mut doc_files = Vec::new();
            collect_files(doc, &mut doc_files);
            for file in doc_files {
                let Some(text) = try_read_text(&file) else {
                    continue;
                };
                for (i, line) in text.lines().enumerate() {
                    if has_banned_product_doc_device_marker(line) {
                        product_doc_device_hits.push(format!("{}:{}", file.display(), i + 1));
                    }
                }
            }
        } else if doc.is_file() {
            let text = read_to_string(doc);
            for (i, line) in text.lines().enumerate() {
                if has_banned_product_doc_device_marker(line) {
                    product_doc_device_hits.push(format!("{}:{}", doc.display(), i + 1));
                }
            }
        }
    }
    if !product_doc_device_hits.is_empty() {
        eprintln!("rust/no-hardcode guard: product docs stand-specific device marker found");
        for hit in product_doc_device_hits {
            eprintln!("{hit}");
        }
        std::process::exit(1);
    }

    let mut active_product_doc_stand_hits = Vec::new();
    for doc in [
        Path::new("README.md"),
        Path::new("docs/OPERATIONS.md"),
        Path::new("docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md"),
    ] {
        if !doc.is_file() {
            continue;
        }
        let text = read_to_string(doc);
        for (i, line) in text.lines().enumerate() {
            if has_banned_active_product_doc_stand_marker(line) {
                active_product_doc_stand_hits.push(format!("{}:{}", doc.display(), i + 1));
            }
        }
    }
    if !active_product_doc_stand_hits.is_empty() {
        eprintln!("rust/no-hardcode guard: active product docs contain external-stand markers");
        for hit in active_product_doc_stand_hits {
            eprintln!("{hit}");
        }
        std::process::exit(1);
    }

    let probe_rs = Path::new("crates/chimera-lab/src/bin/runtime_real_world_probe.rs");
    if !probe_rs.is_file() {
        fail(
            "rust/no-hardcode guard: missing crates/chimera-lab/src/bin/runtime_real_world_probe.rs",
        );
    }
    let probe_text = read_to_string(probe_rs);
    for required in [
        r#"resolve_non_empty_setting("CHIMERA_REAL_WORLD_DIRECT_URL""#,
        r#"resolve_non_empty_setting("CHIMERA_REAL_WORLD_DATAPATH_TARGETS""#,
        "CHIMERA_REAL_WORLD_DIRECT_TIMEOUT_SEC",
        "CHIMERA_REAL_WORLD_DATAPATH_TIMEOUT_SEC",
        "CHIMERA_REAL_WORLD_CONFIG",
    ] {
        if !probe_text.contains(required) {
            fail(&format!(
                "rust/no-hardcode guard: missing required marker in runtime_real_world_probe.rs: {required}"
            ));
        }
    }

    if has_banned_runtime_probe_endpoint(&probe_text) {
        fail("rust/no-hardcode guard: baked runtime target found in runtime_real_world_probe.rs");
    }

    let mut runtime_bins: Vec<PathBuf> = all_files
        .iter()
        .filter(|p| {
            p.to_string_lossy().contains("/src/bin/runtime_")
                && p.extension().and_then(|v| v.to_str()) == Some("rs")
                && !p.to_string_lossy().ends_with("_schema_guard.rs")
                && !p.to_string_lossy().ends_with("_env.rs")
        })
        .cloned()
        .collect();
    runtime_bins.sort();
    for path in &runtime_bins {
        let text = read_to_string(path);
        if has_banned_runtime_bin_url_literal(&text) {
            fail("rust/no-hardcode guard: baked URL/proxy endpoint found in runtime Rust bins");
        }
    }

    let transparent_tcp_rs = Path::new("crates/chimera-capture/src/bin/chimera-transparent-tcp.rs");
    if !transparent_tcp_rs.is_file() {
        fail("rust/no-hardcode guard: missing chimera-transparent-tcp.rs");
    }
    let transparent_tcp_text = read_to_string(transparent_tcp_rs);
    if has_banned_transparent_tcp_payload_destination(&transparent_tcp_text) {
        fail(
            "rust/no-hardcode guard: transparent TCP must not derive destination from manual proxy payload",
        );
    }
    for banned in [
        "--static-destination",
        "CHIMERA_TRANSPARENT_TCP_STATIC_DESTINATION",
        "DirectMode::Auto",
        "connect_direct(",
        "transparent_direct_failed",
        "route=direct destination_state=resolved",
    ] {
        if transparent_tcp_text.contains(banned) {
            fail(&format!(
                "rust/no-hardcode guard: transparent TCP product direct/proxy-style bypass is forbidden: {banned}"
            ));
        }
    }
    for required in [
        "detect_forbidden_manual_proxy_protocol(initial)",
        "manual_proxy_ingress_forbidden",
        "original_destination(client)",
    ] {
        if !transparent_tcp_text.contains(required) {
            fail(&format!(
                "rust/no-hardcode guard: missing transparent TCP fail-closed marker: {required}"
            ));
        }
    }
    for path in [
        transparent_tcp_rs,
        Path::new("crates/chimera-capture/src/bin/chimera-transparent-runtime.rs"),
    ] {
        let text = read_to_string(path);
        for banned in [
            "--first-response-timeout-ms",
            "CHIMERA_TRANSPARENT_TCP_FIRST_RESPONSE_TIMEOUT_MS",
            "first_response_timeout_ms",
        ] {
            if text.contains(banned) {
                fail(&format!(
                    "rust/no-hardcode guard: transparent path must not probe by sending payload before final route selection: {} contains {banned}",
                    path.display()
                ));
            }
        }
    }
    for (path, required) in [
        (transparent_tcp_rs, "unwrap_or(DirectMode::Disabled)"),
        (
            Path::new("crates/chimera-capture/src/bin/chimera-transparent-runtime.rs"),
            "unwrap_or_else(|| \"disabled\".to_string())",
        ),
        (
            Path::new("scripts/install_desktop_control.sh"),
            "CHIMERA_TRANSPARENT_TCP_DIRECT_MODE:-disabled",
        ),
        (transparent_tcp_rs, "direct-mode auto is forbidden"),
        (
            Path::new("crates/chimera-capture/src/bin/chimera-transparent-runtime.rs"),
            "direct-mode auto is forbidden",
        ),
    ] {
        let text = read_to_string(path);
        if !text.contains(required) {
            fail(&format!(
                "rust/no-hardcode guard: transparent direct path must fail closed by default: {} missing {required}",
                path.display()
            ));
        }
        for banned in [
            "unwrap_or(DirectMode::Auto)",
            "unwrap_or_else(|| \"auto\".to_string())",
            "CHIMERA_TRANSPARENT_TCP_DIRECT_MODE:-auto",
        ] {
            if text.contains(banned) {
                fail(&format!(
                    "rust/no-hardcode guard: transparent direct-mode auto default is forbidden: {} contains {banned}",
                    path.display()
                ));
            }
        }
    }
    for path in [
        Path::new("configs/adaptive_domains.txt"),
        Path::new("configs/auto_failover_seeds.txt"),
        Path::new("configs/manual_transit_domains.txt"),
        Path::new("configs/manual_gateway_domains.txt"),
    ] {
        let text = read_to_string(path);
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            fail(&format!(
                "rust/no-hardcode guard: release default route/adaptive domain list must be empty: {}",
                path.display()
            ));
        }
    }
    let runtime_policy_text = read_to_string(Path::new("configs/policy.runtime.conf"));
    for line in runtime_policy_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        fail("rust/no-hardcode guard: runtime policy default must not contain baked route rules");
    }
    let autofix_text = read_to_string(Path::new("scripts/chimera-autofix.sh"));
    for banned in [
        "default-transit = default => transit",
        "youtube.com",
        "googlevideo.com",
        "ytimg.com",
        "gstatic.com",
    ] {
        if autofix_text.contains(banned) {
            fail(&format!(
                "rust/no-hardcode guard: autofix must not bake public domains or default transit: {banned}"
            ));
        }
    }
    let capture_lib = Path::new("crates/chimera-capture/src/lib.rs");
    let capture_lib_text = read_to_string(capture_lib);
    for banned in [
        "keep direct to avoid false hijack",
        "direct path failed but transit path not verified; keep direct",
    ] {
        if capture_lib_text.contains(banned) {
            fail(&format!(
                "rust/no-hardcode guard: transparent failover must fail closed when CHIMERA transit is not verified: {banned}"
            ));
        }
    }
    for required in [
        "route: DatapathRoute::Block",
        "CHIMERA transit path is not verified; fail closed",
        "observation_fails_closed_when_transit_is_not_verified",
    ] {
        if !capture_lib_text.contains(required) {
            fail(&format!(
                "rust/no-hardcode guard: missing transparent fail-closed evidence: {required}"
            ));
        }
    }

    let mut ambiguous_hits = Vec::new();
    for path in &all_files {
        let is_target = path.file_name().and_then(|v| v.to_str()) == Some("justfile")
            || (path_has_component(path, "scripts")
                && path.extension().and_then(|v| v.to_str()) == Some("sh"));
        if !is_target {
            continue;
        }
        if path.file_name().and_then(|v| v.to_str()) == Some("rust_no_hardcode_guard.sh") {
            continue;
        }
        let Some(text) = try_read_text(path) else {
            continue;
        };
        for (i, line) in text.lines().enumerate() {
            if has_ambiguous_chimera_lab_cargo_run(line) {
                ambiguous_hits.push(format!("{}:{}", path.display(), i + 1));
            }
        }
    }
    if !ambiguous_hits.is_empty() {
        eprintln!("rust/no-hardcode guard: ambiguous cargo run for chimera-lab (missing --bin)");
        for hit in ambiguous_hits {
            eprintln!("{hit}");
        }
        std::process::exit(1);
    }

    println!("rust/no-hardcode guard: PASS");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_match_works_for_python_tokens() {
        assert!(contains_word("python", "python"));
        assert!(contains_word("run python3 -V", "python3"));
        assert!(!contains_word("cpython", "python"));
        assert!(!contains_word("pythonic", "python"));
    }

    #[test]
    fn python_command_detection_works_with_indent() {
        assert!(starts_with_python_command("python -V"));
        assert!(starts_with_python_command("    python3 script.py"));
        assert!(!starts_with_python_command("echo python"));
    }

    #[test]
    fn banned_probe_endpoint_detection_is_precise() {
        assert!(has_banned_runtime_probe_endpoint("x https://youtube.com y"));
        assert!(has_banned_runtime_probe_endpoint(
            "socks5h://127.0.0.1:11080"
        ));
        assert!(!has_banned_runtime_probe_endpoint("youtube.com"));
        assert!(!has_banned_runtime_probe_endpoint("discord.com"));
    }

    #[test]
    fn banned_runtime_bin_url_literal_detection_works() {
        assert!(has_banned_runtime_bin_url_literal("http://x"));
        assert!(has_banned_runtime_bin_url_literal("ws://x"));
        assert!(has_banned_runtime_bin_url_literal("wss://x"));
        assert!(has_banned_runtime_bin_url_literal("socks5://x"));
        assert!(!has_banned_runtime_bin_url_literal("example.org"));
    }

    #[test]
    fn banned_transparent_tcp_payload_destination_detection_works() {
        assert!(has_banned_transparent_tcp_payload_destination(
            "fn parse_proxy_destination() {}"
        ));
        assert!(has_banned_transparent_tcp_payload_destination(
            "text.strip_prefix(\"CONNECT \")"
        ));
        assert!(has_banned_transparent_tcp_payload_destination(
            "GET http://example.invalid"
        ));
        assert!(!has_banned_transparent_tcp_payload_destination(
            "detect_forbidden_manual_proxy_protocol(initial)"
        ));
    }

    #[test]
    fn banned_machine_resource_literal_detection_works() {
        assert!(has_banned_machine_resource_literal("connect 192.168.10.10"));
        assert!(has_banned_machine_resource_literal(
            &["export ", "CHIMERA_", "LAP", "TOP_HOST=x"].concat()
        ));
        assert!(has_banned_machine_resource_literal(
            &["external ", "note", "book", " host"].concat()
        ));
        assert!(has_banned_machine_resource_literal("open gosuslugi"));
        assert!(has_banned_machine_resource_literal("127.0.0.1:12090"));
        assert!(!has_banned_machine_resource_literal("203.0.113.10"));
        assert!(!has_banned_machine_resource_literal("192.168.0.0/16"));
    }

    #[test]
    fn banned_product_doc_device_marker_detection_works() {
        assert!(has_banned_product_doc_device_marker("notebook proof"));
        assert!(has_banned_product_doc_device_marker("laptop proof"));
        assert!(has_banned_product_doc_device_marker("VPS proof"));
        assert!(has_banned_product_doc_device_marker("root@91.example"));
        assert!(has_banned_product_doc_device_marker("192.168.31.21"));
        assert!(!has_banned_product_doc_device_marker(
            "external remote proof node"
        ));
    }

    #[test]
    fn banned_active_product_doc_stand_marker_detection_works() {
        assert!(has_banned_active_product_doc_stand_marker(
            "connect <stand-host-b-ip>:443"
        ));
        assert!(has_banned_active_product_doc_stand_marker(
            "Stand Install/Update Contract"
        ));
        assert!(has_banned_active_product_doc_stand_marker(
            "CHIMERA_SIDE_B_HOST"
        ));
        assert!(has_banned_active_product_doc_stand_marker(
            "Side B/SIDE_A stand verification"
        ));
        assert!(!has_banned_active_product_doc_stand_marker(
            "external proof node_a and node_b"
        ));
    }

    #[test]
    fn ambiguous_cargo_run_detection_works() {
        assert!(has_ambiguous_chimera_lab_cargo_run(
            "cargo run -p chimera-lab -- lab health"
        ));
        assert!(has_ambiguous_chimera_lab_cargo_run(
            "cargo run -q -p chimera-lab -- lab health"
        ));
        assert!(has_ambiguous_chimera_lab_cargo_run(
            "cargo run --package chimera-lab -- --help"
        ));
        assert!(has_ambiguous_chimera_lab_cargo_run(
            "cargo run -q -p chimera-lab --bin -- lab health"
        ));
        assert!(!has_ambiguous_chimera_lab_cargo_run(
            "cargo run -q -p chimera-lab --bin runtime_real_world_probe --"
        ));
        assert!(!has_ambiguous_chimera_lab_cargo_run(
            "cargo run -p chimera-lab --bin=runtime_real_world_probe --"
        ));
        assert!(!has_ambiguous_chimera_lab_cargo_run(
            "cargo run -p chimera-cli -- up"
        ));
        assert!(!has_ambiguous_chimera_lab_cargo_run(
            "# cargo run -p chimera-lab -- lab health"
        ));
    }
}
