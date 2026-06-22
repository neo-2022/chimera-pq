#[derive(Debug, Clone, PartialEq, Eq)]
struct LaunchPreflightVerifyOptions {
    side_a_report: String,
    side_b_report: String,
    json_output: bool,
    out_path: Option<String>,
}

pub(super) fn mesh_launch_preflight_verify_command(usage: &str, args: &[String]) -> i32 {
    let options = match parse_launch_preflight_verify_options(args) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh launch preflight verify parse failed: {error}");
            eprintln!("{usage}");
            return 2;
        }
    };
    let side_a_json = match std::fs::read_to_string(&options.side_a_report) {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "mesh launch preflight verify failed to read side-a report '{}': {error}",
                options.side_a_report
            );
            return 1;
        }
    };
    let side_b_json = match std::fs::read_to_string(&options.side_b_report) {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "mesh launch preflight verify failed to read side-b report '{}': {error}",
                options.side_b_report
            );
            return 1;
        }
    };
    let side_a: serde_json::Value = match serde_json::from_str(&side_a_json) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh launch preflight verify invalid side-a json: {error}");
            return 1;
        }
    };
    let side_b: serde_json::Value = match serde_json::from_str(&side_b_json) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh launch preflight verify invalid side-b json: {error}");
            return 1;
        }
    };
    let all_ready = collect_verify_blockers(&side_a, &side_b).is_empty();
    let output = render_verify_output(&side_a, &side_b, all_ready, options.json_output);
    if let Some(path) = options.out_path.as_deref()
        && let Err(error) = std::fs::write(path, &output)
    {
        eprintln!("mesh launch preflight verify write failed: {error}");
        return 1;
    }
    println!("{output}");
    if all_ready { 0 } else { 1 }
}

fn parse_launch_preflight_verify_options(
    args: &[String],
) -> Result<LaunchPreflightVerifyOptions, String> {
    let mut side_a_report = None;
    let mut side_b_report = None;
    let mut out_path = None;
    let mut json_output = false;
    let mut i = 0;
    while i < args.len() {
        let flag = args[i].as_str();
        if flag == "--json" {
            json_output = true;
            i += 1;
            continue;
        }
        if !flag.starts_with("--") {
            return Err(format!("unexpected positional argument '{flag}'"));
        }
        let value = args
            .get(i + 1)
            .map(String::as_str)
            .ok_or_else(|| format!("missing value for flag '{flag}'"))?;
        match flag {
            "--side-a-report" => {
                if side_a_report.is_some() {
                    return Err("duplicate singleton flag '--side-a-report'".to_string());
                }
                side_a_report = Some(non_empty(flag, value)?);
            }
            "--side-b-report" => {
                if side_b_report.is_some() {
                    return Err("duplicate singleton flag '--side-b-report'".to_string());
                }
                side_b_report = Some(non_empty(flag, value)?);
            }
            "--out" => {
                if out_path.is_some() {
                    return Err("duplicate singleton flag '--out'".to_string());
                }
                out_path = Some(non_empty(flag, value)?);
            }
            _ => return Err(format!("unknown flag '{flag}'")),
        }
        i += 2;
    }
    Ok(LaunchPreflightVerifyOptions {
        side_a_report: side_a_report.ok_or_else(|| "missing --side-a-report".to_string())?,
        side_b_report: side_b_report.ok_or_else(|| "missing --side-b-report".to_string())?,
        json_output,
        out_path,
    })
}

fn non_empty(flag: &str, value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("blank value for flag '{flag}'"));
    }
    Ok(trimmed.to_string())
}

fn is_ready_report(v: &serde_json::Value) -> bool {
    v["status"].as_str() == Some("ready")
        && v["ready_for_real_launch"].as_bool() == Some(true)
        && v["connect_probe_success"].as_bool() == Some(true)
        && v["network_state"].as_str() == Some("not_modified")
        && v["blockers"].as_array().is_some_and(|arr| arr.is_empty())
}

fn collect_verify_blockers(side_a: &serde_json::Value, side_b: &serde_json::Value) -> Vec<String> {
    let mut blockers = Vec::new();
    if !is_ready_report(side_a) {
        blockers.push("side_a_report_not_ready".to_string());
    }
    if !is_ready_report(side_b) {
        blockers.push("side_b_report_not_ready".to_string());
    }
    let side_a_ns = side_a["namespace"].as_str().unwrap_or("").trim();
    let side_b_ns = side_b["namespace"].as_str().unwrap_or("").trim();
    if side_a_ns.is_empty() || side_b_ns.is_empty() {
        blockers.push("namespace_missing".to_string());
    } else if side_a_ns != side_b_ns {
        blockers.push("namespace_mismatch".to_string());
    }
    blockers
}

fn render_verify_output(
    side_a: &serde_json::Value,
    side_b: &serde_json::Value,
    all_ready: bool,
    json_output: bool,
) -> String {
    let side_a_ready = is_ready_report(side_a);
    let side_b_ready = is_ready_report(side_b);
    let blockers = collect_verify_blockers(side_a, side_b);
    let namespace = side_a["namespace"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();
    if json_output {
        let blockers_json = blockers
            .iter()
            .map(|b| format!("\"{}\"", b))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"status\":\"{}\",\"all_ready\":{},\"side_a_ready\":{},\"side_b_ready\":{},\"namespace\":\"{}\",\"network_state\":\"not_modified\",\"blockers\":[{}]}}",
            if all_ready { "ready" } else { "blocked" },
            if all_ready { "true" } else { "false" },
            if side_a_ready { "true" } else { "false" },
            if side_b_ready { "true" } else { "false" },
            namespace,
            blockers_json,
        )
    } else {
        format!(
            "mesh launch preflight verify\nstatus: {}\nall_ready: {}\nside_a_ready: {}\nside_b_ready: {}\nnamespace: {}\nnetwork_state: not_modified\nblockers: {}",
            if all_ready { "ready" } else { "blocked" },
            all_ready,
            side_a_ready,
            side_b_ready,
            namespace,
            if blockers.is_empty() {
                "none".to_string()
            } else {
                blockers.join(",")
            },
        )
    }
}
