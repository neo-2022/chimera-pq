use chimera_session::FrameKind;
use std::thread;

use super::super::{BoundPeerTransitPolicy, forward_bound_peer_sealed_transit_to_next_hop};
use super::helpers::{
    assert_bound_payload, binding, bound_payload, encoded_frame, read_first_bound_frame,
    test_peer_pair,
};
use crate::peer_egress::live_lane_selection::select_carrier_lane_from_mesh_plan;
use crate::peer_egress::modes::{handle_reverse_peer, handle_reverse_peer_with_lane_document};
use crate::peer_egress::pool::new_shared_pool;
use crate::peer_egress::transit::PeerTransitPolicy;

fn planned_lane_document(
    route_binding_id: u64,
) -> Result<crate::peer_egress::lane_binding::TransitLaneDocument, String> {
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
        &format!("mesh_allowed_regions=eu;mesh_max_peers=2;mesh_max_selected_per_region=2;mesh_multipath_mode=flow_shard;mesh_route_binding_id={route_binding_id}"),
    )?;
    crate::peer_egress::lane_binding::transit_lane_document_from_mesh_plan(&plan)
}

#[test]
fn bound_peer_transit_rejects_reverse_binding_change() -> Result<(), String> {
    let path_binding = binding(194, 1);
    let changed_binding = binding(194, 2);
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let (source_first_payload, source_first_sealed) = bound_payload(
        path_binding,
        FrameKind::Data,
        1901,
        b"source bound first payload",
    )?;
    let (source_fin_payload, _) = bound_payload(path_binding, FrameKind::Fin, 1904, b"")?;
    let (reverse_payload, reverse_sealed) = bound_payload(
        path_binding,
        FrameKind::Data,
        1902,
        b"reverse bound first payload",
    )?;
    let (changed_payload, _) = bound_payload(
        changed_binding,
        FrameKind::Data,
        1903,
        b"REVERSE_CHANGED_PAYLOAD_MARKER",
    )?;

    source_writer.write_secure_payload(&source_first_payload)?;
    let mut source_reader = source_reader;
    let first_bound = read_first_bound_frame(&mut source_reader)?;
    source_writer.write_secure_payload(&source_fin_payload)?;
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(path_binding, next_writer)?;
    next_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(500)))
        .map_err(|error| format!("set next timeout failed: {error}"))?;
    source_writer
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(500)))
        .map_err(|error| format!("set source timeout failed: {error}"))?;

    next_reader.write_secure_payload(&reverse_payload)?;
    next_reader.write_secure_payload(&changed_payload)?;

    let error = match forward_bound_peer_sealed_transit_to_next_hop(
        source_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        Some(dispatcher),
        first_bound,
    ) {
        Ok(()) => return Err("reverse bound transit binding change must fail".to_string()),
        Err(error) => error,
    };

    assert!(
        error.contains("binding changed"),
        "unexpected reverse-binding error: {error}"
    );
    assert!(!error.contains("REVERSE_CHANGED_PAYLOAD_MARKER"));
    if let Ok(payload) = next_reader.read_secure_payload() {
        assert_bound_payload(&payload, path_binding, &source_first_sealed)?;
    }
    if let Ok(payload) = source_writer.read_secure_payload() {
        assert_bound_payload(&payload, path_binding, &reverse_sealed)?;
    }
    let changed_result = source_writer.read_secure_payload();
    assert!(
        !format!("{changed_result:?}").contains("REVERSE_CHANGED_PAYLOAD_MARKER"),
        "changed reverse payload must not be exposed: {changed_result:?}"
    );
    Ok(())
}

