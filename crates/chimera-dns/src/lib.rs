#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::net::IpAddr;
use std::time::{Duration, Instant};

const MAX_TTL_SECONDS: u64 = 86_400; // 24 hours

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsBinding {
    pub domain: String,
    pub ip: IpAddr,
    expires_at: Instant,
}

impl DnsBinding {
    pub fn new(domain: impl Into<String>, ip: IpAddr, ttl: Duration, now: Instant) -> Self {
        let ttl = ttl.min(Duration::from_secs(MAX_TTL_SECONDS));
        Self {
            domain: domain.into(),
            ip,
            expires_at: now + ttl,
        }
    }

    pub fn is_expired(&self, now: Instant) -> bool {
        now >= self.expires_at
    }

    pub fn is_expired_with_grace(&self, now: Instant, grace: Duration) -> bool {
        saturating_add_instant(self.expires_at, grace)
            .map(|deadline| now >= deadline)
            .unwrap_or(false)
    }
}

fn saturating_add_instant(instant: Instant, duration: Duration) -> Option<Instant> {
    instant.checked_add(duration)
}

#[derive(Debug, Default)]
pub struct DnsBindingStore {
    by_ip: BTreeMap<IpAddr, DnsBinding>,
    by_domain: BTreeMap<String, BTreeSet<IpAddr>>,
}

impl DnsBindingStore {
    pub fn insert(&mut self, binding: DnsBinding) {
        self.remove_by_ip(binding.ip);
        self.by_domain
            .entry(binding.domain.clone())
            .or_default()
            .insert(binding.ip);
        self.by_ip.insert(binding.ip, binding);
    }

    pub fn remove(&mut self, ip: IpAddr) {
        self.remove_by_ip(ip);
    }

    fn remove_by_ip(&mut self, ip: IpAddr) {
        if let Some(old) = self.by_ip.remove(&ip)
            && let Some(ips) = self.by_domain.get_mut(&old.domain)
        {
            ips.remove(&ip);
            if ips.is_empty() {
                self.by_domain.remove(&old.domain);
            }
        }
    }

    pub fn lookup(&self, ip: IpAddr, now: Instant) -> Option<&DnsBinding> {
        self.by_ip
            .get(&ip)
            .filter(|binding| !binding.is_expired(now))
    }

    pub fn lookup_domain(&self, domain: &str, now: Instant) -> Vec<&DnsBinding> {
        self.by_domain
            .get(domain)
            .map(|ips| {
                ips.iter()
                    .filter_map(|ip| self.by_ip.get(ip))
                    .filter(|binding| !binding.is_expired(now))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn purge_expired(&mut self, now: Instant) {
        let expired: Vec<IpAddr> = self
            .by_ip
            .values()
            .filter(|binding| binding.is_expired(now))
            .map(|binding| binding.ip)
            .collect();
        for ip in expired {
            self.remove_by_ip(ip);
        }
    }

    pub fn len(&self) -> usize {
        self.by_ip.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_ip.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{DnsBinding, DnsBindingStore, MAX_TTL_SECONDS};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::time::{Duration, Instant};

    #[test]
    fn binding_expires_by_ttl() {
        let now = Instant::now();
        let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10));
        let mut store = DnsBindingStore::default();
        store.insert(DnsBinding::new(
            "example.org",
            ip,
            Duration::from_secs(1),
            now,
        ));

        assert!(store.lookup(ip, now).is_some());
        assert!(store.lookup(ip, now + Duration::from_secs(2)).is_none());
    }

    #[test]
    fn binding_expires_with_grace_after_ttl_plus_grace() {
        let now = Instant::now();
        let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 11));
        let mut store = DnsBindingStore::default();
        store.insert(DnsBinding::new(
            "example.org",
            ip,
            Duration::from_secs(1),
            now,
        ));

        assert!(store
            .lookup(ip, now + Duration::from_millis(900))
            .is_some());
        assert!(store
            .lookup(ip, now + Duration::from_secs(2))
            .is_none());
        assert!(DnsBinding::new("example.org", ip, Duration::from_secs(1), now)
            .is_expired_with_grace(now + Duration::from_secs(2), Duration::from_secs(1)));
        assert!(!DnsBinding::new("example.org", ip, Duration::from_secs(1), now)
            .is_expired_with_grace(now + Duration::from_millis(900), Duration::from_secs(1)));
    }

