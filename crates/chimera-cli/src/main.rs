#![forbid(unsafe_code)]

use std::net::IpAddr;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;
use std::time::{Duration, Instant};

use chimera_capture::{
    CaptureMode, CapturePlan, DatapathRoute, TransparentFailoverConfig, TransparentFailoverEngine,
    detect_tun_support, plan_capture_mode,
};
use chimera_carrier::CarrierEndpoint;
use chimera_carrier_quic::{QuicCarrier, QuicCarrierConfig};
use chimera_carrier_tls::{TlsCarrier, TlsCarrierConfig};
use chimera_config::{ConfigCaptureMode, ConfigCarrierProfile, parse_node_config_text};
use chimera_dns::{DnsBinding, DnsBindingStore};
use chimera_policy::{
    FlowContext, OutboundMode, Policy, PolicySummary, Protocol, RouteDecision, RouteExplainTrace,
    RouteRule, RuleMatcher,
};
use chimera_session::{RekeyPolicy, RekeyReason, RekeyState};

mod diagnostic_redaction;
mod mesh_cli;

fn route_explain_dns_binding_ttl() -> Duration {
    const DEFAULT_TTL_SECONDS: u64 = 60;
    const MAX_TTL_SECONDS: u64 = 86_400;
    match std::env::var("CHIMERA_ROUTE_EXPLAIN_DNS_BINDING_TTL_SECONDS") {
        Ok(value) => value
            .parse::<u64>()
            .ok()
            .filter(|&n| n > 0 && n <= MAX_TTL_SECONDS)
            .map(Duration::from_secs)
            .unwrap_or_else(|| {
                eprintln!("warning: invalid CHIMERA_ROUTE_EXPLAIN_DNS_BINDING_TTL_SECONDS='{value}', using default {DEFAULT_TTL_SECONDS}");
                Duration::from_secs(DEFAULT_TTL_SECONDS)
            }),
        Err(_) => Duration::from_secs(DEFAULT_TTL_SECONDS),
    }
}

fn route_explain_failover_ttl_ticks() -> u32 {
    const DEFAULT_TICKS: u32 = 10;
    const MAX_TICKS: u32 = 10_000;
    match std::env::var("CHIMERA_ROUTE_EXPLAIN_FAILOVER_TTL_TICKS") {
        Ok(value) => value
            .parse::<u32>()
            .ok()
            .filter(|&n| n > 0 && n <= MAX_TICKS)
            .unwrap_or_else(|| {
                eprintln!("warning: invalid CHIMERA_ROUTE_EXPLAIN_FAILOVER_TTL_TICKS='{value}', using default {DEFAULT_TICKS}");
                DEFAULT_TICKS
            }),
        Err(_) => DEFAULT_TICKS,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Language {
    En,
    Ru,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LanguageSource {
    Flag,
    Default,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StatusOptions {
    config_path: Option<String>,
    mock_packets: u64,
    mock_age_seconds: u64,
    max_age_seconds: u64,
    max_packets_per_key: u64,
    capture_preference: CapturePreference,
    tun_supported: bool,
    carrier_profile: CarrierProfile,
    carrier_addr: String,
    carrier_server_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CapturePreference {
    Auto,
    Tun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CarrierProfile {
    InMemory,
    Tls,
    Quic,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct RouteExplainOptions {
    config_path: Option<String>,
    domain_override: Option<String>,
    policy_path: Option<String>,
    destination_ip: Option<IpAddr>,
    protocol: Option<Protocol>,
    port: Option<u16>,
    dns_bind_domain: Option<String>,
    dns_bind_ip: Option<IpAddr>,
    show_all_matches: bool,
    json_output: bool,
    out_path: Option<String>,
}

const LEGACY_RUNTIME_FAILOVER_OVERRIDES_PATH: &str = "configs/failover_overrides.txt";
const RUNTIME_FAILOVER_OVERRIDES_FILE: &str = "failover_overrides.txt";
const DEFAULT_CARRIER_ADDR: &str = "127.0.0.1:443";
const DEFAULT_CARRIER_SERVER_NAME: &str = "node.example.org";

#[derive(Debug, Clone, PartialEq, Eq)]
struct UpDownOptions {
    state_path: String,
    config_path: Option<String>,
    skip_connect_check: bool,
    apply_tun: bool,
    tun_name: String,
    tun_local_cidr: String,
    tun_peer_cidr: String,
    apply_route: bool,
    route_cidr: String,
    route_policy: bool,
    route_table: String,
    route_rule_priority: String,
    route_fwmark: Option<u32>,
    apply_dns: bool,
    dns_server: String,
    resolv_conf_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RollbackOptions {
    up_down: UpDownOptions,
    json_output: bool,
    out_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StateProofOptions {
    state_path: String,
    flow_path: Option<String>,
    require_flow: bool,
    max_flow_age_sec: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProbeOptions {
    urls: Vec<String>,
    url_file: Option<String>,
    timeout_seconds: u64,
    apply_policy_path: Option<String>,
    rule_id_prefix: String,
    fail_threshold: usize,
    json_output: bool,
    out_path: Option<String>,
}

const STATUS_USAGE_EN: &str = "usage: chimera [--lang en|ru] status [--config <node_config_file>] [--mock-traffic <packets> --age <seconds> --max-age <seconds> --max-packets <count>] [--capture <auto|tun>] [--tun-supported <true|false>] [--carrier <in-memory|tls|quic>] [--carrier-addr <host:port>] [--server-name <name>]";
const STATUS_USAGE_RU: &str = "использование: chimera [--lang en|ru] status [--config <файл_node_config>] [--mock-traffic <пакеты> --age <секунды> --max-age <секунды> --max-packets <число>] [--capture <auto|tun>] [--tun-supported <true|false>] [--carrier <in-memory|tls|quic>] [--carrier-addr <хост:порт>] [--server-name <имя>]";
const HEALTH_USAGE_EN: &str = "usage: chimera [--lang en|ru] health [--config <node_config_file>] [--capture <auto|tun>] [--tun-supported <true|false>] [--carrier <in-memory|tls|quic>] [--carrier-addr <host:port>] [--server-name <name>]";
const HEALTH_USAGE_RU: &str = "использование: chimera [--lang en|ru] health [--config <файл_node_config>] [--capture <auto|tun>] [--tun-supported <true|false>] [--carrier <in-memory|tls|quic>] [--carrier-addr <хост:порт>] [--server-name <имя>]";
const DOCTOR_USAGE_EN: &str = "usage: chimera [--lang en|ru] doctor [--config <node_config_file>] [--mock-traffic <packets> --age <seconds> --max-age <seconds> --max-packets <count>] [--capture <auto|tun>] [--tun-supported <true|false>] [--carrier <in-memory|tls|quic>] [--carrier-addr <host:port>] [--server-name <name>] [--json] [--out <file>]";
const DOCTOR_USAGE_RU: &str = "использование: chimera [--lang en|ru] doctor [--config <файл_node_config>] [--mock-traffic <пакеты> --age <секунды> --max-age <секунды> --max-packets <число>] [--capture <auto|tun>] [--tun-supported <true|false>] [--carrier <in-memory|tls|quic>] [--carrier-addr <хост:порт>] [--server-name <имя>] [--json] [--out <файл>]";
const ROUTE_USAGE_EN: &str = "usage: chimera [--lang en|ru] route explain [domain] [--domain <domain>] [--policy <policy_file>] [--ip <ipv4|ipv6>] [--proto <tcp|udp|icmp>] [--port <n>] [--dns-bind-domain <domain>] [--dns-bind-ip <ipv4|ipv6>] [--show-all-matches] [--json] [--out <file>]";
const ROUTE_USAGE_RU: &str = "использование: chimera [--lang en|ru] route explain [домен] [--domain <домен>] [--policy <файл_policy>] [--ip <ipv4|ipv6>] [--proto <tcp|udp|icmp>] [--port <число>] [--dns-bind-domain <домен>] [--dns-bind-ip <ipv4|ipv6>] [--show-all-matches] [--json] [--out <файл>]";
const MESH_USAGE_EN: &str = "usage: chimera [--lang en|ru] mesh <nodes|contracts|route-explain|connect-probe|launch-preflight|launch-preflight-verify> ...";
const MESH_USAGE_RU: &str = "использование: chimera [--lang en|ru] mesh <nodes|contracts|route-explain|connect-probe|launch-preflight|launch-preflight-verify> ...";
const DIAG_REKEY_USAGE_EN: &str =
    "usage: chimera [--lang en|ru] diag rekey <age_sec> <packets_sent>";
const DIAG_REKEY_USAGE_RU: &str =
    "использование: chimera [--lang en|ru] diag rekey <секунды_возраста> <пакеты>";
const DIAG_EXPORT_USAGE_EN: &str = "usage: chimera [--lang en|ru] diag export [--config <node_config_file>] [--age <seconds>] [--packets <count>] [--out <file>]";
const DIAG_EXPORT_USAGE_RU: &str = "использование: chimera [--lang en|ru] diag export [--config <файл_node_config>] [--age <секунды>] [--packets <число>] [--out <файл>]";
const POLICY_USAGE_EN: &str = "usage: chimera [--lang en|ru] policy validate <policy_file>";
const POLICY_USAGE_RU: &str = "использование: chimera [--lang en|ru] policy validate <файл_policy>";
const UP_USAGE_EN: &str = "usage: chimera [--lang en|ru] up [--state-file <file>] [--config <node_config_file>] [--skip-connect-check <true|false>] [--apply-tun <true|false>] [--tun-name <name>] [--tun-local-cidr <cidr>] [--tun-peer-cidr <cidr>] [--apply-route <true|false>] [--route-cidr <cidr[,cidr2,...]>] [--route-policy <true|false>] [--route-table <id>] [--route-rule-priority <pref>] [--route-fwmark <mark>] [--apply-dns <true|false>] [--dns-server <ip>] [--resolv-conf <path>]";
const UP_USAGE_RU: &str = "использование: chimera [--lang en|ru] up [--state-file <файл>] [--config <файл_node_config>] [--skip-connect-check <true|false>] [--apply-tun <true|false>] [--tun-name <имя>] [--tun-local-cidr <cidr>] [--tun-peer-cidr <cidr>] [--apply-route <true|false>] [--route-cidr <cidr[,cidr2,...]>] [--route-policy <true|false>] [--route-table <id>] [--route-rule-priority <pref>] [--route-fwmark <mark>] [--apply-dns <true|false>] [--dns-server <ip>] [--resolv-conf <путь>]";
const DOWN_USAGE_EN: &str = "usage: chimera [--lang en|ru] down [--state-file <file>] [--config <node_config_file>] [--skip-connect-check <true|false>] [--apply-tun <true|false>] [--tun-name <name>] [--tun-local-cidr <cidr>] [--tun-peer-cidr <cidr>] [--apply-route <true|false>] [--route-cidr <cidr[,cidr2,...]>] [--route-policy <true|false>] [--route-table <id>] [--route-rule-priority <pref>] [--route-fwmark <mark>] [--apply-dns <true|false>] [--dns-server <ip>] [--resolv-conf <path>]";
const DOWN_USAGE_RU: &str = "использование: chimera [--lang en|ru] down [--state-file <файл>] [--config <файл_node_config>] [--skip-connect-check <true|false>] [--apply-tun <true|false>] [--tun-name <имя>] [--tun-local-cidr <cidr>] [--tun-peer-cidr <cidr>] [--apply-route <true|false>] [--route-cidr <cidr[,cidr2,...]>] [--route-policy <true|false>] [--route-table <id>] [--route-rule-priority <pref>] [--route-fwmark <mark>] [--apply-dns <true|false>] [--dns-server <ip>] [--resolv-conf <путь>]";
const ROLLBACK_USAGE_EN: &str = "usage: chimera [--lang en|ru] rollback <status|clean|recover> [--state-file <file>] [--json] [--out <file>]";
const ROLLBACK_USAGE_RU: &str = "использование: chimera [--lang en|ru] rollback <status|clean|recover> [--state-file <файл>] [--json] [--out <файл>]";
const STATE_USAGE_EN: &str = "usage: chimera [--lang en|ru] state proof [--state-file <file>] [--flow-file <file>] [--require-flow <true|false>] [--max-flow-age-sec <seconds>]";
const STATE_USAGE_RU: &str = "использование: chimera [--lang en|ru] state proof [--state-file <файл>] [--flow-file <файл>] [--require-flow <true|false>] [--max-flow-age-sec <секунды>]";
const PROBE_USAGE_EN: &str = "usage: chimera [--lang en|ru] probe access --url <http|https_url> [--url-file <file>] [--timeout-sec <n>] [--apply-policy <file>] [--rule-id-prefix <prefix>] [--fail-threshold <n>] [--json] [--out <file>]";
const PROBE_USAGE_RU: &str = "использование: chimera [--lang en|ru] probe access --url <http|https_url> [--url-file <файл>] [--timeout-sec <n>] [--apply-policy <файл>] [--rule-id-prefix <префикс>] [--fail-threshold <n>] [--json] [--out <файл>]";
const PROBE_CI_SNAPSHOT_HOST: &str = "chimera-ci-snapshot.local";
const LAB_USAGE_EN: &str = "usage: chimera [--lang en|ru] lab <smoke|doctor|config-smoke|fuzz-smoke|perf-smoke|net-sim|benchmark-report|benchmark-regression-check|hardening-smoke|mvp-spec-check|mvp-spec-report|m5-artifacts-report|m6-artifacts-report|release-readiness-report|report-pack|artifact-audit|mvp-snapshot|mvp-verify|mvp-check> [extra args for chimera-lab]";
const LAB_USAGE_RU: &str = "использование: chimera [--lang en|ru] lab <smoke|doctor|config-smoke|fuzz-smoke|perf-smoke|net-sim|benchmark-report|benchmark-regression-check|hardening-smoke|mvp-spec-check|mvp-spec-report|m5-artifacts-report|m6-artifacts-report|release-readiness-report|report-pack|artifact-audit|mvp-snapshot|mvp-verify|mvp-check> [доп. аргументы для chimera-lab]";

fn status_usage(lang: Language) -> &'static str {
    match lang {
        Language::En => STATUS_USAGE_EN,
        Language::Ru => STATUS_USAGE_RU,
    }
}

fn route_usage(lang: Language) -> &'static str {
    match lang {
        Language::En => ROUTE_USAGE_EN,
        Language::Ru => ROUTE_USAGE_RU,
    }
}

fn mesh_usage(lang: Language) -> &'static str {
    match lang {
        Language::En => MESH_USAGE_EN,
        Language::Ru => MESH_USAGE_RU,
    }
}

fn health_usage(lang: Language) -> &'static str {
    match lang {
        Language::En => HEALTH_USAGE_EN,
        Language::Ru => HEALTH_USAGE_RU,
    }
}

fn doctor_usage(lang: Language) -> &'static str {
    match lang {
        Language::En => DOCTOR_USAGE_EN,
        Language::Ru => DOCTOR_USAGE_RU,
    }
}

fn diag_rekey_usage(lang: Language) -> &'static str {
    match lang {
        Language::En => DIAG_REKEY_USAGE_EN,
        Language::Ru => DIAG_REKEY_USAGE_RU,
    }
}

fn diag_export_usage(lang: Language) -> &'static str {
    match lang {
        Language::En => DIAG_EXPORT_USAGE_EN,
        Language::Ru => DIAG_EXPORT_USAGE_RU,
    }
}

fn policy_usage(lang: Language) -> &'static str {
    match lang {
        Language::En => POLICY_USAGE_EN,
        Language::Ru => POLICY_USAGE_RU,
    }
}

fn up_usage(lang: Language) -> &'static str {
    match lang {
        Language::En => UP_USAGE_EN,
        Language::Ru => UP_USAGE_RU,
    }
}

fn down_usage(lang: Language) -> &'static str {
    match lang {
        Language::En => DOWN_USAGE_EN,
        Language::Ru => DOWN_USAGE_RU,
    }
}

fn rollback_usage(lang: Language) -> &'static str {
    match lang {
        Language::En => ROLLBACK_USAGE_EN,
        Language::Ru => ROLLBACK_USAGE_RU,
    }
}

fn state_usage(lang: Language) -> &'static str {
    match lang {
        Language::En => STATE_USAGE_EN,
        Language::Ru => STATE_USAGE_RU,
    }
}

fn probe_usage(lang: Language) -> &'static str {
    match lang {
        Language::En => PROBE_USAGE_EN,
        Language::Ru => PROBE_USAGE_RU,
    }
}

fn lab_usage(lang: Language) -> &'static str {
    match lang {
        Language::En => LAB_USAGE_EN,
        Language::Ru => LAB_USAGE_RU,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (lang, lang_source, command_index) = match parse_language_flag(&args) {
        Some(parsed) => parsed,
        None => {
            eprintln!("Ошибка языка. Используйте: --lang en или --lang ru.");
            std::process::exit(2);
        }
    };
    let command = args
        .get(command_index)
        .map(String::as_str)
        .unwrap_or("help");

    let exit_code = match command {
        "status" => status_command(lang, lang_source, args_tail(&args, command_index + 1)),
        "health" => health_command(lang, args_tail(&args, command_index + 1)),
        "doctor" => doctor_command(lang, args_tail(&args, command_index + 1)),
        "mvp-verify" => lab_command(
            lang,
            Some("mvp-verify"),
            args_tail(&args, command_index + 1),
        ),
        "mvp-snapshot" => lab_command(
            lang,
            Some("mvp-snapshot"),
            args_tail(&args, command_index + 1),
        ),
        "mvp-spec-check" => lab_command(
            lang,
            Some("mvp-spec-check"),
            args_tail(&args, command_index + 1),
        ),
        "mvp-spec-report" => lab_command(
            lang,
            Some("mvp-spec-report"),
            args_tail(&args, command_index + 1),
        ),
        "m5-artifacts-report" => lab_command(
            lang,
            Some("m5-artifacts-report"),
            args_tail(&args, command_index + 1),
        ),
        "m6-artifacts-report" => lab_command(
            lang,
            Some("m6-artifacts-report"),
            args_tail(&args, command_index + 1),
        ),
        "lab-smoke" => lab_command(lang, Some("smoke"), args_tail(&args, command_index + 1)),
        "lab-doctor" => lab_command(lang, Some("doctor"), args_tail(&args, command_index + 1)),
        "lab-hardening-smoke" => lab_command(
            lang,
            Some("hardening-smoke"),
            args_tail(&args, command_index + 1),
        ),
        "benchmark-report" => lab_command(
            lang,
            Some("benchmark-report"),
            args_tail(&args, command_index + 1),
        ),
        "hardening-smoke" => lab_command(
            lang,
            Some("hardening-smoke"),
            args_tail(&args, command_index + 1),
        ),
        "benchmark-regression-check" => lab_command(
            lang,
            Some("benchmark-regression-check"),
            args_tail(&args, command_index + 1),
        ),
        "net-sim" => lab_command(lang, Some("net-sim"), args_tail(&args, command_index + 1)),
        "perf-smoke" => lab_command(
            lang,
            Some("perf-smoke"),
            args_tail(&args, command_index + 1),
        ),
        "fuzz-smoke" => lab_command(
            lang,
            Some("fuzz-smoke"),
            args_tail(&args, command_index + 1),
        ),
        "config-smoke" => lab_command(
            lang,
            Some("config-smoke"),
            args_tail(&args, command_index + 1),
        ),
        "release-readiness-report" => lab_command(
            lang,
            Some("release-readiness-report"),
            args_tail(&args, command_index + 1),
        ),
        "artifact-audit" => lab_command(
            lang,
            Some("artifact-audit"),
            args_tail(&args, command_index + 1),
        ),
        "report-pack" => lab_command(
            lang,
            Some("report-pack"),
            args_tail(&args, command_index + 1),
        ),
        "up" => up_command(lang, args_tail(&args, command_index + 1)),
        "down" => down_command(lang, args_tail(&args, command_index + 1)),
        "rollback" => rollback_command(lang, args_tail(&args, command_index + 1)),
        "state" => state_command(
            lang,
            args.get(command_index + 1).map(String::as_str),
            args_tail(&args, command_index + 2),
        ),
        "probe" => probe_command(
            lang,
            args.get(command_index + 1).map(String::as_str),
            args_tail(&args, command_index + 2),
        ),
        "lab" => lab_command(
            lang,
            args.get(command_index + 1).map(String::as_str),
            args_tail(&args, command_index + 2),
        ),
        "mvp-check" => lab_command(lang, Some("mvp-check"), args_tail(&args, command_index + 1)),
        "route" => route_command(
            lang,
            args.get(command_index + 1).map(String::as_str),
            args.get(command_index + 2).map(String::as_str),
            args_tail(&args, command_index + 3),
        ),
        "mesh" => mesh_command(
            lang,
            args.get(command_index + 1).map(String::as_str),
            args_tail(&args, command_index + 2),
        ),
        "nodes" => nodes_short_command(lang, args_tail(&args, command_index + 1)),
        "connect" => connect_short_command(lang, args_tail(&args, command_index + 1)),
        "pin" => pin_short_command(lang, args_tail(&args, command_index + 1)),
        "policy" => policy_command(
            lang,
            args.get(command_index + 1).map(String::as_str),
            args.get(command_index + 2).map(String::as_str),
        ),
        "diag" => diag_command(lang, args_tail(&args, command_index + 1)),
        "help" | "--help" | "-h" => {
            print_help(lang);
            0
        }
        other => {
            eprintln!("{}", render_unknown_command(lang, other));
            print_help(lang);
            2
        }
    };

    std::process::exit(exit_code);
}

fn args_tail(args: &[String], start: usize) -> &[String] {
    args.get(start..).unwrap_or(&[])
}

fn parse_language_flag(args: &[String]) -> Option<(Language, LanguageSource, usize)> {
    if args.get(1).map(String::as_str) != Some("--lang") {
        return Some((Language::Ru, LanguageSource::Default, 1));
    }
    match args.get(2).map(String::as_str) {
        Some("en") => Some((Language::En, LanguageSource::Flag, 3)),
        Some("ru") => Some((Language::Ru, LanguageSource::Flag, 3)),
        _ => None,
    }
}

#[cfg(test)]
fn detect_language_from_lang_value(lang: Option<&str>) -> Language {
    match lang {
        Some(value) if value.to_ascii_lowercase().starts_with("ru") => Language::Ru,
        _ => Language::En,
    }
}

fn probe_command(lang: Language, subcommand: Option<&str>, args: &[String]) -> i32 {
    if subcommand != Some("access") {
        eprintln!("{}", probe_usage(lang));
        return 2;
    }
    if args.iter().any(|arg| arg == "--proxy-url") {
        match lang {
            Language::En => eprintln!(
                "--proxy-url is forbidden for CHIMERA proof paths; use transparent WEAVE datapath"
            ),
            Language::Ru => eprintln!(
                "--proxy-url запрещен для proof-проверок CHIMERA; используйте прозрачный WEAVE datapath"
            ),
        }
        return 2;
    }
    let options = match parse_probe_options(args) {
        Ok(options) => options,
        Err(()) => {
            eprintln!("{}", probe_usage(lang));
            return 2;
        }
    };
    let urls = match collect_probe_urls(&options) {
        Ok(v) => v,
        Err(error) => {
            match lang {
                Language::En => eprintln!("Invalid probe targets: {error}"),
                Language::Ru => eprintln!("Некорректные цели probe: {error}"),
            }
            return 2;
        }
    };
    let mut rows: Vec<String> = Vec::new();
    let mut plain_lines: Vec<String> = Vec::new();
    let mut total = 0usize;
    let mut direct_ok_total = 0usize;
    let mut unreachable_total = 0usize;
    let mut policy_apply_failed_total = 0usize;
    let target_profile = if urls
        .iter()
        .all(|url| probe_url_matches_host(url, PROBE_CI_SNAPSHOT_HOST))
    {
        "ci_snapshot"
    } else {
        "live"
    };
    for url in &urls {
        total += 1;
        let target_ref = probe_target_ref(total);
        let policy_rule_ref = probe_policy_rule_ref(total);
        let direct_ok = run_curl_probe(url, options.timeout_seconds).unwrap_or(false);
        if direct_ok {
            direct_ok_total += 1;
        }
        let recommendation = if direct_ok { "direct" } else { "transit" };
        if !direct_ok {
            unreachable_total += 1;
        }
        let suggested_domain = extract_domain_from_url(url);
        let policy_hint = redacted_probe_policy_hint(suggested_domain.as_deref(), recommendation);
        let mut policy_apply_result = "not_requested".to_string();
        let mut policy_rule_id = String::new();
        let mut policy_verify_ok = false;
        let mut policy_verify_outbound = String::new();
        let mut target_error = String::new();
        let mut failover_override_failed = false;
        if direct_ok && let Some(domain) = suggested_domain.as_deref() {
            let flow_key = flow_key_for(domain, Protocol::Tcp, 443);
            if update_failover_override_key(&flow_key, false).is_err() {
                failover_override_failed = true;
            }
        }
        if let Some(path) = options.apply_policy_path.as_deref() {
            let Some(domain) = suggested_domain.as_deref() else {
                policy_apply_failed_total += 1;
                policy_apply_result = "failed".to_string();
                target_error = "domain_extract_failed".to_string();
                rows.push(render_probe_access_row(
                    &target_ref,
                    direct_ok,
                    recommendation,
                    &policy_hint,
                    &policy_apply_result,
                    "",
                    policy_verify_ok,
                    &policy_verify_outbound,
                    &target_error,
                ));
                plain_lines.push(format!(
                    "Target: {target_ref} | direct={} | route={} | policy={} | error={}",
                    if direct_ok { "ok" } else { "fail" },
                    recommendation,
                    policy_apply_result,
                    target_error
                ));
                continue;
            };
            if is_ip_literal(domain) {
                policy_apply_failed_total += 1;
                policy_apply_result = "failed".to_string();
                target_error = "policy_domain_ip_literal_not_supported".to_string();
                rows.push(render_probe_access_row(
                    &target_ref,
                    direct_ok,
                    recommendation,
                    &policy_hint,
                    &policy_apply_result,
                    "",
                    policy_verify_ok,
                    &policy_verify_outbound,
                    &target_error,
                ));
                plain_lines.push(format!(
                    "Target: {target_ref} | direct={} | route={} | policy={} | error={}",
                    if direct_ok { "ok" } else { "fail" },
                    recommendation,
                    policy_apply_result,
                    target_error
                ));
                continue;
            }
            if matches!(recommendation, "direct" | "transit") {
                let outbound = if recommendation == "transit" {
                    OutboundMode::Transit
                } else {
                    OutboundMode::Direct
                };
                match apply_probe_policy_rule(path, &options.rule_id_prefix, domain, outbound) {
                    Ok(rule_id) => {
                        policy_apply_result = "applied".to_string();
                        policy_rule_id = rule_id;
                        match verify_probe_policy_route(path, domain, outbound) {
                            Ok((ok, actual_outbound)) => {
                                policy_verify_ok = ok;
                                policy_verify_outbound = actual_outbound;
                                if !policy_verify_ok {
                                    policy_apply_failed_total += 1;
                                    policy_apply_result = "failed".to_string();
                                    target_error = "policy_verify_mismatch".to_string();
                                }
                            }
                            Err(_) => {
                                policy_apply_failed_total += 1;
                                policy_apply_result = "failed".to_string();
                                target_error = "policy_verify_error".to_string();
                            }
                        }
                    }
                    Err(_) => {
                        policy_apply_failed_total += 1;
                        policy_apply_result = "failed".to_string();
                        target_error = "policy_apply_error".to_string();
                    }
                }
            } else {
                policy_apply_result = "skipped_unknown_recommendation".to_string();
            }
        }
        if failover_override_failed && policy_apply_result != "failed" {
            policy_apply_failed_total += 1;
            policy_apply_result = "failed".to_string();
            policy_verify_ok = false;
            policy_rule_id.clear();
            policy_verify_outbound.clear();
            target_error = "failover_override_update_error".to_string();
        }
        let public_rule_ref = if policy_rule_id.is_empty() {
            ""
        } else {
            policy_rule_ref.as_str()
        };
        rows.push(render_probe_access_row(
            &target_ref,
            direct_ok,
            recommendation,
            &policy_hint,
            &policy_apply_result,
            public_rule_ref,
            policy_verify_ok,
            &policy_verify_outbound,
            &target_error,
        ));
        plain_lines.push(format!(
            "Target: {target_ref} | direct={} | route={} | policy={}{}",
            if direct_ok { "ok" } else { "fail" },
            recommendation,
            policy_apply_result,
            if target_error.is_empty() {
                String::new()
            } else {
                format!(" | error={target_error}")
            }
        ));
    }
    let failed_total = unreachable_total + policy_apply_failed_total;
    let threshold_exceeded = failed_total > options.fail_threshold;

    if options.json_output {
        let report = format!(
            "{{\"status\":\"ok\",\"kind\":\"probe_access\",\"redaction\":\"raw_targets_redacted\",\"target_profile\":\"{}\",\"totals\":{{\"all\":{},\"direct_ok\":{},\"unreachable\":{},\"policy_apply_failed\":{},\"failed_total\":{},\"fail_threshold\":{},\"threshold_exceeded\":{}}},\"targets\":[{}],\"network_state\":\"not_modified\"}}\n",
            target_profile,
            total,
            direct_ok_total,
            unreachable_total,
            policy_apply_failed_total,
            failed_total,
            options.fail_threshold,
            if threshold_exceeded { "true" } else { "false" },
            rows.join(",")
        );
        if let Some(path) = options.out_path.as_deref()
            && let Err(error) = std::fs::write(path, &report)
        {
            match lang {
                Language::En => eprintln!("Cannot write probe report: {error}"),
                Language::Ru => eprintln!("Не удалось записать отчет probe: {error}"),
            }
            return 1;
        }
        print!("{report}");
    } else {
        println!(
            "Итоги: всего={} direct_ok={} недоступно={} ошибок_применения_policy={} всего_ошибок={} порог_ошибок={}",
            total,
            direct_ok_total,
            unreachable_total,
            policy_apply_failed_total,
            failed_total,
            options.fail_threshold
        );
        for line in plain_lines {
            println!("{line}");
        }
    }
    if threshold_exceeded { 1 } else { 0 }
}

fn probe_target_ref(index: usize) -> String {
    format!("target#{index}")
}

fn probe_policy_rule_ref(index: usize) -> String {
    format!("rule#{index}")
}

fn redacted_probe_policy_hint(domain: Option<&str>, recommendation: &str) -> String {
    let target_kind = match domain {
        Some(value) if is_ip_literal(value) => "ip_literal",
        Some(_) => "domain_exact_present",
        None => "domain_absent",
    };
    format!("target_kind={target_kind} outbound={recommendation}")
}

#[allow(clippy::too_many_arguments)]
fn render_probe_access_row(
    target_ref: &str,
    direct_ok: bool,
    recommendation: &str,
    policy_hint: &str,
    policy_apply_result: &str,
    policy_rule_ref: &str,
    policy_verify_ok: bool,
    policy_verify_outbound: &str,
    target_error: &str,
) -> String {
    format!(
        "{{\"url\":\"{}\",\"target_ref\":\"{}\",\"direct_ok\":{},\"recommended_route\":\"{}\",\"policy_hint\":\"{}\",\"policy_apply_result\":\"{}\",\"policy_rule_ref\":\"{}\",\"policy_verify_ok\":{},\"policy_verify_outbound\":\"{}\",\"target_error\":\"{}\"}}",
        escape_json(target_ref),
        escape_json(target_ref),
        if direct_ok { "true" } else { "false" },
        recommendation,
        escape_json(policy_hint),
        escape_json(policy_apply_result),
        escape_json(policy_rule_ref),
        if policy_verify_ok { "true" } else { "false" },
        escape_json(policy_verify_outbound),
        escape_json(target_error),
    )
}

