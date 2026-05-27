#![forbid(unsafe_code)]

use std::env;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

use chimera_capture::redirect::{TransparentRedirectPlan, default_bypass_cidrs_v4};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Options {
    transparent_bin: String,
    listen: String,
    gateway_local: String,
    direct_mode: String,
    direct_timeout_ms: u64,
    first_response_timeout_ms: u64,
    initial_read_timeout_ms: u64,
    table_name: String,
    chain_name: String,
    exempt_uid: u32,
    transparent_uid: Option<u32>,
    transparent_gid: Option<u32>,
    bypass_cidrs_v4: Vec<String>,
    capture_cidrs_v4: Vec<String>,
    capture_tcp_ports: Vec<u16>,
    run_ms: Option<u64>,
    print_only: bool,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut transparent_bin = env_value("CHIMERA_TRANSPARENT_BIN")
            .unwrap_or_else(|| "chimera-transparent-tcp".to_string());
        let mut listen = env_value("CHIMERA_TRANSPARENT_TCP_LISTEN");
        let mut gateway_local = env_value("CHIMERA_TRANSPARENT_TCP_GATEWAY_LOCAL");
        let mut direct_mode =
            env_value("CHIMERA_TRANSPARENT_TCP_DIRECT_MODE").unwrap_or_else(|| "auto".to_string());
        let mut direct_timeout_ms = env_value("CHIMERA_TRANSPARENT_TCP_DIRECT_TIMEOUT_MS")
            .map(|value| parse_positive_u64(&value, "direct-timeout-ms"))
            .transpose()?
            .unwrap_or(1200);
        let mut first_response_timeout_ms =
            env_value("CHIMERA_TRANSPARENT_TCP_FIRST_RESPONSE_TIMEOUT_MS")
                .map(|value| parse_positive_u64(&value, "first-response-timeout-ms"))
                .transpose()?
                .unwrap_or(1800);
        let mut initial_read_timeout_ms =
            env_value("CHIMERA_TRANSPARENT_TCP_INITIAL_READ_TIMEOUT_MS")
                .map(|value| parse_positive_u64(&value, "initial-read-timeout-ms"))
                .transpose()?
                .unwrap_or(500);
        let mut table_name =
            env_value("CHIMERA_REDIRECT_TABLE").unwrap_or_else(|| "chimera_redirect".to_string());
        let mut chain_name =
            env_value("CHIMERA_REDIRECT_CHAIN").unwrap_or_else(|| "output".to_string());
        let mut exempt_uid = env_value("CHIMERA_REDIRECT_EXEMPT_UID")
            .map(|value| parse_u32(&value, "exempt-uid"))
            .transpose()?;
        let mut transparent_uid = env_value("CHIMERA_TRANSPARENT_RUNTIME_UID")
            .map(|value| parse_u32(&value, "transparent-uid"))
            .transpose()?;
        let mut transparent_gid = env_value("CHIMERA_TRANSPARENT_RUNTIME_GID")
            .map(|value| parse_u32(&value, "transparent-gid"))
            .transpose()?;
        let mut bypass_cidrs_v4 = default_bypass_cidrs_v4();
        let mut capture_cidrs_v4 = Vec::new();
        let mut capture_tcp_ports = Vec::new();
        let mut run_ms = env_value("CHIMERA_TRANSPARENT_RUNTIME_RUN_MS")
            .map(|value| parse_positive_u64(&value, "run-ms"))
            .transpose()?;
        let mut print_only = false;

        let mut index = 0usize;
        while index < args.len() {
            let flag = args[index].as_str();
            match flag {
                "--transparent-bin" => {
                    transparent_bin = arg_value(args, index, flag)?;
                    index += 2;
                }
                "--listen" => {
                    listen = Some(arg_value(args, index, flag)?);
                    index += 2;
                }
                "--gateway-local" => {
                    gateway_local = Some(arg_value(args, index, flag)?);
                    index += 2;
                }
                "--direct-mode" => {
                    direct_mode = arg_value(args, index, flag)?;
                    index += 2;
                }
                "--direct-timeout-ms" => {
                    direct_timeout_ms =
                        parse_positive_u64(&arg_value(args, index, flag)?, "direct-timeout-ms")?;
                    index += 2;
                }
                "--first-response-timeout-ms" => {
                    first_response_timeout_ms = parse_positive_u64(
                        &arg_value(args, index, flag)?,
                        "first-response-timeout-ms",
                    )?;
                    index += 2;
                }
                "--initial-read-timeout-ms" => {
                    initial_read_timeout_ms = parse_positive_u64(
                        &arg_value(args, index, flag)?,
                        "initial-read-timeout-ms",
                    )?;
                    index += 2;
                }
                "--table" => {
                    table_name = arg_value(args, index, flag)?;
                    index += 2;
                }
                "--chain" => {
                    chain_name = arg_value(args, index, flag)?;
                    index += 2;
                }
                "--exempt-uid" => {
                    exempt_uid = Some(parse_u32(&arg_value(args, index, flag)?, "exempt-uid")?);
                    index += 2;
                }
                "--transparent-uid" => {
                    transparent_uid = Some(parse_u32(
                        &arg_value(args, index, flag)?,
                        "transparent-uid",
                    )?);
                    index += 2;
                }
                "--transparent-gid" => {
                    transparent_gid = Some(parse_u32(
                        &arg_value(args, index, flag)?,
                        "transparent-gid",
                    )?);
                    index += 2;
                }
                "--bypass-cidr-v4" => {
                    bypass_cidrs_v4.push(arg_value(args, index, flag)?);
                    index += 2;
                }
                "--capture-cidr-v4" => {
                    capture_cidrs_v4.push(arg_value(args, index, flag)?);
                    index += 2;
                }
                "--capture-tcp-port" => {
                    capture_tcp_ports.push(parse_u16(
                        &arg_value(args, index, flag)?,
                        "capture-tcp-port",
                    )?);
                    index += 2;
                }
                "--no-default-bypass" => {
                    bypass_cidrs_v4.clear();
                    index += 1;
                }
                "--run-ms" => {
                    run_ms = Some(parse_positive_u64(
                        &arg_value(args, index, flag)?,
                        "run-ms",
                    )?);
                    index += 2;
                }
                "--print-only" => {
                    print_only = true;
                    index += 1;
                }
                _ => return Err(format!("unknown argument: {flag}")),
            }
        }

        if direct_mode != "auto" && direct_mode != "disabled" {
            return Err("direct-mode must be auto or disabled".to_string());
        }
        Ok(Self {
            transparent_bin,
            listen: required_value(listen, "missing --listen or CHIMERA_TRANSPARENT_TCP_LISTEN")?,
            gateway_local: required_value(
                gateway_local,
                "missing --gateway-local or CHIMERA_TRANSPARENT_TCP_GATEWAY_LOCAL",
            )?,
            direct_mode,
            direct_timeout_ms,
            first_response_timeout_ms,
            initial_read_timeout_ms,
            table_name,
            chain_name,
            exempt_uid: exempt_uid.or(transparent_uid).ok_or_else(|| {
                "missing --exempt-uid/--transparent-uid or CHIMERA_REDIRECT_EXEMPT_UID".to_string()
            })?,
            transparent_uid,
            transparent_gid,
            bypass_cidrs_v4,
            capture_cidrs_v4,
            capture_tcp_ports,
            run_ms,
            print_only,
        })
    }

    fn plan(&self) -> TransparentRedirectPlan {
        TransparentRedirectPlan {
            table_name: self.table_name.clone(),
            chain_name: self.chain_name.clone(),
            listen_port: parse_listen_port(&self.listen).unwrap_or(0),
            exempt_uid: self.exempt_uid,
            bypass_cidrs_v4: self.bypass_cidrs_v4.clone(),
            capture_cidrs_v4: self.capture_cidrs_v4.clone(),
            capture_tcp_ports: self.capture_tcp_ports.clone(),
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let options = match Options::parse(&args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(2);
        }
    };
    if let Err(error) = run(options) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run(options: Options) -> Result<(), String> {
    let plan = options.plan();
    plan.validate().map_err(|error| error.to_string())?;
    let apply = plan.render_apply_nft().map_err(|error| error.to_string())?;
    let delete = plan
        .render_delete_nft()
        .map_err(|error| error.to_string())?;
    if options.print_only {
        print!("{apply}\n--- cleanup ---\n{delete}");
        return Ok(());
    }

    let stopping = Arc::new(AtomicBool::new(false));
    install_signal_handlers(Arc::clone(&stopping))?;
    run_nft(&apply)?;
    println!(
        "chimera_transparent_runtime=rules_applied table={} listen={} exempt_uid={}",
        options.table_name, options.listen, options.exempt_uid
    );

    let mut child = match spawn_transparent(&options) {
        Ok(child) => child,
        Err(error) => {
            let _ = run_nft(&delete);
            return Err(error);
        }
    };

    let result = supervise_child(&mut child, options.run_ms, Arc::clone(&stopping));
    let _ = child.kill();
    let _ = child.wait();
    let cleanup = run_nft(&delete);
    match (result, cleanup) {
        (Ok(()), Ok(())) => {
            println!("chimera_transparent_runtime=stopped cleanup=ok");
            Ok(())
        }
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(format!("cleanup failed: {error}")),
        (Err(error), Err(cleanup_error)) => {
            Err(format!("{error}; cleanup failed: {cleanup_error}"))
        }
    }
}

fn spawn_transparent(options: &Options) -> Result<Child, String> {
    let mut command = Command::new(&options.transparent_bin);
    command
        .arg("--listen")
        .arg(&options.listen)
        .arg("--gateway-local")
        .arg(&options.gateway_local)
        .arg("--direct-mode")
        .arg(&options.direct_mode)
        .arg("--direct-timeout-ms")
        .arg(options.direct_timeout_ms.to_string())
        .arg("--first-response-timeout-ms")
        .arg(options.first_response_timeout_ms.to_string())
        .arg("--initial-read-timeout-ms")
        .arg(options.initial_read_timeout_ms.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(gid) = options.transparent_gid {
        command.gid(gid);
    }
    if let Some(uid) = options.transparent_uid {
        command.uid(uid);
    }
    command
        .spawn()
        .map_err(|error| format!("spawn transparent tcp failed: {error}"))
}

fn supervise_child(
    child: &mut Child,
    run_ms: Option<u64>,
    stopping: Arc<AtomicBool>,
) -> Result<(), String> {
    let started = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("wait transparent child failed: {error}"))?
        {
            if status.success() {
                return Ok(());
            }
            return Err(format!("transparent child exited: {status}"));
        }
        if stopping.load(Ordering::SeqCst) {
            return Ok(());
        }
        if let Some(limit) = run_ms {
            if started.elapsed() >= Duration::from_millis(limit) {
                return Ok(());
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn run_nft(script: &str) -> Result<(), String> {
    let mut child = Command::new("nft")
        .arg("-f")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("start nft failed: {error}"))?;
    let Some(stdin) = child.stdin.as_mut() else {
        return Err("nft stdin unavailable".to_string());
    };
    stdin
        .write_all(script.as_bytes())
        .map_err(|error| format!("write nft script failed: {error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("wait nft failed: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "nft failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

fn install_signal_handlers(stopping: Arc<AtomicBool>) -> Result<(), String> {
    ctrlc::set_handler(move || {
        stopping.store(true, Ordering::SeqCst);
    })
    .map_err(|error| format!("install signal handler failed: {error}"))
}

fn parse_listen_port(value: &str) -> Result<u16, String> {
    let port = value
        .rsplit_once(':')
        .ok_or_else(|| "listen address must include port".to_string())?
        .1;
    parse_u16(port, "listen port")
}

fn arg_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn env_value(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn required_value(value: Option<String>, error: &str) -> Result<String, String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| error.to_string())
}

fn parse_positive_u64(value: &str, name: &str) -> Result<u64, String> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| format!("{name} must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{name} must be positive"));
    }
    Ok(parsed)
}

fn parse_u16(value: &str, name: &str) -> Result<u16, String> {
    let parsed = value
        .parse::<u16>()
        .map_err(|_| format!("{name} must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{name} must be > 0"));
    }
    Ok(parsed)
}

fn parse_u32(value: &str, name: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|_| format!("{name} must be a non-negative integer"))
}

