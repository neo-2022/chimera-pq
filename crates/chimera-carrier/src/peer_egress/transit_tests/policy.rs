use std::io::Write;

use chimera_session::FrameKind;

use super::super::{
    BoundPeerTransitPolicy, PeerTransitPolicy, forward_bound_peer_sealed_transit_to_next_hop,
    forward_peer_sealed_transit_to_next_hop, relay_local_bound_sealed_transit,
    validate_transit_relay_frame,
};
use super::helpers::{binding, encoded_frame, tcp_pair, test_peer_pair};
use crate::peer_egress::wire::{PeerMessage, read_peer_message, write_connect_message};

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
fn local_bound_sealed_transit_rejects_midstream_binding_change() -> Result<(), String> {
    let (mut local_writer, local_reader) = tcp_pair()?;
    let (mut peer_writer, peer_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 831, b"first bound lane payload");
    let changed_encoded = encoded_frame(FrameKind::Data, 832, b"changed bound lane payload");
    let first_bound = crate::peer_egress::transit_binding::BoundTransitRelayFrame::new(
        binding(91, 1),
        validate_transit_relay_frame(&first_encoded)?,
    );
    let changed_bound = crate::peer_egress::transit_binding::BoundTransitRelayFrame::new(
        binding(91, 2),
        validate_transit_relay_frame(&changed_encoded)?,
    );
    let first_payload =
        crate::peer_egress::transit_binding::encode_bound_transit_relay_frame(&first_bound);
    let changed_payload =
        crate::peer_egress::transit_binding::encode_bound_transit_relay_frame(&changed_bound);
    let first_byte = first_payload[0];
    local_writer
        .write_all(&first_payload[1..])
        .map_err(|error| format!("write first local bound payload failed: {error}"))?;
    local_writer
        .write_all(&changed_payload)
        .map_err(|error| format!("write changed local bound payload failed: {error}"))?;
    drop(local_writer);

    let error = match relay_local_bound_sealed_transit(local_reader, peer_reader, first_byte) {
        Ok(()) => return Err("local bound transit binding change must fail".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("binding changed"));
    let forwarded = peer_writer.read_secure_payload()?;
    assert_eq!(forwarded, first_payload);
    assert!(peer_writer.read_secure_payload().is_err());
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
fn peer_sealed_transit_rejects_ambiguous_pool_next_hop() -> Result<(), String> {
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (first_next_writer, _first_next_reader) = test_peer_pair()?;
    let (second_next_writer, _second_next_reader) = test_peer_pair()?;
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
    Ok(())
}

#[test]
fn peer_sealed_transit_rejects_midstream_connect_and_unblocks_reverse() -> Result<(), String> {
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 501, b"closed transit payload");
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
    let destination = crate::peer_egress::protocol::Destination {
        host: "example.org".to_string(),
        port: 443,
    };
    write_connect_message(&mut source_writer, &destination)?;

    let error = match forward_peer_sealed_transit_to_next_hop(
        source_reader,
        PeerTransitPolicy::AllowPoolNextHop,
        Some(pool),
        first_frame,
    ) {
        Ok(()) => return Err("midstream CONNECT in sealed transit must fail".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("connect message"));
    assert_eq!(next_reader.read_secure_payload()?, first_encoded);
    assert!(source_writer.read_secure_payload().is_err());
    Ok(())
}

#[test]
fn peer_sealed_transit_rejects_midstream_bound_frame() -> Result<(), String> {
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 901, b"closed transit payload");
    let nested_encoded = encoded_frame(FrameKind::Data, 902, b"nested bound payload");
    source_writer.write_secure_payload(&first_encoded)?;
    let mut source_reader = source_reader;
    let first_frame = match read_peer_message(
        &mut source_reader,
        crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
    )? {
        PeerMessage::SealedTransit(frame) => frame,
        other => return Err(format!("unexpected first message: {other:?}")),
    };
    let nested_bound = crate::peer_egress::transit_binding::BoundTransitRelayFrame::new(
        binding(99, 5),
        validate_transit_relay_frame(&nested_encoded)?,
    );
    let nested_payload =
        crate::peer_egress::transit_binding::encode_bound_transit_relay_frame(&nested_bound);
    source_writer.write_secure_payload(&nested_payload)?;
    let pool = crate::peer_egress::pool::new_shared_pool();
    pool.push(next_writer)?;

    let error = match forward_peer_sealed_transit_to_next_hop(
        source_reader,
        PeerTransitPolicy::AllowPoolNextHop,
        Some(pool),
        first_frame,
    ) {
        Ok(()) => return Err("midstream bound transit frame must fail".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("nested bound transit frame"));
    assert_eq!(next_reader.read_secure_payload()?, first_encoded);
    assert!(source_writer.read_secure_payload().is_err());
    Ok(())
}
