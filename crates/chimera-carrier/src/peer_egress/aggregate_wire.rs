use core::fmt;
use std::num::NonZeroU64;

use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};

pub const AGGREGATE_TRANSIT_MAGIC: u8 = 0x81;
const AGGREGATE_TRANSIT_VERSION: u8 = 1;
const SEALED_FRAME_HEADER_LEN: usize = 14;
pub const MAX_AGGREGATE_OBJECT_LEN: usize =
    chimera_session::MAX_PAYLOAD_LEN + SEALED_FRAME_HEADER_LEN;
pub const MAX_AGGREGATE_SHARD_COUNT: usize = 256;

const AGGREGATE_TRANSIT_HEADER_LEN: usize = 1 + 1 + 8 + 2 + 8 + 4 + 2 + 2 + 4 + 4;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AggregateObjectId(NonZeroU64);

impl AggregateObjectId {
    pub fn new(value: u64) -> Result<Self, String> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or_else(|| "aggregate transit object id must be nonzero".to_string())
    }

    pub fn get(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Debug for AggregateObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AggregateObjectId(<opaque>)")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct AggregateTransitShardFrame {
    binding: TransitPathBinding,
    aggregate_id: AggregateObjectId,
    object_len: usize,
    shard_count: u16,
    shard_index: u16,
    byte_offset: usize,
    shard_bytes: Vec<u8>,
}

impl fmt::Debug for AggregateTransitShardFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AggregateTransitShardFrame")
            .field("binding", &"<opaque>")
            .field("aggregate_id", &self.aggregate_id)
            .field("object_len", &"<redacted>")
            .field("shard_count", &self.shard_count)
            .field("shard_index", &"<redacted>")
            .field("byte_range", &"<redacted>")
            .field("shard_bytes", &"<sealed>")
            .finish()
    }
}

impl AggregateTransitShardFrame {
    pub fn new(
        binding: TransitPathBinding,
        aggregate_id: AggregateObjectId,
        object_len: usize,
        shard_count: u16,
        shard_index: u16,
        byte_offset: usize,
        shard_bytes: Vec<u8>,
    ) -> Result<Self, String> {
        validate_shard_fields(
            object_len,
            shard_count,
            shard_index,
            byte_offset,
            shard_bytes.len(),
        )?;
        Ok(Self {
            binding,
            aggregate_id,
            object_len,
            shard_count,
            shard_index,
            byte_offset,
            shard_bytes,
        })
    }

    pub fn binding(&self) -> TransitPathBinding {
        self.binding
    }

    pub fn aggregate_id(&self) -> AggregateObjectId {
        self.aggregate_id
    }

    pub fn object_len(&self) -> usize {
        self.object_len
    }

    pub fn shard_count(&self) -> u16 {
        self.shard_count
    }

    pub fn shard_index(&self) -> u16 {
        self.shard_index
    }

    pub fn byte_offset(&self) -> usize {
        self.byte_offset
    }

    pub fn shard_bytes(&self) -> &[u8] {
        &self.shard_bytes
    }
}

pub fn encode_aggregate_transit_shard_frame(frame: &AggregateTransitShardFrame) -> Vec<u8> {
    let mut out = Vec::with_capacity(AGGREGATE_TRANSIT_HEADER_LEN + frame.shard_bytes().len());
    out.push(AGGREGATE_TRANSIT_MAGIC);
    out.push(AGGREGATE_TRANSIT_VERSION);
    out.extend_from_slice(&frame.binding().route_id().get().to_be_bytes());
    out.extend_from_slice(&frame.binding().lane_id().get().to_be_bytes());
    out.extend_from_slice(&frame.aggregate_id().get().to_be_bytes());
    out.extend_from_slice(&(frame.object_len() as u32).to_be_bytes());
    out.extend_from_slice(&frame.shard_count().to_be_bytes());
    out.extend_from_slice(&frame.shard_index().to_be_bytes());
    out.extend_from_slice(&(frame.byte_offset() as u32).to_be_bytes());
    out.extend_from_slice(&(frame.shard_bytes().len() as u32).to_be_bytes());
    out.extend_from_slice(frame.shard_bytes());
    out
}

