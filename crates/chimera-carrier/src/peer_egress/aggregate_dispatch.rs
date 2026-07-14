use core::fmt;
use std::thread;

use chimera_mesh::{
    MeshMultipathAggregateAction, MeshMultipathSchedule, MeshPathPlan,
    plan_multipath_aggregate_object,
};

use crate::peer_egress::aggregate_wire::{AggregateObjectId, AggregateTransitShardFrame};
use crate::peer_egress::protocol::SecurePeerStream;
use crate::peer_egress::transit::TransitRelayFrame;
use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};
use crate::peer_egress::transit_dispatch::SharedTransitNextHopDispatcher;
use crate::peer_egress::wire::write_aggregate_sealed_transit_message;

pub(crate) struct AggregateTransitShardSet {
    active_binding_count: usize,
    rebuild_recommended: bool,
    rebuild_reason: String,
    explain: Vec<String>,
    shards: Vec<AggregateTransitShardFrame>,
}

impl fmt::Debug for AggregateTransitShardSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AggregateTransitShardSet")
            .field("active_binding_count", &self.active_binding_count)
            .field("rebuild_recommended", &self.rebuild_recommended)
            .field("rebuild_reason", &self.rebuild_reason)
            .field("shard_count", &self.shards.len())
            .field("shards", &"<sealed>")
            .finish()
    }
}

impl AggregateTransitShardSet {
    pub(crate) fn shards(&self) -> &[AggregateTransitShardFrame] {
        &self.shards
    }

    pub(crate) fn bindings(&self) -> Vec<TransitPathBinding> {
        self.shards.iter().map(|shard| shard.binding()).collect()
    }

    pub(crate) fn explain(&self) -> &[String] {
        &self.explain
    }

    pub(crate) fn into_shards(self) -> Vec<AggregateTransitShardFrame> {
        self.shards
    }
}

pub(crate) struct ClaimedAggregateTransitShard {
    pub(crate) frame: AggregateTransitShardFrame,
    pub(crate) peer: SecurePeerStream,
}

impl fmt::Debug for ClaimedAggregateTransitShard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClaimedAggregateTransitShard")
            .field("frame", &self.frame)
            .field("peer", &"<redacted>")
            .finish()
    }
}

pub(crate) fn build_aggregate_transit_shards(
    plan: &MeshPathPlan,
    frame: &TransitRelayFrame,
    aggregate_id: AggregateObjectId,
) -> Result<AggregateTransitShardSet, String> {
    build_aggregate_transit_shards_from_schedule(&plan.multipath_schedule, frame, aggregate_id)
}

pub(crate) fn build_aggregate_transit_shards_from_schedule(
    schedule: &MeshMultipathSchedule,
    frame: &TransitRelayFrame,
    aggregate_id: AggregateObjectId,
) -> Result<AggregateTransitShardSet, String> {
    let sealed = frame.sealed_bytes();
    let object_bytes = u64::try_from(sealed.len())
        .map_err(|_| "aggregate transit object length overflow".to_string())?;
    let aggregate = plan_multipath_aggregate_object(schedule, object_bytes);
    if aggregate.action != MeshMultipathAggregateAction::Assigned {
        return Err(format!(
            "aggregate transit planning failed: {}",
            aggregate.reason
        ));
    }
    if aggregate.transit_payload_policy != "sealed_opaque_only" {
        return Err("aggregate transit payload policy invalid".to_string());
    }

    let object_len = usize::try_from(aggregate.object_bytes)
        .map_err(|_| "aggregate transit object length overflow".to_string())?;
    if object_len != sealed.len() {
        return Err("aggregate transit object length mismatch".to_string());
    }
    let shard_count = u16::try_from(aggregate.shards.len())
        .map_err(|_| "aggregate transit shard count overflow".to_string())?;
    if shard_count == 0 {
        return Err("aggregate transit shard set empty".to_string());
    }

    let mut shards = Vec::with_capacity(aggregate.shards.len());
    for planned in &aggregate.shards {
        let binding = TransitPathBinding::new(
            TransitRouteId::new(planned.route_binding_id.get())?,
            TransitLaneId::from_zero_based_lane_index(planned.lane_id)?,
        );
        let byte_offset = usize::try_from(planned.byte_offset)
            .map_err(|_| "aggregate transit shard offset overflow".to_string())?;
        let byte_len = usize::try_from(planned.byte_len)
            .map_err(|_| "aggregate transit shard length overflow".to_string())?;
        let end = byte_offset
            .checked_add(byte_len)
            .ok_or_else(|| "aggregate transit shard range overflow".to_string())?;
        let shard_bytes = sealed
            .get(byte_offset..end)
            .ok_or_else(|| "aggregate transit shard range outside object".to_string())?
            .to_vec();
        let shard_index = u16::try_from(shards.len())
            .map_err(|_| "aggregate transit shard index overflow".to_string())?;
        shards.push(AggregateTransitShardFrame::new(
            binding,
            aggregate_id,
            object_len,
            shard_count,
            shard_index,
            byte_offset,
            shard_bytes,
        )?);
    }

    Ok(AggregateTransitShardSet {
        active_binding_count: aggregate.active_binding_count,
        rebuild_recommended: aggregate.rebuild_recommended,
        rebuild_reason: aggregate.rebuild_reason,
        explain: aggregate.explain,
        shards,
    })
}

