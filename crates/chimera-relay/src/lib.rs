#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayConsentMode {
    Disabled,
    TrustedOnly,
    BoundedPublic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayPolicy {
    pub mode: RelayConsentMode,
    pub max_concurrent_flows: u32,
    pub max_mbps: u32,
}

impl RelayPolicy {
    pub fn validate(&self) -> Result<(), String> {
        match self.mode {
            RelayConsentMode::Disabled => Ok(()),
            RelayConsentMode::TrustedOnly | RelayConsentMode::BoundedPublic => {
                if self.max_concurrent_flows == 0 {
                    return Err("relay max_concurrent_flows must be > 0".to_string());
                }
                if self.max_mbps == 0 {
                    return Err("relay max_mbps must be > 0".to_string());
                }
                Ok(())
            }
        }
    }

    pub fn allows_flow(&self, active_flows: u32) -> bool {
        match self.mode {
            RelayConsentMode::Disabled => false,
            RelayConsentMode::TrustedOnly | RelayConsentMode::BoundedPublic => {
                active_flows < self.max_concurrent_flows
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RelayConsentMode, RelayPolicy};

    #[test]
    fn disabled_mode_is_valid_without_limits() {
        let policy = RelayPolicy {
            mode: RelayConsentMode::Disabled,
            max_concurrent_flows: 0,
            max_mbps: 0,
        };
        assert!(policy.validate().is_ok());
        assert!(!policy.allows_flow(0));
    }

    #[test]
    fn trusted_mode_requires_positive_limits() {
        let invalid = RelayPolicy {
            mode: RelayConsentMode::TrustedOnly,
            max_concurrent_flows: 0,
            max_mbps: 10,
        };
        assert!(invalid.validate().is_err());

        let valid = RelayPolicy {
            mode: RelayConsentMode::TrustedOnly,
            max_concurrent_flows: 2,
            max_mbps: 10,
        };
        assert!(valid.validate().is_ok());
        assert!(valid.allows_flow(1));
        assert!(!valid.allows_flow(2));
    }

    #[test]
    fn bounded_public_mode_obeys_capacity() {
        let policy = RelayPolicy {
            mode: RelayConsentMode::BoundedPublic,
            max_concurrent_flows: 3,
            max_mbps: 50,
        };

        assert!(policy.validate().is_ok());
        assert!(policy.allows_flow(0));
        assert!(policy.allows_flow(2));
        assert!(!policy.allows_flow(3));
    }
}
