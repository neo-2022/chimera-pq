use chimera_session::FrameKind;
use std::io::Write;

use super::helpers::{
    assert_bytes_eq_redacted, binding, bound_payload, encoded_frame, tcp_pair, test_peer_pair,
};
use crate::peer_egress::pool::new_shared_pool;
use crate::peer_egress::transit::BoundPeerTransitPolicy;
use crate::peer_egress::transit::{
    PeerTransitPolicy, forward_peer_sealed_transit_to_next_hop_with_limits,
};
use crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher;
use crate::peer_egress::transit_guard::TransitRelayLimits;
use crate::peer_egress::transit_local::{
    relay_local_bound_sealed_transit_to_next_hop_with_limits,
    relay_local_bound_sealed_transit_with_limits,
    relay_local_sealed_transit_to_next_hop_with_limits, relay_local_sealed_transit_with_limits,
};
use crate::peer_egress::wire::{PeerMessage, read_peer_message};

fn read_first_transit_frame(
    peer: &mut crate::peer_egress::protocol::SecurePeerStream,
) -> Result<crate::peer_egress::transit::TransitRelayFrame, String> {
    match read_peer_message(
        peer,
        crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
    )? {
        PeerMessage::SealedTransit(frame) => Ok(frame),
        other => Err(format!("unexpected first transit message: {other:?}")),
    }
}

#[test]
fn peer_transit_frame_budget_fails_closed_without_payload_leak() -> Result<(), String> {
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 701, b"first guard payload");
    let second_encoded = encoded_frame(FrameKind::Data, 702, b"SECRET_GUARD_FRAME_BUDGET");
    source_writer.write_secure_payload(&first_encoded)?;
    source_writer.write_secure_payload(&second_encoded)?;
    let mut source_reader = source_reader;
    let first_frame = read_first_transit_frame(&mut source_reader)?;
    let pool = new_shared_pool();
    pool.push(next_writer)?;

    let error = match forward_peer_sealed_transit_to_next_hop_with_limits(
        source_reader,
        PeerTransitPolicy::AllowPoolNextHop,
        Some(pool),
        first_frame,
        TransitRelayLimits::new(1, 1024 * 1024, 50)?,
    ) {
        Ok(()) => return Err("transit frame budget must fail closed".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("frame budget") || error.contains("read secure frame"));
    assert!(!error.contains("SECRET_GUARD_FRAME_BUDGET"));
    assert_bytes_eq_redacted(
        &next_reader.read_secure_payload()?,
        &first_encoded,
        "peer frame-budget first frame",
    )?;
    assert!(next_reader.read_secure_payload().is_err());
    Ok(())
}

#[test]
fn peer_transit_byte_budget_fails_closed_before_forwarding_payload() -> Result<(), String> {
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 801, b"SECRET_GUARD_BYTE_BUDGET");
    source_writer.write_secure_payload(&first_encoded)?;
    let mut source_reader = source_reader;
    let first_frame = read_first_transit_frame(&mut source_reader)?;
    let pool = new_shared_pool();
    pool.push(next_writer)?;

    let error = match forward_peer_sealed_transit_to_next_hop_with_limits(
        source_reader,
        PeerTransitPolicy::AllowPoolNextHop,
        Some(pool),
        first_frame,
        TransitRelayLimits::new(10, first_encoded.len() as u64 - 1, 50)?,
    ) {
        Ok(()) => return Err("transit byte budget must fail closed".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("byte budget"));
    assert!(!error.contains("SECRET_GUARD_BYTE_BUDGET"));
    next_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set next reader timeout failed: {error}"))?;
    assert!(next_reader.read_secure_payload().is_err());
    Ok(())
}