pub(crate) fn claim_aggregate_transit_shards(
    plan: &MeshPathPlan,
    frame: &TransitRelayFrame,
    aggregate_id: AggregateObjectId,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
) -> Result<Vec<ClaimedAggregateTransitShard>, String> {
    claim_aggregate_transit_shards_from_schedule(
        &plan.multipath_schedule,
        frame,
        aggregate_id,
        dispatcher,
    )
}

pub(crate) fn claim_aggregate_transit_shards_from_schedule(
    schedule: &MeshMultipathSchedule,
    frame: &TransitRelayFrame,
    aggregate_id: AggregateObjectId,
    dispatcher: Option<SharedTransitNextHopDispatcher>,
) -> Result<Vec<ClaimedAggregateTransitShard>, String> {
    let shard_set = build_aggregate_transit_shards_from_schedule(schedule, frame, aggregate_id)?;
    let bindings = shard_set.bindings();
    let dispatcher = dispatcher
        .ok_or_else(|| "aggregate transit path binding dispatcher unavailable".to_string())?;
    let claimed = dispatcher
        .pop_many_for(&bindings)
        .map_err(|error| format!("aggregate transit lane claim failed: {error}"))?;
    let mut claimed_by_binding = claimed
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut out = Vec::with_capacity(shard_set.shards.len());
    for frame in shard_set.into_shards() {
        let peer = claimed_by_binding
            .remove(&frame.binding())
            .ok_or_else(|| "aggregate transit claimed lane missing".to_string())?;
        out.push(ClaimedAggregateTransitShard { frame, peer });
    }
    Ok(out)
}

pub(crate) fn forward_claimed_aggregate_transit_shards(
    claimed: Vec<ClaimedAggregateTransitShard>,
) -> Result<(), String> {
    if claimed.len() <= 1 {
        for claimed_shard in claimed {
            write_claimed_aggregate_transit_shard(claimed_shard)?;
        }
        return Ok(());
    }

    let mut workers = Vec::with_capacity(claimed.len());
    for claimed_shard in claimed {
        workers.push(thread::spawn(move || {
            write_claimed_aggregate_transit_shard(claimed_shard)
        }));
    }

    let mut first_error: Option<String> = None;
    for worker in workers {
        match worker.join() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
            Err(_) => {
                if first_error.is_none() {
                    first_error = Some("aggregate transit shard worker panicked".to_string());
                }
            }
        }
    }
    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn write_claimed_aggregate_transit_shard(
    claimed_shard: ClaimedAggregateTransitShard,
) -> Result<(), String> {
    let ClaimedAggregateTransitShard { frame, mut peer } = claimed_shard;
    write_aggregate_sealed_transit_message(&mut peer, &frame)
}