    #[test]
    fn zero_ttl_expires_immediately() {
        let now = Instant::now();
        let ip = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
        let binding = DnsBinding::new("expired.example", ip, Duration::ZERO, now);
        assert!(binding.is_expired(now));
    }

    #[test]
    fn ttl_is_capped_at_max() {
        let now = Instant::now();
        let ip = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 2));
        let binding = DnsBinding::new(
            "longttl.example",
            ip,
            Duration::from_secs(MAX_TTL_SECONDS + 1),
            now,
        );
        assert!(!binding.is_expired(now + Duration::from_secs(MAX_TTL_SECONDS - 1)));
    }

    #[test]
    fn refresh_updates_domain_for_same_ip() {
        let now = Instant::now();
        let ip = IpAddr::V4(Ipv4Addr::new(198, 51, 100, 5));
        let mut store = DnsBindingStore::default();
        store.insert(DnsBinding::new("old.example", ip, Duration::from_secs(60), now));
        store.insert(DnsBinding::new("new.example", ip, Duration::from_secs(60), now));

        assert_eq!(
            store.lookup(ip, now).map(|binding| binding.domain.as_str()),
            Some("new.example")
        );
        assert!(store.lookup_domain("old.example", now).is_empty());
        assert_eq!(store.lookup_domain("new.example", now).len(), 1);
    }

    #[test]
    fn multiple_ips_for_same_domain() {
        let now = Instant::now();
        let ip1 = IpAddr::V4(Ipv4Addr::new(198, 51, 100, 10));
        let ip2 = IpAddr::V4(Ipv4Addr::new(198, 51, 100, 11));
        let mut store = DnsBindingStore::default();
        store.insert(DnsBinding::new("multi.example", ip1, Duration::from_secs(60), now));
        store.insert(DnsBinding::new("multi.example", ip2, Duration::from_secs(60), now));

        assert_eq!(store.lookup_domain("multi.example", now).len(), 2);
        assert!(store.lookup(ip1, now).is_some());
        assert!(store.lookup(ip2, now).is_some());
    }

    #[test]
    fn purge_removes_expired_binding() {
        let now = Instant::now();
        let ip = IpAddr::V4(Ipv4Addr::new(198, 51, 100, 5));
        let mut store = DnsBindingStore::default();
        store.insert(DnsBinding::new(
            "example.net",
            ip,
            Duration::from_secs(1),
            now,
        ));

        store.purge_expired(now + Duration::from_secs(2));
        assert_eq!(store.len(), 0);
        assert!(store.lookup_domain("example.net", now + Duration::from_secs(2)).is_empty());
    }

    #[test]
    fn purge_keeps_live_binding() {
        let now = Instant::now();
        let live_ip = IpAddr::V4(Ipv4Addr::new(198, 51, 100, 7));
        let expired_ip = IpAddr::V4(Ipv4Addr::new(198, 51, 100, 8));
        let mut store = DnsBindingStore::default();
        store.insert(DnsBinding::new("live.example", live_ip, Duration::from_secs(60), now));
        store.insert(DnsBinding::new(
            "expired.example",
            expired_ip,
            Duration::from_secs(1),
            now,
        ));

        store.purge_expired(now + Duration::from_secs(2));
        assert_eq!(store.len(), 1);
        assert!(store.lookup(live_ip, now + Duration::from_secs(2)).is_some());
        assert!(store.lookup(expired_ip, now + Duration::from_secs(2)).is_none());
    }

    #[test]
    fn ipv6_binding_round_trips() {
        let now = Instant::now();
        let ip = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        let mut store = DnsBindingStore::default();
        store.insert(DnsBinding::new("ipv6.example", ip, Duration::from_secs(60), now));
        assert_eq!(
            store.lookup(ip, now).map(|binding| binding.domain.as_str()),
            Some("ipv6.example")
        );
    }

    #[test]
    fn remove_deletes_binding() {
        let now = Instant::now();
        let ip = IpAddr::V4(Ipv4Addr::new(198, 51, 100, 9));
        let mut store = DnsBindingStore::default();
        store.insert(DnsBinding::new("gone.example", ip, Duration::from_secs(60), now));
        store.remove(ip);
        assert!(store.lookup(ip, now).is_none());
        assert!(store.lookup_domain("gone.example", now).is_empty());
    }
}
