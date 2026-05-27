#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;

use chimera_core::{ChimeraError, ChimeraResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboundMode {
    Direct,
    Gateway,
    Block,
    LocalProxy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Tcp,
    Udp,
    Icmp,
    Other(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowContext {
    pub domain: Option<String>,
    pub destination_ip: Option<IpAddr>,
    pub protocol: Protocol,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleMatcher {
    ExactDomain(String),
    DomainSuffix(String),
    CidrV4 { network: Ipv4Addr, prefix: u8 },
    ProtocolPort { protocol: Protocol, port: u16 },
    Default,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteRule {
    pub id: String,
    pub matcher: RuleMatcher,
    pub outbound: OutboundMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteDecision {
    pub matched_rule_id: String,
    pub outbound: OutboundMode,
    pub explanation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteExplainTrace {
    pub decision: RouteDecision,
    pub examined_rules: usize,
    pub matched_rules: usize,
    pub matched_rule_ids_by_priority: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Policy {
    rules: Vec<RouteRule>,
}

impl Policy {
    pub fn new(rules: Vec<RouteRule>) -> Self {
        Self { rules }
    }

    pub fn decide(&self, flow: &FlowContext) -> RouteDecision {
        self.explain(flow).decision
    }

    pub fn explain(&self, flow: &FlowContext) -> RouteExplainTrace {
        let mut best: Option<(u8, &RouteRule)> = None;
        let mut matched_rules = 0usize;
        let mut matched_rule_ids_with_precedence: Vec<(u8, String)> = Vec::new();

        for rule in &self.rules {
            if let Some(precedence) = rule_precedence(rule, flow) {
                matched_rules += 1;
                matched_rule_ids_with_precedence.push((precedence, rule.id.clone()));
                if best.is_none_or(|(best_precedence, _)| precedence < best_precedence) {
                    best = Some((precedence, rule));
                }
            }
        }
        matched_rule_ids_with_precedence.sort_by(
            |(left_precedence, left_id), (right_precedence, right_id)| {
                left_precedence
                    .cmp(right_precedence)
                    .then_with(|| left_id.cmp(right_id))
            },
        );

        let decision = if let Some((_, rule)) = best {
            RouteDecision {
                matched_rule_id: rule.id.clone(),
                outbound: rule.outbound,
                explanation: format!("matched rule '{}'", rule.id),
            }
        } else {
            RouteDecision {
                matched_rule_id: "implicit-default-direct".to_string(),
                outbound: OutboundMode::Direct,
                explanation: "no policy rule matched; using implicit direct route".to_string(),
            }
        };

        RouteExplainTrace {
            decision,
            examined_rules: self.rules.len(),
            matched_rules,
            matched_rule_ids_by_priority: matched_rule_ids_with_precedence
                .into_iter()
                .map(|(_, id)| id)
                .collect(),
        }
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    pub fn summary(&self) -> PolicySummary {
        let mut summary = PolicySummary::default();
        for rule in &self.rules {
            match &rule.matcher {
                RuleMatcher::ExactDomain(_) => summary.exact_domain_rules += 1,
                RuleMatcher::DomainSuffix(_) => summary.domain_suffix_rules += 1,
                RuleMatcher::CidrV4 { .. } => summary.cidr4_rules += 1,
                RuleMatcher::ProtocolPort { .. } => summary.protoport_rules += 1,
                RuleMatcher::Default => summary.default_rules += 1,
            }
            match rule.outbound {
                OutboundMode::Direct => summary.direct_outbound_rules += 1,
                OutboundMode::Gateway => summary.gateway_outbound_rules += 1,
                OutboundMode::Block => summary.block_outbound_rules += 1,
                OutboundMode::LocalProxy => summary.local_proxy_outbound_rules += 1,
            }
        }
        summary.total_rules = self.rules.len();
        summary
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PolicySummary {
    pub total_rules: usize,
    pub exact_domain_rules: usize,
    pub domain_suffix_rules: usize,
    pub cidr4_rules: usize,
    pub protoport_rules: usize,
    pub default_rules: usize,
    pub direct_outbound_rules: usize,
    pub gateway_outbound_rules: usize,
    pub block_outbound_rules: usize,
    pub local_proxy_outbound_rules: usize,
}

pub fn parse_policy_text(input: &str) -> ChimeraResult<Policy> {
    let mut rules = Vec::new();
    let mut seen_rule_ids = BTreeSet::new();
    let mut default_rule_count = 0usize;
    for (line_index, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let rule = parse_rule_line(line, line_index + 1)?;
        if !seen_rule_ids.insert(rule.id.clone()) {
            return Err(ChimeraError::InvalidConfig(format!(
                "line {}: duplicate rule id '{}'",
                line_index + 1,
                rule.id
            )));
        }
        if matches!(rule.matcher, RuleMatcher::Default) {
            default_rule_count += 1;
            if default_rule_count > 1 {
                return Err(ChimeraError::InvalidConfig(format!(
                    "line {}: only one default rule is allowed",
                    line_index + 1
                )));
            }
        }
        rules.push(rule);
    }
    if rules.is_empty() {
        return Err(ChimeraError::InvalidConfig(
            "policy has no rules".to_string(),
        ));
    }
    Ok(Policy::new(rules))
}

fn parse_rule_line(line: &str, line_number: usize) -> ChimeraResult<RouteRule> {
    let (id_raw, right_side) = line
        .split_once('=')
        .ok_or_else(|| ChimeraError::InvalidConfig(format!("line {line_number}: missing '='")))?;
    let id = id_raw.trim();
    if id.is_empty() {
        return Err(ChimeraError::InvalidConfig(format!(
            "line {line_number}: empty rule id"
        )));
    }

    let (matcher_raw, outbound_raw) = right_side
        .split_once("=>")
        .ok_or_else(|| ChimeraError::InvalidConfig(format!("line {line_number}: missing '=>'")))?;
    let matcher = parse_matcher(matcher_raw.trim(), line_number)?;
    let outbound = parse_outbound(outbound_raw.trim(), line_number)?;

    Ok(RouteRule {
        id: id.to_string(),
        matcher,
        outbound,
    })
}

fn parse_matcher(input: &str, line_number: usize) -> ChimeraResult<RuleMatcher> {
    if input.eq_ignore_ascii_case("default") {
        return Ok(RuleMatcher::Default);
    }

    let (kind_raw, value_raw) = input.split_once(':').ok_or_else(|| {
        ChimeraError::InvalidConfig(format!("line {line_number}: matcher must contain ':'"))
    })?;
    let kind = kind_raw.trim().to_ascii_lowercase();
    let value = value_raw.trim();
    if value.is_empty() {
        return Err(ChimeraError::InvalidConfig(format!(
            "line {line_number}: matcher value is empty"
        )));
    }

    match kind.as_str() {
        "exact" => Ok(RuleMatcher::ExactDomain(value.to_string())),
        "suffix" => Ok(RuleMatcher::DomainSuffix(value.to_string())),
        "cidr4" => parse_cidr4_matcher(value, line_number),
        "protoport" => parse_protoport_matcher(value, line_number),
        _ => Err(ChimeraError::InvalidConfig(format!(
            "line {line_number}: unknown matcher kind '{kind}'"
        ))),
    }
}

fn parse_cidr4_matcher(input: &str, line_number: usize) -> ChimeraResult<RuleMatcher> {
    let (ip_raw, prefix_raw) = input.split_once('/').ok_or_else(|| {
        ChimeraError::InvalidConfig(format!(
            "line {line_number}: cidr4 must look like ip/prefix"
        ))
    })?;
    let network = Ipv4Addr::from_str(ip_raw.trim()).map_err(|_| {
        ChimeraError::InvalidConfig(format!("line {line_number}: invalid cidr4 network"))
    })?;
    let prefix = prefix_raw.trim().parse::<u8>().map_err(|_| {
        ChimeraError::InvalidConfig(format!("line {line_number}: invalid cidr4 prefix"))
    })?;
    if prefix > 32 {
        return Err(ChimeraError::InvalidConfig(format!(
            "line {line_number}: cidr4 prefix must be <= 32"
        )));
    }
    Ok(RuleMatcher::CidrV4 { network, prefix })
}

fn parse_protoport_matcher(input: &str, line_number: usize) -> ChimeraResult<RuleMatcher> {
    let (proto_raw, port_raw) = input.split_once('/').ok_or_else(|| {
        ChimeraError::InvalidConfig(format!(
            "line {line_number}: protoport must look like tcp/443"
        ))
    })?;
    let protocol = parse_protocol(proto_raw.trim(), line_number)?;
    let port = port_raw
        .trim()
        .parse::<u16>()
        .map_err(|_| ChimeraError::InvalidConfig(format!("line {line_number}: invalid port")))?;
    Ok(RuleMatcher::ProtocolPort { protocol, port })
}

fn parse_protocol(input: &str, line_number: usize) -> ChimeraResult<Protocol> {
    match input.to_ascii_lowercase().as_str() {
        "tcp" => Ok(Protocol::Tcp),
        "udp" => Ok(Protocol::Udp),
        "icmp" => Ok(Protocol::Icmp),
        other => Err(ChimeraError::InvalidConfig(format!(
            "line {line_number}: unknown protocol '{other}'"
        ))),
    }
}

fn parse_outbound(input: &str, line_number: usize) -> ChimeraResult<OutboundMode> {
    match input.to_ascii_lowercase().as_str() {
        "direct" => Ok(OutboundMode::Direct),
        "gateway" => Ok(OutboundMode::Gateway),
        "block" => Ok(OutboundMode::Block),
        "local-proxy" => Ok(OutboundMode::LocalProxy),
        other => Err(ChimeraError::InvalidConfig(format!(
            "line {line_number}: unknown outbound '{other}'"
        ))),
    }
}

fn rule_precedence(rule: &RouteRule, flow: &FlowContext) -> Option<u8> {
    match &rule.matcher {
        RuleMatcher::ExactDomain(domain) => flow
            .domain
            .as_deref()
            .filter(|candidate| candidate.eq_ignore_ascii_case(domain))
            .map(|_| 1),
        RuleMatcher::DomainSuffix(suffix) => flow
            .domain
            .as_deref()
            .filter(|candidate| domain_matches_suffix(candidate, suffix))
            .map(|_| 2),
        RuleMatcher::CidrV4 { network, prefix } => flow
            .destination_ip
            .and_then(|ip| match ip {
                IpAddr::V4(ipv4) => Some(ipv4),
                IpAddr::V6(_) => None,
            })
            .filter(|ip| ipv4_in_cidr(*ip, *network, *prefix))
            .map(|_| 3),
        RuleMatcher::ProtocolPort { protocol, port } => {
            (flow.protocol == *protocol && flow.port == Some(*port)).then_some(4)
        }
        RuleMatcher::Default => Some(5),
    }
}

fn domain_matches_suffix(candidate: &str, suffix: &str) -> bool {
    candidate.eq_ignore_ascii_case(suffix)
        || candidate
            .to_ascii_lowercase()
            .ends_with(&format!(".{}", suffix.to_ascii_lowercase()))
}

fn ipv4_in_cidr(ip: Ipv4Addr, network: Ipv4Addr, prefix: u8) -> bool {
    if prefix > 32 {
        return false;
    }

    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - u32::from(prefix))
    };

    (u32::from(ip) & mask) == (u32::from(network) & mask)
}

#[cfg(test)]
mod tests {
    use super::{
        FlowContext, OutboundMode, Policy, PolicySummary, Protocol, RouteRule, RuleMatcher,
        parse_policy_text,
    };
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn exact_domain_wins_over_suffix() {
        let policy = Policy::new(vec![
            RouteRule {
                id: "suffix".to_string(),
                matcher: RuleMatcher::DomainSuffix("example.com".to_string()),
                outbound: OutboundMode::Gateway,
            },
            RouteRule {
                id: "exact".to_string(),
                matcher: RuleMatcher::ExactDomain("api.example.com".to_string()),
                outbound: OutboundMode::Block,
            },
        ]);

        let decision = policy.decide(&FlowContext {
            domain: Some("api.example.com".to_string()),
            destination_ip: None,
            protocol: Protocol::Tcp,
            port: Some(443),
        });

        assert_eq!(decision.matched_rule_id, "exact");
        assert_eq!(decision.outbound, OutboundMode::Block);
    }

    #[test]
    fn cidr_rule_matches_ipv4() {
        let policy = Policy::new(vec![RouteRule {
            id: "private-net".to_string(),
            matcher: RuleMatcher::CidrV4 {
                network: Ipv4Addr::new(10, 0, 0, 0),
                prefix: 8,
            },
            outbound: OutboundMode::Direct,
        }]);

        let decision = policy.decide(&FlowContext {
            domain: None,
            destination_ip: Some(IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3))),
            protocol: Protocol::Tcp,
            port: Some(443),
        });

        assert_eq!(decision.matched_rule_id, "private-net");
    }

    #[test]
    fn parses_policy_text_and_matches_rule() {
        let text = r#"
            # simple policy file
            ads_block = suffix:ads.example => block
            default_route = default => direct
        "#;
        let policy = match parse_policy_text(text) {
            Ok(policy) => policy,
            Err(error) => unreachable!("policy should parse: {error}"),
        };
        assert_eq!(policy.rule_count(), 2);

        let decision = policy.decide(&FlowContext {
            domain: Some("cdn.ads.example".to_string()),
            destination_ip: None,
            protocol: Protocol::Tcp,
            port: Some(443),
        });
        assert_eq!(decision.matched_rule_id, "ads_block");
        assert_eq!(decision.outbound, OutboundMode::Block);
    }

    #[test]
    fn rejects_policy_with_bad_syntax() {
        let bad = "broken line without required separators";
        let parsed = parse_policy_text(bad);
        assert!(parsed.is_err());
    }

    #[test]
    fn rejects_duplicate_rule_ids() {
        let text = r#"
            same = default => direct
            same = suffix:example.org => gateway
        "#;
        let parsed = parse_policy_text(text);
        assert!(parsed.is_err());
    }

    #[test]
    fn rejects_multiple_default_rules() {
        let text = r#"
            default_a = default => direct
            default_b = default => gateway
        "#;
        let parsed = parse_policy_text(text);
        assert!(parsed.is_err());
    }

    #[test]
    fn computes_policy_summary() {
        let policy = Policy::new(vec![
            RouteRule {
                id: "a".to_string(),
                matcher: RuleMatcher::ExactDomain("api.example.org".to_string()),
                outbound: OutboundMode::Gateway,
            },
            RouteRule {
                id: "b".to_string(),
                matcher: RuleMatcher::CidrV4 {
                    network: Ipv4Addr::new(10, 0, 0, 0),
                    prefix: 8,
                },
                outbound: OutboundMode::Direct,
            },
            RouteRule {
                id: "c".to_string(),
                matcher: RuleMatcher::Default,
                outbound: OutboundMode::Block,
            },
        ]);
        let summary = policy.summary();
        assert_eq!(
            summary,
            PolicySummary {
                total_rules: 3,
                exact_domain_rules: 1,
                domain_suffix_rules: 0,
                cidr4_rules: 1,
                protoport_rules: 0,
                default_rules: 1,
                direct_outbound_rules: 1,
                gateway_outbound_rules: 1,
                block_outbound_rules: 1,
                local_proxy_outbound_rules: 0,
            }
        );
    }

    #[test]
    fn route_decision_is_deterministic_for_same_flow() {
        let policy = Policy::new(vec![
            RouteRule {
                id: "suffix-example".to_string(),
                matcher: RuleMatcher::DomainSuffix("example.org".to_string()),
                outbound: OutboundMode::Gateway,
            },
            RouteRule {
                id: "dns-cidr".to_string(),
                matcher: RuleMatcher::CidrV4 {
                    network: Ipv4Addr::new(203, 0, 113, 0),
                    prefix: 24,
                },
                outbound: OutboundMode::Block,
            },
            RouteRule {
                id: "web-tcp".to_string(),
                matcher: RuleMatcher::ProtocolPort {
                    protocol: Protocol::Tcp,
                    port: 443,
                },
                outbound: OutboundMode::LocalProxy,
            },
            RouteRule {
                id: "default".to_string(),
                matcher: RuleMatcher::Default,
                outbound: OutboundMode::Direct,
            },
        ]);
        let flow = FlowContext {
            domain: Some("api.example.org".to_string()),
            destination_ip: Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10))),
            protocol: Protocol::Tcp,
            port: Some(443),
        };

        let baseline = policy.explain(&flow);
        for _ in 0..100 {
            let current = policy.explain(&flow);
            assert_eq!(current, baseline);
        }
    }

    #[test]
    fn matched_rules_order_is_stable_for_same_precedence() {
        let text = r#"
            b_rule = suffix:example.org => gateway
            a_rule = suffix:example.org => block
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
        assert_eq!(
            trace.matched_rule_ids_by_priority,
            vec![
                "a_rule".to_string(),
                "b_rule".to_string(),
                "default_route".to_string()
            ]
        );
    }
}