fn parse_probe_options(args: &[String]) -> Result<ProbeOptions, ()> {
    let mut options = ProbeOptions {
        urls: Vec::new(),
        url_file: None,
        timeout_seconds: 8,
        apply_policy_path: None,
        rule_id_prefix: "auto-probe".to_string(),
        fail_threshold: 0,
        json_output: false,
        out_path: None,
    };
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--url" => {
                options.urls.push(args.get(index + 1).cloned().ok_or(())?);
                index += 2;
            }
            "--url-file" => {
                options.url_file = Some(args.get(index + 1).cloned().ok_or(())?);
                index += 2;
            }
            "--proxy-url" => {
                return Err(());
            }
            "--timeout-sec" => {
                let raw = args.get(index + 1).ok_or(())?;
                let parsed = raw.parse::<u64>().map_err(|_| ())?;
                if parsed == 0 || parsed > 300 {
                    return Err(());
                }
                options.timeout_seconds = parsed;
                index += 2;
            }
            "--apply-policy" => {
                options.apply_policy_path = Some(args.get(index + 1).cloned().ok_or(())?);
                index += 2;
            }
            "--rule-id-prefix" => {
                options.rule_id_prefix = args.get(index + 1).cloned().ok_or(())?;
                index += 2;
            }
            "--fail-threshold" => {
                let raw = args.get(index + 1).ok_or(())?;
                options.fail_threshold = raw.parse::<usize>().map_err(|_| ())?;
                index += 2;
            }
            "--json" => {
                options.json_output = true;
                index += 1;
            }
            "--out" => {
                options.out_path = Some(args.get(index + 1).cloned().ok_or(())?);
                index += 2;
            }
            _ => return Err(()),
        }
    }
    if options.urls.is_empty() && options.url_file.is_none() {
        return Err(());
    }
    if options.rule_id_prefix.trim().is_empty() {
        return Err(());
    }
    Ok(options)
}

fn collect_probe_urls(options: &ProbeOptions) -> Result<Vec<String>, String> {
    let mut out = options.urls.clone();
    if let Some(path) = options.url_file.as_deref() {
        let text =
            std::fs::read_to_string(path).map_err(|e| format!("cannot read --url-file: {e}"))?;
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            out.push(line.to_string());
        }
    }
    if out.is_empty() {
        return Err("no urls provided".to_string());
    }
    let mut deduped: Vec<String> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for url in out {
        let key = url.trim().to_ascii_lowercase();
        if key.is_empty() || !seen.insert(key) {
            continue;
        }
        deduped.push(url);
    }
    if deduped.is_empty() {
        return Err("no urls provided after normalization".to_string());
    }
    for (idx, url) in deduped.iter().enumerate() {
        if !is_supported_probe_url(url) {
            return Err(format!("invalid url at {}", probe_target_ref(idx + 1)));
        }
    }
    Ok(deduped)
}

fn is_ip_literal(value: &str) -> bool {
    value.parse::<std::net::IpAddr>().is_ok()
}

fn run_curl_probe(url: &str, timeout_seconds: u64) -> Result<bool, String> {
    fn run_once(url: &str, timeout_seconds: u64) -> Result<bool, String> {
        let timeout_arg = timeout_seconds.to_string();
        let mut cmd = Command::new("curl");
        cmd.arg("--disable")
            .arg("-sS")
            .arg("-L")
            .arg("--noproxy")
            .arg("*")
            .arg("--output")
            .arg("/dev/null")
            .arg("--max-time")
            .arg(&timeout_arg)
            .arg("--connect-timeout")
            .arg(&timeout_arg)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        for key in [
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "NO_PROXY",
            "http_proxy",
            "https_proxy",
            "all_proxy",
            "no_proxy",
        ] {
            cmd.env_remove(key);
        }
        cmd.arg(url);
        let status = cmd
            .status()
            .map_err(|error| format!("curl failed to start: {error}"))?;
        Ok(status.success())
    }

    // A single probe can fail due to short-lived upstream jitter.
    // Retry once to reduce false negatives in real-world checks.
    const MAX_ATTEMPTS: usize = 2;
    let mut last_ok = false;
    for _ in 0..MAX_ATTEMPTS {
        last_ok = run_once(url, timeout_seconds)?;
        if last_ok {
            return Ok(true);
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    Ok(last_ok)
}

fn is_supported_probe_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

fn extract_domain_from_url(url: &str) -> Option<String> {
    let trimmed = url.trim();
    let (_, rest) = trimmed.split_once("://")?;
    let authority = rest.split('/').next().unwrap_or(rest);
    let host_port = authority.rsplit('@').next().unwrap_or(authority);
    let host = if host_port.starts_with('[') {
        let end = host_port.find(']')?;
        &host_port[1..end]
    } else {
        host_port.split(':').next().unwrap_or(host_port)
    };
    let normalized = host.trim().trim_end_matches('.').to_ascii_lowercase();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn probe_url_matches_host(url: &str, expected_host: &str) -> bool {
    extract_domain_from_url(url).is_some_and(|host| host == expected_host)
}

fn outbound_to_policy_token(outbound: OutboundMode) -> &'static str {
    match outbound {
        OutboundMode::Direct => "direct",
        OutboundMode::Transit => "transit",
        OutboundMode::Block => "block",
    }
}

fn sanitize_rule_id_token(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('-');
        }
    }
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}

fn apply_probe_policy_rule(
    policy_path: &str,
    rule_id_prefix: &str,
    domain: &str,
    outbound: OutboundMode,
) -> Result<String, String> {
    let existing = std::fs::read_to_string(policy_path).unwrap_or_default();
    let prefix = sanitize_rule_id_token(rule_id_prefix);
    if prefix.is_empty() {
        return Err("rule id prefix is empty after sanitization".to_string());
    }
    let domain_token = sanitize_rule_id_token(domain);
    if domain_token.is_empty() {
        return Err("domain cannot be converted to stable rule id token".to_string());
    }
    let rule_id = format!("{prefix}-{domain_token}");
    let matcher = format!("exact:{domain}");
    let desired_line = format!(
        "{rule_id} = {matcher} => {}",
        outbound_to_policy_token(outbound)
    );

    let mut replaced = false;
    let mut found_exact = false;
    let mut output_lines: Vec<String> = Vec::new();
    for raw in existing.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            output_lines.push(raw.to_string());
            continue;
        }
        if let Some((left, right)) = trimmed.split_once('=')
            && let Some((matcher_raw, _outbound_raw)) = right.split_once("=>")
        {
            let current_id = left.trim();
            let current_matcher = matcher_raw.trim();
            if current_matcher.eq_ignore_ascii_case(&matcher) {
                found_exact = true;
                output_lines.push(desired_line.clone());
                replaced = true;
                continue;
            }
            if current_id == rule_id {
                output_lines.push(desired_line.clone());
                replaced = true;
                continue;
            }
        }
        output_lines.push(raw.to_string());
    }
    if existing.trim().is_empty() {
        output_lines.push("# runtime policy (auto-generated by probe access)".to_string());
        output_lines.push("default = default => direct".to_string());
    }
    if !replaced && !found_exact {
        output_lines.push(desired_line);
    }
    let mut rendered = output_lines.join("\n");
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    chimera_policy::parse_policy_text(&rendered)
        .map_err(|error| format!("resulting policy is invalid: {error}"))?;
    atomic_write_text_file(Path::new(policy_path), &rendered)
        .map_err(|error| format!("write failed: {error}"))?;
    Ok(rule_id)
}

fn atomic_write_text_file(target: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = target.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir failed: {e}"))?;
    }
    let mut tmp_name = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("chimera-policy")
        .to_string();
    tmp_name.push_str(".tmp");
    let tmp_path = target.with_file_name(tmp_name);
    std::fs::write(&tmp_path, content).map_err(|e| format!("temp write failed: {e}"))?;
    std::fs::rename(&tmp_path, target).map_err(|e| format!("rename failed: {e}"))?;
    Ok(())
}

fn verify_probe_policy_route(
    policy_path: &str,
    domain: &str,
    expected: OutboundMode,
) -> Result<(bool, String), String> {
    let text =
        std::fs::read_to_string(policy_path).map_err(|error| format!("read failed: {error}"))?;
    let policy = chimera_policy::parse_policy_text(&text)
        .map_err(|error| format!("parse failed: {error}"))?;
    let flow = FlowContext {
        domain: Some(domain.to_string()),
        destination_ip: None,
        protocol: Protocol::Tcp,
        port: Some(443),
    };
    let decision = policy.decide(&flow);
    let actual = outbound_to_policy_token(decision.outbound).to_string();
    Ok((decision.outbound == expected, actual))
}

fn flow_key_for(domain: &str, protocol: Protocol, port: u16) -> String {
    let proto = match protocol {
        Protocol::Tcp => "tcp",
        Protocol::Udp => "udp",
        Protocol::Icmp => "icmp",
        Protocol::Other(_) => "other",
    };
    format!("{}:{port}/{proto}", domain.to_ascii_lowercase())
}

fn runtime_failover_overrides_path() -> PathBuf {
    if let Ok(path) = std::env::var("CHIMERA_FAILOVER_OVERRIDES_PATH")
        && !path.trim().is_empty()
    {
        return PathBuf::from(path);
    }
    if let Ok(xdg_state_home) = std::env::var("XDG_STATE_HOME")
        && !xdg_state_home.trim().is_empty()
    {
        return PathBuf::from(xdg_state_home)
            .join("chimera")
            .join(RUNTIME_FAILOVER_OVERRIDES_FILE);
    }
    if let Ok(home) = std::env::var("HOME")
        && !home.trim().is_empty()
    {
        return PathBuf::from(home)
            .join(".local")
            .join("state")
            .join("chimera")
            .join(RUNTIME_FAILOVER_OVERRIDES_FILE);
    }
    std::env::temp_dir()
        .join("chimera")
        .join(RUNTIME_FAILOVER_OVERRIDES_FILE)
}

fn load_failover_override_keys_from_path(
    path: &Path,
) -> Option<std::collections::BTreeSet<String>> {
    if !path.is_file() {
        return None;
    }
    let mut out = std::collections::BTreeSet::new();
    let Ok(text) = std::fs::read_to_string(path) else {
        return Some(out);
    };
    for line in text.lines() {
        let item = line.trim();
        if item.is_empty() || item.starts_with('#') {
            continue;
        }
        out.insert(item.to_string());
    }
    Some(out)
}

fn load_failover_override_keys() -> std::collections::BTreeSet<String> {
    if let Some(out) = load_failover_override_keys_from_path(&runtime_failover_overrides_path()) {
        return out;
    }
    load_failover_override_keys_from_path(Path::new(LEGACY_RUNTIME_FAILOVER_OVERRIDES_PATH))
        .unwrap_or_default()
}

fn write_failover_override_keys(
    path: &Path,
    values: &std::collections::BTreeSet<String>,
) -> Result<(), String> {
    let mut rendered = String::new();
    rendered.push_str("# auto-generated runtime failover overrides\n");
    for value in values {
        rendered.push_str(value);
        rendered.push('\n');
    }
    atomic_write_text_file(path, &rendered)
}

fn update_failover_override_key(key: &str, blocked: bool) -> Result<(), String> {
    let mut values = load_failover_override_keys();
    if blocked {
        values.insert(key.to_string());
    } else {
        values.remove(key);
    }
    write_failover_override_keys(&runtime_failover_overrides_path(), &values)
}

fn outbound_to_datapath_route(outbound: OutboundMode) -> DatapathRoute {
    match outbound {
        OutboundMode::Direct => DatapathRoute::Direct,
        OutboundMode::Transit => DatapathRoute::Transit,
        OutboundMode::Block => DatapathRoute::Block,
    }
}

fn datapath_route_to_outbound(route: DatapathRoute) -> OutboundMode {
    match route {
        DatapathRoute::Direct => OutboundMode::Direct,
        DatapathRoute::Transit => OutboundMode::Transit,
        DatapathRoute::Block => OutboundMode::Block,
    }
}

fn route_command(
    lang: Language,
    subcommand: Option<&str>,
    domain: Option<&str>,
    extra_args: &[String],
) -> i32 {
    if subcommand != Some("explain") {
        eprintln!("{}", route_usage(lang));
        return 2;
    }

    let mut option_args: Vec<String> = Vec::new();
    let positional_domain = match domain {
        Some(value) if value.starts_with("--") => {
            option_args.push(value.to_string());
            None
        }
        Some(value) => Some(value.to_string()),
        None => None,
    };
    option_args.extend(extra_args.iter().cloned());

    let options = match parse_route_explain_options(&option_args) {
        Ok(options) => options,
        Err(()) => {
            eprintln!("{}", route_usage(lang));
            return 2;
        }
    };
    let mut effective_domain = options.domain_override.clone().or(positional_domain);

    let auto_runtime_policy_path = "configs/policy.runtime.conf";
    let policy_path = if let Some(path) = options.policy_path.as_deref() {
        Some(path)
    } else if Path::new(auto_runtime_policy_path).exists() {
        Some(auto_runtime_policy_path)
    } else {
        None
    };

    let policy = if let Some(path) = policy_path {
        let file_content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) => {
                match lang {
                    Language::En => eprintln!("Cannot read policy file: {error}"),
                    Language::Ru => eprintln!("Не удалось прочитать файл policy: {error}"),
                }
                return 2;
            }
        };
        match chimera_policy::parse_policy_text(&file_content) {
            Ok(policy) => policy,
            Err(error) => {
                match lang {
                    Language::En => eprintln!("Policy is not valid: {error}"),
                    Language::Ru => eprintln!("Policy некорректен: {error}"),
                }
                return 2;
            }
        }
    } else {
        Policy::new(vec![
            RouteRule {
                id: "example-transit".to_string(),
                matcher: RuleMatcher::DomainSuffix("example.org".to_string()),
                outbound: OutboundMode::Transit,
            },
            RouteRule {
                id: "default-direct".to_string(),
                matcher: RuleMatcher::Default,
                outbound: OutboundMode::Direct,
            },
        ])
    };

    let mut domain_source_from_dns = false;
    let mut dns_note: Option<&'static str> = None;
    if effective_domain.is_none()
        && let (Some(bind_domain), Some(bind_ip), Some(destination_ip)) = (
            options.dns_bind_domain.as_deref(),
            options.dns_bind_ip,
            options.destination_ip,
        )
    {
        let mut store = DnsBindingStore::default();
        store.insert(DnsBinding::new(
            bind_domain,
            bind_ip,
            route_explain_dns_binding_ttl(),
            Instant::now(),
        ));
        if let Some(binding) = store.lookup(destination_ip, Instant::now()) {
            effective_domain = Some(binding.domain.clone());
            domain_source_from_dns = true;
        } else {
            dns_note = Some(match lang {
                Language::En => "DNS binding was provided, but IP did not match binding IP.",
                Language::Ru => "DNS binding задан, но IP не совпал с IP в binding.",
            });
        }
    } else if effective_domain.is_none()
        && options.dns_bind_domain.is_none()
        && options.destination_ip.is_some()
    {
        dns_note = Some(match lang {
            Language::En => "No DNS binding provided for this IP. Domain remains unknown.",
            Language::Ru => "Для этого IP DNS binding не задан. Домен остался неизвестным.",
        });
    }

    let flow = FlowContext {
        domain: effective_domain.clone(),
        destination_ip: options.destination_ip,
        protocol: options.protocol.unwrap_or(Protocol::Tcp),
        port: options.port.or(Some(443)),
    };
    let trace = policy.explain(&flow);
    let mut runtime_outbound = trace.decision.outbound;
    let mut runtime_reason = trace.decision.explanation.clone();
    if let Some(domain_value) = effective_domain.as_deref() {
        let config_flags = if let Some(path) = options.config_path.as_deref() {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|text| parse_node_config_text(&text).ok())
                .map(|cfg| {
                    (
                        cfg.split_tunnel_default,
                        cfg.auto_failover,
                        cfg.invisible_mode_required,
                    )
                })
                .unwrap_or((true, true, true))
        } else {
            (true, true, true)
        };
        let mut engine = match TransparentFailoverEngine::new(TransparentFailoverConfig {
            split_tunnel_default: config_flags.0,
            auto_failover: config_flags.1,
            failover_ttl_ticks: route_explain_failover_ttl_ticks(),
        }) {
            Ok(engine) => engine,
            Err(_) => {
                if options.json_output {
                    eprintln!("invalid transparent failover config");
                }
                return 2;
            }
        };
        let flow_key = flow_key_for(domain_value, flow.protocol, flow.port.unwrap_or(443));
        if load_failover_override_keys().contains(&flow_key) {
            engine.report_direct_blocked(&flow_key);
        }
        let decision = engine.evaluate(
            &flow_key,
            outbound_to_datapath_route(trace.decision.outbound),
        );
        runtime_outbound = datapath_route_to_outbound(decision.route);
        runtime_reason = if config_flags.2 {
            format!("{}; invisible_mode_required=true", decision.reason)
        } else {
            decision.reason
        };
    }

    if options.json_output {
        let render_context = RouteExplainRenderContext {
            lang,
            domain: effective_domain.as_deref(),
            domain_source_from_dns,
            dns_note,
            flow: &flow,
            trace: &trace,
            runtime_outbound,
            runtime_reason: &runtime_reason,
            show_all_matches: options.show_all_matches,
        };
        let json = render_route_explain_json(&render_context);
        if let Some(path) = options.out_path.as_deref()
            && let Err(error) = std::fs::write(path, &json)
        {
            eprintln!("Не удалось записать вывод route explain: {error}");
            return 1;
        }
        println!("{json}");
    } else {
        let render_context = RouteExplainRenderContext {
            lang,
            domain: effective_domain.as_deref(),
            domain_source_from_dns,
            dns_note,
            flow: &flow,
            trace: &trace,
            runtime_outbound,
            runtime_reason: &runtime_reason,
            show_all_matches: options.show_all_matches,
        };
        print!("{}", render_route_explain_block(&render_context));
    }
    0
}

fn mesh_command(lang: Language, subcommand: Option<&str>, args: &[String]) -> i32 {
    mesh_cli::mesh_command(mesh_usage(lang), subcommand, args)
}

fn nodes_short_command(lang: Language, args: &[String]) -> i32 {
    let mut nodes_args = Vec::with_capacity(args.len() + 1);
    if args
        .first()
        .map(|value| value.starts_with('-'))
        .unwrap_or(true)
    {
        nodes_args.push("list".to_string());
    }
    nodes_args.extend_from_slice(args);
    mesh_command(lang, Some("nodes"), &nodes_args)
}

fn connect_short_command(lang: Language, args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!(
            "использование: chimera connect <index|node_id> [--country DE,NL] [--status healthy]"
        );
        return 2;
    }
    let mut nodes_args = Vec::with_capacity(args.len() + 1);
    nodes_args.push("connect".to_string());
    nodes_args.extend_from_slice(args);
    mesh_command(lang, Some("nodes"), &nodes_args)
}

fn pin_short_command(lang: Language, args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!(
            "использование: chimera pin <index|node_id> [--country DE,NL] [--status healthy]"
        );
        return 2;
    }
    let mut nodes_args = Vec::with_capacity(args.len() + 1);
    nodes_args.push("pin".to_string());
    nodes_args.extend_from_slice(args);
    mesh_command(lang, Some("nodes"), &nodes_args)
}

#[cfg(test)]
fn parse_mesh_route_explain_options(
    args: &[String],
) -> Result<mesh_cli::MeshRouteExplainOptions, String> {
    mesh_cli::parse_mesh_route_explain_options(args)
}

fn parse_route_explain_options(args: &[String]) -> Result<RouteExplainOptions, ()> {
    let mut options = RouteExplainOptions::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        if flag == "--show-all-matches" {
            options.show_all_matches = true;
            index += 1;
            continue;
        }
        if flag == "--json" {
            options.json_output = true;
            index += 1;
            continue;
        }
        let value = args.get(index + 1).map(String::as_str).ok_or(())?;
        match flag {
            "--domain" => {
                options.domain_override = Some(value.to_string());
            }
            "--config" => {
                options.config_path = Some(value.to_string());
            }
            "--policy" => {
                options.policy_path = Some(value.to_string());
            }
            "--ip" => {
                options.destination_ip = Some(value.parse::<IpAddr>().map_err(|_| ())?);
            }
            "--proto" => {
                options.protocol = Some(parse_cli_protocol(value)?);
            }
            "--port" => {
                options.port = Some(value.parse::<u16>().map_err(|_| ())?);
            }
            "--dns-bind-domain" => {
                options.dns_bind_domain = Some(value.to_string());
            }
            "--dns-bind-ip" => {
                options.dns_bind_ip = Some(value.parse::<IpAddr>().map_err(|_| ())?);
            }
            "--out" => {
                options.out_path = Some(value.to_string());
            }
            _ => return Err(()),
        }
        index += 2;
    }
    if options.dns_bind_domain.is_some() ^ options.dns_bind_ip.is_some() {
        return Err(());
    }
    Ok(options)
}

struct RouteExplainRenderContext<'a> {
    lang: Language,
    domain: Option<&'a str>,
    domain_source_from_dns: bool,
    dns_note: Option<&'a str>,
    flow: &'a FlowContext,
    trace: &'a RouteExplainTrace,
    runtime_outbound: OutboundMode,
    runtime_reason: &'a str,
    show_all_matches: bool,
}

fn render_route_explain_json(context: &RouteExplainRenderContext<'_>) -> String {
    let decision: &RouteDecision = &context.trace.decision;
    let domain_value = context.domain.unwrap_or("");
    let flow = context.flow;
    let ip_value = flow
        .destination_ip
        .map(|ip| ip.to_string())
        .unwrap_or_default();
    let proto_value = match flow.protocol {
        Protocol::Tcp => "tcp",
        Protocol::Udp => "udp",
        Protocol::Icmp => "icmp",
        Protocol::Other(_) => "other",
    };
    let port_value = flow.port.unwrap_or(0);
    let outbound_value = match decision.outbound {
        OutboundMode::Direct => "direct",
        OutboundMode::Transit => "transit",
        OutboundMode::Block => "block",
    };
    let dns_note_json = match context.dns_note {
        Some(note) => format!("\"{note}\""),
        None => "null".to_string(),
    };
    let runtime_outbound_value = outbound_to_policy_token(context.runtime_outbound);
    let matched_rules_json = if context.show_all_matches {
        let items = context
            .trace
            .matched_rule_ids_by_priority
            .iter()
            .map(|id| format!("\"{id}\""))
            .collect::<Vec<_>>()
            .join(",");
        format!("[{items}]")
    } else {
        "[]".to_string()
    };
    format!(
        "{{\"status\":\"ok\",\"kind\":\"route_explain\",\"message_en\":\"Route explanation is ready.\",\"message_ru\":\"Объяснение маршрута готово.\",\"domain\":\"{}\",\"domain_source_dns\":{},\"dns_note\":{},\"ip\":\"{}\",\"proto\":\"{}\",\"port\":{},\"rule_used\":\"{}\",\"outbound\":\"{}\",\"reason\":\"{}\",\"runtime_outbound\":\"{}\",\"runtime_reason\":\"{}\",\"rules_checked\":{},\"rules_matched\":{},\"matched_rules\":{},\"network_state\":\"not_modified\"}}",
        domain_value,
        context.domain_source_from_dns,
        dns_note_json,
        ip_value,
        proto_value,
        port_value,
        decision.matched_rule_id,
        outbound_value,
        decision.explanation,
        runtime_outbound_value,
        escape_json(context.runtime_reason),
        context.trace.examined_rules,
        context.trace.matched_rules,
        matched_rules_json
    )
}

fn parse_cli_protocol(input: &str) -> Result<Protocol, ()> {
    match input.to_ascii_lowercase().as_str() {
        "tcp" => Ok(Protocol::Tcp),
        "udp" => Ok(Protocol::Udp),
        "icmp" => Ok(Protocol::Icmp),
        _ => Err(()),
    }
}

fn policy_command(lang: Language, subcommand: Option<&str>, file_path: Option<&str>) -> i32 {
    if subcommand != Some("validate") {
        eprintln!("{}", policy_usage(lang));
        return 2;
    }
    let Some(file_path) = file_path else {
        eprintln!("{}", policy_usage(lang));
        return 2;
    };

    let file_content = match std::fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(error) => {
            match lang {
                Language::En => eprintln!("Cannot read policy file: {error}"),
                Language::Ru => eprintln!("Не удалось прочитать файл policy: {error}"),
            }
            return 2;
        }
    };

    let policy = match chimera_policy::parse_policy_text(&file_content) {
        Ok(policy) => policy,
        Err(error) => {
            match lang {
                Language::En => eprintln!("Policy is not valid: {error}"),
                Language::Ru => eprintln!("Policy некорректен: {error}"),
            }
            return 2;
        }
    };
    print!("{}", render_policy_validate_block(lang, &policy.summary()));
    0
}

fn lab_command(lang: Language, subcommand: Option<&str>, extra_args: &[String]) -> i32 {
    let mut use_benchmark_regression_defaults = false;
    let mut use_mvp_check_defaults = false;
    let lab_subcommand = match subcommand {
        Some("smoke") => "smoke",
        Some("doctor") => "doctor",
        Some("config-smoke") => "config-smoke",
        Some("fuzz-smoke") => "fuzz-smoke",
        Some("perf-smoke") => "perf-smoke",
        Some("net-sim") => "net-sim",
        Some("benchmark-report") => "benchmark-report",
        Some("benchmark-regression-check") => {
            use_benchmark_regression_defaults = true;
            "benchmark-report"
        }
        Some("hardening-smoke") => "hardening-smoke",
        Some("mvp-spec-check") => "mvp-spec-check",
        Some("mvp-spec-report") => "mvp-spec-report",
        Some("m5-artifacts-report") => "m5-artifacts-report",
        Some("m6-artifacts-report") => "m6-artifacts-report",
        Some("release-readiness-report") => "release-readiness-report",
        Some("report-pack") => "report-pack",
        Some("artifact-audit") => "artifact-audit",
        Some("mvp-snapshot") => "mvp-snapshot",
        Some("mvp-verify") => "mvp-verify",
        Some("mvp-check") => {
            use_mvp_check_defaults = true;
            "mvp-verify"
        }
        _ => {
            eprintln!("{}", lab_usage(lang));
            return 2;
        }
    };
    let forwarded_args: Vec<String> = if use_mvp_check_defaults && extra_args.is_empty() {
        vec![
            "--refresh".to_string(),
            "--json".to_string(),
            "--out".to_string(),
            "docs/MVP_VERIFY.json".to_string(),
        ]
    } else if use_benchmark_regression_defaults && extra_args.is_empty() {
        vec![
            "--baseline".to_string(),
            "docs/benchmark_latest.json".to_string(),
            "--max-regression-pct".to_string(),
            "20".to_string(),
            "--out".to_string(),
            "docs/benchmark_latest.json".to_string(),
        ]
    } else {
        extra_args.to_vec()
    };

    let lang_value = match lang {
        Language::En => "en",
        Language::Ru => "ru",
    };

    let mut direct = std::process::Command::new("chimera-lab");
    direct.args(["--lang", lang_value, lab_subcommand]);
    direct.args(&forwarded_args);
    match direct.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                match lang {
                    Language::En => eprintln!("Cannot run chimera-lab {lab_subcommand}: {error}"),
                    Language::Ru => {
                        eprintln!("Не удалось запустить chimera-lab {lab_subcommand}: {error}")
                    }
                }
                return 1;
            }
            let mut cargo = std::process::Command::new("cargo");
            cargo.args([
                "run",
                "-q",
                "-p",
                "chimera-lab",
                "--",
                "--lang",
                lang_value,
                lab_subcommand,
            ]);
            cargo.args(&forwarded_args);
            match cargo.status() {
                Ok(status) => status.code().unwrap_or(1),
                Err(cargo_error) => {
                    match lang {
                        Language::En => eprintln!(
                            "Cannot run chimera-lab {lab_subcommand}. Also failed via cargo: {cargo_error}"
                        ),
                        Language::Ru => eprintln!(
                            "Не удалось запустить chimera-lab {lab_subcommand}. Также не вышло через cargo: {cargo_error}"
                        ),
                    }
                    1
                }
            }
        }
    }
}

fn render_policy_validate_block(lang: Language, summary: &PolicySummary) -> String {
    let warnings = policy_warnings(lang, summary);
    let warning_block = if warnings.is_empty() {
        String::new()
    } else {
        let mut out = String::new();
        match lang {
            Language::En => out.push_str("Warnings:\n"),
            Language::Ru => out.push_str("Предупреждения:\n"),
        }
        for warning in warnings {
            out.push_str("- ");
            out.push_str(warning);
            out.push('\n');
        }
        out
    };

    match lang {
        Language::En => format!(
            "Policy file is valid.\n\
Total rules: {}\n\
Domain rules: exact={}, suffix={}\n\
IP/protocol rules: cidr4={}, protoport={}\n\
Default rules: {}\n\
Traffic actions: direct={}, peer transit={}, block={}\n\
{}\
Next step: run `chimera route explain --policy <file> --domain example.org`\n",
            summary.total_rules,
            summary.exact_domain_rules,
            summary.domain_suffix_rules,
            summary.cidr4_rules,
            summary.protoport_rules,
            summary.default_rules,
            summary.direct_outbound_rules,
            summary.transit_outbound_rules,
            summary.block_outbound_rules,
            warning_block
        ),
        Language::Ru => format!(
            "Файл policy корректный.\n\
Всего правил: {}\n\
Правила по доменам: exact={}, suffix={}\n\
Правила по IP/протоколу: cidr4={}, protoport={}\n\
Правила по умолчанию: default={}\n\
Действия для трафика: direct={}, peer transit={}, block={}\n\
{}\
Дальше: запустите `chimera route explain --policy <файл> --domain example.org`\n",
            summary.total_rules,
            summary.exact_domain_rules,
            summary.domain_suffix_rules,
            summary.cidr4_rules,
            summary.protoport_rules,
            summary.default_rules,
            summary.direct_outbound_rules,
            summary.transit_outbound_rules,
            summary.block_outbound_rules,
            warning_block
        ),
    }
}

fn policy_warnings(lang: Language, summary: &PolicySummary) -> Vec<&'static str> {
    let mut warnings = Vec::new();
    match lang {
        Language::En => {
            if summary.default_rules == 0 {
                warnings.push("No default rule. Unknown traffic will use implicit direct route.");
            }
            if summary.transit_outbound_rules == 0 {
                warnings.push("No peer transit rules. WEAVE node may never relay.");
            }
        }
        Language::Ru => {
            if summary.default_rules == 0 {
                warnings.push(
                    "Нет default-правила. Неизвестный трафик уйдет напрямую по неявному правилу.",
                );
            }
            if summary.transit_outbound_rules == 0 {
                warnings.push("Нет правил peer transit. Узел WEAVE может никогда не транзитить.");
            }
        }
    }
    warnings
}

