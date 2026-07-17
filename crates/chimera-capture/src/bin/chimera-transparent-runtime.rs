// chimera-transparent-runtime.rs
//
// CHIMERA transparent local-capture datapath launcher.
// This binary is intentionally small: it owns the OS-level redirect rules,
// resolves capture targets, and hands accepted TCP flows to
// `chimera-transparent-tcp` over a local acceptor.

use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, Write};
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use dns_lookup::{AddrFamily, AddrInfoHints, getaddrinfo};

const DEFAULT_LISTEN: &str = "127.0.0.1:18134";
const DEFAULT_TRANSIT_LOCAL: &str = "127.0.0.1:18135";
const DEFAULT_REDIRECT_TABLE: &str = "chimera_redirect";
const DEFAULT_REDIRECT_CHAIN: &str = "output";
const DEFAULT_SERVICE_FWMARK: u32 = 0x5244;
const DEFAULT_EXEMPT_UID: u32 = 65534;
const DEFAULT_RUNTIME_UID: u32 = 1000;
const DEFAULT_RUNTIME_GID: u32 = 1000;
const DEFAULT_DIRECT_MODE: &str = "disabled";
const DEFAULT_DIRECT_TIMEOUT_MS: u64 = 1200;
const DEFAULT_INITIAL_READ_TIMEOUT_MS: u64 = 500;
const DEFAULT_CAPTURE_TCP_PORTS: &str = "443";

const DEFAULT_BYPASS_CIDR_V4: &[&str] = &[
    "10.0.0.0/8",
    "172.16.0.0/12",
    "192.168.0.0/16",
    "127.0.0.0/8",
    "169.254.0.0/16",
    "224.0.0.0/4",
    "240.0.0.0/4",
];

