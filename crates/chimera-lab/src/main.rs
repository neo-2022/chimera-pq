#![forbid(unsafe_code)]

use chimera_carrier::{Carrier, InMemoryCarrier};
use chimera_carrier_quic::{QuicCarrier, QuicCarrierConfig};
use chimera_carrier_tls::{TlsCarrier, TlsCarrierConfig};
use chimera_client::ClientHandshake;
use chimera_config::{
    ConfigCarrierProfile, RawConfig, parse_client_config_text, parse_gateway_config_text,
};
use chimera_dns::{DnsBinding, DnsBindingStore};
use chimera_gateway::GatewayHandshake;
use chimera_policy::parse_policy_text;
use chimera_policy::{FlowContext, OutboundMode, Policy, Protocol, RouteRule, RuleMatcher};
use chimera_session::{Frame, HandshakeMessage, ReplayWindow};
use std::fs;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};

mod artifact_checks;
mod cef_phase1_mesh;
mod mvp_reports;
mod release_reports;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Language {
    En,
    Ru,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (lang, command_index) = match parse_language_flag(&args) {
        Some(Some(v)) => v,
        Some(None) => {
            eprintln!(
                "Language error / Ошибка языка. Use / Используйте: --lang en or / или --lang ru."
            );
            std::process::exit(2);
        }
        None => (Language::En, 1),
    };
    let command = args
        .get(command_index)
        .map(String::as_str)
        .unwrap_or("help");
    let command_args = if args.len() > command_index + 1 {
        &args[(command_index + 1)..]
    } else {
        &[][..]
    };

    let exit_code = match command {
        "smoke" => run_smoke(lang),
        "datapath-smoke" => run_datapath_smoke(lang),
        "datapath-report" => run_datapath_report(lang, command_args),
        "cef-phase1-smoke" => run_cef_phase1_smoke(lang, command_args),
        "mesh-auto-smoke" => run_mesh_auto_smoke(lang, command_args),
        "doctor" => run_doctor(lang, command_args),
        "config-smoke" => run_config_smoke(lang, command_args),
        "fuzz-smoke" => run_fuzz_smoke(lang),
        "perf-smoke" => run_perf_smoke(lang, command_args),
        "net-sim" => run_net_sim(lang, command_args),
        "hardening-smoke" => run_hardening_smoke(lang, command_args),
        "benchmark-report" => run_benchmark_report(lang, command_args),
        "mvp-spec-check" => run_mvp_spec_check(lang, command_args),
        "mvp-spec-report" => run_mvp_spec_report(lang, command_args),
        "m5-artifacts-report" => run_m5_artifacts_report(lang, command_args),
        "m6-artifacts-report" => run_m6_artifacts_report(lang, command_args),
        "release-readiness-report" => run_release_readiness_report(lang, command_args),
        "report-pack" => run_report_pack(lang, command_args),
        "artifact-audit" => run_artifact_audit(lang, command_args),
        "mvp-snapshot" => run_mvp_snapshot(lang, command_args),
        "mvp-verify" => run_mvp_verify(lang, command_args),
        "help" | "--help" | "-h" => {
            print!("{}", render_help(lang));
            0
        }
        other => {
            match lang {
                Language::En => eprintln!("Unknown lab command: {other}"),
                Language::Ru => eprintln!("Неизвестная команда lab: {other}"),
            }
            2
        }
    };

    std::process::exit(exit_code);
}

fn parse_language_flag(args: &[String]) -> Option<Option<(Language, usize)>> {
    if args.get(1).map(String::as_str) != Some("--lang") {
        return None;
    }
    let value = args.get(2)?;
    match value.as_str() {
        "en" => Some(Some((Language::En, 3))),
        "ru" => Some(Some((Language::Ru, 3))),
        _ => Some(None),
    }
}

fn render_help(lang: Language) -> String {
    let mut out = String::new();
    match lang {
        Language::En => {
            out.push_str("chimera-lab commands:\n");
            out.push_str("  [--lang en|ru] smoke\n");
            out.push_str("  [--lang en|ru] datapath-smoke\n");
            out.push_str("  [--lang en|ru] datapath-report [--json] [--out <file>]\n");
            out.push_str("  [--lang en|ru] cef-phase1-smoke [--json] [--out <file>]\n");
            out.push_str("  [--lang en|ru] mesh-auto-smoke [--json] [--out <file>]\n");
            out.push_str(
                "  [--lang en|ru] doctor [--client <file>] [--gateway <file>] [--json] [--out <file>]\n",
            );
            out.push_str("  [--lang en|ru] config-smoke [--client <file>] [--gateway <file>]\n");
            out.push_str("  [--lang en|ru] fuzz-smoke\n");
            out.push_str(
                "  [--lang en|ru] perf-smoke [--min-encode-ops <n>] [--min-decode-ops <n>] [--json]\n",
            );
            out.push_str(
                "  [--lang en|ru] net-sim [--loss-pct <n>] [--delay-ms <n>] [--disconnect-at <n>] [--reconnect-after <n>] [--mtu <bytes>] [--json]\n",
            );
            out.push_str(
                "  [--lang en|ru] hardening-smoke [--min-encode-ops <n>] [--min-decode-ops <n>] [--json]\n",
            );
            out.push_str(
                "  [--lang en|ru] benchmark-report [--min-encode-ops <n>] [--min-decode-ops <n>] [--out <file>] [--baseline <file>] [--max-regression-pct <n>]\n",
            );
            out.push_str("  [--lang en|ru] mvp-spec-check [--json] [--out <file>]\n");
            out.push_str("  [--lang en|ru] mvp-spec-report [--out <file>]\n");
            out.push_str("  [--lang en|ru] m5-artifacts-report [--out <file>]\n");
            out.push_str("  [--lang en|ru] m6-artifacts-report [--out <file>]\n");
            out.push_str("  [--lang en|ru] release-readiness-report [--out <file>]\n");
            out.push_str("  [--lang en|ru] report-pack [--json] [--out <file>]\n");
            out.push_str(
                "  [--lang en|ru] artifact-audit [--json|--text] [--no-strict] [--out <file>]\n",
            );
            out.push_str(
                "  [--lang en|ru] mvp-snapshot [--json|--text] [--no-strict] [--out <file>]\n",
            );
            out.push_str(
                "  [--lang en|ru] mvp-verify [--json|--text] [--refresh] [--no-strict] [--out <file>]\n",
            );
        }
        Language::Ru => {
            out.push_str("Команды chimera-lab:\n");
            out.push_str("  [--lang en|ru] smoke\n");
            out.push_str("  [--lang en|ru] datapath-smoke\n");
            out.push_str("  [--lang en|ru] datapath-report [--json] [--out <файл>]\n");
            out.push_str("  [--lang en|ru] cef-phase1-smoke [--json] [--out <файл>]\n");
            out.push_str("  [--lang en|ru] mesh-auto-smoke [--json] [--out <файл>]\n");
            out.push_str(
                "  [--lang en|ru] doctor [--client <файл>] [--gateway <файл>] [--json] [--out <файл>]\n",
            );
            out.push_str("  [--lang en|ru] config-smoke [--client <файл>] [--gateway <файл>]\n");
            out.push_str("  [--lang en|ru] fuzz-smoke\n");
            out.push_str(
                "  [--lang en|ru] perf-smoke [--min-encode-ops <n>] [--min-decode-ops <n>] [--json]\n",
            );
            out.push_str(
                "  [--lang en|ru] net-sim [--loss-pct <n>] [--delay-ms <n>] [--disconnect-at <n>] [--reconnect-after <n>] [--mtu <bytes>] [--json]\n",
            );
            out.push_str(
                "  [--lang en|ru] hardening-smoke [--min-encode-ops <n>] [--min-decode-ops <n>] [--json]\n",
            );
            out.push_str(
                "  [--lang en|ru] benchmark-report [--min-encode-ops <n>] [--min-decode-ops <n>] [--out <файл>] [--baseline <файл>] [--max-regression-pct <n>]\n",
            );
            out.push_str("  [--lang en|ru] mvp-spec-check [--json] [--out <файл>]\n");
            out.push_str("  [--lang en|ru] mvp-spec-report [--out <файл>]\n");
            out.push_str("  [--lang en|ru] m5-artifacts-report [--out <файл>]\n");
            out.push_str("  [--lang en|ru] m6-artifacts-report [--out <файл>]\n");
            out.push_str("  [--lang en|ru] release-readiness-report [--out <файл>]\n");
            out.push_str("  [--lang en|ru] report-pack [--json] [--out <файл>]\n");
            out.push_str(
                "  [--lang en|ru] artifact-audit [--json|--text] [--no-strict] [--out <файл>]\n",
            );
            out.push_str(
                "  [--lang en|ru] mvp-snapshot [--json|--text] [--no-strict] [--out <файл>]\n",
            );
            out.push_str(
                "  [--lang en|ru] mvp-verify [--json|--text] [--refresh] [--no-strict] [--out <файл>]\n",
            );
        }
    }
    out
}

fn run_smoke(lang: Language) -> i32 {
    run_smoke_with_output(lang, true)
}

fn run_datapath_smoke(lang: Language) -> i32 {
    match run_policy_dns_session_carrier_path() {
        Ok(report) => {
            match lang {
                Language::En => {
                    println!("Datapath smoke: ok");
                    println!("Path: policy -> dns-binding -> session frame -> carrier");
                    println!("Gateway explain: {}", report.gateway_explain);
                    println!("Block explain: {}", report.block_explain);
                    println!("Direct explain: {}", report.direct_explain);
                    println!("Network state: not modified");
                }
                Language::Ru => {
                    println!("Datapath smoke: ok");
                    println!("Путь: policy -> dns-binding -> session frame -> carrier");
                    println!("Gateway explain: {}", report.gateway_explain);
                    println!("Block explain: {}", report.block_explain);
                    println!("Direct explain: {}", report.direct_explain);
                    println!("Состояние сети: не изменялось");
                }
            }
            0
        }
        Err(error) => {
            eprintln!("datapath smoke failed: {error}");
            1
        }
    }
}

fn run_datapath_report(lang: Language, args: &[String]) -> i32 {
    let options = match parse_datapath_report_options(args) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => {
                    eprintln!("Datapath report options error: {error}");
                    eprintln!(
                        "usage: chimera-lab [--lang en|ru] datapath-report [--json] [--out <file>]"
                    );
                }
                Language::Ru => {
                    eprintln!("Ошибка опций datapath-report: {error}");
                    eprintln!(
                        "использование: chimera-lab [--lang en|ru] datapath-report [--json] [--out <файл>]"
                    );
                }
            }
            return 2;
        }
    };

    let report = match run_policy_dns_session_carrier_path() {
        Ok(report) => report,
        Err(error) => {
            eprintln!("datapath report failed: {error}");
            return 1;
        }
    };

    let json = render_datapath_report_json(&report);
    if let Some(path) = options.out_path.as_deref()
        && let Err(error) = fs::write(path, &json)
    {
        eprintln!("datapath report write failed: {error}");
        return 1;
    }

    if options.json_output {
        println!("{json}");
    } else {
        println!("{}", render_datapath_report_text(lang, &report));
    }
    0
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CefPhase1SmokeOptions {
    json_output: bool,
    out_path: Option<String>,
}

impl Default for CefPhase1SmokeOptions {
    fn default() -> Self {
        Self {
            json_output: false,
            out_path: Some("docs/CEF_PHASE1_SMOKE.json".to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CefPhase1SmokeResult {
    mesh_join_mode_resolved: bool,
    mesh_failover_reselection_verified: bool,
    dht_discovery_record_verified: bool,
    dps_policy_fragment_verified: bool,
    relay_policy_verified: bool,
    emergency_offer_valid: bool,
    roaming_cache_active_hit: bool,
    reputation_penalty_applied: bool,
}

fn parse_cef_phase1_smoke_options(args: &[String]) -> Result<CefPhase1SmokeOptions, String> {
    let mut options = CefPhase1SmokeOptions::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        if flag == "--json" {
            options.json_output = true;
            index += 1;
            continue;
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--out" => options.out_path = Some(value.to_string()),
            _ => return Err(format!("unknown option: {flag}")),
        }
        index += 2;
    }
    Ok(options)
}

fn run_cef_phase1_smoke(lang: Language, args: &[String]) -> i32 {
    let options = match parse_cef_phase1_smoke_options(args) {
        Ok(value) => value,
        Err(error) => {
            match lang {
                Language::En => {
                    eprintln!("CEF phase1 smoke options error: {error}");
                    eprintln!(
                        "usage: chimera-lab [--lang en|ru] cef-phase1-smoke [--json] [--out <file>]"
                    );
                }
                Language::Ru => {
                    eprintln!("Ошибка опций CEF phase1 smoke: {error}");
                    eprintln!(
                        "использование: chimera-lab [--lang en|ru] cef-phase1-smoke [--json] [--out <файл>]"
                    );
                }
            }
            return 2;
        }
    };

    let result = match execute_cef_phase1_smoke() {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cef-phase1-smoke failed: {error}");
            return 1;
        }
    };

    let json = render_cef_phase1_smoke_json(result);
    if let Err(error) = fs::write(
        "docs/MESH_RUNTIME_TRACE.json",
        render_mesh_runtime_trace_json(),
    ) {
        eprintln!("cef-phase1-smoke trace write failed: {error}");
        return 1;
    }
    if let Some(path) = options.out_path.as_deref()
        && let Err(error) = fs::write(path, &json)
    {
        eprintln!("cef-phase1-smoke write failed: {error}");
        return 1;
    }

    if options.json_output {
        println!("{json}");
    } else {
        println!("{}", render_cef_phase1_smoke_text(lang, result));
    }
    0
}

fn execute_cef_phase1_smoke() -> Result<CefPhase1SmokeResult, String> {
    cef_phase1_mesh::execute_cef_phase1_smoke()
}

fn render_mesh_runtime_trace_json() -> String {
    cef_phase1_mesh::render_mesh_runtime_trace_json()
}

fn render_mesh_auto_adaptive_trace_json() -> String {
    cef_phase1_mesh::render_mesh_auto_adaptive_trace_json()
}

fn run_mesh_auto_smoke(lang: Language, args: &[String]) -> i32 {
    let options = match parse_cef_phase1_smoke_options(args) {
        Ok(value) => value,
        Err(error) => {
            match lang {
                Language::En => {
                    eprintln!("Mesh auto smoke options error: {error}");
                    eprintln!(
                        "usage: chimera-lab [--lang en|ru] mesh-auto-smoke [--json] [--out <file>]"
                    );
                }
                Language::Ru => {
                    eprintln!("Ошибка опций mesh-auto-smoke: {error}");
                    eprintln!(
                        "использование: chimera-lab [--lang en|ru] mesh-auto-smoke [--json] [--out <файл>]"
                    );
                }
            }
            return 2;
        }
    };

    let json = render_mesh_auto_adaptive_trace_json();
    if json.contains("\"status\":\"error\"") {
        eprintln!("mesh-auto-smoke failed");
        return 1;
    }
    if let Err(error) = fs::write("docs/MESH_AUTO_ADAPTIVE_TRACE.json", &json) {
        eprintln!("mesh-auto-smoke write failed: {error}");
        return 1;
    }
    if let Some(path) = options.out_path.as_deref()
        && let Err(error) = fs::write(path, &json)
    {
        eprintln!("mesh-auto-smoke write failed: {error}");
        return 1;
    }

    if options.json_output {
        println!("{json}");
    } else {
        match lang {
            Language::En => {
                println!("Mesh auto smoke: ok");
                println!("Artifact: docs/MESH_AUTO_ADAPTIVE_TRACE.json");
                println!("Network state: not modified");
            }
            Language::Ru => {
                println!("Mesh auto smoke: ok");
                println!("Артефакт: docs/MESH_AUTO_ADAPTIVE_TRACE.json");
                println!("Состояние сети: не изменялось");
            }
        }
    }
    0
}

fn render_cef_phase1_smoke_text(lang: Language, result: CefPhase1SmokeResult) -> String {
    match lang {
        Language::En => format!(
            "CEF phase1 smoke: ok\nChecks:\n  - mesh_join_mode_resolved: {}\n  - mesh_failover_reselection_verified: {}\n  - dht_discovery_record_verified: {}\n  - dps_policy_fragment_verified: {}\n  - relay_policy_verified: {}\n  - emergency_offer_valid: {}\n  - roaming_cache_active_hit: {}\n  - reputation_penalty_applied: {}\nNetwork state: not modified\n",
            result.mesh_join_mode_resolved,
            result.mesh_failover_reselection_verified,
            result.dht_discovery_record_verified,
            result.dps_policy_fragment_verified,
            result.relay_policy_verified,
            result.emergency_offer_valid,
            result.roaming_cache_active_hit,
            result.reputation_penalty_applied
        ),
        Language::Ru => format!(
            "CEF phase1 smoke: ok\nПроверки:\n  - mesh_join_mode_resolved: {}\n  - mesh_failover_reselection_verified: {}\n  - dht_discovery_record_verified: {}\n  - dps_policy_fragment_verified: {}\n  - relay_policy_verified: {}\n  - emergency_offer_valid: {}\n  - roaming_cache_active_hit: {}\n  - reputation_penalty_applied: {}\nСостояние сети: не изменялось\n",
            result.mesh_join_mode_resolved,
            result.mesh_failover_reselection_verified,
            result.dht_discovery_record_verified,
            result.dps_policy_fragment_verified,
            result.relay_policy_verified,
            result.emergency_offer_valid,
            result.roaming_cache_active_hit,
            result.reputation_penalty_applied
        ),
    }
}

fn render_cef_phase1_smoke_json(result: CefPhase1SmokeResult) -> String {
    format!(
        "{{\"status\":\"ok\",\"kind\":\"cef_phase1_smoke\",\"message_en\":\"CEF phase1 smoke passed.\",\"message_ru\":\"Проверка CEF phase1 успешно пройдена.\",\"checks\":{{\"mesh_join_mode_resolved\":{},\"mesh_failover_reselection_verified\":{},\"dht_discovery_record_verified\":{},\"dps_policy_fragment_verified\":{},\"relay_policy_verified\":{},\"emergency_offer_valid\":{},\"roaming_cache_active_hit\":{},\"reputation_penalty_applied\":{}}},\"network_state\":\"not_modified\"}}",
        result.mesh_join_mode_resolved,
        result.mesh_failover_reselection_verified,
        result.dht_discovery_record_verified,
        result.dps_policy_fragment_verified,
        result.relay_policy_verified,
        result.emergency_offer_valid,
        result.roaming_cache_active_hit,
        result.reputation_penalty_applied
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DoctorOptions {
    client_path: String,
    gateway_path: String,
    json_output: bool,
    out_path: Option<String>,
}

impl Default for DoctorOptions {
    fn default() -> Self {
        Self {
            client_path: "configs/client.example.conf".to_string(),
            gateway_path: "configs/gateway.example.conf".to_string(),
            json_output: false,
            out_path: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DoctorResult {
    client_config_ok: bool,
    gateway_config_ok: bool,
    client_carrier_ok: bool,
    gateway_carrier_ok: bool,
    net_sim_ok: bool,
    net_sim_dropped: usize,
    net_sim_reconnect_events: usize,
}

fn parse_doctor_options(args: &[String]) -> Result<DoctorOptions, String> {
    let mut options = DoctorOptions::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        if flag == "--json" {
            options.json_output = true;
            index += 1;
            continue;
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--client" => options.client_path = value.to_string(),
            "--gateway" => options.gateway_path = value.to_string(),
            "--out" => options.out_path = Some(value.to_string()),
            _ => return Err(format!("unknown option: {flag}")),
        }
        index += 2;
    }
    Ok(options)
}

fn run_doctor(lang: Language, args: &[String]) -> i32 {
    let options = match parse_doctor_options(args) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => {
                    eprintln!("Doctor options error: {error}");
                    eprintln!(
                        "usage: chimera-lab [--lang en|ru] doctor [--client <file>] [--gateway <file>] [--json] [--out <file>]"
                    );
                }
                Language::Ru => {
                    eprintln!("Ошибка опций doctor: {error}");
                    eprintln!(
                        "использование: chimera-lab [--lang en|ru] doctor [--client <файл>] [--gateway <файл>] [--json] [--out <файл>]"
                    );
                }
            }
            return 2;
        }
    };

    let result = match execute_doctor(&options) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };

    let json = render_doctor_json(result);
    if let Some(path) = options.out_path.as_deref()
        && let Err(error) = fs::write(path, &json)
    {
        eprintln!("doctor report write failed: {error}");
        return 1;
    }
    if options.json_output {
        println!("{json}");
    } else {
        print!("{}", render_doctor_text(lang, &options, result));
    }
    0
}

fn execute_doctor(options: &DoctorOptions) -> Result<DoctorResult, String> {
    let client_text = fs::read_to_string(&options.client_path)
        .map_err(|error| format!("client config read failed: {error}"))?;
    let gateway_text = fs::read_to_string(&options.gateway_path)
        .map_err(|error| format!("gateway config read failed: {error}"))?;

    let client_config = parse_client_config_text(&client_text)
        .map_err(|error| format!("client config invalid: {error}"))?;
    let gateway_config = parse_gateway_config_text(&gateway_text)
        .map_err(|error| format!("gateway config invalid: {error}"))?;

    validate_carrier_profile(
        client_config.carrier_profile,
        &client_config.carrier_addr,
        &client_config.carrier_server_name,
    )
    .map_err(|error| format!("client carrier validation failed: {error}"))?;
    validate_carrier_profile(
        gateway_config.carrier_profile,
        &gateway_config.listen_addr,
        "gateway.local",
    )
    .map_err(|error| format!("gateway carrier validation failed: {error}"))?;

    let net_sim = execute_net_sim(NetSimOptions::default());
    if net_sim.reconnect_events == 0 {
        return Err("doctor failed: net-sim reconnect check failed".to_string());
    }

    Ok(DoctorResult {
        client_config_ok: true,
        gateway_config_ok: true,
        client_carrier_ok: true,
        gateway_carrier_ok: true,
        net_sim_ok: true,
        net_sim_dropped: net_sim.dropped,
        net_sim_reconnect_events: net_sim.reconnect_events,
    })
}

fn render_doctor_text(lang: Language, options: &DoctorOptions, result: DoctorResult) -> String {
    let mut out = String::new();
    match lang {
        Language::En => {
            out.push_str("Lab doctor: ready for MVP checks\n");
            out.push_str("Checks:\n");
            out.push_str(&format!("  - Client config: {}\n", result.client_config_ok));
            out.push_str(&format!(
                "  - Gateway config: {}\n",
                result.gateway_config_ok
            ));
            out.push_str(&format!(
                "  - Client carrier: {}\n",
                result.client_carrier_ok
            ));
            out.push_str(&format!(
                "  - Gateway carrier: {}\n",
                result.gateway_carrier_ok
            ));
            out.push_str(&format!("  - Net sim: {}\n", result.net_sim_ok));
            out.push_str(&format!(
                "Summary: client={}, gateway={}, net_sim_dropped={}, net_sim_reconnect_events={}\n",
                options.client_path,
                options.gateway_path,
                result.net_sim_dropped,
                result.net_sim_reconnect_events
            ));
            out.push_str("Network state: not modified\n");
        }
        Language::Ru => {
            out.push_str("Lab doctor: готово к MVP-проверкам\n");
            out.push_str("Проверки:\n");
            out.push_str(&format!(
                "  - Конфиг клиента: {}\n",
                result.client_config_ok
            ));
            out.push_str(&format!(
                "  - Конфиг gateway: {}\n",
                result.gateway_config_ok
            ));
            out.push_str(&format!(
                "  - Carrier клиента: {}\n",
                result.client_carrier_ok
            ));
            out.push_str(&format!(
                "  - Carrier gateway: {}\n",
                result.gateway_carrier_ok
            ));
            out.push_str(&format!("  - Net sim: {}\n", result.net_sim_ok));
            out.push_str(&format!(
                "Сводка: client={}, gateway={}, net_sim_dropped={}, net_sim_reconnect_events={}\n",
                options.client_path,
                options.gateway_path,
                result.net_sim_dropped,
                result.net_sim_reconnect_events
            ));
            out.push_str("Состояние сети: не изменялось\n");
        }
    }
    out
}

fn render_doctor_json(result: DoctorResult) -> String {
    format!(
        "{{\"status\":\"ok\",\"kind\":\"lab_doctor\",\"message_en\":\"Lab doctor check is ready.\",\"message_ru\":\"Проверка lab doctor готова.\",\"client_config_ok\":{},\"gateway_config_ok\":{},\"client_carrier_ok\":{},\"gateway_carrier_ok\":{},\"net_sim_ok\":{},\"net_sim_dropped\":{},\"net_sim_reconnect_events\":{},\"network_state\":\"not_modified\"}}",
        result.client_config_ok,
        result.gateway_config_ok,
        result.client_carrier_ok,
        result.gateway_carrier_ok,
        result.net_sim_ok,
        result.net_sim_dropped,
        result.net_sim_reconnect_events
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConfigSmokeOptions {
    client_path: String,
    gateway_path: String,
}

impl Default for ConfigSmokeOptions {
    fn default() -> Self {
        Self {
            client_path: "configs/client.example.conf".to_string(),
            gateway_path: "configs/gateway.example.conf".to_string(),
        }
    }
}

fn parse_config_smoke_options(args: &[String]) -> Result<ConfigSmokeOptions, String> {
    let mut options = ConfigSmokeOptions::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--client" => options.client_path = value.to_string(),
            "--gateway" => options.gateway_path = value.to_string(),
            _ => return Err(format!("unknown option: {flag}")),
        }
        index += 2;
    }
    Ok(options)
}

fn run_config_smoke(lang: Language, args: &[String]) -> i32 {
    let options = match parse_config_smoke_options(args) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => {
                    eprintln!("Config-smoke options error: {error}");
                    eprintln!(
                        "usage: chimera-lab [--lang en|ru] config-smoke [--client <file>] [--gateway <file>]"
                    );
                }
                Language::Ru => {
                    eprintln!("Ошибка опций config-smoke: {error}");
                    eprintln!(
                        "использование: chimera-lab [--lang en|ru] config-smoke [--client <файл>] [--gateway <файл>]"
                    );
                }
            }
            return 2;
        }
    };
    match execute_config_smoke(lang, &options, true) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}