#[test]
fn reverse_peer_bound_transit_uses_shared_dispatcher() -> Result<(), String> {
    let path_binding = binding(196, 3);
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(path_binding, next_writer)?;

    let (source_first_payload, source_first_sealed) = bound_payload(
        path_binding,
        FrameKind::Data,
        2101,
        b"reverse bound transit payload",
    )?;
    let (source_fin_payload, source_fin_sealed) =
        bound_payload(path_binding, FrameKind::Fin, 2102, b"")?;
    let (reverse_payload, reverse_sealed) = bound_payload(
        path_binding,
        FrameKind::Data,
        2103,
        b"reverse bound transit reply",
    )?;
    let (reverse_fin_payload, reverse_fin_sealed) =
        bound_payload(path_binding, FrameKind::Fin, 2104, b"")?;

    source_writer.write_secure_payload(&source_first_payload)?;
    source_writer.write_secure_payload(&source_fin_payload)?;
    next_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set next timeout failed: {error}"))?;
    source_writer
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set source timeout failed: {error}"))?;
    next_reader.write_secure_payload(&reverse_payload)?;
    next_reader.write_secure_payload(&reverse_fin_payload)?;

    let handler = thread::spawn(move || {
        handle_reverse_peer(
            source_reader,
            PeerTransitPolicy::AllowPoolNextHop,
            BoundPeerTransitPolicy::AllowBoundNextHop,
            new_shared_pool(),
            Some(dispatcher),
        )
    });
    handler
        .join()
        .map_err(|_| "reverse peer transit worker panicked".to_string())??;

    assert_bound_payload(
        &next_reader.read_secure_payload()?,
        path_binding,
        &source_first_sealed,
    )?;
    assert_bound_payload(
        &next_reader.read_secure_payload()?,
        path_binding,
        &source_fin_sealed,
    )?;
    assert_bound_payload(
        &source_writer.read_secure_payload()?,
        path_binding,
        &reverse_sealed,
    )?;
    assert_bound_payload(
        &source_writer.read_secure_payload()?,
        path_binding,
        &reverse_fin_sealed,
    )?;
    Ok(())
}

#[test]
fn reverse_peer_sealed_transit_uses_planned_lane_document_binding() -> Result<(), String> {
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (selected_peer_writer, mut selected_peer_reader) = test_peer_pair()?;
    let document = planned_lane_document(7103)?;
    let first_encoded = encoded_frame(FrameKind::Data, 3101, b"reverse planned opaque payload");
    let fin_encoded = encoded_frame(FrameKind::Fin, 3102, b"");
    let reverse_encoded = encoded_frame(FrameKind::Data, 3103, b"reverse planned reply");
    let reverse_fin_encoded = encoded_frame(FrameKind::Fin, 3104, b"");

    source_writer.write_secure_payload(&first_encoded)?;
    let plan = document
        .mesh_path_plan()?
        .ok_or_else(|| "planned lane document missing snapshot".to_string())?;
    let flow_key = chimera_mesh::MeshMultipathFlowKey::from_opaque_flow_bytes(&first_encoded)?;
    let selection = select_carrier_lane_from_mesh_plan(&plan, flow_key);
    let binding = selection
        .selected_binding
        .ok_or_else(|| "planned binding missing".to_string())?;
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(binding, selected_peer_writer)?;
    selected_peer_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set selected timeout failed: {error}"))?;
    source_writer
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set source timeout failed: {error}"))?;
    selected_peer_reader.write_secure_payload(&reverse_encoded)?;
    selected_peer_reader.write_secure_payload(&reverse_fin_encoded)?;
    source_writer.write_secure_payload(&fin_encoded)?;

    let handler = thread::spawn(move || {
        handle_reverse_peer_with_lane_document(
            source_reader,
            PeerTransitPolicy::DenyPoolNextHop,
            BoundPeerTransitPolicy::DenyBoundNextHop,
            new_shared_pool(),
            Some(dispatcher),
            Some(&document),
        )
    });
    handler
        .join()
        .map_err(|_| "planned reverse peer transit worker panicked".to_string())??;

    assert_eq!(selected_peer_reader.read_secure_payload()?, first_encoded);
    assert_eq!(selected_peer_reader.read_secure_payload()?, fin_encoded);
    assert_eq!(source_writer.read_secure_payload()?, reverse_encoded);
    assert_eq!(source_writer.read_secure_payload()?, reverse_fin_encoded);
    Ok(())
}

#[test]
fn reverse_peer_sealed_transit_fails_closed_without_planned_binding() -> Result<(), String> {
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let document = planned_lane_document(7104)?;
    let first_encoded = encoded_frame(FrameKind::Data, 3201, b"reverse planned opaque payload");
    source_writer.write_secure_payload(&first_encoded)?;
    let fin_encoded = encoded_frame(FrameKind::Fin, 3202, b"");
    source_writer.write_secure_payload(&fin_encoded)?;
    let handler = thread::spawn(move || {
        handle_reverse_peer_with_lane_document(
            source_reader,
            PeerTransitPolicy::DenyPoolNextHop,
            BoundPeerTransitPolicy::DenyBoundNextHop,
            new_shared_pool(),
            None,
            Some(&document),
        )
    });

    let error = match handler
        .join()
        .map_err(|_| "planned reverse peer transit worker panicked".to_string())?
    {
        Ok(()) => {
            return Err("planned reverse transit without dispatcher binding must fail".to_string());
        }
        Err(error) => error,
    };
    assert!(
        error.contains("dispatcher unavailable")
            || error.contains("binding unavailable")
            || error.contains("selection failed")
    );
    Ok(())
}
