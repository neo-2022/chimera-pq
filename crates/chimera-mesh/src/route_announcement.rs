#![forbid(unsafe_code)]

use crate::multipath_model::MeshRouteBindingId;
use base64::Engine;
use std::fmt;
use std::net::IpAddr;
use std::time::{Duration, SystemTime};

/// A capability-based route advertisement from one peer to another.
#[derive(Clone, PartialEq, Eq)]
pub enum RouteAnnouncement {
    Static {
        destination: RouteDestination,
        via: PeerId,
        route_binding_id: MeshRouteBindingId,
        ttl: Duration,
        auth: CapabilityToken,
    },
}

impl RouteAnnouncement {
    pub fn destination(&self) -> &RouteDestination {
        match self {
            Self::Static { destination, .. } => destination,
        }
    }

    pub fn via(&self) -> &PeerId {
        match self {
            Self::Static { via, .. } => via,
        }
    }

    pub fn route_binding_id(&self) -> MeshRouteBindingId {
        match self {
            Self::Static { route_binding_id, .. } => *route_binding_id,
        }
    }

    pub fn ttl(&self) -> Duration {
        match self {
            Self::Static { ttl, .. } => *ttl,
        }
    }

    pub fn auth(&self) -> &CapabilityToken {
        match self {
            Self::Static { auth, .. } => auth,
        }
    }

    pub fn is_expired(&self, now: SystemTime) -> bool {
        match self {
            Self::Static { ttl, .. } => {
                let Some(created) = self.auth().created_at else {
                    return true;
                };
                now.duration_since(created).unwrap_or(Duration::MAX) >= *ttl
            }
        }
    }
}

impl fmt::Debug for RouteAnnouncement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Static {
                destination,
                via,
                route_binding_id,
                ttl,
                auth,
            } => f
                .debug_struct("RouteAnnouncement::Static")
                .field("destination", destination)
                .field("via", via)
                .field("route_binding_id", route_binding_id)
                .field("ttl_seconds", &ttl.as_secs())
                .field("auth", auth)
                .finish(),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum RouteDestination {
    Cidr(IpCidr),
    Domain(String),
}

impl RouteDestination {
    pub fn to_wire_string(&self) -> String {
        match self {
            Self::Cidr(cidr) => format!("cidr/{}", cidr.format()),
            Self::Domain(domain) => format!("domain/{}", domain),
        }
    }
}

impl fmt::Debug for RouteDestination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cidr(cidr) => f.debug_tuple("Cidr").field(cidr).finish(),
            Self::Domain(_domain) => f.debug_tuple("Domain").field(&"<redacted>").finish(),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct IpCidr {
    address: IpAddr,
    prefix: u8,
}

impl IpCidr {
    pub fn format(&self) -> String {
        format!("{}/{}", self.address, self.prefix)
    }

    pub fn new(address: IpAddr, prefix: u8) -> Result<Self, String> {
        let max = match address {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        if prefix > max {
            return Err(format!("prefix {prefix} exceeds {max}"));
        }
        Ok(Self { address, prefix })
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        let value = value.trim();
        let Some((addr_part, prefix_part)) = value.rsplit_once('/') else {
            return Err(format!("CIDR '{value}' missing '/'"));
        };
        let address: IpAddr = addr_part
            .parse()
            .map_err(|error| format!("CIDR address invalid: {error}"))?;
        let prefix: u8 = prefix_part
            .parse()
            .map_err(|error| format!("CIDR prefix invalid: {error}"))?;
        Self::new(address, prefix)
    }

    pub fn address(&self) -> IpAddr {
        self.address
    }

    pub fn prefix(&self) -> u8 {
        self.prefix
    }
}

impl fmt::Debug for IpCidr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.address, self.prefix)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct PeerId(pub String);

impl PeerId {
    pub fn new(value: impl AsRef<str>) -> Result<Self, String> {
        let value = value.as_ref().trim();
        if value.is_empty() {
            return Err("peer id is empty".to_string());
        }
        if value.contains(|c: char| c.is_whitespace() || c == ',' || c == '|' || c == ';') {
            return Err(format!("peer id contains illegal character: '{value}'"));
        }
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PeerId").field(&"<redacted>").finish()
    }
}

