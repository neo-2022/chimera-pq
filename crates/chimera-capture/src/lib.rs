#![forbid(unsafe_code)]

pub mod nft_exec;
pub mod redirect;

use chimera_core::{ChimeraError, ChimeraResult};
use std::collections::BTreeMap;
use std::os::unix::fs::FileTypeExt;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    Tun,
    FailClosed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturePlan {
    pub mode: CaptureMode,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForbiddenManualProxyProtocol {
    HttpConnect,
    HttpAbsoluteForm,
    Socks5Connect,
}

impl ForbiddenManualProxyProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HttpConnect => "http_connect",
            Self::HttpAbsoluteForm => "http_absolute_form",
            Self::Socks5Connect => "socks5_connect",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatapathRoute {
    Direct,
    Transit,
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
    pub transit_ok: bool,
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
                route: DatapathRoute::Transit,
                reason: "full-tunnel transit mode".to_string(),
            };
        }
        if self.overrides.contains_key(flow_key) {
            return DatapathDecision {
                route: DatapathRoute::Transit,
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
        if observation.transit_ok {
            let signal = observation
                .failure_signal
                .map(DirectFailureSignal::as_str)
                .unwrap_or("unknown_failure");
            self.report_direct_blocked(&observation.flow_key);
            return DatapathDecision {
                route: DatapathRoute::Transit,
                reason: format!("direct path degraded; signal={signal}; transit path verified"),
            };
        }
        DatapathDecision {
            route: DatapathRoute::Block,
            reason: "direct path failed and CHIMERA transit path is not verified; fail closed"
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
            mode: CaptureMode::FailClosed,
            reason: "TUN is unavailable; fail closed because proxy capture is forbidden"
                .to_string(),
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
        "local-proxy" => Err(ChimeraError::InvalidConfig(
            "capture mode 'local-proxy' is forbidden; use transparent TUN datapath".to_string(),
        )),
        _ => Err(ChimeraError::InvalidConfig(format!(
            "unknown capture mode '{value}'"
        ))),
    }
}

pub fn detect_forbidden_manual_proxy_protocol(
    initial: &[u8],
) -> Option<ForbiddenManualProxyProtocol> {
    if initial.starts_with(b"CONNECT ") {
        return Some(ForbiddenManualProxyProtocol::HttpConnect);
    }
    if starts_with_http_absolute_form(initial) {
        return Some(ForbiddenManualProxyProtocol::HttpAbsoluteForm);
    }
    if is_socks5_connect_request(initial) {
        return Some(ForbiddenManualProxyProtocol::Socks5Connect);
    }
    None
}

fn starts_with_http_absolute_form(initial: &[u8]) -> bool {
    const METHODS: &[&[u8]] = &[
        b"GET http://",
        b"GET https://",
        b"POST http://",
        b"POST https://",
        b"HEAD http://",
        b"HEAD https://",
        b"PUT http://",
        b"PUT https://",
        b"DELETE http://",
        b"DELETE https://",
        b"OPTIONS http://",
        b"OPTIONS https://",
        b"PATCH http://",
        b"PATCH https://",
    ];
    METHODS.iter().any(|prefix| initial.starts_with(prefix))
}

fn is_socks5_connect_request(initial: &[u8]) -> bool {
    const SOCKS5_VERSION: u8 = 5;
    const SOCKS5_CONNECT: u8 = 1;
    if initial.len() < 4 {
        return false;
    }
    initial[0] == SOCKS5_VERSION
        && initial[1] == SOCKS5_CONNECT
        && initial[2] == 0
        && matches!(initial[3], 1 | 3 | 4)
}

#[cfg(test)]
mod tests {
    use super::{
        CaptureMode, DatapathRoute, DirectFailureSignal, DirectPathObservation,
        ForbiddenManualProxyProtocol, TransparentFailoverConfig, TransparentFailoverEngine,
        detect_forbidden_manual_proxy_protocol, detect_tun_support, parse_capture_mode,
        plan_capture_mode,
    };
    use chimera_core::ChimeraResult;

    fn test_engine(failover_ttl_ticks: u32) -> ChimeraResult<TransparentFailoverEngine> {
        TransparentFailoverEngine::new(TransparentFailoverConfig {
            split_tunnel_default: true,
            auto_failover: true,
            failover_ttl_ticks,
        })
    }

    #[test]
    fn tun_is_selected_when_supported() -> ChimeraResult<()> {
        let plan = plan_capture_mode(true);
        assert_eq!(plan.mode, CaptureMode::Tun);
        Ok(())
    }

    #[test]
    fn fail_closed_is_selected_when_tun_is_unavailable() -> ChimeraResult<()> {
        let plan = plan_capture_mode(false);
        assert_eq!(plan.mode, CaptureMode::FailClosed);
        assert!(plan.reason.contains("fail closed"));
        Ok(())
    }

    #[test]
    fn parse_rejects_unknown_mode() -> ChimeraResult<()> {
        assert!(parse_capture_mode("bad").is_err());
        Ok(())
    }

    #[test]
    fn parse_rejects_local_proxy_mode() -> ChimeraResult<()> {
        assert!(parse_capture_mode("local-proxy").is_err());
        Ok(())
    }

    #[test]
    fn detects_manual_proxy_protocols_before_transparent_routing() -> ChimeraResult<()> {
        let socks_connect_ipv4 = [5, 1, 0, 1, 203, 0, 113, 7, 1, 187];
        let cases = [
            (
                b"CONNECT example.org:443 HTTP/1.1\r\n\r\n".as_slice(),
                ForbiddenManualProxyProtocol::HttpConnect,
            ),
            (
                b"GET http://example.org/path HTTP/1.1\r\n\r\n".as_slice(),
                ForbiddenManualProxyProtocol::HttpAbsoluteForm,
            ),
            (
                b"POST https://example.org/path HTTP/1.1\r\n\r\n".as_slice(),
                ForbiddenManualProxyProtocol::HttpAbsoluteForm,
            ),
            (
                socks_connect_ipv4.as_slice(),
                ForbiddenManualProxyProtocol::Socks5Connect,
            ),
        ];

        for (payload, expected) in cases {
            assert_eq!(
                detect_forbidden_manual_proxy_protocol(payload),
                Some(expected)
            );
        }
        Ok(())
    }

    #[test]
    fn ordinary_transparent_payload_is_not_marked_as_manual_proxy() -> ChimeraResult<()> {
        let cases = [
            b"GET /path HTTP/1.1\r\nHost: example.org\r\n\r\n".as_slice(),
            b"\x16\x03\x01\x00\x2aopaque tls bytes".as_slice(),
            b"CHIMERA-LOCAL/1\n".as_slice(),
            b"".as_slice(),
        ];

        for payload in cases {
            assert_eq!(detect_forbidden_manual_proxy_protocol(payload), None);
        }
        Ok(())
    }

    #[test]
    fn detect_tun_support_does_not_panic() -> ChimeraResult<()> {
        let _ = detect_tun_support();
        Ok(())
    }

    #[test]
    fn failover_switches_only_blocked_flow_to_transit() -> ChimeraResult<()> {
        let mut engine = test_engine(3)?;

        let blocked = "blocked.example.invalid:443/tcp";
        let normal = "ordinary.example.invalid:443/tcp";

        assert_eq!(
            engine.evaluate(blocked, DatapathRoute::Direct).route,
            DatapathRoute::Direct
        );
        engine.report_direct_blocked(blocked);
        assert_eq!(
            engine.evaluate(blocked, DatapathRoute::Direct).route,
            DatapathRoute::Transit
        );
        assert_eq!(
            engine.evaluate(normal, DatapathRoute::Direct).route,
            DatapathRoute::Direct
        );
        Ok(())
    }

    #[test]
    fn failover_override_expires_by_ttl() -> ChimeraResult<()> {
        let mut engine = test_engine(2)?;
        let key = "blocked.example.invalid:443/tcp";
        engine.report_direct_blocked(key);
        assert_eq!(
            engine.evaluate(key, DatapathRoute::Direct).route,
            DatapathRoute::Transit
        );
        engine.tick();
        assert_eq!(
            engine.evaluate(key, DatapathRoute::Direct).route,
            DatapathRoute::Transit
        );
        engine.tick();
        assert_eq!(
            engine.evaluate(key, DatapathRoute::Direct).route,
            DatapathRoute::Direct
        );
        Ok(())
    }

    #[test]
    fn explicit_policy_route_has_priority_over_failover() -> ChimeraResult<()> {
        let mut engine = test_engine(2)?;
        let key = "any:53/udp";
        engine.report_direct_blocked(key);

        assert_eq!(
            engine.evaluate(key, DatapathRoute::Block).route,
            DatapathRoute::Block
        );
        assert_eq!(
            engine.evaluate(key, DatapathRoute::Transit).route,
            DatapathRoute::Transit
        );
        Ok(())
    }

    #[test]
    fn observation_fails_closed_when_transit_is_not_verified() -> ChimeraResult<()> {
        let mut engine = test_engine(3)?;
        let key = "resource.example.invalid:443/tcp";

        let unverified = engine.observe_direct_path(&DirectPathObservation {
            flow_key: key.to_string(),
            direct_ok: false,
            transit_ok: false,
            failure_signal: Some(DirectFailureSignal::Timeout),
        });
        assert_eq!(unverified.route, DatapathRoute::Block);
        assert!(unverified.reason.contains("fail closed"));

        let verified = engine.observe_direct_path(&DirectPathObservation {
            flow_key: key.to_string(),
            direct_ok: false,
            transit_ok: true,
            failure_signal: Some(DirectFailureSignal::ConnectionReset),
        });
        assert_eq!(verified.route, DatapathRoute::Transit);
        assert_eq!(
            engine.evaluate(key, DatapathRoute::Direct).route,
            DatapathRoute::Transit
        );
        Ok(())
    }

    #[test]
    fn observation_direct_recovery_clears_failover() -> ChimeraResult<()> {
        let mut engine = test_engine(3)?;
        let key = "recovering.example.invalid:443/tcp";
        engine.report_direct_blocked(key);
        assert_eq!(
            engine.evaluate(key, DatapathRoute::Direct).route,
            DatapathRoute::Transit
        );

        let recovered = engine.observe_direct_path(&DirectPathObservation {
            flow_key: key.to_string(),
            direct_ok: true,
            transit_ok: false,
            failure_signal: None,
        });
        assert_eq!(recovered.route, DatapathRoute::Direct);
        assert_eq!(
            engine.evaluate(key, DatapathRoute::Direct).route,
            DatapathRoute::Direct
        );
        Ok(())
    }
}
