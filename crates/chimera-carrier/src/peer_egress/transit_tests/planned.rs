use chimera_mesh::MeshMultipathFlowKey;
use chimera_session::FrameKind;
use std::io::Write;

use super::helpers::{binding, encoded_frame, tcp_pair, test_peer_pair};
use crate::peer_egress::lane_binding::TransitLaneRegistration;
use crate::peer_egress::live_lane_selection::select_carrier_lane_from_registrations;
use crate::peer_egress::transit::{
    forward_peer_sealed_transit_with_registrations, relay_local_sealed_transit_with_registrations,
};
use crate::peer_egress::wire::{PeerMessage, read_peer_message};

fn registrations() -> Result<Vec<TransitLaneRegistration>, String> {
    Ok(vec![
        TransitLaneRegistration::new(binding(91, 1), "198.51.100.91:443".to_string())?,
        TransitLaneRegistration::new(binding(91, 2), "198.51.100.92:443".to_string())?,
    ])
}

#[test]
fn planned_runtime_selection_dispatches_only_the_selected_lane() -> Result<(), String> {
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (selected_peer_writer, mut selected_peer_reader) = test_peer_pair()?;
    let (_wrong_peer_writer, mut wrong_peer_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 611, b"planned transit payload");
    let fin_encoded = encoded_frame(FrameKind::Fin, 612, b"");
    let reverse_encoded = encoded_frame(FrameKind::Data, 613, b"planned reverse payload");
    let reverse_fin_encoded = encoded_frame(FrameKind::Fin, 614, b"");
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

    let registrations = registrations()?;
    let flow_key = MeshMultipathFlowKey::from_opaque_flow_bytes(first_frame.sealed_bytes())?;
    let expected_selection = select_carrier_lane_from_registrations(&registrations, flow_key)?;
    let expected_binding = expected_selection
        .selected_binding
        .ok_or_else(|| "selected binding missing".to_string())?;

    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    selected_peer_writer
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set selected writer timeout failed: {error}"))?;
    dispatcher.register(expected_binding, selected_peer_writer)?;

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
    let forward_result = forward_peer_sealed_transit_with_registrations(
        source_reader,
        &registrations,
        Some(dispatcher),
        first_frame,
    );
    assert!(forward_result.is_ok());
    let selected_first = selected_peer_reader.read_secure_payload()?;
    let selected_fin = selected_peer_reader.read_secure_payload()?;
    assert_eq!(selected_first, first_encoded);
    assert_eq!(selected_fin, fin_encoded);
    assert_eq!(source_writer.read_secure_payload()?, reverse_encoded);
    assert_eq!(source_writer.read_secure_payload()?, reverse_fin_encoded);
    assert!(wrong_peer_reader.read_secure_payload().is_err());
    Ok(())
}

#[test]
fn planned_runtime_selection_fails_closed_without_selected_binding() -> Result<(), String> {
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (wrong_peer_writer, mut wrong_peer_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 721, b"planned transit payload");
    source_writer.write_secure_payload(&first_encoded)?;

    let mut source_reader = source_reader;
    let first_frame = match read_peer_message(
        &mut source_reader,
        crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
    )? {
        PeerMessage::SealedTransit(frame) => frame,
        other => return Err(format!("unexpected first message: {other:?}")),
    };

    let registrations = registrations()?;
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(binding(93, 3), wrong_peer_writer)?;
    wrong_peer_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set wrong timeout failed: {error}"))?;

    let error = match forward_peer_sealed_transit_with_registrations(
        source_reader,
        &registrations,
        Some(dispatcher),
        first_frame,
    ) {
        Ok(()) => return Err("planned transit without selected binding must fail".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("binding unavailable") || error.contains("selection failed"));
    assert!(wrong_peer_reader.read_secure_payload().is_err());
    Ok(())
}

#[test]
fn planned_local_ingress_selection_dispatches_selected_lane() -> Result<(), String> {
    let (mut local_writer, local_reader) = tcp_pair()?;
    let (selected_peer_writer, mut selected_peer_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 811, b"planned local payload");
    let fin_encoded = encoded_frame(FrameKind::Fin, 812, b"");
    let registrations = vec![TransitLaneRegistration::new(
        binding(94, 4),
        "198.51.100.94:443".to_string(),
    )?];
    let first_byte = first_encoded[0];
    local_writer
        .write_all(&first_encoded[1..])
        .map_err(|error| format!("write local payload failed: {error}"))?;
    local_writer
        .write_all(&fin_encoded)
        .map_err(|error| format!("write local fin failed: {error}"))?;
    drop(local_writer);

    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(binding(94, 4), selected_peer_writer)?;
    selected_peer_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set selected timeout failed: {error}"))?;

    relay_local_sealed_transit_with_registrations(
        local_reader,
        &registrations,
        Some(dispatcher),
        first_byte,
    )?;

    assert_eq!(selected_peer_reader.read_secure_payload()?, first_encoded);
    assert_eq!(selected_peer_reader.read_secure_payload()?, fin_encoded);
    Ok(())
}
