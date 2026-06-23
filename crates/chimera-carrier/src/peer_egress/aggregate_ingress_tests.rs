use super::{
    AggregateTransitIngressLimits, AggregateTransitIngressRegistry, AggregateTransitIngressStatus,
    new_shared_aggregate_transit_ingress_registry,
};
use crate::peer_egress::aggregate_reassembly::AggregateTransitReassemblyLimits;
use crate::peer_egress::aggregate_wire::{AggregateObjectId, AggregateTransitShardFrame};
use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};
use chimera_session::{Frame, FrameKind};

fn binding(route: u64, lane: u16) -> Result<TransitPathBinding, String> {
    Ok(TransitPathBinding::new(
        TransitRouteId::new(route)?,
        TransitLaneId::new(lane)?,
    ))
}

fn object_id(value: u64) -> Result<AggregateObjectId, String> {
    AggregateObjectId::new(value)
}

fn ingress_limits(
    max_active_objects: usize,
    max_completed_objects: usize,
) -> AggregateTransitIngressLimits {
    AggregateTransitIngressLimits {
        reassembly: AggregateTransitReassemblyLimits::default(),
        max_active_objects,
        max_completed_objects,
    }
}

fn encoded_frame(payload: &[u8]) -> Result<Vec<u8>, String> {
    Frame {
        kind: FrameKind::Data,
        packet_number: 77,
        payload: payload.to_vec(),
    }
    .encode()
    .map_err(|error| format!("test frame encode failed: {error}"))
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
    let shard_bytes = object
        .get(start..end)
        .ok_or_else(|| "test aggregate shard range invalid".to_string())?
        .to_vec();
    AggregateTransitShardFrame::new(
        route_binding,
        aggregate_id,
        object.len(),
        shard_count,
        shard_index,
        start,
        shard_bytes,
    )
}

#[test]
fn aggregate_ingress_reassembles_multilane_shards() -> Result<(), String> {
    let sealed = encoded_frame(b"SECRET_AGGREGATE_INGRESS_PAYLOAD")?;
    let aggregate_id = object_id(91)?;
    let first = shard(aggregate_id, binding(7, 1)?, &sealed, 2, 0, 0, 8)?;
    let second = shard(aggregate_id, binding(7, 2)?, &sealed, 2, 1, 8, sealed.len())?;
    let registry = AggregateTransitIngressRegistry::default();

    assert!(matches!(
        registry.accept_shard(second)?,
        AggregateTransitIngressStatus::Pending
    ));
    let complete = registry.accept_shard(first)?;

    match complete {
        AggregateTransitIngressStatus::Complete(frame) => {
            assert_eq!(frame.sealed_bytes(), sealed.as_slice());
        }
        AggregateTransitIngressStatus::Pending => {
            return Err("aggregate ingress should complete".to_string());
        }
    }
    let debug = format!("{registry:?}");
    assert!(!debug.contains("SECRET_AGGREGATE_INGRESS_PAYLOAD"));
    assert!(!debug.contains("route_id: 7"));
    assert!(debug.contains("<opaque>"));
    Ok(())
}