/// Cryptographic attestation that the issuer peer agreed to forward traffic.
///
/// For MVP the signature may be empty (verification disabled) if no public
/// key is configured on the receiving node. This is logged but not fatal.
#[derive(Clone, PartialEq, Eq)]
pub struct CapabilityToken {
    pub issuer: PeerId,
    pub destination: RouteDestination,
    pub created_at: Option<SystemTime>,
    pub signature: Vec<u8>,
}

impl CapabilityToken {
    pub fn new(
        issuer: PeerId,
        destination: RouteDestination,
        created_at: Option<SystemTime>,
        signature: Vec<u8>,
    ) -> Self {
        Self {
            issuer,
            destination,
            created_at,
            signature,
        }
    }
}

impl fmt::Debug for CapabilityToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CapabilityToken")
            .field("issuer", &self.issuer)
            .field("destination", &self.destination)
            .field("created_at", &self.created_at)
            .field("signature", &format!("{} bytes", self.signature.len()))
            .finish()
    }
}

/// Serialize route announcements to the wire/policy format.
///
/// Format is the same as `parse_route_announcements`: pipe-separated entries of
/// `static,<destination>,<via_peer_id>,<ttl_seconds>,<route_binding_id>[,<base64_signature]`.
pub fn format_route_announcements(announcements: &[RouteAnnouncement]) -> String {
    announcements
        .iter()
        .map(|announcement| match announcement {
            RouteAnnouncement::Static {
                destination,
                via,
                route_binding_id,
                ttl,
                auth,
            } => {
                let mut base = format!(
                    "static,{},{},{},{}",
                    destination.to_wire_string(),
                    via.as_str(),
                    ttl.as_secs(),
                    route_binding_id.get()
                );
                if !auth.signature.is_empty() {
                    let signature = base64::engine::general_purpose::STANDARD
                        .encode(&auth.signature);
                    base.push(',');
                    base.push_str(&signature);
                }
                base
            }
        })
        .collect::<Vec<_>>()
        .join("|")
}

/// Parse a `mesh_announcements` value into route announcements.
///
/// Format (announcements separated by `|`):
/// `static,<destination>,<via_peer_id>,<ttl_seconds>,<route_binding_id>,[base64_signature]`
///
/// Destination may be `cidr/192.168.31.0/24` or `domain/example.internal`.
/// Empty signature means the announcement is not signed.
pub fn parse_route_announcements(value: &str) -> Result<Vec<RouteAnnouncement>, String> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    value
        .split('|')
        .enumerate()
        .map(|(index, part)| parse_one_route_announcement(index, part))
        .collect()
}

fn parse_one_route_announcement(index: usize, part: &str) -> Result<RouteAnnouncement, String> {
    let part = part.trim();
    let fields: Vec<&str> = part.split(',').collect();
    if fields.is_empty() || fields[0].trim() != "static" {
        return Err(format!(
            "announcement {index}: expected 'static' kind, got '{}'",
            fields.first().unwrap_or(&"")
        ));
    }
    if fields.len() < 5 {
        return Err(format!(
            "announcement {index}: expected at least 5 comma-separated fields"
        ));
    }

    let destination = parse_route_destination(fields[1].trim())
        .map_err(|e| format!("announcement {index} destination: {e}"))?;
    let via = PeerId::new(fields[2].trim())
        .map_err(|e| format!("announcement {index} via: {e}"))?;
    let ttl_seconds: u64 = fields[3]
        .trim()
        .parse()
        .map_err(|e| format!("announcement {index} ttl: {e}"))?;
    if ttl_seconds == 0 {
        return Err(format!("announcement {index}: ttl must be > 0"));
    }
    let route_binding_id: u64 = fields[4]
        .trim()
        .parse()
        .map_err(|e| format!("announcement {index} route_binding_id: {e}"))?;
    let route_binding_id = MeshRouteBindingId::new(route_binding_id)
        .map_err(|_| format!("announcement {index}: route_binding_id must be nonzero"))?;

    let signature = if fields.len() > 5 {
        let raw = fields[5].trim();
        if raw.is_empty() {
            Vec::new()
        } else {
            base64::engine::general_purpose::STANDARD
                .decode(raw)
                .map_err(|e| format!("announcement {index} signature base64: {e}"))?
        }
    } else {
        Vec::new()
    };

    let auth = CapabilityToken::new(via.clone(), destination.clone(), Some(now()), signature);

    Ok(RouteAnnouncement::Static {
        destination,
        via,
        route_binding_id,
        ttl: Duration::from_secs(ttl_seconds),
        auth,
    })
}

