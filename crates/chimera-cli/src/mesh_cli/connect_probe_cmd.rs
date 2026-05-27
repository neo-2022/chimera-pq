use crate::mesh_cli::connect_probe_flow::run_mesh_connect_probe_flow;
use crate::mesh_cli::options;
use crate::mesh_cli::route_explain_error::emit_route_explain_error;
use crate::mesh_cli::route_explain_error_consts::STAGE_OPTIONS_PARSE;
use chimera_mesh::MeshConnectProbeReport;

pub(super) fn mesh_connect_probe_command(usage: &str, args: &[String]) -> i32 {
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
                eprintln!("mesh connect probe error write failed: {error}");
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
    let report = match run_mesh_connect_probe_flow(&options, "cli-connect-probe") {
        Ok((value, _timeout_ms)) => value,
        Err(error) => {
            return emit_route_explain_error(&options, error.stage, &error.message);
        }
    };
    let output = render_connect_probe_output(&report, options.json_output);
    if let Some(path) = options.out_path.as_deref()
        && let Err(error) = std::fs::write(path, &output)
    {
        eprintln!("mesh connect probe write failed: {error}");
        return 1;
    }
    println!("{output}");
    if report.success { 0 } else { 1 }
}

fn render_connect_probe_output(report: &MeshConnectProbeReport, json_output: bool) -> String {
    if json_output {
        let mut attempts = String::new();
        for (idx, attempt) in report.attempts.iter().enumerate() {
            if idx > 0 {
                attempts.push(',');
            }
            attempts.push_str(&format!(
                "{{\"peer_id\":\"{}\",\"endpoint\":\"{}\",\"success\":{},\"error\":\"{}\"}}",
                escape_json(&attempt.peer_id),
                escape_json(&attempt.endpoint),
                if attempt.success { "true" } else { "false" },
                escape_json(&attempt.error),
            ));
        }
        let selected_peers = report
            .selected_peers
            .iter()
            .map(|v| format!("\"{}\"", escape_json(v)))
            .collect::<Vec<_>>()
            .join(",");
        let explain = report
            .explain
            .iter()
            .map(|v| format!("\"{}\"", escape_json(v)))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"namespace\":\"{}\",\"success\":{},\"connected_peer\":\"{}\",\"connected_endpoint\":\"{}\",\"selected_peers\":[{}],\"attempts\":[{}],\"explain\":[{}]}}",
            escape_json(&report.namespace),
            if report.success { "true" } else { "false" },
            escape_json(&report.connected_peer),
            escape_json(&report.connected_endpoint),
            selected_peers,
            attempts,
            explain,
        )
    } else {
        let mut out = String::new();
        out.push_str("mesh connect probe\n");
        out.push_str(&format!("namespace: {}\n", report.namespace));
        out.push_str(&format!("success: {}\n", report.success));
        if report.success {
            out.push_str(&format!("connected_peer: {}\n", report.connected_peer));
            out.push_str(&format!(
                "connected_endpoint: {}\n",
                report.connected_endpoint
            ));
        }
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
