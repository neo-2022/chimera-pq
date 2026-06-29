use serde::{Deserialize, Serialize};

pub const PEER_UPDATE_STATE_KIND: &str = "chimera_peer_update_serve_state";
pub const PEER_UPDATE_STATE_STATUS_READY: &str = "ready";
pub const PEER_UPDATE_STATE_NOOP_FRESH_SEC: u64 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerUpdateStatePublishAction {
    Noop,
    Changed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerUpdateStatePublishDecision {
    pub action: PeerUpdateStatePublishAction,
    pub endpoint_generation: u64,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerUpdateStateAdvertisement<'a> {
    pub listen: &'a str,
    pub base_url: Option<&'a str>,
    pub update_bootstrap_url: Option<&'a str>,
    pub version: &'a str,
    pub sha256: &'a str,
    pub endpoint_epoch: u64,
}

#[derive(Debug, Deserialize)]
pub struct ExistingPeerUpdateServeState {
    kind: String,
    status: String,
    listen: String,
    base_url: Option<String>,
    update_bootstrap_url: Option<String>,
    version: String,
    sha256: String,
    endpoint_epoch: u64,
    endpoint_generation: Option<u64>,
}

#[derive(Serialize)]
struct PeerUpdateServeState<'a> {
    kind: &'static str,
    status: &'static str,
    listen: &'a str,
    base_url: Option<&'a str>,
    update_bootstrap_url: Option<&'a str>,
    version: &'a str,
    sha256: &'a str,
    endpoint_epoch: u64,
    endpoint_generation: u64,
}

impl ExistingPeerUpdateServeState {
    fn validate(&self) -> crate::Result<()> {
        if self.kind != PEER_UPDATE_STATE_KIND {
            return Err("peer update state kind mismatch".into());
        }
        if self.status != PEER_UPDATE_STATE_STATUS_READY {
            return Err("peer update state status mismatch".into());
        }
        validate_listen(&self.listen)?;
        validate_optional_url_pair(&self.base_url, &self.update_bootstrap_url)?;
        validate_non_empty_asciiish(&self.version, "peer update state version")?;
        validate_sha256(&self.sha256)?;
        if matches!(self.endpoint_generation, Some(0)) {
            return Err("peer update state endpoint_generation must be > 0".into());
        }
        Ok(())
    }

    fn matches_advertisement(&self, advertisement: &PeerUpdateStateAdvertisement<'_>) -> bool {
        self.kind == PEER_UPDATE_STATE_KIND
            && self.status == PEER_UPDATE_STATE_STATUS_READY
            && self.listen == advertisement.listen
            && self.base_url.as_deref() == advertisement.base_url
            && self.update_bootstrap_url.as_deref() == advertisement.update_bootstrap_url
            && self.version == advertisement.version
            && self.sha256 == advertisement.sha256
    }

    fn is_fresh_noop(&self, now_epoch: u64) -> bool {
        now_epoch.saturating_sub(self.endpoint_epoch) <= PEER_UPDATE_STATE_NOOP_FRESH_SEC
    }

    fn has_valid_generation(&self) -> bool {
        self.endpoint_generation
            .is_some_and(|generation| generation > 0)
    }
}

pub fn parse_existing_peer_update_state(
    text: &str,
) -> crate::Result<Option<ExistingPeerUpdateServeState>> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    let state: ExistingPeerUpdateServeState = serde_json::from_str(text)
        .map_err(|error| format!("peer update state JSON invalid: {error}"))?;
    state.validate()?;
    Ok(Some(state))
}

pub fn decide_peer_update_state_publish(
    existing: Option<&ExistingPeerUpdateServeState>,
    advertisement: PeerUpdateStateAdvertisement<'_>,
) -> crate::Result<PeerUpdateStatePublishDecision> {
    let same_advertisement = existing
        .as_ref()
        .is_some_and(|state| state.matches_advertisement(&advertisement));
    if same_advertisement
        && existing.as_ref().is_some_and(|state| {
            state.has_valid_generation() && state.is_fresh_noop(advertisement.endpoint_epoch)
        })
    {
        return Ok(PeerUpdateStatePublishDecision {
            action: PeerUpdateStatePublishAction::Noop,
            endpoint_generation: existing
                .and_then(|state| state.endpoint_generation)
                .unwrap_or(1),
            body: None,
        });
    }
    let endpoint_generation = next_endpoint_generation(existing, same_advertisement)?;
    let state = PeerUpdateServeState {
        kind: PEER_UPDATE_STATE_KIND,
        status: PEER_UPDATE_STATE_STATUS_READY,
        listen: advertisement.listen,
        base_url: advertisement.base_url,
        update_bootstrap_url: advertisement.update_bootstrap_url,
        version: advertisement.version,
        sha256: advertisement.sha256,
        endpoint_epoch: advertisement.endpoint_epoch,
        endpoint_generation,
    };
    Ok(PeerUpdateStatePublishDecision {
        action: PeerUpdateStatePublishAction::Changed,
        endpoint_generation,
        body: Some(serde_json::to_string_pretty(&state)?),
    })
}

