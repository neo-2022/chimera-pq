use core::fmt;

use crate::peer_egress::aggregate_wire::{
    AggregateObjectId, AggregateTransitShardFrame, MAX_AGGREGATE_OBJECT_LEN,
    MAX_AGGREGATE_SHARD_COUNT,
};
use crate::peer_egress::transit::{TransitRelayFrame, validate_transit_relay_frame};
use crate::peer_egress::transit_binding::TransitPathBinding;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggregateTransitReassemblyLimits {
    pub max_object_len: usize,
    pub max_shard_count: usize,
}

impl Default for AggregateTransitReassemblyLimits {
    fn default() -> Self {
        Self {
            max_object_len: MAX_AGGREGATE_OBJECT_LEN,
            max_shard_count: MAX_AGGREGATE_SHARD_COUNT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateTransitReassemblyStatus {
    Pending,
    Complete(TransitRelayFrame),
}

#[derive(Default)]
pub struct AggregateTransitObjectReassembler {
    limits: AggregateTransitReassemblyLimits,
    state: Option<ReassemblyState>,
    completed: bool,
}

impl fmt::Debug for AggregateTransitObjectReassembler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("AggregateTransitObjectReassembler");
        debug.field("completed", &self.completed);
        match self.state.as_ref() {
            Some(state) => {
                debug
                    .field("aggregate_id", &state.aggregate_id)
                    .field("binding", &"<opaque>")
                    .field("object_len", &"<redacted>")
                    .field("shard_count", &state.shard_count)
                    .field("received_shards", &state.received_shards);
            }
            None => {
                debug.field("state", &"empty");
            }
        }
        debug.finish()
    }
}

impl AggregateTransitObjectReassembler {
    pub fn new(limits: AggregateTransitReassemblyLimits) -> Result<Self, String> {
        if limits.max_object_len == 0 || limits.max_object_len > MAX_AGGREGATE_OBJECT_LEN {
            return Err("aggregate reassembly object limit invalid".to_string());
        }
        if limits.max_shard_count == 0 || limits.max_shard_count > MAX_AGGREGATE_SHARD_COUNT {
            return Err("aggregate reassembly shard limit invalid".to_string());
        }
        Ok(Self {
            limits,
            state: None,
            completed: false,
        })
    }

    pub fn accept(
        &mut self,
        shard: AggregateTransitShardFrame,
    ) -> Result<AggregateTransitReassemblyStatus, String> {
        if self.completed {
            return Err("aggregate reassembly already complete".to_string());
        }
        validate_limits(&self.limits, &shard)?;
        if self.state.is_none() {
            self.state = Some(ReassemblyState::new(&self.limits, &shard)?);
        }
        let state = self
            .state
            .as_mut()
            .ok_or_else(|| "aggregate reassembly state unavailable".to_string())?;
        state.accept(shard)?;
        if !state.is_complete() {
            return Ok(AggregateTransitReassemblyStatus::Pending);
        }
        let sealed = state.reassemble_sealed_bytes()?;
        let frame = validate_transit_relay_frame(&sealed)
            .map_err(|_| "aggregate reassembly sealed frame invalid".to_string())?;
        self.completed = true;
        Ok(AggregateTransitReassemblyStatus::Complete(frame))
    }
}

struct ReassemblyState {
    binding: TransitPathBinding,
    aggregate_id: AggregateObjectId,
    object_len: usize,
    shard_count: usize,
    received_shards: usize,
    shards: Vec<Option<AcceptedShard>>,
}

impl ReassemblyState {
    fn new(
        limits: &AggregateTransitReassemblyLimits,
        first: &AggregateTransitShardFrame,
    ) -> Result<Self, String> {
        let shard_count = usize::from(first.shard_count());
        if first.object_len() > limits.max_object_len || shard_count > limits.max_shard_count {
            return Err("aggregate reassembly limits exceeded".to_string());
        }
        Ok(Self {
            binding: first.binding(),
            aggregate_id: first.aggregate_id(),
            object_len: first.object_len(),
            shard_count,
            received_shards: 0,
            shards: vec![None; shard_count],
        })
    }

