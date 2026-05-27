#![forbid(unsafe_code)]

pub mod redirect;

use chimera_core::{ChimeraError, ChimeraResult};
use std::collections::BTreeMap;
use std::os::unix::fs::FileTypeExt;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    Tun,
    LocalProxy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturePlan {
    pub mode: CaptureMode,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatapathRoute {
    Direct,
    Gateway,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapathDecision {
    pub route: DatapathRoute,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectFailureSignal {
    Timeout,
    ConnectionReset,
    TlsHandshakeFailed,
    DnsNoAnswer,
    NetworkUnreachable,
    AccessDenied,
}

impl DirectFailureSignal {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::ConnectionReset => "connection_reset",
            Self::TlsHandshakeFailed => "tls_handshake_failed",
            Self::DnsNoAnswer => "dns_no_answer",
            Self::NetworkUnreachable => "network_unreachable",
            Self::AccessDenied => "access_denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectPathObservation {
    pub flow_key: String,
    pub direct_ok: bool,
    pub gateway_ok: bool,
    pub failure_signal: Option<DirectFailureSignal>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransparentFailoverConfig {
    pub split_tunnel_default: bool,
    pub auto_failover: bool,
    pub failover_ttl_ticks: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FailoverState {
    remaining_ticks: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransparentFailoverEngine {
    config: TransparentFailoverConfig,
    overrides: BTreeMap<String, FailoverState>,
}

impl TransparentFailoverEngine {
    pub fn new(config: TransparentFailoverConfig) -> ChimeraResult<Self> {
        if config.failover_ttl_ticks == 0 {
            return Err(ChimeraError::InvalidConfig(
                "failover_ttl_ticks must be > 0".to_string(),
            ));
        }
        Ok(Self {
            config,
            overrides: BTreeMap::new(),
        })
    }

    pub fn evaluate(&self, flow_key: &str, policy_route: DatapathRoute) -> DatapathDecision {
        if policy_route != DatapathRoute::Direct {
            return DatapathDecision {
                route: policy_route,
                reason: "explicit policy route".to_string(),
            };
        }
        if !self.config.split_tunnel_default {
            return DatapathDecision {
                route: DatapathRoute::Gateway,
                reason: "full-tunnel mode".to_string(),
            };
        }
        if self.overrides.contains_key(flow_key) {
            return DatapathDecision {
                route: DatapathRoute::Gateway,
                reason: "auto-failover override is active".to_string(),
            };
        }
        DatapathDecision {
            route: DatapathRoute::Direct,
            reason: "split default direct route".to_string(),
        }
    }

    pub fn report_direct_blocked(&mut self, flow_key: &str) {
        if !self.config.auto_failover {
            return;
        }
        self.overrides.insert(
            flow_key.to_string(),
            FailoverState {
                remaining_ticks: self.config.failover_ttl_ticks,
            },
        );
    }

    pub fn report_direct_ok(&mut self, flow_key: &str) {
        self.overrides.remove(flow_key);
    }

    pub fn observe_direct_path(&mut self, observation: &DirectPathObservation) -> DatapathDecision {
        if observation.direct_ok {
            self.report_direct_ok(&observation.flow_key);
            return DatapathDecision {
                route: DatapathRoute::Direct,
                reason: "direct path recovered".to_string(),
            };
        }
        if observation.gateway_ok {
            let signal = observation
                .failure_signal
                .map(DirectFailureSignal::as_str)
                .unwrap_or("unknown_failure");
            self.report_direct_blocked(&observation.flow_key);
            return DatapathDecision {
                route: DatapathRoute::Gateway,
                reason: format!("direct path degraded; signal={signal}; gateway verified"),
            };
        }
        DatapathDecision {
            route: DatapathRoute::Direct,
            reason:
                "direct path failed but gateway not verified; keep direct to avoid false hijack"
                    .to_string(),
        }
    }

    pub fn tick(&mut self) {
        let mut expired: Vec<String> = Vec::new();
        for (key, state) in &mut self.overrides {
            if state.remaining_ticks > 1 {
                state.remaining_ticks -= 1;
            } else {
                expired.push(key.clone());
            }
        }
        for key in expired {
            self.overrides.remove(&key);
        }
    }
}

pub fn plan_capture_mode(tun_supported: bool) -> CapturePlan {
    if tun_supported {
        CapturePlan {
            mode: CaptureMode::Tun,
            reason: "TUN is available on this system".to_string(),
        }
    } else {
        CapturePlan {
            mode: CaptureMode::LocalProxy,
            reason: "TUN is unavailable, fallback to local proxy mode".to_string(),
        }
    }
}

pub fn detect_tun_support() -> bool {
    let path = Path::new("/dev/net/tun");
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    metadata.file_type().is_char_device()
}

pub fn parse_capture_mode(value: &str) -> ChimeraResult<CaptureMode> {
    match value.to_ascii_lowercase().as_str() {
        "tun" => Ok(CaptureMode::Tun),
        "local-proxy" => Ok(CaptureMode::LocalProxy),
        _ => Err(ChimeraError::InvalidConfig(format!(
            "unknown capture mode '{value}'"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CaptureMode, DatapathRoute, DirectFailureSignal, DirectPathObservation,
        TransparentFailoverConfig, TransparentFailoverEngine, detect_tun_support,
        parse_capture_mode, plan_capture_mode,
    };

    #[test]
    fn tun_is_selected_when_supported() {
        let plan = plan_capture_mode(true);
        assert_eq!(plan.mode, CaptureMode::Tun);
    }

    #[test]
    fn local_proxy_fallback_is_selected_when_tun_is_unavailable() {
        let plan = plan_capture_mode(false);
        assert_eq!(plan.mode, CaptureMode::LocalProxy);
    }

    #[test]
    fn parse_rejects_unknown_mode() {
        assert!(parse_capture_mode("bad").is_err());
    }

    #[test]
    fn detect_tun_support_does_not_panic() {
        let _ = detect_tun_support();
    }

    #[test]
    fn failover_switches_only_blocked_flow_to_gateway() {
        let mut engine = TransparentFailoverEngine::new(TransparentFailoverConfig {
            split_tunnel_default: true,
            auto_failover: true,
            failover_ttl_ticks: 3,
        })
        .expect("config must be valid");

        let blocked = "blocked.example.invalid:443/tcp";
        let normal = "ordinary.example.invalid:443/tcp";

        assert_eq!(
            engine.evaluate(blocked, DatapathRoute::Direct).route,
            DatapathRoute::Direct
        );
        engine.report_direct_blocked(blocked);
        assert_eq!(
            engine.evaluate(blocked, DatapathRoute::Direct).route,
            DatapathRoute::Gateway
        );
        assert_eq!(
            engine.evaluate(normal, DatapathRoute::Direct).route,
            DatapathRoute::Direct
        );
    }

    #[test]
    fn failover_override_expires_by_ttl() {
        let mut engine = TransparentFailoverEngine::new(TransparentFailoverConfig {
            split_tunnel_default: true,
            auto_failover: true,
            failover_ttl_ticks: 2,
        })
        .expect("config must be valid");
        let key = "blocked.example.invalid:443/tcp";
        engine.report_direct_blocked(key);
        assert_eq!(
            engine.evaluate(key, DatapathRoute::Direct).route,
            DatapathRoute::Gateway
        );
        engine.tick();
        assert_eq!(
            engine.evaluate(key, DatapathRoute::Direct).route,
            DatapathRoute::Gateway
        );
        engine.tick();
        assert_eq!(
            engine.evaluate(key, DatapathRoute::Direct).route,
            DatapathRoute::Direct
        );
    }

    #[test]
    fn explicit_policy_route_has_priority_over_failover() {
        let mut engine = TransparentFailoverEngine::new(TransparentFailoverConfig {
            split_tunnel_default: true,
            auto_failover: true,
            failover_ttl_ticks: 2,
        })
        .expect("config must be valid");
        let key = "any:53/udp";
        engine.report_direct_blocked(key);

        assert_eq!(
            engine.evaluate(key, DatapathRoute::Block).route,
            DatapathRoute::Block
        );
        assert_eq!(
            engine.evaluate(key, DatapathRoute::Gateway).route,
            DatapathRoute::Gateway
        );
    }

    #[test]
    fn observation_switches_to_gateway_only_when_gateway_verified() {
        let mut engine = TransparentFailoverEngine::new(TransparentFailoverConfig {
            split_tunnel_default: true,
            auto_failover: true,
            failover_ttl_ticks: 3,
        })
        .expect("config must be valid");
        let key = "resource.example.invalid:443/tcp";

        let unverified = engine.observe_direct_path(&DirectPathObservation {
            flow_key: key.to_string(),
            direct_ok: false,
            gateway_ok: false,
            failure_signal: Some(DirectFailureSignal::Timeout),
        });
        assert_eq!(unverified.route, DatapathRoute::Direct);

        let verified = engine.observe_direct_path(&DirectPathObservation {
            flow_key: key.to_string(),
            direct_ok: false,
            gateway_ok: true,
            failure_signal: Some(DirectFailureSignal::ConnectionReset),
        });
        assert_eq!(verified.route, DatapathRoute::Gateway);
        assert_eq!(
            engine.evaluate(key, DatapathRoute::Direct).route,
            DatapathRoute::Gateway
        );
    }

    #[test]
    fn observation_direct_recovery_clears_failover() {
        let mut engine = TransparentFailoverEngine::new(TransparentFailoverConfig {
            split_tunnel_default: true,
            auto_failover: true,
            failover_ttl_ticks: 3,
        })
        .expect("config must be valid");
        let key = "recovering.example.invalid:443/tcp";
        engine.report_direct_blocked(key);
        assert_eq!(
            engine.evaluate(key, DatapathRoute::Direct).route,
            DatapathRoute::Gateway
        );

        let recovered = engine.observe_direct_path(&DirectPathObservation {
            flow_key: key.to_string(),
            direct_ok: true,
            gateway_ok: false,
            failure_signal: None,
        });
        assert_eq!(recovered.route, DatapathRoute::Direct);
        assert_eq!(
            engine.evaluate(key, DatapathRoute::Direct).route,
            DatapathRoute::Direct
        );
    }
}