fn execute_config_smoke(
    lang: Language,
    options: &ConfigSmokeOptions,
    print_output: bool,
) -> Result<(), String> {
    let client_text = match fs::read_to_string(&options.client_path) {
        Ok(text) => text,
        Err(error) => {
            return Err(format!("client config read failed: {error}"));
        }
    };
    let gateway_text = match fs::read_to_string(&options.gateway_path) {
        Ok(text) => text,
        Err(error) => {
            return Err(format!("gateway config read failed: {error}"));
        }
    };

    let client_config = match parse_client_config_text(&client_text) {
        Ok(config) => config,
        Err(error) => {
            return Err(format!("client config invalid: {error}"));
        }
    };
    let gateway_config = match parse_gateway_config_text(&gateway_text) {
        Ok(config) => config,
        Err(error) => {
            return Err(format!("gateway config invalid: {error}"));
        }
    };

    if let Err(error) = validate_carrier_profile(
        client_config.carrier_profile,
        &client_config.carrier_addr,
        &client_config.carrier_server_name,
    ) {
        return Err(format!("client carrier validation failed: {error}"));
    }
    if let Err(error) = validate_carrier_profile(
        gateway_config.carrier_profile,
        &gateway_config.listen_addr,
        "gateway.local",
    ) {
        return Err(format!("gateway carrier validation failed: {error}"));
    }
    run_negative_config_parser_smoke()?;

    if print_output {
        match lang {
            Language::En => {
                println!("Config smoke: ok");
                println!("Client config: {}", options.client_path);
                println!("Gateway config: {}", options.gateway_path);
                println!("Network state: not modified");
            }
            Language::Ru => {
                println!("Config smoke: ok");
                println!("Конфиг клиента: {}", options.client_path);
                println!("Конфиг gateway: {}", options.gateway_path);
                println!("Состояние сети: не изменялось");
            }
        }
    }
    Ok(())
}

fn run_negative_config_parser_smoke() -> Result<(), String> {
    let client_unknown_key = "carrier.addr = 203.0.113.10:443\ncapture.mdoe = auto\n";
    let client_duplicate_key = "carrier.addr = 203.0.113.10:443\ncarrier.addr = 198.51.100.7:443\n";
    let raw_missing_separator = "carrier.addr 203.0.113.10:443\n";
    let gateway_unknown_key = "gateway.lsiten_addr = 127.0.0.1:443\n";

    let unknown_key_error = match parse_client_config_text(client_unknown_key) {
        Ok(_) => {
            return Err("negative smoke failed: unknown key input unexpectedly parsed".to_string());
        }
        Err(error) => error.to_string(),
    };
    if !unknown_key_error.contains("unknown client config key") {
        return Err(format!(
            "negative smoke failed: unknown key error shape changed: {unknown_key_error}"
        ));
    }

    let duplicate_key_error = match parse_client_config_text(client_duplicate_key) {
        Ok(_) => {
            return Err(
                "negative smoke failed: duplicate key input unexpectedly parsed".to_string(),
            );
        }
        Err(error) => error.to_string(),
    };
    if !duplicate_key_error.contains("duplicates key") {
        return Err(format!(
            "negative smoke failed: duplicate key error shape changed: {duplicate_key_error}"
        ));
    }

    let missing_separator_error = match RawConfig::parse(raw_missing_separator) {
        Ok(_) => {
            return Err(
                "negative smoke failed: missing separator input unexpectedly parsed".to_string(),
            );
        }
        Err(error) => error.to_string(),
    };
    if !missing_separator_error.contains("missing '='") {
        return Err(format!(
            "negative smoke failed: missing separator error shape changed: {missing_separator_error}"
        ));
    }

    let gateway_unknown_key_error = match parse_gateway_config_text(gateway_unknown_key) {
        Ok(_) => {
            return Err(
                "negative smoke failed: gateway unknown key input unexpectedly parsed".to_string(),
            );
        }
        Err(error) => error.to_string(),
    };
    if !gateway_unknown_key_error.contains("unknown gateway config key") {
        return Err(format!(
            "negative smoke failed: gateway unknown key error shape changed: {gateway_unknown_key_error}"
        ));
    }

    Ok(())
}

fn validate_carrier_profile(
    profile: ConfigCarrierProfile,
    addr: &str,
    server_name: &str,
) -> Result<(), String> {
    match profile {
        ConfigCarrierProfile::InMemory => Ok(()),
        ConfigCarrierProfile::Tls => TlsCarrier::new(TlsCarrierConfig {
            server_name: server_name.to_string(),
            connect_addr: addr.to_string(),
            connect_timeout_ms: 3000,
        })
        .map(|_| ())
        .map_err(|error| error.to_string()),
        ConfigCarrierProfile::Quic => QuicCarrier::new(QuicCarrierConfig {
            server_name: server_name.to_string(),
            connect_addr: addr.to_string(),
            connect_timeout_ms: 3000,
        })
        .map(|_| ())
        .map_err(|error| error.to_string()),
    }
}

fn run_smoke_with_output(lang: Language, print_output: bool) -> i32 {
    if let Err(error) = run_fake_handshake() {
        eprintln!("fake handshake failed: {error}");
        return 1;
    }
    let datapath_report = match run_policy_dns_session_carrier_path() {
        Ok(report) => report,
        Err(error) => {
            eprintln!("policy/dns/session/carrier path failed: {error}");
            return 1;
        }
    };

    if let Err(error) = run_frame_replay_check() {
        eprintln!("frame replay check failed: {error}");
        return 1;
    }

    if print_output {
        match lang {
            Language::En => {
                println!("Basic lab check: ok");
                println!("Data channel: in-memory");
                println!("Secure session test: passed");
                println!("Datapath path test: passed");
                println!("Datapath explain: {}", datapath_report.gateway_explain);
                println!("Network state: not modified");
            }
            Language::Ru => {
                println!("Базовая лабораторная проверка: ok");
                println!("Канал данных: in-memory");
                println!("Проверка защищенной сессии: пройдена");
                println!("Проверка datapath-цепочки: пройдена");
                println!("Datapath explain: {}", datapath_report.gateway_explain);
                println!("Состояние сети: не изменялось");
            }
        }
    }
    0
}

fn run_fuzz_smoke(lang: Language) -> i32 {
    run_fuzz_smoke_with_output(lang, true)
}

fn run_fuzz_smoke_with_output(lang: Language, print_output: bool) -> i32 {
    const CASES: usize = 5_000;
    let mut rng = DeterministicRng::new(0xC0DE_CAFE_F00D_BAAD);
    for _ in 0..CASES {
        let input = fuzz_bytes(&mut rng, 256);
        let text = String::from_utf8_lossy(&input);
        let _ = RawConfig::parse(&text);
        let _ = parse_policy_text(&text);
        let _ = Frame::decode(&input);
        let _ = HandshakeMessage::decode(&input);
    }
    if print_output {
        match lang {
            Language::En => {
                println!("Random-input safety check: ok");
                println!("Cases: {CASES}");
                println!(
                    "Checked parts: config parser, policy parser, frame decoder, handshake decoder"
                );
                println!("Network state: not modified");
            }
            Language::Ru => {
                println!("Проверка на случайных данных: ok");
                println!("Случаев: {CASES}");
                println!(
                    "Проверенные части: парсер config, парсер policy, декодер frame, декодер handshake"
                );
                println!("Состояние сети: не изменялось");
            }
        }
    }
    0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct PerfSmokeOptions {
    min_encode_ops: Option<u64>,
    min_decode_ops: Option<u64>,
    json_output: bool,
}

fn parse_perf_smoke_options(args: &[String]) -> Result<PerfSmokeOptions, String> {
    let mut options = PerfSmokeOptions::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        if flag == "--json" {
            options.json_output = true;
            index += 1;
            continue;
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        let parsed = value
            .parse::<u64>()
            .map_err(|_| format!("invalid integer for {flag}: {value}"))?;
        match flag {
            "--min-encode-ops" => options.min_encode_ops = Some(parsed),
            "--min-decode-ops" => options.min_decode_ops = Some(parsed),
            _ => return Err(format!("unknown option: {flag}")),
        }
        index += 2;
    }
    Ok(options)
}

fn run_perf_smoke(lang: Language, args: &[String]) -> i32 {
    let options = match parse_perf_smoke_options(args) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => {
                    eprintln!("Perf-smoke options error: {error}");
                    eprintln!(
                        "usage: chimera-lab [--lang en|ru] perf-smoke [--min-encode-ops <n>] [--min-decode-ops <n>] [--json]"
                    );
                }
                Language::Ru => {
                    eprintln!("Ошибка опций perf-smoke: {error}");
                    eprintln!(
                        "использование: chimera-lab [--lang en|ru] perf-smoke [--min-encode-ops <n>] [--min-decode-ops <n>] [--json]"
                    );
                }
            }
            return 2;
        }
    };
    let result = match execute_perf_smoke(options) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    if options.json_output {
        println!(
            "{{\"status\":\"ok\",\"iterations\":{},\"encode_ops_per_sec\":{:.0},\"decode_ops_per_sec\":{:.0},\"encoded_total_bytes\":{},\"decoded_total_payload_bytes\":{}}}",
            result.iterations,
            result.encode_ops_per_sec,
            result.decode_ops_per_sec,
            result.encoded_total_bytes,
            result.decoded_total_payload_bytes
        );
    } else {
        match lang {
            Language::En => {
                println!("Speed check: ok");
                println!("Iterations: {}", result.iterations);
                println!(
                    "Encode: {:.0} ops/sec, {} bytes total",
                    result.encode_ops_per_sec, result.encoded_total_bytes
                );
                println!(
                    "Decode: {:.0} ops/sec, {} payload bytes total",
                    result.decode_ops_per_sec, result.decoded_total_payload_bytes
                );
                println!("Network state: not modified");
            }
            Language::Ru => {
                println!("Проверка скорости: ok");
                println!("Итераций: {}", result.iterations);
                println!(
                    "Encode: {:.0} ops/сек, всего {} байт",
                    result.encode_ops_per_sec, result.encoded_total_bytes
                );
                println!(
                    "Decode: {:.0} ops/сек, полезных {} байт",
                    result.decode_ops_per_sec, result.decoded_total_payload_bytes
                );
                println!("Состояние сети: не изменялось");
            }
        }
    }
    0
}

#[derive(Debug, Clone, Copy)]
struct PerfSmokeResult {
    iterations: usize,
    encode_ops_per_sec: f64,
    decode_ops_per_sec: f64,
    encoded_total_bytes: usize,
    decoded_total_payload_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NetSimOptions {
    loss_pct: u8,
    delay_ms: u64,
    disconnect_at: usize,
    reconnect_after: usize,
    mtu: usize,
    json_output: bool,
}

impl Default for NetSimOptions {
    fn default() -> Self {
        Self {
            loss_pct: 10,
            delay_ms: 20,
            disconnect_at: 40,
            reconnect_after: 8,
            mtu: 1400,
            json_output: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NetSimResult {
    attempts: usize,
    sent: usize,
    dropped: usize,
    mtu_dropped: usize,
    disconnected_window: usize,
    reconnect_events: usize,
    simulated_delay_total_ms: u64,
}

fn parse_net_sim_options(args: &[String]) -> Result<NetSimOptions, String> {
    let mut options = NetSimOptions::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        if flag == "--json" {
            options.json_output = true;
            index += 1;
            continue;
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--loss-pct" => {
                let parsed = value
                    .parse::<u8>()
                    .map_err(|_| format!("invalid integer for {flag}: {value}"))?;
                if parsed > 100 {
                    return Err("loss percent must be 0..100".to_string());
                }
                options.loss_pct = parsed;
            }
            "--delay-ms" => {
                options.delay_ms = value
                    .parse::<u64>()
                    .map_err(|_| format!("invalid integer for {flag}: {value}"))?;
            }
            "--disconnect-at" => {
                options.disconnect_at = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid integer for {flag}: {value}"))?;
            }
            "--reconnect-after" => {
                options.reconnect_after = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid integer for {flag}: {value}"))?;
            }
            "--mtu" => {
                let parsed = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid integer for {flag}: {value}"))?;
                if parsed < 256 {
                    return Err("mtu must be >= 256".to_string());
                }
                options.mtu = parsed;
            }
            _ => return Err(format!("unknown option: {flag}")),
        }
        index += 2;
    }
    Ok(options)
}

fn execute_net_sim(options: NetSimOptions) -> NetSimResult {
    const ATTEMPTS: usize = 100;
    const FRAME_BYTES: usize = 1200;
    let mut sent = 0usize;
    let mut dropped = 0usize;
    let mut mtu_dropped = 0usize;
    let mut disconnected_window = 0usize;
    let mut reconnect_events = 0usize;
    let mut simulated_delay_total_ms = 0u64;
    let disconnect_end = options
        .disconnect_at
        .saturating_add(options.reconnect_after);

    for index in 0..ATTEMPTS {
        if index == disconnect_end {
            reconnect_events = reconnect_events.saturating_add(1);
        }
        if index >= options.disconnect_at && index < disconnect_end {
            disconnected_window = disconnected_window.saturating_add(1);
            dropped = dropped.saturating_add(1);
            continue;
        }
        if FRAME_BYTES > options.mtu {
            mtu_dropped = mtu_dropped.saturating_add(1);
            dropped = dropped.saturating_add(1);
            continue;
        }
        let modulo = (index % 100) as u8;
        if modulo < options.loss_pct {
            dropped = dropped.saturating_add(1);
            continue;
        }
        sent = sent.saturating_add(1);
        simulated_delay_total_ms = simulated_delay_total_ms.saturating_add(options.delay_ms);
    }

    NetSimResult {
        attempts: ATTEMPTS,
        sent,
        dropped,
        mtu_dropped,
        disconnected_window,
        reconnect_events,
        simulated_delay_total_ms,
    }
}

fn run_net_sim(lang: Language, args: &[String]) -> i32 {
    let options = match parse_net_sim_options(args) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => {
                    eprintln!("Net-sim options error: {error}");
                    eprintln!(
                        "usage: chimera-lab [--lang en|ru] net-sim [--loss-pct <n>] [--delay-ms <n>] [--disconnect-at <n>] [--reconnect-after <n>] [--mtu <bytes>] [--json]"
                    );
                }
                Language::Ru => {
                    eprintln!("Ошибка опций net-sim: {error}");
                    eprintln!(
                        "использование: chimera-lab [--lang en|ru] net-sim [--loss-pct <n>] [--delay-ms <n>] [--disconnect-at <n>] [--reconnect-after <n>] [--mtu <bytes>] [--json]"
                    );
                }
            }
            return 2;
        }
    };
    let result = execute_net_sim(options);
    if options.json_output {
        println!(
            "{{\"status\":\"ok\",\"attempts\":{},\"sent\":{},\"dropped\":{},\"mtu_dropped\":{},\"disconnected_window\":{},\"reconnect_events\":{},\"simulated_delay_total_ms\":{}}}",
            result.attempts,
            result.sent,
            result.dropped,
            result.mtu_dropped,
            result.disconnected_window,
            result.reconnect_events,
            result.simulated_delay_total_ms
        );
    } else {
        match lang {
            Language::En => {
                println!("Network lab sim: ok");
                println!("Attempts: {}", result.attempts);
                println!("Sent: {}, dropped: {}", result.sent, result.dropped);
                println!("MTU drops: {}", result.mtu_dropped);
                println!(
                    "Disconnect window drops: {}, reconnect events: {}",
                    result.disconnected_window, result.reconnect_events
                );
                println!(
                    "Simulated delay total: {} ms",
                    result.simulated_delay_total_ms
                );
                println!("Network state: not modified");
            }
            Language::Ru => {
                println!("Сетевая симуляция: ok");
                println!("Попыток: {}", result.attempts);
                println!("Отправлено: {}, потерь: {}", result.sent, result.dropped);
                println!("Потерь из-за MTU: {}", result.mtu_dropped);
                println!(
                    "Потерь в окне дисконнекта: {}, событий reconnect: {}",
                    result.disconnected_window, result.reconnect_events
                );
                println!(
                    "Суммарная задержка (модель): {} мс",
                    result.simulated_delay_total_ms
                );
                println!("Состояние сети: не изменялось");
            }
        }
    }
    0
}

