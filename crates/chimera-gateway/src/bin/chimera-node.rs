#![forbid(unsafe_code)]

use chimera_carrier_quic::{QuicCarrier, QuicCarrierConfig};
use chimera_carrier_tls::{TlsCarrier, TlsCarrierConfig};
use chimera_config::{ConfigCarrierProfile, NodeConfig, RawConfig, parse_node_config_text};
use chimera_session::RekeyPolicy;
use std::io;
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Language {
    En,
    Ru,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NodeRuntimeConfig {
    node: NodeConfig,
    peer_listen_addr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NodeDoctorOptions {
    config_path: String,
    json_output: bool,
    out_path: Option<String>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (lang, command_index) = match parse_language_flag(&args) {
        Some(Some(value)) => value,
        Some(None) => {
            eprintln!("Ошибка языка. Используйте: --lang en или --lang ru.");
            std::process::exit(2);
        }
        None => (Language::Ru, 1),
    };
    let command = args
        .get(command_index)
        .map(String::as_str)
        .unwrap_or("help");

    let exit_code = match command {
        "run" => run_node_command(
            lang,
            args.get(command_index + 1).map(String::as_str),
            args.get(command_index + 2).map(String::as_str),
        ),
        "health" => health_node_command(
            lang,
            args.get(command_index + 1).map(String::as_str),
            args.get(command_index + 2).map(String::as_str),
        ),
        "doctor" => doctor_node_command(lang, &args[(command_index + 1)..]),
        "help" | "--help" | "-h" => {
            print!("{}", render_help(lang));
            0
        }
        other => {
            match lang {
                Language::En => eprintln!("Unknown node command: {other}"),
                Language::Ru => eprintln!("Неизвестная команда node: {other}"),
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
    let lang = match value.as_str() {
        "en" => Language::En,
        "ru" => Language::Ru,
        _ => return Some(None),
    };
    Some(Some((lang, 3)))
}

fn run_node_command(lang: Language, config_flag: Option<&str>, config_path: Option<&str>) -> i32 {
    let config = match load_node_config(lang, config_flag, config_path) {
        Ok(config) => config,
        Err(code) => return code,
    };
    print!("{}", render_node_plan(lang, &config));
    match run_node_runtime(lang, &config) {
        Ok(()) => 0,
        Err(error) => {
            match lang {
                Language::En => eprintln!("Node runtime error: {error}"),
                Language::Ru => eprintln!("Ошибка runtime узла: {error}"),
            }
            1
        }
    }
}

fn health_node_command(
    lang: Language,
    config_flag: Option<&str>,
    config_path: Option<&str>,
) -> i32 {
    let config = match load_node_config(lang, config_flag, config_path) {
        Ok(config) => config,
        Err(code) => return code,
    };
    print!("{}", render_node_health(lang, &config));
    0
}

fn doctor_node_command(lang: Language, args: &[String]) -> i32 {
    let options = match parse_node_doctor_options(args) {
        Ok(options) => options,
        Err(()) => {
            eprintln!("{}", render_doctor_usage(lang));
            return 2;
        }
    };
    let config = match load_node_config(lang, Some("--config"), Some(&options.config_path)) {
        Ok(config) => config,
        Err(code) => return code,
    };
    let json = render_node_doctor_json(&config);
    if let Some(path) = options.out_path
        && let Err(error) = std::fs::write(&path, &json)
    {
        eprintln!("Не удалось записать отчет node doctor: {error}");
        return 1;
    }
    if options.json_output {
        println!("{json}");
    } else {
        print!("{}", render_node_doctor(lang, &config));
    }
    0
}

fn parse_node_doctor_options(args: &[String]) -> Result<NodeDoctorOptions, ()> {
    let mut config_path: Option<String> = None;
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
            "--config" => {
                config_path = Some(args.get(index + 1).cloned().ok_or(())?);
                index += 2;
            }
            "--out" => {
                out_path = Some(args.get(index + 1).cloned().ok_or(())?);
                index += 2;
            }
            _ => return Err(()),
        }
    }
    Ok(NodeDoctorOptions {
        config_path: config_path.ok_or(())?,
        json_output,
        out_path,
    })
}

fn load_node_config(
    lang: Language,
    config_flag: Option<&str>,
    config_path: Option<&str>,
) -> Result<NodeRuntimeConfig, i32> {
    if config_flag != Some("--config") {
        eprintln!("{}", render_usage(lang));
        return Err(2);
    }
    let Some(config_path) = config_path else {
        eprintln!("{}", render_usage(lang));
        return Err(2);
    };

    let file_content = match std::fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(error) => {
            match lang {
                Language::En => eprintln!("Could not read node config: {error}"),
                Language::Ru => eprintln!("Не удалось прочитать конфиг узла: {error}"),
            }
            return Err(2);
        }
    };
    let node = match parse_node_config_text(&file_content) {
        Ok(config) => config,
        Err(error) => {
            match lang {
                Language::En => eprintln!("Node config has an error: {error}"),
                Language::Ru => eprintln!("В конфиге узла есть ошибка: {error}"),
            }
            return Err(2);
        }
    };
    let peer_listen_addr = match peer_listen_addr_from_node_config(&file_content) {
        Ok(addr) => addr,
        Err(error) => {
            match lang {
                Language::En => eprintln!("Node peer ingress config has an error: {error}"),
                Language::Ru => eprintln!("В конфиге peer-входа узла есть ошибка: {error}"),
            }
            return Err(2);
        }
    };
    let config = NodeRuntimeConfig {
        node,
        peer_listen_addr,
    };
    if let Err(error) = validate_node_runtime(&config) {
        match lang {
            Language::En => eprintln!("Node runtime check failed: {error}"),
            Language::Ru => eprintln!("Проверка запуска узла не пройдена: {error}"),
        }
        return Err(2);
    }
    if let Err(error) = (RekeyPolicy {
        max_session_age_seconds: config.node.rekey.max_age_seconds,
        max_packets_per_key: config.node.rekey.max_packets_per_key,
    })
    .validate()
    {
        match lang {
            Language::En => eprintln!("Rekey policy is invalid: {error}"),
            Language::Ru => eprintln!("Политика смены ключа неверна: {error}"),
        }
        return Err(2);
    }
    Ok(config)
}

fn peer_listen_addr_from_node_config(text: &str) -> Result<String, String> {
    let raw = RawConfig::parse(text).map_err(|error| error.to_string())?;
    Ok(raw.get("peer.listen_addr").unwrap_or("auto").to_string())
}

fn validate_node_runtime(config: &NodeRuntimeConfig) -> Result<(), String> {
    validate_node_carrier(&config.node)?;
    if config.peer_listen_addr.trim().is_empty() {
        return Err("peer.listen_addr is empty".to_string());
    }
    Ok(())
}

fn validate_node_carrier(config: &NodeConfig) -> Result<(), String> {
    match config.carrier_profile {
        ConfigCarrierProfile::InMemory => Ok(()),
        ConfigCarrierProfile::Tls => TlsCarrier::new(TlsCarrierConfig {
            server_name: config.carrier_server_name.clone(),
            connect_addr: config.carrier_addr.clone(),
            connect_timeout_ms: 3000,
        })
        .map(|_| ())
        .map_err(|error| error.to_string()),
        ConfigCarrierProfile::Quic => QuicCarrier::new(QuicCarrierConfig {
            server_name: config.carrier_server_name.clone(),
            connect_addr: config.carrier_addr.clone(),
            connect_timeout_ms: 3000,
        })
        .map(|_| ())
        .map_err(|error| error.to_string()),
    }
}

fn run_node_runtime(lang: Language, config: &NodeRuntimeConfig) -> Result<(), String> {
    if config.node.carrier_profile == ConfigCarrierProfile::InMemory {
        match lang {
            Language::En => {
                println!("Runtime: in-memory carrier selected, peer listener is not required.")
            }
            Language::Ru => {
                println!("Режим выполнения: выбран in-memory канал, peer-слушатель не требуется.")
            }
        }
        return Ok(());
    }

    let bind_addr = resolve_peer_bind_addr(&config.peer_listen_addr);
    let listener = TcpListener::bind(&bind_addr).map_err(|error| {
        format!(
            "Не удалось выполнить bind <redacted> listen_state={}: {error}",
            listen_state(&config.peer_listen_addr)
        )
    })?;
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("Не удалось включить nonblocking: {error}"))?;
    match lang {
        Language::En => println!(
            "Node peer listener started on <redacted> listen_state={}",
            listen_state(&config.peer_listen_addr)
        ),
        Language::Ru => println!(
            "Peer-слушатель узла запущен на <redacted> listen_state={}",
            listen_state(&config.peer_listen_addr)
        ),
    }

    let run_once = std::env::var("CHIMERA_NODE_RUN_ONCE").ok().as_deref() == Some("1");
    let idle_exit_ms = std::env::var("CHIMERA_NODE_IDLE_EXIT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok());
    let started_at = Instant::now();
    loop {
        match listener.accept() {
            Ok((_stream, _addr)) => {
                match lang {
                    Language::En => println!("Node accepted peer connection from <redacted>"),
                    Language::Ru => println!("Узел принял peer-соединение от <redacted>"),
                }
                if run_once {
                    return Ok(());
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                if let Some(limit_ms) = idle_exit_ms
                    && started_at.elapsed() >= Duration::from_millis(limit_ms)
                {
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(200));
            }
            Err(error) => return Err(format!("Ошибка accept: {error}")),
        }
    }
}

fn resolve_peer_bind_addr(listen_addr: &str) -> String {
    let value = listen_addr.trim();
    if value == "auto" || value.starts_with("${") {
        return "0.0.0.0:0".to_string();
    }
    value.to_string()
}

fn render_node_plan(lang: Language, config: &NodeRuntimeConfig) -> String {
    let mut out = String::new();
    match lang {
        Language::En => {
            out.push_str("Node plan: config accepted\n");
            out.push_str(&format!(
                "Carrier: {}\n",
                carrier_label(config.node.carrier_profile)
            ));
            out.push_str(&format!(
                "Peer ingress: <redacted>, listen_state={}\n",
                listen_state(&config.peer_listen_addr)
            ));
            out.push_str(&format!(
                "Rekey limits: max age={} sec, max packets={}\n",
                config.node.rekey.max_age_seconds, config.node.rekey.max_packets_per_key
            ));
            out.push_str("Listener: will be started by `run` command\n");
            out.push_str("Safety: no firewall changes made\n");
        }
        Language::Ru => {
            out.push_str("План узла: конфиг принят\n");
            out.push_str(&format!(
                "Канал: {}\n",
                carrier_label(config.node.carrier_profile)
            ));
            out.push_str(&format!(
                "Peer-вход: <redacted>, listen_state={}\n",
                listen_state(&config.peer_listen_addr)
            ));
            out.push_str(&format!(
                "Лимиты смены ключа: макс. возраст={} сек, макс. пакетов={}\n",
                config.node.rekey.max_age_seconds, config.node.rekey.max_packets_per_key
            ));
            out.push_str("Слушатель: будет запущен командой `run`\n");
            out.push_str("Безопасность: межсетевой экран не менялся\n");
        }
    }
    out
}

fn render_node_health(lang: Language, config: &NodeRuntimeConfig) -> String {
    let mut out = String::new();
    match lang {
        Language::En => {
            out.push_str("Node health: ok\n");
            out.push_str("Checks:\n");
            out.push_str("  - Config format: ok\n");
            out.push_str("  - Carrier profile: ok\n");
            out.push_str("  - Peer ingress: ok\n");
            out.push_str("  - Rekey policy: ok\n");
            out.push_str(&format!(
                "Summary: carrier={}, peer_ingress=<redacted>, listen_state={}, rekey_age={} sec, rekey_packets={}\n",
                carrier_label(config.node.carrier_profile),
                listen_state(&config.peer_listen_addr),
                config.node.rekey.max_age_seconds,
                config.node.rekey.max_packets_per_key
            ));
            out.push_str("Network state: not modified\n");
        }
        Language::Ru => {
            out.push_str("Состояние узла: в норме\n");
            out.push_str("Проверки:\n");
            out.push_str("  - Формат конфига: в норме\n");
            out.push_str("  - Профиль канала: в норме\n");
            out.push_str("  - Peer-вход: в норме\n");
            out.push_str("  - Политика смены ключа: в норме\n");
            out.push_str(&format!(
                "Сводка: канал={}, peer_вход=<redacted>, listen_state={}, возраст_ключа={} сек, пакетов_на_ключ={}\n",
                carrier_label(config.node.carrier_profile),
                listen_state(&config.peer_listen_addr),
                config.node.rekey.max_age_seconds,
                config.node.rekey.max_packets_per_key
            ));
            out.push_str("Состояние сети: не изменялось\n");
        }
    }
    out
}

fn render_node_doctor(lang: Language, config: &NodeRuntimeConfig) -> String {
    let mut out = String::new();
    match lang {
        Language::En => {
            out.push_str("Node doctor: ready for MVP checks\n");
            out.push_str("Checks:\n");
            out.push_str("  - Config format: ok\n");
            out.push_str("  - Carrier profile: ok\n");
            out.push_str("  - Peer ingress: ok\n");
            out.push_str("  - Rekey policy: ok\n");
            out.push_str(&format!(
                "Summary: carrier={}, peer_ingress=<redacted>, listen_state={}, rekey_age={} sec, rekey_packets={}\n",
                carrier_label(config.node.carrier_profile),
                listen_state(&config.peer_listen_addr),
                config.node.rekey.max_age_seconds,
                config.node.rekey.max_packets_per_key
            ));
            out.push_str("Secrets: <redacted>\n");
            out.push_str("Network state: not modified\n");
        }
        Language::Ru => {
            out.push_str("Node doctor: готово к проверкам MVP\n");
            out.push_str("Проверки:\n");
            out.push_str("  - Формат конфига: в норме\n");
            out.push_str("  - Профиль канала: в норме\n");
            out.push_str("  - Peer-вход: в норме\n");
            out.push_str("  - Политика смены ключа: в норме\n");
            out.push_str(&format!(
                "Сводка: канал={}, peer_вход=<redacted>, listen_state={}, возраст_ключа={} сек, пакетов_на_ключ={}\n",
                carrier_label(config.node.carrier_profile),
                listen_state(&config.peer_listen_addr),
                config.node.rekey.max_age_seconds,
                config.node.rekey.max_packets_per_key
            ));
            out.push_str("Секреты: <redacted>\n");
            out.push_str("Состояние сети: не изменялось\n");
        }
    }
    out
}

fn render_node_doctor_json(config: &NodeRuntimeConfig) -> String {
    format!(
        "{{\"status\":\"ok\",\"kind\":\"node_doctor\",\"message_en\":\"Node doctor check is ready.\",\"message_ru\":\"Проверка node doctor готова.\",\"secrets\":\"<redacted>\",\"carrier_profile\":\"{}\",\"peer_ingress\":\"<redacted>\",\"listen_state\":\"{}\",\"rekey_max_age_sec\":{},\"rekey_max_packets\":{},\"network_state\":\"not_modified\"}}",
        carrier_label(config.node.carrier_profile),
        listen_state(&config.peer_listen_addr),
        config.node.rekey.max_age_seconds,
        config.node.rekey.max_packets_per_key
    )
}

fn listen_state(listen_addr: &str) -> &'static str {
    let value = listen_addr.trim();
    if value.is_empty() {
        return "unconfigured";
    }
    if value == "auto" || value.starts_with("${") {
        return "auto";
    }
    if value.starts_with("0.0.0.0:")
        || value.starts_with("127.0.0.1:")
        || value.starts_with("[::]:")
        || value.starts_with("[::1]:")
    {
        return "local_or_wildcard";
    }
    "configured"
}

fn carrier_label(profile: ConfigCarrierProfile) -> &'static str {
    match profile {
        ConfigCarrierProfile::InMemory => "in-memory",
        ConfigCarrierProfile::Tls => "tls-tcp",
        ConfigCarrierProfile::Quic => "quic",
    }
}

fn render_usage(lang: Language) -> &'static str {
    match lang {
        Language::En => "usage: chimera-node [--lang en|ru] run --config <node_config_file>",
        Language::Ru => {
            "использование: chimera-node [--lang en|ru] run --config <файл_node_config>"
        }
    }
}

