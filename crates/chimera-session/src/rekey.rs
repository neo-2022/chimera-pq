use chimera_core::{ChimeraError, ChimeraResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RekeyPolicy {
    pub max_session_age_seconds: u64,
    pub max_packets_per_key: u64,
}

impl RekeyPolicy {
    pub fn validate(self) -> ChimeraResult<Self> {
        if self.max_session_age_seconds == 0 {
            return Err(ChimeraError::InvalidConfig(
                "rekey max_session_age_seconds must be greater than zero".to_string(),
            ));
        }
        if self.max_packets_per_key == 0 {
            return Err(ChimeraError::InvalidConfig(
                "rekey max_packets_per_key must be greater than zero".to_string(),
            ));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RekeyState {
    policy: RekeyPolicy,
    established_at_seconds: u64,
    sent_packets_with_current_key: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RekeyReason {
    SessionAgeExceeded,
    PacketLimitExceeded,
}

impl RekeyState {
    pub fn new(policy: RekeyPolicy, established_at_seconds: u64) -> ChimeraResult<Self> {
        Ok(Self {
            policy: policy.validate()?,
            established_at_seconds,
            sent_packets_with_current_key: 0,
        })
    }

    pub fn on_packet_sent(&mut self) {
        self.sent_packets_with_current_key = self.sent_packets_with_current_key.saturating_add(1);
    }

    pub fn rekey_reason(&self, now_seconds: u64) -> Option<RekeyReason> {
        let age_seconds = now_seconds.saturating_sub(self.established_at_seconds);
        if age_seconds >= self.policy.max_session_age_seconds {
            return Some(RekeyReason::SessionAgeExceeded);
        }
        if self.sent_packets_with_current_key >= self.policy.max_packets_per_key {
            return Some(RekeyReason::PacketLimitExceeded);
        }
        None
    }

    pub fn should_rekey(&self, now_seconds: u64) -> bool {
        self.rekey_reason(now_seconds).is_some()
    }

    pub fn reset_after_rekey(&mut self, now_seconds: u64) {
        self.established_at_seconds = now_seconds;
        self.sent_packets_with_current_key = 0;
    }
}
