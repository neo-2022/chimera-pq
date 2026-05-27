#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedPolicyFragment {
    pub issuer: String,
    pub policy_id: String,
    pub version: u64,
    pub payload: String,
    pub signature: String,
}

#[derive(Debug, Default)]
pub struct PolicyFragmentStore {
    fragments: Vec<SignedPolicyFragment>,
}

impl SignedPolicyFragment {
    pub fn validate(&self) -> Result<(), String> {
        if self.issuer.is_empty() {
            return Err("policy fragment issuer is empty".to_string());
        }
        if self.policy_id.is_empty() {
            return Err("policy fragment id is empty".to_string());
        }
        if self.signature.len() < 8 {
            return Err("policy fragment signature is too short".to_string());
        }
        Ok(())
    }
}

impl PolicyFragmentStore {
    pub fn insert(&mut self, fragment: SignedPolicyFragment) -> Result<(), String> {
        fragment.validate()?;

        if self
            .fragments
            .iter()
            .any(|item| item.policy_id == fragment.policy_id && item.version == fragment.version)
        {
            return Err("duplicate policy fragment version".to_string());
        }

        self.fragments.push(fragment);
        Ok(())
    }

    pub fn latest_by_id(&self, policy_id: &str) -> Option<&SignedPolicyFragment> {
        self.fragments
            .iter()
            .filter(|item| item.policy_id == policy_id)
            .max_by_key(|item| item.version)
    }
}

#[cfg(test)]
mod tests {
    use super::{PolicyFragmentStore, SignedPolicyFragment};

    #[test]
    fn insert_and_resolve_latest_version() {
        let mut store = PolicyFragmentStore::default();

        let v1 = SignedPolicyFragment {
            issuer: "issuer-a".to_string(),
            policy_id: "policy-main".to_string(),
            version: 1,
            payload: "allow=direct".to_string(),
            signature: "abcdef123456".to_string(),
        };
        let v2 = SignedPolicyFragment {
            version: 2,
            payload: "allow=gateway".to_string(),
            ..v1.clone()
        };

        assert!(store.insert(v1).is_ok());
        assert!(store.insert(v2).is_ok());

        let latest = match store.latest_by_id("policy-main") {
            Some(value) => value,
            None => unreachable!("latest fragment should exist"),
        };
        assert_eq!(latest.version, 2);
        assert_eq!(latest.payload, "allow=gateway");
    }

    #[test]
    fn duplicate_version_is_rejected() {
        let mut store = PolicyFragmentStore::default();

        let fragment = SignedPolicyFragment {
            issuer: "issuer-a".to_string(),
            policy_id: "policy-main".to_string(),
            version: 1,
            payload: "allow=direct".to_string(),
            signature: "abcdef123456".to_string(),
        };

        assert!(store.insert(fragment.clone()).is_ok());
        assert!(store.insert(fragment).is_err());
    }
}