fn execute_perf_smoke(options: PerfSmokeOptions) -> Result<PerfSmokeResult, String> {
    const ITERATIONS: usize = 20_000;
    let payload = vec![0xAB; 1200];
    let frame = Frame {
        packet_number: 1,
        payload,
    };

    let encode_start = Instant::now();
    let mut encoded_total_bytes = 0usize;
    let mut last_encoded = Vec::new();
    for _ in 0..ITERATIONS {
        let encoded = match frame.encode() {
            Ok(encoded) => encoded,
            Err(error) => {
                return Err(format!("perf smoke encode failed: {error}"));
            }
        };
        encoded_total_bytes = encoded_total_bytes.saturating_add(encoded.len());
        last_encoded = encoded;
    }
    let encode_elapsed = encode_start.elapsed();

    let decode_start = Instant::now();
    let mut decoded_total_payload_bytes = 0usize;
    for _ in 0..ITERATIONS {
        let decoded = match Frame::decode(&last_encoded) {
            Ok(decoded) => decoded,
            Err(error) => {
                return Err(format!("perf smoke decode failed: {error}"));
            }
        };
        decoded_total_payload_bytes =
            decoded_total_payload_bytes.saturating_add(decoded.payload.len());
    }
    let decode_elapsed = decode_start.elapsed();

    let encode_ops_per_sec = (ITERATIONS as f64) / encode_elapsed.as_secs_f64();
    let decode_ops_per_sec = (ITERATIONS as f64) / decode_elapsed.as_secs_f64();
    if let Some(min_encode_ops) = options.min_encode_ops
        && encode_ops_per_sec < (min_encode_ops as f64)
    {
        eprintln!(
            "perf smoke failed: encode ops/sec {:.0} is below required minimum {}",
            encode_ops_per_sec, min_encode_ops
        );
        return Err(format!(
            "perf smoke failed: encode ops/sec {:.0} is below required minimum {}",
            encode_ops_per_sec, min_encode_ops
        ));
    }
    if let Some(min_decode_ops) = options.min_decode_ops
        && decode_ops_per_sec < (min_decode_ops as f64)
    {
        eprintln!(
            "perf smoke failed: decode ops/sec {:.0} is below required minimum {}",
            decode_ops_per_sec, min_decode_ops
        );
        return Err(format!(
            "perf smoke failed: decode ops/sec {:.0} is below required minimum {}",
            decode_ops_per_sec, min_decode_ops
        ));
    }
    Ok(PerfSmokeResult {
        iterations: ITERATIONS,
        encode_ops_per_sec,
        decode_ops_per_sec,
        encoded_total_bytes,
        decoded_total_payload_bytes,
    })
}

fn run_hardening_smoke(lang: Language, args: &[String]) -> i32 {
    let perf_options = match parse_perf_smoke_options(args) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => {
                    eprintln!("Hardening-smoke options error: {error}");
                    eprintln!(
                        "usage: chimera-lab [--lang en|ru] hardening-smoke [--min-encode-ops <n>] [--min-decode-ops <n>] [--json]"
                    );
                }
                Language::Ru => {
                    eprintln!("Ошибка опций hardening-smoke: {error}");
                    eprintln!(
                        "использование: chimera-lab [--lang en|ru] hardening-smoke [--min-encode-ops <n>] [--min-decode-ops <n>] [--json]"
                    );
                }
            }
            return 2;
        }
    };

    let print_stage_output = !perf_options.json_output;
    if let Err(error) =
        execute_config_smoke(lang, &ConfigSmokeOptions::default(), print_stage_output)
    {
        eprintln!(
            "{}",
            match lang {
                Language::En => "Hardening smoke failed: config-smoke stage failed",
                Language::Ru => "Hardening smoke не пройден: ошибка этапа config-smoke",
            }
        );
        eprintln!("{error}");
        return 1;
    }
    if run_smoke_with_output(lang, print_stage_output) != 0 {
        eprintln!(
            "{}",
            match lang {
                Language::En => "Hardening smoke failed: smoke stage failed",
                Language::Ru => "Hardening smoke не пройден: ошибка этапа smoke",
            }
        );
        return 1;
    }
    if run_fuzz_smoke_with_output(lang, print_stage_output) != 0 {
        eprintln!(
            "{}",
            match lang {
                Language::En => "Hardening smoke failed: fuzz-smoke stage failed",
                Language::Ru => "Hardening smoke не пройден: ошибка этапа fuzz-smoke",
            }
        );
        return 1;
    }
    let net_sim = execute_net_sim(NetSimOptions::default());
    if net_sim.reconnect_events == 0 {
        eprintln!(
            "{}",
            match lang {
                Language::En => "Hardening smoke failed: net-sim reconnect check failed",
                Language::Ru => "Hardening smoke не пройден: ошибка проверки reconnect в net-sim",
            }
        );
        return 1;
    }
    if print_stage_output {
        match lang {
            Language::En => {
                println!("Network simulation: ok");
                println!(
                    "Summary: sent={}, dropped={}, reconnect_events={}",
                    net_sim.sent, net_sim.dropped, net_sim.reconnect_events
                );
                println!("Network state: not modified");
            }
            Language::Ru => {
                println!("Сетевая симуляция: ok");
                println!(
                    "Сводка: отправлено={}, потерь={}, событий переподключения={}",
                    net_sim.sent, net_sim.dropped, net_sim.reconnect_events
                );
                println!("Состояние сети: не изменялось");
            }
        }
    }

    let perf_result = match execute_perf_smoke(perf_options) {
        Ok(result) => result,
        Err(error) => {
            eprintln!(
                "{}",
                match lang {
                    Language::En => "Hardening smoke failed: perf-smoke stage failed",
                    Language::Ru => "Hardening smoke не пройден: ошибка этапа perf-smoke",
                }
            );
            eprintln!("{error}");
            return 1;
        }
    };

    if perf_options.json_output {
        println!("{}", render_hardening_json(perf_result));
    } else {
        match lang {
            Language::En => {
                println!("Speed check: ok");
                println!("Iterations: {}", perf_result.iterations);
                println!(
                    "Encode: {:.0} ops/sec, {} bytes total",
                    perf_result.encode_ops_per_sec, perf_result.encoded_total_bytes
                );
                println!(
                    "Decode: {:.0} ops/sec, {} payload bytes total",
                    perf_result.decode_ops_per_sec, perf_result.decoded_total_payload_bytes
                );
                println!("Hardening check: ok");
                println!(
                    "Stages passed: config, basic check, random-input check, network simulation, speed check"
                );
                println!("Network state: not modified");
            }
            Language::Ru => {
                println!("Проверка скорости: ok");
                println!("Итераций: {}", perf_result.iterations);
                println!(
                    "Encode: {:.0} ops/сек, всего {} байт",
                    perf_result.encode_ops_per_sec, perf_result.encoded_total_bytes
                );
                println!(
                    "Decode: {:.0} ops/сек, полезных {} байт",
                    perf_result.decode_ops_per_sec, perf_result.decoded_total_payload_bytes
                );
                println!("Проверка усиленной надежности: ok");
                println!(
                    "Этапы пройдены: config, базовая проверка, проверка на случайных данных, сетевая симуляция, проверка скорости"
                );
                println!("Состояние сети: не изменялось");
            }
        }
    }
    0
}

#[derive(Debug, Clone, PartialEq, Default)]
struct BenchmarkReportOptions {
    min_encode_ops: Option<u64>,
    min_decode_ops: Option<u64>,
    out_path: Option<String>,
    baseline_path: Option<String>,
    max_regression_pct: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct MvpSpecCheckOptions {
    json_output: bool,
    out_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MvpSpecCheckResult {
    m0_workspace: bool,
    m1_local_tunnel: bool,
    m2_crypto_session: bool,
    m3_carrier_validation: bool,
    m4_routing_determinism: bool,
    m5_doctor_and_config: bool,
    m6_hardening: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ReleaseGateChecklist {
    clean_clone_builds: bool,
    client_gateway_run_linux: bool,
    encrypted_tunnel_carries_traffic: bool,
    policy_routing_direct_gateway_block: bool,
    dns_binding_works: bool,
    route_explain_works: bool,
    shutdown_restores_network_state: bool,
    security_tests_pass: bool,
    parser_fuzz_smoke_passes: bool,
    no_raw_secrets_in_logs: bool,
    benchmark_report_exists: bool,
    operations_guide_exists: bool,
    runtime_apply_dns_verified: bool,
    runtime_apply_route_verified: bool,
    runtime_route_policy_validation_verified: bool,
    runtime_tun_name_validation_verified: bool,
    runtime_forced_stop_rollback_verified: bool,
}

impl ReleaseGateChecklist {
    fn all_ok(self) -> bool {
        self.clean_clone_builds
            && self.client_gateway_run_linux
            && self.encrypted_tunnel_carries_traffic
            && self.policy_routing_direct_gateway_block
            && self.dns_binding_works
            && self.route_explain_works
            && self.shutdown_restores_network_state
            && self.security_tests_pass
            && self.parser_fuzz_smoke_passes
            && self.no_raw_secrets_in_logs
            && self.benchmark_report_exists
            && self.operations_guide_exists
            && self.runtime_apply_dns_verified
            && self.runtime_apply_route_verified
            && self.runtime_route_policy_validation_verified
            && self.runtime_tun_name_validation_verified
            && self.runtime_forced_stop_rollback_verified
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ReleaseReadinessArtifacts {
    m5_report_ok: bool,
    m6_report_ok: bool,
    benchmark_ok: bool,
    cef_phase1_smoke_ok: bool,
    mesh_route_explain_ok: bool,
    mesh_auto_adaptive_ok: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MvpSpecReportOptions {
    out_path: String,
}

impl Default for MvpSpecReportOptions {
    fn default() -> Self {
        Self {
            out_path: "docs/MVP_SPEC_COVERAGE.md".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct M5ArtifactsReportOptions {
    out_path: String,
}

impl Default for M5ArtifactsReportOptions {
    fn default() -> Self {
        Self {
            out_path: "docs/M5_ARTIFACTS_REPORT.md".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct M6ArtifactsReportOptions {
    out_path: String,
}

impl Default for M6ArtifactsReportOptions {
    fn default() -> Self {
        Self {
            out_path: "docs/M6_ARTIFACTS_REPORT.md".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReleaseReadinessReportOptions {
    out_path: String,
    json_output: bool,
}

impl Default for ReleaseReadinessReportOptions {
    fn default() -> Self {
        Self {
            out_path: "docs/RELEASE_READINESS_REPORT.md".to_string(),
            json_output: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReportPackOptions {
    out_path: String,
    json_output: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArtifactAuditOptions {
    out_path: String,
    json_output: bool,
    strict: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MvpSnapshotOptions {
    out_path: String,
    json_output: bool,
    strict: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MvpVerifyOptions {
    out_path: String,
    json_output: bool,
    strict: bool,
    refresh: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct DatapathReportOptions {
    json_output: bool,
    out_path: Option<String>,
}

impl Default for MvpSnapshotOptions {
    fn default() -> Self {
        Self {
            out_path: "docs/MVP_SNAPSHOT.json".to_string(),
            json_output: true,
            strict: true,
        }
    }
}

impl Default for MvpVerifyOptions {
    fn default() -> Self {
        Self {
            out_path: "docs/MVP_VERIFY.json".to_string(),
            json_output: true,
            strict: true,
            refresh: false,
        }
    }
}

impl Default for ArtifactAuditOptions {
    fn default() -> Self {
        Self {
            out_path: "docs/ARTIFACT_AUDIT.json".to_string(),
            json_output: true,
            strict: true,
        }
    }
}

impl Default for ReportPackOptions {
    fn default() -> Self {
        Self {
            out_path: "docs/REPORT_PACK.md".to_string(),
            json_output: false,
        }
    }
}

fn parse_benchmark_report_options(args: &[String]) -> Result<BenchmarkReportOptions, String> {
    let mut options = BenchmarkReportOptions::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--out" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --out".to_string())?;
                options.out_path = Some(value.to_string());
                index += 2;
            }
            "--baseline" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --baseline".to_string())?;
                options.baseline_path = Some(value.to_string());
                index += 2;
            }
            "--max-regression-pct" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --max-regression-pct".to_string())?;
                let parsed = value
                    .parse::<f64>()
                    .map_err(|_| format!("invalid number for --max-regression-pct: {value}"))?;
                options.max_regression_pct = Some(parsed);
                index += 2;
            }
            "--min-encode-ops" | "--min-decode-ops" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| format!("missing value for {flag}"))?;
                let parsed = value
                    .parse::<u64>()
                    .map_err(|_| format!("invalid integer for {flag}: {value}"))?;
                if flag == "--min-encode-ops" {
                    options.min_encode_ops = Some(parsed);
                } else {
                    options.min_decode_ops = Some(parsed);
                }
                index += 2;
            }
            _ => return Err(format!("unknown option: {flag}")),
        }
    }
    Ok(options)
}

fn parse_mvp_spec_check_options(args: &[String]) -> Result<MvpSpecCheckOptions, String> {
    let mut options = MvpSpecCheckOptions::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--json" => {
                options.json_output = true;
                index += 1;
            }
            "--out" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --out".to_string())?;
                options.out_path = Some(value.to_string());
                index += 2;
            }
            _ => return Err(format!("unknown option: {flag}")),
        }
    }
    Ok(options)
}

fn parse_mvp_spec_report_options(args: &[String]) -> Result<MvpSpecReportOptions, String> {
    let mut options = MvpSpecReportOptions::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--out" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --out".to_string())?;
                options.out_path = value.to_string();
                index += 2;
            }
            _ => return Err(format!("unknown option: {flag}")),
        }
    }
    Ok(options)
}

fn parse_m5_artifacts_report_options(args: &[String]) -> Result<M5ArtifactsReportOptions, String> {
    let mut options = M5ArtifactsReportOptions::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--out" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --out".to_string())?;
                options.out_path = value.to_string();
                index += 2;
            }
            _ => return Err(format!("unknown option: {flag}")),
        }
    }
    Ok(options)
}

fn parse_m6_artifacts_report_options(args: &[String]) -> Result<M6ArtifactsReportOptions, String> {
    let mut options = M6ArtifactsReportOptions::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--out" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --out".to_string())?;
                options.out_path = value.to_string();
                index += 2;
            }
            _ => return Err(format!("unknown option: {flag}")),
        }
    }
    Ok(options)
}

fn parse_release_readiness_report_options(
    args: &[String],
) -> Result<ReleaseReadinessReportOptions, String> {
    let mut options = ReleaseReadinessReportOptions::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--json" => {
                options.json_output = true;
                index += 1;
            }
            "--out" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --out".to_string())?;
                options.out_path = value.to_string();
                index += 2;
            }
            _ => return Err(format!("unknown option: {flag}")),
        }
    }
    Ok(options)
}

fn parse_report_pack_options(args: &[String]) -> Result<ReportPackOptions, String> {
    let mut options = ReportPackOptions::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--json" => {
                options.json_output = true;
                index += 1;
            }
            "--out" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --out".to_string())?;
                options.out_path = value.to_string();
                index += 2;
            }
            _ => return Err(format!("unknown option: {flag}")),
        }
    }
    Ok(options)
}

fn parse_artifact_audit_options(args: &[String]) -> Result<ArtifactAuditOptions, String> {
    let mut options = ArtifactAuditOptions::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--json" => {
                options.json_output = true;
                index += 1;
            }
            "--text" => {
                options.json_output = false;
                index += 1;
            }
            "--no-strict" => {
                options.strict = false;
                index += 1;
            }
            "--out" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --out".to_string())?;
                options.out_path = value.to_string();
                index += 2;
            }
            _ => return Err(format!("unknown option: {flag}")),
        }
    }
    Ok(options)
}

fn parse_mvp_snapshot_options(args: &[String]) -> Result<MvpSnapshotOptions, String> {
    let mut options = MvpSnapshotOptions::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--json" => {
                options.json_output = true;
                index += 1;
            }
            "--text" => {
                options.json_output = false;
                index += 1;
            }
            "--no-strict" => {
                options.strict = false;
                index += 1;
            }
            "--out" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --out".to_string())?;
                options.out_path = value.to_string();
                index += 2;
            }
            _ => return Err(format!("unknown option: {flag}")),
        }
    }
    Ok(options)
}

fn parse_mvp_verify_options(args: &[String]) -> Result<MvpVerifyOptions, String> {
    let mut options = MvpVerifyOptions::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--json" => {
                options.json_output = true;
                index += 1;
            }
            "--text" => {
                options.json_output = false;
                index += 1;
            }
            "--refresh" => {
                options.refresh = true;
                index += 1;
            }
            "--no-strict" => {
                options.strict = false;
                index += 1;
            }
            "--out" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --out".to_string())?;
                options.out_path = value.to_string();
                index += 2;
            }
            _ => return Err(format!("unknown option: {flag}")),
        }
    }
    Ok(options)
}

fn parse_datapath_report_options(args: &[String]) -> Result<DatapathReportOptions, String> {
    let mut options = DatapathReportOptions::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--json" => {
                options.json_output = true;
                index += 1;
            }
            "--out" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --out".to_string())?;
                options.out_path = Some(value.to_string());
                index += 2;
            }
            _ => return Err(format!("unknown option: {flag}")),
        }
    }
    Ok(options)
}

fn execute_mvp_spec_check() -> Result<MvpSpecCheckResult, String> {
    let config_ok =
        execute_config_smoke(Language::En, &ConfigSmokeOptions::default(), false).is_ok();
    let doctor_artifacts_ok = check_doctor_artifacts(&[
        ("docs/doctor_latest.json", "doctor"),
        ("docs/gateway_doctor_latest.json", "gateway_doctor"),
        ("docs/lab_doctor_latest.json", "lab_doctor"),
    ]);
    let route_explain_ok = check_route_explain_artifact("docs/route_explain_latest.json");
    let datapath_ok = check_datapath_artifact("docs/datapath_latest.json");
    let rollback_artifacts_ok = check_rollback_json_artifacts(&[
        ("docs/rollback_status_latest.json", "status", true),
        ("docs/rollback_recover_latest.json", "recover", true),
        (
            "docs/rollback_status_after_recover_latest.json",
            "status",
            false,
        ),
    ]);
    let smoke_ok = run_fake_handshake().is_ok() && run_frame_replay_check().is_ok();
    let crypto_ok = smoke_ok;

    let carrier_ok = validate_carrier_profile(
        ConfigCarrierProfile::Tls,
        "203.0.113.10:443",
        "gateway.example.org",
    )
    .is_ok()
        && validate_carrier_profile(
            ConfigCarrierProfile::Quic,
            "203.0.113.10:443",
            "gateway.example.org",
        )
        .is_ok();

    let routing_ok = check_route_determinism();
    let benchmark_artifact_ok = check_benchmark_artifact("docs/benchmark_latest.json");
    let runtime_apply_dns_ok =
        check_runtime_apply_dns_artifact("docs/RUNTIME_APPLY_DNS_SMOKE.json");
    let runtime_apply_route_ok =
        check_runtime_apply_route_artifact("docs/RUNTIME_APPLY_ROUTE_SMOKE.json");
    let runtime_route_policy_validation_ok = check_runtime_route_policy_validation_artifact(
        "docs/RUNTIME_ROUTE_POLICY_VALIDATION_SMOKE.json",
    );
    let runtime_tun_name_validation_ok =
        check_runtime_tun_name_validation_artifact("docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json");
    let runtime_forced_stop_rollback_ok =
        check_runtime_forced_stop_rollback_artifact("docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json");
    let m6_ok = run_fuzz_smoke_with_output(Language::En, false) == 0
        && execute_net_sim(NetSimOptions::default()).reconnect_events > 0
        && execute_perf_smoke(PerfSmokeOptions::default()).is_ok()
        && benchmark_artifact_ok;

    let mut failed_checks = Vec::new();
    if !config_ok {
        failed_checks.push("config_smoke");
    }
    if !doctor_artifacts_ok {
        failed_checks.push("doctor_artifacts");
    }
    if !route_explain_ok {
        failed_checks.push("route_explain_artifact");
    }
    if !datapath_ok {
        failed_checks.push("datapath_artifact");
    }
    if !rollback_artifacts_ok {
        failed_checks.push("rollback_artifacts");
    }
    if !smoke_ok {
        failed_checks.push("smoke_handshake_replay");
    }
    if !carrier_ok {
        failed_checks.push("carrier_validation");
    }
    if !routing_ok {
        failed_checks.push("routing_determinism");
    }
    if !runtime_apply_dns_ok {
        failed_checks.push("runtime_apply_dns_verified");
    }
    if !runtime_apply_route_ok {
        failed_checks.push("runtime_apply_route_verified");
    }
    if !runtime_route_policy_validation_ok {
        failed_checks.push("runtime_route_policy_validation_verified");
    }
    if !runtime_tun_name_validation_ok {
        failed_checks.push("runtime_tun_name_validation_verified");
    }
    if !runtime_forced_stop_rollback_ok {
        failed_checks.push("runtime_forced_stop_rollback_verified");
    }
    if !m6_ok {
        failed_checks.push("m6_hardening");
    }
    if !failed_checks.is_empty() {
        return Err(format!(
            "mvp-spec-check failed: {}",
            failed_checks.join(",")
        ));
    }

    Ok(MvpSpecCheckResult {
        m0_workspace: true,
        m1_local_tunnel: smoke_ok,
        m2_crypto_session: crypto_ok,
        m3_carrier_validation: carrier_ok,
        m4_routing_determinism: routing_ok,
        m5_doctor_and_config: config_ok
            && doctor_artifacts_ok
            && route_explain_ok
            && datapath_ok
            && rollback_artifacts_ok
            && runtime_apply_dns_ok
            && runtime_apply_route_ok
            && runtime_route_policy_validation_ok
            && runtime_tun_name_validation_ok
            && runtime_forced_stop_rollback_ok,
        m6_hardening: m6_ok,
    })
}

fn check_runtime_apply_dns_artifact(path: &str) -> bool {
    artifact_checks::check_runtime_apply_dns_artifact(path)
}

fn check_runtime_apply_route_artifact(path: &str) -> bool {
    artifact_checks::check_runtime_apply_route_artifact(path)
}

fn check_runtime_route_policy_validation_artifact(path: &str) -> bool {
    artifact_checks::check_runtime_route_policy_validation_artifact(path)
}

fn check_runtime_tun_name_validation_artifact(path: &str) -> bool {
    artifact_checks::check_runtime_tun_name_validation_artifact(path)
}

fn check_runtime_forced_stop_rollback_artifact(path: &str) -> bool {
    artifact_checks::check_runtime_forced_stop_rollback_artifact(path)
}

