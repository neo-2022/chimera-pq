#![forbid(unsafe_code)]

use std::env;
use std::io::Write;
use std::process::{Command, Stdio};

use chimera_capture::redirect::{TransparentRedirectPlan, default_bypass_cidrs_v4};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    PrintApply,
    PrintDelete,
    Apply,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Options {
    action: Action,
    table_name: String,
    chain_name: String,
    listen_port: u16,
    exempt_uid: u32,
    bypass_cidrs_v4: Vec<String>,
    capture_cidrs_v4: Vec<String>,
    capture_tcp_ports: Vec<u16>,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut action: Option<Action> = None;
        let mut table_name =
            env_value("CHIMERA_REDIRECT_TABLE").unwrap_or_else(|| "chimera_redirect".to_string());
        let mut chain_name =
            env_value("CHIMERA_REDIRECT_CHAIN").unwrap_or_else(|| "output".to_string());
        let mut listen_port = env_value("CHIMERA_REDIRECT_LISTEN_PORT")
            .map(|value| parse_u16(&value, "listen-port"))
            .transpose()?;
        let mut exempt_uid = env_value("CHIMERA_REDIRECT_EXEMPT_UID")
            .map(|value| parse_u32(&value, "exempt-uid"))
            .transpose()?;
        let mut bypass_cidrs_v4 = default_bypass_cidrs_v4();
        let mut capture_cidrs_v4 = Vec::new();
        let mut capture_tcp_ports = Vec::new();

        let mut index = 0usize;
        while index < args.len() {
            let flag = args[index].as_str();
            match flag {
                "print-apply" => {
                    action = Some(Action::PrintApply);
                    index += 1;
                }
                "print-delete" => {
                    action = Some(Action::PrintDelete);
                    index += 1;
                }
                "apply" => {
                    action = Some(Action::Apply);
                    index += 1;
                }
                "delete" => {
                    action = Some(Action::Delete);
                    index += 1;
                }
                "--table" => {
                    table_name = args
                        .get(index + 1)
                        .cloned()
                        .ok_or_else(|| "missing --table value".to_string())?;
                    index += 2;
                }
                "--chain" => {
                    chain_name = args
                        .get(index + 1)
                        .cloned()
                        .ok_or_else(|| "missing --chain value".to_string())?;
                    index += 2;
                }
                "--listen-port" => {
                    listen_port = Some(parse_u16(
                        args.get(index + 1)
                            .ok_or_else(|| "missing --listen-port value".to_string())?,
                        "listen-port",
                    )?);
                    index += 2;
                }
                "--exempt-uid" => {
                    exempt_uid = Some(parse_u32(
                        args.get(index + 1)
                            .ok_or_else(|| "missing --exempt-uid value".to_string())?,
                        "exempt-uid",
                    )?);
                    index += 2;
                }
                "--bypass-cidr-v4" => {
                    bypass_cidrs_v4.push(
                        args.get(index + 1)
                            .cloned()
                            .ok_or_else(|| "missing --bypass-cidr-v4 value".to_string())?,
                    );
                    index += 2;
                }
                "--capture-cidr-v4" => {
                    capture_cidrs_v4.push(
                        args.get(index + 1)
                            .cloned()
                            .ok_or_else(|| "missing --capture-cidr-v4 value".to_string())?,
                    );
                    index += 2;
                }
                "--capture-tcp-port" => {
                    capture_tcp_ports.push(parse_u16(
                        args.get(index + 1)
                            .ok_or_else(|| "missing --capture-tcp-port value".to_string())?,
                        "capture-tcp-port",
                    )?);
                    index += 2;
                }
                "--no-default-bypass" => {
                    bypass_cidrs_v4.clear();
                    index += 1;
                }
                _ => return Err(format!("unknown argument: {flag}")),
            }
        }

        Ok(Self {
            action: action.ok_or_else(|| {
                "missing action: print-apply|print-delete|apply|delete".to_string()
            })?,
            table_name,
            chain_name,
            listen_port: listen_port.ok_or_else(|| {
                "missing --listen-port or CHIMERA_REDIRECT_LISTEN_PORT".to_string()
            })?,
            exempt_uid: exempt_uid
                .ok_or_else(|| "missing --exempt-uid or CHIMERA_REDIRECT_EXEMPT_UID".to_string())?,
            bypass_cidrs_v4,
            capture_cidrs_v4,
            capture_tcp_ports,
        })
    }

    fn plan(&self) -> TransparentRedirectPlan {
        TransparentRedirectPlan {
            table_name: self.table_name.clone(),
            chain_name: self.chain_name.clone(),
            listen_port: self.listen_port,
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
    let plan = options.plan();
    let result = match options.action {
        Action::PrintApply => plan.render_apply_nft().map(|script| print!("{script}")),
        Action::PrintDelete => plan.render_delete_nft().map(|script| print!("{script}")),
        Action::Apply => plan.render_apply_nft().and_then(|script| run_nft(&script)),
        Action::Delete => plan.render_delete_nft().and_then(|script| run_nft(&script)),
    };
    if let Err(error) = result {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run_nft(script: &str) -> Result<(), chimera_core::ChimeraError> {
    let mut child = Command::new("nft")
        .arg("-f")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            chimera_core::ChimeraError::Unsupported(format!("start nft failed: {error}"))
        })?;
    let Some(stdin) = child.stdin.as_mut() else {
        return Err(chimera_core::ChimeraError::Unsupported(
            "nft stdin unavailable".to_string(),
        ));
    };
    stdin.write_all(script.as_bytes()).map_err(|error| {
        chimera_core::ChimeraError::Unsupported(format!("write nft script failed: {error}"))
    })?;
    let output = child.wait_with_output().map_err(|error| {
        chimera_core::ChimeraError::Unsupported(format!("wait nft failed: {error}"))
    })?;
    if output.status.success() {
        return Ok(());
    }
    Err(chimera_core::ChimeraError::Unsupported(format!(
        "nft failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}

fn env_value(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
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
    use super::{Action, Options};

    #[test]
    fn parse_print_apply_options() {
        let args = vec![
            "print-apply".to_string(),
            "--listen-port".to_string(),
            "18124".to_string(),
            "--exempt-uid".to_string(),
            "4242".to_string(),
            "--capture-cidr-v4".to_string(),
            "203.0.113.10/32".to_string(),
            "--capture-tcp-port".to_string(),
            "18143".to_string(),
        ];
        let parsed = Options::parse(&args).unwrap_or_else(|error| {
            unreachable!("options should parse: {error}");
        });
        assert_eq!(parsed.action, Action::PrintApply);
        assert_eq!(parsed.listen_port, 18124);
        assert_eq!(parsed.exempt_uid, 4242);
        assert_eq!(parsed.capture_cidrs_v4, vec!["203.0.113.10/32"]);
        assert_eq!(parsed.capture_tcp_ports, vec![18143]);
    }

    #[test]
    fn parse_requires_action() {
        let args = vec![
            "--listen-port".to_string(),
            "18124".to_string(),
            "--exempt-uid".to_string(),
            "4242".to_string(),
        ];
        assert!(Options::parse(&args).is_err());
    }
}