#[test]
fn local_sealed_transit_frame_budget_fails_closed_without_payload_leak() -> Result<(), String> {
    let (mut local_writer, local_reader) = tcp_pair()?;
    let (mut peer_writer, peer_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 811, b"local first guard payload");
    let second_encoded = encoded_frame(FrameKind::Data, 812, b"SECRET_LOCAL_FRAME_BUDGET");
    let first_byte = first_encoded[0];
    local_writer
        .write_all(&first_encoded[1..])
        .map_err(|error| format!("write first local sealed frame failed: {error}"))?;
    local_writer
        .write_all(&second_encoded)
        .map_err(|error| format!("write second local sealed frame failed: {error}"))?;
    drop(local_writer);

    let error = match relay_local_sealed_transit_with_limits(
        local_reader,
        peer_reader,
        first_byte,
        TransitRelayLimits::new(1, 1024 * 1024, 50)?,
    ) {
        Ok(()) => return Err("local sealed transit frame budget must fail closed".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("frame budget"));
    assert!(!error.contains("SECRET_LOCAL_FRAME_BUDGET"));
    assert_bytes_eq_redacted(
        &peer_writer.read_secure_payload()?,
        &first_encoded,
        "local frame-budget first frame",
    )?;
    assert!(peer_writer.read_secure_payload().is_err());
    Ok(())
}

#[test]
fn local_sealed_transit_byte_budget_fails_closed_before_forwarding_payload() -> Result<(), String> {
    let (mut local_writer, local_reader) = tcp_pair()?;
    let (mut peer_writer, peer_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 821, b"SECRET_LOCAL_BYTE_BUDGET");
    let first_byte = first_encoded[0];
    local_writer
        .write_all(&first_encoded[1..])
        .map_err(|error| format!("write local sealed frame failed: {error}"))?;
    drop(local_writer);

    let error = match relay_local_sealed_transit_with_limits(
        local_reader,
        peer_reader,
        first_byte,
        TransitRelayLimits::new(10, first_encoded.len() as u64 - 1, 50)?,
    ) {
        Ok(()) => return Err("local sealed transit byte budget must fail closed".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("byte budget"));
    assert!(!error.contains("SECRET_LOCAL_BYTE_BUDGET"));
    peer_writer
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set peer writer timeout failed: {error}"))?;
    assert!(peer_writer.read_secure_payload().is_err());
    Ok(())
}

#[test]
fn local_bound_transit_frame_budget_fails_closed_without_payload_leak() -> Result<(), String> {
    let path_binding = binding(811, 1);
    let (mut local_writer, local_reader) = tcp_pair()?;
    let (mut peer_writer, peer_reader) = test_peer_pair()?;
    let (first_payload, first_sealed) =
        bound_payload(path_binding, FrameKind::Data, 831, b"local bound first")?;
    let (second_payload, _) = bound_payload(
        path_binding,
        FrameKind::Data,
        832,
        b"SECRET_LOCAL_BOUND_FRAME",
    )?;
    let first_byte = first_payload[0];
    local_writer
        .write_all(&first_payload[1..])
        .map_err(|error| format!("write first local bound frame failed: {error}"))?;
    local_writer
        .write_all(&second_payload)
        .map_err(|error| format!("write second local bound frame failed: {error}"))?;
    drop(local_writer);

    let error = match relay_local_bound_sealed_transit_with_limits(
        local_reader,
        peer_reader,
        first_byte,
        TransitRelayLimits::new(1, 1024 * 1024, 50)?,
    ) {
        Ok(()) => return Err("local bound transit frame budget must fail closed".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("frame budget"));
    assert!(!error.contains("SECRET_LOCAL_BOUND_FRAME"));
    super::helpers::assert_bound_payload(
        &peer_writer.read_secure_payload()?,
        path_binding,
        &first_sealed,
    )?;
    assert!(peer_writer.read_secure_payload().is_err());
    Ok(())
}

#[test]
fn local_bound_transit_byte_budget_fails_closed_before_forwarding_payload() -> Result<(), String> {
    let path_binding = binding(821, 1);
    let (mut local_writer, local_reader) = tcp_pair()?;
    let (mut peer_writer, peer_reader) = test_peer_pair()?;
    let (first_payload, _) = bound_payload(
        path_binding,
        FrameKind::Data,
        841,
        b"SECRET_LOCAL_BOUND_BYTE",
    )?;
    let first_byte = first_payload[0];
    local_writer
        .write_all(&first_payload[1..])
        .map_err(|error| format!("write local bound frame failed: {error}"))?;
    drop(local_writer);

    let error = match relay_local_bound_sealed_transit_with_limits(
        local_reader,
        peer_reader,
        first_byte,
        TransitRelayLimits::new(10, first_payload.len() as u64 - 1, 50)?,
    ) {
        Ok(()) => return Err("local bound transit byte budget must fail closed".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("byte budget"));
    assert!(!error.contains("SECRET_LOCAL_BOUND_BYTE"));
    peer_writer
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set peer writer timeout failed: {error}"))?;
    assert!(peer_writer.read_secure_payload().is_err());
    Ok(())
}

#[test]
fn local_next_hop_first_sealed_frame_idle_timeout_fails_closed() -> Result<(), String> {
    let (local_writer, local_reader) = tcp_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 851, b"never fully written");
    let first_byte = first_encoded[0];
    let pool = new_shared_pool();
    let (next_writer, _next_reader) = test_peer_pair()?;
    pool.push(next_writer)?;

    let error = match relay_local_sealed_transit_to_next_hop_with_limits(
        local_reader,
        PeerTransitPolicy::AllowPoolNextHop,
        pool,
        first_byte,
        TransitRelayLimits::new(10, 1024 * 1024, 20)?,
    ) {
        Ok(()) => return Err("incomplete first local sealed frame must time out".to_string()),
        Err(error) => error,
    };
    drop(local_writer);

    assert!(error.contains("read transit frame"));
    assert!(!error.contains("never fully written"));
    Ok(())
}

#[test]
fn local_next_hop_first_bound_frame_idle_timeout_fails_closed() -> Result<(), String> {
    let (local_writer, local_reader) = tcp_pair()?;
    let dispatcher = new_shared_transit_dispatcher();

    let error = match relay_local_bound_sealed_transit_to_next_hop_with_limits(
        local_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        Some(dispatcher),
        crate::peer_egress::transit_binding::BOUND_TRANSIT_MAGIC,
        TransitRelayLimits::new(10, 1024 * 1024, 20)?,
    ) {
        Ok(()) => return Err("incomplete first local bound frame must time out".to_string()),
        Err(error) => error,
    };
    drop(local_writer);

    assert!(error.contains("read bound transit frame"));
    Ok(())
}
