#![forbid(unsafe_code)]

use chimera_carrier_quic::{QuicCarrier, QuicCarrierConfig};
use chimera_carrier_tls::{TlsCarrier, TlsCarrierConfig};
use chimera_config::{ConfigCarrierProfile, GatewayConfig, parse_gateway_config_text};
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
struct GatewayDoctorOptions {
    config_path: String,
    json_output: bool,
    out_path: Option<String>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (lang, command_index) = match parse_language_flag(&args) {
        Some(Some(value)) => value,
        Some(None) => {
            eprintln!(
                "Ошибка языка. Используйте: --lang en или --lang ru."
            );
            std::process::exit(2);
        }
        None => (Language::Ru, 1),
    };
    let command = args
        .get(command_index)
        .map(String::as_str)
        .unwrap_or("help");

    let exit_code = match command {
        "run" => run_gateway_command(
            lang,
            args.get(command_index + 1).map(String::as_str),
            args.get(command_index + 2).map(String::as_str),
        ),
        "health" => health_gateway_command(
            lang,
            args.get(command_index + 1).map(String::as_str),
            args.get(command_index + 2).map(String::as_str),
        ),
        "doctor" => doctor_gateway_command(lang, &args[(command_index + 1)..]),
        "help" | "--help" | "-h" => {
            print!("{}", render_help(lang));
            0
        }
        other => {
            match lang {
                Language::En => eprintln!("Unknown gateway command: {other}"),
                Language::Ru => eprintln!("Неизвестная команда gateway: {other}"),
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

fn run_gateway_command(
    lang: Language,
    config_flag: Option<&str>,
    config_path: Option<&str>,
) -> i32 {
    let config = match load_gateway_config(lang, config_flag, config_path) {
        Ok(config) => config,
        Err(code) => return code,
    };
    print!("{}", render_gateway_plan(lang, &config));
    match run_gateway_runtime(lang, &config) {
        Ok(()) => 0,
        Err(error) => {
            match lang {
                Language::En => eprintln!("Gateway runtime error: {error}"),
                Language::Ru => eprintln!("Ошибка runtime gateway: {error}"),
            }
            1
        }
    }
}

fn health_gateway_command(
    lang: Language,
    config_flag: Option<&str>,
    config_path: Option<&str>,
) -> i32 {
    let config = match load_gateway_config(lang, config_flag, config_path) {
        Ok(config) => config,
        Err(code) => return code,
    };
    print!("{}", render_gateway_health(lang, &config));
    0
}

fn doctor_gateway_command(lang: Language, args: &[String]) -> i32 {
    let options = match parse_gateway_doctor_options(args) {
        Ok(options) => options,
        Err(()) => {
            eprintln!("{}", render_doctor_usage(lang));
            return 2;
        }
    };
    let config = match load_gateway_config(lang, Some("--config"), Some(&options.config_path)) {
        Ok(config) => config,
        Err(code) => return code,
    };
    let json = render_gateway_doctor_json(&config);
    if let Some(path) = options.out_path
        && let Err(error) = std::fs::write(&path, &json)
    {
        eprintln!("Не удалось записать отчет gateway doctor: {error}");
        return 1;
    }
    if options.json_output {
        println!("{json}");
    } else {
        print!("{}", render_gateway_doctor(lang, &config));
    }
    0
}

fn parse_gateway_doctor_options(args: &[String]) -> Result<GatewayDoctorOptions, ()> {
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
    Ok(GatewayDoctorOptions {
        config_path: config_path.ok_or(())?,
        json_output,
        out_path,
    })
}

fn load_gateway_config(
    lang: Language,
    config_flag: Option<&str>,
    config_path: Option<&str>,
) -> Result<GatewayConfig, i32> {
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
                Language::En => eprintln!("Could not read gateway config: {error}"),
                Language::Ru => eprintln!("Не удалось прочитать конфиг gateway: {error}"),
            }
            return Err(2);
        }
    };
    let config = match parse_gateway_config_text(&file_content) {
        Ok(config) => config,
        Err(error) => {
            match lang {
                Language::En => eprintln!("Gateway config has an error: {error}"),
                Language::Ru => eprintln!("В конфиге gateway есть ошибка: {error}"),
            }
            return Err(2);
        }
    };
    if let Err(error) = validate_gateway_runtime(&config) {
        match lang {
            Language::En => eprintln!("Gateway runtime check failed: {error}"),
            Language::Ru => eprintln!("Проверка запуска gateway не пройдена: {error}"),
        }
        return Err(2);
    }
    if let Err(error) = (RekeyPolicy {
        max_session_age_seconds: config.rekey.max_age_seconds,
        max_packets_per_key: config.rekey.max_packets_per_key,
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

fn validate_gateway_runtime(config: &GatewayConfig) -> Result<(), String> {
    match config.carrier_profile {
        ConfigCarrierProfile::InMemory => Ok(()),
        ConfigCarrierProfile::Tls => TlsCarrier::new(TlsCarrierConfig {
            server_name: "gateway.local".to_string(),
            connect_addr: config.listen_addr.clone(),
            connect_timeout_ms: 3000,
        })
        .map(|_| ())
        .map_err(|error| error.to_string()),
        ConfigCarrierProfile::Quic => QuicCarrier::new(QuicCarrierConfig {
            server_name: "gateway.local".to_string(),
            connect_addr: config.listen_addr.clone(),
            connect_timeout_ms: 3000,
        })
        .map(|_| ())
        .map_err(|error| error.to_string()),
    }
}

fn run_gateway_runtime(lang: Language, config: &GatewayConfig) -> Result<(), String> {
    if config.carrier_profile == ConfigCarrierProfile::InMemory {
        match lang {
            Language::En => {
                println!("Runtime: in-memory carrier selected, network listener is not required.")
            }
            Language::Ru => {
                println!("Режим выполнения: выбран in-memory канал, сетевой слушатель не требуется.")
            }
        }
        return Ok(());
    }

    let listener = TcpListener::bind(&config.listen_addr)
        .map_err(|error| format!("Не удалось выполнить bind {}: {error}", config.listen_addr))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("Не удалось включить nonblocking: {error}"))?;
    match lang {
        Language::En => println!("Gateway listener started on {}", config.listen_addr),
        Language::Ru => println!("Слушатель gateway запущен на {}", config.listen_addr),
    }

    let run_once = std::env::var("CHIMERA_GATEWAY_RUN_ONCE").ok().as_deref() == Some("1");
    let idle_exit_ms = std::env::var("CHIMERA_GATEWAY_IDLE_EXIT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok());
    let started_at = Instant::now();
    loop {
        match listener.accept() {
            Ok((_stream, addr)) => {
                match lang {
                    Language::En => println!("Gateway accepted connection from {addr}"),
                    Language::Ru => println!("Gateway принял соединение от {addr}"),
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

fn render_gateway_plan(lang: Language, config: &GatewayConfig) -> String {
    let mut out = String::new();
    match lang {
        Language::En => {
            out.push_str("Gateway plan: config accepted\n");
            out.push_str(&format!(
                "Carrier: {}\n",
                carrier_label(config.carrier_profile)
            ));
            out.push_str(&format!("Listen target: {}\n", config.listen_addr));
            out.push_str(&format!(
                "Rekey limits: max age={} sec, max packets={}\n",
                config.rekey.max_age_seconds, config.rekey.max_packets_per_key
            ));
            out.push_str("Listener: will be started by `run` command\n");
            out.push_str("Safety: no firewall changes made\n");
        }
        Language::Ru => {
            out.push_str("План gateway: конфиг принят\n");
            out.push_str(&format!(
                "Канал: {}\n",
                carrier_label(config.carrier_profile)
            ));
            out.push_str(&format!("Точка прослушивания: {}\n", config.listen_addr));
            out.push_str(&format!(
                "Лимиты смены ключа: макс. возраст={} сек, макс. пакетов={}\n",
                config.rekey.max_age_seconds, config.rekey.max_packets_per_key
            ));
            out.push_str("Слушатель: будет запущен командой `run`\n");
            out.push_str("Безопасность: межсетевой экран не менялся\n");
        }
    }
    out
}

fn render_gateway_health(lang: Language, config: &GatewayConfig) -> String {
    let mut out = String::new();
    match lang {
        Language::En => {
            out.push_str("Gateway health: ok\n");
            out.push_str("Checks:\n");
            out.push_str("  - Config format: ok\n");
            out.push_str("  - Carrier profile: ok\n");
            out.push_str("  - Rekey policy: ok\n");
            out.push_str(&format!(
                "Summary: carrier={}, listen={}, rekey_age={} sec, rekey_packets={}\n",
                carrier_label(config.carrier_profile),
                config.listen_addr,
                config.rekey.max_age_seconds,
                config.rekey.max_packets_per_key
            ));
            out.push_str("Network state: not modified\n");
        }
        Language::Ru => {
            out.push_str("Состояние gateway: в норме\n");
            out.push_str("Проверки:\n");
            out.push_str("  - Формат конфига: в норме\n");
            out.push_str("  - Профиль канала: в норме\n");
            out.push_str("  - Политика смены ключа: в норме\n");
            out.push_str(&format!(
                "Сводка: канал={}, прослушивание={}, возраст_ключа={} сек, пакетов_на_ключ={}\n",
                carrier_label(config.carrier_profile),
                config.listen_addr,
                config.rekey.max_age_seconds,
                config.rekey.max_packets_per_key
            ));
            out.push_str("Состояние сети: не изменялось\n");
        }
    }
    out
}

fn render_gateway_doctor(lang: Language, config: &GatewayConfig) -> String {
    let mut out = String::new();
    match lang {
        Language::En => {
            out.push_str("Gateway doctor: ready for MVP checks\n");
            out.push_str("Checks:\n");
            out.push_str("  - Config format: ok\n");
            out.push_str("  - Carrier profile: ok\n");
            out.push_str("  - Rekey policy: ok\n");
            out.push_str(&format!(
                "Summary: carrier={}, listen={}, rekey_age={} sec, rekey_packets={}\n",
                carrier_label(config.carrier_profile),
                config.listen_addr,
                config.rekey.max_age_seconds,
                config.rekey.max_packets_per_key
            ));
            out.push_str("Secrets: <redacted>\n");
            out.push_str("Network state: not modified\n");
        }
        Language::Ru => {
            out.push_str("Gateway doctor: готово к проверкам MVP\n");
            out.push_str("Проверки:\n");
            out.push_str("  - Формат конфига: в норме\n");
            out.push_str("  - Профиль канала: в норме\n");
            out.push_str("  - Политика смены ключа: в норме\n");
            out.push_str(&format!(
                "Сводка: канал={}, прослушивание={}, возраст_ключа={} сек, пакетов_на_ключ={}\n",
                carrier_label(config.carrier_profile),
                config.listen_addr,
                config.rekey.max_age_seconds,
                config.rekey.max_packets_per_key
            ));
            out.push_str("Секреты: <redacted>\n");
            out.push_str("Состояние сети: не изменялось\n");
        }
    }
    out
}

fn render_gateway_doctor_json(config: &GatewayConfig) -> String {
    format!(
        "{{\"status\":\"ok\",\"kind\":\"gateway_doctor\",\"message_en\":\"Gateway doctor check is ready.\",\"message_ru\":\"Проверка gateway doctor готова.\",\"secrets\":\"<redacted>\",\"carrier_profile\":\"{}\",\"listen_addr\":\"{}\",\"rekey_max_age_sec\":{},\"rekey_max_packets\":{},\"network_state\":\"not_modified\"}}",
        carrier_label(config.carrier_profile),
        config.listen_addr,
        config.rekey.max_age_seconds,
        config.rekey.max_packets_per_key
    )
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
        Language::En => "usage: chimera-gateway [--lang en|ru] run --config <gateway_config_file>",
        Language::Ru => {
            "использование: chimera-gateway [--lang en|ru] run --config <файл_gateway_config>"
        }
    }
}

fn render_doctor_usage(lang: Language) -> &'static str {
    match lang {
        Language::En => {
            "usage: chimera-gateway [--lang en|ru] doctor --config <gateway_config_file> [--json] [--out <file>]"
        }
        Language::Ru => {
            "использование: chimera-gateway [--lang en|ru] doctor --config <файл_gateway_config> [--json] [--out <файл>]"
        }
    }
}

fn render_help(lang: Language) -> String {
    let mut out = String::new();
    match lang {
        Language::En => {
            out.push_str("chimera-gateway commands:\n");
            out.push_str("  [--lang en|ru] run --config <gateway_config_file>\n");
            out.push_str("  [--lang en|ru] health --config <gateway_config_file>\n");
            out.push_str(
                "  [--lang en|ru] doctor --config <gateway_config_file> [--json] [--out <file>]\n",
            );
        }
        Language::Ru => {
            out.push_str("Команды chimera-gateway:\n");
            out.push_str("  [--lang en|ru] run --config <файл_gateway_config>\n");
            out.push_str("  [--lang en|ru] health --config <файл_gateway_config>\n");
            out.push_str(
                "  [--lang en|ru] doctor --config <файл_gateway_config> [--json] [--out <файл>]\n",
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        Language, carrier_label, parse_gateway_doctor_options, parse_language_flag,
        render_gateway_doctor_json, render_gateway_health, render_gateway_plan, render_help,
        render_usage, validate_gateway_runtime,
    };
    use chimera_config::{ConfigCarrierProfile, GatewayConfig, RekeyLimits};

    #[test]
    fn gateway_plan_render_contains_core_fields() {
        let config = GatewayConfig {
            carrier_profile: ConfigCarrierProfile::Tls,
            listen_addr: "0.0.0.0:443".to_string(),
            rekey: RekeyLimits {
                max_age_seconds: 300,
                max_packets_per_key: 10_000,
            },
        };
        let rendered = render_gateway_plan(Language::En, &config);
        assert!(rendered.contains("Gateway plan: config accepted"));
        assert!(rendered.contains("Carrier: tls-tcp"));
        assert!(rendered.contains("Listen target: 0.0.0.0:443"));
        assert!(rendered.contains("Listener: will be started by `run` command"));
    }

    #[test]
    fn runtime_validation_rejects_empty_addr_for_tls() {
        let config = GatewayConfig {
            carrier_profile: ConfigCarrierProfile::Tls,
            listen_addr: String::new(),
            rekey: RekeyLimits {
                max_age_seconds: 300,
                max_packets_per_key: 10_000,
            },
        };
        assert!(validate_gateway_runtime(&config).is_err());
    }

    #[test]
    fn usage_and_help_are_not_empty() {
        assert!(render_usage(Language::Ru).contains("chimera-gateway [--lang en|ru] run --config"));
        assert!(render_help(Language::Ru).contains("[--lang en|ru] run --config"));
        assert!(render_help(Language::En).contains("health --config"));
        assert!(render_help(Language::En).contains("doctor --config"));
    }

    #[test]
    fn carrier_label_maps_values() {
        assert_eq!(carrier_label(ConfigCarrierProfile::InMemory), "in-memory");
        assert_eq!(carrier_label(ConfigCarrierProfile::Tls), "tls-tcp");
        assert_eq!(carrier_label(ConfigCarrierProfile::Quic), "quic");
    }

    #[test]
    fn language_flag_is_parsed() {
        let args = vec![
            "chimera-gateway".to_string(),
            "--lang".to_string(),
            "ru".to_string(),
            "help".to_string(),
        ];
        assert_eq!(parse_language_flag(&args), Some(Some((Language::Ru, 3))));
    }

    #[test]
    fn gateway_health_render_contains_core_fields_ru() {
        let config = GatewayConfig {
            carrier_profile: ConfigCarrierProfile::Quic,
            listen_addr: "127.0.0.1:8443".to_string(),
            rekey: RekeyLimits {
                max_age_seconds: 240,
                max_packets_per_key: 4096,
            },
        };
        let rendered = render_gateway_health(Language::Ru, &config);
        assert!(rendered.contains("Состояние gateway: в норме"));
        assert!(rendered.contains("канал=quic"));
        assert!(rendered.contains("Состояние сети: не изменялось"));
    }

    #[test]
    fn doctor_options_parse_full() {
        let args = vec![
            "--config".to_string(),
            "gateway.conf".to_string(),
            "--json".to_string(),
            "--out".to_string(),
            "gateway_doctor.json".to_string(),
        ];
        let parsed = match parse_gateway_doctor_options(&args) {
            Ok(parsed) => parsed,
            Err(()) => unreachable!("doctor options should parse"),
        };
        assert_eq!(parsed.config_path, "gateway.conf");
        assert!(parsed.json_output);
        assert_eq!(parsed.out_path, Some("gateway_doctor.json".to_string()));
    }

    #[test]
    fn doctor_json_contains_redacted_marker() {
        let config = GatewayConfig {
            carrier_profile: ConfigCarrierProfile::Tls,
            listen_addr: "0.0.0.0:443".to_string(),
            rekey: RekeyLimits {
                max_age_seconds: 300,
                max_packets_per_key: 10_000,
            },
        };
        let json = render_gateway_doctor_json(&config);
        assert!(json.contains("\"kind\":\"gateway_doctor\""));
        assert!(json.contains("\"message_en\":\"Gateway doctor check is ready.\""));
        assert!(json.contains("\"message_ru\":\"Проверка gateway doctor готова.\""));
        assert!(json.contains("\"secrets\":\"<redacted>\""));
        assert!(json.contains("\"carrier_profile\":\"tls-tcp\""));
    }
}