#[cfg(test)]
mod tests {
    use super::Options;

    #[test]
    fn options_parse_runtime_values() {
        let args = vec![
            "--transparent-bin".to_string(),
            "/tmp/chimera-transparent-tcp".to_string(),
            "--listen".to_string(),
            "0.0.0.0:18144".to_string(),
            "--gateway-local".to_string(),
            "127.0.0.1:18142".to_string(),
            "--direct-mode".to_string(),
            "disabled".to_string(),
            "--exempt-uid".to_string(),
            "65534".to_string(),
            "--transparent-uid".to_string(),
            "65534".to_string(),
            "--transparent-gid".to_string(),
            "65534".to_string(),
            "--capture-cidr-v4".to_string(),
            "203.0.113.10/32".to_string(),
            "--capture-tcp-port".to_string(),
            "443".to_string(),
            "--run-ms".to_string(),
            "500".to_string(),
        ];
        let parsed = Options::parse(&args).unwrap_or_else(|error| {
            unreachable!("options should parse: {error}");
        });
        assert_eq!(parsed.transparent_bin, "/tmp/chimera-transparent-tcp");
        assert_eq!(parsed.listen, "0.0.0.0:18144");
        assert_eq!(parsed.gateway_local, "127.0.0.1:18142");
        assert_eq!(parsed.direct_mode, "disabled");
        assert_eq!(parsed.exempt_uid, 65534);
        assert_eq!(parsed.transparent_uid, Some(65534));
        assert_eq!(parsed.transparent_gid, Some(65534));
        assert_eq!(parsed.capture_cidrs_v4, vec!["203.0.113.10/32"]);
        assert_eq!(parsed.capture_tcp_ports, vec![443]);
        assert_eq!(parsed.run_ms, Some(500));
    }

    #[test]
    fn options_reject_bad_direct_mode() {
        let args = vec![
            "--listen".to_string(),
            "127.0.0.1:18144".to_string(),
            "--gateway-local".to_string(),
            "127.0.0.1:18142".to_string(),
            "--direct-mode".to_string(),
            "bad".to_string(),
            "--exempt-uid".to_string(),
            "65534".to_string(),
        ];
        assert!(Options::parse(&args).is_err());
    }
}