fn render_doctor_usage(lang: Language) -> &'static str {
    match lang {
        Language::En => {
            "usage: chimera-node [--lang en|ru] doctor --config <node_config_file> [--json] [--out <file>]"
        }
        Language::Ru => {
            "использование: chimera-node [--lang en|ru] doctor --config <файл_node_config> [--json] [--out <файл>]"
        }
    }
}

fn render_help(lang: Language) -> String {
    let mut out = String::new();
    match lang {
        Language::En => {
            out.push_str("chimera-node commands:\n");
            out.push_str("  [--lang en|ru] run --config <node_config_file>\n");
            out.push_str("  [--lang en|ru] health --config <node_config_file>\n");
            out.push_str(
                "  [--lang en|ru] doctor --config <node_config_file> [--json] [--out <file>]\n",
            );
        }
        Language::Ru => {
            out.push_str("Команды chimera-node:\n");
            out.push_str("  [--lang en|ru] run --config <файл_node_config>\n");
            out.push_str("  [--lang en|ru] health --config <файл_node_config>\n");
            out.push_str(
                "  [--lang en|ru] doctor --config <файл_node_config> [--json] [--out <файл>]\n",
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        Language, NodeRuntimeConfig, carrier_label, listen_state, parse_language_flag,
        parse_node_doctor_options, render_help, render_node_doctor, render_node_doctor_json,
        render_node_health, render_node_plan, render_usage, resolve_peer_bind_addr,
        validate_node_runtime,
    };
    use chimera_config::{ConfigCaptureMode, ConfigCarrierProfile, NodeConfig, RekeyLimits};

    fn test_node_config(peer_listen_addr: &str) -> NodeRuntimeConfig {
        NodeRuntimeConfig {
            node: NodeConfig {
                carrier_profile: ConfigCarrierProfile::Tls,
                carrier_addr: "198.51.100.7:443".to_string(),
                carrier_server_name: "node.example.org".to_string(),
                capture_mode: ConfigCaptureMode::Auto,
                tun_supported: true,
                split_tunnel_default: true,
                auto_failover: true,
                invisible_mode_required: true,
                rekey: RekeyLimits {
                    max_age_seconds: 300,
                    max_packets_per_key: 10_000,
                },
            },
            peer_listen_addr: peer_listen_addr.to_string(),
        }
    }

    #[test]
    fn node_plan_render_contains_core_fields() {
        let config = test_node_config("0.0.0.0:443");
        let rendered = render_node_plan(Language::En, &config);
        assert!(rendered.contains("Node plan: config accepted"));
        assert!(rendered.contains("Carrier: tls-tcp"));
        assert!(rendered.contains("Peer ingress: <redacted>, listen_state=local_or_wildcard"));
        assert!(!rendered.contains("0.0.0.0:443"));
        assert!(rendered.contains("Listener: will be started by `run` command"));
    }

    #[test]
    fn runtime_validation_rejects_empty_peer_ingress() {
        let config = test_node_config("");
        assert!(validate_node_runtime(&config).is_err());
    }

    #[test]
    fn usage_and_help_are_node_scoped() {
        assert!(render_usage(Language::Ru).contains("chimera-node [--lang en|ru] run --config"));
        assert!(render_help(Language::Ru).contains("Команды chimera-node"));
        assert!(render_help(Language::En).contains("health --config <node_config_file>"));
        assert!(render_help(Language::En).contains("doctor --config <node_config_file>"));
    }

    #[test]
    fn language_flag_is_parsed() {
        let args = vec![
            "chimera-node".to_string(),
            "--lang".to_string(),
            "ru".to_string(),
            "help".to_string(),
        ];
        assert_eq!(parse_language_flag(&args), Some(Some((Language::Ru, 3))));
    }

    #[test]
    fn node_health_render_contains_core_fields_ru() {
        let config = test_node_config("127.0.0.1:8443");
        let rendered = render_node_health(Language::Ru, &config);
        assert!(rendered.contains("Состояние узла: в норме"));
        assert!(rendered.contains("канал=tls-tcp"));
        assert!(rendered.contains("peer_вход=<redacted>, listen_state=local_or_wildcard"));
        assert!(!rendered.contains("127.0.0.1:8443"));
        assert!(rendered.contains("Состояние сети: не изменялось"));
    }

    #[test]
    fn doctor_options_parse_full() {
        let args = vec![
            "--config".to_string(),
            "node.conf".to_string(),
            "--json".to_string(),
            "--out".to_string(),
            "node_doctor.json".to_string(),
        ];
        let parsed = match parse_node_doctor_options(&args) {
            Ok(parsed) => parsed,
            Err(()) => unreachable!("doctor options should parse"),
        };
        assert_eq!(parsed.config_path, "node.conf");
        assert!(parsed.json_output);
        assert_eq!(parsed.out_path, Some("node_doctor.json".to_string()));
    }

    #[test]
    fn doctor_json_contains_node_markers() {
        let config = test_node_config("0.0.0.0:443");
        let json = render_node_doctor_json(&config);
        assert!(json.contains("\"kind\":\"node_doctor\""));
        assert!(json.contains("\"message_en\":\"Node doctor check is ready.\""));
        assert!(json.contains("\"message_ru\":\"Проверка node doctor готова.\""));
        assert!(json.contains("\"secrets\":\"<redacted>\""));
        assert!(json.contains("\"carrier_profile\":\"tls-tcp\""));
        assert!(json.contains("\"peer_ingress\":\"<redacted>\""));
        assert!(json.contains("\"listen_state\":\"local_or_wildcard\""));
        assert!(!json.contains("0.0.0.0:443"));
    }

    #[test]
    fn doctor_text_redacts_peer_ingress() {
        let config = test_node_config("0.0.0.0:443");
        let text = render_node_doctor(Language::En, &config);
        assert!(text.contains("peer_ingress=<redacted>"));
        assert!(text.contains("listen_state=local_or_wildcard"));
        assert!(!text.contains("0.0.0.0:443"));
    }

    #[test]
    fn auto_peer_ingress_uses_os_selected_port() {
        assert_eq!(listen_state("auto"), "auto");
        assert_eq!(listen_state("${CHIMERA_NODE_LISTEN_ADDR}"), "auto");
        assert_eq!(resolve_peer_bind_addr("auto"), "0.0.0.0:0");
        assert_eq!(
            resolve_peer_bind_addr("${CHIMERA_NODE_LISTEN_ADDR}"),
            "0.0.0.0:0"
        );
    }

    #[test]
    fn carrier_label_maps_values() {
        assert_eq!(carrier_label(ConfigCarrierProfile::InMemory), "in-memory");
        assert_eq!(carrier_label(ConfigCarrierProfile::Tls), "tls-tcp");
        assert_eq!(carrier_label(ConfigCarrierProfile::Quic), "quic");
    }
}