pub fn validate_aggregate_transit_shard_frame(
    input: &[u8],
) -> Result<AggregateTransitShardFrame, String> {
    if input.len() < AGGREGATE_TRANSIT_HEADER_LEN {
        return Err("aggregate transit shard frame truncated".to_string());
    }
    if input[0] != AGGREGATE_TRANSIT_MAGIC {
        return Err("aggregate transit shard frame magic invalid".to_string());
    }
    if input[1] != AGGREGATE_TRANSIT_VERSION {
        return Err("aggregate transit shard frame version invalid".to_string());
    }

    let route_id = read_u64(&input[2..10], "route id")?;
    let lane_id = read_u16(&input[10..12], "lane id")?;
    let aggregate_id = read_u64(&input[12..20], "object id")?;
    let object_len = read_u32(&input[20..24], "object length")? as usize;
    let shard_count = read_u16(&input[24..26], "shard count")?;
    let shard_index = read_u16(&input[26..28], "shard index")?;
    let byte_offset = read_u32(&input[28..32], "byte offset")? as usize;
    let shard_len = read_u32(&input[32..36], "shard length")? as usize;

    let expected_len = AGGREGATE_TRANSIT_HEADER_LEN
        .checked_add(shard_len)
        .ok_or_else(|| "aggregate transit shard frame length overflow".to_string())?;
    if input.len() != expected_len {
        return Err("aggregate transit shard frame length mismatch".to_string());
    }

    let binding =
        TransitPathBinding::new(TransitRouteId::new(route_id)?, TransitLaneId::new(lane_id)?);
    AggregateTransitShardFrame::new(
        binding,
        AggregateObjectId::new(aggregate_id)?,
        object_len,
        shard_count,
        shard_index,
        byte_offset,
        input[AGGREGATE_TRANSIT_HEADER_LEN..].to_vec(),
    )
}

fn validate_shard_fields(
    object_len: usize,
    shard_count: u16,
    shard_index: u16,
    byte_offset: usize,
    shard_len: usize,
) -> Result<(), String> {
    if object_len == 0 {
        return Err("aggregate transit object length must be nonzero".to_string());
    }
    if object_len > MAX_AGGREGATE_OBJECT_LEN {
        return Err("aggregate transit object length exceeds limit".to_string());
    }
    if shard_count == 0 || usize::from(shard_count) > MAX_AGGREGATE_SHARD_COUNT {
        return Err("aggregate transit shard count invalid".to_string());
    }
    if shard_index >= shard_count {
        return Err("aggregate transit shard index invalid".to_string());
    }
    if shard_len == 0 {
        return Err("aggregate transit shard must be nonempty".to_string());
    }
    let end = byte_offset
        .checked_add(shard_len)
        .ok_or_else(|| "aggregate transit shard range overflow".to_string())?;
    if end > object_len {
        return Err("aggregate transit shard range outside object".to_string());
    }
    Ok(())
}

fn read_u64(bytes: &[u8], field: &str) -> Result<u64, String> {
    let array: [u8; 8] = bytes
        .try_into()
        .map_err(|_| format!("aggregate transit shard {field} invalid"))?;
    Ok(u64::from_be_bytes(array))
}

fn read_u32(bytes: &[u8], field: &str) -> Result<u32, String> {
    let array: [u8; 4] = bytes
        .try_into()
        .map_err(|_| format!("aggregate transit shard {field} invalid"))?;
    Ok(u32::from_be_bytes(array))
}

fn read_u16(bytes: &[u8], field: &str) -> Result<u16, String> {
    let array: [u8; 2] = bytes
        .try_into()
        .map_err(|_| format!("aggregate transit shard {field} invalid"))?;
    Ok(u16::from_be_bytes(array))
}