#[cfg(test)]
#[path = "aggregate_dispatch_parallel_tests.rs"]
mod aggregate_dispatch_parallel_tests;

#[cfg(test)]
mod tests {
    use super::{
        build_aggregate_transit_shards, build_aggregate_transit_shards_from_schedule,
        claim_aggregate_transit_shards, forward_claimed_aggregate_transit_shards,
    };
    use crate::peer_egress::aggregate_reassembly::{
        AggregateTransitObjectReassembler, AggregateTransitReassemblyStatus,
    };
    use crate::peer_egress::aggregate_wire::AggregateObjectId;
    use crate::peer_egress::options::AeadSuite;
    use crate::peer_egress::protocol::SecurePeerStream;
    use crate::peer_egress::transit::validate_transit_relay_frame;
    use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};
    use crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher;
    use crate::peer_egress::wire::{PeerMessage, read_peer_message};
    use chimera_mesh::{MeshDiscoveryRecord, MeshJoinRequest, MeshRuntime};
    use chimera_session::{Frame, FrameKind};

    fn record(
        node_id: &str,
        endpoint: &str,
        region: &str,
        load: u8,
        reliability: u8,
    ) -> MeshDiscoveryRecord {
        MeshDiscoveryRecord {
            node_id: node_id.to_string(),
            endpoint: endpoint.to_string(),
            region: region.to_string(),
            load_score: load,
            reliability_score: reliability,
        }
    }

    fn plan() -> Result<chimera_mesh::MeshPathPlan, String> {
        let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")?;
        runtime.merge_discovery(
            "seed-b",
            &[
                record("node-a", "198.51.100.31:443", "eu", 20, 90),
                record("node-b", "198.51.100.32:443", "eu", 22, 91),
                record("node-c", "198.51.100.33:443", "eu", 24, 92),
            ],
        )?;
        runtime.plan_path_from_dps_payload(
            &MeshJoinRequest {
                namespace: "cef-public".to_string(),
                node_name: "node-client".to_string(),
                invite_token: None,
            },
            "mesh_allowed_regions=eu;mesh_max_peers=3;mesh_max_selected_per_region=3;mesh_multipath_mode=aggregate_buffered;mesh_route_binding_id=7301",
        )
    }

    fn core_plan() -> Result<chimera_mesh::MeshPathPlanCore, String> {
        let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")?;
        runtime.merge_discovery(
            "seed-b",
            &[
                record("node-a", "198.51.100.31:443", "eu", 20, 90),
                record("node-b", "198.51.100.32:443", "eu", 22, 91),
                record("node-c", "198.51.100.33:443", "eu", 24, 92),
            ],
        )?;
        runtime.plan_path_core_from_dps_payload(
            &MeshJoinRequest {
                namespace: "cef-public".to_string(),
                node_name: "node-client".to_string(),
                invite_token: None,
            },
            "mesh_allowed_regions=eu;mesh_max_peers=3;mesh_max_selected_per_region=3;mesh_multipath_mode=aggregate_buffered;mesh_route_binding_id=7301",
        )
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

    fn transit_frame(
        payload: &[u8],
    ) -> Result<crate::peer_egress::transit::TransitRelayFrame, String> {
        let encoded = encoded_frame(payload)?;
        validate_transit_relay_frame(&encoded)
    }

    fn aggregate_id() -> Result<AggregateObjectId, String> {
        AggregateObjectId::new(42)
    }

    fn binding(route: u64, lane: u16) -> Result<TransitPathBinding, String> {
        Ok(TransitPathBinding::new(
            TransitRouteId::new(route)?,
            TransitLaneId::new(lane)?,
        ))
    }

    fn test_peer_stream() -> Result<SecurePeerStream, String> {
        let transcript = chimera_crypto::TranscriptHash::from_messages(&[b"aggregate-dispatch"]);
        let secrets = chimera_crypto::derive_traffic_secrets(
            chimera_crypto::SuiteId(AeadSuite::Chacha20Poly1305.suite_id()),
            &transcript,
            &[31_u8; 32],
        )
        .map_err(|error| format!("test secrets derive failed: {error}"))?;
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|error| format!("bind test listener failed: {error}"))?;
        let addr = listener
            .local_addr()
            .map_err(|error| format!("read listener addr failed: {error}"))?;
        let client = std::net::TcpStream::connect(addr)
            .map_err(|error| format!("connect test client failed: {error}"))?;
        let (server, _) = listener
            .accept()
            .map_err(|error| format!("accept test peer failed: {error}"))?;
        drop(server);
        Ok(SecurePeerStream::new(
            client,
            secrets.initiator_to_responder().clone(),
            secrets.responder_to_initiator().clone(),
            AeadSuite::Chacha20Poly1305,
        ))
    }

    fn test_peer_pair() -> Result<(SecurePeerStream, SecurePeerStream), String> {
        let transcript =
            chimera_crypto::TranscriptHash::from_messages(&[b"aggregate-dispatch-pair"]);
        let secrets = chimera_crypto::derive_traffic_secrets(
            chimera_crypto::SuiteId(AeadSuite::Chacha20Poly1305.suite_id()),
            &transcript,
            &[37_u8; 32],
        )
        .map_err(|error| format!("test secrets derive failed: {error}"))?;
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|error| format!("bind test listener failed: {error}"))?;
        let addr = listener
            .local_addr()
            .map_err(|error| format!("read listener addr failed: {error}"))?;
        let client = std::net::TcpStream::connect(addr)
            .map_err(|error| format!("connect test client failed: {error}"))?;
        let (server, _) = listener
            .accept()
            .map_err(|error| format!("accept test peer failed: {error}"))?;
        Ok((
            SecurePeerStream::new(
                client,
                secrets.initiator_to_responder().clone(),
                secrets.responder_to_initiator().clone(),
                AeadSuite::Chacha20Poly1305,
            ),
            SecurePeerStream::new(
                server,
                secrets.responder_to_initiator().clone(),
                secrets.initiator_to_responder().clone(),
                AeadSuite::Chacha20Poly1305,
            ),
        ))
    }

    #[test]
    fn aggregate_dispatch_builds_shards_and_reassembles_sealed_bytes() -> Result<(), String> {
        let plan = plan()?;
        let frame = transit_frame(b"SECRET_AGGREGATE_DISPATCH_PAYLOAD")?;
        let shard_set = build_aggregate_transit_shards(&plan, &frame, aggregate_id()?)?;

        assert_eq!(shard_set.shards().len(), 3);
        assert!(
            shard_set
                .explain()
                .iter()
                .any(|line| line == "multipath_aggregate_privacy=sealed_opaque_only")
        );

        let mut reassembler = AggregateTransitObjectReassembler::default();
        let mut complete = None;
        for shard in shard_set.shards().iter().rev().cloned() {
            match reassembler.accept(shard)? {
                AggregateTransitReassemblyStatus::Pending => {}
                AggregateTransitReassemblyStatus::Complete(frame) => complete = Some(frame),
            }
        }
        let complete =
            complete.ok_or_else(|| "aggregate reassembly did not complete".to_string())?;
        assert_eq!(complete.sealed_bytes(), frame.sealed_bytes());

        let debug = format!("{shard_set:?}");
        assert!(!debug.contains("SECRET_AGGREGATE_DISPATCH_PAYLOAD"));
        assert!(!debug.contains("198.51.100.31"));
        assert!(debug.contains("<sealed>"));
        Ok(())
    }

    #[test]
    fn aggregate_dispatch_builds_shards_from_core_schedule() -> Result<(), String> {
        let core = core_plan()?;
        let frame = transit_frame(b"SECRET_AGGREGATE_DISPATCH_CORE_PAYLOAD")?;
        let shard_set = build_aggregate_transit_shards_from_schedule(
            &core.multipath_schedule,
            &frame,
            aggregate_id()?,
        )?;

        assert_eq!(shard_set.shards().len(), 3);
        assert!(
            shard_set
                .explain()
                .iter()
                .any(|line| line == "multipath_aggregate_privacy=sealed_opaque_only")
        );

        let debug = format!("{shard_set:?}");
        assert!(!debug.contains("SECRET_AGGREGATE_DISPATCH_CORE_PAYLOAD"));
        assert!(!debug.contains("198.51.100.31"));
        assert!(debug.contains("<sealed>"));
        Ok(())
    }

    #[test]
    fn aggregate_dispatch_claims_live_lanes_all_or_nothing() -> Result<(), String> {
        let plan = plan()?;
        let frame = transit_frame(b"sealed aggregate claim")?;
        let dispatcher = new_shared_transit_dispatcher();
        let first = binding(7301, 1)?;
        let second = binding(7301, 2)?;
        let third = binding(7301, 3)?;
        dispatcher.register(first, test_peer_stream()?)?;
        dispatcher.register(second, test_peer_stream()?)?;

        let error = match claim_aggregate_transit_shards(
            &plan,
            &frame,
            aggregate_id()?,
            Some(dispatcher.clone()),
        ) {
            Ok(_) => return Err("missing aggregate lane must fail closed".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("lane claim failed"));
        assert!(dispatcher.contains_binding(first)?);
        assert!(dispatcher.contains_binding(second)?);
        assert!(!dispatcher.contains_binding(third)?);

        dispatcher.register(third, test_peer_stream()?)?;
        let claimed = claim_aggregate_transit_shards(
            &plan,
            &frame,
            aggregate_id()?,
            Some(dispatcher.clone()),
        )?;
        assert_eq!(claimed.len(), 3);
        assert!(
            claimed
                .iter()
                .all(|claimed| !claimed.frame.shard_bytes().is_empty())
        );
        assert!(!dispatcher.contains_binding(first)?);
        assert!(!dispatcher.contains_binding(second)?);
        assert!(!dispatcher.contains_binding(third)?);

        for claimed in claimed {
            drop(claimed.peer);
        }
        Ok(())
    }

    #[test]
    fn aggregate_dispatch_writes_claimed_shards_to_remote_peers() -> Result<(), String> {
        let plan = plan()?;
        let frame = transit_frame(b"SECRET_AGGREGATE_DISPATCH_LIVE_PAYLOAD")?;
        let dispatcher = new_shared_transit_dispatcher();
        let first = binding(7301, 1)?;
        let second = binding(7301, 2)?;
        let third = binding(7301, 3)?;
        let (claim_first, inspect_first) = test_peer_pair()?;
        let (claim_second, inspect_second) = test_peer_pair()?;
        let (claim_third, inspect_third) = test_peer_pair()?;
        dispatcher.register(first, claim_first)?;
        dispatcher.register(second, claim_second)?;
        dispatcher.register(third, claim_third)?;

        let claimed = claim_aggregate_transit_shards(
            &plan,
            &frame,
            aggregate_id()?,
            Some(dispatcher.clone()),
        )?;
        forward_claimed_aggregate_transit_shards(claimed)?;

        let mut inspectors = vec![inspect_first, inspect_second, inspect_third];
        for inspector in &mut inspectors {
            match read_peer_message(
                inspector,
                crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
            )? {
                PeerMessage::AggregateSealedTransit(parsed) => {
                    assert_eq!(parsed.aggregate_id(), aggregate_id()?);
                    assert!(!parsed.shard_bytes().is_empty());
                }
                other => return Err(format!("unexpected aggregate message: {other:?}")),
            }
        }
        Ok(())
    }

    #[test]
    fn aggregate_dispatch_fails_closed_without_aggregate_mode() -> Result<(), String> {
        let mut plan = plan()?;
        plan.multipath_schedule.mode = chimera_mesh::MeshMultipathMode::FlowShard;
        let frame = transit_frame(b"sealed aggregate wrong mode")?;
        let error = match build_aggregate_transit_shards(&plan, &frame, aggregate_id()?) {
            Ok(_) => return Err("non-aggregate plan must fail closed".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("aggregate_mode_required"));
        assert!(!error.contains("sealed aggregate wrong mode"));
        Ok(())
    }
}