fn render_route_explain_block(context: &RouteExplainRenderContext<'_>) -> String {
    let mut out = String::new();
    let decision: &RouteDecision = &context.trace.decision;
    let flow = context.flow;
    let domain_label = context.domain.unwrap_or("-");
    let ip_label = flow
        .destination_ip
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "-".to_string());
    let proto_label = match flow.protocol {
        Protocol::Tcp => "tcp",
        Protocol::Udp => "udp",
        Protocol::Icmp => "icmp",
        Protocol::Other(_) => "other",
    };
    let port_label = flow
        .port
        .map(|port| port.to_string())
        .unwrap_or_else(|| "-".to_string());
    match context.lang {
        Language::En => {
            out.push_str(&format!("Site: {domain_label}\n"));
            if context.domain_source_from_dns {
                out.push_str("Domain source: DNS binding (IP -> domain)\n");
            }
            if let Some(note) = context.dns_note {
                out.push_str(&format!("DNS note: {note}\n"));
            }
            out.push_str(&format!("IP: {ip_label}\n"));
            out.push_str(&format!("Protocol: {proto_label}\n"));
            out.push_str(&format!("Port: {port_label}\n"));
            out.push_str(&format!("Rule used: {}\n", decision.matched_rule_id));
            out.push_str(&format!(
                "How we send: {}\n",
                outbound_label_en(decision.outbound)
            ));
            out.push_str(&format!("Reason: {}\n", decision.explanation));
            out.push_str(&format!(
                "Runtime route: {}\n",
                outbound_label_en(context.runtime_outbound)
            ));
            out.push_str(&format!("Runtime reason: {}\n", context.runtime_reason));
            out.push_str(&format!(
                "Rules checked: {}\n",
                context.trace.examined_rules
            ));
            out.push_str(&format!("Rules matched: {}\n", context.trace.matched_rules));
            if context.show_all_matches {
                out.push_str("Matched rules (best first): ");
                if context.trace.matched_rule_ids_by_priority.is_empty() {
                    out.push_str("none\n");
                } else {
                    out.push_str(&context.trace.matched_rule_ids_by_priority.join(", "));
                    out.push('\n');
                }
            }
        }
        Language::Ru => {
            out.push_str(&format!("Сайт: {domain_label}\n"));
            if context.domain_source_from_dns {
                out.push_str("Источник домена: DNS binding (IP -> домен)\n");
            }
            if let Some(note) = context.dns_note {
                out.push_str(&format!("Примечание DNS: {note}\n"));
            }
            out.push_str(&format!("IP: {ip_label}\n"));
            out.push_str(&format!("Протокол: {proto_label}\n"));
            out.push_str(&format!("Порт: {port_label}\n"));
            out.push_str(&format!(
                "Сработало правило: {}\n",
                decision.matched_rule_id
            ));
            out.push_str(&format!(
                "Как отправляем: {}\n",
                outbound_label_ru(decision.outbound)
            ));
            out.push_str(&format!("Причина: {}\n", decision.explanation));
            out.push_str(&format!(
                "Runtime маршрут: {}\n",
                outbound_label_ru(context.runtime_outbound)
            ));
            out.push_str(&format!("Причина runtime: {}\n", context.runtime_reason));
            out.push_str(&format!(
                "Проверено правил: {}\n",
                context.trace.examined_rules
            ));
            out.push_str(&format!(
                "Совпало правил: {}\n",
                context.trace.matched_rules
            ));
            if context.show_all_matches {
                out.push_str("Совпавшие правила (лучшее первым): ");
                if context.trace.matched_rule_ids_by_priority.is_empty() {
                    out.push_str("нет\n");
                } else {
                    out.push_str(&context.trace.matched_rule_ids_by_priority.join(", "));
                    out.push('\n');
                }
            }
        }
    }
    out
}

fn outbound_label_en(outbound: chimera_policy::OutboundMode) -> &'static str {
    match outbound {
        chimera_policy::OutboundMode::Direct => "direct connection",
        chimera_policy::OutboundMode::Transit => "through peer transit node",
        chimera_policy::OutboundMode::Block => "blocked by policy",
    }
}

fn outbound_label_ru(outbound: chimera_policy::OutboundMode) -> &'static str {
    match outbound {
        chimera_policy::OutboundMode::Direct => "напрямую",
        chimera_policy::OutboundMode::Transit => "через транзитный узел",
        chimera_policy::OutboundMode::Block => "заблокировано правилом",
    }
}

fn status_command(lang: Language, lang_source: LanguageSource, args: &[String]) -> i32 {
    let options = match parse_status_options(args) {
        Ok(options) => options,
        Err(()) => {
            eprintln!("{}", status_usage(lang));
            return 2;
        }
    };
    let options = match apply_status_config_overrides(options) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => eprintln!("Config error: {error}"),
                Language::Ru => eprintln!("Ошибка конфигурации: {error}"),
            }
            return 2;
        }
    };
    let capture_plan = status_capture_plan(&options);
    if let Err(error) = validate_status_carrier(&options) {
        match lang {
            Language::En => eprintln!("Carrier config error: {error}"),
            Language::Ru => eprintln!("Ошибка конфигурации carrier: {error}"),
        }
        return 2;
    }

    let policy = match (RekeyPolicy {
        max_session_age_seconds: options.max_age_seconds,
        max_packets_per_key: options.max_packets_per_key,
    })
    .validate()
    {
        Ok(policy) => policy,
        Err(error) => {
            match lang {
                Language::En => eprintln!("Policy error: {error}"),
                Language::Ru => eprintln!("Ошибка политики: {error}"),
            }
            return 2;
        }
    };
    let mut rekey_state = match RekeyState::new(policy, 0) {
        Ok(state) => state,
        Err(error) => {
            match lang {
                Language::En => eprintln!("Could not start rekey state: {error}"),
                Language::Ru => eprintln!("Не удалось запустить состояние rekey: {error}"),
            }
            return 2;
        }
    };
    for _ in 0..options.mock_packets {
        rekey_state.on_packet_sent();
    }
    let reason = rekey_state.rekey_reason(options.mock_age_seconds);
    match lang {
        Language::En => {
            println!(
                "Status: {}.",
                if capture_plan_is_ready(&capture_plan) {
                    "ready"
                } else {
                    "fail-closed"
                }
            );
            println!("Network safety: system routes/DNS/firewall were not changed.");
            println!(
                "Language: EN (source: {})",
                language_source_label_en(lang_source)
            );
        }
        Language::Ru => {
            println!(
                "Статус: {}.",
                if capture_plan_is_ready(&capture_plan) {
                    "готово"
                } else {
                    "fail-closed"
                }
            );
            println!("Безопасность сети: системные маршруты/DNS/firewall не менялись.");
            println!(
                "Язык: RU (источник: {})",
                language_source_label_ru(lang_source)
            );
        }
    }
    print!(
        "{}",
        render_status_runtime_profile(lang, &options, &capture_plan)
    );
    print!("{}", render_status_rekey_block(lang, &options, reason));
    0
}

fn health_command(lang: Language, args: &[String]) -> i32 {
    let options = match parse_status_options(args) {
        Ok(options) => options,
        Err(()) => {
            eprintln!("{}", health_usage(lang));
            return 2;
        }
    };
    let options = match apply_status_config_overrides(options) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => eprintln!("Config error: {error}"),
                Language::Ru => eprintln!("Ошибка конфигурации: {error}"),
            }
            return 2;
        }
    };
    let capture_plan = status_capture_plan(&options);
    if let Err(error) = validate_status_carrier(&options) {
        match lang {
            Language::En => eprintln!("Carrier config error: {error}"),
            Language::Ru => eprintln!("Ошибка конфигурации carrier: {error}"),
        }
        return 2;
    }
    if let Err(error) = (RekeyPolicy {
        max_session_age_seconds: options.max_age_seconds,
        max_packets_per_key: options.max_packets_per_key,
    })
    .validate()
    {
        match lang {
            Language::En => eprintln!("Policy error: {error}"),
            Language::Ru => eprintln!("Ошибка политики: {error}"),
        }
        return 2;
    }
    print!("{}", render_health_block(lang, &options, &capture_plan));
    0
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DoctorOptions {
    status: StatusOptions,
    json_output: bool,
    out_path: Option<String>,
}

fn parse_doctor_options(args: &[String]) -> Result<DoctorOptions, ()> {
    let mut status_args: Vec<String> = Vec::new();
    let mut json_output = false;
    let mut out_path: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--json" => {
                json_output = true;
                index += 1;
            }
            "--out" => {
                let value = args.get(index + 1).ok_or(())?;
                out_path = Some(value.to_string());
                index += 2;
            }
            _ => {
                status_args.push(args[index].clone());
                if index + 1 < args.len() {
                    status_args.push(args[index + 1].clone());
                } else {
                    return Err(());
                }
                index += 2;
            }
        }
    }
    let status = parse_status_options(&status_args)?;
    Ok(DoctorOptions {
        status,
        json_output,
        out_path,
    })
}

fn doctor_command(lang: Language, args: &[String]) -> i32 {
    let options = match parse_doctor_options(args) {
        Ok(options) => options,
        Err(()) => {
            eprintln!("{}", doctor_usage(lang));
            return 2;
        }
    };
    let status_options = match apply_status_config_overrides(options.status) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => eprintln!("Config error: {error}"),
                Language::Ru => eprintln!("Ошибка конфигурации: {error}"),
            }
            return 2;
        }
    };
    let capture_plan = status_capture_plan(&status_options);
    if let Err(error) = validate_status_carrier(&status_options) {
        match lang {
            Language::En => eprintln!("Carrier config error: {error}"),
            Language::Ru => eprintln!("Ошибка конфигурации carrier: {error}"),
        }
        return 2;
    }
    let policy = match (RekeyPolicy {
        max_session_age_seconds: status_options.max_age_seconds,
        max_packets_per_key: status_options.max_packets_per_key,
    })
    .validate()
    {
        Ok(policy) => policy,
        Err(error) => {
            match lang {
                Language::En => eprintln!("Policy error: {error}"),
                Language::Ru => eprintln!("Ошибка политики: {error}"),
            }
            return 2;
        }
    };
    let mut rekey_state = match RekeyState::new(policy, 0) {
        Ok(state) => state,
        Err(error) => {
            match lang {
                Language::En => eprintln!("Could not start rekey state: {error}"),
                Language::Ru => eprintln!("Не удалось запустить состояние rekey: {error}"),
            }
            return 2;
        }
    };
    for _ in 0..status_options.mock_packets {
        rekey_state.on_packet_sent();
    }
    let reason = rekey_state.rekey_reason(status_options.mock_age_seconds);
    let json = render_doctor_json(&status_options, &capture_plan, reason);
    if let Some(path) = options.out_path
        && let Err(error) = std::fs::write(&path, &json)
    {
        eprintln!("doctor report write failed: {error}");
        return 1;
    }
    if options.json_output {
        println!("{json}");
    } else {
        print!(
            "{}",
            render_doctor_block(lang, &status_options, &capture_plan, reason)
        );
    }
    if capture_plan_is_ready(&capture_plan) {
        0
    } else {
        2
    }
}

fn status_capture_plan(options: &StatusOptions) -> CapturePlan {
    match options.capture_preference {
        CapturePreference::Auto => {
            let tun_supported = options.tun_supported && detect_tun_support();
            plan_capture_mode(tun_supported)
        }
        CapturePreference::Tun => CapturePlan {
            mode: CaptureMode::Tun,
            reason: "forced by CLI option".to_string(),
        },
    }
}

fn capture_plan_is_ready(capture_plan: &CapturePlan) -> bool {
    capture_plan.mode == CaptureMode::Tun
}

fn validate_status_carrier(options: &StatusOptions) -> Result<(), String> {
    match options.carrier_profile {
        CarrierProfile::InMemory => Ok(()),
        CarrierProfile::Tls => {
            let config = TlsCarrierConfig {
                server_name: options.carrier_server_name.clone(),
                connect_addr: options.carrier_addr.clone(),
                connect_timeout_ms: 3000,
            };
            TlsCarrier::new(config)
                .map(|_| ())
                .map_err(|error| error.to_string())
        }
        CarrierProfile::Quic => {
            let config = QuicCarrierConfig {
                server_name: options.carrier_server_name.clone(),
                connect_addr: options.carrier_addr.clone(),
                connect_timeout_ms: 3000,
            };
            QuicCarrier::new(config)
                .map(|_| ())
                .map_err(|error| error.to_string())
        }
    }
}

fn render_status_runtime_profile(
    lang: Language,
    options: &StatusOptions,
    capture_plan: &CapturePlan,
) -> String {
    let mut out = String::new();
    match lang {
        Language::En => {
            out.push_str("Runtime profile:\n");
            out.push_str(&format!(
                "Capture mode: {}\n",
                capture_mode_label_en(capture_plan.mode)
            ));
            out.push_str(&format!(
                "Capture reason: {}\n",
                capture_reason_label_en(&capture_plan.reason)
            ));
            out.push_str(&format!(
                "Carrier profile: {}\n",
                carrier_profile_label_en(options.carrier_profile)
            ));
            out.push_str(&format!(
                "Carrier target: {}, target_state={}\n",
                diagnostic_redaction::redacted_endpoint(),
                diagnostic_redaction::endpoint_state(&options.carrier_addr)
            ));
            out.push_str(&format!(
                "Carrier server name: {}, server_name_state={}\n",
                diagnostic_redaction::redacted_server_name(),
                diagnostic_redaction::server_name_state(&options.carrier_server_name)
            ));
        }
        Language::Ru => {
            out.push_str("Профиль запуска:\n");
            out.push_str(&format!(
                "Режим захвата: {}\n",
                capture_mode_label_ru(capture_plan.mode)
            ));
            out.push_str(&format!(
                "Причина режима: {}\n",
                capture_reason_label_ru(&capture_plan.reason)
            ));
            out.push_str(&format!(
                "Профиль carrier: {}\n",
                carrier_profile_label_ru(options.carrier_profile)
            ));
            out.push_str(&format!(
                "Цель carrier: {}, target_state={}\n",
                diagnostic_redaction::redacted_endpoint(),
                diagnostic_redaction::endpoint_state(&options.carrier_addr)
            ));
            out.push_str(&format!(
                "Имя сервера carrier: {}, server_name_state={}\n",
                diagnostic_redaction::redacted_server_name(),
                diagnostic_redaction::server_name_state(&options.carrier_server_name)
            ));
        }
    }
    out
}

fn capture_mode_label_en(mode: CaptureMode) -> &'static str {
    match mode {
        CaptureMode::Tun => "tun",
        CaptureMode::FailClosed => "fail-closed",
    }
}

fn capture_mode_label_ru(mode: CaptureMode) -> &'static str {
    match mode {
        CaptureMode::Tun => "tun",
        CaptureMode::FailClosed => "fail-closed",
    }
}

fn capture_reason_label_en(reason: &str) -> &str {
    match reason {
        "TUN is available on this system" => "TUN is available on this system",
        "TUN is unavailable; fail closed because proxy capture is forbidden" => {
            "TUN is unavailable; fail closed because proxy capture is forbidden"
        }
        "forced by CLI option" => "forced by CLI option",
        _ => reason,
    }
}

fn capture_reason_label_ru(reason: &str) -> &str {
    match reason {
        "TUN is available on this system" => "TUN доступен в системе",
        "TUN is unavailable; fail closed because proxy capture is forbidden" => {
            "TUN недоступен; закрытый отказ, потому что proxy-захват запрещен"
        }
        "forced by CLI option" => "принудительно задано в CLI",
        _ => reason,
    }
}

fn carrier_profile_label_en(profile: CarrierProfile) -> &'static str {
    match profile {
        CarrierProfile::InMemory => "in-memory",
        CarrierProfile::Tls => "tls-tcp",
        CarrierProfile::Quic => "quic",
    }
}

fn carrier_profile_label_ru(profile: CarrierProfile) -> &'static str {
    match profile {
        CarrierProfile::InMemory => "in-memory",
        CarrierProfile::Tls => "tls-tcp",
        CarrierProfile::Quic => "quic",
    }
}

fn language_source_label_en(source: LanguageSource) -> &'static str {
    match source {
        LanguageSource::Flag => "--lang",
        LanguageSource::Default => "default",
    }
}

fn language_source_label_ru(source: LanguageSource) -> &'static str {
    match source {
        LanguageSource::Flag => "--lang",
        LanguageSource::Default => "по умолчанию",
    }
}

fn parse_status_options(args: &[String]) -> Result<StatusOptions, ()> {
    let mut options = StatusOptions {
        config_path: None,
        mock_packets: 1,
        mock_age_seconds: 120,
        max_age_seconds: 300,
        max_packets_per_key: 10_000,
        capture_preference: CapturePreference::Auto,
        tun_supported: true,
        carrier_profile: CarrierProfile::InMemory,
        carrier_addr: DEFAULT_CARRIER_ADDR.to_string(),
        carrier_server_name: DEFAULT_CARRIER_SERVER_NAME.to_string(),
    };
    if args.is_empty() {
        return Ok(options);
    }
    let mut start_index = 0usize;
    if args.first().map(String::as_str) == Some("--config") {
        options.config_path = Some(args.get(1).cloned().ok_or(())?);
        start_index = 2;
    }
    if start_index >= args.len() {
        return Ok(options);
    }
    if args.get(start_index).map(String::as_str) != Some("--mock-traffic") {
        return Err(());
    }
    options.mock_packets = args
        .get(start_index + 1)
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or(())?;
    if args.get(start_index + 2).map(String::as_str) != Some("--age") {
        return Err(());
    }
    options.mock_age_seconds = args
        .get(start_index + 3)
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or(())?;
    let mut index = start_index + 4;
    if args.get(start_index + 4).map(String::as_str) == Some("--max-age") {
        options.max_age_seconds = args
            .get(start_index + 5)
            .and_then(|v| v.parse::<u64>().ok())
            .ok_or(())?;
        if args.get(start_index + 6).map(String::as_str) != Some("--max-packets") {
            return Err(());
        }
        options.max_packets_per_key = args
            .get(start_index + 7)
            .and_then(|v| v.parse::<u64>().ok())
            .ok_or(())?;
        index = start_index + 8;
    }
    if !(args.len() - index).is_multiple_of(2) {
        return Err(());
    }
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args.get(index + 1).map(String::as_str).ok_or(())?;
        match flag {
            "--capture" => {
                options.capture_preference = match value {
                    "auto" => CapturePreference::Auto,
                    "tun" => CapturePreference::Tun,
                    "local-proxy" => return Err(()),
                    _ => return Err(()),
                };
            }
            "--tun-supported" => {
                options.tun_supported = match value {
                    "true" => true,
                    "false" => false,
                    _ => return Err(()),
                };
            }
            "--carrier" => {
                options.carrier_profile = match value {
                    "in-memory" => CarrierProfile::InMemory,
                    "tls" => CarrierProfile::Tls,
                    "quic" => CarrierProfile::Quic,
                    _ => return Err(()),
                };
            }
            "--carrier-addr" => {
                options.carrier_addr = value.to_string();
            }
            "--server-name" => {
                options.carrier_server_name = value.to_string();
            }
            _ => return Err(()),
        }
        index += 2;
    }
    Ok(options)
}

fn apply_status_config_overrides(mut options: StatusOptions) -> Result<StatusOptions, String> {
    let Some(path) = options.config_path.as_deref() else {
        return Ok(options);
    };
    let file_content = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read config file: {error}"))?;
    let config = parse_node_config_text(&file_content).map_err(|error| error.to_string())?;

    options.carrier_profile = match config.carrier_profile {
        ConfigCarrierProfile::InMemory => CarrierProfile::InMemory,
        ConfigCarrierProfile::Tls => CarrierProfile::Tls,
        ConfigCarrierProfile::Quic => CarrierProfile::Quic,
    };
    options.carrier_addr = config.carrier_addr;
    options.carrier_server_name = config.carrier_server_name;
    options.capture_preference = match config.capture_mode {
        ConfigCaptureMode::Auto => CapturePreference::Auto,
        ConfigCaptureMode::Tun => CapturePreference::Tun,
    };
    options.tun_supported = config.tun_supported;
    options.max_age_seconds = config.rekey.max_age_seconds;
    options.max_packets_per_key = config.rekey.max_packets_per_key;

    Ok(options)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DiagExportOptions {
    config_path: Option<String>,
    age_seconds: u64,
    packets: u64,
    out_path: Option<String>,
}

impl Default for DiagExportOptions {
    fn default() -> Self {
        Self {
            config_path: None,
            age_seconds: 120,
            packets: 1,
            out_path: None,
        }
    }
}

fn parse_diag_export_options(args: &[String]) -> Result<DiagExportOptions, ()> {
    let mut options = DiagExportOptions::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args.get(index + 1).map(String::as_str).ok_or(())?;
        match flag {
            "--config" => options.config_path = Some(value.to_string()),
            "--age" => options.age_seconds = value.parse::<u64>().map_err(|_| ())?,
            "--packets" => options.packets = value.parse::<u64>().map_err(|_| ())?,
            "--out" => options.out_path = Some(value.to_string()),
            _ => return Err(()),
        }
        index += 2;
    }
    Ok(options)
}

fn diag_command(lang: Language, args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("rekey") => diag_rekey_command(
            lang,
            args.get(1).map(String::as_str),
            args.get(2).map(String::as_str),
        ),
        Some("export") => diag_export_command(lang, &args[1..]),
        _ => {
            match lang {
                Language::En => {
                    println!("Diagnostics: safe summary.");
                    println!("Secrets: <redacted>");
                    println!("Tip: use `chimera diag rekey <age_sec> <packets_sent>`");
                    println!("Tip: use `chimera diag export --out <file>`");
                }
                Language::Ru => {
                    println!("Диагностика: безопасная сводка.");
                    println!("Секреты: <redacted>");
                    println!("Подсказка: `chimera diag rekey <секунды> <пакеты>`");
                    println!("Подсказка: `chimera diag export --out <файл>`");
                }
            }
            0
        }
    }
}

fn diag_rekey_command(lang: Language, age_seconds: Option<&str>, packets: Option<&str>) -> i32 {
    let age_seconds = match age_seconds.and_then(|v| v.parse::<u64>().ok()) {
        Some(value) => value,
        None => {
            eprintln!("{}", diag_rekey_usage(lang));
            return 2;
        }
    };
    let packets = match packets.and_then(|v| v.parse::<u64>().ok()) {
        Some(value) => value,
        None => {
            eprintln!("{}", diag_rekey_usage(lang));
            return 2;
        }
    };

    let policy = match (RekeyPolicy {
        max_session_age_seconds: 300,
        max_packets_per_key: 10_000,
    })
    .validate()
    {
        Ok(policy) => policy,
        Err(error) => {
            match lang {
                Language::En => eprintln!("Could not load policy: {error}"),
                Language::Ru => eprintln!("Не удалось загрузить политику: {error}"),
            }
            return 2;
        }
    };
    let mut state = match RekeyState::new(policy, 0) {
        Ok(state) => state,
        Err(error) => {
            match lang {
                Language::En => eprintln!("Could not start rekey state: {error}"),
                Language::Ru => eprintln!("Не удалось запустить состояние rekey: {error}"),
            }
            return 2;
        }
    };
    for _ in 0..packets {
        state.on_packet_sent();
    }

    let reason = state.rekey_reason(age_seconds);
    print!(
        "{}",
        render_diag_rekey_block(lang, age_seconds, packets, reason)
    );
    0
}

fn diag_export_command(lang: Language, args: &[String]) -> i32 {
    let options = match parse_diag_export_options(args) {
        Ok(options) => options,
        Err(()) => {
            eprintln!("{}", diag_export_usage(lang));
            return 2;
        }
    };

    let mut status_options = StatusOptions {
        config_path: options.config_path.clone(),
        mock_packets: options.packets,
        mock_age_seconds: options.age_seconds,
        max_age_seconds: 300,
        max_packets_per_key: 10_000,
        capture_preference: CapturePreference::Auto,
        tun_supported: true,
        carrier_profile: CarrierProfile::InMemory,
        carrier_addr: DEFAULT_CARRIER_ADDR.to_string(),
        carrier_server_name: DEFAULT_CARRIER_SERVER_NAME.to_string(),
    };
    status_options = match apply_status_config_overrides(status_options) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => eprintln!("Config error: {error}"),
                Language::Ru => eprintln!("Ошибка конфигурации: {error}"),
            }
            return 2;
        }
    };
    let capture_plan = status_capture_plan(&status_options);
    let rekey_reason = {
        let policy = match (RekeyPolicy {
            max_session_age_seconds: status_options.max_age_seconds,
            max_packets_per_key: status_options.max_packets_per_key,
        })
        .validate()
        {
            Ok(policy) => policy,
            Err(error) => {
                eprintln!("{error}");
                return 2;
            }
        };
        let mut state = match RekeyState::new(policy, 0) {
            Ok(state) => state,
            Err(error) => {
                eprintln!("{error}");
                return 2;
            }
        };
        for _ in 0..status_options.mock_packets {
            state.on_packet_sent();
        }
        state.rekey_reason(status_options.mock_age_seconds)
    };
    let json = render_diag_export_json(&status_options, &capture_plan, rekey_reason);
    if let Some(path) = options.out_path {
        if let Err(error) = std::fs::write(&path, &json) {
            eprintln!("diagnostic export write failed: {error}");
            return 1;
        }
        match lang {
            Language::En => println!("Diagnostic export saved: {path}"),
            Language::Ru => println!("Экспорт диагностики сохранен: {path}"),
        }
    }
    println!("{json}");
    0
}

fn render_diag_export_json(
    status: &StatusOptions,
    capture_plan: &CapturePlan,
    rekey_reason: Option<RekeyReason>,
) -> String {
    let reason = match rekey_reason {
        Some(RekeyReason::SessionAgeExceeded) => "session_age_exceeded",
        Some(RekeyReason::PacketLimitExceeded) => "packet_limit_exceeded",
        None => "none",
    };
    let report_status = if capture_plan_is_ready(capture_plan) {
        "ok"
    } else {
        "fail_closed"
    };
    format!(
        "{{\"status\":\"{}\",\"kind\":\"diag_export\",\"message_en\":\"Diagnostic export is ready.\",\"message_ru\":\"Экспорт диагностики готов.\",\"secrets\":\"<redacted>\",\"capture_mode\":\"{}\",\"capture_reason\":\"{}\",\"carrier_profile\":\"{}\",\"carrier_addr\":\"{}\",\"carrier_endpoint_state\":\"{}\",\"carrier_server_name\":\"{}\",\"carrier_server_name_state\":\"{}\",\"session_age_sec\":{},\"packets\":{},\"rekey_required\":{},\"rekey_reason\":\"{}\",\"network_state\":\"not_modified\"}}",
        report_status,
        capture_mode_label_en(capture_plan.mode),
        escape_json(&capture_plan.reason),
        carrier_profile_label_en(status.carrier_profile),
        diagnostic_redaction::redacted_endpoint(),
        diagnostic_redaction::endpoint_state(&status.carrier_addr),
        diagnostic_redaction::redacted_server_name(),
        diagnostic_redaction::server_name_state(&status.carrier_server_name),
        status.mock_age_seconds,
        status.mock_packets,
        rekey_reason.is_some(),
        reason
    )
}

fn render_status_rekey_block(
    lang: Language,
    options: &StatusOptions,
    reason: Option<RekeyReason>,
) -> String {
    let mut out = String::new();
    match lang {
        Language::En => {
            out.push_str(&format!(
                "Session age (sec): {}\n",
                options.mock_age_seconds
            ));
            out.push_str(&format!("Packets sent: {}\n", options.mock_packets));
            out.push_str(&format!("Max age (sec): {}\n", options.max_age_seconds));
            out.push_str(&format!("Max packets: {}\n", options.max_packets_per_key));
            out.push_str(&format!("Need new key: {}\n", reason.is_some()));
            out.push_str("Why: ");
            out.push_str(match reason {
                Some(RekeyReason::SessionAgeExceeded) => "session is too old",
                Some(RekeyReason::PacketLimitExceeded) => "too many packets",
                None => "all good",
            });
            out.push('\n');
        }
        Language::Ru => {
            out.push_str(&format!(
                "Возраст сессии (сек): {}\n",
                options.mock_age_seconds
            ));
            out.push_str(&format!("Отправлено пакетов: {}\n", options.mock_packets));
            out.push_str(&format!(
                "Предел возраста (сек): {}\n",
                options.max_age_seconds
            ));
            out.push_str(&format!(
                "Предел пакетов: {}\n",
                options.max_packets_per_key
            ));
            out.push_str(&format!("Нужен новый ключ: {}\n", reason.is_some()));
            out.push_str("Почему: ");
            out.push_str(match reason {
                Some(RekeyReason::SessionAgeExceeded) => "сессия слишком старая",
                Some(RekeyReason::PacketLimitExceeded) => "слишком много пакетов",
                None => "все в норме",
            });
            out.push('\n');
        }
    }
    out
}

fn render_health_block(
    lang: Language,
    options: &StatusOptions,
    capture_plan: &CapturePlan,
) -> String {
    let mut out = String::new();
    let ready = capture_plan_is_ready(capture_plan);
    match lang {
        Language::En => {
            out.push_str(if ready {
                "Node check: ok\n"
            } else {
                "Node check: fail-closed\n"
            });
            out.push_str("Checks:\n");
            out.push_str("  - Config parse: ok\n");
            out.push_str("  - Carrier setup: ok\n");
            out.push_str("  - Rekey limits: ok\n");
            out.push_str(&format!(
                "  - Capture datapath: {}\n",
                if ready { "ok" } else { "fail-closed" }
            ));
            out.push_str(&format!(
                "Summary: capture={}, carrier={}, target={}, target_state={}\n",
                capture_mode_label_en(capture_plan.mode),
                carrier_profile_label_en(options.carrier_profile),
                diagnostic_redaction::redacted_endpoint(),
                diagnostic_redaction::endpoint_state(&options.carrier_addr)
            ));
            if !ready {
                out.push_str(&format!(
                    "Reason: {}\n",
                    capture_reason_label_en(&capture_plan.reason)
                ));
            }
            out.push_str("Network state: not modified\n");
        }
        Language::Ru => {
            out.push_str(if ready {
                "Проверка узла: ok\n"
            } else {
                "Проверка узла: fail-closed\n"
            });
            out.push_str("Проверки:\n");
            out.push_str("  - Чтение конфига: ok\n");
            out.push_str("  - Настройка carrier: ok\n");
            out.push_str("  - Лимиты rekey: ok\n");
            out.push_str(&format!(
                "  - Захват datapath: {}\n",
                if ready { "ok" } else { "fail-closed" }
            ));
            out.push_str(&format!(
                "Сводка: capture={}, carrier={}, target={}, target_state={}\n",
                capture_mode_label_ru(capture_plan.mode),
                carrier_profile_label_ru(options.carrier_profile),
                diagnostic_redaction::redacted_endpoint(),
                diagnostic_redaction::endpoint_state(&options.carrier_addr)
            ));
            if !ready {
                out.push_str(&format!(
                    "Причина: {}\n",
                    capture_reason_label_ru(&capture_plan.reason)
                ));
            }
            out.push_str("Состояние сети: не изменялось\n");
        }
    }
    out
}

fn render_doctor_block(
    lang: Language,
    options: &StatusOptions,
    capture_plan: &CapturePlan,
    reason: Option<RekeyReason>,
) -> String {
    let mut out = String::new();
    let ready = capture_plan_is_ready(capture_plan);
    match lang {
        Language::En => {
            out.push_str(if ready {
                "Doctor report: ok\n"
            } else {
                "Doctor report: fail-closed\n"
            });
            out.push_str("Checks:\n");
            out.push_str("  - Config parse: ok\n");
            out.push_str("  - Carrier setup: ok\n");
            out.push_str("  - Rekey policy: ok\n");
            out.push_str(&format!(
                "  - Capture datapath: {}\n",
                if ready { "ok" } else { "fail-closed" }
            ));
            out.push_str(&format!(
                "Summary: capture={}, carrier={}, target={}, target_state={}\n",
                capture_mode_label_en(capture_plan.mode),
                carrier_profile_label_en(options.carrier_profile),
                diagnostic_redaction::redacted_endpoint(),
                diagnostic_redaction::endpoint_state(&options.carrier_addr)
            ));
            if !ready {
                out.push_str(&format!(
                    "Reason: {}\n",
                    capture_reason_label_en(&capture_plan.reason)
                ));
            }
            out.push_str(&format!("Rekey needed now: {}\n", reason.is_some()));
            out.push_str("Secrets: <redacted>\n");
            out.push_str("Network state: not modified\n");
        }
        Language::Ru => {
            out.push_str(if ready {
                "Отчет doctor: ok\n"
            } else {
                "Отчет doctor: fail-closed\n"
            });
            out.push_str("Проверки:\n");
            out.push_str("  - Чтение конфига: ok\n");
            out.push_str("  - Настройка carrier: ok\n");
            out.push_str("  - Политика rekey: ok\n");
            out.push_str(&format!(
                "  - Захват datapath: {}\n",
                if ready { "ok" } else { "fail-closed" }
            ));
            out.push_str(&format!(
                "Сводка: capture={}, carrier={}, target={}, target_state={}\n",
                capture_mode_label_ru(capture_plan.mode),
                carrier_profile_label_ru(options.carrier_profile),
                diagnostic_redaction::redacted_endpoint(),
                diagnostic_redaction::endpoint_state(&options.carrier_addr)
            ));
            if !ready {
                out.push_str(&format!(
                    "Причина: {}\n",
                    capture_reason_label_ru(&capture_plan.reason)
                ));
            }
            out.push_str(&format!("Нужен rekey сейчас: {}\n", reason.is_some()));
            out.push_str("Секреты: <redacted>\n");
            out.push_str("Состояние сети: не изменялось\n");
        }
    }
    out
}

