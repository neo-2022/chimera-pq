use chimera_session::FrameKind;

use super::super::{
    BoundPeerTransitPolicy, PeerTransitPolicy, forward_bound_peer_sealed_transit_to_next_hop,
    forward_peer_sealed_transit_to_next_hop,
};
use super::helpers::{
    assert_bound_payload, binding, bound_payload, encoded_frame, read_first_bound_frame,
    test_peer_pair,
};
use crate::peer_egress::wire::{PeerMessage, read_peer_message};

#[test]
fn peer_sealed_transit_pumps_both_directions_without_connect_parsing() -> Result<(), String> {
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 201, b"closed transit payload");
    let fin_encoded = encoded_frame(FrameKind::Fin, 202, b"");
    let reverse_encoded = encoded_frame(FrameKind::Data, 301, b"closed reverse payload");
    let reverse_fin_encoded = encoded_frame(FrameKind::Fin, 302, b"");
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

    source_writer.write_secure_payload(&fin_encoded)?;
    next_reader.write_secure_payload(&reverse_encoded)?;
    next_reader.write_secure_payload(&reverse_fin_encoded)?;
    forward_peer_sealed_transit_to_next_hop(
        source_reader,
        PeerTransitPolicy::AllowPoolNextHop,
        Some(pool),
        first_frame,
    )?;

    let forwarded_first = next_reader.read_secure_payload()?;
    let forwarded_fin = next_reader.read_secure_payload()?;
    let reverse_first = source_writer.read_secure_payload()?;
    let reverse_fin = source_writer.read_secure_payload()?;
    assert_eq!(forwarded_first, first_encoded);
    assert_eq!(forwarded_fin, fin_encoded);
    assert_eq!(reverse_first, reverse_encoded);
    assert_eq!(reverse_fin, reverse_fin_encoded);
    assert!(
        !String::from_utf8_lossy(&forwarded_first)
            .to_ascii_uppercase()
            .contains("CONNECT")
    );
    assert!(
        !String::from_utf8_lossy(&reverse_first)
            .to_ascii_uppercase()
            .contains("CONNECT")
    );
    Ok(())
}

#[test]
fn bound_peer_sealed_transit_dispatches_matching_next_hop() -> Result<(), String> {
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let (wrong_writer, mut wrong_reader) = test_peer_pair()?;
    let path_binding = binding(77, 3);
    let wrong_binding = binding(77, 4);
    let (first_bound_payload, first_sealed) = bound_payload(
        path_binding,
        FrameKind::Data,
        601,
        b"bound closed transit payload",
    )?;
    let (fin_bound_payload, fin_sealed) = bound_payload(path_binding, FrameKind::Fin, 602, b"")?;
    let (reverse_bound_payload, reverse_sealed) = bound_payload(
        path_binding,
        FrameKind::Data,
        701,
        b"bound closed reverse payload",
    )?;
    let (reverse_fin_bound_payload, reverse_fin_sealed) =
        bound_payload(path_binding, FrameKind::Fin, 702, b"")?;

    source_writer.write_secure_payload(&first_bound_payload)?;
    let mut source_reader = source_reader;
    let first_bound = read_first_bound_frame(&mut source_reader)?;
    let first_bound_debug = format!("{first_bound:?}");
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(wrong_binding, wrong_writer)?;
    dispatcher.register(path_binding, next_writer)?;
    wrong_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set wrong next-hop timeout failed: {error}"))?;

    source_writer.write_secure_payload(&fin_bound_payload)?;
    next_reader.write_secure_payload(&reverse_bound_payload)?;
    next_reader.write_secure_payload(&reverse_fin_bound_payload)?;
    forward_bound_peer_sealed_transit_to_next_hop(
        source_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        Some(dispatcher.clone()),
        first_bound,
    )?;

    let forwarded_first = next_reader.read_secure_payload()?;
    let forwarded_fin = next_reader.read_secure_payload()?;
    let reverse_first = source_writer.read_secure_payload()?;
    let reverse_fin = source_writer.read_secure_payload()?;
    assert_bound_payload(&forwarded_first, path_binding, &first_sealed)?;
    assert_bound_payload(&forwarded_fin, path_binding, &fin_sealed)?;
    assert_bound_payload(&reverse_first, path_binding, &reverse_sealed)?;
    assert_bound_payload(&reverse_fin, path_binding, &reverse_fin_sealed)?;
    assert!(wrong_reader.read_secure_payload().is_err());
    assert!(dispatcher.contains_binding(wrong_binding)?);
    assert!(!first_bound_debug.contains("bound closed transit payload"));
    assert!(!first_bound_debug.contains("route_id: 77"));
    assert!(!first_bound_debug.contains("lane_id: 3"));
    Ok(())
}

