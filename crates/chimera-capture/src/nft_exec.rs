use std::env;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NftPrivilegeMode {
    Direct,
    Sudo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NftExecutionConfig {
    pub nft_path: PathBuf,
    pub sudo_path: Option<PathBuf>,
    pub privilege_mode: NftPrivilegeMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NftCommandPlan {
    pub program: PathBuf,
    pub args: Vec<String>,
}

impl NftExecutionConfig {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            nft_path: resolve_nft_path_from_env()?,
            sudo_path: if matches!(resolve_privilege_mode_from_env()?, NftPrivilegeMode::Sudo) {
                Some(resolve_sudo_path_from_env()?)
            } else {
                None
            },
            privilege_mode: resolve_privilege_mode_from_env()?,
        })
    }

    pub fn command_plan(&self) -> NftCommandPlan {
        match self.privilege_mode {
            NftPrivilegeMode::Direct => NftCommandPlan {
                program: self.nft_path.clone(),
                args: vec!["-f".to_string(), "-".to_string()],
            },
            NftPrivilegeMode::Sudo => NftCommandPlan {
                program: self
                    .sudo_path
                    .clone()
                    .unwrap_or_else(|| PathBuf::from("/usr/bin/sudo")),
                args: vec![
                    "-n".to_string(),
                    self.nft_path.to_string_lossy().to_string(),
                    "-f".to_string(),
                    "-".to_string(),
                ],
            },
        }
    }
}

pub fn run_nft_script(script: &str) -> Result<(), String> {
    let config = NftExecutionConfig::from_env()?;
    run_nft_script_with_config(script, &config)
}

pub fn run_nft_script_with_config(script: &str, config: &NftExecutionConfig) -> Result<(), String> {
    let plan = config.command_plan();
    let mut child = Command::new(&plan.program)
        .args(&plan.args)
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

fn resolve_privilege_mode_from_env() -> Result<NftPrivilegeMode, String> {
    match env_value("CHIMERA_NFT_PRIVILEGE_MODE").as_deref() {
        Some("direct") => Ok(NftPrivilegeMode::Direct),
        Some("sudo") => Ok(NftPrivilegeMode::Sudo),
        Some(value) => Err(format!(
            "CHIMERA_NFT_PRIVILEGE_MODE must be direct or sudo, got {value}"
        )),
        None => {
            if matches!(
                env_value("CHIMERA_RUNNER_USE_SUDO").as_deref(),
                Some("1" | "true" | "yes")
            ) {
                Ok(NftPrivilegeMode::Sudo)
            } else {
                Ok(NftPrivilegeMode::Direct)
            }
        }
    }
}

fn resolve_nft_path_from_env() -> Result<PathBuf, String> {
    if let Some(path) = env_value("CHIMERA_NFT_BIN") {
        let path = PathBuf::from(path);
        if matches!(
            env_value("CHIMERA_ALLOW_TEST_NFT_BIN").as_deref(),
            Some("1" | "true" | "yes")
        ) {
            validate_test_nft_path(&path)?;
            return Ok(path);
        }
        validate_system_nft_path(&path)?;
        return Ok(path);
    }
    for candidate in ["/usr/sbin/nft", "/usr/bin/nft"] {
        let path = PathBuf::from(candidate);
        if is_executable(&path) {
            return Ok(path);
        }
    }
    Err("nft binary not found in /usr/sbin/nft or /usr/bin/nft".to_string())
}

fn resolve_sudo_path_from_env() -> Result<PathBuf, String> {
    for candidate in ["/usr/bin/sudo", "/bin/sudo"] {
        let path = PathBuf::from(candidate);
        if is_executable(&path) {
            return Ok(path);
        }
    }
    Err("sudo binary not found in /usr/bin/sudo or /bin/sudo".to_string())
}

fn validate_system_nft_path(path: &Path) -> Result<(), String> {
    match path.to_string_lossy().as_ref() {
        "/usr/sbin/nft" | "/usr/bin/nft" if is_executable(path) => Ok(()),
        "/usr/sbin/nft" | "/usr/bin/nft" => {
            Err(format!("nft binary is not executable: {}", path.display()))
        }
        _ => Err("CHIMERA_NFT_BIN is restricted to /usr/sbin/nft or /usr/bin/nft".to_string()),
    }
}

fn validate_test_nft_path(path: &Path) -> Result<(), String> {
    if path.file_name().and_then(|name| name.to_str()) != Some("nft") {
        return Err("test nft override must be named nft".to_string());
    }
    if !is_executable(path) {
        return Err(format!(
            "test nft override is not executable: {}",
            path.display()
        ));
    }
    Ok(())
}

fn is_executable(path: &Path) -> bool {
    path.metadata()
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

fn env_value(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{NftExecutionConfig, NftPrivilegeMode};
    use std::path::PathBuf;

    fn config(mode: NftPrivilegeMode) -> NftExecutionConfig {
        NftExecutionConfig {
            nft_path: PathBuf::from("/usr/sbin/nft"),
            sudo_path: Some(PathBuf::from("/usr/bin/sudo")),
            privilege_mode: mode,
        }
    }

    #[test]
    fn direct_plan_execs_only_nft() {
        let plan = config(NftPrivilegeMode::Direct).command_plan();
        assert_eq!(plan.program, PathBuf::from("/usr/sbin/nft"));
        assert_eq!(plan.args, vec!["-f", "-"]);
    }

    #[test]
    fn sudo_plan_wraps_only_allowlisted_nft() {
        let plan = config(NftPrivilegeMode::Sudo).command_plan();
        assert_eq!(plan.program, PathBuf::from("/usr/bin/sudo"));
        assert_eq!(plan.args, vec!["-n", "/usr/sbin/nft", "-f", "-"]);
    }
}