fn parse_route_destination(value: &str) -> Result<RouteDestination, String> {
    let value = value.trim();
    if let Some(rest) = value.strip_prefix("cidr/") {
        return Ok(RouteDestination::Cidr(IpCidr::parse(rest)?));
    }
    if let Some(rest) = value.strip_prefix("domain/") {
        let domain = rest.trim();
        if domain.is_empty() {
            return Err("domain destination is empty".to_string());
        }
        if domain.contains(|c: char| c.is_whitespace() || c == ',' || c == '|' || c == ';') {
            return Err(format!("domain destination contains illegal character: '{domain}'"));
        }
        return Ok(RouteDestination::Domain(domain.to_string()));
    }
    Err(format!(
        "destination must start with 'cidr/' or 'domain/', got '{value}'"
    ))
}

fn now() -> SystemTime {
    SystemTime::now()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_announcement() -> String {
        "static,cidr/192.168.31.0/24,vdsina,3600,7,AAAA".to_string()
    }

    #[test]
    fn parse_valid_static_announcement() -> Result<(), String> {
        let parsed = parse_route_announcements(&sample_announcement())?;
        assert_eq!(parsed.len(), 1);
        let ann = &parsed[0];
        assert_eq!(ann.via().as_str(), "vdsina");
        assert_eq!(ann.route_binding_id().get(), 7);
        assert_eq!(ann.ttl(), Duration::from_secs(3600));
        assert!(!ann.is_expired(now()));
        match ann.destination() {
            RouteDestination::Cidr(cidr) => {
                assert_eq!(cidr.prefix(), 24);
                assert!(matches!(cidr.address(), IpAddr::V4(_)));
            }
            other => return Err(format!("unexpected destination: {other:?}")),
        }
        Ok(())
    }

    #[test]
    fn parse_domain_announcement() -> Result<(), String> {
        let value = "static,domain/phase3.laptop.local,amai,1800,11";
        let parsed = parse_route_announcements(value)?;
        assert_eq!(parsed.len(), 1);
        match parsed[0].destination() {
            RouteDestination::Domain(d) => assert_eq!(d, "phase3.laptop.local"),
            other => return Err(format!("unexpected destination: {other:?}")),
        }
        Ok(())
    }

    #[test]
    fn parse_multiple_announcements() -> Result<(), String> {
        let value = "static,cidr/192.168.31.0/24,vdsina,3600,7|static,domain/example.internal,amai,1800,11";
        let parsed = parse_route_announcements(value)?;
        assert_eq!(parsed.len(), 2);
        Ok(())
    }

    #[test]
    fn parse_rejects_zero_ttl() {
        let value = "static,cidr/192.168.31.0/24,vdsina,0,7";
        assert!(parse_route_announcements(value).is_err());
    }

    #[test]
    fn parse_rejects_zero_route_binding_id() {
        let value = "static,cidr/192.168.31.0/24,vdsina,3600,0";
        assert!(parse_route_announcements(value).is_err());
    }

    #[test]
    fn parse_rejects_missing_kind() {
        let value = "cidr/192.168.31.0/24,vdsina,3600,7";
        assert!(parse_route_announcements(value).is_err());
    }

    #[test]
    fn parse_rejects_bad_cidr_prefix() {
        let value = "static,cidr/192.168.31.0/33,vdsina,3600,7";
        assert!(parse_route_announcements(value).is_err());
    }

    #[test]
    fn format_and_parse_round_trip_preserves_announcement() -> Result<(), String> {
        let parsed = parse_route_announcements(
            "static,cidr/192.168.31.0/24,vdsina,3600,7|static,domain/example.internal,amai,1800,11",
        )?;
        let formatted = format_route_announcements(&parsed);
        let reparsed = parse_route_announcements(&formatted)?;

        assert_eq!(reparsed.len(), 2);
        assert_eq!(reparsed[0].via().as_str(), parsed[0].via().as_str());
        assert_eq!(reparsed[1].via().as_str(), parsed[1].via().as_str());
        assert_eq!(
            reparsed[0].destination().to_wire_string(),
            parsed[0].destination().to_wire_string()
        );
        Ok(())
    }

    #[test]
    fn format_includes_signature_when_present() {
        let parsed = parse_route_announcements("static,cidr/192.168.31.0/24,vdsina,3600,7,AAAA")
            .expect("parse");
        let formatted = format_route_announcements(&parsed);
        assert!(formatted.contains("AAAA"), "formatted value must include base64 signature");
    }
}
