use chimera_mesh::MeshMultipathFlowKey;
use chimera_session::FrameKind;

use super::helpers::{binding, encoded_frame, test_peer_pair};
use crate::peer_egress::lane_binding::{TransitLaneDocument, transit_lane_document_from_mesh_plan};
use crate::peer_egress::live_lane_selection::select_carrier_lane_from_mesh_plan;
use crate::peer_egress::transit::forward_peer_sealed_transit_to_planned_next_hop;
use crate::peer_egress::wire::{PeerMessage, read_peer_message};

fn document() -> Result<TransitLaneDocument, String> {
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
        concat!(
            "mesh_allowed_regions=eu;",
            "mesh_max_peers=2;",
            "mesh_max_selected_per_region=2;",
            "mesh_multipath_mode=flow_shard;",
            "mesh_route_binding_id=7003"
        ),
    )?;
    transit_lane_document_from_mesh_plan(&plan)
}

#[test]
fn planned_peer_ingress_selection_dispatches_selected_lane_from_mesh_path_plan()
-> Result<(), String> {
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (selected_peer_writer, mut selected_peer_reader) = test_peer_pair()?;
    let (wrong_peer_writer, mut wrong_peer_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 615, b"planned peer ingress opaque payload");
    let fin_encoded = encoded_frame(FrameKind::Fin, 616, b"");
    let reverse_encoded = encoded_frame(FrameKind::Data, 617, b"planned peer reverse payload");
    let reverse_fin_encoded = encoded_frame(FrameKind::Fin, 618, b"");
    source_writer.write_secure_payload(&first_encoded)?;

    let mut source_reader = source_reader;
    source_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set source timeout failed: {error}"))?;
    let first_frame = match read_peer_message(
        &mut source_reader,
        crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
    )? {
        PeerMessage::SealedTransit(frame) => frame,
        other => return Err(format!("unexpected first message: {other:?}")),
    };

    let document = document()?;
    let plan = document
        .mesh_path_plan()?
        .ok_or_else(|| "planned lane document missing snapshot".to_string())?;
    let flow_key = MeshMultipathFlowKey::from_opaque_flow_bytes(first_frame.sealed_bytes())?;
    let selection = select_carrier_lane_from_mesh_plan(&plan, flow_key);
    let selected_binding = selection
        .selected_binding
        .ok_or_else(|| "planned binding missing".to_string())?;

    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(selected_binding, selected_peer_writer)?;
    dispatcher.register(binding(7104, 99), wrong_peer_writer)?;
    selected_peer_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set selected timeout failed: {error}"))?;
    wrong_peer_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set wrong timeout failed: {error}"))?;
    selected_peer_reader.write_secure_payload(&reverse_encoded)?;
    selected_peer_reader.write_secure_payload(&reverse_fin_encoded)?;

    source_writer.write_secure_payload(&fin_encoded)?;
    forward_peer_sealed_transit_to_planned_next_hop(
        source_reader,
        &plan,
        Some(dispatcher),
        first_frame,
    )?;

    assert_eq!(selected_peer_reader.read_secure_payload()?, first_encoded);
    assert_eq!(selected_peer_reader.read_secure_payload()?, fin_encoded);
    assert_eq!(source_writer.read_secure_payload()?, reverse_encoded);
    assert_eq!(source_writer.read_secure_payload()?, reverse_fin_encoded);
    assert!(wrong_peer_reader.read_secure_payload().is_err());
    Ok(())
}

#[test]
fn planned_peer_ingress_selection_fails_closed_without_dispatcher_binding() -> Result<(), String> {
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (wrong_peer_writer, mut wrong_peer_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 719, b"planned peer ingress secret marker");
    source_writer.write_secure_payload(&first_encoded)?;

    let mut source_reader = source_reader;
    source_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set source timeout failed: {error}"))?;
    let first_frame = match read_peer_message(
        &mut source_reader,
        crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
    )? {
        PeerMessage::SealedTransit(frame) => frame,
        other => return Err(format!("unexpected first message: {other:?}")),
    };

    let document = document()?;
    let plan = document
        .mesh_path_plan()?
        .ok_or_else(|| "planned lane document missing snapshot".to_string())?;
    let flow_key = MeshMultipathFlowKey::from_opaque_flow_bytes(first_frame.sealed_bytes())?;
    let selection = select_carrier_lane_from_mesh_plan(&plan, flow_key);
    let selected_binding = selection
        .selected_binding
        .ok_or_else(|| "planned binding missing".to_string())?;

    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(binding(7104, 99), wrong_peer_writer)?;
    assert!(!dispatcher.contains_binding(selected_binding)?);
    wrong_peer_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set wrong timeout failed: {error}"))?;
    source_writer
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set source writer timeout failed: {error}"))?;

    let error = match forward_peer_sealed_transit_to_planned_next_hop(
        source_reader,
        &plan,
        Some(dispatcher),
        first_frame,
    ) {
        Ok(()) => {
            return Err("planned peer ingress without selected binding must fail".to_string());
        }
        Err(error) => error,
    };

    assert!(
        error.contains("sealed transit path binding unavailable")
            || error.contains("binding unavailable")
            || error.contains("selection failed")
            || error.contains("missing binding")
    );
    assert!(!error.contains("planned peer ingress secret marker"));
    assert!(wrong_peer_reader.read_secure_payload().is_err());
    assert!(source_writer.read_secure_payload().is_err());
    Ok(())
}