fn render_doctor_json(
    status: &StatusOptions,
    capture_plan: &CapturePlan,
    rekey_reason: Option<RekeyReason>,
) -> String {
    let reason = match rekey_reason {
        Some(RekeyReason::SessionAgeExceeded) => "session_age_exceeded",
        Some(RekeyReason::PacketLimitExceeded) => "packet_limit_exceeded",
        None => "none",
    };
    let report_status = if capture_plan_is_ready(capture_plan) {
        "ok"
    } else {
        "fail_closed"
    };
    format!(
        "{{\"status\":\"{}\",\"kind\":\"doctor\",\"message_en\":\"Doctor check is ready.\",\"message_ru\":\"Проверка doctor готова.\",\"secrets\":\"<redacted>\",\"capture_mode\":\"{}\",\"capture_reason\":\"{}\",\"carrier_profile\":\"{}\",\"carrier_addr\":\"{}\",\"carrier_endpoint_state\":\"{}\",\"session_age_sec\":{},\"packets\":{},\"rekey_required\":{},\"rekey_reason\":\"{}\",\"network_state\":\"not_modified\"}}",
        report_status,
        capture_mode_label_en(capture_plan.mode),
        escape_json(&capture_plan.reason),
        carrier_profile_label_en(status.carrier_profile),
        diagnostic_redaction::redacted_endpoint(),
        diagnostic_redaction::endpoint_state(&status.carrier_addr),
        status.mock_age_seconds,
        status.mock_packets,
        rekey_reason.is_some(),
        reason
    )
}

fn render_diag_rekey_block(
    lang: Language,
    age_seconds: u64,
    packets: u64,
    reason: Option<RekeyReason>,
) -> String {
    let mut out = String::new();
    match lang {
        Language::En => {
            out.push_str("Policy max age (sec): 300\n");
            out.push_str("Policy max packets: 10000\n");
            out.push_str(&format!("Session age (sec): {age_seconds}\n"));
            out.push_str(&format!("Packets with current key: {packets}\n"));
            out.push_str(&format!("Need new key: {}\n", reason.is_some()));
            out.push_str("Why: ");
            out.push_str(match reason {
                Some(RekeyReason::SessionAgeExceeded) => "session is too old",
                Some(RekeyReason::PacketLimitExceeded) => "too many packets",
                None => "all good",
            });
            out.push('\n');
        }
        Language::Ru => {
            out.push_str("Предел возраста по политике (сек): 300\n");
            out.push_str("Предел пакетов по политике: 10000\n");
            out.push_str(&format!("Возраст сессии (сек): {age_seconds}\n"));
            out.push_str(&format!("Пакетов с текущим ключом: {packets}\n"));
            out.push_str(&format!("Нужен новый ключ: {}\n", reason.is_some()));
            out.push_str("Почему: ");
            out.push_str(match reason {
                Some(RekeyReason::SessionAgeExceeded) => "сессия слишком старая",
                Some(RekeyReason::PacketLimitExceeded) => "слишком много пакетов",
                None => "все в норме",
            });
            out.push('\n');
        }
    }
    out
}

fn print_help(lang: Language) {
    print!("{}", render_help_text(lang));
}

fn render_help_text(lang: Language) -> String {
    let mut out = String::new();
    match lang {
        Language::En => {
            out.push_str("Chimera CLI (simple mode)\n");
            out.push_str("Commands:\n");
            out.push_str("  chimera [--lang en|ru] status [--config <node_config_file>] [--mock-traffic <packets> --age <seconds> --max-age <seconds> --max-packets <count>] [--capture <auto|tun>] [--tun-supported <true|false>] [--carrier <in-memory|tls|quic>] [--carrier-addr <host:port>] [--server-name <name>]\n");
            out.push_str("  chimera [--lang en|ru] health [--config <node_config_file>] [--capture <auto|tun>] [--tun-supported <true|false>] [--carrier <in-memory|tls|quic>] [--carrier-addr <host:port>] [--server-name <name>]\n");
            out.push_str("  chimera [--lang en|ru] doctor [--config <node_config_file>] [--mock-traffic <packets> --age <seconds> --max-age <seconds> --max-packets <count>] [--capture <auto|tun>] [--tun-supported <true|false>] [--carrier <in-memory|tls|quic>] [--carrier-addr <host:port>] [--server-name <name>] [--json] [--out <file>]\n");
            out.push_str("  chimera [--lang en|ru] up\n");
            out.push_str("  chimera [--lang en|ru] down\n");
            out.push_str("  chimera [--lang en|ru] mvp-check\n");
            out.push_str("  chimera [--lang en|ru] mvp-verify [extra args]\n");
            out.push_str("  chimera [--lang en|ru] mvp-snapshot [extra args]\n");
            out.push_str("  chimera [--lang en|ru] mvp-spec-check [extra args]\n");
            out.push_str("  chimera [--lang en|ru] mvp-spec-report [extra args]\n");
            out.push_str("  chimera [--lang en|ru] m5-artifacts-report [extra args]\n");
            out.push_str("  chimera [--lang en|ru] m6-artifacts-report [extra args]\n");
            out.push_str("  chimera [--lang en|ru] lab-smoke [extra args]\n");
            out.push_str("  chimera [--lang en|ru] lab-doctor [extra args]\n");
            out.push_str("  chimera [--lang en|ru] lab-hardening-smoke [extra args]\n");
            out.push_str("  chimera [--lang en|ru] hardening-smoke\n");
            out.push_str("  chimera [--lang en|ru] benchmark-report [extra args]\n");
            out.push_str("  chimera [--lang en|ru] benchmark-regression-check\n");
            out.push_str("  chimera [--lang en|ru] net-sim\n");
            out.push_str("  chimera [--lang en|ru] perf-smoke\n");
            out.push_str("  chimera [--lang en|ru] fuzz-smoke\n");
            out.push_str("  chimera [--lang en|ru] config-smoke\n");
            out.push_str("  chimera [--lang en|ru] release-readiness-report [extra args]\n");
            out.push_str("  chimera [--lang en|ru] artifact-audit [extra args]\n");
            out.push_str("  chimera [--lang en|ru] report-pack [extra args]\n");
            out.push_str(
                "  chimera [--lang en|ru] rollback <status|clean|recover> [--state-file <file>] [--json] [--out <file>]\n",
            );
            out.push_str("  chimera [--lang en|ru] state proof [--state-file <file>] [--flow-file <file>] [--require-flow <true|false>] [--max-flow-age-sec <seconds>]\n");
            out.push_str(
                "  chimera [--lang en|ru] lab <smoke|doctor|config-smoke|fuzz-smoke|perf-smoke|net-sim|benchmark-report|benchmark-regression-check|hardening-smoke|mvp-spec-check|mvp-spec-report|m5-artifacts-report|m6-artifacts-report|release-readiness-report|report-pack|artifact-audit|mvp-snapshot|mvp-verify|mvp-check> [extra args for chimera-lab]\n",
            );
            out.push_str("  chimera [--lang en|ru] mvp-check\n");
            out.push_str("  chimera [--lang en|ru] route explain [domain] [--domain <domain>] [--policy <policy_file>] [--ip <ipv4|ipv6>] [--proto <tcp|udp|icmp>] [--port <n>] [--dns-bind-domain <domain>] [--dns-bind-ip <ipv4|ipv6>] [--show-all-matches] [--json] [--out <file>]\n");
            out.push_str("  chimera [--lang en|ru] nodes [--country DE,NL] [--status healthy,checking] [--available-only] [--search text]\n");
            out.push_str("  chimera [--lang en|ru] connect <index|node_id> [--country DE,NL] [--status healthy,checking]\n");
            out.push_str("  chimera [--lang en|ru] pin <index|node_id> [--country DE,NL] [--status healthy,checking]\n");
            out.push_str("  chimera [--lang en|ru] mesh <route-explain|connect-probe|launch-preflight|launch-preflight-verify> --namespace <name> --node <name> (--policy-payload <payload> | --traffic-profile <high_speed_anonymous|privacy_first|speed_first|low_latency_private>) --peer <node@endpoint#region@load@reliability> [--peer ...] [--invite-token <token>] [--failed-node <id>] [--cooldown-node <id>] [--table-max-entries <n>] [--table-max-per-region <n>] [--table-stale-after <ticks>] [--timeout-ms <n>] [--json] [--out <file>]\n");
            out.push_str("  chimera [--lang en|ru] mesh launch-preflight-verify --side-a-report <file> --side-b-report <file> [--json] [--out <file>]\n");
            out.push_str("  chimera [--lang en|ru] policy validate <policy_file>\n");
            out.push_str("  chimera [--lang en|ru] probe access --url <http|https_url> [--url-file <file>] [--timeout-sec <n>] [--apply-policy <file>] [--rule-id-prefix <prefix>] [--fail-threshold <n>] [--json] [--out <file>]\n");
            out.push_str("  chimera [--lang en|ru] diag rekey <age_sec> <packets_sent>\n");
            out.push_str("  chimera [--lang en|ru] diag export [--config <node_config_file>] [--age <seconds>] [--packets <count>] [--out <file>]\n");
        }
        Language::Ru => {
            out.push_str("Chimera CLI (простой режим)\n");
            out.push_str("Команды:\n");
            out.push_str("  chimera [--lang en|ru] status [--config <файл_node_config>] [--mock-traffic <пакеты> --age <секунды> --max-age <секунды> --max-packets <число>] [--capture <auto|tun>] [--tun-supported <true|false>] [--carrier <in-memory|tls|quic>] [--carrier-addr <хост:порт>] [--server-name <имя>]\n");
            out.push_str("  chimera [--lang en|ru] health [--config <файл_node_config>] [--capture <auto|tun>] [--tun-supported <true|false>] [--carrier <in-memory|tls|quic>] [--carrier-addr <хост:порт>] [--server-name <имя>]\n");
            out.push_str("  chimera [--lang en|ru] doctor [--config <файл_node_config>] [--mock-traffic <пакеты> --age <секунды> --max-age <секунды> --max-packets <число>] [--capture <auto|tun>] [--tun-supported <true|false>] [--carrier <in-memory|tls|quic>] [--carrier-addr <хост:порт>] [--server-name <имя>] [--json] [--out <файл>]\n");
            out.push_str("  chimera [--lang en|ru] up\n");
            out.push_str("  chimera [--lang en|ru] down\n");
            out.push_str("  chimera [--lang en|ru] mvp-check\n");
            out.push_str("  chimera [--lang en|ru] mvp-verify [доп. аргументы]\n");
            out.push_str("  chimera [--lang en|ru] mvp-snapshot [доп. аргументы]\n");
            out.push_str("  chimera [--lang en|ru] mvp-spec-check [доп. аргументы]\n");
            out.push_str("  chimera [--lang en|ru] mvp-spec-report [доп. аргументы]\n");
            out.push_str("  chimera [--lang en|ru] m5-artifacts-report [доп. аргументы]\n");
            out.push_str("  chimera [--lang en|ru] m6-artifacts-report [доп. аргументы]\n");
            out.push_str("  chimera [--lang en|ru] lab-smoke [доп. аргументы]\n");
            out.push_str("  chimera [--lang en|ru] lab-doctor [доп. аргументы]\n");
            out.push_str("  chimera [--lang en|ru] lab-hardening-smoke [доп. аргументы]\n");
            out.push_str("  chimera [--lang en|ru] hardening-smoke\n");
            out.push_str("  chimera [--lang en|ru] benchmark-report [доп. аргументы]\n");
            out.push_str("  chimera [--lang en|ru] benchmark-regression-check\n");
            out.push_str("  chimera [--lang en|ru] net-sim\n");
            out.push_str("  chimera [--lang en|ru] perf-smoke\n");
            out.push_str("  chimera [--lang en|ru] fuzz-smoke\n");
            out.push_str("  chimera [--lang en|ru] config-smoke\n");
            out.push_str("  chimera [--lang en|ru] release-readiness-report [доп. аргументы]\n");
            out.push_str("  chimera [--lang en|ru] artifact-audit [доп. аргументы]\n");
            out.push_str("  chimera [--lang en|ru] report-pack [доп. аргументы]\n");
            out.push_str(
                "  chimera [--lang en|ru] rollback <status|clean|recover> [--state-file <файл>] [--json] [--out <файл>]\n",
            );
            out.push_str("  chimera [--lang en|ru] state proof [--state-file <файл>] [--flow-file <файл>] [--require-flow <true|false>] [--max-flow-age-sec <секунды>]\n");
            out.push_str(
                "  chimera [--lang en|ru] lab <smoke|doctor|config-smoke|fuzz-smoke|perf-smoke|net-sim|benchmark-report|benchmark-regression-check|hardening-smoke|mvp-spec-check|mvp-spec-report|m5-artifacts-report|m6-artifacts-report|release-readiness-report|report-pack|artifact-audit|mvp-snapshot|mvp-verify|mvp-check> [доп. аргументы для chimera-lab]\n",
            );
            out.push_str("  chimera [--lang en|ru] mvp-check\n");
            out.push_str("  chimera [--lang en|ru] route explain [домен] [--domain <домен>] [--policy <файл_policy>] [--ip <ipv4|ipv6>] [--proto <tcp|udp|icmp>] [--port <число>] [--dns-bind-domain <домен>] [--dns-bind-ip <ipv4|ipv6>] [--show-all-matches] [--json] [--out <файл>]\n");
            out.push_str("  chimera [--lang en|ru] nodes [--country DE,NL] [--status healthy,checking] [--available-only] [--search text]\n");
            out.push_str("  chimera [--lang en|ru] connect <index|node_id> [--country DE,NL] [--status healthy,checking]\n");
            out.push_str("  chimera [--lang en|ru] pin <index|node_id> [--country DE,NL] [--status healthy,checking]\n");
            out.push_str("  chimera [--lang en|ru] mesh <route-explain|connect-probe|launch-preflight|launch-preflight-verify> --namespace <имя> --node <имя> (--policy-payload <payload> | --traffic-profile <high_speed_anonymous|privacy_first|speed_first|low_latency_private>) --peer <node@endpoint#region@load@reliability> [--peer ...] [--invite-token <токен>] [--failed-node <id>] [--cooldown-node <id>] [--table-max-entries <n>] [--table-max-per-region <n>] [--table-stale-after <ticks>] [--timeout-ms <n>] [--json] [--out <файл>]\n");
            out.push_str("  chimera [--lang en|ru] mesh launch-preflight-verify --side-a-report <файл> --side-b-report <файл> [--json] [--out <файл>]\n");
            out.push_str("  chimera [--lang en|ru] policy validate <файл_policy>\n");
            out.push_str("  chimera [--lang en|ru] probe access --url <http|https_url> [--url-file <файл>] [--timeout-sec <n>] [--apply-policy <файл>] [--rule-id-prefix <префикс>] [--fail-threshold <n>] [--json] [--out <файл>]\n");
            out.push_str("  chimera [--lang en|ru] diag rekey <секунды> <пакеты>\n");
            out.push_str("  chimera [--lang en|ru] diag export [--config <файл_node_config>] [--age <секунды>] [--packets <число>] [--out <файл>]\n");
        }
    }
    out
}

fn render_unknown_command(lang: Language, command: &str) -> String {
    match lang {
        Language::En => format!("Unknown command: {command}"),
        Language::Ru => format!("Неизвестная команда: {command}"),
    }
}

fn parse_optional_fwmark(value: &str) -> Option<u32> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
        u32::from_str_radix(&trimmed[2..], 16).ok()
    } else {
        trimmed.parse::<u32>().ok()
    }
}

fn parse_up_down_options(args: &[String]) -> Result<UpDownOptions, ()> {
    let mut options = UpDownOptions {
        state_path: "docs/runtime_state_latest.json".to_string(),
        config_path: None,
        skip_connect_check: false,
        apply_tun: false,
        tun_name: "chimera0".to_string(),
        tun_local_cidr: "10.201.0.2/30".to_string(),
        tun_peer_cidr: "10.201.0.1/30".to_string(),
        apply_route: false,
        route_cidr: "0.0.0.0/1".to_string(),
        route_policy: false,
        route_table: "51820".to_string(),
        route_rule_priority: "11000".to_string(),
        route_fwmark: std::env::var("CHIMERA_ROUTE_FWMARK")
            .ok()
            .and_then(|value| parse_optional_fwmark(&value)),
        apply_dns: false,
        dns_server: "1.1.1.1".to_string(),
        resolv_conf_path: "/etc/resolv.conf".to_string(),
    };
    if args.is_empty() {
        return Ok(options);
    }
    let mut index = 0usize;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--state-file" => {
                options.state_path = args.get(index + 1).cloned().ok_or(())?;
                index += 2;
            }
            "--config" => {
                options.config_path = Some(args.get(index + 1).cloned().ok_or(())?);
                index += 2;
            }
            "--skip-connect-check" => {
                options.skip_connect_check = match args.get(index + 1).map(String::as_str) {
                    Some("true") => true,
                    Some("false") => false,
                    _ => return Err(()),
                };
                index += 2;
            }
            "--apply-tun" => {
                options.apply_tun = match args.get(index + 1).map(String::as_str) {
                    Some("true") => true,
                    Some("false") => false,
                    _ => return Err(()),
                };
                index += 2;
            }
            "--tun-name" => {
                options.tun_name = args.get(index + 1).cloned().ok_or(())?;
                index += 2;
            }
            "--tun-local-cidr" => {
                options.tun_local_cidr = args.get(index + 1).cloned().ok_or(())?;
                index += 2;
            }
            "--tun-peer-cidr" => {
                options.tun_peer_cidr = args.get(index + 1).cloned().ok_or(())?;
                index += 2;
            }
            "--apply-route" => {
                options.apply_route = match args.get(index + 1).map(String::as_str) {
                    Some("true") => true,
                    Some("false") => false,
                    _ => return Err(()),
                };
                index += 2;
            }
            "--route-cidr" => {
                options.route_cidr = args.get(index + 1).cloned().ok_or(())?;
                index += 2;
            }
            "--route-policy" => {
                options.route_policy = match args.get(index + 1).map(String::as_str) {
                    Some("true") => true,
                    Some("false") => false,
                    _ => return Err(()),
                };
                index += 2;
            }
            "--route-table" => {
                options.route_table = args.get(index + 1).cloned().ok_or(())?;
                index += 2;
            }
            "--route-rule-priority" => {
                options.route_rule_priority = args.get(index + 1).cloned().ok_or(())?;
                index += 2;
            }
            "--route-fwmark" => {
                let value = args.get(index + 1).cloned().ok_or(())?;
                options.route_fwmark = parse_optional_fwmark(&value);
                if options.route_fwmark.is_none() {
                    return Err(());
                }
                index += 2;
            }
            "--apply-dns" => {
                options.apply_dns = match args.get(index + 1).map(String::as_str) {
                    Some("true") => true,
                    Some("false") => false,
                    _ => return Err(()),
                };
                index += 2;
            }
            "--dns-server" => {
                options.dns_server = args.get(index + 1).cloned().ok_or(())?;
                index += 2;
            }
            "--resolv-conf" => {
                options.resolv_conf_path = args.get(index + 1).cloned().ok_or(())?;
                index += 2;
            }
            _ => return Err(()),
        }
    }
    if options.route_policy && !options.apply_route {
        return Err(());
    }
    if options.route_fwmark.is_some() && !options.route_policy {
        return Err(());
    }
    Ok(options)
}

fn parse_rollback_options(args: &[String]) -> Result<RollbackOptions, ()> {
    let mut up_down_args: Vec<String> = Vec::new();
    let mut json_output = false;
    let mut out_path: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--json" => {
                json_output = true;
                index += 1;
            }
            "--out" => {
                let value = args.get(index + 1).ok_or(())?;
                out_path = Some(value.to_string());
                index += 2;
            }
            "--state-file" => {
                up_down_args.push(args[index].clone());
                let value = args.get(index + 1).ok_or(())?;
                up_down_args.push(value.clone());
                index += 2;
            }
            _ => return Err(()),
        }
    }
    let up_down = parse_up_down_options(&up_down_args)?;
    Ok(RollbackOptions {
        up_down,
        json_output,
        out_path,
    })
}

fn parse_state_proof_options(args: &[String]) -> Result<StateProofOptions, ()> {
    let mut state_path = "docs/runtime_state_latest.json".to_string();
    let mut flow_path = None;
    let mut require_flow = false;
    let mut max_flow_age_sec = 300_u64;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--state-file" => {
                state_path = args.get(index + 1).cloned().ok_or(())?;
                index += 2;
            }
            "--flow-file" => {
                flow_path = Some(args.get(index + 1).cloned().ok_or(())?);
                index += 2;
            }
            "--require-flow" => {
                let raw = args.get(index + 1).cloned().ok_or(())?;
                require_flow = parse_bool_flag(&raw).ok_or(())?;
                index += 2;
            }
            "--max-flow-age-sec" => {
                let raw = args.get(index + 1).cloned().ok_or(())?;
                let parsed = raw.parse::<u64>().map_err(|_| ())?;
                if parsed == 0 {
                    return Err(());
                }
                max_flow_age_sec = parsed;
                index += 2;
            }
            _ => return Err(()),
        }
    }
    Ok(StateProofOptions {
        state_path,
        flow_path,
        require_flow,
        max_flow_age_sec,
    })
}

fn parse_bool_flag(raw: &str) -> Option<bool> {
    match raw {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[allow(clippy::collapsible_if)]
fn up_command(lang: Language, args: &[String]) -> i32 {
    let options = match parse_up_down_options(args) {
        Ok(options) => options,
        Err(()) => {
            eprintln!("{}", up_usage(lang));
            return 2;
        }
    };
    if let Some(parent) = Path::new(&options.state_path).parent()
        && !parent.as_os_str().is_empty()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        match lang {
            Language::En => eprintln!("Cannot create state directory: {error}"),
            Language::Ru => eprintln!("Не удалось создать папку state: {error}"),
        }
        return 1;
    }
    let runtime_state = match build_up_runtime_state(&options) {
        Ok(state) => state,
        Err(error) => {
            match lang {
                Language::En => eprintln!("Cannot prepare runtime state: {error}"),
                Language::Ru => eprintln!("Не удалось подготовить runtime state: {error}"),
            }
            return 2;
        }
    };
    if options.apply_route
        && options.route_policy
        && let Err(error) =
            validate_policy_route_options(&options.route_table, &options.route_rule_priority)
    {
        match lang {
            Language::En => eprintln!("Invalid policy route options: {error}"),
            Language::Ru => eprintln!("Некорректные параметры policy route: {error}"),
        }
        return 2;
    }
    if options.apply_route
        && let Err(error) = validate_route_cidrs(&options.route_cidr)
    {
        match lang {
            Language::En => eprintln!("Invalid route CIDR: {error}"),
            Language::Ru => eprintln!("Некорректный route CIDR: {error}"),
        }
        return 2;
    }
    if (options.apply_tun || options.apply_route)
        && let Err(error) = validate_tun_name(&options.tun_name)
    {
        match lang {
            Language::En => eprintln!("Invalid TUN interface name: {error}"),
            Language::Ru => eprintln!("Некорректное имя TUN-интерфейса: {error}"),
        }
        return 2;
    }
    if options.apply_tun
        && let Err(error) = validate_tun_cidr_pair(&options.tun_local_cidr, &options.tun_peer_cidr)
    {
        match lang {
            Language::En => eprintln!("Invalid TUN CIDR settings: {error}"),
            Language::Ru => eprintln!("Некорректные TUN CIDR параметры: {error}"),
        }
        return 2;
    }
    if options.apply_dns
        && let Err(error) = validate_dns_server(&options.dns_server)
    {
        match lang {
            Language::En => eprintln!("Invalid DNS server IP: {error}"),
            Language::Ru => eprintln!("Некорректный IP DNS-сервера: {error}"),
        }
        return 2;
    }
    if options.apply_dns
        && let Err(error) = validate_resolv_conf_path(&options.resolv_conf_path)
    {
        match lang {
            Language::En => eprintln!("Invalid resolv.conf path: {error}"),
            Language::Ru => eprintln!("Некорректный путь resolv.conf: {error}"),
        }
        return 2;
    }
    let (mut network_state, tun_applied) =
        if options.apply_tun && runtime_state.capture_plan.mode == CaptureMode::Tun {
            if let Err(error) = apply_tun_interface(
                &options.tun_name,
                &options.tun_local_cidr,
                &options.tun_peer_cidr,
            ) {
                match lang {
                    Language::En => eprintln!("Cannot apply TUN interface: {error}"),
                    Language::Ru => eprintln!("Не удалось применить TUN-интерфейс: {error}"),
                }
                return 1;
            }
            ("modified", true)
        } else {
            ("not_modified", false)
        };
    let tun_ready_for_route = if tun_applied {
        true
    } else {
        tun_device_exists(&options.tun_name)
    };
    let mut route_applied = false;
    let route_cidrs = if options.apply_route {
        match parse_route_cidr_list(&options.route_cidr) {
            Ok(v) => v,
            Err(error) => {
                match lang {
                    Language::En => eprintln!("Invalid route CIDR list: {error}"),
                    Language::Ru => eprintln!("Некорректный список route CIDR: {error}"),
                }
                return 2;
            }
        }
    } else {
        Vec::new()
    };
    let mut applied_route_cidrs: Vec<String> = Vec::new();
    if options.apply_route {
        if !tun_ready_for_route {
            match lang {
                Language::En => eprintln!(
                    "Cannot apply route: TUN interface is not ready (neither applied now nor pre-existing)"
                ),
                Language::Ru => eprintln!(
                    "Нельзя применить маршрут: TUN-интерфейс не готов (не применен сейчас и не найден в системе)"
                ),
            }
            return 2;
        }
        for route_cidr in &route_cidrs {
            let route_apply_result = if options.route_policy {
                apply_policy_route_via_tun(
                    route_cidr,
                    &options.tun_name,
                    &options.route_table,
                    &options.route_rule_priority,
                )
            } else {
                apply_route_via_tun(route_cidr, &options.tun_name)
            };
            if let Err(error) = route_apply_result {
                let _ = rollback_applied_routes(
                    &applied_route_cidrs,
                    &options.tun_name,
                    options.route_policy,
                    &options.route_table,
                    &options.route_rule_priority,
                    options.route_fwmark,
                );
                if tun_applied {
                    let _ = remove_tun_interface(&options.tun_name);
                }
                match lang {
                    Language::En => eprintln!("Cannot apply route: {error}"),
                    Language::Ru => eprintln!("Не удалось применить маршрут: {error}"),
                }
                return 1;
            }
            applied_route_cidrs.push(route_cidr.clone());
        }
        route_applied = true;
        if options.route_policy {
            if let Some(fwmark) = options.route_fwmark {
                if let Err(error) = apply_policy_route_fwmark(fwmark) {
                    let _ = rollback_applied_routes(
                        &applied_route_cidrs,
                        &options.tun_name,
                        options.route_policy,
                        &options.route_table,
                        &options.route_rule_priority,
                        options.route_fwmark,
                    );
                    if tun_applied {
                        let _ = remove_tun_interface(&options.tun_name);
                    }
                    match lang {
                        Language::En => eprintln!("Cannot apply route fwmark rule: {error}"),
                        Language::Ru => {
                            eprintln!("Не удалось применить fwmark-правило маршрута: {error}")
                        }
                    }
                    return 1;
                }
            }
        }
        network_state = "modified";
    }
    let mut dns_applied = false;
    let mut dns_backup_path = String::new();
    if options.apply_dns {
        dns_backup_path = format!("{}.chimera.bak", options.resolv_conf_path);
        if let Err(error) = apply_dns_resolver(
            &options.resolv_conf_path,
            &dns_backup_path,
            &options.dns_server,
        ) {
            if route_applied {
                let _ = rollback_applied_routes(
                    &applied_route_cidrs,
                    &options.tun_name,
                    options.route_policy,
                    &options.route_table,
                    &options.route_rule_priority,
                    options.route_fwmark,
                );
            }
            if tun_applied {
                let _ = remove_tun_interface(&options.tun_name);
            }
            match lang {
                Language::En => eprintln!("Cannot apply DNS: {error}"),
                Language::Ru => eprintln!("Не удалось применить DNS: {error}"),
            }
            return 1;
        }
        dns_applied = true;
        network_state = "modified";
    }
    let route_cidrs_applied_csv = if applied_route_cidrs.is_empty() {
        options.route_cidr.clone()
    } else {
        applied_route_cidrs.join(",")
    };
    let route_fwmark_json = options
        .route_fwmark
        .map(|fwmark| format!("0x{fwmark:x}"))
        .unwrap_or_default();
    let state_json = format!(
        "{{\"status\":\"up\",\"network_state\":\"{}\",\"rollback_ready\":true,\"secrets\":\"<redacted>\",\"capture_mode\":\"{}\",\"capture_reason\":\"{}\",\"carrier_profile\":\"{}\",\"carrier_addr\":\"{}\",\"carrier_server_name\":\"{}\",\"tun_applied\":{},\"tun_device\":\"{}\",\"tun_local_cidr\":\"{}\",\"tun_peer_cidr\":\"{}\",\"route_applied\":{},\"route_cidr\":\"{}\",\"route_cidrs_applied\":\"{}\",\"route_policy\":{},\"route_table\":\"{}\",\"route_rule_priority\":\"{}\",\"route_fwmark\":\"{}\",\"route_fwmark_priority\":\"100\",\"dns_applied\":{},\"dns_server\":\"{}\",\"resolv_conf_path\":\"{}\",\"dns_backup_path\":\"{}\"}}\n",
        network_state,
        capture_mode_label_en(runtime_state.capture_plan.mode),
        escape_json(&runtime_state.capture_plan.reason),
        carrier_profile_label_en(runtime_state.status.carrier_profile),
        escape_json(&runtime_state.status.carrier_addr),
        escape_json(&runtime_state.status.carrier_server_name),
        tun_applied,
        escape_json(&options.tun_name),
        escape_json(&options.tun_local_cidr),
        escape_json(&options.tun_peer_cidr),
        route_applied,
        escape_json(&options.route_cidr),
        escape_json(&route_cidrs_applied_csv),
        options.route_policy,
        escape_json(&options.route_table),
        escape_json(&options.route_rule_priority),
        escape_json(&route_fwmark_json),
        dns_applied,
        escape_json(&options.dns_server),
        escape_json(&options.resolv_conf_path),
        escape_json(&dns_backup_path)
    );
    if let Err(error) = atomic_write_text_file(Path::new(&options.state_path), &state_json) {
        if dns_applied {
            let backup_path = format!("{}.chimera.bak", options.resolv_conf_path);
            let _ = rollback_dns_resolver(&options.resolv_conf_path, &backup_path);
        }
        if route_applied {
            let _ = rollback_applied_routes(
                &applied_route_cidrs,
                &options.tun_name,
                options.route_policy,
                &options.route_table,
                &options.route_rule_priority,
                options.route_fwmark,
            );
        }
        if tun_applied {
            let _ = remove_tun_interface(&options.tun_name);
        }
        match lang {
            Language::En => eprintln!(
                "Cannot write state file: {error}. Applied runtime changes were rolled back."
            ),
            Language::Ru => eprintln!(
                "Не удалось записать state-файл: {error}. Примененные runtime-изменения были откачены."
            ),
        }
        return 1;
    }
    print!("{}", render_up_ready_message(lang, &options.state_path));
    0
}

fn down_command(lang: Language, args: &[String]) -> i32 {
    let options = match parse_up_down_options(args) {
        Ok(options) => options,
        Err(()) => {
            eprintln!("{}", down_usage(lang));
            return 2;
        }
    };
    let state_path = Path::new(&options.state_path);
    if !state_path.exists() {
        print!("{}", render_down_nothing_to_rollback(lang));
        return 0;
    }
    if let Err(error) = rollback_from_state_file(lang, state_path, &options) {
        match lang {
            Language::En => eprintln!("Down rollback failed: {error}"),
            Language::Ru => eprintln!("Down rollback завершился ошибкой: {error}"),
        }
        return 1;
    }
    print!("{}", render_down_rollback_done(lang, &options.state_path));
    0
}

struct UpRuntimeState {
    status: StatusOptions,
    capture_plan: CapturePlan,
}

fn build_up_runtime_state(options: &UpDownOptions) -> Result<UpRuntimeState, String> {
    let mut status = StatusOptions {
        config_path: options.config_path.clone(),
        mock_packets: 1,
        mock_age_seconds: 120,
        max_age_seconds: 300,
        max_packets_per_key: 10_000,
        capture_preference: CapturePreference::Auto,
        tun_supported: true,
        carrier_profile: CarrierProfile::InMemory,
        carrier_addr: DEFAULT_CARRIER_ADDR.to_string(),
        carrier_server_name: DEFAULT_CARRIER_SERVER_NAME.to_string(),
    };
    status = apply_status_config_overrides(status)?;
    let capture_plan = status_capture_plan(&status);
    if status.capture_preference == CapturePreference::Tun && !detect_tun_support() {
        return Err("capture mode is forced to tun, but /dev/net/tun is unavailable".to_string());
    }
    if !capture_plan_is_ready(&capture_plan) {
        return Err(capture_plan.reason.clone());
    }
    validate_status_carrier(&status)?;
    if !options.skip_connect_check {
        probe_carrier_reachability(&status)?;
    }
    Ok(UpRuntimeState {
        status,
        capture_plan,
    })
}

fn probe_carrier_reachability(status: &StatusOptions) -> Result<(), String> {
    if is_self_loop_carrier_target(&status.carrier_addr)? {
        return Err(format!(
            "carrier target <redacted> target_state={} matches local host address (self-loop blocked)",
            diagnostic_redaction::endpoint_state(&status.carrier_addr)
        ));
    }
    match status.carrier_profile {
        CarrierProfile::InMemory => Ok(()),
        CarrierProfile::Tls => {
            let target = resolve_first_socket_addr(&status.carrier_addr)?;
            TcpStream::connect_timeout(&target, Duration::from_millis(1500))
                .map(|_| ())
                .map_err(|error| {
                    format!(
                        "carrier tcp reachability check failed for <redacted> target_state={}: {error}",
                        diagnostic_redaction::endpoint_state(&status.carrier_addr)
                    )
                })
        }
        CarrierProfile::Quic => {
            let target = resolve_first_socket_addr(&status.carrier_addr)?;
            let socket = UdpSocket::bind("0.0.0.0:0")
                .map_err(|error| format!("udp bind failed: {error}"))?;
            socket.connect(target).map_err(|error| {
                format!(
                    "carrier udp reachability check failed for <redacted> target_state={}: {error}",
                    diagnostic_redaction::endpoint_state(&status.carrier_addr)
                )
            })
        }
    }
}

fn local_host_ips() -> Vec<IpAddr> {
    let mut out = vec![
        IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
        IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
    ];
    if let Ok(sock) = UdpSocket::bind("0.0.0.0:0") {
        let _ = sock.connect("8.8.8.8:53");
        if let Ok(addr) = sock.local_addr() {
            out.push(addr.ip());
        }
    }
    out.sort();
    out.dedup();
    out
}

fn is_self_loop_carrier_target(carrier_addr: &str) -> Result<bool, String> {
    if std::env::var("CHIMERA_ALLOW_SELF_UPSTREAM")
        .ok()
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        return Ok(false);
    }
    let endpoint =
        CarrierEndpoint::parse(carrier_addr).map_err(|e| format!("carrier addr invalid: {e}"))?;
    Ok(endpoint.is_self_loop_candidate(&local_host_ips()))
}

