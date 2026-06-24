use crate::peer_egress::transit_guard::{
    DEFAULT_TRANSIT_IDLE_TIMEOUT_MS, DEFAULT_TRANSIT_MAX_BYTES_PER_DIRECTION,
    DEFAULT_TRANSIT_MAX_FRAMES_PER_DIRECTION, TransitRelayLimits,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TransitRelayGuardOptionValues {
    pub(super) max_frames_per_direction: u64,
    pub(super) max_bytes_per_direction: u64,
    pub(super) idle_timeout_ms: u64,
}

impl TransitRelayGuardOptionValues {
    pub(super) fn from_env() -> Result<Self, String> {
        Ok(Self {
            max_frames_per_direction: super::options::env_value(
                "CHIMERA_PEER_EGRESS_TRANSIT_MAX_FRAMES_PER_DIRECTION",
            )
            .map(|value| {
                super::options::parse_positive_u64(&value, "transit-max-frames-per-direction")
            })
            .transpose()?
            .unwrap_or(DEFAULT_TRANSIT_MAX_FRAMES_PER_DIRECTION),
            max_bytes_per_direction: super::options::env_value(
                "CHIMERA_PEER_EGRESS_TRANSIT_MAX_BYTES_PER_DIRECTION",
            )
            .map(|value| {
                super::options::parse_positive_u64(&value, "transit-max-bytes-per-direction")
            })
            .transpose()?
            .unwrap_or(DEFAULT_TRANSIT_MAX_BYTES_PER_DIRECTION),
            idle_timeout_ms: super::options::env_value(
                "CHIMERA_PEER_EGRESS_TRANSIT_IDLE_TIMEOUT_MS",
            )
            .map(|value| super::options::parse_positive_u64(&value, "transit-idle-timeout-ms"))
            .transpose()?
            .unwrap_or(DEFAULT_TRANSIT_IDLE_TIMEOUT_MS),
        })
    }

    pub(super) fn apply_flag(&mut self, flag: &str, value: &str) -> Result<bool, String> {
        match flag {
            "--transit-max-frames-per-direction" => {
                self.max_frames_per_direction =
                    super::options::parse_positive_u64(value, "transit-max-frames-per-direction")?;
                Ok(true)
            }
            "--transit-max-bytes-per-direction" => {
                self.max_bytes_per_direction =
                    super::options::parse_positive_u64(value, "transit-max-bytes-per-direction")?;
                Ok(true)
            }
            "--transit-idle-timeout-ms" => {
                self.idle_timeout_ms =
                    super::options::parse_positive_u64(value, "transit-idle-timeout-ms")?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub(super) fn limits(self) -> Result<TransitRelayLimits, String> {
        TransitRelayLimits::new(
            self.max_frames_per_direction,
            self.max_bytes_per_direction,
            self.idle_timeout_ms,
        )
    }
}