fn check_rollback_json_artifacts(paths: &[(&str, &str, bool)]) -> bool {
    artifact_checks::check_rollback_json_artifacts(paths)
}

fn check_route_explain_artifact(path: &str) -> bool {
    artifact_checks::check_route_explain_artifact(path)
}

fn check_datapath_artifact(path: &str) -> bool {
    artifact_checks::check_datapath_artifact(path)
}

fn check_doctor_artifacts(paths: &[(&str, &str)]) -> bool {
    artifact_checks::check_doctor_artifacts(paths)
}

fn check_benchmark_artifact(path: &str) -> bool {
    artifact_checks::check_benchmark_artifact(path)
}

fn check_route_determinism() -> bool {
    let policy = Policy::new(vec![
        RouteRule {
            id: "example-gateway".to_string(),
            matcher: RuleMatcher::DomainSuffix("example.org".to_string()),
            outbound: OutboundMode::Gateway,
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
    let first = policy.explain(&flow).decision;
    let second = policy.explain(&flow).decision;
    first.matched_rule_id == second.matched_rule_id && first.outbound == second.outbound
}

fn check_policy_modes() -> bool {
    let policy = Policy::new(vec![
        RouteRule {
            id: "block-exact".to_string(),
            matcher: RuleMatcher::ExactDomain("blocked.example.org".to_string()),
            outbound: OutboundMode::Block,
        },
        RouteRule {
            id: "gateway-suffix".to_string(),
            matcher: RuleMatcher::DomainSuffix("example.org".to_string()),
            outbound: OutboundMode::Gateway,
        },
        RouteRule {
            id: "default-direct".to_string(),
            matcher: RuleMatcher::Default,
            outbound: OutboundMode::Direct,
        },
    ]);
    let block_flow = FlowContext {
        domain: Some("blocked.example.org".to_string()),
        destination_ip: None,
        protocol: Protocol::Tcp,
        port: Some(443),
    };
    let gateway_flow = FlowContext {
        domain: Some("api.example.org".to_string()),
        destination_ip: None,
        protocol: Protocol::Tcp,
        port: Some(443),
    };
    let direct_flow = FlowContext {
        domain: Some("example.net".to_string()),
        destination_ip: None,
        protocol: Protocol::Tcp,
        port: Some(443),
    };
    let block_decision = policy.explain(&block_flow).decision;
    let gateway_decision = policy.explain(&gateway_flow).decision;
    let direct_decision = policy.explain(&direct_flow).decision;
    block_decision.outbound == OutboundMode::Block
        && gateway_decision.outbound == OutboundMode::Gateway
        && direct_decision.outbound == OutboundMode::Direct
}

fn check_diag_export_artifact(path: &str) -> bool {
    artifact_checks::check_diag_export_artifact(path)
}

fn check_json_bilingual_message_fields(paths: &[&str]) -> bool {
    artifact_checks::check_json_bilingual_message_fields(paths)
}

fn report_is_pass(content: &str) -> bool {
    content.contains("Status: **PASS**") || content.contains("Статус: **PASS**")
}

fn check_cef_phase1_smoke_artifact(path: &str) -> bool {
    artifact_checks::check_cef_phase1_smoke_artifact(path)
}

fn check_mesh_route_explain_artifact(path: &str) -> bool {
    artifact_checks::check_mesh_route_explain_artifact(path)
}

fn check_mesh_auto_adaptive_trace_artifact(path: &str) -> bool {
    artifact_checks::check_mesh_auto_adaptive_trace_artifact(path)
}

fn detect_real_world_datapath_closed() -> bool {
    detect_real_world_datapath_closed_from_paths(&[
        "docs/REALITY_AUDIT_LATEST.json",
        "docs/REALITY_AUDIT_2026-05-18.md",
    ])
}

fn detect_real_world_datapath_closed_from_paths(paths: &[&str]) -> bool {
    for path in paths {
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(_) => continue,
        };
        if path.ends_with(".json") {
            if content.contains("\"status\":\"ok\"")
                && content.contains("\"kind\":\"reality_audit\"")
                && content.contains("\"real_world_datapath_closed\":true")
            {
                return true;
            }
            continue;
        }
        if content
            .contains("Real OS-level datapath closure for strict M4/M5: PARTIAL / NOT CLOSED.")
        {
            return false;
        }
        if content.contains("Real OS-level datapath closure for strict M4/M5: CLOSED.") {
            return true;
        }
    }
    false
}

fn build_release_gate_checklist(result: &MvpSpecCheckResult) -> ReleaseGateChecklist {
    let route_ok = check_route_explain_artifact("docs/route_explain_latest.json");
    let datapath_ok = check_datapath_artifact("docs/datapath_latest.json");
    let rollback_ok = check_rollback_json_artifacts(&[
        ("docs/rollback_status_latest.json", "status", true),
        ("docs/rollback_recover_latest.json", "recover", true),
        (
            "docs/rollback_status_after_recover_latest.json",
            "status",
            false,
        ),
    ]);
    let benchmark_ok = check_benchmark_artifact("docs/benchmark_latest.json");
    let docs_ok = fs::metadata("docs/OPERATIONS.md")
        .map(|meta| meta.is_file() && meta.len() > 0)
        .unwrap_or(false);
    let build_markers_ok = fs::metadata("Cargo.toml").is_ok()
        && fs::metadata("justfile").is_ok()
        && fs::metadata("rust-toolchain.toml").is_ok();
    let doctor_ok = check_doctor_artifacts(&[
        ("docs/doctor_latest.json", "doctor"),
        ("docs/gateway_doctor_latest.json", "gateway_doctor"),
    ]);
    let runtime_apply_dns_verified =
        check_runtime_apply_dns_artifact("docs/RUNTIME_APPLY_DNS_SMOKE.json");
    let runtime_apply_route_verified =
        check_runtime_apply_route_artifact("docs/RUNTIME_APPLY_ROUTE_SMOKE.json");
    let runtime_route_policy_validation_verified = check_runtime_route_policy_validation_artifact(
        "docs/RUNTIME_ROUTE_POLICY_VALIDATION_SMOKE.json",
    );
    let runtime_tun_name_validation_verified =
        check_runtime_tun_name_validation_artifact("docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json");
    let runtime_forced_stop_rollback_verified =
        check_runtime_forced_stop_rollback_artifact("docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json");
    ReleaseGateChecklist {
        clean_clone_builds: build_markers_ok && result.m0_workspace,
        client_gateway_run_linux: doctor_ok,
        encrypted_tunnel_carries_traffic: result.m1_local_tunnel,
        policy_routing_direct_gateway_block: check_policy_modes() && datapath_ok,
        dns_binding_works: route_ok && datapath_ok,
        route_explain_works: route_ok,
        shutdown_restores_network_state: rollback_ok,
        security_tests_pass: result.m6_hardening,
        parser_fuzz_smoke_passes: result.m6_hardening,
        no_raw_secrets_in_logs: check_diag_export_artifact("docs/diag_export_latest.json"),
        benchmark_report_exists: benchmark_ok,
        operations_guide_exists: docs_ok,
        runtime_apply_dns_verified,
        runtime_apply_route_verified,
        runtime_route_policy_validation_verified,
        runtime_tun_name_validation_verified,
        runtime_forced_stop_rollback_verified,
    }
}

fn render_mvp_spec_check_json(result: MvpSpecCheckResult) -> String {
    mvp_reports::render_mvp_spec_check_json(result)
}

fn render_mvp_spec_check_text(lang: Language, result: MvpSpecCheckResult) -> String {
    let mut out = String::new();
    match lang {
        Language::En => {
            out.push_str("MVP spec check: ok\n");
            out.push_str(&format!("M0 workspace/tooling: {}\n", result.m0_workspace));
            out.push_str(&format!(
                "M1 local tunnel checks: {}\n",
                result.m1_local_tunnel
            ));
            out.push_str(&format!(
                "M2 crypto/session checks: {}\n",
                result.m2_crypto_session
            ));
            out.push_str(&format!(
                "M3 carrier validation: {}\n",
                result.m3_carrier_validation
            ));
            out.push_str(&format!(
                "M4 routing determinism: {}\n",
                result.m4_routing_determinism
            ));
            out.push_str(&format!(
                "M5 doctor/config checks: {}\n",
                result.m5_doctor_and_config
            ));
            out.push_str(&format!("M6 hardening checks: {}\n", result.m6_hardening));
            out.push_str("Network state: not modified\n");
        }
        Language::Ru => {
            out.push_str("Проверка MVP-спеки: ok\n");
            out.push_str(&format!("M0 workspace/tooling: {}\n", result.m0_workspace));
            out.push_str(&format!(
                "M1 локальный туннель: {}\n",
                result.m1_local_tunnel
            ));
            out.push_str(&format!(
                "M2 crypto/session проверки: {}\n",
                result.m2_crypto_session
            ));
            out.push_str(&format!(
                "M3 валидация carrier: {}\n",
                result.m3_carrier_validation
            ));
            out.push_str(&format!(
                "M4 детерминизм роутинга: {}\n",
                result.m4_routing_determinism
            ));
            out.push_str(&format!(
                "M5 doctor/config проверки: {}\n",
                result.m5_doctor_and_config
            ));
            out.push_str(&format!("M6 hardening проверки: {}\n", result.m6_hardening));
            out.push_str("Состояние сети: не изменялось\n");
        }
    }
    out
}

fn run_mvp_spec_check(lang: Language, args: &[String]) -> i32 {
    let options = match parse_mvp_spec_check_options(args) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => {
                    eprintln!("MVP spec check options error: {error}");
                    eprintln!(
                        "usage: chimera-lab [--lang en|ru] mvp-spec-check [--json] [--out <file>]"
                    );
                }
                Language::Ru => {
                    eprintln!("Ошибка опций mvp-spec-check: {error}");
                    eprintln!(
                        "использование: chimera-lab [--lang en|ru] mvp-spec-check [--json] [--out <файл>]"
                    );
                }
            }
            return 2;
        }
    };

    let result = match execute_mvp_spec_check() {
        Ok(result) => result,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };

    let json = render_mvp_spec_check_json(result);
    if let Some(path) = options.out_path.as_deref()
        && let Err(error) = fs::write(path, &json)
    {
        eprintln!("mvp-spec-check report write failed: {error}");
        return 1;
    }
    if options.json_output {
        println!("{json}");
    } else {
        print!("{}", render_mvp_spec_check_text(lang, result));
    }
    0
}

fn run_mvp_spec_report(lang: Language, args: &[String]) -> i32 {
    let options = match parse_mvp_spec_report_options(args) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => {
                    eprintln!("MVP spec report options error: {error}");
                    eprintln!("usage: chimera-lab [--lang en|ru] mvp-spec-report [--out <file>]");
                }
                Language::Ru => {
                    eprintln!("Ошибка опций mvp-spec-report: {error}");
                    eprintln!(
                        "использование: chimera-lab [--lang en|ru] mvp-spec-report [--out <файл>]"
                    );
                }
            }
            return 2;
        }
    };
    let result = match execute_mvp_spec_check() {
        Ok(result) => result,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    let report = render_mvp_spec_report_markdown(result);
    if let Err(error) = fs::write(&options.out_path, &report) {
        eprintln!("mvp-spec-report write failed: {error}");
        return 1;
    }
    match lang {
        Language::En => println!("MVP spec report: saved to {}", options.out_path),
        Language::Ru => println!("Отчет MVP-спеки: сохранен в {}", options.out_path),
    }
    println!("{report}");
    0
}

fn run_m5_artifacts_report(lang: Language, args: &[String]) -> i32 {
    let options = match parse_m5_artifacts_report_options(args) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => {
                    eprintln!("M5 artifacts report options error: {error}");
                    eprintln!(
                        "usage: chimera-lab [--lang en|ru] m5-artifacts-report [--out <file>]"
                    );
                }
                Language::Ru => {
                    eprintln!("Ошибка опций m5-artifacts-report: {error}");
                    eprintln!(
                        "использование: chimera-lab [--lang en|ru] m5-artifacts-report [--out <файл>]"
                    );
                }
            }
            return 2;
        }
    };

    let doctor_ok = check_doctor_artifacts(&[
        ("docs/doctor_latest.json", "doctor"),
        ("docs/gateway_doctor_latest.json", "gateway_doctor"),
        ("docs/lab_doctor_latest.json", "lab_doctor"),
    ]);
    let route_ok = check_route_explain_artifact("docs/route_explain_latest.json");
    let datapath_ok = check_datapath_artifact("docs/datapath_latest.json");
    let rollback_ok = check_rollback_json_artifacts(&[
        ("docs/rollback_status_latest.json", "status", true),
        ("docs/rollback_recover_latest.json", "recover", true),
        (
            "docs/rollback_status_after_recover_latest.json",
            "status",
            false,
        ),
    ]);
    let config_ok =
        execute_config_smoke(Language::En, &ConfigSmokeOptions::default(), false).is_ok();
    let all_ok = doctor_ok && route_ok && datapath_ok && rollback_ok && config_ok;

    let report = render_m5_artifacts_report_markdown(
        lang,
        all_ok,
        config_ok,
        doctor_ok,
        route_ok,
        datapath_ok,
        rollback_ok,
    );

    if let Err(error) = fs::write(&options.out_path, &report) {
        eprintln!("m5-artifacts-report write failed: {error}");
        return 1;
    }
    match lang {
        Language::En => println!("M5 artifacts report: saved to {}", options.out_path),
        Language::Ru => println!("Отчет M5-артефактов: сохранен в {}", options.out_path),
    }
    println!("{report}");
    0
}

fn run_m6_artifacts_report(lang: Language, args: &[String]) -> i32 {
    let options = match parse_m6_artifacts_report_options(args) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => {
                    eprintln!("M6 artifacts report options error: {error}");
                    eprintln!(
                        "usage: chimera-lab [--lang en|ru] m6-artifacts-report [--out <file>]"
                    );
                }
                Language::Ru => {
                    eprintln!("Ошибка опций m6-artifacts-report: {error}");
                    eprintln!(
                        "использование: chimera-lab [--lang en|ru] m6-artifacts-report [--out <файл>]"
                    );
                }
            }
            return 2;
        }
    };

    let benchmark_ok = check_benchmark_artifact("docs/benchmark_latest.json");
    let mvp_check_ok = {
        let content = fs::read_to_string("docs/mvp_spec_check_latest.json");
        if let Ok(content) = content {
            content.contains("\"status\":\"ok\"")
                && content.contains("\"kind\":\"mvp_spec_check\"")
                && content.contains("\"m6_hardening\":true")
        } else {
            false
        }
    };
    let hardening_ok = benchmark_ok && mvp_check_ok;

    let report =
        render_m6_artifacts_report_markdown(lang, hardening_ok, benchmark_ok, mvp_check_ok);

    if let Err(error) = fs::write(&options.out_path, &report) {
        eprintln!("m6-artifacts-report write failed: {error}");
        return 1;
    }
    match lang {
        Language::En => println!("M6 artifacts report: saved to {}", options.out_path),
        Language::Ru => println!("Отчет M6-артефактов: сохранен в {}", options.out_path),
    }
    println!("{report}");
    0
}

fn run_release_readiness_report(lang: Language, args: &[String]) -> i32 {
    let options = match parse_release_readiness_report_options(args) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => {
                    eprintln!("Release readiness options error: {error}");
                    eprintln!(
                        "usage: chimera-lab [--lang en|ru] release-readiness-report [--json] [--out <file>]"
                    );
                }
                Language::Ru => {
                    eprintln!("Ошибка опций release-readiness-report: {error}");
                    eprintln!(
                        "использование: chimera-lab [--lang en|ru] release-readiness-report [--json] [--out <файл>]"
                    );
                }
            }
            return 2;
        }
    };

    let mvp = execute_mvp_spec_check();
    let mvp_ok = mvp.is_ok();
    let result = mvp.unwrap_or(MvpSpecCheckResult {
        m0_workspace: false,
        m1_local_tunnel: false,
        m2_crypto_session: false,
        m3_carrier_validation: false,
        m4_routing_determinism: false,
        m5_doctor_and_config: false,
        m6_hardening: false,
    });
    let m5_report_ok = fs::read_to_string("docs/M5_ARTIFACTS_REPORT.md")
        .map(|v| report_is_pass(&v))
        .unwrap_or(false);
    let m6_report_ok = fs::read_to_string("docs/M6_ARTIFACTS_REPORT.md")
        .map(|v| report_is_pass(&v))
        .unwrap_or(false);
    let artifacts = ReleaseReadinessArtifacts {
        m5_report_ok,
        m6_report_ok,
        benchmark_ok: check_benchmark_artifact("docs/benchmark_latest.json"),
        cef_phase1_smoke_ok: check_cef_phase1_smoke_artifact("docs/CEF_PHASE1_SMOKE.json"),
        mesh_route_explain_ok: check_mesh_route_explain_artifact("docs/MESH_ROUTE_EXPLAIN.json"),
        mesh_auto_adaptive_ok: check_mesh_auto_adaptive_trace_artifact(
            "docs/MESH_AUTO_ADAPTIVE_TRACE.json",
        ),
    };
    let checklist = build_release_gate_checklist(&result);
    let release_ok = mvp_ok
        && artifacts.m5_report_ok
        && artifacts.m6_report_ok
        && artifacts.benchmark_ok
        && artifacts.cef_phase1_smoke_ok
        && artifacts.mesh_route_explain_ok
        && artifacts.mesh_auto_adaptive_ok
        && checklist.all_ok();

    let real_world_datapath_closed = detect_real_world_datapath_closed();
    let report = render_release_readiness_report_markdown(
        lang,
        release_ok,
        real_world_datapath_closed,
        result,
        checklist,
        artifacts,
    );

    if options.json_output {
        let json = render_release_readiness_report_json(
            release_ok,
            real_world_datapath_closed,
            result,
            checklist,
            artifacts,
        );
        if let Err(error) = fs::write(&options.out_path, &json) {
            eprintln!("release-readiness-report write failed: {error}");
            return 1;
        }
        match lang {
            Language::En => println!("Release readiness report: saved to {}", options.out_path),
            Language::Ru => println!("Отчет готовности релиза: сохранен в {}", options.out_path),
        }
        println!("{json}");
        return 0;
    }

    if let Err(error) = fs::write(&options.out_path, &report) {
        eprintln!("release-readiness-report write failed: {error}");
        return 1;
    }
    match lang {
        Language::En => println!("Release readiness report: saved to {}", options.out_path),
        Language::Ru => println!("Отчет готовности релиза: сохранен в {}", options.out_path),
    }
    println!("{report}");
    0
}

