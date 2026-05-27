#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoamingEntry {
    pub namespace: String,
    pub gateway_hint: String,
    pub expires_epoch: u64,
}

#[derive(Debug, Default)]
pub struct RoamingCache {
    entries: Vec<RoamingEntry>,
}

impl RoamingCache {
    pub fn insert(&mut self, entry: RoamingEntry) -> Result<(), String> {
        if entry.namespace.trim().is_empty() {
            return Err("roaming entry namespace is empty".to_string());
        }
        if entry.gateway_hint.trim().is_empty() {
            return Err("roaming entry gateway_hint is empty".to_string());
        }
        self.entries
            .retain(|value| value.namespace != entry.namespace);
        self.entries.push(entry);
        Ok(())
    }

    pub fn resolve_active(&self, namespace: &str, now_epoch: u64) -> Option<&RoamingEntry> {
        self.entries
            .iter()
            .find(|entry| entry.namespace == namespace && entry.expires_epoch > now_epoch)
    }
}

#[cfg(test)]
mod tests {
    use super::{RoamingCache, RoamingEntry};

    #[test]
    fn insert_and_resolve_active_entry() {
        let mut cache = RoamingCache::default();
        let inserted = cache.insert(RoamingEntry {
            namespace: "cef-public".to_string(),
            gateway_hint: "198.51.100.9:443".to_string(),
            expires_epoch: 10_000,
        });
        assert!(inserted.is_ok());

        let found = cache.resolve_active("cef-public", 9_000);
        assert!(found.is_some());
    }

    #[test]
    fn resolve_returns_none_for_expired_entry() {
        let mut cache = RoamingCache::default();
        let inserted = cache.insert(RoamingEntry {
            namespace: "cef-public".to_string(),
            gateway_hint: "198.51.100.9:443".to_string(),
            expires_epoch: 100,
        });
        assert!(inserted.is_ok());

        let found = cache.resolve_active("cef-public", 100);
        assert!(found.is_none());
    }

    #[test]
    fn empty_namespace_is_rejected() {
        let mut cache = RoamingCache::default();
        let inserted = cache.insert(RoamingEntry {
            namespace: "".to_string(),
            gateway_hint: "198.51.100.9:443".to_string(),
            expires_epoch: 1,
        });
        assert!(inserted.is_err());
    }
}
