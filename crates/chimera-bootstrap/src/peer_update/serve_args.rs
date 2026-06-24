use crate::Result;
use std::env;

pub(crate) struct ServeReleaseOptions {
    root: String,
    listen: String,
    base_url: Option<String>,
    state_file: Option<String>,
}

impl ServeReleaseOptions {
    pub(crate) fn from_args(args: &mut impl Iterator<Item = String>) -> Result<Self> {
        let mut root = None;
        let mut listen = None;
        let mut base_url = None;
        let mut state_file = None;
        while let Some(flag) = args.next() {
            let value = args
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag.as_str() {
                "--root" => root = Some(value),
                "--listen" => listen = Some(value),
                "--base-url" => base_url = Some(value),
                "--state-file" => state_file = Some(value),
                other => return Err(format!("unexpected serve-release flag: {other}").into()),
            }
        }
        let state_file = state_file.or_else(|| {
            env::var("CHIMERA_PEER_UPDATE_STATE_FILE")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        });
        let base_url = base_url.or_else(|| {
            env::var("CHIMERA_PEER_UPDATE_BASE_URL")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        });
        Ok(Self {
            root: root.ok_or("missing --root")?,
            listen: listen.unwrap_or_else(|| "0.0.0.0:0".to_string()),
            base_url,
            state_file,
        })
    }

    pub(crate) fn root(&self) -> &str {
        &self.root
    }

    pub(crate) fn listen(&self) -> &str {
        &self.listen
    }

    pub(crate) fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    pub(crate) fn state_file(&self) -> Option<&str> {
        self.state_file.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::ServeReleaseOptions;

    #[test]
    fn serve_release_defaults_to_os_selected_port() -> Result<(), String> {
        let mut args = ["--root", "/tmp/chimera"].into_iter().map(str::to_string);
        let options = ServeReleaseOptions::from_args(&mut args).map_err(|e| e.to_string())?;
        assert_eq!(options.listen(), "0.0.0.0:0");
        assert_eq!(options.base_url(), None);
        assert_eq!(options.state_file(), None);
        Ok(())
    }

    #[test]
    fn serve_release_accepts_flags_in_any_order() -> Result<(), String> {
        let mut args = [
            "--base-url",
            "http://node.example",
            "--listen",
            "127.0.0.1:0",
            "--state-file",
            "/tmp/chimera-peer-update-state.json",
            "--root",
            "/tmp/chimera",
        ]
        .into_iter()
        .map(str::to_string);
        let options = ServeReleaseOptions::from_args(&mut args).map_err(|e| e.to_string())?;
        assert_eq!(options.root(), "/tmp/chimera");
        assert_eq!(options.listen(), "127.0.0.1:0");
        assert_eq!(options.base_url(), Some("http://node.example"));
        assert_eq!(
            options.state_file(),
            Some("/tmp/chimera-peer-update-state.json")
        );
        Ok(())
    }
}