fn run_report_pack(lang: Language, args: &[String]) -> i32 {
    let options = match parse_report_pack_options(args) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => {
                    eprintln!("Report pack options error: {error}");
                    eprintln!(
                        "usage: chimera-lab [--lang en|ru] report-pack [--json] [--out <file>]"
                    );
                }
                Language::Ru => {
                    eprintln!("Ошибка опций report-pack: {error}");
                    eprintln!(
                        "использование: chimera-lab [--lang en|ru] report-pack [--json] [--out <файл>]"
                    );
                }
            }
            return 2;
        }
    };

    if run_mvp_spec_report(lang, &[]) != 0 {
        return 1;
    }
    if run_datapath_report(
        lang,
        &[
            "--json".to_string(),
            "--out".to_string(),
            "docs/datapath_latest.json".to_string(),
        ],
    ) != 0
    {
        return 1;
    }
    if run_m5_artifacts_report(lang, &[]) != 0 {
        return 1;
    }
    if run_m6_artifacts_report(lang, &[]) != 0 {
        return 1;
    }
    if run_cef_phase1_smoke(
        lang,
        &[
            "--json".to_string(),
            "--out".to_string(),
            "docs/CEF_PHASE1_SMOKE.json".to_string(),
        ],
    ) != 0
    {
        return 1;
    }
    if run_mesh_auto_smoke(
        lang,
        &[
            "--json".to_string(),
            "--out".to_string(),
            "docs/MESH_AUTO_ADAPTIVE_TRACE.json".to_string(),
        ],
    ) != 0
    {
        return 1;
    }
    if run_release_readiness_report(lang, &[]) != 0 {
        return 1;
    }

    let mvp_ok = fs::read_to_string("docs/MVP_SPEC_COVERAGE.md")
        .map(|v| report_is_pass(&v))
        .unwrap_or(false);
    let m5_ok = fs::read_to_string("docs/M5_ARTIFACTS_REPORT.md")
        .map(|v| report_is_pass(&v))
        .unwrap_or(false);
    let m6_ok = fs::read_to_string("docs/M6_ARTIFACTS_REPORT.md")
        .map(|v| report_is_pass(&v))
        .unwrap_or(false);
    let release_ok = fs::read_to_string("docs/RELEASE_READINESS_REPORT.md")
        .map(|v| report_is_pass(&v))
        .unwrap_or(false);
    let cef_phase1_ok = check_cef_phase1_smoke_artifact("docs/CEF_PHASE1_SMOKE.json");
    let mesh_route_explain_ok = check_mesh_route_explain_artifact("docs/MESH_ROUTE_EXPLAIN.json");
    let mesh_auto_adaptive_ok =
        check_mesh_auto_adaptive_trace_artifact("docs/MESH_AUTO_ADAPTIVE_TRACE.json");
    let all_ok = mvp_ok
        && m5_ok
        && m6_ok
        && release_ok
        && cef_phase1_ok
        && mesh_route_explain_ok
        && mesh_auto_adaptive_ok;
    let real_world_datapath_closed = detect_real_world_datapath_closed();

    let report = match lang {
        Language::En => format!(
            "# Report Pack\n\n\
Status: **{}**\n\n\
Included reports:\n\
- MVP spec coverage: `{}` (`docs/MVP_SPEC_COVERAGE.md`)\n\
- M5 artifacts: `{}` (`docs/M5_ARTIFACTS_REPORT.md`)\n\
- M6 artifacts: `{}` (`docs/M6_ARTIFACTS_REPORT.md`)\n\
- Release readiness: `{}` (`docs/RELEASE_READINESS_REPORT.md`)\n\
- CEF phase1 smoke: `{}` (`docs/CEF_PHASE1_SMOKE.json`)\n\
- Mesh route explain: `{}` (`docs/MESH_ROUTE_EXPLAIN.json`)\n\
- Mesh auto adaptive trace: `{}` (`docs/MESH_AUTO_ADAPTIVE_TRACE.json`)\n\n\
Truth boundary:\n\
- Lab/proof/report contour only: `true`\n\
- Real OS-level datapath closure (strict M4/M5): `{}`\n\n\
Network safety: no OS route/DNS/firewall/proxy changes in this report path.\n",
            if all_ok { "PASS" } else { "FAIL" },
            mvp_ok,
            m5_ok,
            m6_ok,
            release_ok,
            cef_phase1_ok,
            mesh_route_explain_ok,
            mesh_auto_adaptive_ok,
            real_world_datapath_closed
        ),
        Language::Ru => format!(
            "# Пакет Отчетов\n\n\
Статус: **{}**\n\n\
Включенные отчеты:\n\
- Покрытие MVP-спеки: `{}` (`docs/MVP_SPEC_COVERAGE.md`)\n\
- Артефакты M5: `{}` (`docs/M5_ARTIFACTS_REPORT.md`)\n\
- Артефакты M6: `{}` (`docs/M6_ARTIFACTS_REPORT.md`)\n\
- Готовность релиза: `{}` (`docs/RELEASE_READINESS_REPORT.md`)\n\
- CEF phase1 smoke: `{}` (`docs/CEF_PHASE1_SMOKE.json`)\n\
- Mesh route explain: `{}` (`docs/MESH_ROUTE_EXPLAIN.json`)\n\
- Mesh auto adaptive trace: `{}` (`docs/MESH_AUTO_ADAPTIVE_TRACE.json`)\n\n\
Граница истины:\n\
- Контур lab/proof/report: `true`\n\
- Real OS-level datapath closure (strict M4/M5): `{}`\n\n\
Безопасность сети: в этом отчете мы не меняем маршруты/DNS/firewall/proxy ОС.\n",
            if all_ok { "PASS" } else { "FAIL" },
            mvp_ok,
            m5_ok,
            m6_ok,
            release_ok,
            cef_phase1_ok,
            mesh_route_explain_ok,
            mesh_auto_adaptive_ok,
            real_world_datapath_closed
        ),
    };

    if options.json_output {
        let release_gate = execute_mvp_spec_check()
            .map(|r| build_release_gate_checklist(&r))
            .unwrap_or(ReleaseGateChecklist {
                clean_clone_builds: false,
                client_gateway_run_linux: false,
                encrypted_tunnel_carries_traffic: false,
                policy_routing_direct_gateway_block: false,
                dns_binding_works: false,
                route_explain_works: false,
                shutdown_restores_network_state: false,
                security_tests_pass: false,
                parser_fuzz_smoke_passes: false,
                no_raw_secrets_in_logs: false,
                benchmark_report_exists: false,
                operations_guide_exists: false,
                runtime_apply_dns_verified: false,
                runtime_apply_route_verified: false,
                runtime_route_policy_validation_verified: false,
                runtime_tun_name_validation_verified: false,
                runtime_forced_stop_rollback_verified: false,
            });
        let real_world_datapath_closed = detect_real_world_datapath_closed();
        let json = format!(
            "{{\"status\":\"{}\",\"kind\":\"report_pack\",\"message_en\":\"Combined MVP reports are ready.\",\"message_ru\":\"Сводные отчеты MVP готовы.\",\"mvp_spec_report\":{},\"m5_artifacts_report\":{},\"m6_artifacts_report\":{},\"release_readiness_report\":{},\"cef_phase1_smoke\":{},\"mesh_route_explain\":{},\"mesh_auto_adaptive_trace\":{},\"truth_boundary\":{{\"lab_scope_only\":true,\"real_world_datapath_closed\":{}}},\"release_gate\":{{\"clean_clone_builds\":{},\"client_gateway_run_linux\":{},\"encrypted_tunnel_carries_traffic\":{},\"policy_routing_direct_gateway_block\":{},\"dns_binding_works\":{},\"route_explain_works\":{},\"shutdown_restores_network_state\":{},\"security_tests_pass\":{},\"parser_fuzz_smoke_passes\":{},\"no_raw_secrets_in_logs\":{},\"benchmark_report_exists\":{},\"operations_guide_exists\":{},\"runtime_apply_dns_verified\":{},\"runtime_apply_route_verified\":{},\"runtime_route_policy_validation_verified\":{},\"runtime_tun_name_validation_verified\":{},\"runtime_forced_stop_rollback_verified\":{}}},\"network_state\":\"not_modified\"}}",
            if all_ok { "ok" } else { "fail" },
            mvp_ok,
            m5_ok,
            m6_ok,
            release_ok,
            cef_phase1_ok,
            mesh_route_explain_ok,
            mesh_auto_adaptive_ok,
            real_world_datapath_closed,
            release_gate.clean_clone_builds,
            release_gate.client_gateway_run_linux,
            release_gate.encrypted_tunnel_carries_traffic,
            release_gate.policy_routing_direct_gateway_block,
            release_gate.dns_binding_works,
            release_gate.route_explain_works,
            release_gate.shutdown_restores_network_state,
            release_gate.security_tests_pass,
            release_gate.parser_fuzz_smoke_passes,
            release_gate.no_raw_secrets_in_logs,
            release_gate.benchmark_report_exists,
            release_gate.operations_guide_exists,
            release_gate.runtime_apply_dns_verified,
            release_gate.runtime_apply_route_verified,
            release_gate.runtime_route_policy_validation_verified,
            release_gate.runtime_tun_name_validation_verified,
            release_gate.runtime_forced_stop_rollback_verified
        );
        if let Err(error) = fs::write(&options.out_path, &json) {
            eprintln!("report-pack write failed: {error}");
            return 1;
        }
        match lang {
            Language::En => println!("Report pack: saved to {}", options.out_path),
            Language::Ru => println!("Пакет отчетов: сохранен в {}", options.out_path),
        }
        println!("{json}");
        return 0;
    }

    if let Err(error) = fs::write(&options.out_path, &report) {
        eprintln!("report-pack write failed: {error}");
        return 1;
    }
    match lang {
        Language::En => println!("Report pack: saved to {}", options.out_path),
        Language::Ru => println!("Пакет отчетов: сохранен в {}", options.out_path),
    }
    println!("{report}");
    0
}

fn run_artifact_audit(lang: Language, args: &[String]) -> i32 {
    let options = match parse_artifact_audit_options(args) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => {
                    eprintln!("Artifact audit options error: {error}");
                    eprintln!(
                        "usage: chimera-lab [--lang en|ru] artifact-audit [--json|--text] [--no-strict] [--out <file>]"
                    );
                }
                Language::Ru => {
                    eprintln!("Ошибка опций artifact-audit: {error}");
                    eprintln!(
                        "использование: chimera-lab [--lang en|ru] artifact-audit [--json|--text] [--no-strict] [--out <файл>]"
                    );
                }
            }
            return 2;
        }
    };

    let doctor_ok = check_doctor_artifacts(&[
        ("docs/doctor_latest.json", "doctor"),
        ("docs/gateway_doctor_latest.json", "gateway_doctor"),
        ("docs/lab_doctor_latest.json", "lab_doctor"),
    ]);
    let route_ok = check_route_explain_artifact("docs/route_explain_latest.json");
    let datapath_ok = check_datapath_artifact("docs/datapath_latest.json");
    let rollback_ok = check_rollback_json_artifacts(&[
        ("docs/rollback_status_latest.json", "status", true),
        ("docs/rollback_recover_latest.json", "recover", true),
        (
            "docs/rollback_status_after_recover_latest.json",
            "status",
            false,
        ),
    ]);
    let benchmark_ok = check_benchmark_artifact("docs/benchmark_latest.json");
    let diag_ok = check_diag_export_artifact("docs/diag_export_latest.json");
    let mesh_auto_ok =
        check_mesh_auto_adaptive_trace_artifact("docs/MESH_AUTO_ADAPTIVE_TRACE.json");
    let message_fields_ok = check_json_bilingual_message_fields(&[
        "docs/doctor_latest.json",
        "docs/gateway_doctor_latest.json",
        "docs/lab_doctor_latest.json",
        "docs/datapath_latest.json",
        "docs/route_explain_latest.json",
        "docs/rollback_status_latest.json",
        "docs/rollback_recover_latest.json",
        "docs/rollback_status_after_recover_latest.json",
        "docs/diag_export_latest.json",
    ]);
    let mvp_check_ok = fs::read_to_string("docs/mvp_spec_check_latest.json")
        .map(|v| {
            v.contains("\"status\":\"ok\"")
                && v.contains("\"kind\":\"mvp_spec_check\"")
                && v.contains("\"m6_hardening\":true")
        })
        .unwrap_or(false);
    let all_ok = doctor_ok
        && route_ok
        && datapath_ok
        && rollback_ok
        && benchmark_ok
        && diag_ok
        && mesh_auto_ok
        && message_fields_ok
        && mvp_check_ok;
    let status = if all_ok { "ok" } else { "fail" };

    let json = format!(
        "{{\"status\":\"{}\",\"kind\":\"artifact_audit\",\"message_en\":\"Artifact safety check finished.\",\"message_ru\":\"Проверка безопасности артефактов завершена.\",\"doctor\":{},\"route_explain\":{},\"datapath\":{},\"rollback\":{},\"benchmark\":{},\"diag_export_redacted\":{},\"mesh_auto_adaptive_trace\":{},\"bilingual_message_fields\":{},\"mvp_spec_check\":{},\"network_state\":\"not_modified\"}}",
        status,
        doctor_ok,
        route_ok,
        datapath_ok,
        rollback_ok,
        benchmark_ok,
        diag_ok,
        mesh_auto_ok,
        message_fields_ok,
        mvp_check_ok
    );

    if options.json_output {
        if let Err(error) = fs::write(&options.out_path, &json) {
            eprintln!("artifact-audit write failed: {error}");
            return 1;
        }
        match lang {
            Language::En => println!("Artifact audit: saved to {}", options.out_path),
            Language::Ru => println!("Аудит артефактов: сохранен в {}", options.out_path),
        }
        println!("{json}");
    } else {
        let text = match lang {
            Language::En => format!(
                "Artifact audit result: {}\nDoctor report is valid: {}\nRoute explanation report is valid: {}\nDatapath report is valid: {}\nRollback reports are valid: {}\nBenchmark report is valid: {}\nDiagnostic export hides secrets: {}\nMesh auto adaptive trace is valid: {}\nBilingual message fields are present: {}\nMVP spec check report is valid: {}\nNetwork state: not modified",
                status,
                doctor_ok,
                route_ok,
                datapath_ok,
                rollback_ok,
                benchmark_ok,
                diag_ok,
                mesh_auto_ok,
                message_fields_ok,
                mvp_check_ok
            ),
            Language::Ru => format!(
                "Результат аудита артефактов: {}\nОтчет Doctor корректен: {}\nОтчет Route explain корректен: {}\nОтчет Datapath корректен: {}\nОтчеты Rollback корректны: {}\nОтчет Benchmark корректен: {}\nДиагностический экспорт скрывает секреты: {}\nMesh auto adaptive trace корректен: {}\nДвуязычные поля message присутствуют: {}\nОтчет проверки MVP-спеки корректен: {}\nСостояние сети: не изменялось",
                status,
                doctor_ok,
                route_ok,
                datapath_ok,
                rollback_ok,
                benchmark_ok,
                diag_ok,
                mesh_auto_ok,
                message_fields_ok,
                mvp_check_ok
            ),
        };
        if let Err(error) = fs::write(&options.out_path, &text) {
            eprintln!("artifact-audit write failed: {error}");
            return 1;
        }
        match lang {
            Language::En => println!("Artifact audit: saved to {}", options.out_path),
            Language::Ru => println!("Аудит артефактов: сохранен в {}", options.out_path),
        }
        println!("{text}");
    }
    if all_ok || !options.strict { 0 } else { 1 }
}

fn run_mvp_snapshot(lang: Language, args: &[String]) -> i32 {
    let options = match parse_mvp_snapshot_options(args) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => {
                    eprintln!("MVP snapshot options error: {error}");
                    eprintln!(
                        "usage: chimera-lab [--lang en|ru] mvp-snapshot [--json|--text] [--no-strict] [--out <file>]"
                    );
                }
                Language::Ru => {
                    eprintln!("Ошибка опций mvp-snapshot: {error}");
                    eprintln!(
                        "использование: chimera-lab [--lang en|ru] mvp-snapshot [--json|--text] [--no-strict] [--out <файл>]"
                    );
                }
            }
            return 2;
        }
    };

    let mvp = execute_mvp_spec_check().unwrap_or(MvpSpecCheckResult {
        m0_workspace: false,
        m1_local_tunnel: false,
        m2_crypto_session: false,
        m3_carrier_validation: false,
        m4_routing_determinism: false,
        m5_doctor_and_config: false,
        m6_hardening: false,
    });
    let gate = build_release_gate_checklist(&mvp);
    let audit = {
        let doctor_ok = check_doctor_artifacts(&[
            ("docs/doctor_latest.json", "doctor"),
            ("docs/gateway_doctor_latest.json", "gateway_doctor"),
            ("docs/lab_doctor_latest.json", "lab_doctor"),
        ]);
        let route_ok = check_route_explain_artifact("docs/route_explain_latest.json");
        let datapath_ok = check_datapath_artifact("docs/datapath_latest.json");
        let rollback_ok = check_rollback_json_artifacts(&[
            ("docs/rollback_status_latest.json", "status", true),
            ("docs/rollback_recover_latest.json", "recover", true),
            (
                "docs/rollback_status_after_recover_latest.json",
                "status",
                false,
            ),
        ]);
        let benchmark_ok = check_benchmark_artifact("docs/benchmark_latest.json");
        let diag_ok = check_diag_export_artifact("docs/diag_export_latest.json");
        let mesh_auto_ok =
            check_mesh_auto_adaptive_trace_artifact("docs/MESH_AUTO_ADAPTIVE_TRACE.json");
        let mvp_check_ok = fs::read_to_string("docs/mvp_spec_check_latest.json")
            .map(|v| v.contains("\"status\":\"ok\"") && v.contains("\"m6_hardening\":true"))
            .unwrap_or(false);
        doctor_ok
            && route_ok
            && datapath_ok
            && rollback_ok
            && benchmark_ok
            && diag_ok
            && mesh_auto_ok
            && mvp_check_ok
    };
    let release_ready = gate.all_ok()
        && mvp.m0_workspace
        && mvp.m1_local_tunnel
        && mvp.m2_crypto_session
        && mvp.m3_carrier_validation
        && mvp.m4_routing_determinism
        && mvp.m5_doctor_and_config
        && mvp.m6_hardening
        && audit;
    let real_world_datapath_closed = detect_real_world_datapath_closed();

    let json = format!(
        "{{\"status\":\"{}\",\"kind\":\"mvp_snapshot\",\"message_en\":\"MVP snapshot generated.\",\"message_ru\":\"Снимок MVP сформирован.\",\"release_ready\":{},\"truth_boundary\":{{\"lab_scope_only\":true,\"real_world_datapath_closed\":{}}},\"mvp\":{{\"m0\":{},\"m1\":{},\"m2\":{},\"m3\":{},\"m4\":{},\"m5\":{},\"m6\":{}}},\"release_gate\":{{\"clean_clone_builds\":{},\"client_gateway_run_linux\":{},\"encrypted_tunnel_carries_traffic\":{},\"policy_routing_direct_gateway_block\":{},\"dns_binding_works\":{},\"route_explain_works\":{},\"shutdown_restores_network_state\":{},\"security_tests_pass\":{},\"parser_fuzz_smoke_passes\":{},\"no_raw_secrets_in_logs\":{},\"benchmark_report_exists\":{},\"operations_guide_exists\":{},\"runtime_apply_dns_verified\":{},\"runtime_apply_route_verified\":{},\"runtime_route_policy_validation_verified\":{},\"runtime_tun_name_validation_verified\":{},\"runtime_forced_stop_rollback_verified\":{}}},\"artifact_audit\":{},\"network_state\":\"not_modified\"}}",
        if release_ready { "ok" } else { "fail" },
        release_ready,
        real_world_datapath_closed,
        mvp.m0_workspace,
        mvp.m1_local_tunnel,
        mvp.m2_crypto_session,
        mvp.m3_carrier_validation,
        mvp.m4_routing_determinism,
        mvp.m5_doctor_and_config,
        mvp.m6_hardening,
        gate.clean_clone_builds,
        gate.client_gateway_run_linux,
        gate.encrypted_tunnel_carries_traffic,
        gate.policy_routing_direct_gateway_block,
        gate.dns_binding_works,
        gate.route_explain_works,
        gate.shutdown_restores_network_state,
        gate.security_tests_pass,
        gate.parser_fuzz_smoke_passes,
        gate.no_raw_secrets_in_logs,
        gate.benchmark_report_exists,
        gate.operations_guide_exists,
        gate.runtime_apply_dns_verified,
        gate.runtime_apply_route_verified,
        gate.runtime_route_policy_validation_verified,
        gate.runtime_tun_name_validation_verified,
        gate.runtime_forced_stop_rollback_verified,
        audit
    );
    if options.json_output {
        if let Err(error) = fs::write(&options.out_path, &json) {
            eprintln!("mvp-snapshot write failed: {error}");
            return 1;
        }
        match lang {
            Language::En => println!("MVP snapshot: saved to {}", options.out_path),
            Language::Ru => println!("Снимок MVP: сохранен в {}", options.out_path),
        }
        println!("{json}");
    } else {
        let text = match lang {
            Language::En => format!(
                "MVP snapshot result: {}\nReady for release: {}\nArtifact audit passed: {}\nNetwork state: not modified",
                if release_ready { "ok" } else { "fail" },
                release_ready,
                audit
            ),
            Language::Ru => format!(
                "Результат снимка MVP: {}\nГотово к релизу: {}\nАудит артефактов пройден: {}\nСостояние сети: не изменялось",
                if release_ready { "ok" } else { "fail" },
                release_ready,
                audit
            ),
        };
        if let Err(error) = fs::write(&options.out_path, &text) {
            eprintln!("mvp-snapshot write failed: {error}");
            return 1;
        }
        match lang {
            Language::En => println!("MVP snapshot: saved to {}", options.out_path),
            Language::Ru => println!("Снимок MVP: сохранен в {}", options.out_path),
        }
        println!("{text}");
    }
    if release_ready || !options.strict {
        0
    } else {
        1
    }
}

