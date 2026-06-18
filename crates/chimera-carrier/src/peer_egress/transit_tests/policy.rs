use chimera_session::FrameKind;

use super::super::{
    BoundPeerTransitPolicy, PeerTransitPolicy, forward_bound_peer_sealed_transit_to_next_hop,
    forward_peer_sealed_transit_to_next_hop, validate_transit_relay_frame,
};
use super::helpers::{binding, encoded_frame, test_peer_pair};
use crate::peer_egress::wire::{PeerMessage, read_peer_message};

#[test]
fn bound_peer_sealed_transit_fails_closed_without_explicit_binding() -> Result<(), String> {
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (first_next_writer, mut first_next_reader) = test_peer_pair()?;
    let (second_next_writer, mut second_next_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 621, b"bound lane payload");
    let fin_encoded = encoded_frame(FrameKind::Fin, 622, b"");
    let binding = binding(79, 2);
    let first_frame = validate_transit_relay_frame(&first_encoded)?;
    let bound_first =
        crate::peer_egress::transit_binding::BoundTransitRelayFrame::new(binding, first_frame);
    let bound_payload =
        crate::peer_egress::transit_binding::encode_bound_transit_relay_frame(&bound_first);

    source_writer.write_secure_payload(&bound_payload)?;
    let mut source_reader = source_reader;
    let first_bound = match read_peer_message(
        &mut source_reader,
        crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
    )? {
        PeerMessage::BoundSealedTransit(frame) => frame,
        other => return Err(format!("unexpected first bound message: {other:?}")),
    };
    let pool = crate::peer_egress::pool::new_shared_pool();
    pool.push(first_next_writer)?;
    pool.push(second_next_writer)?;

    source_writer.write_secure_payload(&fin_encoded)?;
    first_next_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set wrong lane read timeout failed: {error}"))?;
    second_next_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set lane read timeout failed: {error}"))?;
    let error = match forward_bound_peer_sealed_transit_to_next_hop(
        source_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        Some(crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher()),
        first_bound,
    ) {
        Ok(()) => return Err("bound transit without explicit binding must fail".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("binding unavailable"));
    assert!(pool.try_pop()?.is_some());
    assert!(pool.try_pop()?.is_some());
    assert!(first_next_reader.read_secure_payload().is_err());
    assert!(second_next_reader.read_secure_payload().is_err());
    Ok(())
}

#[test]
fn bound_peer_sealed_transit_requires_policy_dispatcher_and_known_binding() -> Result<(), String> {
    let (_source_writer, source_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 801, b"bound closed transit payload");
    let binding = binding(88, 4);
    let first = crate::peer_egress::transit_binding::BoundTransitRelayFrame::new(
        binding,
        validate_transit_relay_frame(&first_encoded)?,
    );

    let error = match forward_bound_peer_sealed_transit_to_next_hop(
        source_reader,
        BoundPeerTransitPolicy::DenyBoundNextHop,
        None,
        first.clone(),
    ) {
        Ok(()) => return Err("bound transit denied by policy must fail".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("denied by policy"));

    let (_source_writer, source_reader) = test_peer_pair()?;
    let error = match forward_bound_peer_sealed_transit_to_next_hop(
        source_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        None,
        first.clone(),
    ) {
        Ok(()) => return Err("bound transit without dispatcher must fail".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("dispatcher unavailable"));

    let (_source_writer, source_reader) = test_peer_pair()?;
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    let error = match forward_bound_peer_sealed_transit_to_next_hop(
        source_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        Some(dispatcher),
        first,
    ) {
        Ok(()) => return Err("bound transit with unknown binding must fail".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("binding unavailable"));
    Ok(())
}

#[test]
fn peer_sealed_transit_denies_pool_next_hop_without_policy() -> Result<(), String> {
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, _next_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 401, b"closed transit payload");
    source_writer.write_secure_payload(&first_encoded)?;
    let mut source_reader = source_reader;
    let first_frame = match read_peer_message(
        &mut source_reader,
        crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
    )? {
        PeerMessage::SealedTransit(frame) => frame,
        other => return Err(format!("unexpected first message: {other:?}")),
    };
    let pool = crate::peer_egress::pool::new_shared_pool();
    pool.push(next_writer)?;

    let error = match forward_peer_sealed_transit_to_next_hop(
        source_reader,
        PeerTransitPolicy::DenyPoolNextHop,
        Some(pool),
        first_frame,
    ) {
        Ok(()) => return Err("pool transit without policy must fail".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("denied by policy"));
    Ok(())
}

#[test]
fn peer_sealed_transit_fails_closed_with_empty_pool() -> Result<(), String> {
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 441, b"closed transit payload");
    source_writer.write_secure_payload(&first_encoded)?;
    let mut source_reader = source_reader;
    let first_frame = match read_peer_message(
        &mut source_reader,
        crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
    )? {
        PeerMessage::SealedTransit(frame) => frame,
        other => return Err(format!("unexpected first message: {other:?}")),
    };
    let error = match forward_peer_sealed_transit_to_next_hop(
        source_reader,
        PeerTransitPolicy::AllowPoolNextHop,
        Some(crate::peer_egress::pool::new_shared_pool()),
        first_frame,
    ) {
        Ok(()) => return Err("empty pool transit next hop must fail".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("unavailable"));
    Ok(())
}

#[test]
fn peer_sealed_transit_rejects_ambiguous_pool_next_hop() -> Result<(), String> {
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (first_next_writer, mut first_next_reader) = test_peer_pair()?;
    let (second_next_writer, mut second_next_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 451, b"closed transit payload");
    source_writer.write_secure_payload(&first_encoded)?;
    let mut source_reader = source_reader;
    let first_frame = match read_peer_message(
        &mut source_reader,
        crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
    )? {
        PeerMessage::SealedTransit(frame) => frame,
        other => return Err(format!("unexpected first message: {other:?}")),
    };
    let pool = crate::peer_egress::pool::new_shared_pool();
    pool.push(first_next_writer)?;
    pool.push(second_next_writer)?;
    first_next_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set first transit timeout failed: {error}"))?;
    second_next_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set second transit timeout failed: {error}"))?;

    let error = match forward_peer_sealed_transit_to_next_hop(
        source_reader,
        PeerTransitPolicy::AllowPoolNextHop,
        Some(pool),
        first_frame,
    ) {
        Ok(()) => return Err("ambiguous pool transit next hop must fail".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("ambiguous"));
    assert!(first_next_reader.read_secure_payload().is_err());
    assert!(second_next_reader.read_secure_payload().is_err());
    Ok(())
}
