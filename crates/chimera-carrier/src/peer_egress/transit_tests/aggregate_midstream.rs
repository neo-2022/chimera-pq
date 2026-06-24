use chimera_session::FrameKind;

use super::super::{
    BoundPeerTransitPolicy, PeerTransitPolicy, forward_bound_peer_sealed_transit_to_next_hop,
    forward_peer_sealed_transit_to_next_hop,
};
use super::helpers::{
    assert_bound_payload, assert_bytes_eq_redacted, binding, bound_payload, encoded_frame,
    read_first_bound_frame, test_peer_pair,
};
use crate::peer_egress::aggregate_wire::{AggregateObjectId, AggregateTransitShardFrame};
use crate::peer_egress::wire::{
    PeerMessage, read_peer_message, write_aggregate_sealed_transit_message,
};

#[test]
fn peer_sealed_transit_rejects_midstream_aggregate_frame() -> Result<(), String> {
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 1001, b"closed transit payload");
    source_writer.write_secure_payload(&first_encoded)?;
    let mut source_reader = source_reader;
    let first_frame = match read_peer_message(
        &mut source_reader,
        crate::peer_egress::options::SECURE_PLAINTEXT_CHUNK_LEN,
    )? {
        PeerMessage::SealedTransit(frame) => frame,
        other => return Err(format!("unexpected first message: {other:?}")),
    };
    let aggregate = AggregateTransitShardFrame::new(
        binding(193, 5),
        AggregateObjectId::new(901)?,
        32,
        1,
        0,
        0,
        b"AGGREGATE_STREAM_MARKER".to_vec(),
    )?;
    write_aggregate_sealed_transit_message(&mut source_writer, &aggregate)?;
    let pool = crate::peer_egress::pool::new_shared_pool();
    pool.push(next_writer)?;

    let error = match forward_peer_sealed_transit_to_next_hop(
        source_reader,
        PeerTransitPolicy::AllowPoolNextHop,
        Some(pool),
        first_frame,
    ) {
        Ok(()) => return Err("midstream aggregate transit frame must fail".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("aggregate transit frame"));
    assert!(!error.contains("AGGREGATE_STREAM_MARKER"));
    assert_bytes_eq_redacted(
        &next_reader.read_secure_payload()?,
        &first_encoded,
        "aggregate midstream first frame",
    )?;
    assert!(next_reader.read_secure_payload().is_err());
    Ok(())
}

#[test]
fn bound_peer_transit_rejects_midstream_aggregate_frame() -> Result<(), String> {
    let binding = binding(194, 1);
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let (first_payload, first_sealed) =
        bound_payload(binding, FrameKind::Data, 1603, b"peer bound first payload")?;
    let aggregate = AggregateTransitShardFrame::new(
        binding,
        AggregateObjectId::new(902)?,
        32,
        1,
        0,
        0,
        b"BOUND_AGGREGATE_STREAM_MARKER".to_vec(),
    )?;

    source_writer.write_secure_payload(&first_payload)?;
    let mut source_reader = source_reader;
    let first_bound = read_first_bound_frame(&mut source_reader)?;
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(binding, next_writer)?;
    write_aggregate_sealed_transit_message(&mut source_writer, &aggregate)?;

    let error = match forward_bound_peer_sealed_transit_to_next_hop(
        source_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        Some(dispatcher),
        first_bound,
    ) {
        Ok(()) => {
            return Err("peer bound transit must reject aggregate midstream frame".to_string());
        }
        Err(error) => error,
    };

    assert!(error.contains("aggregate transit frame"));
    assert!(!error.contains("BOUND_AGGREGATE_STREAM_MARKER"));
    assert_bound_payload(&next_reader.read_secure_payload()?, binding, &first_sealed)?;
    assert!(next_reader.read_secure_payload().is_err());
    Ok(())
}