    fn accept(&mut self, shard: AggregateTransitShardFrame) -> Result<(), String> {
        if shard.binding() != self.binding {
            return Err("aggregate reassembly binding mismatch".to_string());
        }
        if shard.aggregate_id() != self.aggregate_id
            || shard.object_len() != self.object_len
            || usize::from(shard.shard_count()) != self.shard_count
        {
            return Err("aggregate reassembly identity mismatch".to_string());
        }
        let index = usize::from(shard.shard_index());
        if index >= self.shards.len() {
            return Err("aggregate reassembly shard index invalid".to_string());
        }
        if self.shards[index].is_some() {
            return Err("aggregate reassembly duplicate shard".to_string());
        }
        let start = shard.byte_offset();
        let end = start
            .checked_add(shard.shard_bytes().len())
            .ok_or_else(|| "aggregate reassembly shard range overflow".to_string())?;
        for existing in self.shards.iter().flatten() {
            if ranges_overlap(start, end, existing.start, existing.end) {
                return Err("aggregate reassembly overlapping shard".to_string());
            }
        }
        self.shards[index] = Some(AcceptedShard {
            start,
            end,
            bytes: shard.shard_bytes().to_vec(),
        });
        self.received_shards = self.received_shards.saturating_add(1);
        Ok(())
    }

    fn is_complete(&self) -> bool {
        self.received_shards == self.shard_count
    }

    fn reassemble_sealed_bytes(&self) -> Result<Vec<u8>, String> {
        let mut ranges = self.shards.iter().flatten().collect::<Vec<_>>();
        ranges.sort_by_key(|shard| shard.start);
        let mut cursor = 0usize;
        let mut object = vec![0_u8; self.object_len];
        for shard in ranges {
            if shard.start != cursor {
                return Err("aggregate reassembly coverage gap".to_string());
            }
            object[shard.start..shard.end].copy_from_slice(&shard.bytes);
            cursor = shard.end;
        }
        if cursor != self.object_len {
            return Err("aggregate reassembly coverage incomplete".to_string());
        }
        Ok(object)
    }
}

#[derive(Clone)]
struct AcceptedShard {
    start: usize,
    end: usize,
    bytes: Vec<u8>,
}

fn validate_limits(
    limits: &AggregateTransitReassemblyLimits,
    shard: &AggregateTransitShardFrame,
) -> Result<(), String> {
    if shard.object_len() > limits.max_object_len
        || usize::from(shard.shard_count()) > limits.max_shard_count
    {
        return Err("aggregate reassembly limits exceeded".to_string());
    }
    Ok(())
}

fn ranges_overlap(
    left_start: usize,
    left_end: usize,
    right_start: usize,
    right_end: usize,
) -> bool {
    left_start < right_end && right_start < left_end
}

#[cfg(test)]
mod tests {
    use super::{
        AggregateTransitObjectReassembler, AggregateTransitReassemblyLimits,
        AggregateTransitReassemblyStatus,
    };
    use crate::peer_egress::aggregate_wire::{
        AggregateObjectId, AggregateTransitShardFrame, MAX_AGGREGATE_OBJECT_LEN,
    };
    use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};
    use chimera_session::{Frame, FrameKind};

    fn binding(route: u64, lane: u16) -> TransitPathBinding {
        TransitPathBinding::new(
            TransitRouteId::new(route).unwrap_or_else(|error| unreachable!("{error}")),
            TransitLaneId::new(lane).unwrap_or_else(|error| unreachable!("{error}")),
        )
    }

    fn object_id(value: u64) -> AggregateObjectId {
        AggregateObjectId::new(value).unwrap_or_else(|error| unreachable!("{error}"))
    }

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

    fn shard(
        aggregate_id: AggregateObjectId,
        route_binding: TransitPathBinding,
        object: &[u8],
        shard_count: u16,
        shard_index: u16,
        start: usize,
        end: usize,
    ) -> Result<AggregateTransitShardFrame, String> {
        AggregateTransitShardFrame::new(
            route_binding,
            aggregate_id,
            object.len(),
            shard_count,
            shard_index,
            start,
            object[start..end].to_vec(),
        )
    }

