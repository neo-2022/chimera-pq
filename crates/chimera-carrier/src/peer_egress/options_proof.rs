use std::env;

use crate::peer_egress::options_mode::Mode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransitProofOptions {
    pub payload_bytes: usize,
    pub packet_number: u64,
    pub route_id: Option<u64>,
    pub lane_index: Option<usize>,
}

impl TransitProofOptions {
    pub(crate) fn from_env() -> Result<Self, String> {
        let mut options = Self::default();
        if let Some(value) = env_value("CHIMERA_PEER_EGRESS_TRANSIT_PAYLOAD_BYTES") {
            options.payload_bytes = parse_positive_usize(&value, "transit-payload-bytes")?;
        }
        if let Some(value) = env_value("CHIMERA_PEER_EGRESS_TRANSIT_PACKET_NUMBER") {
            options.packet_number = parse_nonnegative_u64(&value, "transit-packet-number")?;
        }
        if let Some(value) = env_value("CHIMERA_PEER_EGRESS_TRANSIT_ROUTE_ID") {
            options.route_id = Some(parse_positive_u64(&value, "transit-route-id")?);
        }
        if let Some(value) = env_value("CHIMERA_PEER_EGRESS_TRANSIT_LANE_INDEX") {
            options.lane_index = Some(parse_nonnegative_usize(&value, "transit-lane-index")?);
        }
        Ok(options)
    }

    pub(crate) fn is_flag(flag: &str) -> bool {
        matches!(
            flag,
            "--transit-payload-bytes"
                | "--transit-packet-number"
                | "--transit-route-id"
                | "--transit-lane-index"
        )
    }

    pub(crate) fn apply_flag(&mut self, flag: &str, value: &str) -> Result<bool, String> {
        match flag {
            "--transit-payload-bytes" => {
                self.payload_bytes = parse_positive_usize(value, "transit-payload-bytes")?;
                Ok(true)
            }
            "--transit-packet-number" => {
                self.packet_number = parse_nonnegative_u64(value, "transit-packet-number")?;
                Ok(true)
            }
            "--transit-route-id" => {
                self.route_id = Some(parse_positive_u64(value, "transit-route-id")?);
                Ok(true)
            }
            "--transit-lane-index" => {
                self.lane_index = Some(parse_nonnegative_usize(value, "transit-lane-index")?);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub(crate) fn validate_for_mode(&self, mode: &Mode) -> Result<(), String> {
        if mode != &Mode::BoundTransitInject {
            return Ok(());
        }
        if self.route_id.is_none() {
            return Err(
                "bound transit inject mode requires --transit-route-id or CHIMERA_PEER_EGRESS_TRANSIT_ROUTE_ID"
                    .to_string(),
            );
        }
        if self.lane_index.is_none() {
            return Err(
                "bound transit inject mode requires --transit-lane-index or CHIMERA_PEER_EGRESS_TRANSIT_LANE_INDEX"
                    .to_string(),
            );
        }
        Ok(())
    }
}

impl Default for TransitProofOptions {
    fn default() -> Self {
        Self {
            payload_bytes: 64,
            packet_number: 1,
            route_id: None,
            lane_index: None,
        }
    }
}

fn env_value(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_positive_usize(value: &str, name: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{name} must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{name} must be positive"));
    }
    Ok(parsed)
}

fn parse_nonnegative_usize(value: &str, name: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("{name} must be a non-negative integer"))
}

fn parse_positive_u64(value: &str, name: &str) -> Result<u64, String> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| format!("{name} must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{name} must be positive"));
    }
    Ok(parsed)
}

fn parse_nonnegative_u64(value: &str, name: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("{name} must be a non-negative integer"))
}