fn next_endpoint_generation(
    existing: Option<&ExistingPeerUpdateServeState>,
    same_advertisement: bool,
) -> crate::Result<u64> {
    let current = existing
        .and_then(|state| state.endpoint_generation)
        .unwrap_or(0);
    let next = if same_advertisement {
        current.max(1)
    } else if current == u64::MAX {
        return Err("peer update state endpoint_generation exhausted".into());
    } else {
        current.saturating_add(1).max(1)
    };
    Ok(next)
}

fn validate_listen(listen: &str) -> crate::Result<()> {
    validate_non_empty_asciiish(listen, "peer update state listen")?;
    let port = if listen.starts_with('[') {
        let close = listen
            .find(']')
            .ok_or("peer update state listen invalid IPv6 host")?;
        listen[(close + 1)..]
            .strip_prefix(':')
            .ok_or("peer update state listen must be host:port")?
    } else {
        listen
            .rsplit_once(':')
            .map(|(_host, port)| port)
            .ok_or("peer update state listen must be host:port")?
    };
    let port = port
        .parse::<u16>()
        .map_err(|_| "peer update state listen port is invalid")?;
    if port == 0 {
        return Err("peer update state listen port must be > 0".into());
    }
    Ok(())
}

fn validate_optional_url_pair(
    base_url: &Option<String>,
    update_bootstrap_url: &Option<String>,
) -> crate::Result<()> {
    match (base_url.as_deref(), update_bootstrap_url.as_deref()) {
        (None, None) => Ok(()),
        (Some(base_url), Some(update_bootstrap_url)) => {
            validate_non_empty_asciiish(base_url, "peer update state base_url")?;
            validate_non_empty_asciiish(
                update_bootstrap_url,
                "peer update state update_bootstrap_url",
            )?;
            if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
                return Err("peer update state base_url must be http(s)".into());
            }
            if !(update_bootstrap_url.starts_with("http://")
                || update_bootstrap_url.starts_with("https://"))
            {
                return Err("peer update state update_bootstrap_url must be http(s)".into());
            }
            Ok(())
        }
        _ => Err("peer update state base_url/update_bootstrap_url mismatch".into()),
    }
}

fn validate_non_empty_asciiish(value: &str, label: &str) -> crate::Result<()> {
    if value.trim().is_empty() {
        return Err(format!("{label} is empty").into());
    }
    if value != value.trim()
        || value
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace())
    {
        return Err(format!("{label} contains invalid whitespace").into());
    }
    Ok(())
}