fn resolve_first_socket_addr(addr: &str) -> Result<SocketAddr, String> {
    let endpoint =
        CarrierEndpoint::parse(addr).map_err(|e| format!("carrier addr invalid: {e}"))?;
    format!("{}:{}", endpoint.host, endpoint.port)
        .to_socket_addrs()
        .map_err(|error| {
            format!(
                "carrier address resolve failed for <redacted> target_state={}: {error}",
                diagnostic_redaction::endpoint_state(addr)
            )
        })?
        .next()
        .ok_or_else(|| {
            format!(
                "carrier address has no resolved endpoints: <redacted> target_state={}",
                diagnostic_redaction::endpoint_state(addr)
            )
        })
}

fn apply_tun_interface(
    tun_name: &str,
    tun_local_cidr: &str,
    tun_peer_cidr: &str,
) -> Result<(), String> {
    run_ip_command(&["tuntap", "add", "dev", tun_name, "mode", "tun"])?;
    if let Err(error) = run_ip_command(&["link", "set", "dev", tun_name, "up"]) {
        let _ = run_ip_command(&["link", "delete", "dev", tun_name]);
        return Err(error);
    }
    if let Err(error) = run_ip_command(&[
        "addr",
        "add",
        tun_local_cidr,
        "peer",
        tun_peer_cidr,
        "dev",
        tun_name,
    ]) {
        let _ = run_ip_command(&["link", "delete", "dev", tun_name]);
        return Err(error);
    }
    Ok(())
}

fn remove_tun_interface(tun_name: &str) -> Result<(), String> {
    run_ip_delete_command(&["link", "delete", "dev", tun_name])
}

fn apply_route_via_tun(route_cidr: &str, tun_name: &str) -> Result<(), String> {
    run_ip_command(&["route", "replace", route_cidr, "dev", tun_name])
}

fn remove_route_via_tun(route_cidr: &str, tun_name: &str) -> Result<(), String> {
    run_ip_delete_command(&["route", "del", route_cidr, "dev", tun_name])
}

fn apply_policy_route_via_tun(
    route_cidr: &str,
    tun_name: &str,
    route_table: &str,
    route_rule_priority: &str,
) -> Result<(), String> {
    run_ip_command(&[
        "route",
        "replace",
        route_cidr,
        "dev",
        tun_name,
        "table",
        route_table,
    ])?;
    if let Err(error) = run_ip_command(&[
        "rule",
        "add",
        "to",
        route_cidr,
        "pref",
        route_rule_priority,
        "table",
        route_table,
    ]) {
        let _ = run_ip_command(&[
            "route",
            "del",
            route_cidr,
            "dev",
            tun_name,
            "table",
            route_table,
        ]);
        return Err(error);
    }
    Ok(())
}

fn remove_policy_route_via_tun(
    route_cidr: &str,
    tun_name: &str,
    route_table: &str,
    route_rule_priority: &str,
) -> Result<(), String> {
    let mut errors: Vec<String> = Vec::new();
    if let Err(error) = run_ip_delete_command(&[
        "rule",
        "del",
        "to",
        route_cidr,
        "pref",
        route_rule_priority,
        "table",
        route_table,
    ]) {
        errors.push(format!("policy rule del failed: {error}"));
    }
    if let Err(error) = run_ip_delete_command(&[
        "route",
        "del",
        route_cidr,
        "dev",
        tun_name,
        "table",
        route_table,
    ]) {
        errors.push(format!("policy route del failed: {error}"));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn apply_policy_route_fwmark(fwmark: u32) -> Result<(), String> {
    run_ip_command(&[
        "rule",
        "add",
        "fwmark",
        &format!("0x{fwmark:x}"),
        "pref",
        "100",
        "table",
        "main",
    ])
}

fn remove_policy_route_fwmark(fwmark: u32) -> Result<(), String> {
    run_ip_delete_command(&[
        "rule",
        "del",
        "fwmark",
        &format!("0x{fwmark:x}"),
        "pref",
        "100",
        "table",
        "main",
    ])
}

#[allow(clippy::collapsible_if)]
fn rollback_applied_routes(
    route_cidrs: &[String],
    tun_name: &str,
    route_policy: bool,
    route_table: &str,
    route_rule_priority: &str,
    route_fwmark: Option<u32>,
) -> Result<(), String> {
    let mut errors: Vec<String> = Vec::new();
    for route_cidr in route_cidrs.iter().rev() {
        let rollback_result = if route_policy {
            remove_policy_route_via_tun(route_cidr, tun_name, route_table, route_rule_priority)
        } else {
            remove_route_via_tun(route_cidr, tun_name)
        };
        if let Err(error) = rollback_result {
            errors.push(format!("{route_cidr}: {error}"));
        }
    }
    if route_policy {
        if let Some(fwmark) = route_fwmark {
            if let Err(error) = remove_policy_route_fwmark(fwmark) {
                errors.push(format!("fwmark 0x{fwmark:x}: {error}"));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn tun_device_exists(tun_name: &str) -> bool {
    run_ip_command(&["link", "show", "dev", tun_name]).is_ok()
}

fn validate_policy_route_options(
    route_table: &str,
    route_rule_priority: &str,
) -> Result<(), String> {
    let parse_num = |name: &str, value: &str| -> Result<u32, String> {
        value
            .parse::<u32>()
            .map_err(|_| format!("{name} must be a positive integer, got: {value}"))
    };
    let table = parse_num("route_table", route_table)?;
    let priority = parse_num("route_rule_priority", route_rule_priority)?;
    if table == 0 {
        return Err("route_table must be > 0".to_string());
    }
    if matches!(table, 253..=255) {
        return Err("route_table must not be one of reserved system tables: 253(default), 254(main), 255(local)".to_string());
    }
    if priority == 0 {
        return Err("route_rule_priority must be > 0".to_string());
    }
    if priority < 1000 {
        return Err(
            "route_rule_priority must be >= 1000 to avoid clashing with system rules".to_string(),
        );
    }
    Ok(())
}

fn validate_route_cidr(route_cidr: &str) -> Result<(), String> {
    let _ = parse_cidr_parts(route_cidr)?;
    Ok(())
}

fn parse_route_cidr_list(route_cidrs: &str) -> Result<Vec<String>, String> {
    let mut result: Vec<String> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for raw in route_cidrs.split(',') {
        let item = raw.trim();
        if item.is_empty() {
            return Err("empty CIDR in list".to_string());
        }
        if seen.contains(item) {
            return Err(format!("duplicate CIDR in list: {item}"));
        }
        seen.insert(item.to_string());
        result.push(item.to_string());
    }
    if result.is_empty() {
        return Err("CIDR list is empty".to_string());
    }
    Ok(result)
}

fn validate_route_cidrs(route_cidrs: &str) -> Result<(), String> {
    let list = parse_route_cidr_list(route_cidrs)?;
    for route_cidr in list {
        validate_route_cidr(&route_cidr)?;
    }
    Ok(())
}

fn parse_cidr_parts(route_cidr: &str) -> Result<(IpAddr, u8), String> {
    let (ip_part, prefix_part) = route_cidr
        .split_once('/')
        .ok_or_else(|| format!("missing `/` in CIDR: {route_cidr}"))?;
    let ip: IpAddr = ip_part
        .parse()
        .map_err(|_| format!("invalid IP in CIDR: {ip_part}"))?;
    let prefix: u8 = prefix_part
        .parse::<u8>()
        .map_err(|_| format!("invalid prefix in CIDR: {prefix_part}"))?;
    let max_prefix = match ip {
        IpAddr::V4(_) => 32,
        IpAddr::V6(_) => 128,
    };
    if prefix > max_prefix {
        return Err(format!(
            "prefix out of range for {ip_part}: {prefix} > {max_prefix}"
        ));
    }
    Ok((ip, prefix))
}

fn validate_tun_cidr_pair(tun_local_cidr: &str, tun_peer_cidr: &str) -> Result<(), String> {
    let (local_ip, _local_prefix) = parse_cidr_parts(tun_local_cidr)
        .map_err(|error| format!("tun_local_cidr invalid: {error}"))?;
    let (peer_ip, _peer_prefix) = parse_cidr_parts(tun_peer_cidr)
        .map_err(|error| format!("tun_peer_cidr invalid: {error}"))?;
    match (local_ip, peer_ip) {
        (IpAddr::V4(local), IpAddr::V4(peer)) => {
            if local == peer {
                return Err(
                    "tun_local_cidr and tun_peer_cidr must use different IP addresses".to_string(),
                );
            }
        }
        (IpAddr::V6(local), IpAddr::V6(peer)) => {
            if local == peer {
                return Err(
                    "tun_local_cidr and tun_peer_cidr must use different IP addresses".to_string(),
                );
            }
        }
        _ => {
            return Err("tun_local_cidr and tun_peer_cidr must use the same IP family".to_string());
        }
    }
    Ok(())
}

fn validate_dns_server(dns_server: &str) -> Result<(), String> {
    dns_server
        .parse::<IpAddr>()
        .map(|_| ())
        .map_err(|_| format!("invalid IP address: {dns_server}"))
}

fn validate_resolv_conf_path(resolv_conf_path: &str) -> Result<(), String> {
    if resolv_conf_path.is_empty() {
        return Err("path must not be empty".to_string());
    }
    let path = Path::new(resolv_conf_path);
    if !path.is_absolute() {
        return Err("path must be absolute".to_string());
    }
    let metadata =
        std::fs::metadata(path).map_err(|error| format!("path is not accessible: {error}"))?;
    if !metadata.is_file() {
        return Err("path must point to a regular file".to_string());
    }
    Ok(())
}

fn validate_tun_name(tun_name: &str) -> Result<(), String> {
    if tun_name.is_empty() {
        return Err("name must not be empty".to_string());
    }
    if tun_name.len() > 15 {
        return Err("name must be at most 15 characters".to_string());
    }
    if !tun_name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("name must contain only [A-Za-z0-9_-]".to_string());
    }
    Ok(())
}

fn apply_dns_resolver(
    resolv_conf_path: &str,
    backup_path: &str,
    dns_server: &str,
) -> Result<(), String> {
    let resolv_path = Path::new(resolv_conf_path);
    let backup = Path::new(backup_path);
    if resolv_path == backup {
        return Err("dns backup path must differ from resolv.conf path".to_string());
    }
    let original = std::fs::read_to_string(resolv_conf_path)
        .map_err(|error| format!("cannot read resolv.conf at {resolv_conf_path}: {error}"))?;
    std::fs::write(backup_path, &original)
        .map_err(|error| format!("cannot write DNS backup at {backup_path}: {error}"))?;
    let content = format!("nameserver {dns_server}\n");
    std::fs::write(resolv_conf_path, content)
        .map_err(|error| format!("cannot write resolv.conf at {resolv_conf_path}: {error}"))?;
    Ok(())
}

fn rollback_dns_resolver(resolv_conf_path: &str, backup_path: &str) -> Result<(), String> {
    let backup = std::fs::read_to_string(backup_path)
        .map_err(|error| format!("cannot read DNS backup at {backup_path}: {error}"))?;
    std::fs::write(resolv_conf_path, backup)
        .map_err(|error| format!("cannot restore resolv.conf at {resolv_conf_path}: {error}"))?;
    let _ = std::fs::remove_file(backup_path);
    Ok(())
}

fn run_ip_command(args: &[&str]) -> Result<(), String> {
    let output = Command::new("ip")
        .args(args)
        .output()
        .map_err(|error| format!("failed to execute `ip {}`: {error}", args.join(" ")))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!("`ip {}` failed: {}", args.join(" "), stderr.trim()))
}

fn run_ip_delete_command(args: &[&str]) -> Result<(), String> {
    let output = Command::new("ip")
        .args(args)
        .output()
        .map_err(|error| format!("failed to execute `ip {}`: {error}", args.join(" ")))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if is_ip_absent_delete_error(stderr.trim()) {
        return Ok(());
    }
    Err(format!("`ip {}` failed: {}", args.join(" "), stderr.trim()))
}

fn is_ip_absent_delete_error(stderr: &str) -> bool {
    stderr.contains("No such process")
        || stderr.contains("Cannot find device")
        || stderr.contains("Cannot find")
}

fn parse_state_json(state_json: &str) -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(state_json).ok()
}

fn parse_json_string_at(input: &str, start: usize) -> Option<(String, usize)> {
    if input.as_bytes().get(start) != Some(&b'"') {
        return None;
    }
    let mut index = start + 1;
    let mut escaped = false;
    let mut raw = String::new();
    while index < input.len() {
        let ch = input[index..].chars().next()?;
        index += ch.len_utf8();
        if escaped {
            raw.push('\\');
            raw.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => return unescape_json_string(&raw).map(|decoded| (decoded, index)),
            c => raw.push(c),
        }
    }
    None
}

fn top_level_json_key_count(input: &str, target: &str) -> usize {
    let mut index = 0usize;
    let mut depth = 0usize;
    let mut count = 0usize;
    while index < input.len() {
        let Some(ch) = input[index..].chars().next() else {
            break;
        };
        match ch {
            '"' => {
                if let Some((key, next_index)) = parse_json_string_at(input, index) {
                    let rest = input[next_index..].trim_start();
                    if depth == 1 && rest.starts_with(':') && key == target {
                        count += 1;
                    }
                    index = next_index;
                    continue;
                }
                break;
            }
            '{' | '[' => depth = depth.saturating_add(1),
            '}' | ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        index += ch.len_utf8();
    }
    count
}

fn state_required_key_status(state_text: &str, field: &str) -> Result<(), &'static str> {
    match top_level_json_key_count(state_text, field) {
        0 => Err("missing_field"),
        1 => Ok(()),
        _ => Err("duplicate_field"),
    }
}

fn validate_datapath_state_proof(state_path: &Path) -> Result<(), &'static str> {
    let state_text = match std::fs::read_to_string(state_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err("missing_state");
        }
        Err(_) => return Err("state_unreadable"),
    };
    let Some(value) = parse_state_json(&state_text) else {
        return Err("state_invalid_json");
    };
    if !value.is_object() {
        return Err("state_not_object");
    }
    let require_dns = std::env::var("CHIMERA_STATE_PROOF_REQUIRE_DNS")
        .map(|v| v != "false")
        .unwrap_or(true);
    let required_fields: &[&str] = if require_dns {
        &[
            "status",
            "network_state",
            "rollback_ready",
            "tun_applied",
            "route_applied",
            "dns_applied",
        ]
    } else {
        &[
            "status",
            "network_state",
            "rollback_ready",
            "tun_applied",
            "route_applied",
        ]
    };
    for field in required_fields {
        state_required_key_status(&state_text, field)?;
    }
    if extract_state_string_field_from_value(&value, "status").as_deref() != Some("up") {
        return Err("status_not_up");
    }
    if extract_state_string_field_from_value(&value, "network_state").as_deref() != Some("modified")
    {
        return Err("network_not_modified");
    }
    if extract_state_bool_field_from_value(&value, "rollback_ready") != Some(true) {
        return Err("rollback_not_ready");
    }
    if extract_state_bool_field_from_value(&value, "tun_applied") != Some(true) {
        return Err("tun_not_applied");
    }
    if extract_state_bool_field_from_value(&value, "route_applied") != Some(true) {
        return Err("route_not_applied");
    }
    if require_dns && extract_state_bool_field_from_value(&value, "dns_applied") != Some(true) {
        return Err("dns_not_applied");
    }
    Ok(())
}

fn default_flow_proof_path(state_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.flow.json", state_path.display()))
}

fn validate_datapath_flow_proof(
    flow_path: &Path,
    max_flow_age_sec: u64,
) -> Result<(), &'static str> {
    let flow_text = match std::fs::read_to_string(flow_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err("missing_flow_proof");
        }
        Err(_) => return Err("flow_file_unreadable"),
    };
    let metadata = std::fs::metadata(flow_path).map_err(|_| "flow_file_unreadable")?;
    let age = metadata
        .modified()
        .map_err(|_| "flow_file_unreadable")?
        .elapsed()
        .map_err(|_| "flow_file_unreadable")?;
    if age > Duration::from_secs(max_flow_age_sec) {
        return Err("flow_stale");
    }
    let Some(value) = parse_state_json(&flow_text) else {
        return Err("flow_invalid_json");
    };
    if !value.is_object() {
        return Err("flow_not_object");
    }
    for field in [
        "status",
        "kind",
        "flow_id",
        "path_kind",
        "transparent_flow_observed",
        "counter_delta_ok",
        "secure_peer_egress_observed",
        "secure_peer_bytes_delta_ok",
        "network_state",
    ] {
        match state_required_key_status(&flow_text, field) {
            Ok(()) => {}
            Err("duplicate_field") => return Err("flow_duplicate_field"),
            Err(_) => return Err("flow_missing_field"),
        }
    }
    if extract_state_string_field_from_value(&value, "status").as_deref() != Some("ok") {
        return Err("flow_status_not_ok");
    }
    if extract_state_string_field_from_value(&value, "kind").as_deref()
        != Some("chimera_datapath_flow_proof")
    {
        return Err("flow_kind_invalid");
    }
    if extract_state_string_field_from_value(&value, "flow_id")
        .is_none_or(|value| value.trim().is_empty())
    {
        return Err("flow_id_missing");
    }
    if extract_state_string_field_from_value(&value, "path_kind")
        .is_none_or(|value| value.trim().is_empty())
    {
        return Err("flow_path_kind_missing");
    }
    if extract_state_bool_field_from_value(&value, "transparent_flow_observed") != Some(true) {
        return Err("flow_not_observed");
    }
    if extract_state_bool_field_from_value(&value, "counter_delta_ok") != Some(true) {
        return Err("flow_counter_delta_not_ok");
    }
    if extract_state_bool_field_from_value(&value, "secure_peer_egress_observed") != Some(true) {
        return Err("flow_secure_peer_not_observed");
    }
    if extract_state_bool_field_from_value(&value, "secure_peer_bytes_delta_ok") != Some(true) {
        return Err("flow_secure_peer_bytes_not_ok");
    }
    if extract_state_string_field_from_value(&value, "network_state").as_deref() != Some("modified")
    {
        return Err("flow_network_not_modified");
    }
    Ok(())
}

fn extract_state_string_field_from_value(value: &serde_json::Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
}

fn extract_state_bool_field_from_value(value: &serde_json::Value, field: &str) -> Option<bool> {
    value.get(field).and_then(serde_json::Value::as_bool)
}

fn extract_state_string_field(state_json: &str, field: &str) -> Option<String> {
    if let Some(value) = parse_state_json(state_json)
        && let Some(found) = extract_state_string_field_from_value(&value, field)
    {
        return Some(found);
    }
    let needle = format!("\"{field}\":\"");
    let start = state_json.find(&needle)?;
    let value_start = start + needle.len();
    let rest = &state_json[value_start..];
    let mut escaped = false;
    let mut raw = String::new();
    for ch in rest.chars() {
        if escaped {
            raw.push('\\');
            raw.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => return unescape_json_string(&raw),
            c => raw.push(c),
        }
    }
    None
}

fn unescape_json_string(input: &str) -> Option<String> {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        let esc = chars.next()?;
        match esc {
            '\\' => out.push('\\'),
            '"' => out.push('"'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'u' => {
                let mut code = String::with_capacity(4);
                code.push(chars.next()?);
                code.push(chars.next()?);
                code.push(chars.next()?);
                code.push(chars.next()?);
                let value = u32::from_str_radix(&code, 16).ok()?;
                let decoded = char::from_u32(value)?;
                out.push(decoded);
            }
            _ => return None,
        }
    }
    Some(out)
}

fn extract_state_bool_field(state_json: &str, field: &str) -> Option<bool> {
    if let Some(value) = parse_state_json(state_json)
        && let Some(found) = extract_state_bool_field_from_value(&value, field)
    {
        return Some(found);
    }
    let key = format!("\"{field}\"");
    let start = state_json.find(&key)?;
    let after_key = &state_json[start + key.len()..];
    let colon_index = after_key.find(':')?;
    let rest = after_key[colon_index + 1..].trim_start();
    if let Some(tail) = rest.strip_prefix("true")
        && !tail
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Some(true);
    }
    if let Some(tail) = rest.strip_prefix("false")
        && !tail
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Some(false);
    }
    None
}

fn rollback_command(lang: Language, args: &[String]) -> i32 {
    let Some(action) = args.first().map(String::as_str) else {
        eprintln!("{}", rollback_usage(lang));
        return 2;
    };
    let options = match parse_rollback_options(&args[1..]) {
        Ok(options) => options,
        Err(()) => {
            eprintln!("{}", rollback_usage(lang));
            return 2;
        }
    };
    let state_path = Path::new(&options.up_down.state_path);
    match action {
        "status" => {
            let exists = state_path.exists();
            if options.json_output {
                let details = rollback_state_details(state_path);
                let json =
                    render_rollback_json("status", exists, &options.up_down.state_path, &details);
                if let Some(path) = options.out_path.as_deref()
                    && let Err(error) = std::fs::write(path, &json)
                {
                    eprintln!("rollback report write failed: {error}");
                    return 1;
                }
                println!("{json}");
                return 0;
            }
            if exists {
                print!(
                    "{}",
                    render_rollback_status_found(lang, &options.up_down.state_path)
                );
            } else {
                print!(
                    "{}",
                    render_rollback_status_missing(lang, &options.up_down.state_path)
                );
            }
            0
        }
        "clean" => {
            let existed = state_path.exists();
            let details_before = rollback_state_details(state_path);
            if existed
                && let Err(error) = rollback_from_state_file(lang, state_path, &options.up_down)
            {
                match lang {
                    Language::En => eprintln!("Rollback clean failed: {error}"),
                    Language::Ru => eprintln!("Rollback clean завершился ошибкой: {error}"),
                }
                return 1;
            }
            if options.json_output {
                let json = render_rollback_json(
                    "clean",
                    existed,
                    &options.up_down.state_path,
                    &details_before,
                );
                if let Some(path) = options.out_path.as_deref()
                    && let Err(error) = std::fs::write(path, &json)
                {
                    eprintln!("rollback report write failed: {error}");
                    return 1;
                }
                println!("{json}");
                return 0;
            }
            print!(
                "{}",
                render_rollback_clean_done(lang, &options.up_down.state_path)
            );
            0
        }
        "recover" => {
            let existed = state_path.exists();
            let details_before = rollback_state_details(state_path);
            if !existed {
                if options.json_output {
                    let json = render_rollback_json(
                        "recover",
                        false,
                        &options.up_down.state_path,
                        &details_before,
                    );
                    if let Some(path) = options.out_path.as_deref()
                        && let Err(error) = std::fs::write(path, &json)
                    {
                        eprintln!("rollback report write failed: {error}");
                        return 1;
                    }
                    println!("{json}");
                    return 0;
                }
                print!(
                    "{}",
                    render_rollback_recover_nothing(lang, &options.up_down.state_path)
                );
                return 0;
            }
            if let Err(error) = rollback_from_state_file(lang, state_path, &options.up_down) {
                match lang {
                    Language::En => eprintln!("Rollback recover failed: {error}"),
                    Language::Ru => eprintln!("Rollback recover завершился ошибкой: {error}"),
                }
                return 1;
            }
            if options.json_output {
                let json = render_rollback_json(
                    "recover",
                    true,
                    &options.up_down.state_path,
                    &details_before,
                );
                if let Some(path) = options.out_path.as_deref()
                    && let Err(error) = std::fs::write(path, &json)
                {
                    eprintln!("rollback report write failed: {error}");
                    return 1;
                }
                println!("{json}");
                return 0;
            }
            print!(
                "{}",
                render_rollback_recover_done(lang, &options.up_down.state_path)
            );
            0
        }
        _ => {
            eprintln!("{}", rollback_usage(lang));
            2
        }
    }
}

fn state_command(lang: Language, subcommand: Option<&str>, args: &[String]) -> i32 {
    match subcommand {
        Some("proof") => state_proof_command(lang, args),
        _ => {
            eprintln!("{}", state_usage(lang));
            2
        }
    }
}

fn state_proof_command(lang: Language, args: &[String]) -> i32 {
    let options = match parse_state_proof_options(args) {
        Ok(options) => options,
        Err(()) => {
            eprintln!("{}", state_usage(lang));
            return 2;
        }
    };
    let state_path = Path::new(&options.state_path);
    match validate_datapath_state_proof(state_path) {
        Ok(()) => {}
        Err(reason) => {
            println!("datapath_proof={reason}");
            return 1;
        }
    };
    if options.require_flow {
        let flow_path_buf = options
            .flow_path
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| default_flow_proof_path(state_path));
        match validate_datapath_flow_proof(&flow_path_buf, options.max_flow_age_sec) {
            Ok(()) => {
                println!("datapath_proof=ok");
                0
            }
            Err(reason) => {
                println!("datapath_proof={reason}");
                1
            }
        }
    } else {
        println!("datapath_proof=ok");
        0
    }
}

fn rollback_from_state_file(
    _lang: Language,
    state_path: &Path,
    fallback_options: &UpDownOptions,
) -> Result<(), String> {
    let state_text = std::fs::read_to_string(state_path)
        .map_err(|error| format!("cannot read state file: {error}"))?;
    let state_json = parse_state_json(&state_text);
    let mut errors: Vec<String> = Vec::new();
    if state_json
        .as_ref()
        .and_then(|v| extract_state_bool_field_from_value(v, "dns_applied"))
        .or_else(|| extract_state_bool_field(&state_text, "dns_applied"))
        == Some(true)
    {
        let resolv_conf_path = state_json
            .as_ref()
            .and_then(|v| extract_state_string_field_from_value(v, "resolv_conf_path"))
            .or_else(|| extract_state_string_field(&state_text, "resolv_conf_path"))
            .unwrap_or_else(|| fallback_options.resolv_conf_path.clone());
        let backup_path = state_json
            .as_ref()
            .and_then(|v| extract_state_string_field_from_value(v, "dns_backup_path"))
            .or_else(|| extract_state_string_field(&state_text, "dns_backup_path"))
            .unwrap_or_else(|| format!("{}.chimera.bak", resolv_conf_path));
        if let Err(error) = rollback_dns_resolver(&resolv_conf_path, &backup_path) {
            errors.push(format!("dns rollback failed: {error}"));
        }
    }
    if state_json
        .as_ref()
        .and_then(|v| extract_state_bool_field_from_value(v, "route_applied"))
        .or_else(|| extract_state_bool_field(&state_text, "route_applied"))
        == Some(true)
    {
        let tun_name = state_json
            .as_ref()
            .and_then(|v| extract_state_string_field_from_value(v, "tun_device"))
            .or_else(|| extract_state_string_field(&state_text, "tun_device"))
            .unwrap_or_else(|| fallback_options.tun_name.clone());
        let route_cidrs = if let Some(parsed) = state_json.as_ref() {
            resolve_route_cidrs_for_rollback_from_value(parsed, &fallback_options.route_cidr)
        } else {
            resolve_route_cidrs_for_rollback(&state_text, &fallback_options.route_cidr)
        };
        let route_policy = state_json
            .as_ref()
            .and_then(|v| extract_state_bool_field_from_value(v, "route_policy"))
            .or_else(|| extract_state_bool_field(&state_text, "route_policy"))
            == Some(true);
        let route_table = state_json
            .as_ref()
            .and_then(|v| extract_state_string_field_from_value(v, "route_table"))
            .or_else(|| extract_state_string_field(&state_text, "route_table"))
            .unwrap_or_else(|| fallback_options.route_table.clone());
        let route_rule_priority = state_json
            .as_ref()
            .and_then(|v| extract_state_string_field_from_value(v, "route_rule_priority"))
            .or_else(|| extract_state_string_field(&state_text, "route_rule_priority"))
            .unwrap_or_else(|| fallback_options.route_rule_priority.clone());
        let route_fwmark = state_json
            .as_ref()
            .and_then(|v| extract_state_string_field_from_value(v, "route_fwmark"))
            .or_else(|| extract_state_string_field(&state_text, "route_fwmark"))
            .and_then(|value| parse_optional_fwmark(&value));
        let route_rollback_result = rollback_applied_routes(
            &route_cidrs,
            &tun_name,
            route_policy,
            &route_table,
            &route_rule_priority,
            route_fwmark,
        );
        if let Err(error) = route_rollback_result {
            errors.push(format!("route rollback failed: {error}"));
        }
    }
    if state_json
        .as_ref()
        .and_then(|v| extract_state_bool_field_from_value(v, "tun_applied"))
        .or_else(|| extract_state_bool_field(&state_text, "tun_applied"))
        == Some(true)
    {
        let tun_name = state_json
            .as_ref()
            .and_then(|v| extract_state_string_field_from_value(v, "tun_device"))
            .or_else(|| extract_state_string_field(&state_text, "tun_device"))
            .unwrap_or_else(|| fallback_options.tun_name.clone());
        if let Err(error) = remove_tun_interface(&tun_name) {
            errors.push(format!("tun rollback failed: {error}"));
        }
    }
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    std::fs::remove_file(state_path).map_err(|error| format!("cannot remove state file: {error}"))
}

