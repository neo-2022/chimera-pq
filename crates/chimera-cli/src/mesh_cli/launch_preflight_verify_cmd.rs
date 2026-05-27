#[derive(Debug, Clone, PartialEq, Eq)]
struct LaunchPreflightVerifyOptions {
    vps_report: String,
    laptop_report: String,
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
    let vps_json = match std::fs::read_to_string(&options.vps_report) {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "mesh launch preflight verify failed to read vps report '{}': {error}",
                options.vps_report
            );
            return 1;
        }
    };
    let laptop_json = match std::fs::read_to_string(&options.laptop_report) {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "mesh launch preflight verify failed to read laptop report '{}': {error}",
                options.laptop_report
            );
            return 1;
        }
    };
    let vps: serde_json::Value = match serde_json::from_str(&vps_json) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh launch preflight verify invalid vps json: {error}");
            return 1;
        }
    };
    let laptop: serde_json::Value = match serde_json::from_str(&laptop_json) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh launch preflight verify invalid laptop json: {error}");
            return 1;
        }
    };
    let all_ready = collect_verify_blockers(&vps, &laptop).is_empty();
    let output = render_verify_output(&vps, &laptop, all_ready, options.json_output);
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
    let mut vps_report = None;
    let mut laptop_report = None;
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
            "--vps-report" => {
                if vps_report.is_some() {
                    return Err("duplicate singleton flag '--vps-report'".to_string());
                }
                vps_report = Some(non_empty(flag, value)?);
            }
            "--laptop-report" => {
                if laptop_report.is_some() {
                    return Err("duplicate singleton flag '--laptop-report'".to_string());
                }
                laptop_report = Some(non_empty(flag, value)?);
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
        vps_report: vps_report.ok_or_else(|| "missing --vps-report".to_string())?,
        laptop_report: laptop_report.ok_or_else(|| "missing --laptop-report".to_string())?,
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

fn collect_verify_blockers(vps: &serde_json::Value, laptop: &serde_json::Value) -> Vec<String> {
    let mut blockers = Vec::new();
    if !is_ready_report(vps) {
        blockers.push("vps_report_not_ready".to_string());
    }
    if !is_ready_report(laptop) {
        blockers.push("laptop_report_not_ready".to_string());
    }
    let vps_ns = vps["namespace"].as_str().unwrap_or("").trim();
    let laptop_ns = laptop["namespace"].as_str().unwrap_or("").trim();
    if vps_ns.is_empty() || laptop_ns.is_empty() {
        blockers.push("namespace_missing".to_string());
    } else if vps_ns != laptop_ns {
        blockers.push("namespace_mismatch".to_string());
    }
    blockers
}

fn render_verify_output(
    vps: &serde_json::Value,
    laptop: &serde_json::Value,
    all_ready: bool,
    json_output: bool,
) -> String {
    let vps_ready = is_ready_report(vps);
    let laptop_ready = is_ready_report(laptop);
    let blockers = collect_verify_blockers(vps, laptop);
    let namespace = vps["namespace"].as_str().unwrap_or("").trim().to_string();
    if json_output {
        let blockers_json = blockers
            .iter()
            .map(|b| format!("\"{}\"", b))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"status\":\"{}\",\"all_ready\":{},\"vps_ready\":{},\"laptop_ready\":{},\"namespace\":\"{}\",\"network_state\":\"not_modified\",\"blockers\":[{}]}}",
            if all_ready { "ready" } else { "blocked" },
            if all_ready { "true" } else { "false" },
            if vps_ready { "true" } else { "false" },
            if laptop_ready { "true" } else { "false" },
            namespace,
            blockers_json,
        )
    } else {
        format!(
            "mesh launch preflight verify\nstatus: {}\nall_ready: {}\nvps_ready: {}\nlaptop_ready: {}\nnamespace: {}\nnetwork_state: not_modified\nblockers: {}",
            if all_ready { "ready" } else { "blocked" },
            all_ready,
            vps_ready,
            laptop_ready,
            namespace,
            if blockers.is_empty() {
                "none".to_string()
            } else {
                blockers.join(",")
            },
        )
    }
}