fn validate_sha256(value: &str) -> crate::Result<()> {
    validate_non_empty_asciiish(value, "peer update state sha256")?;
    if value.len() != 64 || !value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err("peer update state sha256 is invalid".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        PeerUpdateStateAdvertisement, PeerUpdateStatePublishAction,
        decide_peer_update_state_publish, parse_existing_peer_update_state,
    };

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

    const SHA256: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn advertisement<'a>(
        listen: &'a str,
        update_bootstrap_url: &'a str,
        endpoint_epoch: u64,
    ) -> PeerUpdateStateAdvertisement<'a> {
        PeerUpdateStateAdvertisement {
            listen,
            base_url: Some(update_bootstrap_url.trim_end_matches("/chimera.sh")),
            update_bootstrap_url: Some(update_bootstrap_url),
            version: "1.2.3",
            sha256: SHA256,
            endpoint_epoch,
        }
    }

    fn state_json(endpoint_epoch: u64, endpoint_generation: Option<u64>) -> String {
        let generation = endpoint_generation.map_or_else(String::new, |value| {
            format!(",\"endpoint_generation\":{value}")
        });
        format!(
            "{{\"kind\":\"chimera_peer_update_serve_state\",\"status\":\"ready\",\"listen\":\"127.0.0.1:18179\",\"base_url\":\"http://node.example:18179\",\"update_bootstrap_url\":\"http://node.example:18179/chimera.sh\",\"version\":\"1.2.3\",\"sha256\":\"{SHA256}\",\"endpoint_epoch\":{endpoint_epoch}{generation}}}"
        )
    }

    #[test]
    fn fresh_same_advertisement_is_noop_with_same_generation() -> TestResult {
        let existing_text = state_json(1_000, Some(7));
        let existing = parse_existing_peer_update_state(&existing_text)
            .map_err(|error| format!("expected existing state to parse: {error}"))?
            .ok_or("expected existing state to exist")?;
        let decision = decide_peer_update_state_publish(
            Some(&existing),
            advertisement(
                "127.0.0.1:18179",
                "http://node.example:18179/chimera.sh",
                1_010,
            ),
        )?;
        assert_eq!(decision.action, PeerUpdateStatePublishAction::Noop);
        assert_eq!(decision.endpoint_generation, 7);
        assert!(decision.body.is_none());
        Ok(())
    }

    #[test]
    fn fresh_legacy_same_advertisement_upgrades_generation() -> TestResult {
        let existing_text = state_json(1_000, None);
        let existing = parse_existing_peer_update_state(&existing_text)
            .map_err(|error| format!("expected existing state to parse: {error}"))?
            .ok_or("expected existing state to exist")?;
        let decision = decide_peer_update_state_publish(
            Some(&existing),
            advertisement(
                "127.0.0.1:18179",
                "http://node.example:18179/chimera.sh",
                1_010,
            ),
        )?;
        assert_eq!(decision.action, PeerUpdateStatePublishAction::Changed);
        assert_eq!(decision.endpoint_generation, 1);
        assert!(
            decision
                .body
                .as_deref()
                .is_some_and(|body| body.contains("\"endpoint_generation\": 1"))
        );
        Ok(())
    }

    #[test]
    fn endpoint_change_increments_generation() -> TestResult {
        let existing_text = state_json(1_000, Some(7));
        let existing = parse_existing_peer_update_state(&existing_text)
            .map_err(|error| format!("expected existing state to parse: {error}"))?
            .ok_or("expected existing state to exist")?;
        let decision = decide_peer_update_state_publish(
            Some(&existing),
            advertisement(
                "127.0.0.1:18180",
                "http://node.example:18180/chimera.sh",
                1_010,
            ),
        )?;
        assert_eq!(decision.action, PeerUpdateStatePublishAction::Changed);
        assert_eq!(decision.endpoint_generation, 8);
        let body = decision.body.ok_or("expected changed body")?;
        assert!(body.contains("\"listen\": \"127.0.0.1:18180\""));
        assert!(body.contains("\"endpoint_generation\": 8"));
        Ok(())
    }

    #[test]
    fn stale_same_advertisement_refreshes_without_incrementing_generation() -> TestResult {
        let existing_text = state_json(1_000, Some(7));
        let existing = parse_existing_peer_update_state(&existing_text)
            .map_err(|error| format!("expected existing state to parse: {error}"))?
            .ok_or("expected existing state to exist")?;
        let decision = decide_peer_update_state_publish(
            Some(&existing),
            advertisement(
                "127.0.0.1:18179",
                "http://node.example:18179/chimera.sh",
                1_400,
            ),
        )?;
        assert_eq!(decision.action, PeerUpdateStatePublishAction::Changed);
        assert_eq!(decision.endpoint_generation, 7);
        assert!(decision.body.is_some());
        Ok(())
    }

    #[test]
    fn malformed_existing_state_is_rejected_fail_closed() {
        let error = parse_existing_peer_update_state("{not-json")
            .err()
            .unwrap_or_else(|| unreachable!("malformed state must fail"));
        assert!(error.to_string().contains("JSON invalid"));
    }

    #[test]
    fn zero_generation_existing_state_is_rejected_fail_closed() {
        let error = parse_existing_peer_update_state(&state_json(1_000, Some(0)))
            .err()
            .unwrap_or_else(|| unreachable!("zero generation state must fail"));
        assert!(error.to_string().contains("endpoint_generation"));
    }

    #[test]
    fn generation_exhaustion_is_rejected_fail_closed() -> TestResult {
        let existing_text = state_json(1_000, Some(u64::MAX));
        let existing = parse_existing_peer_update_state(&existing_text)
            .map_err(|error| format!("expected existing state to parse: {error}"))?
            .ok_or("expected existing state to exist")?;

        let error = decide_peer_update_state_publish(
            Some(&existing),
            advertisement(
                "127.0.0.1:18180",
                "http://node.example:18180/chimera.sh",
                1_010,
            ),
        )
        .err()
        .ok_or("generation exhaustion must fail")?;

        assert!(error.to_string().contains("endpoint_generation exhausted"));
        Ok(())
    }
}