#[test]
fn same_bound_lane_can_forward_second_stream_after_replenishment() -> Result<(), String> {
    let binding = binding(177, 9);
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();

    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let (first_bound_payload, first_sealed) =
        bound_payload(binding, FrameKind::Data, 1001, b"first sealed lane payload")?;
    let (fin_bound_payload, fin_sealed) = bound_payload(binding, FrameKind::Fin, 1002, b"")?;
    let (reverse_bound_payload, reverse_sealed) = bound_payload(
        binding,
        FrameKind::Data,
        1101,
        b"first sealed reverse payload",
    )?;
    let (reverse_fin_bound_payload, reverse_fin_sealed) =
        bound_payload(binding, FrameKind::Fin, 1102, b"")?;
    source_writer.write_secure_payload(&first_bound_payload)?;
    let mut source_reader = source_reader;
    let first_bound = read_first_bound_frame(&mut source_reader)?;
    let first_bound_debug = format!("{first_bound:?}");
    dispatcher.register(binding, next_writer)?;

    source_writer.write_secure_payload(&fin_bound_payload)?;
    next_reader.write_secure_payload(&reverse_bound_payload)?;
    next_reader.write_secure_payload(&reverse_fin_bound_payload)?;
    forward_bound_peer_sealed_transit_to_next_hop(
        source_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        Some(dispatcher.clone()),
        first_bound,
    )?;
    assert!(!dispatcher.contains_binding(binding)?);
    assert_bound_payload(&next_reader.read_secure_payload()?, binding, &first_sealed)?;
    assert_bound_payload(&next_reader.read_secure_payload()?, binding, &fin_sealed)?;
    assert_bound_payload(
        &source_writer.read_secure_payload()?,
        binding,
        &reverse_sealed,
    )?;
    assert_bound_payload(
        &source_writer.read_secure_payload()?,
        binding,
        &reverse_fin_sealed,
    )?;

    let (mut denied_writer, denied_reader) = test_peer_pair()?;
    let (denied_payload, _) = bound_payload(
        binding,
        FrameKind::Data,
        1201,
        b"second sealed lane payload",
    )?;
    denied_writer.write_secure_payload(&denied_payload)?;
    let mut denied_reader = denied_reader;
    let denied_first = read_first_bound_frame(&mut denied_reader)?;
    let error = match forward_bound_peer_sealed_transit_to_next_hop(
        denied_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        Some(dispatcher.clone()),
        denied_first,
    ) {
        Ok(()) => return Err("bound lane must require replenishment after claim".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("binding unavailable"));

    let (mut second_writer, second_reader) = test_peer_pair()?;
    let (second_next_writer, mut second_next_reader) = test_peer_pair()?;
    let (second_payload, second_sealed) = bound_payload(
        binding,
        FrameKind::Data,
        1301,
        b"replenished sealed lane payload",
    )?;
    let (second_fin_payload, second_fin_sealed) =
        bound_payload(binding, FrameKind::Fin, 1302, b"")?;
    let (second_reverse_payload, second_reverse_sealed) =
        bound_payload(binding, FrameKind::Data, 1401, b"replenished reverse")?;
    let (second_reverse_fin_payload, second_reverse_fin_sealed) =
        bound_payload(binding, FrameKind::Fin, 1402, b"")?;
    second_writer.write_secure_payload(&second_payload)?;
    let mut second_reader = second_reader;
    let second_first = read_first_bound_frame(&mut second_reader)?;
    let second_first_debug = format!("{second_first:?}");
    dispatcher.register(binding, second_next_writer)?;

    second_writer.write_secure_payload(&second_fin_payload)?;
    second_next_reader.write_secure_payload(&second_reverse_payload)?;
    second_next_reader.write_secure_payload(&second_reverse_fin_payload)?;
    forward_bound_peer_sealed_transit_to_next_hop(
        second_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        Some(dispatcher.clone()),
        second_first,
    )?;
    assert!(!dispatcher.contains_binding(binding)?);
    assert_bound_payload(
        &second_next_reader.read_secure_payload()?,
        binding,
        &second_sealed,
    )?;
    assert_bound_payload(
        &second_next_reader.read_secure_payload()?,
        binding,
        &second_fin_sealed,
    )?;
    assert_bound_payload(
        &second_writer.read_secure_payload()?,
        binding,
        &second_reverse_sealed,
    )?;
    assert_bound_payload(
        &second_writer.read_secure_payload()?,
        binding,
        &second_reverse_fin_sealed,
    )?;
    assert!(!first_bound_debug.contains("first sealed lane payload"));
    assert!(!second_first_debug.contains("replenished sealed lane payload"));
    Ok(())
}
