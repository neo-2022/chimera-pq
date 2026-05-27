use crate::mesh_cli::connect_probe_flow::run_mesh_connect_probe_flow;
use crate::mesh_cli::options;
use crate::mesh_cli::route_explain_error::emit_route_explain_error;
use crate::mesh_cli::route_explain_error_consts::STAGE_OPTIONS_PARSE;
use chimera_mesh::MeshConnectProbeReport;

pub(super) fn mesh_launch_preflight_command(usage: &str, args: &[String]) -> i32 {
    let options = match options::parse_mesh_route_explain_options(args) {
        Ok(value) => value,
        Err(error) => {
            let namespace = options::extract_non_empty_flag_value(args, "--namespace")
                .unwrap_or_else(|| "unknown".to_string());
            let node = options::extract_non_empty_flag_value(args, "--node")
                .unwrap_or_else(|| "unknown".to_string());
            let json =
                crate::mesh_cli::route_explain_error::build_route_explain_error_json_with_identity(
                    &namespace,
                    &node,
                    STAGE_OPTIONS_PARSE,
                    &error,
                );
            if let Some(path) = options::extract_non_empty_flag_value(args, "--out")
                && let Err(error) = std::fs::write(path, &json)
            {
                eprintln!("Не удалось записать ошибку mesh launch preflight: {error}");
                return 1;
            }
            if options::wants_json_output(args) {
                println!("{json}");
                return 2;
            }
            eprintln!("{usage}");
            return 2;
        }
    };

    let (report, timeout_ms) = match run_mesh_connect_probe_flow(&options, "cli-launch-preflight") {
        Ok(value) => value,
        Err(error) => {
            return emit_route_explain_error(&options, error.stage, &error.message);
        }
    };

    let output = render_launch_preflight_output(
        &options.node_name,
        timeout_ms,
        &report,
        options.json_output,
    );
    if let Some(path) = options.out_path.as_deref()
        && let Err(error) = std::fs::write(path, &output)
    {
        eprintln!("Не удалось записать вывод mesh launch preflight: {error}");
        return 1;
    }
    println!("{output}");
    if report.success { 0 } else { 1 }
}

fn render_launch_preflight_output(
    node_name: &str,
    timeout_ms: u64,
    report: &MeshConnectProbeReport,
    json_output: bool,
) -> String {
    let ready_for_real_launch = report.success;
    let blockers = if ready_for_real_launch {
        String::new()
    } else {
        "connectivity_probe_failed".to_string()
    };
    if json_output {
        let selected_peers = report
            .selected_peers
            .iter()
            .map(|v| format!("\"{}\"", escape_json(v)))
            .collect::<Vec<_>>()
            .join(",");
        let attempts = report
            .attempts
            .iter()
            .map(|attempt| {
                format!(
                    "{{\"peer_id\":\"{}\",\"endpoint\":\"{}\",\"success\":{},\"error\":\"{}\"}}",
                    escape_json(&attempt.peer_id),
                    escape_json(&attempt.endpoint),
                    if attempt.success { "true" } else { "false" },
                    escape_json(&attempt.error)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let explain = report
            .explain
            .iter()
            .map(|v| format!("\"{}\"", escape_json(v)))
            .collect::<Vec<_>>()
            .join(",");
        let blockers_json = if blockers.is_empty() {
            String::new()
        } else {
            format!("\"{}\"", escape_json(&blockers))
        };
        format!(
            "{{\"status\":\"{}\",\"network_state\":\"not_modified\",\"namespace\":\"{}\",\"node\":\"{}\",\"timeout_ms\":{},\"ready_for_real_launch\":{},\"blockers\":[{}],\"selected_peers\":[{}],\"connected_peer\":\"{}\",\"connected_endpoint\":\"{}\",\"connect_probe_success\":{},\"attempts\":[{}],\"explain\":[{}]}}",
            if ready_for_real_launch {
                "ready"
            } else {
                "blocked"
            },
            escape_json(&report.namespace),
            escape_json(node_name),
            timeout_ms,
            if ready_for_real_launch {
                "true"
            } else {
                "false"
            },
            blockers_json,
            selected_peers,
            escape_json(&report.connected_peer),
            escape_json(&report.connected_endpoint),
            if report.success { "true" } else { "false" },
            attempts,
            explain,
        )
    } else {
        let mut out = String::new();
        out.push_str("mesh launch preflight\n");
        out.push_str("network_state: not_modified\n");
        out.push_str(&format!("namespace: {}\n", report.namespace));
        out.push_str(&format!("node: {}\n", node_name));
        out.push_str(&format!("timeout_ms: {}\n", timeout_ms));
        out.push_str(&format!(
            "ready_for_real_launch: {}\n",
            ready_for_real_launch
        ));
        if !blockers.is_empty() {
            out.push_str(&format!("blocker: {}\n", blockers));
        }
        out.push_str(&format!("connect_probe_success: {}\n", report.success));
        out.push_str(&format!("connected_peer: {}\n", report.connected_peer));
        out.push_str(&format!(
            "connected_endpoint: {}\n",
            report.connected_endpoint
        ));
        out.push_str("attempts:\n");
        for attempt in &report.attempts {
            if attempt.success {
                out.push_str(&format!(
                    "  - {} {} ok\n",
                    attempt.peer_id, attempt.endpoint
                ));
            } else {
                out.push_str(&format!(
                    "  - {} {} fail ({})\n",
                    attempt.peer_id, attempt.endpoint, attempt.error
                ));
            }
        }
        out
    }
}

fn escape_json(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}
