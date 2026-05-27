#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsBinding {
    pub domain: String,
    pub ip: IpAddr,
    expires_at: Instant,
}

impl DnsBinding {
    pub fn new(domain: impl Into<String>, ip: IpAddr, ttl: Duration, now: Instant) -> Self {
        Self {
            domain: domain.into(),
            ip,
            expires_at: now + ttl,
        }
    }

    pub fn is_expired(&self, now: Instant) -> bool {
        now >= self.expires_at
    }
}

#[derive(Debug, Default)]
pub struct DnsBindingStore {
    by_ip: BTreeMap<IpAddr, DnsBinding>,
}

impl DnsBindingStore {
    pub fn insert(&mut self, binding: DnsBinding) {
        self.by_ip.insert(binding.ip, binding);
    }

    pub fn lookup(&self, ip: IpAddr, now: Instant) -> Option<&DnsBinding> {
        self.by_ip
            .get(&ip)
            .filter(|binding| !binding.is_expired(now))
    }

    pub fn purge_expired(&mut self, now: Instant) {
        self.by_ip.retain(|_, binding| !binding.is_expired(now));
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
    use super::{DnsBinding, DnsBindingStore};
    use std::net::{IpAddr, Ipv4Addr};
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
    }
}
