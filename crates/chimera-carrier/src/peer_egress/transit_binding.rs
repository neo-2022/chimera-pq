use core::fmt;
use std::num::{NonZeroU16, NonZeroU64};

use crate::peer_egress::transit::{TransitRelayFrame, validate_transit_relay_frame};

pub const BOUND_TRANSIT_MAGIC: u8 = 0x80;
const BOUND_TRANSIT_VERSION: u8 = 1;
pub const BOUND_TRANSIT_HEADER_LEN: usize = 1 + 1 + 8 + 2;
pub const BOUND_TRANSIT_HEADER_REST_LEN: usize = BOUND_TRANSIT_HEADER_LEN - 1;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitRouteId(NonZeroU64);

impl TransitRouteId {
    pub fn new(value: u64) -> Result<Self, String> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or_else(|| "sealed transit route binding id must be nonzero".to_string())
    }

    pub fn get(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Debug for TransitRouteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TransitRouteId(<opaque>)")
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitLaneId(NonZeroU16);

impl TransitLaneId {
    pub fn new(value: u16) -> Result<Self, String> {
        NonZeroU16::new(value)
            .map(Self)
            .ok_or_else(|| "sealed transit lane binding id must be nonzero".to_string())
    }

    pub fn from_zero_based_lane_index(index: usize) -> Result<Self, String> {
        let value = index
            .checked_add(1)
            .ok_or_else(|| "sealed transit lane binding index overflow".to_string())?;
        let value = u16::try_from(value)
            .map_err(|_| "sealed transit lane binding id overflow".to_string())?;
        Self::new(value)
    }

    pub fn get(self) -> u16 {
        self.0.get()
    }
}

impl fmt::Debug for TransitLaneId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TransitLaneId(<opaque>)")
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitPathBinding {
    route_id: TransitRouteId,
    lane_id: TransitLaneId,
}

impl TransitPathBinding {
    pub fn new(route_id: TransitRouteId, lane_id: TransitLaneId) -> Self {
        Self { route_id, lane_id }
    }

    pub fn route_id(self) -> TransitRouteId {
        self.route_id
    }

    pub fn lane_id(self) -> TransitLaneId {
        self.lane_id
    }
}

impl fmt::Debug for TransitPathBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransitPathBinding")
            .field("route_id", &self.route_id)
            .field("lane_id", &self.lane_id)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundTransitRelayFrame {
    binding: TransitPathBinding,
    frame: TransitRelayFrame,
}

impl BoundTransitRelayFrame {
    pub fn new(binding: TransitPathBinding, frame: TransitRelayFrame) -> Self {
        Self { binding, frame }
    }

    pub fn binding(&self) -> TransitPathBinding {
        self.binding
    }

    pub fn frame(&self) -> &TransitRelayFrame {
        &self.frame
    }

    pub fn into_frame(self) -> TransitRelayFrame {
        self.frame
    }
}

pub fn validate_bound_transit_relay_frame(input: &[u8]) -> Result<BoundTransitRelayFrame, String> {
    if input.len() < BOUND_TRANSIT_HEADER_LEN {
        return Err("bound sealed transit frame truncated".to_string());
    }
    if input[0] != BOUND_TRANSIT_MAGIC {
        return Err("bound sealed transit frame magic invalid".to_string());
    }
    if input[1] != BOUND_TRANSIT_VERSION {
        return Err("bound sealed transit frame version invalid".to_string());
    }
    let route_id = u64::from_be_bytes(
        input[2..10]
            .try_into()
            .map_err(|_| "bound sealed transit route id invalid".to_string())?,
    );
    let lane_id = u16::from_be_bytes(
        input[10..12]
            .try_into()
            .map_err(|_| "bound sealed transit lane id invalid".to_string())?,
    );
    let binding =
        TransitPathBinding::new(TransitRouteId::new(route_id)?, TransitLaneId::new(lane_id)?);
    let frame = validate_transit_relay_frame(&input[BOUND_TRANSIT_HEADER_LEN..])?;
    Ok(BoundTransitRelayFrame { binding, frame })
}

pub fn encode_bound_transit_relay_frame(frame: &BoundTransitRelayFrame) -> Vec<u8> {
    let sealed = frame.frame().sealed_bytes();
    let mut out = Vec::with_capacity(BOUND_TRANSIT_HEADER_LEN + sealed.len());
    out.push(BOUND_TRANSIT_MAGIC);
    out.push(BOUND_TRANSIT_VERSION);
    out.extend_from_slice(&frame.binding().route_id().get().to_be_bytes());
    out.extend_from_slice(&frame.binding().lane_id().get().to_be_bytes());
    out.extend_from_slice(sealed);
    out
}

#[cfg(test)]
mod tests {
    use super::{
        BoundTransitRelayFrame, TransitLaneId, TransitPathBinding, TransitRouteId,
        encode_bound_transit_relay_frame, validate_bound_transit_relay_frame,
    };
    use crate::peer_egress::transit::validate_transit_relay_frame;
    use chimera_session::{Frame, FrameKind};

    fn encoded_frame(payload: &[u8]) -> Vec<u8> {
        match (Frame {
            kind: FrameKind::Data,
            packet_number: 77,
            payload: payload.to_vec(),
        })
        .encode()
        {
            Ok(encoded) => encoded,
            Err(error) => unreachable!("frame must encode: {error}"),
        }
    }

    fn binding() -> TransitPathBinding {
        TransitPathBinding::new(
            TransitRouteId::new(7).unwrap_or_else(|e| unreachable!("{e}")),
            TransitLaneId::new(2).unwrap_or_else(|e| unreachable!("{e}")),
        )
    }

    #[test]
    fn bound_transit_frame_round_trips_and_preserves_sealed_bytes() -> Result<(), String> {
        let sealed = encoded_frame(b"closed payload");
        let frame = validate_transit_relay_frame(&sealed)?;
        let bound = BoundTransitRelayFrame::new(binding(), frame);
        let encoded = encode_bound_transit_relay_frame(&bound);
        let parsed = validate_bound_transit_relay_frame(&encoded)?;

        assert_eq!(parsed.binding(), binding());
        assert_eq!(parsed.frame().sealed_bytes(), sealed.as_slice());
        Ok(())
    }

    #[test]
    fn bound_transit_frame_rejects_zero_ids() -> Result<(), String> {
        assert!(TransitRouteId::new(0).is_err());
        assert!(TransitLaneId::new(0).is_err());

        let sealed = encoded_frame(b"closed payload");
        let frame = validate_transit_relay_frame(&sealed)?;
        let bound = BoundTransitRelayFrame::new(binding(), frame);
        let mut encoded = encode_bound_transit_relay_frame(&bound);
        encoded[9] = 0;
        encoded[10] = 0;
        encoded[11] = 0;
        assert!(validate_bound_transit_relay_frame(&encoded).is_err());
        Ok(())
    }

    #[test]
    fn bound_transit_frame_rejects_bad_version_and_truncated_input() -> Result<(), String> {
        let sealed = encoded_frame(b"closed payload");
        let frame = validate_transit_relay_frame(&sealed)?;
        let bound = BoundTransitRelayFrame::new(binding(), frame);
        let mut encoded = encode_bound_transit_relay_frame(&bound);
        encoded[1] = 2;
        assert!(validate_bound_transit_relay_frame(&encoded).is_err());
        assert!(validate_bound_transit_relay_frame(&encoded[..3]).is_err());
        Ok(())
    }

    #[test]
    fn bound_transit_debug_redacts_binding_and_payload() -> Result<(), String> {
        let sealed = encoded_frame(b"SECRET_BOUND_PAYLOAD");
        let frame = validate_transit_relay_frame(&sealed)?;
        let bound = BoundTransitRelayFrame::new(binding(), frame);
        let debug = format!("{bound:?}");

        assert!(debug.contains("<opaque>"));
        assert!(debug.contains("<sealed>"));
        assert!(!debug.contains("SECRET_BOUND_PAYLOAD"));
        assert!(!debug.contains("route_id: 7"));
        assert!(!debug.contains("lane_id: 2"));
        Ok(())
    }
}