fn resolve_route_cidrs_for_rollback(state_text: &str, fallback_route_cidr: &str) -> Vec<String> {
    let route_cidr = extract_state_string_field(state_text, "route_cidrs_applied")
        .or_else(|| extract_state_string_field(state_text, "route_cidr"))
        .unwrap_or_else(|| fallback_route_cidr.to_string());
    parse_route_cidr_list(&route_cidr).unwrap_or_else(|_| vec![route_cidr])
}

fn resolve_route_cidrs_for_rollback_from_value(
    value: &serde_json::Value,
    fallback_route_cidr: &str,
) -> Vec<String> {
    let route_cidr = extract_state_string_field_from_value(value, "route_cidrs_applied")
        .or_else(|| extract_state_string_field_from_value(value, "route_cidr"))
        .unwrap_or_else(|| fallback_route_cidr.to_string());
    parse_route_cidr_list(&route_cidr).unwrap_or_else(|_| vec![route_cidr])
}

fn escape_json(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn render_up_ready_message(lang: Language, state_path: &str) -> String {
    match lang {
        Language::En => {
            format!(
                "Command `up`: runtime state created.\nState file: {state_path}\nRollback: ready.\nSafety: network changes are applied only by explicit `--apply-*` flags.\n"
            )
        }
        Language::Ru => {
            format!(
                "Команда `up`: runtime state создан.\nState-файл: {state_path}\nОткат: готов.\nБезопасность: сетевые изменения применяются только явными флагами `--apply-*`.\n"
            )
        }
    }
}

fn render_down_nothing_to_rollback(lang: Language) -> String {
    match lang {
        Language::En => "Command `down`: nothing to rollback right now.\n".to_string(),
        Language::Ru => "Команда `down`: сейчас нечего откатывать.\n".to_string(),
    }
}

fn render_down_rollback_done(lang: Language, state_path: &str) -> String {
    match lang {
        Language::En => format!(
            "Command `down`: rollback state cleaned.\nState file removed: {state_path}\nSafety: rollback attempted for any state-recorded `--apply-*` changes.\n"
        ),
        Language::Ru => format!(
            "Команда `down`: state отката очищен.\nState-файл удален: {state_path}\nБезопасность: для всех изменений из state (`--apply-*`) выполнена попытка отката.\n"
        ),
    }
}

fn render_rollback_status_found(lang: Language, state_path: &str) -> String {
    match lang {
        Language::En => format!(
            "Rollback status: state file exists.\nState file: {state_path}\nRollback can be cleaned with `chimera rollback clean`.\n"
        ),
        Language::Ru => format!(
            "Rollback статус: state-файл найден.\nState-файл: {state_path}\nОчистка: `chimera rollback clean`.\n"
        ),
    }
}

fn render_rollback_status_missing(lang: Language, state_path: &str) -> String {
    match lang {
        Language::En => format!(
            "Rollback status: no state file.\nState file path: {state_path}\nNothing to clean.\n"
        ),
        Language::Ru => format!(
            "Rollback статус: state-файл отсутствует.\nПуть state-файла: {state_path}\nОчищать нечего.\n"
        ),
    }
}

fn render_rollback_clean_done(lang: Language, state_path: &str) -> String {
    match lang {
        Language::En => format!(
            "Rollback clean: done.\nState file path: {state_path}\nSafety: system network settings were not changed.\n"
        ),
        Language::Ru => format!(
            "Rollback clean: выполнено.\nПуть state-файла: {state_path}\nБезопасность: системные сетевые настройки не менялись.\n"
        ),
    }
}

fn render_rollback_recover_nothing(lang: Language, state_path: &str) -> String {
    match lang {
        Language::En => format!(
            "Rollback recover: no saved state.\nState file path: {state_path}\nNothing to recover.\nSafety: system network settings were not changed.\n"
        ),
        Language::Ru => format!(
            "Rollback recover: сохраненного state нет.\nПуть state-файла: {state_path}\nВосстанавливать нечего.\nБезопасность: системные сетевые настройки не менялись.\n"
        ),
    }
}

fn render_rollback_recover_done(lang: Language, state_path: &str) -> String {
    match lang {
        Language::En => format!(
            "Rollback recover: done.\nState file removed: {state_path}\nUse this after crash or forced stop.\nSafety: system network settings were not changed.\n"
        ),
        Language::Ru => format!(
            "Rollback recover: выполнено.\nState-файл удален: {state_path}\nИспользуйте после сбоя или принудительной остановки.\nБезопасность: системные сетевые настройки не менялись.\n"
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RollbackStateDetails {
    network_state: &'static str,
    tun_applied: bool,
    route_applied: bool,
    dns_applied: bool,
}

fn rollback_state_details(state_path: &Path) -> RollbackStateDetails {
    let Ok(state_text) = std::fs::read_to_string(state_path) else {
        return RollbackStateDetails {
            network_state: "not_modified",
            tun_applied: false,
            route_applied: false,
            dns_applied: false,
        };
    };
    let state_json = parse_state_json(&state_text);
    let tun_applied = state_json
        .as_ref()
        .and_then(|v| extract_state_bool_field_from_value(v, "tun_applied"))
        .or_else(|| extract_state_bool_field(&state_text, "tun_applied"))
        == Some(true);
    let route_applied = state_json
        .as_ref()
        .and_then(|v| extract_state_bool_field_from_value(v, "route_applied"))
        .or_else(|| extract_state_bool_field(&state_text, "route_applied"))
        == Some(true);
    let dns_applied = state_json
        .as_ref()
        .and_then(|v| extract_state_bool_field_from_value(v, "dns_applied"))
        .or_else(|| extract_state_bool_field(&state_text, "dns_applied"))
        == Some(true);
    let network_state = if tun_applied || route_applied || dns_applied {
        "modified"
    } else {
        "not_modified"
    };
    RollbackStateDetails {
        network_state,
        tun_applied,
        route_applied,
        dns_applied,
    }
}

fn render_rollback_json(
    action: &str,
    state_existed: bool,
    _state_path: &str,
    details: &RollbackStateDetails,
) -> String {
    format!(
        "{{\"status\":\"ok\",\"kind\":\"rollback\",\"message_en\":\"Rollback action completed.\",\"message_ru\":\"Действие rollback завершено.\",\"action\":\"{}\",\"state_existed\":{},\"state_file\":\"<redacted>\",\"state_file_state\":\"{}\",\"network_state\":\"{}\",\"tun_applied\":{},\"route_applied\":{},\"dns_applied\":{}}}",
        action,
        state_existed,
        if state_existed { "present" } else { "missing" },
        details.network_state,
        details.tun_applied,
        details.route_applied,
        details.dns_applied
    )
}

#[cfg(test)]
mod tests {
    use super::{
        CapturePreference, CarrierProfile, Language, LanguageSource, RekeyReason,
        RouteExplainRenderContext, StatusOptions, args_tail, detect_language_from_lang_value,
        down_command, lab_command, mesh_command, parse_diag_export_options, parse_doctor_options,
        parse_language_flag, parse_mesh_route_explain_options, parse_rollback_options,
        parse_route_explain_options, parse_status_options, parse_up_down_options, probe_command,
        render_diag_export_json, render_diag_rekey_block, render_doctor_block, render_doctor_json,
        render_health_block, render_help_text, render_policy_validate_block,
        render_route_explain_block, render_route_explain_json, render_status_rekey_block,
        resolve_first_socket_addr, rollback_command, route_command, up_command,
    };
    use chimera_policy::{
        FlowContext, OutboundMode, Policy, PolicySummary, Protocol, RouteDecision,
        RouteExplainTrace, RouteRule, RuleMatcher, parse_policy_text,
    };
    use std::net::IpAddr;
    use std::path::PathBuf;

    struct TestRouteRenderInput<'a> {
        lang: Language,
        domain: Option<&'a str>,
        domain_source_from_dns: bool,
        dns_note: Option<&'a str>,
        runtime_outbound: OutboundMode,
        show_all_matches: bool,
    }

    impl Default for TestRouteRenderInput<'_> {
        fn default() -> Self {
            Self {
                lang: Language::En,
                domain: None,
                domain_source_from_dns: false,
                dns_note: None,
                runtime_outbound: OutboundMode::Direct,
                show_all_matches: false,
            }
        }
    }

    fn render_context<'a>(
        input: TestRouteRenderInput<'a>,
        flow: &'a FlowContext,
        trace: &'a RouteExplainTrace,
    ) -> RouteExplainRenderContext<'a> {
        RouteExplainRenderContext {
            lang: input.lang,
            domain: input.domain,
            domain_source_from_dns: input.domain_source_from_dns,
            dns_note: input.dns_note,
            flow,
            trace,
            runtime_outbound: input.runtime_outbound,
            runtime_reason: "runtime default",
            show_all_matches: input.show_all_matches,
        }
    }

    #[test]
    fn parse_status_options_defaults_when_empty() {
        let parsed = match parse_status_options(&[]) {
            Ok(parsed) => parsed,
            Err(()) => unreachable!("defaults should parse"),
        };
        assert_eq!(
            parsed,
            StatusOptions {
                config_path: None,
                mock_packets: 1,
                mock_age_seconds: 120,
                max_age_seconds: 300,
                max_packets_per_key: 10_000,
                capture_preference: CapturePreference::Auto,
                tun_supported: true,
                carrier_profile: CarrierProfile::InMemory,
                carrier_addr: crate::DEFAULT_CARRIER_ADDR.to_string(),
                carrier_server_name: crate::DEFAULT_CARRIER_SERVER_NAME.to_string()
            }
        );
    }

    #[test]
    fn parse_status_options_with_mock_only() {
        let args = vec![
            "--mock-traffic".to_string(),
            "50".to_string(),
            "--age".to_string(),
            "20".to_string(),
        ];
        let parsed = match parse_status_options(&args) {
            Ok(parsed) => parsed,
            Err(()) => unreachable!("mock-only args should parse"),
        };
        assert_eq!(parsed.mock_packets, 50);
        assert_eq!(parsed.mock_age_seconds, 20);
        assert_eq!(parsed.max_age_seconds, 300);
        assert_eq!(parsed.max_packets_per_key, 10_000);
        assert_eq!(parsed.config_path, None);
        assert_eq!(parsed.capture_preference, CapturePreference::Auto);
    }

    #[test]
    fn parse_status_options_with_custom_policy() {
        let args = vec![
            "--mock-traffic".to_string(),
            "5".to_string(),
            "--age".to_string(),
            "15".to_string(),
            "--max-age".to_string(),
            "10".to_string(),
            "--max-packets".to_string(),
            "100".to_string(),
        ];
        let parsed = match parse_status_options(&args) {
            Ok(parsed) => parsed,
            Err(()) => unreachable!("full args should parse"),
        };
        assert_eq!(parsed.max_age_seconds, 10);
        assert_eq!(parsed.max_packets_per_key, 100);
    }

    #[test]
    fn parse_status_options_with_runtime_profile_flags() {
        let args = vec![
            "--mock-traffic".to_string(),
            "2".to_string(),
            "--age".to_string(),
            "3".to_string(),
            "--capture".to_string(),
            "tun".to_string(),
            "--tun-supported".to_string(),
            "false".to_string(),
            "--carrier".to_string(),
            "quic".to_string(),
            "--carrier-addr".to_string(),
            "198.51.100.10:9443".to_string(),
            "--server-name".to_string(),
            "gw.example.org".to_string(),
        ];
        let parsed = match parse_status_options(&args) {
            Ok(parsed) => parsed,
            Err(()) => unreachable!("status runtime profile args should parse"),
        };
        assert_eq!(parsed.capture_preference, CapturePreference::Tun);
        assert!(!parsed.tun_supported);
        assert_eq!(parsed.carrier_profile, CarrierProfile::Quic);
        assert_eq!(parsed.carrier_addr, "198.51.100.10:9443");
        assert_eq!(parsed.carrier_server_name, "gw.example.org");
    }

    #[test]
    fn status_runtime_profile_redacts_carrier_endpoint_and_server_name() {
        let options = StatusOptions {
            config_path: None,
            mock_packets: 2,
            mock_age_seconds: 10,
            max_age_seconds: 300,
            max_packets_per_key: 10_000,
            capture_preference: CapturePreference::Tun,
            tun_supported: true,
            carrier_profile: CarrierProfile::Tls,
            carrier_addr: "198.51.100.10:9443".to_string(),
            carrier_server_name: "gw.example.org".to_string(),
        };
        let plan = crate::status_capture_plan(&options);
        let rendered = crate::render_status_runtime_profile(Language::En, &options, &plan);
        assert!(rendered.contains("Carrier target: <redacted>, target_state=placeholder"));
        assert!(
            rendered.contains("Carrier server name: <redacted>, server_name_state=placeholder")
        );
        assert!(!rendered.contains("198.51.100.10:9443"));
        assert!(!rendered.contains("gw.example.org"));
    }

    #[test]
    fn status_runtime_profile_redacts_tcp_scheme_carrier_endpoint() {
        let options = StatusOptions {
            config_path: None,
            mock_packets: 2,
            mock_age_seconds: 10,
            max_age_seconds: 300,
            max_packets_per_key: 10_000,
            capture_preference: CapturePreference::Tun,
            tun_supported: true,
            carrier_profile: CarrierProfile::Tls,
            carrier_addr: "tcp://198.51.100.10:9443".to_string(),
            carrier_server_name: "gw.example.org".to_string(),
        };
        let plan = crate::status_capture_plan(&options);
        let rendered = crate::render_status_runtime_profile(Language::En, &options, &plan);
        assert!(rendered.contains("Carrier target: <redacted>, target_state=placeholder"));
        assert!(!rendered.contains("tcp://198.51.100.10:9443"));
    }

    #[test]
    fn resolve_first_socket_addr_accepts_tcp_scheme() {
        let addr = resolve_first_socket_addr("tcp://127.0.0.1:9443")
            .unwrap_or_else(|error| unreachable!("tcp scheme should resolve: {error}"));
        assert_eq!(addr.port(), 9443);
        assert_eq!(addr.ip().to_string(), "127.0.0.1");
    }

    #[test]
    fn parse_status_options_rejects_local_proxy_capture() {
        let args = vec![
            "--mock-traffic".to_string(),
            "2".to_string(),
            "--age".to_string(),
            "3".to_string(),
            "--capture".to_string(),
            "local-proxy".to_string(),
        ];
        assert!(parse_status_options(&args).is_err());
    }

    #[test]
    fn parse_status_options_with_config_path_only() {
        let args = vec!["--config".to_string(), "node.toml".to_string()];
        let parsed = match parse_status_options(&args) {
            Ok(parsed) => parsed,
            Err(()) => unreachable!("config-only args should parse"),
        };
        assert_eq!(parsed.config_path, Some("node.toml".to_string()));
        assert_eq!(parsed.mock_packets, 1);
    }

    #[test]
    fn parse_status_options_rejects_invalid_shapes() {
        let bad_flag = vec!["--age".to_string(), "10".to_string()];
        assert!(parse_status_options(&bad_flag).is_err());

        let missing_value = vec![
            "--mock-traffic".to_string(),
            "5".to_string(),
            "--age".to_string(),
        ];
        assert!(parse_status_options(&missing_value).is_err());

        let bad_policy_order = vec![
            "--mock-traffic".to_string(),
            "5".to_string(),
            "--age".to_string(),
            "15".to_string(),
            "--max-packets".to_string(),
            "100".to_string(),
            "--max-age".to_string(),
            "10".to_string(),
        ];
        assert!(parse_status_options(&bad_policy_order).is_err());
    }

    #[test]
    fn status_render_snapshot_packet_limit() {
        let options = StatusOptions {
            config_path: None,
            mock_packets: 10_000,
            mock_age_seconds: 10,
            max_age_seconds: 300,
            max_packets_per_key: 10_000,
            capture_preference: CapturePreference::Auto,
            tun_supported: true,
            carrier_profile: CarrierProfile::InMemory,
            carrier_addr: crate::DEFAULT_CARRIER_ADDR.to_string(),
            carrier_server_name: crate::DEFAULT_CARRIER_SERVER_NAME.to_string(),
        };
        let rendered = render_status_rekey_block(
            Language::En,
            &options,
            Some(RekeyReason::PacketLimitExceeded),
        );
        assert!(rendered.contains("Session age (sec): 10"));
        assert!(rendered.contains("Packets sent: 10000"));
        assert!(rendered.contains("Need new key: true"));
        assert!(rendered.contains("Why: too many packets"));
    }

    #[test]
    fn diag_render_snapshot_age_exceeded() {
        let rendered =
            render_diag_rekey_block(Language::En, 15, 5, Some(RekeyReason::SessionAgeExceeded));
        assert!(rendered.contains("Policy max age (sec): 300"));
        assert!(rendered.contains("Session age (sec): 15"));
        assert!(rendered.contains("Packets with current key: 5"));
        assert!(rendered.contains("Need new key: true"));
        assert!(rendered.contains("Why: session is too old"));
    }

    #[test]
    fn status_render_snapshot_ru_packet_limit() {
        let options = StatusOptions {
            config_path: None,
            mock_packets: 10_000,
            mock_age_seconds: 10,
            max_age_seconds: 300,
            max_packets_per_key: 10_000,
            capture_preference: CapturePreference::Auto,
            tun_supported: true,
            carrier_profile: CarrierProfile::InMemory,
            carrier_addr: crate::DEFAULT_CARRIER_ADDR.to_string(),
            carrier_server_name: crate::DEFAULT_CARRIER_SERVER_NAME.to_string(),
        };
        let rendered = render_status_rekey_block(
            Language::Ru,
            &options,
            Some(RekeyReason::PacketLimitExceeded),
        );
        assert!(rendered.contains("Возраст сессии (сек): 10"));
        assert!(rendered.contains("Отправлено пакетов: 10000"));
        assert!(rendered.contains("Нужен новый ключ: true"));
        assert!(rendered.contains("Почему: слишком много пакетов"));
    }

    #[test]
    fn diag_render_snapshot_ru_age_exceeded() {
        let rendered =
            render_diag_rekey_block(Language::Ru, 15, 5, Some(RekeyReason::SessionAgeExceeded));
        assert!(rendered.contains("Предел возраста по политике (сек): 300"));
        assert!(rendered.contains("Возраст сессии (сек): 15"));
        assert!(rendered.contains("Пакетов с текущим ключом: 5"));
        assert!(rendered.contains("Нужен новый ключ: true"));
        assert!(rendered.contains("Почему: сессия слишком старая"));
    }

    #[test]
    fn route_render_snapshot_gateway_match() {
        let policy = Policy::new(vec![
            RouteRule {
                id: "example-transit".to_string(),
                matcher: RuleMatcher::DomainSuffix("example.org".to_string()),
                outbound: OutboundMode::Transit,
            },
            RouteRule {
                id: "default-direct".to_string(),
                matcher: RuleMatcher::Default,
                outbound: OutboundMode::Direct,
            },
        ]);
        let flow = FlowContext {
            domain: Some("api.example.org".to_string()),
            destination_ip: None,
            protocol: Protocol::Tcp,
            port: Some(443),
        };
        let trace = policy.explain(&flow);
        let rendered = render_route_explain_block(&render_context(
            TestRouteRenderInput {
                domain: Some("api.example.org"),
                ..TestRouteRenderInput::default()
            },
            &flow,
            &trace,
        ));
        assert!(rendered.contains("Site: api.example.org"));
        assert!(rendered.contains("Rule used: example-transit"));
        assert!(rendered.contains("How we send: through peer transit node"));
        assert!(rendered.contains("Rules checked: 2"));
        assert!(rendered.contains("Rules matched: 2"));
    }

    #[test]
    fn route_render_snapshot_default_direct() {
        let policy = Policy::new(vec![
            RouteRule {
                id: "example-transit".to_string(),
                matcher: RuleMatcher::DomainSuffix("example.org".to_string()),
                outbound: OutboundMode::Transit,
            },
            RouteRule {
                id: "default-direct".to_string(),
                matcher: RuleMatcher::Default,
                outbound: OutboundMode::Direct,
            },
        ]);
        let flow = FlowContext {
            domain: Some("rust-lang.org".to_string()),
            destination_ip: None,
            protocol: Protocol::Tcp,
            port: Some(443),
        };
        let trace = policy.explain(&flow);
        let rendered = render_route_explain_block(&render_context(
            TestRouteRenderInput {
                domain: Some("rust-lang.org"),
                ..TestRouteRenderInput::default()
            },
            &flow,
            &trace,
        ));
        assert!(rendered.contains("Site: rust-lang.org"));
        assert!(rendered.contains("Rule used: default-direct"));
        assert!(rendered.contains("How we send: direct connection"));
    }

    #[test]
    fn route_render_snapshot_ru_gateway_match() {
        let policy = Policy::new(vec![
            RouteRule {
                id: "example-transit".to_string(),
                matcher: RuleMatcher::DomainSuffix("example.org".to_string()),
                outbound: OutboundMode::Transit,
            },
            RouteRule {
                id: "default-direct".to_string(),
                matcher: RuleMatcher::Default,
                outbound: OutboundMode::Direct,
            },
        ]);
        let flow = FlowContext {
            domain: Some("api.example.org".to_string()),
            destination_ip: None,
            protocol: Protocol::Tcp,
            port: Some(443),
        };
        let trace = policy.explain(&flow);
        let rendered = render_route_explain_block(&render_context(
            TestRouteRenderInput {
                lang: Language::Ru,
                domain: Some("api.example.org"),
                ..TestRouteRenderInput::default()
            },
            &flow,
            &trace,
        ));
        assert!(rendered.contains("Сайт: api.example.org"));
        assert!(rendered.contains("Сработало правило: example-transit"));
        assert!(rendered.contains("Как отправляем: через транзитный узел"));
    }

    #[test]
    fn route_policy_text_changes_decision() {
        let text = r#"
            block_ads = suffix:ads.example => block
            default_route = default => direct
        "#;
        let policy = match parse_policy_text(text) {
            Ok(policy) => policy,
            Err(error) => unreachable!("policy should parse: {error}"),
        };
        let flow = FlowContext {
            domain: Some("cdn.ads.example".to_string()),
            destination_ip: None,
            protocol: Protocol::Tcp,
            port: Some(443),
        };
        let trace = policy.explain(&flow);
        let rendered = render_route_explain_block(&render_context(
            TestRouteRenderInput {
                domain: Some("cdn.ads.example"),
                ..TestRouteRenderInput::default()
            },
            &flow,
            &trace,
        ));
        assert!(rendered.contains("Rule used: block_ads"));
        assert!(rendered.contains("How we send: blocked by policy"));
    }

    #[test]
    fn help_render_snapshot_contains_key_commands() {
        let rendered = render_help_text(Language::En);
        assert!(rendered.contains("Chimera CLI (simple mode)"));
        assert!(rendered.contains("status [--config <node_config_file>]"));
        assert!(rendered.contains("health [--config <node_config_file>]"));
        assert!(rendered.contains("doctor [--config <node_config_file>]"));
        assert!(rendered.contains("--capture <auto|tun>"));
        assert!(!rendered.contains("--capture <auto|tun|local-proxy>"));
        assert!(rendered.contains("--carrier <in-memory|tls|quic>"));
        assert!(rendered.contains("route explain [domain]"));
        assert!(rendered.contains(
            "lab <smoke|doctor|config-smoke|fuzz-smoke|perf-smoke|net-sim|benchmark-report|benchmark-regression-check|hardening-smoke|mvp-spec-check|mvp-spec-report|m5-artifacts-report|m6-artifacts-report|release-readiness-report|report-pack|artifact-audit|mvp-snapshot|mvp-verify|mvp-check> [extra args for chimera-lab]"
        ));
        assert!(rendered.contains("rollback <status|clean|recover>"));
        assert!(rendered.contains("--dns-bind-domain <domain>"));
        assert!(rendered.contains("--dns-bind-ip <ipv4|ipv6>"));
        assert!(rendered.contains("--show-all-matches"));
        assert!(rendered.contains("diag rekey <age_sec> <packets_sent>"));
        assert!(rendered.contains("diag export [--config <node_config_file>]"));
        assert!(!rendered.contains("client_config_file"));
        assert!(rendered.contains("probe access --url <http|https_url>"));
        assert!(rendered.contains("--url-file <file>"));
        assert!(rendered.contains("--fail-threshold <n>"));
        assert!(rendered.contains("chimera [--lang en|ru] mvp-check"));
        assert!(rendered.contains("chimera [--lang en|ru] mvp-verify [extra args]"));
        assert!(rendered.contains("chimera [--lang en|ru] mvp-snapshot [extra args]"));
        assert!(rendered.contains("chimera [--lang en|ru] mvp-spec-check [extra args]"));
        assert!(rendered.contains("chimera [--lang en|ru] mvp-spec-report [extra args]"));
        assert!(rendered.contains("chimera [--lang en|ru] m5-artifacts-report [extra args]"));
        assert!(rendered.contains("chimera [--lang en|ru] m6-artifacts-report [extra args]"));
        assert!(rendered.contains("chimera [--lang en|ru] lab-smoke [extra args]"));
        assert!(rendered.contains("chimera [--lang en|ru] lab-doctor [extra args]"));
        assert!(rendered.contains("chimera [--lang en|ru] lab-hardening-smoke [extra args]"));
        assert!(rendered.contains("chimera [--lang en|ru] hardening-smoke"));
        assert!(rendered.contains("chimera [--lang en|ru] benchmark-report [extra args]"));
        assert!(rendered.contains("chimera [--lang en|ru] benchmark-regression-check"));
        assert!(rendered.contains("chimera [--lang en|ru] net-sim"));
        assert!(rendered.contains("chimera [--lang en|ru] perf-smoke"));
        assert!(rendered.contains("chimera [--lang en|ru] fuzz-smoke"));
        assert!(rendered.contains("chimera [--lang en|ru] config-smoke"));
        assert!(rendered.contains("chimera [--lang en|ru] release-readiness-report [extra args]"));
        assert!(rendered.contains("chimera [--lang en|ru] artifact-audit [extra args]"));
        assert!(rendered.contains("chimera [--lang en|ru] report-pack [extra args]"));
    }

    #[test]
    fn help_render_ru_snapshot_contains_key_commands() {
        let rendered = render_help_text(Language::Ru);
        assert!(rendered.contains("Chimera CLI (простой режим)"));
        assert!(rendered.contains("status [--config <файл_node_config>]"));
        assert!(rendered.contains("health [--config <файл_node_config>]"));
        assert!(rendered.contains("doctor [--config <файл_node_config>]"));
        assert!(rendered.contains("--capture <auto|tun>"));
        assert!(!rendered.contains("--capture <auto|tun|local-proxy>"));
        assert!(rendered.contains("--carrier <in-memory|tls|quic>"));
        assert!(rendered.contains("route explain [домен]"));
        assert!(rendered.contains(
            "lab <smoke|doctor|config-smoke|fuzz-smoke|perf-smoke|net-sim|benchmark-report|benchmark-regression-check|hardening-smoke|mvp-spec-check|mvp-spec-report|m5-artifacts-report|m6-artifacts-report|release-readiness-report|report-pack|artifact-audit|mvp-snapshot|mvp-verify|mvp-check> [доп. аргументы для chimera-lab]"
        ));
        assert!(rendered.contains("rollback <status|clean|recover>"));
        assert!(rendered.contains("--dns-bind-domain <домен>"));
        assert!(rendered.contains("--dns-bind-ip <ipv4|ipv6>"));
        assert!(rendered.contains("--show-all-matches"));
        assert!(rendered.contains("diag rekey <секунды> <пакеты>"));
        assert!(rendered.contains("diag export [--config <файл_node_config>]"));
        assert!(!rendered.contains("файл_client_config"));
        assert!(rendered.contains("probe access --url <http|https_url>"));
        assert!(rendered.contains("--url-file <файл>"));
        assert!(rendered.contains("--fail-threshold <n>"));
        assert!(rendered.contains("chimera [--lang en|ru] mvp-check"));
        assert!(rendered.contains("chimera [--lang en|ru] mvp-verify [доп. аргументы]"));
        assert!(rendered.contains("chimera [--lang en|ru] mvp-snapshot [доп. аргументы]"));
        assert!(rendered.contains("chimera [--lang en|ru] mvp-spec-check [доп. аргументы]"));
        assert!(rendered.contains("chimera [--lang en|ru] mvp-spec-report [доп. аргументы]"));
        assert!(rendered.contains("chimera [--lang en|ru] m5-artifacts-report [доп. аргументы]"));
        assert!(rendered.contains("chimera [--lang en|ru] m6-artifacts-report [доп. аргументы]"));
        assert!(rendered.contains("chimera [--lang en|ru] lab-smoke [доп. аргументы]"));
        assert!(rendered.contains("chimera [--lang en|ru] lab-doctor [доп. аргументы]"));
        assert!(rendered.contains("chimera [--lang en|ru] lab-hardening-smoke [доп. аргументы]"));
        assert!(rendered.contains("chimera [--lang en|ru] hardening-smoke"));
        assert!(rendered.contains("chimera [--lang en|ru] benchmark-report [доп. аргументы]"));
        assert!(rendered.contains("chimera [--lang en|ru] benchmark-regression-check"));
        assert!(rendered.contains("chimera [--lang en|ru] net-sim"));
        assert!(rendered.contains("chimera [--lang en|ru] perf-smoke"));
        assert!(rendered.contains("chimera [--lang en|ru] fuzz-smoke"));
        assert!(rendered.contains("chimera [--lang en|ru] config-smoke"));
        assert!(
            rendered.contains("chimera [--lang en|ru] release-readiness-report [доп. аргументы]")
        );
        assert!(rendered.contains("chimera [--lang en|ru] artifact-audit [доп. аргументы]"));
        assert!(rendered.contains("chimera [--lang en|ru] report-pack [доп. аргументы]"));
    }

    #[test]
    fn lab_command_rejects_unknown_subcommand() {
        let exit = lab_command(Language::En, Some("unknown"), &[]);
        assert_eq!(exit, 2);
    }

    #[test]
    fn args_tail_returns_empty_for_missing_tail() {
        let args = vec!["chimera".to_string(), "mesh".to_string()];
        assert_eq!(args_tail(&args, 2), &[] as &[String]);
        assert_eq!(args_tail(&args, 3), &[] as &[String]);
    }

    #[test]
    fn nested_commands_without_subcommands_return_usage_errors() {
        assert_eq!(mesh_command(Language::En, None, &[]), 2);
        assert_eq!(probe_command(Language::En, None, &[]), 2);
        assert_eq!(lab_command(Language::En, None, &[]), 2);
        assert_eq!(route_command(Language::En, None, None, &[]), 2);
    }

    #[test]
    fn health_render_snapshot_ru() {
        let options = StatusOptions {
            config_path: None,
            mock_packets: 1,
            mock_age_seconds: 120,
            max_age_seconds: 300,
            max_packets_per_key: 10_000,
            capture_preference: CapturePreference::Tun,
            tun_supported: true,
            carrier_profile: CarrierProfile::Tls,
            carrier_addr: "203.0.113.10:443".to_string(),
            carrier_server_name: crate::DEFAULT_CARRIER_SERVER_NAME.to_string(),
        };
        let plan = crate::status_capture_plan(&options);
        let rendered = render_health_block(Language::Ru, &options, &plan);
        assert!(rendered.contains("Проверка узла: ok"));
        assert!(rendered.contains("capture=tun, carrier=tls-tcp"));
        assert!(rendered.contains("target=<redacted>, target_state=placeholder"));
        assert!(!rendered.contains("203.0.113.10:443"));
        assert!(rendered.contains("Состояние сети: не изменялось"));
    }

    #[test]
    fn doctor_text_render_redacts_carrier_endpoint() {
        let options = StatusOptions {
            config_path: None,
            mock_packets: 1,
            mock_age_seconds: 120,
            max_age_seconds: 300,
            max_packets_per_key: 10_000,
            capture_preference: CapturePreference::Tun,
            tun_supported: true,
            carrier_profile: CarrierProfile::Tls,
            carrier_addr: "198.51.100.10:443".to_string(),
            carrier_server_name: crate::DEFAULT_CARRIER_SERVER_NAME.to_string(),
        };
        let plan = crate::status_capture_plan(&options);
        let rendered = render_doctor_block(Language::En, &options, &plan, None);
        assert!(rendered.contains("Doctor report: ok"));
        assert!(rendered.contains("target=<redacted>, target_state=placeholder"));
        assert!(!rendered.contains("198.51.100.10:443"));
    }

    #[test]
    fn parse_diag_export_options_defaults() {
        let parsed = match parse_diag_export_options(&[]) {
            Ok(parsed) => parsed,
            Err(()) => unreachable!("default diag export options should parse"),
        };
        assert_eq!(parsed.age_seconds, 120);
        assert_eq!(parsed.packets, 1);
        assert_eq!(parsed.config_path, None);
        assert_eq!(parsed.out_path, None);
    }

    #[test]
    fn parse_diag_export_options_full() {
        let args = vec![
            "--config".to_string(),
            "node.conf".to_string(),
            "--age".to_string(),
            "15".to_string(),
            "--packets".to_string(),
            "20".to_string(),
            "--out".to_string(),
            "diag.json".to_string(),
        ];
        let parsed = match parse_diag_export_options(&args) {
            Ok(parsed) => parsed,
            Err(()) => unreachable!("full diag export options should parse"),
        };
        assert_eq!(parsed.config_path, Some("node.conf".to_string()));
        assert_eq!(parsed.age_seconds, 15);
        assert_eq!(parsed.packets, 20);
        assert_eq!(parsed.out_path, Some("diag.json".to_string()));
    }

    #[test]
    fn parse_doctor_options_defaults() {
        let parsed = match parse_doctor_options(&[]) {
            Ok(parsed) => parsed,
            Err(()) => unreachable!("default doctor options should parse"),
        };
        assert_eq!(parsed.status.mock_packets, 1);
        assert_eq!(parsed.status.mock_age_seconds, 120);
        assert!(!parsed.json_output);
        assert_eq!(parsed.out_path, None);
    }

    #[test]
    fn parse_doctor_options_full() {
        let args = vec![
            "--config".to_string(),
            "node.conf".to_string(),
            "--mock-traffic".to_string(),
            "50".to_string(),
            "--age".to_string(),
            "20".to_string(),
            "--json".to_string(),
            "--out".to_string(),
            "doctor.json".to_string(),
        ];
        let parsed = match parse_doctor_options(&args) {
            Ok(parsed) => parsed,
            Err(()) => unreachable!("full doctor options should parse"),
        };
        assert_eq!(parsed.status.config_path, Some("node.conf".to_string()));
        assert_eq!(parsed.status.mock_packets, 50);
        assert_eq!(parsed.status.mock_age_seconds, 20);
        assert!(parsed.json_output);
        assert_eq!(parsed.out_path, Some("doctor.json".to_string()));
    }

    #[test]
    fn parse_up_down_options_defaults_and_override() {
        let parsed = match parse_up_down_options(&[]) {
            Ok(parsed) => parsed,
            Err(()) => unreachable!("default up/down options should parse"),
        };
        assert_eq!(parsed.state_path, "docs/runtime_state_latest.json");
        assert_eq!(parsed.config_path, None);
        assert!(!parsed.skip_connect_check);
        assert_eq!(parsed.tun_name, "chimera0");
        assert_eq!(parsed.tun_local_cidr, "10.201.0.2/30");
        assert_eq!(parsed.tun_peer_cidr, "10.201.0.1/30");
        assert!(!parsed.apply_route);
        assert_eq!(parsed.route_cidr, "0.0.0.0/1");
        assert!(!parsed.route_policy);
        assert_eq!(parsed.route_table, "51820");
        assert_eq!(parsed.route_rule_priority, "11000");
        assert!(!parsed.apply_dns);
        assert_eq!(parsed.dns_server, "1.1.1.1");
        assert_eq!(parsed.resolv_conf_path, "/etc/resolv.conf");

        let args = vec!["--state-file".to_string(), "tmp/state.json".to_string()];
        let parsed = match parse_up_down_options(&args) {
            Ok(parsed) => parsed,
            Err(()) => unreachable!("state file options should parse"),
        };
        assert_eq!(parsed.state_path, "tmp/state.json");
        assert_eq!(parsed.config_path, None);
        assert!(!parsed.skip_connect_check);

        let args = vec![
            "--state-file".to_string(),
            "tmp/state.json".to_string(),
            "--config".to_string(),
            "configs/mesh-node.example.conf".to_string(),
        ];
        let parsed = match parse_up_down_options(&args) {
            Ok(parsed) => parsed,
            Err(()) => unreachable!("up/down options with config should parse"),
        };
        assert_eq!(parsed.state_path, "tmp/state.json");
        assert_eq!(
            parsed.config_path,
            Some("configs/mesh-node.example.conf".to_string())
        );
        assert!(!parsed.skip_connect_check);

        let args = vec![
            "--state-file".to_string(),
            "tmp/state.json".to_string(),
            "--skip-connect-check".to_string(),
            "true".to_string(),
        ];
        let parsed = match parse_up_down_options(&args) {
            Ok(parsed) => parsed,
            Err(()) => unreachable!("up/down options with skip-connect-check should parse"),
        };
        assert!(parsed.skip_connect_check);

        let args = vec![
            "--tun-name".to_string(),
            "chimera42".to_string(),
            "--tun-local-cidr".to_string(),
            "10.55.0.2/30".to_string(),
            "--tun-peer-cidr".to_string(),
            "10.55.0.1/30".to_string(),
            "--apply-route".to_string(),
            "true".to_string(),
            "--route-cidr".to_string(),
            "203.0.113.0/24".to_string(),
            "--route-policy".to_string(),
            "true".to_string(),
            "--route-table".to_string(),
            "60001".to_string(),
            "--route-rule-priority".to_string(),
            "12000".to_string(),
            "--apply-dns".to_string(),
            "true".to_string(),
            "--dns-server".to_string(),
            "9.9.9.9".to_string(),
            "--resolv-conf".to_string(),
            "/tmp/chimera_resolv.conf".to_string(),
        ];
        let parsed = match parse_up_down_options(&args) {
            Ok(parsed) => parsed,
            Err(()) => unreachable!("tun options should parse"),
        };
        assert_eq!(parsed.tun_name, "chimera42");
        assert_eq!(parsed.tun_local_cidr, "10.55.0.2/30");
        assert_eq!(parsed.tun_peer_cidr, "10.55.0.1/30");
        assert!(parsed.apply_route);
        assert_eq!(parsed.route_cidr, "203.0.113.0/24");
        assert!(parsed.route_policy);
        assert_eq!(parsed.route_table, "60001");
        assert_eq!(parsed.route_rule_priority, "12000");
        assert!(parsed.apply_dns);
        assert_eq!(parsed.dns_server, "9.9.9.9");
        assert_eq!(parsed.resolv_conf_path, "/tmp/chimera_resolv.conf");

        let invalid_args = vec![
            "--route-policy".to_string(),
            "true".to_string(),
            "--route-table".to_string(),
            "60001".to_string(),
            "--route-rule-priority".to_string(),
            "12000".to_string(),
        ];
        assert!(parse_up_down_options(&invalid_args).is_err());
    }

    #[test]
    fn parse_rollback_options_json_and_out() {
        let args = vec![
            "--state-file".to_string(),
            "tmp/state.json".to_string(),
            "--json".to_string(),
            "--out".to_string(),
            "tmp/rollback.json".to_string(),
        ];
        let parsed = match parse_rollback_options(&args) {
            Ok(parsed) => parsed,
            Err(()) => unreachable!("rollback options should parse"),
        };
        assert_eq!(parsed.up_down.state_path, "tmp/state.json");
        assert!(parsed.json_output);
        assert_eq!(parsed.out_path, Some("tmp/rollback.json".to_string()));
    }

    #[test]
    fn parse_probe_options_defaults() {
        let args = vec!["--url".to_string(), "https://example.org".to_string()];
        let parsed = crate::parse_probe_options(&args);
        match parsed {
            Ok(options) => {
                assert_eq!(options.urls, vec!["https://example.org".to_string()]);
                assert_eq!(options.url_file, None);
                assert_eq!(options.timeout_seconds, 8);
                assert_eq!(options.apply_policy_path, None);
                assert_eq!(options.rule_id_prefix, "auto-probe");
                assert_eq!(options.fail_threshold, 0);
                assert!(!options.json_output);
                assert_eq!(options.out_path, None);
            }
            Err(()) => unreachable!("probe options should parse"),
        }
    }

    #[test]
    fn parse_probe_options_full() {
        let args = vec![
            "--url".to_string(),
            "https://youtube.com".to_string(),
            "--url".to_string(),
            "https://discord.com".to_string(),
            "--url-file".to_string(),
            "/tmp/targets.txt".to_string(),
            "--timeout-sec".to_string(),
            "15".to_string(),
            "--apply-policy".to_string(),
            "/tmp/runtime.policy".to_string(),
            "--rule-id-prefix".to_string(),
            "myprobe".to_string(),
            "--fail-threshold".to_string(),
            "2".to_string(),
            "--json".to_string(),
            "--out".to_string(),
            "/tmp/probe.json".to_string(),
        ];
        let parsed = crate::parse_probe_options(&args);
        match parsed {
            Ok(options) => {
                assert_eq!(
                    options.urls,
                    vec![
                        "https://youtube.com".to_string(),
                        "https://discord.com".to_string()
                    ]
                );
                assert_eq!(options.url_file, Some("/tmp/targets.txt".to_string()));
                assert_eq!(options.timeout_seconds, 15);
                assert_eq!(
                    options.apply_policy_path,
                    Some("/tmp/runtime.policy".to_string())
                );
                assert_eq!(options.rule_id_prefix, "myprobe");
                assert_eq!(options.fail_threshold, 2);
                assert!(options.json_output);
                assert_eq!(options.out_path, Some("/tmp/probe.json".to_string()));
            }
            Err(()) => unreachable!("probe options should parse"),
        }
    }

    #[test]
    fn parse_probe_options_rejects_proxy_url() {
        let args = vec![
            "--url".to_string(),
            "https://example.org".to_string(),
            "--proxy-url".to_string(),
            "socks5h://127.0.0.1:11080".to_string(),
        ];
        assert!(crate::parse_probe_options(&args).is_err());
    }

    #[test]
    fn parse_probe_options_rejects_invalid_timeout() {
        let args = vec![
            "--url".to_string(),
            "https://example.org".to_string(),
            "--timeout-sec".to_string(),
            "0".to_string(),
        ];
        assert!(crate::parse_probe_options(&args).is_err());
    }

    #[test]
    fn collect_probe_urls_combines_flags_and_file() {
        let mut path = std::env::temp_dir();
        path.push("chimera_probe_targets.txt");
        let body = "\
# comment\n\
https://www.youtube.com\n\
\n\
https://discord.com\n";
        let _ = std::fs::write(&path, body);
        let options = crate::ProbeOptions {
            urls: vec!["https://example.org".to_string()],
            url_file: Some(path.to_string_lossy().to_string()),
            timeout_seconds: 8,
            apply_policy_path: None,
            rule_id_prefix: "auto-probe".to_string(),
            fail_threshold: 0,
            json_output: false,
            out_path: None,
        };
        let urls = crate::collect_probe_urls(&options).unwrap_or_default();
        assert_eq!(
            urls,
            vec![
                "https://example.org".to_string(),
                "https://www.youtube.com".to_string(),
                "https://discord.com".to_string()
            ]
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn collect_probe_urls_deduplicates_case_insensitively() {
        let options = crate::ProbeOptions {
            urls: vec![
                "https://EXAMPLE.org".to_string(),
                "https://example.org".to_string(),
                "https://discord.com".to_string(),
            ],
            url_file: None,
            timeout_seconds: 8,
            apply_policy_path: None,
            rule_id_prefix: "auto-probe".to_string(),
            fail_threshold: 0,
            json_output: false,
            out_path: None,
        };
        let urls = crate::collect_probe_urls(&options).unwrap_or_default();
        assert_eq!(
            urls,
            vec![
                "https://EXAMPLE.org".to_string(),
                "https://discord.com".to_string()
            ]
        );
    }

    #[test]
    fn is_ip_literal_detects_ipv4_and_ipv6() {
        assert!(crate::is_ip_literal("203.0.113.7"));
        assert!(crate::is_ip_literal("2001:db8::1"));
        assert!(!crate::is_ip_literal("example.org"));
    }

    #[test]
    fn parse_probe_options_rejects_invalid_fail_threshold() {
        let args = vec![
            "--url".to_string(),
            "https://example.org".to_string(),
            "--fail-threshold".to_string(),
            "bad".to_string(),
        ];
        assert!(crate::parse_probe_options(&args).is_err());
    }

    #[test]
    fn extract_domain_from_url_handles_common_forms() {
        assert_eq!(
            crate::extract_domain_from_url("https://www.youtube.com/watch?v=1"),
            Some("www.youtube.com".to_string())
        );
        assert_eq!(
            crate::extract_domain_from_url("http://USER:PASS@Example.ORG:8080/path"),
            Some("example.org".to_string())
        );
        assert_eq!(
            crate::extract_domain_from_url("https://localhost./"),
            Some("localhost".to_string())
        );
    }

    #[test]
    fn extract_domain_from_url_rejects_invalid_shapes() {
        assert_eq!(crate::extract_domain_from_url("not-a-url"), None);
        assert_eq!(crate::extract_domain_from_url("https://"), None);
        assert_eq!(crate::extract_domain_from_url("https://:8080"), None);
    }

    #[test]
    fn apply_probe_policy_rule_creates_file_with_default_and_exact_rule() {
        let mut path = std::env::temp_dir();
        path.push("chimera_probe_policy_create.conf");
        let _ = std::fs::remove_file(&path);
        let path_text = path.to_string_lossy().to_string();
        let result = crate::apply_probe_policy_rule(
            &path_text,
            "auto-probe",
            "www.youtube.com",
            OutboundMode::Transit,
        );
        assert_eq!(result, Ok("auto-probe-www.youtube.com".to_string()));
        let content = std::fs::read_to_string(&path_text).unwrap_or_default();
        assert!(content.contains("default = default => direct"));
        assert!(content.contains("auto-probe-www.youtube.com = exact:www.youtube.com => transit"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn apply_probe_policy_rule_updates_existing_exact_match() {
        let mut path = std::env::temp_dir();
        path.push("chimera_probe_policy_update.conf");
        let path_text = path.to_string_lossy().to_string();
        let initial = "\
# policy\n\
old-id = exact:www.youtube.com => direct\n\
default = default => direct\n";
        let _ = std::fs::write(&path, initial);
        let result = crate::apply_probe_policy_rule(
            &path_text,
            "auto-probe",
            "www.youtube.com",
            OutboundMode::Transit,
        );
        assert_eq!(result, Ok("auto-probe-www.youtube.com".to_string()));
        let content = std::fs::read_to_string(&path_text).unwrap_or_default();
        assert!(!content.contains("old-id = exact:www.youtube.com => direct"));
        assert!(content.contains("auto-probe-www.youtube.com = exact:www.youtube.com => transit"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn verify_probe_policy_route_matches_expected_outbound() {
        let mut path = std::env::temp_dir();
        path.push("chimera_probe_policy_verify_ok.conf");
        let path_text = path.to_string_lossy().to_string();
        let body = "\
default = default => direct\n\
yt = exact:www.youtube.com => transit\n";
        let _ = std::fs::write(&path, body);
        let result =
            crate::verify_probe_policy_route(&path_text, "www.youtube.com", OutboundMode::Transit);
        assert_eq!(result, Ok((true, "transit".to_string())));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn verify_probe_policy_route_reports_mismatch() {
        let mut path = std::env::temp_dir();
        path.push("chimera_probe_policy_verify_mismatch.conf");
        let path_text = path.to_string_lossy().to_string();
        let body = "\
default = default => direct\n\
yt = exact:www.youtube.com => direct\n";
        let _ = std::fs::write(&path, body);
        let result =
            crate::verify_probe_policy_route(&path_text, "www.youtube.com", OutboundMode::Transit);
        assert_eq!(result, Ok((false, "direct".to_string())));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn atomic_write_text_file_replaces_existing_file() {
        let mut path = std::env::temp_dir();
        path.push("chimera_atomic_write_test.conf");
        let path_text = path.to_string_lossy().to_string();
        let _ = std::fs::write(&path, "old\n");
        assert!(crate::atomic_write_text_file(&path, "new\n").is_ok());
        let content = std::fs::read_to_string(&path_text).unwrap_or_default();
        assert_eq!(content, "new\n");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn validate_policy_route_options_accepts_positive_numbers() {
        assert!(crate::validate_policy_route_options("51820", "11000").is_ok());
        assert!(crate::validate_policy_route_options("1", "1000").is_ok());
    }

    #[test]
    fn validate_policy_route_options_rejects_invalid_values() {
        assert!(crate::validate_policy_route_options("0", "11000").is_err());
        assert!(crate::validate_policy_route_options("253", "11000").is_err());
        assert!(crate::validate_policy_route_options("254", "11000").is_err());
        assert!(crate::validate_policy_route_options("255", "11000").is_err());
        assert!(crate::validate_policy_route_options("51820", "0").is_err());
        assert!(crate::validate_policy_route_options("51820", "999").is_err());
        assert!(crate::validate_policy_route_options("main", "11000").is_err());
        assert!(crate::validate_policy_route_options("51820", "pref").is_err());
    }

    #[test]
    fn ip_absent_delete_error_detection() {
        assert!(crate::is_ip_absent_delete_error(
            "RTNETLINK answers: No such process"
        ));
        assert!(crate::is_ip_absent_delete_error(
            "Cannot find device \"chimera0\""
        ));
        assert!(!crate::is_ip_absent_delete_error("Operation not permitted"));
    }

    #[test]
    fn up_command_rejects_invalid_policy_route_options_early() {
        let args = vec![
            "--skip-connect-check".to_string(),
            "true".to_string(),
            "--apply-route".to_string(),
            "true".to_string(),
            "--route-policy".to_string(),
            "true".to_string(),
            "--route-table".to_string(),
            "main".to_string(),
            "--route-rule-priority".to_string(),
            "11000".to_string(),
        ];
        assert_eq!(up_command(Language::En, &args), 2);

        let args2 = vec![
            "--skip-connect-check".to_string(),
            "true".to_string(),
            "--apply-route".to_string(),
            "true".to_string(),
            "--route-policy".to_string(),
            "true".to_string(),
            "--route-table".to_string(),
            "60001".to_string(),
            "--route-rule-priority".to_string(),
            "0".to_string(),
        ];
        assert_eq!(up_command(Language::En, &args2), 2);
    }

    #[test]
    fn validate_route_cidr_accepts_ipv4_and_ipv6() {
        assert!(crate::validate_route_cidr("203.0.113.0/24").is_ok());
        assert!(crate::validate_route_cidr("2001:db8::/64").is_ok());
    }

    #[test]
    fn validate_route_cidr_rejects_invalid_values() {
        assert!(crate::validate_route_cidr("203.0.113.0").is_err());
        assert!(crate::validate_route_cidr("203.0.113.0/x").is_err());
        assert!(crate::validate_route_cidr("203.0.113.0/33").is_err());
        assert!(crate::validate_route_cidr("2001:db8::/129").is_err());
        assert!(crate::validate_route_cidr("bad/24").is_err());
    }

    #[test]
    fn parse_route_cidr_list_accepts_multi_value() {
        assert_eq!(
            crate::parse_route_cidr_list("203.0.113.0/24, 198.51.100.0/24"),
            Ok(vec![
                "203.0.113.0/24".to_string(),
                "198.51.100.0/24".to_string(),
            ])
        );
    }

    #[test]
    fn parse_route_cidr_list_rejects_empty_items() {
        assert!(crate::parse_route_cidr_list("").is_err());
        assert!(crate::parse_route_cidr_list("203.0.113.0/24,").is_err());
        assert!(crate::parse_route_cidr_list(",203.0.113.0/24").is_err());
    }

    #[test]
    fn parse_route_cidr_list_rejects_duplicates() {
        assert!(crate::parse_route_cidr_list("203.0.113.0/24,203.0.113.0/24").is_err());
        assert!(crate::parse_route_cidr_list("203.0.113.0/24, 203.0.113.0/24").is_err());
    }

    #[test]
    fn resolve_route_cidrs_for_rollback_prefers_applied_list() {
        let state = "{\"route_cidr\":\"0.0.0.0/1\",\"route_cidrs_applied\":\"203.0.113.0/24,198.51.100.0/24\"}";
        let resolved = crate::resolve_route_cidrs_for_rollback(state, "10.0.0.0/8");
        assert_eq!(
            resolved,
            vec!["203.0.113.0/24".to_string(), "198.51.100.0/24".to_string()]
        );
    }

    #[test]
    fn resolve_route_cidrs_for_rollback_falls_back_to_route_cidr() {
        let state = "{\"route_cidr\":\"0.0.0.0/1\"}";
        let resolved = crate::resolve_route_cidrs_for_rollback(state, "10.0.0.0/8");
        assert_eq!(resolved, vec!["0.0.0.0/1".to_string()]);
    }

    #[test]
    fn resolve_route_cidrs_for_rollback_falls_back_to_options() {
        let state = "{\"status\":\"up\"}";
        let resolved = crate::resolve_route_cidrs_for_rollback(state, "10.0.0.0/8");
        assert_eq!(resolved, vec!["10.0.0.0/8".to_string()]);
    }

    #[test]
    fn resolve_route_cidrs_for_rollback_from_value_prefers_applied_list() {
        let value: serde_json::Value = serde_json::json!({
            "route_cidr":"0.0.0.0/1",
            "route_cidrs_applied":"203.0.113.0/24,198.51.100.0/24"
        });
        let resolved = crate::resolve_route_cidrs_for_rollback_from_value(&value, "10.0.0.0/8");
        assert_eq!(
            resolved,
            vec!["203.0.113.0/24".to_string(), "198.51.100.0/24".to_string()]
        );
    }

    #[test]
    fn resolve_route_cidrs_for_rollback_from_value_falls_back_to_options() {
        let value: serde_json::Value = serde_json::json!({ "status":"up" });
        let resolved = crate::resolve_route_cidrs_for_rollback_from_value(&value, "10.0.0.0/8");
        assert_eq!(resolved, vec!["10.0.0.0/8".to_string()]);
    }

    #[test]
    fn escape_json_escapes_quotes_backslashes_and_controls() {
        let value = "a\"b\\c\n";
        assert_eq!(crate::escape_json(value), "a\\\"b\\\\c\\n");
    }

    #[test]
    fn extract_state_string_field_handles_escaped_json_value() {
        let state = "{\"path\":\"/tmp/a\\\"b\\\\c\\n\\u0041\"}";
        let value = crate::extract_state_string_field(state, "path");
        assert_eq!(value, Some("/tmp/a\"b\\c\nA".to_string()));
    }

    #[test]
    fn extract_state_string_field_rejects_broken_escape_sequence() {
        let state = "{\"path\":\"/tmp/a\\x\"}";
        assert_eq!(crate::extract_state_string_field(state, "path"), None);
    }

    #[test]
    fn extract_state_bool_field_reads_true_false_with_spacing() {
        let state = "{ \"dns_applied\" : true, \"route_applied\": false }";
        assert_eq!(
            crate::extract_state_bool_field(state, "dns_applied"),
            Some(true)
        );
        assert_eq!(
            crate::extract_state_bool_field(state, "route_applied"),
            Some(false)
        );
        assert_eq!(crate::extract_state_bool_field(state, "tun_applied"), None);
    }

    #[test]
    fn extract_state_bool_field_rejects_non_boolean_literals() {
        let state = "{\"dns_applied\":truex,\"route_applied\":\"true\"}";
        assert_eq!(crate::extract_state_bool_field(state, "dns_applied"), None);
        assert_eq!(
            crate::extract_state_bool_field(state, "route_applied"),
            None
        );
    }

    #[test]
    fn extract_state_bool_field_uses_json_field_not_string_fragment() {
        let state = "{\"note\":\"dns_applied:true\",\"dns_applied\":false}";
        assert_eq!(
            crate::extract_state_bool_field(state, "dns_applied"),
            Some(false)
        );
    }

    #[test]
    fn datapath_state_proof_accepts_valid_state() -> Result<(), Box<dyn std::error::Error>> {
        let mut path = std::env::temp_dir();
        path.push("chimera_cli_datapath_state_proof_valid.json");
        std::fs::write(
            &path,
            r#"{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true}"#,
        )?;
        assert_eq!(crate::validate_datapath_state_proof(&path), Ok(()));
        let _ = std::fs::remove_file(path);
        Ok(())
    }

    #[test]
    fn datapath_state_proof_rejects_missing_invalid_and_weak_state()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut path = std::env::temp_dir();
        path.push("chimera_cli_datapath_state_proof_invalid.json");
        let _ = std::fs::remove_file(&path);
        assert_eq!(
            crate::validate_datapath_state_proof(&path),
            Err("missing_state")
        );
        std::fs::write(&path, "{not json")?;
        assert_eq!(
            crate::validate_datapath_state_proof(&path),
            Err("state_invalid_json")
        );
        std::fs::write(
            &path,
            r#"{"status":"up","network_state":"not_modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true}"#,
        )?;
        assert_eq!(
            crate::validate_datapath_state_proof(&path),
            Err("network_not_modified")
        );
        std::fs::write(
            &path,
            r#"{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":false}"#,
        )?;
        assert_eq!(
            crate::validate_datapath_state_proof(&path),
            Err("dns_not_applied")
        );
        let _ = std::fs::remove_file(path);
        Ok(())
    }

    #[test]
    fn datapath_state_proof_rejects_duplicate_required_keys()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut path = std::env::temp_dir();
        path.push("chimera_cli_datapath_state_proof_duplicate.json");
        std::fs::write(
            &path,
            r#"{"status":"down","status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true}"#,
        )?;
        assert_eq!(
            crate::validate_datapath_state_proof(&path),
            Err("duplicate_field")
        );
        let _ = std::fs::remove_file(path);
        Ok(())
    }

    #[test]
    fn datapath_flow_proof_requires_fresh_valid_sidecar() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut path = std::env::temp_dir();
        path.push("chimera_cli_datapath_flow_proof_valid.json");
        std::fs::write(
            &path,
            r#"{"status":"ok","kind":"chimera_datapath_flow_proof","flow_id":"flow#1","path_kind":"local_egress_via_secure_peer","transparent_flow_observed":true,"counter_delta_ok":true,"secure_peer_egress_observed":true,"secure_peer_bytes_delta_ok":true,"network_state":"modified"}"#,
        )?;
        assert_eq!(crate::validate_datapath_flow_proof(&path, 300), Ok(()));
        let _ = std::fs::remove_file(path);
        Ok(())
    }

    #[test]
    fn datapath_flow_proof_rejects_missing_file() {
        let mut path = std::env::temp_dir();
        path.push("chimera_cli_datapath_flow_proof_missing.json");
        let _ = std::fs::remove_file(&path);
        assert_eq!(
            crate::validate_datapath_flow_proof(&path, 300),
            Err("missing_flow_proof")
        );
    }

    #[test]
    fn datapath_flow_proof_rejects_duplicate_required_keys()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut path = std::env::temp_dir();
        path.push("chimera_cli_datapath_flow_proof_duplicate.json");
        std::fs::write(
            &path,
            r#"{"status":"ok","kind":"chimera_datapath_flow_proof","flow_id":"flow#1","path_kind":"local_egress_via_secure_peer","transparent_flow_observed":true,"counter_delta_ok":true,"secure_peer_egress_observed":true,"secure_peer_bytes_delta_ok":true,"network_state":"modified","network_state":"modified"}"#,
        )?;
        assert_eq!(
            crate::validate_datapath_flow_proof(&path, 300),
            Err("flow_duplicate_field")
        );
        let _ = std::fs::remove_file(path);
        Ok(())
    }

    #[test]
    fn state_proof_command_requires_flow_when_requested() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut state_path = std::env::temp_dir();
        state_path.push("chimera_cli_state_proof_command_flow.json");
        let flow_path = std::path::PathBuf::from(format!("{}.flow.json", state_path.display()));
        let _ = std::fs::remove_file(&state_path);
        let _ = std::fs::remove_file(&flow_path);
        std::fs::write(
            &state_path,
            r#"{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true}"#,
        )?;
        let args = [
            "proof".to_string(),
            "--state-file".to_string(),
            state_path.to_string_lossy().to_string(),
            "--require-flow".to_string(),
            "true".to_string(),
        ];
        assert_eq!(
            crate::state_command(Language::En, Some("proof"), &args[1..]),
            1
        );
        std::fs::write(
            &flow_path,
            r#"{"status":"ok","kind":"chimera_datapath_flow_proof","flow_id":"flow#1","path_kind":"local_egress_via_secure_peer","transparent_flow_observed":true,"counter_delta_ok":true,"secure_peer_egress_observed":true,"secure_peer_bytes_delta_ok":true,"network_state":"modified"}"#,
        )?;
        assert_eq!(
            crate::state_command(Language::En, Some("proof"), &args[1..]),
            0
        );
        let _ = std::fs::remove_file(state_path);
        let _ = std::fs::remove_file(flow_path);
        Ok(())
    }

    #[test]
    fn state_proof_command_returns_nonzero_without_valid_state()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut path = std::env::temp_dir();
        path.push("chimera_cli_state_proof_command.json");
        let _ = std::fs::remove_file(&path);
        let args = [
            "proof".to_string(),
            "--state-file".to_string(),
            path.to_string_lossy().to_string(),
        ];
        assert_eq!(
            crate::state_command(Language::En, Some("proof"), &args[1..]),
            1
        );
        std::fs::write(
            &path,
            r#"{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true}"#,
        )?;
        assert_eq!(
            crate::state_command(Language::En, Some("proof"), &args[1..]),
            0
        );
        let _ = std::fs::remove_file(path);
        Ok(())
    }

    #[test]
    fn validate_route_cidrs_accepts_multi_value() {
        assert!(crate::validate_route_cidrs("203.0.113.0/24,198.51.100.0/24").is_ok());
        assert!(crate::validate_route_cidrs("2001:db8::/64,203.0.113.0/24").is_ok());
    }

    #[test]
    fn validate_route_cidrs_rejects_invalid_values() {
        assert!(crate::validate_route_cidrs("203.0.113.0/24,bad").is_err());
        assert!(crate::validate_route_cidrs("203.0.113.0/33,198.51.100.0/24").is_err());
        assert!(crate::validate_route_cidrs("203.0.113.0/24,203.0.113.0/24").is_err());
    }

    #[test]
    fn up_command_rejects_invalid_route_cidr_early() {
        let args = vec![
            "--skip-connect-check".to_string(),
            "true".to_string(),
            "--apply-route".to_string(),
            "true".to_string(),
            "--route-cidr".to_string(),
            "203.0.113.0/33".to_string(),
        ];
        assert_eq!(up_command(Language::En, &args), 2);

        let args_multi = vec![
            "--skip-connect-check".to_string(),
            "true".to_string(),
            "--apply-route".to_string(),
            "true".to_string(),
            "--route-cidr".to_string(),
            "203.0.113.0/24,bad".to_string(),
        ];
        assert_eq!(up_command(Language::En, &args_multi), 2);
    }

    #[test]
    fn validate_tun_cidr_pair_accepts_valid_values() {
        assert!(crate::validate_tun_cidr_pair("10.99.0.2/30", "10.99.0.1/30").is_ok());
        assert!(crate::validate_tun_cidr_pair("2001:db8::2/126", "2001:db8::1/126").is_ok());
    }

    #[test]
    fn validate_tun_cidr_pair_rejects_invalid_values() {
        assert!(crate::validate_tun_cidr_pair("10.99.0.2/33", "10.99.0.1/30").is_err());
        assert!(crate::validate_tun_cidr_pair("10.99.0.2/30", "bad/30").is_err());
        assert!(crate::validate_tun_cidr_pair("10.99.0.2/30", "10.99.0.2/30").is_err());
        assert!(crate::validate_tun_cidr_pair("10.99.0.2/30", "2001:db8::1/126").is_err());
    }

    #[test]
    fn up_command_rejects_invalid_tun_cidr_early() {
        let args = vec![
            "--skip-connect-check".to_string(),
            "true".to_string(),
            "--apply-tun".to_string(),
            "true".to_string(),
            "--tun-local-cidr".to_string(),
            "10.99.0.2/33".to_string(),
            "--tun-peer-cidr".to_string(),
            "10.99.0.1/30".to_string(),
        ];
        assert_eq!(up_command(Language::En, &args), 2);
    }

    #[test]
    fn validate_dns_server_accepts_ipv4_and_ipv6() {
        assert!(crate::validate_dns_server("9.9.9.9").is_ok());
        assert!(crate::validate_dns_server("2001:4860:4860::8888").is_ok());
    }

    #[test]
    fn validate_dns_server_rejects_invalid_values() {
        assert!(crate::validate_dns_server("dns.google").is_err());
        assert!(crate::validate_dns_server("999.999.999.999").is_err());
        assert!(crate::validate_dns_server("not-an-ip").is_err());
    }

    #[test]
    fn up_command_rejects_invalid_dns_server_early() {
        let args = vec![
            "--skip-connect-check".to_string(),
            "true".to_string(),
            "--apply-dns".to_string(),
            "true".to_string(),
            "--dns-server".to_string(),
            "dns.google".to_string(),
            "--resolv-conf".to_string(),
            "/tmp/chimera_invalid_dns_smoke.conf".to_string(),
        ];
        assert_eq!(up_command(Language::En, &args), 2);
    }

    #[test]
    fn validate_resolv_conf_path_accepts_absolute_path() {
        let mut path = std::env::temp_dir();
        path.push("chimera_resolv_validate_ok.conf");
        let _ = std::fs::write(&path, "nameserver 1.1.1.1\n");
        let result = crate::validate_resolv_conf_path(&path.to_string_lossy());
        assert!(result.is_ok());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn validate_resolv_conf_path_rejects_invalid_values() {
        assert!(crate::validate_resolv_conf_path("").is_err());
        assert!(crate::validate_resolv_conf_path("relative/resolv.conf").is_err());

        let mut missing = std::env::temp_dir();
        missing.push("chimera_resolv_validate_missing.conf");
        let _ = std::fs::remove_file(&missing);
        assert!(crate::validate_resolv_conf_path(&missing.to_string_lossy()).is_err());

        let dir = std::env::temp_dir();
        assert!(crate::validate_resolv_conf_path(&dir.to_string_lossy()).is_err());
    }

    #[test]
    fn up_command_rejects_invalid_resolv_conf_path_early() {
        let args = vec![
            "--skip-connect-check".to_string(),
            "true".to_string(),
            "--apply-dns".to_string(),
            "true".to_string(),
            "--dns-server".to_string(),
            "9.9.9.9".to_string(),
            "--resolv-conf".to_string(),
            "relative/resolv.conf".to_string(),
        ];
        assert_eq!(up_command(Language::En, &args), 2);
    }

    #[test]
    fn validate_tun_name_accepts_valid_values() {
        assert!(crate::validate_tun_name("chimera0").is_ok());
        assert!(crate::validate_tun_name("chimera-pre_1").is_ok());
    }

    #[test]
    fn validate_tun_name_rejects_invalid_values() {
        assert!(crate::validate_tun_name("").is_err());
        assert!(crate::validate_tun_name("chimera interface").is_err());
        assert!(crate::validate_tun_name("chimera@if").is_err());
        assert!(crate::validate_tun_name("chimera_interface_name_too_long").is_err());
    }

    #[test]
    fn up_command_rejects_invalid_tun_name_early() {
        let args = vec![
            "--skip-connect-check".to_string(),
            "true".to_string(),
            "--apply-tun".to_string(),
            "true".to_string(),
            "--tun-name".to_string(),
            "bad@if".to_string(),
            "--tun-local-cidr".to_string(),
            "10.99.0.2/30".to_string(),
            "--tun-peer-cidr".to_string(),
            "10.99.0.1/30".to_string(),
        ];
        assert_eq!(up_command(Language::En, &args), 2);
    }

    #[test]
    fn up_command_rejects_invalid_tun_name_for_route_mode_too() {
        let args = vec![
            "--skip-connect-check".to_string(),
            "true".to_string(),
            "--apply-route".to_string(),
            "true".to_string(),
            "--route-policy".to_string(),
            "true".to_string(),
            "--route-table".to_string(),
            "60001".to_string(),
            "--route-rule-priority".to_string(),
            "12000".to_string(),
            "--tun-name".to_string(),
            "bad@if".to_string(),
        ];
        assert_eq!(up_command(Language::En, &args), 2);
    }

    #[test]
    fn apply_dns_resolver_rejects_same_backup_path() {
        let mut path = std::env::temp_dir();
        path.push("chimera_same_dns_path_test.conf");
        let _ = std::fs::write(&path, "nameserver 1.1.1.1\n");
        let same = path.to_string_lossy().to_string();
        let result = crate::apply_dns_resolver(&same, &same, "9.9.9.9");
        assert!(result.is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn up_down_lifecycle_creates_and_removes_state_file() {
        let mut path = std::env::temp_dir();
        path.push("chimera_cli_runtime_state_test.json");
        let path_text = path.to_string_lossy().to_string();
        let config_path = test_node_config_path();
        let up_args = vec![
            "--state-file".to_string(),
            path_text.clone(),
            "--config".to_string(),
            config_path.to_string_lossy().to_string(),
            "--skip-connect-check".to_string(),
            "true".to_string(),
        ];
        assert_eq!(up_command(Language::En, &up_args), 0);
        assert!(PathBuf::from(&path_text).exists());

        let down_args = vec!["--state-file".to_string(), path_text.clone()];
        assert_eq!(down_command(Language::En, &down_args), 0);
        assert!(!PathBuf::from(&path_text).exists());

        assert_eq!(down_command(Language::En, &down_args), 0);
    }

    #[test]
    fn rollback_status_and_clean_flow() {
        let mut path = std::env::temp_dir();
        path.push("chimera_cli_runtime_state_rollback_test.json");
        let path_text = path.to_string_lossy().to_string();
        let config_path = test_node_config_path();
        let up_args = vec![
            "--state-file".to_string(),
            path_text.clone(),
            "--config".to_string(),
            config_path.to_string_lossy().to_string(),
            "--skip-connect-check".to_string(),
            "true".to_string(),
        ];
        assert_eq!(up_command(Language::En, &up_args), 0);
        assert!(PathBuf::from(&path_text).exists());

        let status_args = vec![
            "status".to_string(),
            "--state-file".to_string(),
            path_text.clone(),
        ];
        assert_eq!(rollback_command(Language::En, &status_args), 0);

        let clean_args = vec![
            "clean".to_string(),
            "--state-file".to_string(),
            path_text.clone(),
        ];
        assert_eq!(rollback_command(Language::En, &clean_args), 0);
        assert!(!PathBuf::from(&path_text).exists());
    }

    #[test]
    fn rollback_recover_flow() {
        let mut path = std::env::temp_dir();
        path.push("chimera_cli_runtime_state_recover_test.json");
        let path_text = path.to_string_lossy().to_string();
        let config_path = test_node_config_path();
        let up_args = vec![
            "--state-file".to_string(),
            path_text.clone(),
            "--config".to_string(),
            config_path.to_string_lossy().to_string(),
            "--skip-connect-check".to_string(),
            "true".to_string(),
        ];
        assert_eq!(up_command(Language::En, &up_args), 0);
        assert!(PathBuf::from(&path_text).exists());

        let recover_args = vec![
            "recover".to_string(),
            "--state-file".to_string(),
            path_text.clone(),
        ];
        assert_eq!(rollback_command(Language::En, &recover_args), 0);
        assert!(!PathBuf::from(&path_text).exists());
        assert_eq!(rollback_command(Language::En, &recover_args), 0);
    }

    #[test]
    fn rollback_status_json_output() {
        let mut state_path = std::env::temp_dir();
        state_path.push("chimera_cli_runtime_state_json_test.json");
        let state_text = state_path.to_string_lossy().to_string();
        let config_path = test_node_config_path();
        let up_args = vec![
            "--state-file".to_string(),
            state_text.clone(),
            "--config".to_string(),
            config_path.to_string_lossy().to_string(),
            "--skip-connect-check".to_string(),
            "true".to_string(),
        ];
        assert_eq!(up_command(Language::En, &up_args), 0);
        assert!(PathBuf::from(&state_text).exists());

        let mut out_path = std::env::temp_dir();
        out_path.push("chimera_cli_rollback_status_json_test.json");
        let out_text = out_path.to_string_lossy().to_string();
        let status_json_args = vec![
            "status".to_string(),
            "--state-file".to_string(),
            state_text.clone(),
            "--json".to_string(),
            "--out".to_string(),
            out_text.clone(),
        ];
        assert_eq!(rollback_command(Language::En, &status_json_args), 0);
        let report_result = std::fs::read_to_string(&out_text);
        assert!(report_result.is_ok());
        let report = report_result.unwrap_or_default();
        assert!(report.contains("\"kind\":\"rollback\""));
        assert!(report.contains("\"message_en\":\"Rollback action completed.\""));
        assert!(report.contains("\"message_ru\":\"Действие rollback завершено.\""));
        assert!(report.contains("\"action\":\"status\""));
        assert!(report.contains("\"state_existed\":true"));
        assert!(report.contains("\"state_file\":\"<redacted>\""));
        assert!(report.contains("\"state_file_state\":\"present\""));
        assert!(!report.contains(&state_text));

        let clean_args = vec![
            "clean".to_string(),
            "--state-file".to_string(),
            state_text.clone(),
        ];
        assert_eq!(rollback_command(Language::En, &clean_args), 0);
        let _ = std::fs::remove_file(out_text);
    }

    #[test]
    fn rollback_status_json_reports_modified_state_flags() {
        let mut state_path = std::env::temp_dir();
        state_path.push("chimera_cli_rollback_status_modified_state.json");
        let state_text = state_path.to_string_lossy().to_string();
        let state_json = format!(
            "{{\"status\":\"up\",\"network_state\":\"modified\",\"rollback_ready\":true,\"secrets\":\"<redacted>\",\"capture_mode\":\"tun\",\"capture_reason\":\"forced\",\"carrier_profile\":\"tls-tcp\",\"carrier_addr\":\"{}\",\"carrier_server_name\":\"node.local\",\"tun_applied\":false,\"tun_device\":\"chimera0\",\"tun_local_cidr\":\"10.201.0.2/30\",\"tun_peer_cidr\":\"10.201.0.1/30\",\"route_applied\":false,\"route_cidr\":\"0.0.0.0/1\",\"route_cidrs_applied\":\"0.0.0.0/1\",\"dns_applied\":true,\"dns_server\":\"9.9.9.9\",\"resolv_conf_path\":\"/tmp/chimera_resolv_test.conf\",\"dns_backup_path\":\"/tmp/chimera_resolv_test.conf.chimera.bak\"}}\n",
            test_node_carrier_addr()
        );
        let write_result = std::fs::write(&state_text, state_json);
        assert!(write_result.is_ok());

        let mut out_path = std::env::temp_dir();
        out_path.push("chimera_cli_rollback_status_modified_out.json");
        let out_text = out_path.to_string_lossy().to_string();
        let args = vec![
            "status".to_string(),
            "--state-file".to_string(),
            state_text.clone(),
            "--json".to_string(),
            "--out".to_string(),
            out_text.clone(),
        ];
        assert_eq!(rollback_command(Language::En, &args), 0);
        let report = std::fs::read_to_string(&out_text).unwrap_or_default();
        assert!(report.contains("\"network_state\":\"modified\""));
        assert!(report.contains("\"dns_applied\":true"));
        assert!(report.contains("\"state_file\":\"<redacted>\""));
        assert!(!report.contains(&state_text));

        let _ = std::fs::remove_file(state_text);
        let _ = std::fs::remove_file(out_text);
    }

    #[test]
    fn rollback_clean_json_keeps_pre_rollback_state_details() {
        let mut resolv_path = std::env::temp_dir();
        resolv_path.push("chimera_cli_resolv_test.conf");
        let resolv_text = resolv_path.to_string_lossy().to_string();
        let write_resolv = std::fs::write(&resolv_text, "nameserver 9.9.9.9\n");
        assert!(write_resolv.is_ok());

        let backup_path = format!("{}.chimera.bak", resolv_text);
        let write_backup = std::fs::write(&backup_path, "nameserver 8.8.8.8\n");
        assert!(write_backup.is_ok());

        let mut state_path = std::env::temp_dir();
        state_path.push("chimera_cli_rollback_clean_prestate.json");
        let state_text = state_path.to_string_lossy().to_string();
        let carrier_addr = test_node_carrier_addr();
        let state_json = format!(
            "{{\"status\":\"up\",\"network_state\":\"modified\",\"rollback_ready\":true,\"secrets\":\"<redacted>\",\"capture_mode\":\"tun\",\"capture_reason\":\"forced\",\"carrier_profile\":\"tls-tcp\",\"carrier_addr\":\"{}\",\"carrier_server_name\":\"node.local\",\"tun_applied\":false,\"tun_device\":\"chimera0\",\"tun_local_cidr\":\"10.201.0.2/30\",\"tun_peer_cidr\":\"10.201.0.1/30\",\"route_applied\":false,\"route_cidr\":\"0.0.0.0/1\",\"route_cidrs_applied\":\"0.0.0.0/1\",\"dns_applied\":true,\"dns_server\":\"9.9.9.9\",\"resolv_conf_path\":\"{}\",\"dns_backup_path\":\"{}\"}}\n",
            carrier_addr, resolv_text, backup_path
        );
        let write_state = std::fs::write(&state_text, state_json);
        assert!(write_state.is_ok());

        let mut out_path = std::env::temp_dir();
        out_path.push("chimera_cli_rollback_clean_prestate_out.json");
        let out_text = out_path.to_string_lossy().to_string();
        let args = vec![
            "clean".to_string(),
            "--state-file".to_string(),
            state_text.clone(),
            "--json".to_string(),
            "--out".to_string(),
            out_text.clone(),
        ];
        assert_eq!(rollback_command(Language::En, &args), 0);
        let report = std::fs::read_to_string(&out_text).unwrap_or_default();
        assert!(report.contains("\"network_state\":\"modified\""));
        assert!(report.contains("\"dns_applied\":true"));
        let resolv_after = std::fs::read_to_string(&resolv_text).unwrap_or_default();
        assert!(resolv_after.contains("8.8.8.8"));
        assert!(!PathBuf::from(&state_text).exists());

        let _ = std::fs::remove_file(resolv_text);
        let _ = std::fs::remove_file(backup_path);
        let _ = std::fs::remove_file(out_text);
    }

    #[test]
    fn diag_export_render_contains_redacted_marker() {
        let status = StatusOptions {
            config_path: None,
            mock_packets: 5,
            mock_age_seconds: 10,
            max_age_seconds: 300,
            max_packets_per_key: 10000,
            capture_preference: CapturePreference::Auto,
            tun_supported: true,
            carrier_profile: CarrierProfile::InMemory,
            carrier_addr: crate::DEFAULT_CARRIER_ADDR.to_string(),
            carrier_server_name: crate::DEFAULT_CARRIER_SERVER_NAME.to_string(),
        };
        let plan = crate::status_capture_plan(&status);
        let json = render_diag_export_json(&status, &plan, Some(RekeyReason::PacketLimitExceeded));
        assert!(json.contains("\"secrets\":\"<redacted>\""));
        assert!(json.contains("\"carrier_addr\":\"<redacted>\""));
        assert!(json.contains("\"carrier_server_name\":\"<redacted>\""));
        assert!(json.contains("\"carrier_endpoint_state\":\"placeholder\""));
        assert!(json.contains("\"carrier_server_name_state\":\"placeholder\""));
        assert!(!json.contains(crate::DEFAULT_CARRIER_ADDR));
        assert!(!json.contains(crate::DEFAULT_CARRIER_SERVER_NAME));
        assert!(json.contains("\"kind\":\"diag_export\""));
        assert!(json.contains("\"message_en\":\"Diagnostic export is ready.\""));
        assert!(json.contains("\"message_ru\":\"Экспорт диагностики готов.\""));
        assert!(json.contains("\"rekey_reason\":\"packet_limit_exceeded\""));
        assert!(json.contains("\"network_state\":\"not_modified\""));
    }

    #[test]
    fn doctor_json_render_contains_redacted_marker() {
        let status = StatusOptions {
            config_path: None,
            mock_packets: 2,
            mock_age_seconds: 10,
            max_age_seconds: 300,
            max_packets_per_key: 10_000,
            capture_preference: CapturePreference::Tun,
            tun_supported: true,
            carrier_profile: CarrierProfile::InMemory,
            carrier_addr: crate::DEFAULT_CARRIER_ADDR.to_string(),
            carrier_server_name: crate::DEFAULT_CARRIER_SERVER_NAME.to_string(),
        };
        let plan = crate::status_capture_plan(&status);
        let json = render_doctor_json(&status, &plan, Some(RekeyReason::SessionAgeExceeded));
        assert!(json.contains("\"kind\":\"doctor\""));
        assert!(json.contains("\"message_en\":\"Doctor check is ready.\""));
        assert!(json.contains("\"message_ru\":\"Проверка doctor готова.\""));
        assert!(json.contains("\"secrets\":\"<redacted>\""));
        assert!(json.contains("\"carrier_addr\":\"<redacted>\""));
        assert!(json.contains("\"carrier_endpoint_state\":\"placeholder\""));
        assert!(!json.contains(crate::DEFAULT_CARRIER_ADDR));
        assert!(json.contains("\"rekey_reason\":\"session_age_exceeded\""));
    }

    #[test]
    fn route_render_snapshot_without_domain() {
        let ip = match "10.1.2.3".parse::<IpAddr>() {
            Ok(ip) => ip,
            Err(error) => unreachable!("ip should parse: {error}"),
        };
        let flow = FlowContext {
            domain: None,
            destination_ip: Some(ip),
            protocol: Protocol::Udp,
            port: Some(53),
        };
        let trace = RouteExplainTrace {
            decision: RouteDecision {
                matched_rule_id: "default-direct".to_string(),
                outbound: OutboundMode::Direct,
                explanation: "matched rule 'default-direct'".to_string(),
            },
            examined_rules: 2,
            matched_rules: 1,
            matched_rule_ids_by_priority: vec!["default-direct".to_string()],
        };
        let rendered = render_route_explain_block(&render_context(
            TestRouteRenderInput::default(),
            &flow,
            &trace,
        ));
        assert!(rendered.contains("Site: -"));
        assert!(rendered.contains("IP: 10.1.2.3"));
        assert!(rendered.contains("Protocol: udp"));
        assert!(rendered.contains("Port: 53"));
    }

    #[test]
    fn route_render_snapshot_show_all_matches() {
        let text = r#"
            exact_api = exact:api.example.org => block
            suffix_example = suffix:example.org => transit
            default_route = default => direct
        "#;
        let policy = match parse_policy_text(text) {
            Ok(policy) => policy,
            Err(error) => unreachable!("policy should parse: {error}"),
        };
        let flow = FlowContext {
            domain: Some("api.example.org".to_string()),
            destination_ip: None,
            protocol: Protocol::Tcp,
            port: Some(443),
        };
        let trace = policy.explain(&flow);
        let rendered = render_route_explain_block(&render_context(
            TestRouteRenderInput {
                domain: Some("api.example.org"),
                show_all_matches: true,
                ..TestRouteRenderInput::default()
            },
            &flow,
            &trace,
        ));
        assert!(
            rendered
                .contains("Matched rules (best first): exact_api, suffix_example, default_route")
        );
    }

    #[test]
    fn route_render_snapshot_dns_binding_source() {
        let destination_ip = match "203.0.113.10".parse::<IpAddr>() {
            Ok(ip) => ip,
            Err(error) => unreachable!("ip should parse: {error}"),
        };
        let flow = FlowContext {
            domain: Some("resolved.example.org".to_string()),
            destination_ip: Some(destination_ip),
            protocol: Protocol::Tcp,
            port: Some(443),
        };
        let trace = RouteExplainTrace {
            decision: RouteDecision {
                matched_rule_id: "default-direct".to_string(),
                outbound: OutboundMode::Direct,
                explanation: "matched rule 'default-direct'".to_string(),
            },
            examined_rules: 1,
            matched_rules: 1,
            matched_rule_ids_by_priority: vec!["default-direct".to_string()],
        };
        let rendered = render_route_explain_block(&render_context(
            TestRouteRenderInput {
                domain: Some("resolved.example.org"),
                domain_source_from_dns: true,
                ..TestRouteRenderInput::default()
            },
            &flow,
            &trace,
        ));
        assert!(rendered.contains("Domain source: DNS binding (IP -> domain)"));
    }

    #[test]
    fn route_render_snapshot_dns_binding_mismatch_note() {
        let flow = FlowContext {
            domain: None,
            destination_ip: None,
            protocol: Protocol::Tcp,
            port: Some(443),
        };
        let trace = RouteExplainTrace {
            decision: RouteDecision {
                matched_rule_id: "default-direct".to_string(),
                outbound: OutboundMode::Direct,
                explanation: "matched rule 'default-direct'".to_string(),
            },
            examined_rules: 1,
            matched_rules: 1,
            matched_rule_ids_by_priority: vec!["default-direct".to_string()],
        };
        let rendered = render_route_explain_block(&render_context(
            TestRouteRenderInput {
                dns_note: Some("DNS binding was provided, but IP did not match binding IP."),
                ..TestRouteRenderInput::default()
            },
            &flow,
            &trace,
        ));
        assert!(
            rendered
                .contains("DNS note: DNS binding was provided, but IP did not match binding IP.")
        );
    }

    #[test]
    fn route_render_snapshot_dns_binding_missing_note_ru() {
        let flow = FlowContext {
            domain: None,
            destination_ip: None,
            protocol: Protocol::Tcp,
            port: Some(443),
        };
        let trace = RouteExplainTrace {
            decision: RouteDecision {
                matched_rule_id: "default-direct".to_string(),
                outbound: OutboundMode::Direct,
                explanation: "matched rule 'default-direct'".to_string(),
            },
            examined_rules: 1,
            matched_rules: 1,
            matched_rule_ids_by_priority: vec!["default-direct".to_string()],
        };
        let rendered = render_route_explain_block(&render_context(
            TestRouteRenderInput {
                lang: Language::Ru,
                dns_note: Some("Для этого IP DNS binding не задан. Домен остался неизвестным."),
                ..TestRouteRenderInput::default()
            },
            &flow,
            &trace,
        ));
        assert!(rendered.contains(
            "Примечание DNS: Для этого IP DNS binding не задан. Домен остался неизвестным."
        ));
    }

    #[test]
    fn parse_route_options_dns_binding_pair_is_required() {
        let bad_args = vec![
            "--ip".to_string(),
            "203.0.113.10".to_string(),
            "--dns-bind-domain".to_string(),
            "example.org".to_string(),
        ];
        assert!(parse_route_explain_options(&bad_args).is_err());
    }

    #[test]
    fn parse_route_options_json_and_out() {
        let args = vec![
            "--domain".to_string(),
            "example.org".to_string(),
            "--json".to_string(),
            "--out".to_string(),
            "route.json".to_string(),
        ];
        let parsed = match parse_route_explain_options(&args) {
            Ok(parsed) => parsed,
            Err(()) => unreachable!("route json/out args should parse"),
        };
        assert!(parsed.json_output);
        assert_eq!(parsed.out_path, Some("route.json".to_string()));
    }

    #[test]
    fn parse_mesh_route_explain_options_with_table_policy() {
        let args = vec![
            "--namespace".to_string(),
            "cef-public".to_string(),
            "--node".to_string(),
            "node-a".to_string(),
            "--policy-payload".to_string(),
            "allow=mesh;mesh_max_peers=1".to_string(),
            "--peer".to_string(),
            "n1@198.51.100.1:443@eu@20@90".to_string(),
            "--table-max-entries".to_string(),
            "128".to_string(),
            "--table-max-per-region".to_string(),
            "16".to_string(),
            "--table-stale-after".to_string(),
            "12".to_string(),
        ];
        let parsed = match parse_mesh_route_explain_options(&args) {
            Ok(value) => value,
            Err(_) => unreachable!("mesh options with table policy should parse"),
        };
        assert_eq!(parsed.table_max_entries, Some(128));
        assert_eq!(parsed.table_max_entries_per_region, Some(16));
        assert_eq!(parsed.table_stale_after_ticks, Some(12));
    }

    #[test]
    fn parse_mesh_route_explain_options_rejects_bad_table_policy_number() {
        let args = vec![
            "--namespace".to_string(),
            "cef-public".to_string(),
            "--node".to_string(),
            "node-a".to_string(),
            "--policy-payload".to_string(),
            "allow=mesh;mesh_max_peers=1".to_string(),
            "--peer".to_string(),
            "n1@198.51.100.1:443@eu@20@90".to_string(),
            "--table-max-entries".to_string(),
            "bad".to_string(),
        ];
        assert!(parse_mesh_route_explain_options(&args).is_err());
    }

    #[test]
    fn route_render_json_contains_expected_fields() {
        let flow = FlowContext {
            domain: Some("api.example.org".to_string()),
            destination_ip: Some(
                "203.0.113.10"
                    .parse::<IpAddr>()
                    .unwrap_or_else(|_| unreachable!("ip parse should work")),
            ),
            protocol: Protocol::Tcp,
            port: Some(443),
        };
        let trace = RouteExplainTrace {
            decision: RouteDecision {
                outbound: OutboundMode::Transit,
                matched_rule_id: "example-transit".to_string(),
                explanation: "matched domain suffix rule".to_string(),
            },
            examined_rules: 2,
            matched_rules: 1,
            matched_rule_ids_by_priority: vec!["example-transit".to_string()],
        };
        let json = render_route_explain_json(&render_context(
            TestRouteRenderInput {
                domain: Some("api.example.org"),
                runtime_outbound: OutboundMode::Transit,
                show_all_matches: true,
                ..TestRouteRenderInput::default()
            },
            &flow,
            &trace,
        ));
        assert!(json.contains("\"kind\":\"route_explain\""));
        assert!(json.contains("\"message_en\":\"Route explanation is ready.\""));
        assert!(json.contains("\"message_ru\":\"Объяснение маршрута готово.\""));
        assert!(json.contains("\"outbound\":\"transit\""));
        assert!(json.contains("\"matched_rules\":[\"example-transit\"]"));
        assert!(json.contains("\"network_state\":\"not_modified\""));
    }

    #[test]
    fn policy_validate_render_snapshot_en() {
        let summary = PolicySummary {
            total_rules: 4,
            exact_domain_rules: 1,
            domain_suffix_rules: 1,
            cidr4_rules: 1,
            protoport_rules: 1,
            default_rules: 1,
            direct_outbound_rules: 2,
            transit_outbound_rules: 1,
            block_outbound_rules: 1,
        };
        let rendered = render_policy_validate_block(Language::En, &summary);
        assert!(rendered.contains("Policy file is valid."));
        assert!(rendered.contains("Total rules: 4"));
        assert!(rendered.contains("Domain rules: exact=1, suffix=1"));
        assert!(rendered.contains("IP/protocol rules: cidr4=1, protoport=1"));
        assert!(rendered.contains("Default rules: 1"));
        assert!(rendered.contains("Traffic actions: direct=2, peer transit=1, block=1"));
        assert!(!rendered.contains("local-proxy="));
    }

    #[test]
    fn policy_validate_render_snapshot_ru() {
        let summary = PolicySummary {
            total_rules: 4,
            exact_domain_rules: 1,
            domain_suffix_rules: 1,
            cidr4_rules: 0,
            protoport_rules: 1,
            default_rules: 1,
            direct_outbound_rules: 1,
            transit_outbound_rules: 2,
            block_outbound_rules: 1,
        };
        let rendered = render_policy_validate_block(Language::Ru, &summary);
        assert!(rendered.contains("Файл policy корректный."));
        assert!(rendered.contains("Всего правил: 4"));
        assert!(rendered.contains("Правила по доменам: exact=1, suffix=1"));
        assert!(rendered.contains("Правила по IP/протоколу: cidr4=0, protoport=1"));
        assert!(rendered.contains("Правила по умолчанию: default=1"));
        assert!(rendered.contains("Действия для трафика: direct=1, peer transit=2, block=1"));
        assert!(!rendered.contains("local-proxy="));
    }

    #[test]
    fn policy_validate_render_warnings_en() {
        let summary = PolicySummary {
            total_rules: 2,
            exact_domain_rules: 1,
            domain_suffix_rules: 1,
            cidr4_rules: 0,
            protoport_rules: 0,
            default_rules: 0,
            direct_outbound_rules: 2,
            transit_outbound_rules: 0,
            block_outbound_rules: 0,
        };
        let rendered = render_policy_validate_block(Language::En, &summary);
        assert!(rendered.contains("Warnings:"));
        assert!(rendered.contains("No default rule."));
        assert!(rendered.contains("No peer transit rules."));
    }

    #[test]
    fn policy_validate_render_warnings_ru() {
        let summary = PolicySummary {
            total_rules: 1,
            exact_domain_rules: 0,
            domain_suffix_rules: 0,
            cidr4_rules: 1,
            protoport_rules: 0,
            default_rules: 0,
            direct_outbound_rules: 1,
            transit_outbound_rules: 0,
            block_outbound_rules: 0,
        };
        let rendered = render_policy_validate_block(Language::Ru, &summary);
        assert!(rendered.contains("Предупреждения:"));
        assert!(rendered.contains("Нет default-правила."));
        assert!(rendered.contains("Нет правил peer transit."));
    }

    #[test]
    fn language_flag_is_parsed() {
        let args = vec![
            "chimera".to_string(),
            "--lang".to_string(),
            "ru".to_string(),
            "status".to_string(),
        ];
        let parsed = parse_language_flag(&args);
        assert_eq!(parsed, Some((Language::Ru, LanguageSource::Flag, 3)));
    }

    #[test]
    fn language_flag_overrides_env_detection() {
        let args = vec![
            "chimera".to_string(),
            "--lang".to_string(),
            "en".to_string(),
            "help".to_string(),
        ];
        let parsed = parse_language_flag(&args);
        assert_eq!(parsed, Some((Language::En, LanguageSource::Flag, 3)));
    }

    #[test]
    fn language_flag_rejects_unknown_value() {
        let args = vec![
            "chimera".to_string(),
            "--lang".to_string(),
            "de".to_string(),
            "help".to_string(),
        ];
        let parsed = parse_language_flag(&args);
        assert_eq!(parsed, None);
    }

    #[test]
    fn detect_language_from_lang_value_ru() {
        assert_eq!(
            detect_language_from_lang_value(Some("ru_RU.UTF-8")),
            Language::Ru
        );
    }

    #[test]
    fn detect_language_from_lang_value_default_en() {
        assert_eq!(
            detect_language_from_lang_value(Some("en_US.UTF-8")),
            Language::En
        );
        assert_eq!(detect_language_from_lang_value(None), Language::En);
    }

    fn test_node_config_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("configs")
            .join("mesh-node.example.conf")
    }

    fn test_node_carrier_addr() -> String {
        let text = std::fs::read_to_string(test_node_config_path())
            .unwrap_or_else(|err| unreachable!("read mesh-node.example.conf failed: {err}"));
        for line in text.lines() {
            let line = line.trim();
            if let Some(value) = line.strip_prefix("carrier.addr = ") {
                let value = value.trim();
                if !value.is_empty() {
                    return value.to_string();
                }
            }
        }
        unreachable!("carrier.addr not found in mesh-node.example.conf")
    }
}