    #[test]
    fn reassembly_accepts_out_of_order_shards_and_preserves_sealed_bytes() -> Result<(), String> {
        let sealed = encoded_frame(b"SECRET_AGGREGATE_PAYLOAD");
        let binding = binding(7, 1);
        let object_id = object_id(42);
        let first = shard(object_id, binding, &sealed, 3, 0, 0, 8)?;
        let second = shard(object_id, binding, &sealed, 3, 1, 8, 16)?;
        let third = shard(object_id, binding, &sealed, 3, 2, 16, sealed.len())?;
        let mut reassembler = AggregateTransitObjectReassembler::default();

        assert!(matches!(
            reassembler.accept(second)?,
            AggregateTransitReassemblyStatus::Pending
        ));
        assert!(matches!(
            reassembler.accept(first)?,
            AggregateTransitReassemblyStatus::Pending
        ));
        let complete = reassembler.accept(third)?;

        match complete {
            AggregateTransitReassemblyStatus::Complete(frame) => {
                assert_eq!(frame.sealed_bytes(), sealed.as_slice());
            }
            AggregateTransitReassemblyStatus::Pending => {
                return Err("aggregate reassembly must complete".to_string());
            }
        }
        let debug = format!("{reassembler:?}");
        assert!(!debug.contains("SECRET_AGGREGATE_PAYLOAD"));
        assert!(debug.contains("<redacted>"));
        Ok(())
    }

    #[test]
    fn reassembly_keeps_missing_shard_pending() -> Result<(), String> {
        let sealed = encoded_frame(b"sealed pending");
        let binding = binding(7, 1);
        let mut reassembler = AggregateTransitObjectReassembler::default();
        let first = shard(object_id(42), binding, &sealed, 2, 0, 0, 8)?;

        assert!(matches!(
            reassembler.accept(first)?,
            AggregateTransitReassemblyStatus::Pending
        ));
        Ok(())
    }

    #[test]
    fn reassembly_rejects_duplicate_and_overlapping_shards() -> Result<(), String> {
        let sealed = encoded_frame(b"sealed duplicate");
        let binding = binding(7, 1);
        let object_id = object_id(42);
        let first = shard(object_id, binding, &sealed, 2, 0, 0, 8)?;
        let duplicate = shard(object_id, binding, &sealed, 2, 0, 0, 8)?;
        let overlap = shard(object_id, binding, &sealed, 2, 1, 4, sealed.len())?;

        let mut reassembler = AggregateTransitObjectReassembler::default();
        assert!(matches!(
            reassembler.accept(first.clone())?,
            AggregateTransitReassemblyStatus::Pending
        ));
        assert!(reassembler.accept(duplicate).is_err());

        let mut reassembler = AggregateTransitObjectReassembler::default();
        assert!(matches!(
            reassembler.accept(first)?,
            AggregateTransitReassemblyStatus::Pending
        ));
        assert!(reassembler.accept(overlap).is_err());
        Ok(())
    }

    #[test]
    fn reassembly_rejects_mismatched_identity() -> Result<(), String> {
        let sealed = encoded_frame(b"sealed mismatch");
        let route_binding = binding(7, 1);
        let mut reassembler = AggregateTransitObjectReassembler::default();
        let first = shard(object_id(42), route_binding, &sealed, 2, 0, 0, 8)?;
        let wrong_id = shard(object_id(43), route_binding, &sealed, 2, 1, 8, sealed.len())?;
        let wrong_binding = shard(object_id(42), binding(8, 1), &sealed, 2, 1, 8, sealed.len())?;

        assert!(matches!(
            reassembler.accept(first.clone())?,
            AggregateTransitReassemblyStatus::Pending
        ));
        assert!(reassembler.accept(wrong_id).is_err());

        let mut reassembler = AggregateTransitObjectReassembler::default();
        assert!(matches!(
            reassembler.accept(first)?,
            AggregateTransitReassemblyStatus::Pending
        ));
        assert!(reassembler.accept(wrong_binding).is_err());
        Ok(())
    }

    #[test]
    fn reassembly_rejects_oversized_limits_and_invalid_sealed_frame() -> Result<(), String> {
        assert!(
            AggregateTransitObjectReassembler::new(AggregateTransitReassemblyLimits {
                max_object_len: MAX_AGGREGATE_OBJECT_LEN + 1,
                max_shard_count: 2,
            })
            .is_err()
        );

        let invalid = b"not-a-valid-sealed-frame".to_vec();
        let binding = binding(7, 1);
        let object_id = object_id(42);
        let only =
            AggregateTransitShardFrame::new(binding, object_id, invalid.len(), 1, 0, 0, invalid)?;
        let mut reassembler = AggregateTransitObjectReassembler::default();
        assert!(reassembler.accept(only).is_err());
        Ok(())
    }
}