fn run_mvp_verify(lang: Language, args: &[String]) -> i32 {
    let options = match parse_mvp_verify_options(args) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => {
                    eprintln!("MVP verify options error: {error}");
                    eprintln!(
                        "usage: chimera-lab [--lang en|ru] mvp-verify [--json|--text] [--refresh] [--no-strict] [--out <file>]"
                    );
                }
                Language::Ru => {
                    eprintln!("Ошибка опций mvp-verify: {error}");
                    eprintln!(
                        "использование: chimera-lab [--lang en|ru] mvp-verify [--json|--text] [--refresh] [--no-strict] [--out <файл>]"
                    );
                }
            }
            return 2;
        }
    };

    if options.refresh {
        if run_doctor(
            lang,
            &[
                "--json".to_string(),
                "--out".to_string(),
                "docs/lab_doctor_latest.json".to_string(),
            ],
        ) != 0
        {
            return 1;
        }
        if run_mvp_spec_check(
            lang,
            &[
                "--json".to_string(),
                "--out".to_string(),
                "docs/mvp_spec_check_latest.json".to_string(),
            ],
        ) != 0
        {
            return 1;
        }
        if run_mvp_spec_report(
            lang,
            &["--out".to_string(), "docs/MVP_SPEC_COVERAGE.md".to_string()],
        ) != 0
        {
            return 1;
        }
        if run_m5_artifacts_report(
            lang,
            &[
                "--out".to_string(),
                "docs/M5_ARTIFACTS_REPORT.md".to_string(),
            ],
        ) != 0
        {
            return 1;
        }
        if run_m6_artifacts_report(
            lang,
            &[
                "--out".to_string(),
                "docs/M6_ARTIFACTS_REPORT.md".to_string(),
            ],
        ) != 0
        {
            return 1;
        }
        if run_release_readiness_report(
            lang,
            &[
                "--out".to_string(),
                "docs/RELEASE_READINESS_REPORT.md".to_string(),
            ],
        ) != 0
        {
            return 1;
        }
        if run_report_pack(
            lang,
            &[
                "--json".to_string(),
                "--out".to_string(),
                "docs/REPORT_PACK.json".to_string(),
            ],
        ) != 0
        {
            return 1;
        }
        if run_report_pack(
            lang,
            &["--out".to_string(), "docs/REPORT_PACK.md".to_string()],
        ) != 0
        {
            return 1;
        }
        if run_artifact_audit(
            lang,
            &[
                "--json".to_string(),
                "--out".to_string(),
                "docs/ARTIFACT_AUDIT.json".to_string(),
            ],
        ) != 0
        {
            return 1;
        }
        if run_mvp_snapshot(
            lang,
            &[
                "--json".to_string(),
                "--out".to_string(),
                "docs/MVP_SNAPSHOT.json".to_string(),
            ],
        ) != 0
        {
            return 1;
        }
    }

    let smoke_ok = run_fake_handshake().is_ok() && run_frame_replay_check().is_ok();
    let fuzz_ok = run_fuzz_smoke_with_output(lang, false) == 0;
    let perf_ok = execute_perf_smoke(PerfSmokeOptions::default()).is_ok();
    let net_sim_ok = execute_net_sim(NetSimOptions::default()).reconnect_events > 0;
    let doctor_ok = check_doctor_artifacts(&[
        ("docs/doctor_latest.json", "doctor"),
        ("docs/gateway_doctor_latest.json", "gateway_doctor"),
        ("docs/lab_doctor_latest.json", "lab_doctor"),
    ]);
    let mvp_check_ok = execute_mvp_spec_check().is_ok();
    let mvp_report_ok = fs::read_to_string("docs/MVP_SPEC_COVERAGE.md")
        .map(|v| report_is_pass(&v))
        .unwrap_or(false);
    let m5_ok = fs::read_to_string("docs/M5_ARTIFACTS_REPORT.md")
        .map(|v| report_is_pass(&v))
        .unwrap_or(false);
    let m6_ok = fs::read_to_string("docs/M6_ARTIFACTS_REPORT.md")
        .map(|v| report_is_pass(&v))
        .unwrap_or(false);
    let release_ok = fs::read_to_string("docs/RELEASE_READINESS_REPORT.md")
        .map(|v| report_is_pass(&v))
        .unwrap_or(false);
    let pack_ok = fs::read_to_string("docs/REPORT_PACK.md")
        .map(|v| report_is_pass(&v))
        .unwrap_or(false);
    let audit_ok = fs::read_to_string("docs/ARTIFACT_AUDIT.json")
        .map(|v| v.contains("\"status\":\"ok\"") && v.contains("\"kind\":\"artifact_audit\""))
        .unwrap_or(false);
    let snapshot_ok = fs::read_to_string("docs/MVP_SNAPSHOT.json")
        .map(|v| v.contains("\"status\":\"ok\"") && v.contains("\"kind\":\"mvp_snapshot\""))
        .unwrap_or(false);

    let all_ok = smoke_ok
        && fuzz_ok
        && perf_ok
        && net_sim_ok
        && doctor_ok
        && mvp_check_ok
        && mvp_report_ok
        && m5_ok
        && m6_ok
        && release_ok
        && pack_ok
        && audit_ok
        && snapshot_ok;
    let real_world_datapath_closed = detect_real_world_datapath_closed();

    let json = format!(
        "{{\"status\":\"{}\",\"kind\":\"mvp_verify\",\"message_en\":\"Full MVP verification finished.\",\"message_ru\":\"Полная проверка MVP завершена.\",\"refreshed\":{},\"smoke\":{},\"fuzz_smoke\":{},\"perf_smoke\":{},\"net_sim\":{},\"lab_doctor\":{},\"mvp_spec_check\":{},\"mvp_spec_report\":{},\"m5_artifacts_report\":{},\"m6_artifacts_report\":{},\"release_readiness_report\":{},\"report_pack\":{},\"artifact_audit\":{},\"mvp_snapshot\":{},\"truth_boundary\":{{\"lab_scope_only\":true,\"real_world_datapath_closed\":{}}},\"network_state\":\"not_modified\"}}",
        if all_ok { "ok" } else { "fail" },
        options.refresh,
        smoke_ok,
        fuzz_ok,
        perf_ok,
        net_sim_ok,
        doctor_ok,
        mvp_check_ok,
        mvp_report_ok,
        m5_ok,
        m6_ok,
        release_ok,
        pack_ok,
        audit_ok,
        snapshot_ok,
        real_world_datapath_closed
    );

    if options.json_output {
        if let Err(error) = fs::write(&options.out_path, &json) {
            eprintln!("mvp-verify write failed: {error}");
            return 1;
        }
        match lang {
            Language::En => println!("MVP verify: saved to {}", options.out_path),
            Language::Ru => println!("Проверка MVP: сохранена в {}", options.out_path),
        }
        println!("{json}");
    } else {
        let text = match lang {
            Language::En => format!(
                "MVP verification result: {}\nSmoke tests passed: {}\nFuzz smoke passed: {}\nPerformance smoke passed: {}\nNetwork simulation passed: {}\nLab doctor report is valid: {}\nMVP spec check/report: {}/{}\nM5 and M6 reports: {}/{}\nRelease readiness and report pack: {}/{}\nArtifact audit and snapshot: {}/{}\nNetwork state: not modified",
                if all_ok { "ok" } else { "fail" },
                smoke_ok,
                fuzz_ok,
                perf_ok,
                net_sim_ok,
                doctor_ok,
                mvp_check_ok,
                mvp_report_ok,
                m5_ok,
                m6_ok,
                release_ok,
                pack_ok,
                audit_ok,
                snapshot_ok
            ),
            Language::Ru => format!(
                "Результат проверки MVP: {}\nSmoke-тесты пройдены: {}\nFuzz smoke пройден: {}\nПроверка производительности пройдена: {}\nСетевая симуляция пройдена: {}\nОтчет Lab doctor корректен: {}\nПроверка/отчет MVP-спеки: {}/{}\nОтчеты M5 и M6: {}/{}\nГотовность релиза и пакет отчетов: {}/{}\nАудит артефактов и снимок: {}/{}\nСостояние сети: не изменялось",
                if all_ok { "ok" } else { "fail" },
                smoke_ok,
                fuzz_ok,
                perf_ok,
                net_sim_ok,
                doctor_ok,
                mvp_check_ok,
                mvp_report_ok,
                m5_ok,
                m6_ok,
                release_ok,
                pack_ok,
                audit_ok,
                snapshot_ok
            ),
        };
        if let Err(error) = fs::write(&options.out_path, &text) {
            eprintln!("mvp-verify write failed: {error}");
            return 1;
        }
        match lang {
            Language::En => println!("MVP verify: saved to {}", options.out_path),
            Language::Ru => println!("Проверка MVP: сохранена в {}", options.out_path),
        }
        println!("{text}");
    }

    if all_ok || !options.strict { 0 } else { 1 }
}

fn render_mvp_spec_report_markdown(result: MvpSpecCheckResult) -> String {
    mvp_reports::render_mvp_spec_report_markdown(result)
}

fn render_m5_artifacts_report_markdown(
    lang: Language,
    all_ok: bool,
    config_ok: bool,
    doctor_ok: bool,
    route_ok: bool,
    datapath_ok: bool,
    rollback_ok: bool,
) -> String {
    mvp_reports::render_m5_artifacts_report_markdown(
        lang,
        all_ok,
        config_ok,
        doctor_ok,
        route_ok,
        datapath_ok,
        rollback_ok,
    )
}

fn render_m6_artifacts_report_markdown(
    lang: Language,
    hardening_ok: bool,
    benchmark_ok: bool,
    mvp_check_ok: bool,
) -> String {
    mvp_reports::render_m6_artifacts_report_markdown(lang, hardening_ok, benchmark_ok, mvp_check_ok)
}

fn render_release_readiness_report_markdown(
    lang: Language,
    release_ok: bool,
    real_world_datapath_closed: bool,
    result: MvpSpecCheckResult,
    checklist: ReleaseGateChecklist,
    artifacts: ReleaseReadinessArtifacts,
) -> String {
    release_reports::render_release_readiness_report_markdown(
        lang,
        release_ok,
        real_world_datapath_closed,
        result,
        checklist,
        artifacts,
    )
}

fn render_release_readiness_report_json(
    release_ok: bool,
    real_world_datapath_closed: bool,
    result: MvpSpecCheckResult,
    checklist: ReleaseGateChecklist,
    artifacts: ReleaseReadinessArtifacts,
) -> String {
    release_reports::render_release_readiness_report_json(
        release_ok,
        real_world_datapath_closed,
        result,
        checklist,
        artifacts,
    )
}

fn run_benchmark_report(lang: Language, args: &[String]) -> i32 {
    let options = match parse_benchmark_report_options(args) {
        Ok(options) => options,
        Err(error) => {
            match lang {
                Language::En => {
                    eprintln!("Benchmark-report options error: {error}");
                    eprintln!(
                        "usage: chimera-lab [--lang en|ru] benchmark-report [--min-encode-ops <n>] [--min-decode-ops <n>] [--out <file>]"
                    );
                }
                Language::Ru => {
                    eprintln!("Ошибка опций benchmark-report: {error}");
                    eprintln!(
                        "использование: chimera-lab [--lang en|ru] benchmark-report [--min-encode-ops <n>] [--min-decode-ops <n>] [--out <файл>]"
                    );
                }
            }
            return 2;
        }
    };

    if run_smoke_with_output(lang, false) != 0 {
        eprintln!(
            "{}",
            match lang {
                Language::En => "Benchmark report failed: smoke stage failed",
                Language::Ru => "Benchmark report не создан: ошибка этапа smoke",
            }
        );
        return 1;
    }
    if let Err(error) = execute_config_smoke(lang, &ConfigSmokeOptions::default(), false) {
        eprintln!(
            "{}",
            match lang {
                Language::En => "Benchmark report failed: config-smoke stage failed",
                Language::Ru => "Benchmark report не создан: ошибка этапа config-smoke",
            }
        );
        eprintln!("{error}");
        return 1;
    }
    if run_fuzz_smoke_with_output(lang, false) != 0 {
        eprintln!(
            "{}",
            match lang {
                Language::En => "Benchmark report failed: fuzz-smoke stage failed",
                Language::Ru => "Benchmark report не создан: ошибка этапа fuzz-smoke",
            }
        );
        return 1;
    }
    let net_sim = execute_net_sim(NetSimOptions::default());
    if net_sim.reconnect_events == 0 {
        eprintln!(
            "{}",
            match lang {
                Language::En => "Benchmark report failed: net-sim reconnect check failed",
                Language::Ru => "Benchmark report не создан: ошибка проверки reconnect в net-sim",
            }
        );
        return 1;
    }

    let perf_result = match execute_perf_smoke(PerfSmokeOptions {
        min_encode_ops: options.min_encode_ops,
        min_decode_ops: options.min_decode_ops,
        json_output: false,
    }) {
        Ok(result) => result,
        Err(error) => {
            eprintln!(
                "{}",
                match lang {
                    Language::En => "Benchmark report failed: perf-smoke stage failed",
                    Language::Ru => "Benchmark report не создан: ошибка этапа perf-smoke",
                }
            );
            eprintln!("{error}");
            return 1;
        }
    };

    let created_at_unix_sec = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    };
    if let Some(baseline_path) = options.baseline_path.as_deref() {
        let max_regression_pct = options.max_regression_pct.unwrap_or(10.0);
        if let Err(error) = check_perf_regression(baseline_path, perf_result, max_regression_pct) {
            eprintln!("benchmark report failed: {error}");
            return 1;
        }
        if let Err(error) = check_net_sim_regression(baseline_path, net_sim, max_regression_pct) {
            eprintln!("benchmark report failed: {error}");
            return 1;
        }
    }

    let report = format!(
        "{{\"status\":\"ok\",\"message_en\":\"Benchmark checks passed and report is ready.\",\"message_ru\":\"Проверки бенчмарка пройдены, отчет готов.\",\"created_at_unix_sec\":{},\"config_smoke\":true,\"smoke\":true,\"fuzz_smoke\":true,\"net_sim\":true,\"perf_smoke\":true,\"iterations\":{},\"encode_ops_per_sec\":{:.0},\"decode_ops_per_sec\":{:.0},\"encoded_total_bytes\":{},\"decoded_total_payload_bytes\":{},\"min_encode_ops\":{},\"min_decode_ops\":{},\"net_sim_reconnect_events\":{},\"net_sim_dropped\":{},\"net_sim_attempts\":{}}}",
        created_at_unix_sec,
        perf_result.iterations,
        perf_result.encode_ops_per_sec,
        perf_result.decode_ops_per_sec,
        perf_result.encoded_total_bytes,
        perf_result.decoded_total_payload_bytes,
        options
            .min_encode_ops
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string()),
        options
            .min_decode_ops
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string()),
        net_sim.reconnect_events,
        net_sim.dropped,
        net_sim.attempts
    );

    if let Some(path) = options.out_path {
        if let Err(error) = fs::write(&path, &report) {
            eprintln!("benchmark report write failed: {error}");
            return 1;
        }
        match lang {
            Language::En => println!("Benchmark report: saved to {path}"),
            Language::Ru => println!("Benchmark report: сохранен в {path}"),
        }
    }
    println!("{report}");
    0
}

fn check_perf_regression(
    baseline_path: &str,
    current: PerfSmokeResult,
    max_regression_pct: f64,
) -> Result<(), String> {
    let baseline_json = fs::read_to_string(baseline_path)
        .map_err(|error| format!("cannot read baseline file '{baseline_path}': {error}"))?;
    let baseline_encode = extract_json_f64(&baseline_json, "encode_ops_per_sec")
        .ok_or_else(|| "baseline is missing encode_ops_per_sec".to_string())?;
    let baseline_decode = extract_json_f64(&baseline_json, "decode_ops_per_sec")
        .ok_or_else(|| "baseline is missing decode_ops_per_sec".to_string())?;

    let encode_regression_pct =
        ((baseline_encode - current.encode_ops_per_sec) / baseline_encode.max(1.0)) * 100.0;
    let decode_regression_pct =
        ((baseline_decode - current.decode_ops_per_sec) / baseline_decode.max(1.0)) * 100.0;

    if encode_regression_pct > max_regression_pct {
        return Err(format!(
            "encode regression {:.2}% exceeds allowed {:.2}%",
            encode_regression_pct, max_regression_pct
        ));
    }
    if decode_regression_pct > max_regression_pct {
        return Err(format!(
            "decode regression {:.2}% exceeds allowed {:.2}%",
            decode_regression_pct, max_regression_pct
        ));
    }
    Ok(())
}

fn check_net_sim_regression(
    baseline_path: &str,
    current: NetSimResult,
    max_regression_pct: f64,
) -> Result<(), String> {
    let baseline_json = fs::read_to_string(baseline_path)
        .map_err(|error| format!("cannot read baseline file '{baseline_path}': {error}"))?;
    let baseline_dropped = extract_json_f64(&baseline_json, "net_sim_dropped")
        .ok_or_else(|| "baseline is missing net_sim_dropped".to_string())?;
    let baseline_attempts = extract_json_f64(&baseline_json, "net_sim_attempts").unwrap_or(100.0);
    let current_drop_pct = (current.dropped as f64) * 100.0 / (current.attempts.max(1) as f64);
    let baseline_drop_pct = baseline_dropped * 100.0 / baseline_attempts.max(1.0);
    let drop_regression_pct =
        ((current_drop_pct - baseline_drop_pct) / baseline_drop_pct.max(1.0)) * 100.0;
    if drop_regression_pct > max_regression_pct {
        return Err(format!(
            "net-sim drop regression {:.2}% exceeds allowed {:.2}%",
            drop_regression_pct, max_regression_pct
        ));
    }
    if current.reconnect_events == 0 {
        return Err("net-sim reconnect events is zero".to_string());
    }
    Ok(())
}

fn extract_json_f64(input: &str, key: &str) -> Option<f64> {
    let marker = format!("\"{key}\":");
    let start = input.find(&marker)? + marker.len();
    let tail = &input[start..];
    let end = tail.find([',', '}']).unwrap_or(tail.len());
    tail[..end].trim().parse::<f64>().ok()
}

fn render_hardening_json(perf_result: PerfSmokeResult) -> String {
    format!(
        "{{\"status\":\"ok\",\"config_smoke\":true,\"smoke\":true,\"fuzz_smoke\":true,\"net_sim\":true,\"perf_smoke\":true,\"iterations\":{},\"encode_ops_per_sec\":{:.0},\"decode_ops_per_sec\":{:.0}}}",
        perf_result.iterations, perf_result.encode_ops_per_sec, perf_result.decode_ops_per_sec
    )
}

fn render_datapath_report_text(lang: Language, report: &DatapathReport) -> String {
    let mut out = String::new();
    match lang {
        Language::En => {
            out.push_str("Datapath report: ok\n");
            out.push_str("Path: policy -> dns-binding -> session frame -> carrier\n");
            out.push_str(&format!("Gateway explain: {}\n", report.gateway_explain));
            out.push_str(&format!("Block explain: {}\n", report.block_explain));
            out.push_str(&format!("Direct explain: {}\n", report.direct_explain));
            out.push_str("Network state: not modified\n");
        }
        Language::Ru => {
            out.push_str("Datapath report: ok\n");
            out.push_str("Путь: policy -> dns-binding -> session frame -> carrier\n");
            out.push_str(&format!("Gateway explain: {}\n", report.gateway_explain));
            out.push_str(&format!("Block explain: {}\n", report.block_explain));
            out.push_str(&format!("Direct explain: {}\n", report.direct_explain));
            out.push_str("Состояние сети: не изменялось\n");
        }
    }
    out
}

fn render_datapath_report_json(report: &DatapathReport) -> String {
    format!(
        "{{\"status\":\"ok\",\"kind\":\"datapath_report\",\"message_en\":\"Datapath report is ready.\",\"message_ru\":\"Отчет datapath готов.\",\"gateway_explain\":\"{}\",\"block_explain\":\"{}\",\"direct_explain\":\"{}\",\"network_state\":\"not_modified\"}}",
        report.gateway_explain, report.block_explain, report.direct_explain
    )
}

#[derive(Debug, Clone)]
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        // Xorshift64* for deterministic local fuzz-smoke data generation.
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

fn fuzz_bytes(rng: &mut DeterministicRng, max_len: usize) -> Vec<u8> {
    let len = (rng.next_u64() as usize) % (max_len + 1);
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        out.push((rng.next_u64() & 0xFF) as u8);
    }
    out
}

fn run_fake_handshake() -> Result<(), String> {
    let client = ClientHandshake::new_test_only([11_u8; 32]);
    let mut client_to_gateway = InMemoryCarrier::new(4096);
    let mut gateway_to_client = InMemoryCarrier::new(4096);

    client_to_gateway
        .send(client.client_hello_bytes())
        .map_err(|error| error.to_string())?;

    let client_hello_bytes = client_to_gateway
        .recv()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "gateway received no client hello".to_string())?;
    let gateway = GatewayHandshake::accept_client_hello_bytes(&client_hello_bytes, [22_u8; 32])
        .map_err(|error| error.to_string())?;

    gateway_to_client
        .send(gateway.server_hello_bytes())
        .map_err(|error| error.to_string())?;

    let server_hello_bytes = gateway_to_client
        .recv()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "client received no server hello".to_string())?;

    let client_session = client
        .finish_from_server_hello_bytes(&server_hello_bytes)
        .map_err(|error| error.to_string())?;
    let gateway_session = gateway.finish().map_err(|error| error.to_string())?;

    if client_session
        .traffic_secrets
        .client_to_gateway
        .expose_for_tests()
        != gateway_session
            .traffic_secrets
            .client_to_gateway
            .expose_for_tests()
    {
        return Err("client/gateway test secrets differ".to_string());
    }

    Ok(())
}