#[test]
fn aggregate_ingress_fails_closed_and_evicts_corrupt_partial_state() -> Result<(), String> {
    let sealed = encoded_frame(b"sealed aggregate corrupt")?;
    let aggregate_id = object_id(92)?;
    let first = shard(aggregate_id, binding(7, 1)?, &sealed, 2, 0, 0, 8)?;
    let duplicate = shard(aggregate_id, binding(7, 2)?, &sealed, 2, 0, 8, sealed.len())?;
    let second = shard(aggregate_id, binding(7, 2)?, &sealed, 2, 1, 8, sealed.len())?;
    let registry = AggregateTransitIngressRegistry::default();

    assert!(matches!(
        registry.accept_shard(first.clone())?,
        AggregateTransitIngressStatus::Pending
    ));
    let error = match registry.accept_shard(duplicate) {
        Ok(_) => return Err("duplicate shard index must fail closed".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("duplicate"));
    assert!(!error.contains("sealed aggregate corrupt"));

    assert!(matches!(
        registry.accept_shard(first)?,
        AggregateTransitIngressStatus::Pending
    ));
    assert!(matches!(
        registry.accept_shard(second)?,
        AggregateTransitIngressStatus::Complete(_)
    ));
    Ok(())
}

#[test]
fn aggregate_ingress_rejects_replay_after_complete() -> Result<(), String> {
    let sealed = encoded_frame(b"sealed aggregate replay")?;
    let aggregate_id = object_id(93)?;
    let first = shard(aggregate_id, binding(7, 1)?, &sealed, 2, 0, 0, 8)?;
    let second = shard(aggregate_id, binding(7, 2)?, &sealed, 2, 1, 8, sealed.len())?;
    let registry = AggregateTransitIngressRegistry::default();

    assert!(matches!(
        registry.accept_shard(first.clone())?,
        AggregateTransitIngressStatus::Pending
    ));
    assert!(matches!(
        registry.accept_shard(second)?,
        AggregateTransitIngressStatus::Complete(_)
    ));
    let error = match registry.accept_shard(first) {
        Ok(_) => return Err("completed aggregate replay must fail closed".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("already complete"));
    assert!(!error.contains("sealed aggregate replay"));
    Ok(())
}

#[test]
fn aggregate_ingress_registry_is_session_scoped_by_construction() -> Result<(), String> {
    let first = AggregateTransitIngressRegistry::new_session_scoped();
    let second = AggregateTransitIngressRegistry::new_session_scoped();
    let sealed = encoded_frame(b"session scoped aggregate")?;
    let aggregate_id = object_id(94)?;
    let shard_first = shard(aggregate_id, binding(7, 1)?, &sealed, 2, 0, 0, 8)?;
    let shard_second = shard(aggregate_id, binding(7, 2)?, &sealed, 2, 1, 8, sealed.len())?;

    assert!(matches!(
        first.accept_shard(shard_first)?,
        AggregateTransitIngressStatus::Pending
    ));
    assert!(matches!(
        second.accept_shard(shard_second)?,
        AggregateTransitIngressStatus::Pending
    ));
    Ok(())
}

#[test]
fn aggregate_ingress_rejects_new_object_when_active_limit_is_full() -> Result<(), String> {
    let registry = AggregateTransitIngressRegistry::new(ingress_limits(1, 8))?;
    let sealed_first = encoded_frame(b"ACTIVE_LIMIT_SECRET_ONE")?;
    let sealed_second = encoded_frame(b"ACTIVE_LIMIT_SECRET_TWO")?;
    let first = shard(object_id(101)?, binding(70, 1)?, &sealed_first, 2, 0, 0, 8)?;
    let second = shard(object_id(102)?, binding(71, 1)?, &sealed_second, 2, 0, 0, 8)?;

    assert!(matches!(
        registry.accept_shard(first)?,
        AggregateTransitIngressStatus::Pending
    ));
    let error = match registry.accept_shard(second) {
        Ok(_) => return Err("active aggregate object flood must fail closed".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("active object limit exceeded"));
    assert!(!error.contains("ACTIVE_LIMIT_SECRET_TWO"));
    Ok(())
}

#[test]
fn aggregate_ingress_completed_cache_trims_by_completion_order() -> Result<(), String> {
    let registry = AggregateTransitIngressRegistry::new(ingress_limits(2, 1))?;
    let sealed_first = encoded_frame(b"sealed aggregate completed first")?;
    let sealed_second = encoded_frame(b"sealed aggregate completed second")?;
    let first_id = object_id(300)?;
    let second_id = object_id(100)?;
    let first_a = shard(first_id, binding(90, 1)?, &sealed_first, 2, 0, 0, 8)?;
    let first_b = shard(
        first_id,
        binding(90, 2)?,
        &sealed_first,
        2,
        1,
        8,
        sealed_first.len(),
    )?;
    let second_a = shard(second_id, binding(7, 1)?, &sealed_second, 2, 0, 0, 8)?;
    let second_b = shard(
        second_id,
        binding(7, 2)?,
        &sealed_second,
        2,
        1,
        8,
        sealed_second.len(),
    )?;

    assert!(matches!(
        registry.accept_shard(first_a.clone())?,
        AggregateTransitIngressStatus::Pending
    ));
    assert!(matches!(
        registry.accept_shard(first_b)?,
        AggregateTransitIngressStatus::Complete(_)
    ));
    assert!(matches!(
        registry.accept_shard(second_a.clone())?,
        AggregateTransitIngressStatus::Pending
    ));
    assert!(matches!(
        registry.accept_shard(second_b)?,
        AggregateTransitIngressStatus::Complete(_)
    ));

    assert!(matches!(
        registry.accept_shard(first_a)?,
        AggregateTransitIngressStatus::Pending
    ));
    let error = match registry.accept_shard(second_a) {
        Ok(_) => return Err("newest completed aggregate must remain replay-protected".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("already complete"));
    Ok(())
}

#[test]
fn aggregate_ingress_rejects_malformed_final_sealed_frame_without_payload_leak()
-> Result<(), String> {
    let registry = AggregateTransitIngressRegistry::default();
    let aggregate_id = object_id(104)?;
    let malformed = b"MALFORMED_AGGREGATE_FINAL_SECRET".to_vec();
    let first = shard(aggregate_id, binding(8, 1)?, &malformed, 2, 0, 0, 8)?;
    let second = shard(
        aggregate_id,
        binding(8, 2)?,
        &malformed,
        2,
        1,
        8,
        malformed.len(),
    )?;

    assert!(matches!(
        registry.accept_shard(first)?,
        AggregateTransitIngressStatus::Pending
    ));
    let error = match registry.accept_shard(second) {
        Ok(_) => return Err("malformed aggregate final frame must fail closed".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("sealed frame invalid"));
    assert!(!error.contains("MALFORMED_AGGREGATE_FINAL_SECRET"));
    let debug = format!("{registry:?}");
    assert!(!debug.contains("MALFORMED_AGGREGATE_FINAL_SECRET"));
    Ok(())
}

#[test]
fn aggregate_ingress_validates_explicit_limits_and_shared_constructor() -> Result<(), String> {
    let invalid_active = AggregateTransitIngressRegistry::new(ingress_limits(0, 1));
    let error = match invalid_active {
        Ok(_) => return Err("zero active object limit must be rejected".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("active object limit invalid"));

    let invalid_reassembly = AggregateTransitIngressLimits {
        reassembly: AggregateTransitReassemblyLimits {
            max_object_len: 0,
            max_shard_count: 1,
        },
        max_active_objects: 1,
        max_completed_objects: 1,
    };
    let error = match AggregateTransitIngressRegistry::new(invalid_reassembly) {
        Ok(_) => return Err("invalid reassembly limits must be rejected".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("reassembly limits invalid"));

    let shared = new_shared_aggregate_transit_ingress_registry(ingress_limits(2, 2))?;
    let debug = format!("{shared:?}");
    assert!(debug.contains("AggregateTransitIngressRegistry"));
    assert!(debug.contains("<opaque>"));
    Ok(())
}
