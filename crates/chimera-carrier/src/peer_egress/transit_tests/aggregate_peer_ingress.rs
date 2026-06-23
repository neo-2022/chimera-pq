use chimera_mesh::MeshMultipathFlowKey;
use chimera_session::FrameKind;

use super::helpers::{binding, encoded_frame, test_peer_pair};
use crate::peer_egress::aggregate_ingress::{
    AggregateTransitIngressLimits, new_shared_aggregate_transit_ingress_registry,
};
use crate::peer_egress::aggregate_wire::{AggregateObjectId, AggregateTransitShardFrame};
use crate::peer_egress::lane_binding::{TransitLaneDocument, transit_lane_document_from_mesh_plan};
use crate::peer_egress::live_lane_selection::select_carrier_lane_from_mesh_plan;
use crate::peer_egress::modes::{
    handle_reverse_peer_with_lane_document,
    handle_reverse_peer_with_lane_document_and_aggregate_ingress,
};
use crate::peer_egress::pool::new_shared_pool;
use crate::peer_egress::transit::{BoundPeerTransitPolicy, PeerTransitPolicy};
use crate::peer_egress::transit_binding::TransitPathBinding;
use crate::peer_egress::wire::write_aggregate_sealed_transit_message;

fn planned_lane_document(route_binding_id: u64) -> Result<TransitLaneDocument, String> {
    let mut runtime = chimera_mesh::MeshRuntime::bootstrap("cef-public", "seed-a")?;
    runtime.merge_discovery(
        "seed-b",
        &[
            chimera_mesh::MeshDiscoveryRecord {
                node_id: "node-a".to_string(),
                endpoint: "198.51.100.31:443".to_string(),
                region: "eu".to_string(),
                load_score: 20,
                reliability_score: 90,
            },
            chimera_mesh::MeshDiscoveryRecord {
                node_id: "node-b".to_string(),
                endpoint: "198.51.100.32:443".to_string(),
                region: "eu".to_string(),
                load_score: 22,
                reliability_score: 91,
            },
        ],
    )?;
    let plan = runtime.plan_path_from_dps_payload(
        &chimera_mesh::MeshJoinRequest {
            namespace: "cef-public".to_string(),
            node_name: "node-client".to_string(),
            invite_token: None,
        },
        &format!(
            "mesh_allowed_regions=eu;mesh_max_peers=2;mesh_max_selected_per_region=2;mesh_multipath_mode=flow_shard;mesh_route_binding_id={route_binding_id}"
        ),
    )?;
    transit_lane_document_from_mesh_plan(&plan)
}