#[cfg(test)]
mod tests {
    use super::{
        AggregateObjectId, AggregateTransitShardFrame, encode_aggregate_transit_shard_frame,
        validate_aggregate_transit_shard_frame,
    };
    use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};

    fn binding() -> TransitPathBinding {
        TransitPathBinding::new(
            TransitRouteId::new(7).unwrap_or_else(|error| unreachable!("{error}")),
            TransitLaneId::new(2).unwrap_or_else(|error| unreachable!("{error}")),
        )
    }

    fn object_id() -> AggregateObjectId {
        AggregateObjectId::new(99).unwrap_or_else(|error| unreachable!("{error}"))
    }

    #[test]
    fn aggregate_transit_shard_round_trips_without_debug_payload_leak() -> Result<(), String> {
        let frame = AggregateTransitShardFrame::new(
            binding(),
            object_id(),
            32,
            2,
            1,
            16,
            b"SECRET_SHARD_001".to_vec(),
        )?;
        let encoded = encode_aggregate_transit_shard_frame(&frame);
        let parsed = validate_aggregate_transit_shard_frame(&encoded)?;

        assert_eq!(parsed.binding(), binding());
        assert_eq!(parsed.aggregate_id(), object_id());
        assert_eq!(parsed.object_len(), 32);
        assert_eq!(parsed.shard_count(), 2);
        assert_eq!(parsed.shard_index(), 1);
        assert_eq!(parsed.byte_offset(), 16);
        assert_eq!(parsed.shard_bytes(), b"SECRET_SHARD_001");

        let debug = format!("{parsed:?}");
        assert!(debug.contains("<sealed>"));
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("SECRET_SHARD_001"));
        assert!(!debug.contains("16"));
        Ok(())
    }

    #[test]
    fn aggregate_transit_shard_rejects_bad_header_and_ids() -> Result<(), String> {
        let frame = AggregateTransitShardFrame::new(
            binding(),
            object_id(),
            8,
            1,
            0,
            0,
            b"sealed-1".to_vec(),
        )?;
        let mut encoded = encode_aggregate_transit_shard_frame(&frame);
        encoded[0] = 0x7f;
        assert!(validate_aggregate_transit_shard_frame(&encoded).is_err());

        let mut encoded = encode_aggregate_transit_shard_frame(&frame);
        encoded[1] = 9;
        assert!(validate_aggregate_transit_shard_frame(&encoded).is_err());

        let mut encoded = encode_aggregate_transit_shard_frame(&frame);
        encoded[19] = 0;
        assert!(validate_aggregate_transit_shard_frame(&encoded).is_err());
        Ok(())
    }

    #[test]
    fn aggregate_transit_shard_rejects_invalid_ranges() {
        assert!(
            AggregateTransitShardFrame::new(binding(), object_id(), 8, 1, 0, 8, b"x".to_vec())
                .is_err()
        );
        assert!(
            AggregateTransitShardFrame::new(binding(), object_id(), 8, 1, 1, 0, b"x".to_vec())
                .is_err()
        );
        assert!(
            AggregateTransitShardFrame::new(binding(), object_id(), 8, 0, 0, 0, b"x".to_vec())
                .is_err()
        );
        assert!(
            AggregateTransitShardFrame::new(binding(), object_id(), 8, 1, 0, 0, Vec::new())
                .is_err()
        );
    }

    #[test]
    fn aggregate_transit_shard_rejects_length_mismatch() -> Result<(), String> {
        let frame = AggregateTransitShardFrame::new(
            binding(),
            object_id(),
            8,
            1,
            0,
            0,
            b"sealed-1".to_vec(),
        )?;
        let mut encoded = encode_aggregate_transit_shard_frame(&frame);
        let _ = encoded.pop();
        assert!(validate_aggregate_transit_shard_frame(&encoded).is_err());
        Ok(())
    }
}