fn main() {
    if let Err(error) = run() {
        eprintln!("transparent-runtime fatal: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let options = Options::parse();
    setup_signal_handler();

    let (listen_addr, transit_local) = parse_local_endpoints(&options)?;

    let ports = parse_ports(&options.capture_tcp_ports)?;
    let capture_cidrs = build_capture_cidrs(&options)?;

    let redirect_table = options.redirect_table.clone();
    let redirect_chain = options.redirect_chain.clone();
    let service_fwmark = options.service_fwmark;
    let exempt_uid = options.exempt_uid;
    let runtime_uid = options.runtime_uid;
    let runtime_gid = options.runtime_gid;
    let transparent_bin = options.transparent_bin.clone();
    let use_sudo = options.use_sudo;
    let privilege_mode = options.privilege_mode.clone();

    install_redirect_rules(
        &redirect_table,
        &redirect_chain,
        service_fwmark,
        exempt_uid,
        &capture_cidrs,
        &ports,
        use_sudo,
        &privilege_mode,
    )?;

    let cleanup_on_drop = CleanupState {
        redirect_table,
        redirect_chain,
        use_sudo,
        privilege_mode,
    };

    let child = spawn_transparent_tcp(
        &transparent_bin,
        listen_addr,
        transit_local,
        &options,
        runtime_uid,
        runtime_gid,
    )?;

    let mut child = child;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let _ = cleanup_redirect_rules(&cleanup_on_drop);
                return Err(format!(
                    "transparent-tcp exited early with status: {status}"
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(500)),
            Err(error) => {
                let _ = cleanup_redirect_rules(&cleanup_on_drop);
                return Err(format!("failed to wait on transparent-tcp: {error}"));
            }
        }
    }
}

struct Options {
    transparent_bin: String,
    listen: String,
    transit_local: String,
    redirect_table: String,
    redirect_chain: String,
    service_fwmark: u32,
    exempt_uid: u32,
    runtime_uid: u32,
    runtime_gid: u32,
    use_sudo: bool,
    privilege_mode: String,
    capture_domain: Vec<String>,
    capture_cidr_v4: Vec<String>,
    capture_tcp_ports: String,
    capture_domains_file: Option<String>,
    bypass_cidr_v4: Vec<String>,
    no_default_bypass: bool,
    direct_mode: String,
    direct_timeout_ms: u64,
    initial_read_timeout_ms: u64,
}

impl Options {
    fn parse() -> Self {
        let env_or = |key: &str, default: &str| -> String {
            std::env::var(key).unwrap_or_else(|_| default.to_owned())
        };
        let env_flag = |key: &str, default: bool| -> bool {
            std::env::var(key)
                .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
                .unwrap_or(default)
        };
        let env_u32 = |key: &str, default: u32| -> u32 {
            std::env::var(key)
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(default)
        };
        let env_u64 = |key: &str, default: u64| -> u64 {
            std::env::var(key)
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(default)
        };

        let mut capture_domain = Vec::new();
        if let Ok(value) = std::env::var("CHIMERA_CAPTURE_DOMAIN") {
            for token in value.split(',') {
                let token = token.trim();
                if !token.is_empty() {
                    capture_domain.push(token.to_lowercase());
                }
            }
        }

        let capture_cidr_v4 = std::env::var("CHIMERA_CAPTURE_CIDR_V4")
            .map(|value| {
                value
                    .split(',')
                    .map(|token| token.trim().to_owned())
                    .filter(|token| !token.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let bypass_cidr_v4 = std::env::var("CHIMERA_BYPASS_CIDR_V4")
            .map(|value| {
                value
                    .split(',')
                    .map(|token| token.trim().to_owned())
                    .filter(|token| !token.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        Self {
            transparent_bin: env_or(
                "CHIMERA_TRANSPARENT_BIN",
                "/usr/local/bin/chimera-transparent-tcp",
            ),
            listen: env_or("CHIMERA_TRANSPARENT_TCP_LISTEN", DEFAULT_LISTEN),
            transit_local: env_or(
                "CHIMERA_TRANSPARENT_TCP_TRANSIT_LOCAL",
                DEFAULT_TRANSIT_LOCAL,
            ),
            redirect_table: env_or("CHIMERA_REDIRECT_TABLE", DEFAULT_REDIRECT_TABLE),
            redirect_chain: env_or("CHIMERA_REDIRECT_CHAIN", DEFAULT_REDIRECT_CHAIN),
            service_fwmark: env_u32("CHIMERA_REDIRECT_SERVICE_FWMARK", DEFAULT_SERVICE_FWMARK),
            exempt_uid: env_u32("CHIMERA_REDIRECT_EXEMPT_UID", DEFAULT_EXEMPT_UID),
            runtime_uid: env_u32("CHIMERA_TRANSPARENT_RUNTIME_UID", DEFAULT_RUNTIME_UID),
            runtime_gid: env_u32("CHIMERA_TRANSPARENT_RUNTIME_GID", DEFAULT_RUNTIME_GID),
            use_sudo: env_flag("CHIMERA_RUNNER_USE_SUDO", false),
            privilege_mode: env_or("CHIMERA_NFT_PRIVILEGE_MODE", "none"),
            capture_domain,
            capture_cidr_v4,
            capture_tcp_ports: env_or("CHIMERA_CAPTURE_TCP_PORTS", DEFAULT_CAPTURE_TCP_PORTS),
            capture_domains_file: std::env::var("CHIMERA_CAPTURE_DOMAINS_FILE")
                .ok()
                .filter(|path| !path.is_empty()),
            bypass_cidr_v4,
            no_default_bypass: env_flag("CHIMERA_NO_DEFAULT_BYPASS", false),
            direct_mode: env_or("CHIMERA_TRANSPARENT_TCP_DIRECT_MODE", DEFAULT_DIRECT_MODE),
            direct_timeout_ms: env_u64(
                "CHIMERA_TRANSPARENT_TCP_DIRECT_TIMEOUT_MS",
                DEFAULT_DIRECT_TIMEOUT_MS,
            ),
            initial_read_timeout_ms: env_u64(
                "CHIMERA_TRANSPARENT_TCP_INITIAL_READ_TIMEOUT_MS",
                DEFAULT_INITIAL_READ_TIMEOUT_MS,
            ),
        }
    }
}

fn parse_local_endpoints(options: &Options) -> Result<(SocketAddr, SocketAddr), String> {
    let listen_addr: SocketAddr = options
        .listen
        .parse()
        .map_err(|error| format!("invalid listen address '{}': {error}", options.listen))?;
    if !listen_addr.ip().is_loopback() {
        return Err(format!(
            "listen address '{listen_addr}' must be loopback for safety"
        ));
    }
    let transit_local: SocketAddr = options.transit_local.parse().map_err(|error| {
        format!(
            "invalid transit-local address '{}': {error}",
            options.transit_local
        )
    })?;
    Ok((listen_addr, transit_local))
}

fn parse_ports(raw: &str) -> Result<Vec<u16>, String> {
    let mut ports = Vec::new();
    for token in raw.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let port: u16 = token
            .parse()
            .map_err(|error| format!("invalid TCP port '{token}': {error}"))?;
        ports.push(port);
    }
    if ports.is_empty() {
        return Err("at least one capture TCP port is required".to_owned());
    }
    Ok(ports)
}

fn build_capture_cidrs(options: &Options) -> Result<Vec<String>, String> {
    let mut cidrs: BTreeSet<String> = BTreeSet::new();

    if !options.no_default_bypass {
        for cidr in DEFAULT_BYPASS_CIDR_V4 {
            cidrs.insert((*cidr).to_owned());
        }
    }

    for cidr in &options.bypass_cidr_v4 {
        if !cidr.is_empty() {
            cidrs.insert(cidr.clone());
        }
    }

    for cidr in &options.capture_cidr_v4 {
        cidrs.insert(cidr.clone());
    }

    let mut domains: Vec<String> = options.capture_domain.clone();

    if let Some(path) = &options.capture_domains_file {
        let file_domains = read_capture_domains_file(Path::new(path))
            .map_err(|error| format!("failed to read capture domains file '{}': {error}", path))?;
        domains.extend(file_domains);
    }

    if domains.is_empty() && options.capture_cidr_v4.is_empty() {
        return Err(
            "no capture domains or CIDRs specified; set CHIMERA_CAPTURE_DOMAIN, CHIMERA_CAPTURE_CIDR_V4, or CHIMERA_CAPTURE_DOMAINS_FILE"
                .to_owned(),
        );
    }

    let resolved = resolve_domains_to_cidrs(&domains)?;
    for cidr in resolved {
        cidrs.insert(cidr);
    }

    Ok(cidrs.into_iter().collect())
}

fn read_capture_domains_file(path: &Path) -> Result<Vec<String>, std::io::Error> {
    let file = fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut domains = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        for token in line.split(|c: char| c.is_whitespace() || c == ',') {
            let token = token.trim();
            if !token.is_empty() {
                domains.push(token.to_lowercase());
            }
        }
    }
    Ok(domains)
}

fn resolve_domains_to_cidrs(domains: &[String]) -> Result<Vec<String>, String> {
    let mut cidrs = Vec::new();
    for domain in domains {
        let resolved = resolve_domain_ipv4(domain)
            .map_err(|error| format!("failed to resolve domain '{domain}': {error}"))?;
        if resolved.is_empty() {
            return Err(format!(
                "domain '{domain}' did not resolve to any IPv4 address"
            ));
        }
        for ip in resolved {
            cidrs.push(format!("{}/32", ip));
        }
    }
    Ok(cidrs)
}

fn resolve_domain_ipv4(domain: &str) -> Result<Vec<Ipv4Addr>, String> {
    let hints = AddrInfoHints {
        address: AddrFamily::Inet.into(),
        socktype: dns_lookup::SockType::Stream.into(),
        ..AddrInfoHints::default()
    };

    let iterator = getaddrinfo(Some(domain), None, Some(hints))
        .map_err(|error| format!("dns lookup failed: {error:?}"))?;

    let mut ipv4s = Vec::new();
    for item in iterator {
        let addr_info = item.map_err(|error| format!("dns lookup result failed: {error:?}"))?;
        if let SocketAddr::V4(socket_addr_v4) = addr_info.sockaddr {
            ipv4s.push(*socket_addr_v4.ip());
        }
    }

    Ok(ipv4s)
}

fn install_redirect_rules(
    table: &str,
    chain: &str,
    service_fwmark: u32,
    exempt_uid: u32,
    capture_cidrs: &[String],
    ports: &[u16],
    use_sudo: bool,
    privilege_mode: &str,
) -> Result<(), String> {
    cleanup_redirect_rules(&CleanupState {
        redirect_table: table.to_owned(),
        redirect_chain: chain.to_owned(),
        use_sudo,
        privilege_mode: privilege_mode.to_owned(),
    })?;

    let mut commands = String::new();
    commands.push_str(&format!(
        "add table ip {table}\nadd chain ip {table} {chain} {{ type nat hook output priority 0; policy accept; }}\n"
    ));

    for cidr in capture_cidrs {
        if is_default_bypass_cidr(cidr) {
            commands.push_str(&format!(
                "add rule ip {table} {chain} ip daddr {cidr} return comment \"RFC1918/loopback bypass\"\n"
            ));
        }
    }

    commands.push_str(&format!(
        "add rule ip {table} {chain} meta mark 0x{service_fwmark:x} return comment \"service self-exempt\"\n"
    ));
    commands.push_str(&format!(
        "add rule ip {table} {chain} meta skuid {exempt_uid} return comment \"runtime UID exempt\"\n"
    ));

    let capture_cidrs_iter = capture_cidrs
        .iter()
        .filter(|cidr| !is_default_bypass_cidr(cidr));
    let capture_cidrs_list: Vec<_> = capture_cidrs_iter.clone().collect();

    if capture_cidrs_list.is_empty() && ports.is_empty() {
        return Err("no capture CIDRs or ports to redirect; aborting rule installation".to_owned());
    }

    let ports_expr = ports
        .iter()
        .map(|port| port.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    if capture_cidrs_list.is_empty() {
        commands.push_str(&format!(
            "add rule ip {table} {chain} tcp dport {{ {ports_expr} }} redirect to :18134 comment \"CHIMERA capture all ports\"\n"
        ));
    } else {
        let cidrs_expr = capture_cidrs_list
            .iter()
            .map(|cidr| (*cidr).clone())
            .collect::<Vec<_>>()
            .join(", ");
        commands.push_str(&format!(
            "add rule ip {table} {chain} ip daddr {{ {cidrs_expr} }} tcp dport {{ {ports_expr} }} redirect to :18134 comment \"CHIMERA domain capture\"\n"
        ));
    }

    let mut child = create_privileged_child("nft", &["-f", "-"], use_sudo, privilege_mode, false)?;
    {
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "failed to open nft stdin".to_owned())?;
        let mut stdin = std::io::BufWriter::new(stdin);
        stdin
            .write_all(commands.as_bytes())
            .map_err(|error| format!("failed to write nft commands: {error}"))?;
        stdin
            .flush()
            .map_err(|error| format!("failed to flush nft stdin: {error}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|error| format!("nft command failed: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("nft failed:\n{stderr}"));
    }

    eprintln!(
        "transparent-runtime: installed nft rules in {}:{}",
        table, chain
    );
    Ok(())
}

fn is_default_bypass_cidr(cidr: &str) -> bool {
    DEFAULT_BYPASS_CIDR_V4
        .iter()
        .any(|default| *default == cidr)
}

#[derive(Clone)]
struct CleanupState {
    redirect_table: String,
    redirect_chain: String,
    use_sudo: bool,
    privilege_mode: String,
}

fn cleanup_redirect_rules(state: &CleanupState) -> Result<(), String> {
    let table = &state.redirect_table;
    let chain = &state.redirect_chain;
    let commands = format!("delete chain ip {table} {chain}\ndelete table ip {table}\n");

    let mut child = create_privileged_child(
        "nft",
        &["-f", "-"],
        state.use_sudo,
        &state.privilege_mode,
        false,
    )?;
    {
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "failed to open nft stdin".to_owned())?;
        let mut stdin = std::io::BufWriter::new(stdin);
        let _ = stdin.write_all(commands.as_bytes());
        let _ = stdin.flush();
    }

    let _ = child.wait_with_output();
    Ok(())
}

fn create_privileged_child(
    program: &str,
    args: &[&str],
    use_sudo: bool,
    privilege_mode: &str,
    preserve_env: bool,
) -> Result<std::process::Child, String> {
    let effective_use_sudo = use_sudo || privilege_mode.eq_ignore_ascii_case("sudo");

    if effective_use_sudo {
        let mut cmd = Command::new("sudo");
        if preserve_env {
            cmd.arg("-E");
        }
        cmd.arg(program);
        cmd.args(args);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd.spawn()
            .map_err(|error| format!("failed to spawn sudo {program}: {error}"))
    } else if privilege_mode.eq_ignore_ascii_case("pkexec") {
        let mut cmd = Command::new("pkexec");
        cmd.arg(program);
        cmd.args(args);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd.spawn()
            .map_err(|error| format!("failed to spawn pkexec {program}: {error}"))
    } else {
        let mut cmd = Command::new(program);
        cmd.args(args);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd.spawn()
            .map_err(|error| format!("failed to spawn {program}: {error}"))
    }
}

fn spawn_transparent_tcp(
    transparent_bin: &str,
    listen_addr: SocketAddr,
    transit_local: SocketAddr,
    options: &Options,
    runtime_uid: u32,
    runtime_gid: u32,
) -> Result<std::process::Child, String> {
    if !Path::new(transparent_bin).is_file() {
        return Err(format!(
            "transparent-tcp binary not found: {transparent_bin}"
        ));
    }

    let mut cmd = Command::new(transparent_bin);
    cmd.arg("--listen").arg(listen_addr.to_string());
    cmd.arg("--transit-local").arg(transit_local.to_string());
    if options.direct_mode != "disabled" {
        cmd.arg("--direct-mode").arg(&options.direct_mode);
        if options.direct_timeout_ms > 0 {
            cmd.arg("--direct-timeout-ms")
                .arg(options.direct_timeout_ms.to_string());
        }
    }
    if options.initial_read_timeout_ms > 0 {
        cmd.arg("--initial-read-timeout-ms")
            .arg(options.initial_read_timeout_ms.to_string());
    }

    cmd.env(
        "CHIMERA_SERVICE_FWMARK",
        format!("{}", options.service_fwmark),
    );

    if runtime_uid == 0 || runtime_gid == 0 {
        return Err(
            "refusing to run transparent-tcp as root (runtime_uid/runtime_gid must be non-zero)"
                .to_owned(),
        );
    }

    cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    cmd.spawn()
        .map_err(|error| format!("failed to spawn transparent-tcp: {error}"))
}

fn setup_signal_handler() {
    let _ = ctrlc::set_handler(move || {
        eprintln!("transparent-runtime: caught interrupt, exiting");
        std::process::exit(0);
    });
}