fn aggregate_id(value: u64) -> Result<AggregateObjectId, String> {
    AggregateObjectId::new(value)
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

fn selected_binding(
    document: &TransitLaneDocument,
    sealed: &[u8],
) -> Result<TransitPathBinding, String> {
    let plan = document
        .mesh_path_plan()?
        .ok_or_else(|| "planned lane document missing snapshot".to_string())?;
    let flow_key = MeshMultipathFlowKey::from_opaque_flow_bytes(sealed)?;
    select_carrier_lane_from_mesh_plan(&plan, flow_key)
        .selected_binding
        .ok_or_else(|| "planned binding missing".to_string())
}

#[test]
fn planned_aggregate_peer_ingress_reassembles_and_forwards_selected_lane() -> Result<(), String> {
    let document = planned_lane_document(7307)?;
    let sealed = encoded_frame(
        FrameKind::Data,
        4301,
        b"AGGREGATE_PEER_INGRESS_SECRET_PAYLOAD",
    );
    let aggregate_id = aggregate_id(7701)?;
    let first = shard(aggregate_id, binding(5201, 1), &sealed, 2, 0, 0, 8)?;
    let second = shard(
        aggregate_id,
        binding(5201, 2),
        &sealed,
        2,
        1,
        8,
        sealed.len(),
    )?;
    let (mut first_source_writer, first_source_reader) = test_peer_pair()?;
    let (mut second_source_writer, second_source_reader) = test_peer_pair()?;
    let (selected_peer_writer, mut selected_peer_reader) = test_peer_pair()?;
    let (wrong_peer_writer, mut wrong_peer_reader) = test_peer_pair()?;
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(selected_binding(&document, &sealed)?, selected_peer_writer)?;
    dispatcher.register(binding(7308, 99), wrong_peer_writer)?;
    selected_peer_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set selected timeout failed: {error}"))?;
    wrong_peer_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set wrong timeout failed: {error}"))?;

    let registry =
        new_shared_aggregate_transit_ingress_registry(AggregateTransitIngressLimits::default())?;
    write_aggregate_sealed_transit_message(&mut first_source_writer, &first)?;
    handle_reverse_peer_with_lane_document_and_aggregate_ingress(
        first_source_reader,
        PeerTransitPolicy::DenyPoolNextHop,
        BoundPeerTransitPolicy::DenyBoundNextHop,
        new_shared_pool(),
        Some(dispatcher.clone()),
        Some(&document),
        Some(registry.clone()),
    )?;
    assert!(selected_peer_reader.read_secure_payload().is_err());
    assert!(wrong_peer_reader.read_secure_payload().is_err());

    write_aggregate_sealed_transit_message(&mut second_source_writer, &second)?;
    handle_reverse_peer_with_lane_document_and_aggregate_ingress(
        second_source_reader,
        PeerTransitPolicy::DenyPoolNextHop,
        BoundPeerTransitPolicy::DenyBoundNextHop,
        new_shared_pool(),
        Some(dispatcher),
        Some(&document),
        Some(registry.clone()),
    )?;

    assert_eq!(selected_peer_reader.read_secure_payload()?, sealed);
    assert!(wrong_peer_reader.read_secure_payload().is_err());
    let debug = format!("{registry:?}");
    assert!(!debug.contains("AGGREGATE_PEER_INGRESS_SECRET_PAYLOAD"));
    assert!(debug.contains("<opaque>"));
    Ok(())
}

#[test]
fn aggregate_peer_ingress_fails_closed_without_registry() -> Result<(), String> {
    let document = planned_lane_document(7311)?;
    let sealed = encoded_frame(FrameKind::Data, 4401, b"NO_REGISTRY_SECRET_PAYLOAD");
    let first = shard(
        aggregate_id(7702)?,
        binding(5202, 1),
        &sealed,
        1,
        0,
        0,
        sealed.len(),
    )?;
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (selected_peer_writer, mut selected_peer_reader) = test_peer_pair()?;
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(selected_binding(&document, &sealed)?, selected_peer_writer)?;
    selected_peer_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set selected timeout failed: {error}"))?;

    write_aggregate_sealed_transit_message(&mut source_writer, &first)?;
    let error = match handle_reverse_peer_with_lane_document_and_aggregate_ingress(
        source_reader,
        PeerTransitPolicy::DenyPoolNextHop,
        BoundPeerTransitPolicy::DenyBoundNextHop,
        new_shared_pool(),
        Some(dispatcher),
        Some(&document),
        None,
    ) {
        Ok(()) => return Err("aggregate peer ingress without registry must fail".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("registry unavailable"));
    assert!(!error.contains("NO_REGISTRY_SECRET_PAYLOAD"));
    assert!(selected_peer_reader.read_secure_payload().is_err());
    Ok(())
}

#[test]
fn aggregate_peer_ingress_duplicate_shard_does_not_forward_or_leak() -> Result<(), String> {
    let document = planned_lane_document(7313)?;
    let sealed = encoded_frame(FrameKind::Data, 4501, b"DUPLICATE_AGGREGATE_SECRET_PAYLOAD");
    let aggregate_id = aggregate_id(7703)?;
    let first = shard(aggregate_id, binding(5203, 1), &sealed, 2, 0, 0, 8)?;
    let duplicate = shard(
        aggregate_id,
        binding(5203, 2),
        &sealed,
        2,
        0,
        8,
        sealed.len(),
    )?;
    let (mut first_source_writer, first_source_reader) = test_peer_pair()?;
    let (mut second_source_writer, second_source_reader) = test_peer_pair()?;
    let (selected_peer_writer, mut selected_peer_reader) = test_peer_pair()?;
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(selected_binding(&document, &sealed)?, selected_peer_writer)?;
    selected_peer_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set selected timeout failed: {error}"))?;
    let registry =
        new_shared_aggregate_transit_ingress_registry(AggregateTransitIngressLimits::default())?;

    write_aggregate_sealed_transit_message(&mut first_source_writer, &first)?;
    handle_reverse_peer_with_lane_document_and_aggregate_ingress(
        first_source_reader,
        PeerTransitPolicy::DenyPoolNextHop,
        BoundPeerTransitPolicy::DenyBoundNextHop,
        new_shared_pool(),
        Some(dispatcher.clone()),
        Some(&document),
        Some(registry.clone()),
    )?;

    write_aggregate_sealed_transit_message(&mut second_source_writer, &duplicate)?;
    let error = match handle_reverse_peer_with_lane_document_and_aggregate_ingress(
        second_source_reader,
        PeerTransitPolicy::DenyPoolNextHop,
        BoundPeerTransitPolicy::DenyBoundNextHop,
        new_shared_pool(),
        Some(dispatcher),
        Some(&document),
        Some(registry),
    ) {
        Ok(()) => return Err("duplicate aggregate shard must fail closed".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("duplicate"));
    assert!(!error.contains("DUPLICATE_AGGREGATE_SECRET_PAYLOAD"));
    assert!(selected_peer_reader.read_secure_payload().is_err());
    Ok(())
}

#[test]
fn aggregate_peer_ingress_without_runtime_wiring_still_fails_closed() -> Result<(), String> {
    let document = planned_lane_document(7315)?;
    let sealed = encoded_frame(FrameKind::Data, 4601, b"LEGACY_AGGREGATE_SECRET_PAYLOAD");
    let shard = shard(
        aggregate_id(7704)?,
        binding(5204, 1),
        &sealed,
        1,
        0,
        0,
        sealed.len(),
    )?;
    let (mut source_writer, source_reader) = test_peer_pair()?;
    write_aggregate_sealed_transit_message(&mut source_writer, &shard)?;

    let error = match handle_reverse_peer_with_lane_document(
        source_reader,
        PeerTransitPolicy::DenyPoolNextHop,
        BoundPeerTransitPolicy::DenyBoundNextHop,
        new_shared_pool(),
        None,
        Some(&document),
    ) {
        Ok(()) => return Err("legacy aggregate peer ingress must fail closed".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("registry unavailable"));
    assert!(!error.contains("LEGACY_AGGREGATE_SECRET_PAYLOAD"));
    Ok(())
}