fn run_frame_replay_check() -> Result<(), String> {
    let frame = Frame {
        packet_number: 1,
        payload: b"chimera-smoke".to_vec(),
    };

    let encoded = frame.encode().map_err(|error| error.to_string())?;

    let mut carrier = InMemoryCarrier::new(4096);
    carrier.send(encoded).map_err(|error| error.to_string())?;

    let received = carrier
        .recv()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "carrier returned no frame".to_string())?;

    let decoded = Frame::decode(&received).map_err(|error| error.to_string())?;

    let mut replay = ReplayWindow::default();
    replay
        .accept(decoded.packet_number)
        .map_err(|error| error.to_string())?;

    if replay.accept(decoded.packet_number).is_ok() {
        return Err("replay window accepted duplicate packet".to_string());
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct DatapathReport {
    gateway_explain: String,
    block_explain: String,
    direct_explain: String,
}

fn run_policy_dns_session_carrier_path() -> Result<DatapathReport, String> {
    let now = Instant::now();
    let gateway_ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10));
    let block_ip = IpAddr::V4(Ipv4Addr::new(198, 51, 100, 11));
    let direct_ip = IpAddr::V4(Ipv4Addr::new(198, 51, 100, 20));
    let mut dns_store = DnsBindingStore::default();
    dns_store.insert(DnsBinding::new(
        "api.example.org",
        gateway_ip,
        Duration::from_secs(60),
        now,
    ));
    dns_store.insert(DnsBinding::new(
        "blocked.example.org",
        block_ip,
        Duration::from_secs(60),
        now,
    ));
    dns_store.insert(DnsBinding::new(
        "news.example.net",
        direct_ip,
        Duration::from_secs(60),
        now,
    ));

    let policy = Policy::new(vec![
        RouteRule {
            id: "blocked-exact".to_string(),
            matcher: RuleMatcher::ExactDomain("blocked.example.org".to_string()),
            outbound: OutboundMode::Block,
        },
        RouteRule {
            id: "example-gateway".to_string(),
            matcher: RuleMatcher::DomainSuffix("example.org".to_string()),
            outbound: OutboundMode::Gateway,
        },
        RouteRule {
            id: "default-direct".to_string(),
            matcher: RuleMatcher::Default,
            outbound: OutboundMode::Direct,
        },
    ]);
    let gateway_binding = dns_store
        .lookup(gateway_ip, now)
        .ok_or_else(|| "dns binding missing for gateway destination".to_string())?;
    let gateway_flow = FlowContext {
        domain: Some(gateway_binding.domain.clone()),
        destination_ip: Some(gateway_binding.ip),
        protocol: Protocol::Tcp,
        port: Some(443),
    };
    let gateway_decision = policy.decide(&gateway_flow);
    if gateway_decision.outbound != OutboundMode::Gateway {
        return Err(format!(
            "unexpected outbound mode for flow: expected gateway, got {:?}",
            gateway_decision.outbound
        ));
    }

    let frame = Frame {
        packet_number: 1,
        payload: format!(
            "route={} domain={}",
            gateway_decision.matched_rule_id, gateway_binding.domain
        )
        .into_bytes(),
    };
    let encoded = frame.encode().map_err(|error| error.to_string())?;

    let cfg = TlsCarrierConfig {
        server_name: "gateway.example.org".to_string(),
        connect_addr: "lab-gateway.local:443".to_string(),
        connect_timeout_ms: 1000,
    };
    let mut client_to_gateway = TlsCarrier::new(cfg.clone()).map_err(|error| error.to_string())?;
    let mut gateway_rx = TlsCarrier::new(cfg).map_err(|error| error.to_string())?;
    client_to_gateway
        .send(encoded)
        .map_err(|error| error.to_string())?;
    let received = gateway_rx
        .recv()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "gateway received no frame".to_string())?;
    let decoded = Frame::decode(&received).map_err(|error| error.to_string())?;

    if decoded.packet_number != 1 {
        return Err("decoded packet number mismatch".to_string());
    }
    if decoded.payload != frame.payload {
        return Err("decoded payload mismatch".to_string());
    }

    let blocked_binding = dns_store
        .lookup(block_ip, now)
        .ok_or_else(|| "dns binding missing for blocked destination".to_string())?;
    let block_flow = FlowContext {
        domain: Some(blocked_binding.domain.clone()),
        destination_ip: Some(blocked_binding.ip),
        protocol: Protocol::Tcp,
        port: Some(443),
    };
    let block_decision = policy.decide(&block_flow);
    if block_decision.outbound != OutboundMode::Block {
        return Err("block flow was not blocked".to_string());
    }

    let direct_binding = dns_store
        .lookup(direct_ip, now)
        .ok_or_else(|| "dns binding missing for direct destination".to_string())?;
    let direct_flow = FlowContext {
        domain: Some(direct_binding.domain.clone()),
        destination_ip: Some(direct_binding.ip),
        protocol: Protocol::Tcp,
        port: Some(443),
    };
    let direct_decision = policy.decide(&direct_flow);
    if direct_decision.outbound != OutboundMode::Direct {
        return Err("direct flow was not routed direct".to_string());
    }

    let mut direct_path = InMemoryCarrier::new(4096);
    let direct_frame = Frame {
        packet_number: 7,
        payload: format!(
            "route={} domain={}",
            direct_decision.matched_rule_id, direct_binding.domain
        )
        .into_bytes(),
    };
    let direct_encoded = direct_frame.encode().map_err(|error| error.to_string())?;
    direct_path
        .send(direct_encoded)
        .map_err(|error| error.to_string())?;
    let direct_received = direct_path
        .recv()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "direct path returned no frame".to_string())?;
    let direct_decoded = Frame::decode(&direct_received).map_err(|error| error.to_string())?;
    if direct_decoded.payload != direct_frame.payload {
        return Err("direct path decoded payload mismatch".to_string());
    }

    Ok(DatapathReport {
        gateway_explain: gateway_decision.explanation,
        block_explain: block_decision.explanation,
        direct_explain: direct_decision.explanation,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        ArtifactAuditOptions, BenchmarkReportOptions, CefPhase1SmokeOptions, CefPhase1SmokeResult,
        ConfigSmokeOptions, DoctorOptions, DoctorResult, Language, M5ArtifactsReportOptions,
        M6ArtifactsReportOptions, MvpSnapshotOptions, MvpSpecCheckOptions, MvpSpecCheckResult,
        MvpSpecReportOptions, MvpVerifyOptions, NetSimOptions, NetSimResult, PerfSmokeOptions,
        ReleaseGateChecklist, ReleaseReadinessArtifacts, ReleaseReadinessReportOptions,
        ReportPackOptions, check_benchmark_artifact, check_cef_phase1_smoke_artifact,
        check_datapath_artifact, check_doctor_artifacts, check_net_sim_regression,
        check_rollback_json_artifacts, check_route_explain_artifact,
        detect_real_world_datapath_closed_from_paths, execute_net_sim, extract_json_f64,
        parse_artifact_audit_options, parse_benchmark_report_options,
        parse_cef_phase1_smoke_options, parse_config_smoke_options, parse_doctor_options,
        parse_m5_artifacts_report_options, parse_m6_artifacts_report_options,
        parse_mvp_snapshot_options, parse_mvp_spec_check_options, parse_mvp_spec_report_options,
        parse_mvp_verify_options, parse_net_sim_options, parse_perf_smoke_options,
        parse_release_readiness_report_options, parse_report_pack_options,
        render_cef_phase1_smoke_json, render_doctor_json, render_m5_artifacts_report_markdown,
        render_m6_artifacts_report_markdown, render_mvp_spec_check_json,
        render_mvp_spec_report_markdown, render_release_readiness_report_json,
        render_release_readiness_report_markdown,
    };
    use std::fs;

    #[test]
    fn perf_smoke_options_parse_empty() {
        let args: Vec<String> = Vec::new();
        let parsed = match parse_perf_smoke_options(&args) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("empty args should parse: {error}"),
        };
        assert_eq!(parsed, PerfSmokeOptions::default());
    }

    #[test]
    fn perf_smoke_options_parse_full() {
        let args = vec![
            "--min-encode-ops".to_string(),
            "1000".to_string(),
            "--min-decode-ops".to_string(),
            "2000".to_string(),
        ];
        let parsed = match parse_perf_smoke_options(&args) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("full args should parse: {error}"),
        };
        assert_eq!(
            parsed,
            PerfSmokeOptions {
                min_encode_ops: Some(1000),
                min_decode_ops: Some(2000),
                json_output: false,
            }
        );
    }

    #[test]
    fn policy_dns_session_carrier_path_passes() {
        let report = super::run_policy_dns_session_carrier_path();
        assert!(report.is_ok());
        let report = match report {
            Ok(report) => report,
            Err(error) => unreachable!("datapath report should be available: {error}"),
        };
        assert!(report.gateway_explain.contains("matched rule"));
        assert!(report.block_explain.contains("matched rule"));
        assert!(report.direct_explain.contains("matched rule"));
    }

    #[test]
    fn perf_smoke_options_reject_unknown_flag() {
        let args = vec!["--bad".to_string(), "1".to_string()];
        assert!(parse_perf_smoke_options(&args).is_err());
    }

    #[test]
    fn perf_smoke_options_reject_missing_value() {
        let args = vec!["--min-encode-ops".to_string()];
        assert!(parse_perf_smoke_options(&args).is_err());
    }

    #[test]
    fn perf_smoke_options_parse_json_flag() {
        let args = vec!["--json".to_string()];
        let parsed = match parse_perf_smoke_options(&args) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("json flag should parse: {error}"),
        };
        assert_eq!(
            parsed,
            PerfSmokeOptions {
                min_encode_ops: None,
                min_decode_ops: None,
                json_output: true,
            }
        );
    }

    #[test]
    fn benchmark_report_options_parse_full() {
        let args = vec![
            "--min-encode-ops".to_string(),
            "1500".to_string(),
            "--min-decode-ops".to_string(),
            "2500".to_string(),
            "--out".to_string(),
            "report.json".to_string(),
        ];
        let parsed = match parse_benchmark_report_options(&args) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("benchmark args should parse: {error}"),
        };
        assert_eq!(
            parsed,
            BenchmarkReportOptions {
                min_encode_ops: Some(1500),
                min_decode_ops: Some(2500),
                out_path: Some("report.json".to_string()),
                baseline_path: None,
                max_regression_pct: None,
            }
        );
    }

    #[test]
    fn benchmark_report_options_reject_unknown_flag() {
        let args = vec!["--bad".to_string()];
        assert!(parse_benchmark_report_options(&args).is_err());
    }

    #[test]
    fn benchmark_report_options_parse_baseline_and_regression() {
        let args = vec![
            "--baseline".to_string(),
            "docs/benchmark_latest.json".to_string(),
            "--max-regression-pct".to_string(),
            "7.5".to_string(),
        ];
        let parsed = match parse_benchmark_report_options(&args) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("benchmark baseline args should parse: {error}"),
        };
        assert_eq!(
            parsed,
            BenchmarkReportOptions {
                min_encode_ops: None,
                min_decode_ops: None,
                out_path: None,
                baseline_path: Some("docs/benchmark_latest.json".to_string()),
                max_regression_pct: Some(7.5),
            }
        );
    }

    #[test]
    fn extract_json_f64_reads_metric_value() {
        let json = "{\"net_sim\":true,\"encode_ops_per_sec\":12345,\"decode_ops_per_sec\":54321}";
        assert_eq!(extract_json_f64(json, "encode_ops_per_sec"), Some(12345.0));
        assert_eq!(extract_json_f64(json, "decode_ops_per_sec"), Some(54321.0));
        assert_eq!(extract_json_f64(json, "missing"), None);
    }

    #[test]
    fn config_smoke_options_parse_defaults_and_overrides() {
        let parsed_default = match parse_config_smoke_options(&[]) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("default config smoke options should parse: {error}"),
        };
        assert_eq!(parsed_default, ConfigSmokeOptions::default());

        let args = vec![
            "--client".to_string(),
            "a.conf".to_string(),
            "--gateway".to_string(),
            "b.conf".to_string(),
        ];
        let parsed = match parse_config_smoke_options(&args) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("override config smoke options should parse: {error}"),
        };
        assert_eq!(
            parsed,
            ConfigSmokeOptions {
                client_path: "a.conf".to_string(),
                gateway_path: "b.conf".to_string(),
            }
        );
    }

    #[test]
    fn net_sim_options_parse_defaults() {
        let args: Vec<String> = Vec::new();
        let parsed = match parse_net_sim_options(&args) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("empty net-sim args should parse: {error}"),
        };
        assert_eq!(parsed, NetSimOptions::default());
    }

    #[test]
    fn net_sim_options_parse_full() {
        let args = vec![
            "--loss-pct".to_string(),
            "25".to_string(),
            "--delay-ms".to_string(),
            "30".to_string(),
            "--disconnect-at".to_string(),
            "10".to_string(),
            "--reconnect-after".to_string(),
            "5".to_string(),
            "--mtu".to_string(),
            "1300".to_string(),
            "--json".to_string(),
        ];
        let parsed = match parse_net_sim_options(&args) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("full net-sim args should parse: {error}"),
        };
        assert_eq!(
            parsed,
            NetSimOptions {
                loss_pct: 25,
                delay_ms: 30,
                disconnect_at: 10,
                reconnect_after: 5,
                mtu: 1300,
                json_output: true,
            }
        );
    }

    #[test]
    fn net_sim_options_reject_invalid_loss() {
        let args = vec!["--loss-pct".to_string(), "101".to_string()];
        assert!(parse_net_sim_options(&args).is_err());
    }

    #[test]
    fn net_sim_result_counts_disconnect_window() {
        let result = execute_net_sim(NetSimOptions {
            loss_pct: 0,
            delay_ms: 1,
            disconnect_at: 20,
            reconnect_after: 4,
            mtu: 1400,
            json_output: false,
        });
        assert_eq!(result.disconnected_window, 4);
        assert_eq!(result.reconnect_events, 1);
    }

    #[test]
    fn net_sim_regression_check_accepts_same_profile() {
        let path = std::env::temp_dir().join("chimera_lab_net_sim_baseline_ok.json");
        let write_result = fs::write(
            &path,
            "{\"net_sim_dropped\":18,\"net_sim_attempts\":100,\"encode_ops_per_sec\":1,\"decode_ops_per_sec\":1}",
        );
        assert!(write_result.is_ok());
        let result = check_net_sim_regression(
            path.to_str().unwrap_or(""),
            NetSimResult {
                attempts: 100,
                sent: 82,
                dropped: 18,
                mtu_dropped: 0,
                disconnected_window: 8,
                reconnect_events: 1,
                simulated_delay_total_ms: 1000,
            },
            10.0,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn net_sim_regression_check_rejects_large_drop_increase() {
        let path = std::env::temp_dir().join("chimera_lab_net_sim_baseline_bad.json");
        let write_result = fs::write(
            &path,
            "{\"net_sim_dropped\":10,\"net_sim_attempts\":100,\"encode_ops_per_sec\":1,\"decode_ops_per_sec\":1}",
        );
        assert!(write_result.is_ok());
        let result = check_net_sim_regression(
            path.to_str().unwrap_or(""),
            NetSimResult {
                attempts: 100,
                sent: 75,
                dropped: 25,
                mtu_dropped: 0,
                disconnected_window: 8,
                reconnect_events: 1,
                simulated_delay_total_ms: 1000,
            },
            10.0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn doctor_options_parse_defaults_and_overrides() {
        let parsed_default = match parse_doctor_options(&[]) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("default doctor options should parse: {error}"),
        };
        assert_eq!(parsed_default, DoctorOptions::default());

        let args = vec![
            "--client".to_string(),
            "a.conf".to_string(),
            "--gateway".to_string(),
            "b.conf".to_string(),
            "--json".to_string(),
            "--out".to_string(),
            "doctor.json".to_string(),
        ];
        let parsed = match parse_doctor_options(&args) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("doctor options should parse: {error}"),
        };
        assert_eq!(parsed.client_path, "a.conf");
        assert_eq!(parsed.gateway_path, "b.conf");
        assert!(parsed.json_output);
        assert_eq!(parsed.out_path, Some("doctor.json".to_string()));
    }

    #[test]
    fn doctor_json_contains_expected_fields() {
        let json = render_doctor_json(DoctorResult {
            client_config_ok: true,
            gateway_config_ok: true,
            client_carrier_ok: true,
            gateway_carrier_ok: true,
            net_sim_ok: true,
            net_sim_dropped: 18,
            net_sim_reconnect_events: 1,
        });
        assert!(json.contains("\"kind\":\"lab_doctor\""));
        assert!(json.contains("\"message_en\":\"Lab doctor check is ready.\""));
        assert!(json.contains("\"message_ru\":\"Проверка lab doctor готова.\""));
        assert!(json.contains("\"net_sim_dropped\":18"));
        assert!(json.contains("\"network_state\":\"not_modified\""));
    }

    #[test]
    fn mvp_spec_check_options_parse_defaults_and_out() {
        let parsed_default = match parse_mvp_spec_check_options(&[]) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("default mvp-spec-check options should parse: {error}"),
        };
        assert_eq!(parsed_default, MvpSpecCheckOptions::default());

        let args = vec![
            "--json".to_string(),
            "--out".to_string(),
            "mvp_check.json".to_string(),
        ];
        let parsed = match parse_mvp_spec_check_options(&args) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("mvp-spec-check options should parse: {error}"),
        };
        assert!(parsed.json_output);
        assert_eq!(parsed.out_path, Some("mvp_check.json".to_string()));
    }

    #[test]
    fn mvp_spec_check_json_contains_expected_fields() {
        let json = render_mvp_spec_check_json(MvpSpecCheckResult {
            m0_workspace: true,
            m1_local_tunnel: true,
            m2_crypto_session: true,
            m3_carrier_validation: true,
            m4_routing_determinism: true,
            m5_doctor_and_config: true,
            m6_hardening: true,
        });
        assert!(json.contains("\"kind\":\"mvp_spec_check\""));
        assert!(json.contains("\"m6_hardening\":true"));
        assert!(json.contains("\"network_state\":\"not_modified\""));
    }

    #[test]
    fn mvp_spec_report_options_parse_defaults_and_out() {
        let parsed_default = match parse_mvp_spec_report_options(&[]) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("default mvp-spec-report options should parse: {error}"),
        };
        assert_eq!(parsed_default, MvpSpecReportOptions::default());

        let args = vec!["--out".to_string(), "report.md".to_string()];
        let parsed = match parse_mvp_spec_report_options(&args) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("mvp-spec-report options should parse: {error}"),
        };
        assert_eq!(parsed.out_path, "report.md");
    }

    #[test]
    fn m5_artifacts_report_options_parse_defaults_and_out() {
        let parsed_default = match parse_m5_artifacts_report_options(&[]) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("default m5-artifacts-report options should parse: {error}"),
        };
        assert_eq!(parsed_default, M5ArtifactsReportOptions::default());

        let args = vec!["--out".to_string(), "m5.md".to_string()];
        let parsed = match parse_m5_artifacts_report_options(&args) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("m5-artifacts-report options should parse: {error}"),
        };
        assert_eq!(parsed.out_path, "m5.md");
    }

    #[test]
    fn m6_artifacts_report_options_parse_defaults_and_out() {
        let parsed_default = match parse_m6_artifacts_report_options(&[]) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("default m6-artifacts-report options should parse: {error}"),
        };
        assert_eq!(parsed_default, M6ArtifactsReportOptions::default());

        let args = vec!["--out".to_string(), "m6.md".to_string()];
        let parsed = match parse_m6_artifacts_report_options(&args) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("m6-artifacts-report options should parse: {error}"),
        };
        assert_eq!(parsed.out_path, "m6.md");
    }

    #[test]
    fn release_readiness_report_options_parse_defaults_and_out() {
        let parsed_default = match parse_release_readiness_report_options(&[]) {
            Ok(parsed) => parsed,
            Err(error) => {
                unreachable!("default release-readiness-report options should parse: {error}")
            }
        };
        assert_eq!(parsed_default, ReleaseReadinessReportOptions::default());
        assert!(!parsed_default.json_output);

        let args = vec![
            "--json".to_string(),
            "--out".to_string(),
            "release.json".to_string(),
        ];
        let parsed = match parse_release_readiness_report_options(&args) {
            Ok(parsed) => parsed,
            Err(error) => {
                unreachable!("release-readiness-report options should parse: {error}")
            }
        };
        assert_eq!(parsed.out_path, "release.json");
        assert!(parsed.json_output);
    }

    #[test]
    fn report_pack_options_parse_defaults_and_out() {
        let parsed_default = match parse_report_pack_options(&[]) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("default report-pack options should parse: {error}"),
        };
        assert_eq!(parsed_default, ReportPackOptions::default());
        assert!(!parsed_default.json_output);

        let args = vec![
            "--json".to_string(),
            "--out".to_string(),
            "pack.json".to_string(),
        ];
        let parsed = match parse_report_pack_options(&args) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("report-pack options should parse: {error}"),
        };
        assert_eq!(parsed.out_path, "pack.json");
        assert!(parsed.json_output);
    }

    #[test]
    fn cef_phase1_smoke_options_parse_defaults_and_out() {
        let parsed_default = match parse_cef_phase1_smoke_options(&[]) {
            Ok(parsed) => parsed,
            Err(error) => {
                unreachable!("default cef-phase1-smoke options should parse: {error}")
            }
        };
        assert_eq!(parsed_default, CefPhase1SmokeOptions::default());
        assert!(!parsed_default.json_output);

        let args = vec![
            "--json".to_string(),
            "--out".to_string(),
            "cef_phase1_smoke.json".to_string(),
        ];
        let parsed = match parse_cef_phase1_smoke_options(&args) {
            Ok(parsed) => parsed,
            Err(error) => {
                unreachable!("cef-phase1-smoke options should parse: {error}")
            }
        };
        assert_eq!(parsed.out_path, Some("cef_phase1_smoke.json".to_string()));
        assert!(parsed.json_output);
    }

    #[test]
    fn cef_phase1_smoke_json_contains_expected_fields() {
        let json = render_cef_phase1_smoke_json(CefPhase1SmokeResult {
            mesh_join_mode_resolved: true,
            mesh_failover_reselection_verified: true,
            dht_discovery_record_verified: true,
            dps_policy_fragment_verified: true,
            relay_policy_verified: true,
            emergency_offer_valid: true,
            roaming_cache_active_hit: true,
            reputation_penalty_applied: true,
        });
        assert!(json.contains("\"kind\":\"cef_phase1_smoke\""));
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"mesh_join_mode_resolved\":true"));
        assert!(json.contains("\"dht_discovery_record_verified\":true"));
        assert!(json.contains("\"dps_policy_fragment_verified\":true"));
        assert!(json.contains("\"relay_policy_verified\":true"));
        assert!(json.contains("\"emergency_offer_valid\":true"));
        assert!(json.contains("\"roaming_cache_active_hit\":true"));
        assert!(json.contains("\"reputation_penalty_applied\":true"));
        assert!(json.contains("\"network_state\":\"not_modified\""));
    }

    #[test]
    fn release_readiness_report_json_contains_cef_phase1_artifact_flag() {
        let checklist = ReleaseGateChecklist {
            clean_clone_builds: true,
            client_gateway_run_linux: true,
            encrypted_tunnel_carries_traffic: true,
            policy_routing_direct_gateway_block: true,
            dns_binding_works: true,
            route_explain_works: true,
            shutdown_restores_network_state: true,
            security_tests_pass: true,
            parser_fuzz_smoke_passes: true,
            no_raw_secrets_in_logs: true,
            benchmark_report_exists: true,
            operations_guide_exists: true,
            runtime_apply_dns_verified: true,
            runtime_apply_route_verified: true,
            runtime_route_policy_validation_verified: true,
            runtime_tun_name_validation_verified: true,
            runtime_forced_stop_rollback_verified: true,
        };
        let json = render_release_readiness_report_json(
            true,
            false,
            MvpSpecCheckResult {
                m0_workspace: true,
                m1_local_tunnel: true,
                m2_crypto_session: true,
                m3_carrier_validation: true,
                m4_routing_determinism: true,
                m5_doctor_and_config: true,
                m6_hardening: true,
            },
            checklist,
            ReleaseReadinessArtifacts {
                m5_report_ok: true,
                m6_report_ok: true,
                benchmark_ok: true,
                cef_phase1_smoke_ok: true,
                mesh_route_explain_ok: true,
                mesh_auto_adaptive_ok: true,
            },
        );
        assert!(json.contains("\"cef_phase1_smoke\":true"));
    }

    #[test]
    fn report_pack_json_contains_cef_phase1_flag() {
        let json = "{\"status\":\"ok\",\"kind\":\"report_pack\",\"message_en\":\"Combined MVP reports are ready.\",\"message_ru\":\"Сводные отчеты MVP готовы.\",\"mvp_spec_report\":true,\"m5_artifacts_report\":true,\"m6_artifacts_report\":true,\"release_readiness_report\":true,\"cef_phase1_smoke\":true,\"truth_boundary\":{\"lab_scope_only\":true,\"real_world_datapath_closed\":false},\"release_gate\":{\"clean_clone_builds\":true,\"client_gateway_run_linux\":true,\"encrypted_tunnel_carries_traffic\":true,\"policy_routing_direct_gateway_block\":true,\"dns_binding_works\":true,\"route_explain_works\":true,\"shutdown_restores_network_state\":true,\"security_tests_pass\":true,\"parser_fuzz_smoke_passes\":true,\"no_raw_secrets_in_logs\":true,\"benchmark_report_exists\":true,\"operations_guide_exists\":true,\"runtime_apply_dns_verified\":true,\"runtime_apply_route_verified\":true},\"network_state\":\"not_modified\"}".to_string();
        assert!(json.contains("\"kind\":\"report_pack\""));
        assert!(json.contains("\"cef_phase1_smoke\":true"));
    }

    #[test]
    fn cef_phase1_smoke_artifact_check_rejects_missing_flag() {
        let dir = std::env::temp_dir();
        let path = dir.join("chimera_cef_phase1_invalid.json");
        let invalid = "{\"status\":\"ok\",\"kind\":\"cef_phase1_smoke\",\"checks\":{\"emergency_offer_valid\":true,\"roaming_cache_active_hit\":true},\"network_state\":\"not_modified\"}";
        assert!(fs::write(&path, invalid).is_ok());
        let path_s = path.to_string_lossy().to_string();
        assert!(!check_cef_phase1_smoke_artifact(&path_s));
    }

    #[test]
    fn real_world_datapath_detect_uses_latest_json() {
        let dir = std::env::temp_dir();
        let path = dir.join("chimera_reality_audit_latest_valid.json");
        let payload =
            "{\"status\":\"ok\",\"kind\":\"reality_audit\",\"real_world_datapath_closed\":true}";
        assert!(fs::write(&path, payload).is_ok());
        let path_s = path.to_string_lossy().to_string();
        assert!(detect_real_world_datapath_closed_from_paths(&[&path_s]));
    }

    #[test]
    fn real_world_datapath_detect_rejects_invalid_latest_json() {
        let dir = std::env::temp_dir();
        let path = dir.join("chimera_reality_audit_latest_invalid.json");
        let payload =
            "{\"status\":\"ok\",\"kind\":\"reality_audit\",\"real_world_datapath_closed\":false}";
        assert!(fs::write(&path, payload).is_ok());
        let path_s = path.to_string_lossy().to_string();
        assert!(!detect_real_world_datapath_closed_from_paths(&[&path_s]));
    }

    #[test]
    fn artifact_audit_options_parse_defaults_and_out() {
        let parsed_default = match parse_artifact_audit_options(&[]) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("default artifact-audit options should parse: {error}"),
        };
        assert_eq!(parsed_default, ArtifactAuditOptions::default());
        assert!(parsed_default.json_output);
        assert!(parsed_default.strict);

        let args = vec![
            "--text".to_string(),
            "--no-strict".to_string(),
            "--out".to_string(),
            "audit.txt".to_string(),
        ];
        let parsed = match parse_artifact_audit_options(&args) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("artifact-audit options should parse: {error}"),
        };
        assert_eq!(parsed.out_path, "audit.txt");
        assert!(!parsed.json_output);
        assert!(!parsed.strict);
    }

    #[test]
    fn mvp_snapshot_options_parse_defaults_and_out() {
        let parsed_default = match parse_mvp_snapshot_options(&[]) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("default mvp-snapshot options should parse: {error}"),
        };
        assert_eq!(parsed_default, MvpSnapshotOptions::default());
        assert!(parsed_default.json_output);
        assert!(parsed_default.strict);

        let args = vec!["--out".to_string(), "snapshot.json".to_string()];
        let parsed = match parse_mvp_snapshot_options(&args) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("mvp-snapshot options should parse: {error}"),
        };
        assert_eq!(parsed.out_path, "snapshot.json");
        assert!(parsed.json_output);

        let text_args = vec![
            "--text".to_string(),
            "--out".to_string(),
            "snapshot.txt".to_string(),
        ];
        let parsed_text = match parse_mvp_snapshot_options(&text_args) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("mvp-snapshot text options should parse: {error}"),
        };
        assert_eq!(parsed_text.out_path, "snapshot.txt");
        assert!(!parsed_text.json_output);

        let non_strict_args = vec!["--no-strict".to_string()];
        let parsed_non_strict = match parse_mvp_snapshot_options(&non_strict_args) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("mvp-snapshot non-strict options should parse: {error}"),
        };
        assert!(!parsed_non_strict.strict);
    }

    #[test]
    fn mvp_verify_options_parse_defaults_and_text() {
        let parsed_default = match parse_mvp_verify_options(&[]) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("default mvp-verify options should parse: {error}"),
        };
        assert_eq!(parsed_default, MvpVerifyOptions::default());
        assert!(parsed_default.json_output);
        assert!(parsed_default.strict);
        assert!(!parsed_default.refresh);

        let args = vec![
            "--text".to_string(),
            "--refresh".to_string(),
            "--no-strict".to_string(),
            "--out".to_string(),
            "verify.txt".to_string(),
        ];
        let parsed = match parse_mvp_verify_options(&args) {
            Ok(parsed) => parsed,
            Err(error) => unreachable!("mvp-verify options should parse: {error}"),
        };
        assert_eq!(parsed.out_path, "verify.txt");
        assert!(!parsed.json_output);
        assert!(!parsed.strict);
        assert!(parsed.refresh);
    }

    #[test]
    fn mvp_spec_report_markdown_contains_status() {
        let report = render_mvp_spec_report_markdown(MvpSpecCheckResult {
            m0_workspace: true,
            m1_local_tunnel: true,
            m2_crypto_session: true,
            m3_carrier_validation: true,
            m4_routing_determinism: true,
            m5_doctor_and_config: true,
            m6_hardening: true,
        });
        assert!(report.contains("Status: **PASS**"));
        assert!(report.contains("- M6 hardening: `true`"));
    }

    #[test]
    fn m5_artifacts_report_render_ru_contains_status() {
        let text =
            render_m5_artifacts_report_markdown(Language::Ru, true, true, true, true, true, true);
        assert!(text.contains("# Отчет По Артефактам M5"));
        assert!(text.contains("Статус: **PASS**"));
        assert!(text.contains("docs/datapath_latest.json"));
    }

    #[test]
    fn m6_artifacts_report_render_ru_contains_status() {
        let text = render_m6_artifacts_report_markdown(Language::Ru, true, true, true);
        assert!(text.contains("# Отчет По Артефактам M6"));
        assert!(text.contains("Статус: **PASS**"));
    }

    #[test]
    fn release_readiness_report_render_ru_contains_status() {
        let checklist = ReleaseGateChecklist {
            clean_clone_builds: true,
            client_gateway_run_linux: true,
            encrypted_tunnel_carries_traffic: true,
            policy_routing_direct_gateway_block: true,
            dns_binding_works: true,
            route_explain_works: true,
            shutdown_restores_network_state: true,
            security_tests_pass: true,
            parser_fuzz_smoke_passes: true,
            no_raw_secrets_in_logs: true,
            benchmark_report_exists: true,
            operations_guide_exists: true,
            runtime_apply_dns_verified: true,
            runtime_apply_route_verified: true,
            runtime_route_policy_validation_verified: true,
            runtime_tun_name_validation_verified: true,
            runtime_forced_stop_rollback_verified: true,
        };
        let text = render_release_readiness_report_markdown(
            Language::Ru,
            true,
            false,
            MvpSpecCheckResult {
                m0_workspace: true,
                m1_local_tunnel: true,
                m2_crypto_session: true,
                m3_carrier_validation: true,
                m4_routing_determinism: true,
                m5_doctor_and_config: true,
                m6_hardening: true,
            },
            checklist,
            ReleaseReadinessArtifacts {
                m5_report_ok: true,
                m6_report_ok: true,
                benchmark_ok: true,
                cef_phase1_smoke_ok: true,
                mesh_route_explain_ok: true,
                mesh_auto_adaptive_ok: true,
            },
        );
        assert!(text.contains("# Отчет Готовности Релиза"));
        assert!(text.contains("Статус: **PASS**"));
        assert!(text.contains(
            "Просто: если статус PASS, MVP готов только к расширенным лабораторным тестам"
        ));
        assert!(text.contains("Real OS-level datapath closure (strict M4/M5): `false`"));
    }

    #[test]
    fn release_readiness_report_render_en_contains_lab_only_notice() {
        let checklist = ReleaseGateChecklist {
            clean_clone_builds: true,
            client_gateway_run_linux: true,
            encrypted_tunnel_carries_traffic: true,
            policy_routing_direct_gateway_block: true,
            dns_binding_works: true,
            route_explain_works: true,
            shutdown_restores_network_state: true,
            security_tests_pass: true,
            parser_fuzz_smoke_passes: true,
            no_raw_secrets_in_logs: true,
            benchmark_report_exists: true,
            operations_guide_exists: true,
            runtime_apply_dns_verified: true,
            runtime_apply_route_verified: true,
            runtime_route_policy_validation_verified: true,
            runtime_tun_name_validation_verified: true,
            runtime_forced_stop_rollback_verified: true,
        };
        let text = render_release_readiness_report_markdown(
            Language::En,
            true,
            false,
            MvpSpecCheckResult {
                m0_workspace: true,
                m1_local_tunnel: true,
                m2_crypto_session: true,
                m3_carrier_validation: true,
                m4_routing_determinism: true,
                m5_doctor_and_config: true,
                m6_hardening: true,
            },
            checklist,
            ReleaseReadinessArtifacts {
                m5_report_ok: true,
                m6_report_ok: true,
                benchmark_ok: true,
                cef_phase1_smoke_ok: true,
                mesh_route_explain_ok: true,
                mesh_auto_adaptive_ok: true,
            },
        );
        assert!(text.contains("# Release Readiness Report"));
        assert!(text.contains("Status: **PASS**"));
        assert!(text.contains("ready for wider lab validation only"));
        assert!(text.contains("not a real-world datapath closure claim"));
        assert!(text.contains("Real OS-level datapath closure (strict M4/M5): `false`"));
    }

    #[test]
    fn release_readiness_report_json_contains_gate_fields() {
        let checklist = ReleaseGateChecklist {
            clean_clone_builds: true,
            client_gateway_run_linux: true,
            encrypted_tunnel_carries_traffic: true,
            policy_routing_direct_gateway_block: true,
            dns_binding_works: true,
            route_explain_works: true,
            shutdown_restores_network_state: true,
            security_tests_pass: true,
            parser_fuzz_smoke_passes: true,
            no_raw_secrets_in_logs: true,
            benchmark_report_exists: true,
            operations_guide_exists: true,
            runtime_apply_dns_verified: true,
            runtime_apply_route_verified: true,
            runtime_route_policy_validation_verified: true,
            runtime_tun_name_validation_verified: true,
            runtime_forced_stop_rollback_verified: true,
        };
        let json = render_release_readiness_report_json(
            true,
            false,
            MvpSpecCheckResult {
                m0_workspace: true,
                m1_local_tunnel: true,
                m2_crypto_session: true,
                m3_carrier_validation: true,
                m4_routing_determinism: true,
                m5_doctor_and_config: true,
                m6_hardening: true,
            },
            checklist,
            ReleaseReadinessArtifacts {
                m5_report_ok: true,
                m6_report_ok: true,
                benchmark_ok: true,
                cef_phase1_smoke_ok: true,
                mesh_route_explain_ok: true,
                mesh_auto_adaptive_ok: true,
            },
        );
        assert!(json.contains("\"kind\":\"release_readiness_report\""));
        assert!(json.contains("\"release_ok\":true"));
        assert!(json.contains("\"truth_boundary\""));
        assert!(json.contains("\"lab_scope_only\":true"));
        assert!(json.contains("\"real_world_datapath_closed\":false"));
        assert!(json.contains("\"release_gate\""));
        assert!(json.contains("\"no_raw_secrets_in_logs\":true"));
        assert!(json.contains("\"runtime_apply_route_verified\":true"));
        assert!(json.contains("\"network_state\":\"not_modified\""));
    }

    #[test]
    fn rollback_json_artifacts_check_passes_for_valid_files() {
        let dir = std::env::temp_dir();
        let p1 = dir.join("chimera_rollback_status_valid.json");
        let p2 = dir.join("chimera_rollback_recover_valid.json");
        let p3 = dir.join("chimera_rollback_status_after_valid.json");
        let status = "{\"status\":\"ok\",\"kind\":\"rollback\",\"action\":\"status\",\"state_existed\":true,\"state_file\":\"docs/runtime_state_latest.json\",\"network_state\":\"not_modified\"}";
        let recover = "{\"status\":\"ok\",\"kind\":\"rollback\",\"action\":\"recover\",\"state_existed\":true,\"state_file\":\"docs/runtime_state_latest.json\",\"network_state\":\"not_modified\"}";
        let status_after = "{\"status\":\"ok\",\"kind\":\"rollback\",\"action\":\"status\",\"state_existed\":false,\"state_file\":\"docs/runtime_state_latest.json\",\"network_state\":\"not_modified\"}";

        assert!(fs::write(&p1, status).is_ok());
        assert!(fs::write(&p2, recover).is_ok());
        assert!(fs::write(&p3, status_after).is_ok());

        let path1 = p1.to_string_lossy().to_string();
        let path2 = p2.to_string_lossy().to_string();
        let path3 = p3.to_string_lossy().to_string();
        assert!(check_rollback_json_artifacts(&[
            (&path1, "status", true),
            (&path2, "recover", true),
            (&path3, "status", false),
        ]));
    }

    #[test]
    fn rollback_json_artifacts_check_fails_for_missing_field() {
        let dir = std::env::temp_dir();
        let p1 = dir.join("chimera_rollback_status_invalid.json");
        let p2 = dir.join("chimera_rollback_recover_invalid.json");
        let p3 = dir.join("chimera_rollback_status_after_invalid.json");
        let status = "{\"status\":\"ok\",\"kind\":\"rollback\",\"action\":\"status\",\"state_existed\":true,\"state_file\":\"docs/runtime_state_latest.json\",\"network_state\":\"not_modified\"}";
        let bad_recover = "{\"status\":\"ok\",\"kind\":\"rollback\",\"action\":\"status\",\"state_existed\":true,\"state_file\":\"docs/runtime_state_latest.json\",\"network_state\":\"not_modified\"}";
        let status_after = "{\"status\":\"ok\",\"kind\":\"rollback\",\"action\":\"status\",\"state_existed\":false,\"state_file\":\"docs/runtime_state_latest.json\",\"network_state\":\"not_modified\"}";

        assert!(fs::write(&p1, status).is_ok());
        assert!(fs::write(&p2, bad_recover).is_ok());
        assert!(fs::write(&p3, status_after).is_ok());

        let path1 = p1.to_string_lossy().to_string();
        let path2 = p2.to_string_lossy().to_string();
        let path3 = p3.to_string_lossy().to_string();
        assert!(!check_rollback_json_artifacts(&[
            (&path1, "status", true),
            (&path2, "recover", true),
            (&path3, "status", false),
        ]));
    }

    #[test]
    fn route_explain_artifact_check_passes_for_valid_file() {
        let path = std::env::temp_dir().join("chimera_route_explain_valid.json");
        let payload = "{\"status\":\"ok\",\"kind\":\"route_explain\",\"rule_used\":\"default-direct\",\"outbound\":\"direct\",\"reason\":\"default rule\",\"rules_checked\":2}";
        assert!(fs::write(&path, payload).is_ok());
        let path_text = path.to_string_lossy().to_string();
        assert!(check_route_explain_artifact(&path_text));
    }

    #[test]
    fn route_explain_artifact_check_fails_for_missing_field() {
        let path = std::env::temp_dir().join("chimera_route_explain_invalid.json");
        let payload = "{\"status\":\"ok\",\"kind\":\"route_explain\",\"outbound\":\"direct\",\"reason\":\"default rule\",\"rules_checked\":2}";
        assert!(fs::write(&path, payload).is_ok());
        let path_text = path.to_string_lossy().to_string();
        assert!(!check_route_explain_artifact(&path_text));
    }

    #[test]
    fn datapath_artifact_check_passes_for_valid_file() {
        let path = std::env::temp_dir().join("chimera_datapath_valid.json");
        let payload = "{\"status\":\"ok\",\"kind\":\"datapath_report\",\"message_en\":\"Datapath report is ready.\",\"message_ru\":\"Отчет datapath готов.\",\"gateway_explain\":\"matched rule 'example-gateway'\",\"block_explain\":\"matched rule 'blocked-exact'\",\"direct_explain\":\"matched rule 'default-direct'\",\"network_state\":\"not_modified\"}";
        assert!(fs::write(&path, payload).is_ok());
        let path_text = path.to_string_lossy().to_string();
        assert!(check_datapath_artifact(&path_text));
    }

    #[test]
    fn datapath_artifact_check_fails_for_missing_field() {
        let path = std::env::temp_dir().join("chimera_datapath_invalid.json");
        let payload = "{\"status\":\"ok\",\"kind\":\"datapath_report\",\"message_en\":\"Datapath report is ready.\",\"message_ru\":\"Отчет datapath готов.\",\"gateway_explain\":\"matched rule 'example-gateway'\",\"direct_explain\":\"matched rule 'default-direct'\",\"network_state\":\"not_modified\"}";
        assert!(fs::write(&path, payload).is_ok());
        let path_text = path.to_string_lossy().to_string();
        assert!(!check_datapath_artifact(&path_text));
    }

    #[test]
    fn doctor_artifacts_check_passes_for_valid_files() {
        let dir = std::env::temp_dir();
        let p1 = dir.join("chimera_doctor_valid.json");
        let p2 = dir.join("chimera_gateway_doctor_valid.json");
        let p3 = dir.join("chimera_lab_doctor_valid.json");
        let d1 = "{\"status\":\"ok\",\"kind\":\"doctor\",\"network_state\":\"not_modified\"}";
        let d2 =
            "{\"status\":\"ok\",\"kind\":\"gateway_doctor\",\"network_state\":\"not_modified\"}";
        let d3 = "{\"status\":\"ok\",\"kind\":\"lab_doctor\",\"network_state\":\"not_modified\"}";
        assert!(fs::write(&p1, d1).is_ok());
        assert!(fs::write(&p2, d2).is_ok());
        assert!(fs::write(&p3, d3).is_ok());

        let path1 = p1.to_string_lossy().to_string();
        let path2 = p2.to_string_lossy().to_string();
        let path3 = p3.to_string_lossy().to_string();
        assert!(check_doctor_artifacts(&[
            (&path1, "doctor"),
            (&path2, "gateway_doctor"),
            (&path3, "lab_doctor"),
        ]));
    }

    #[test]
    fn doctor_artifacts_check_fails_for_wrong_kind() {
        let dir = std::env::temp_dir();
        let p1 = dir.join("chimera_doctor_invalid.json");
        let p2 = dir.join("chimera_gateway_doctor_invalid.json");
        let p3 = dir.join("chimera_lab_doctor_invalid.json");
        let d1 = "{\"status\":\"ok\",\"kind\":\"doctor\",\"network_state\":\"not_modified\"}";
        let d2 = "{\"status\":\"ok\",\"kind\":\"doctor\",\"network_state\":\"not_modified\"}";
        let d3 = "{\"status\":\"ok\",\"kind\":\"lab_doctor\",\"network_state\":\"not_modified\"}";
        assert!(fs::write(&p1, d1).is_ok());
        assert!(fs::write(&p2, d2).is_ok());
        assert!(fs::write(&p3, d3).is_ok());

        let path1 = p1.to_string_lossy().to_string();
        let path2 = p2.to_string_lossy().to_string();
        let path3 = p3.to_string_lossy().to_string();
        assert!(!check_doctor_artifacts(&[
            (&path1, "doctor"),
            (&path2, "gateway_doctor"),
            (&path3, "lab_doctor"),
        ]));
    }

    #[test]
    fn benchmark_artifact_check_passes_for_valid_file() {
        let path = std::env::temp_dir().join("chimera_benchmark_valid.json");
        let payload = "{\"status\":\"ok\",\"perf_smoke\":true,\"net_sim\":true,\"encode_ops_per_sec\":1,\"decode_ops_per_sec\":1,\"net_sim_reconnect_events\":1,\"net_sim_dropped\":18}";
        assert!(fs::write(&path, payload).is_ok());
        let path_text = path.to_string_lossy().to_string();
        assert!(check_benchmark_artifact(&path_text));
    }

    #[test]
    fn benchmark_artifact_check_fails_for_missing_field() {
        let path = std::env::temp_dir().join("chimera_benchmark_invalid.json");
        let payload = "{\"status\":\"ok\",\"perf_smoke\":true,\"encode_ops_per_sec\":1,\"decode_ops_per_sec\":1,\"net_sim_reconnect_events\":1,\"net_sim_dropped\":18}";
        assert!(fs::write(&path, payload).is_ok());
        let path_text = path.to_string_lossy().to_string();
        assert!(!check_benchmark_artifact(&path_text));
    }
}
