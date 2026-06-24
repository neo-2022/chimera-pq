use std::io::Write;

use chimera_session::FrameKind;

use super::super::{
    BoundPeerTransitPolicy, PeerTransitPolicy, forward_bound_peer_sealed_transit_to_next_hop,
    forward_peer_sealed_transit_to_next_hop, validate_transit_relay_frame,
};
use super::helpers::{
    assert_bound_payload, assert_bytes_eq_redacted, binding, bound_payload, encoded_frame,
    read_first_bound_frame, tcp_pair, test_peer_pair,
};
use crate::peer_egress::transit_local::relay_local_bound_sealed_transit;
use crate::peer_egress::wire::{
    PeerMessage, read_peer_message, write_ack_ok, write_connect_message,
};

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
    assert_bytes_eq_redacted(&forwarded, &first_payload, "local bound first payload")?;
    assert!(peer_writer.read_secure_payload().is_err());
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
    assert_bytes_eq_redacted(
        &next_reader.read_secure_payload()?,
        &first_encoded,
        "midstream connect first frame",
    )?;
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
    assert_bytes_eq_redacted(
        &next_reader.read_secure_payload()?,
        &first_encoded,
        "midstream nested bound first frame",
    )?;
    assert!(source_writer.read_secure_payload().is_err());
    Ok(())
}

#[test]
fn bound_peer_transit_rejects_midstream_binding_change() -> Result<(), String> {
    let path_binding = binding(191, 1);
    let changed_binding = binding(191, 2);
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let (first_payload, first_sealed) = bound_payload(
        path_binding,
        FrameKind::Data,
        1501,
        b"peer bound first payload",
    )?;
    let (changed_payload, changed_sealed) = bound_payload(
        changed_binding,
        FrameKind::Data,
        1502,
        b"PEER_BOUND_CHANGED_PAYLOAD_MARKER",
    )?;

    source_writer.write_secure_payload(&first_payload)?;
    let mut source_reader = source_reader;
    let first_bound = read_first_bound_frame(&mut source_reader)?;
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(path_binding, next_writer)?;

    next_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set next timeout failed: {error}"))?;
    source_writer.write_secure_payload(&changed_payload)?;

    let error = match forward_bound_peer_sealed_transit_to_next_hop(
        source_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        Some(dispatcher),
        first_bound,
    ) {
        Ok(()) => return Err("peer bound transit binding change must fail".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("binding changed"));
    assert!(!error.contains("PEER_BOUND_CHANGED_PAYLOAD_MARKER"));
    assert_bound_payload(
        &next_reader.read_secure_payload()?,
        path_binding,
        &first_sealed,
    )?;
    let changed_result = next_reader.read_secure_payload();
    assert!(changed_result.is_err());
    assert!(!format!("{changed_result:?}").contains("PEER_BOUND_CHANGED_PAYLOAD_MARKER"));
    assert!(!changed_sealed.is_empty());
    Ok(())
}

#[test]
fn bound_peer_transit_rejects_midstream_unbound_frame() -> Result<(), String> {
    let binding = binding(192, 1);
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let (first_payload, first_sealed) =
        bound_payload(binding, FrameKind::Data, 1601, b"peer bound first payload")?;
    let unbound = encoded_frame(FrameKind::Data, 1602, b"UNBOUND_MARKER_DO_NOT_FORWARD");

    source_writer.write_secure_payload(&first_payload)?;
    let mut source_reader = source_reader;
    let first_bound = read_first_bound_frame(&mut source_reader)?;
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(binding, next_writer)?;

    next_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set next timeout failed: {error}"))?;
    source_writer.write_secure_payload(&unbound)?;

    let error = match forward_bound_peer_sealed_transit_to_next_hop(
        source_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        Some(dispatcher),
        first_bound,
    ) {
        Ok(()) => return Err("peer bound transit must reject unbound midstream frame".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("unbound transit frame"));
    assert!(!error.contains("UNBOUND_MARKER_DO_NOT_FORWARD"));
    assert_bound_payload(&next_reader.read_secure_payload()?, binding, &first_sealed)?;
    assert!(next_reader.read_secure_payload().is_err());
    Ok(())
}

#[test]
fn bound_peer_transit_rejects_midstream_connect_and_ack() -> Result<(), String> {
    let binding = binding(193, 1);
    let (case, write_invalid, expected) = (
        "connect",
        write_connect_message
            as fn(
                &mut crate::peer_egress::protocol::SecurePeerStream,
                &crate::peer_egress::protocol::Destination,
            ) -> Result<(), String>,
        "connect message",
    );
    {
        let (mut source_writer, source_reader) = test_peer_pair()?;
        let (next_writer, mut next_reader) = test_peer_pair()?;
        let (first_payload, first_sealed) = bound_payload(
            binding,
            FrameKind::Data,
            1701,
            format!("peer bound first payload {case}").as_bytes(),
        )?;
        source_writer.write_secure_payload(&first_payload)?;
        let mut source_reader = source_reader;
        let first_bound = read_first_bound_frame(&mut source_reader)?;
        let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
        dispatcher.register(binding, next_writer)?;
        next_reader
            .stream
            .set_read_timeout(Some(std::time::Duration::from_millis(50)))
            .map_err(|error| format!("set next timeout failed: {error}"))?;

        let destination = crate::peer_egress::protocol::Destination {
            host: "example.org".to_string(),
            port: 443,
        };
        write_invalid(&mut source_writer, &destination)?;
        let error = match forward_bound_peer_sealed_transit_to_next_hop(
            source_reader,
            BoundPeerTransitPolicy::AllowBoundNextHop,
            Some(dispatcher),
            first_bound,
        ) {
            Ok(()) => return Err(format!("peer bound transit must reject {case} midstream")),
            Err(error) => error,
        };
        assert!(error.contains(expected));
        assert!(!error.contains("example.org"));
        assert_bound_payload(&next_reader.read_secure_payload()?, binding, &first_sealed)?;
        assert!(next_reader.read_secure_payload().is_err());
    }

    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let (first_payload, first_sealed) =
        bound_payload(binding, FrameKind::Data, 1801, b"peer bound first ack")?;
    source_writer.write_secure_payload(&first_payload)?;
    let mut source_reader = source_reader;
    let first_bound = read_first_bound_frame(&mut source_reader)?;
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(binding, next_writer)?;
    next_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set next timeout failed: {error}"))?;
    write_ack_ok(&mut source_writer)?;

    let error = match forward_bound_peer_sealed_transit_to_next_hop(
        source_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        Some(dispatcher),
        first_bound,
    ) {
        Ok(()) => return Err("peer bound transit must reject ack midstream".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("ack"));
    assert_bound_payload(&next_reader.read_secure_payload()?, binding, &first_sealed)?;
    assert!(next_reader.read_secure_payload().is_err());
    Ok(())
}
