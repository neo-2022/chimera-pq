use chimera_session::FrameKind;

use super::super::{
    BoundPeerTransitPolicy, PeerTransitPolicy, forward_bound_peer_sealed_transit_to_next_hop,
    forward_peer_sealed_transit_to_next_hop, validate_transit_relay_frame,
};
use super::helpers::{binding, encoded_frame, test_peer_pair};
use crate::peer_egress::wire::{PeerMessage, read_peer_message};

fn read_first_bound_frame(
    source_reader: &mut crate::peer_egress::protocol::SecurePeerStream,
) -> Result<crate::peer_egress::transit_binding::BoundTransitRelayFrame, String> {
    match read_peer_message(
        source_reader,
        crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
    )? {
        PeerMessage::BoundSealedTransit(frame) => Ok(frame),
        other => Err(format!("unexpected first bound message: {other:?}")),
    }
}

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
    let first_encoded = encoded_frame(FrameKind::Data, 601, b"bound closed transit payload");
    let fin_encoded = encoded_frame(FrameKind::Fin, 602, b"");
    let reverse_encoded = encoded_frame(FrameKind::Data, 701, b"bound closed reverse payload");
    let reverse_fin_encoded = encoded_frame(FrameKind::Fin, 702, b"");
    let binding = binding(77, 3);
    let first_frame = validate_transit_relay_frame(&first_encoded)?;
    let bound_first =
        crate::peer_egress::transit_binding::BoundTransitRelayFrame::new(binding, first_frame);
    let bound_payload =
        crate::peer_egress::transit_binding::encode_bound_transit_relay_frame(&bound_first);

    source_writer.write_secure_payload(&bound_payload)?;
    let mut source_reader = source_reader;
    let first_bound = read_first_bound_frame(&mut source_reader)?;
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(binding, next_writer)?;

    source_writer.write_secure_payload(&fin_encoded)?;
    next_reader.write_secure_payload(&reverse_encoded)?;
    next_reader.write_secure_payload(&reverse_fin_encoded)?;
    forward_bound_peer_sealed_transit_to_next_hop(
        source_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        Some(dispatcher),
        first_bound,
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
        !forwarded_first.starts_with(&[crate::peer_egress::transit_binding::BOUND_TRANSIT_MAGIC,])
    );
    Ok(())
}

#[test]
fn same_bound_lane_can_forward_second_stream_after_replenishment() -> Result<(), String> {
    let binding = binding(177, 9);
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();

    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 1001, b"first sealed lane payload");
    let fin_encoded = encoded_frame(FrameKind::Fin, 1002, b"");
    let reverse_encoded = encoded_frame(FrameKind::Data, 1101, b"first sealed reverse payload");
    let reverse_fin_encoded = encoded_frame(FrameKind::Fin, 1102, b"");
    let first_frame = validate_transit_relay_frame(&first_encoded)?;
    let bound_first =
        crate::peer_egress::transit_binding::BoundTransitRelayFrame::new(binding, first_frame);
    let bound_payload =
        crate::peer_egress::transit_binding::encode_bound_transit_relay_frame(&bound_first);
    source_writer.write_secure_payload(&bound_payload)?;
    let mut source_reader = source_reader;
    let first_bound = read_first_bound_frame(&mut source_reader)?;
    dispatcher.register(binding, next_writer)?;

    source_writer.write_secure_payload(&fin_encoded)?;
    next_reader.write_secure_payload(&reverse_encoded)?;
    next_reader.write_secure_payload(&reverse_fin_encoded)?;
    forward_bound_peer_sealed_transit_to_next_hop(
        source_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        Some(dispatcher.clone()),
        first_bound,
    )?;
    assert!(!dispatcher.contains_binding(binding)?);
    assert_eq!(next_reader.read_secure_payload()?, first_encoded);
    assert_eq!(next_reader.read_secure_payload()?, fin_encoded);
    assert_eq!(source_writer.read_secure_payload()?, reverse_encoded);
    assert_eq!(source_writer.read_secure_payload()?, reverse_fin_encoded);

    let (mut denied_writer, denied_reader) = test_peer_pair()?;
    let denied_encoded = encoded_frame(FrameKind::Data, 1201, b"second sealed lane payload");
    let denied_frame = validate_transit_relay_frame(&denied_encoded)?;
    let denied_bound =
        crate::peer_egress::transit_binding::BoundTransitRelayFrame::new(binding, denied_frame);
    let denied_payload =
        crate::peer_egress::transit_binding::encode_bound_transit_relay_frame(&denied_bound);
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
    let second_encoded = encoded_frame(FrameKind::Data, 1301, b"replenished sealed lane payload");
    let second_fin_encoded = encoded_frame(FrameKind::Fin, 1302, b"");
    let second_reverse_encoded = encoded_frame(FrameKind::Data, 1401, b"replenished reverse");
    let second_reverse_fin_encoded = encoded_frame(FrameKind::Fin, 1402, b"");
    let second_frame = validate_transit_relay_frame(&second_encoded)?;
    let second_bound =
        crate::peer_egress::transit_binding::BoundTransitRelayFrame::new(binding, second_frame);
    let second_payload =
        crate::peer_egress::transit_binding::encode_bound_transit_relay_frame(&second_bound);
    second_writer.write_secure_payload(&second_payload)?;
    let mut second_reader = second_reader;
    let second_first = read_first_bound_frame(&mut second_reader)?;
    dispatcher.register(binding, second_next_writer)?;

    second_writer.write_secure_payload(&second_fin_encoded)?;
    second_next_reader.write_secure_payload(&second_reverse_encoded)?;
    second_next_reader.write_secure_payload(&second_reverse_fin_encoded)?;
    forward_bound_peer_sealed_transit_to_next_hop(
        second_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        Some(dispatcher.clone()),
        second_first,
    )?;
    assert!(!dispatcher.contains_binding(binding)?);
    assert_eq!(second_next_reader.read_secure_payload()?, second_encoded);
    assert_eq!(
        second_next_reader.read_secure_payload()?,
        second_fin_encoded
    );
    assert_eq!(second_writer.read_secure_payload()?, second_reverse_encoded);
    assert_eq!(
        second_writer.read_secure_payload()?,
        second_reverse_fin_encoded
    );
    assert!(!format!("{bound_first:?}").contains("first sealed lane payload"));
    assert!(!format!("{second_bound:?}").contains("replenished sealed lane payload"));
    Ok(())
}
