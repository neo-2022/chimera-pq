use chimera_session::FrameKind;
use std::thread;
use std::time::Duration;

use super::super::{
    PeerTransitPolicy, forward_peer_sealed_transit_to_next_hop,
    forward_peer_sealed_transit_to_next_hop_with_limits,
};
use super::helpers::{assert_bytes_eq_redacted, encoded_frame, test_peer_pair};
use crate::peer_egress::transit_guard::TransitRelayLimits;
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
    assert_bytes_eq_redacted(&forwarded_first, &first_encoded, "forwarded first frame")?;
    assert_bytes_eq_redacted(&forwarded_fin, &fin_encoded, "forwarded fin frame")?;
    assert_bytes_eq_redacted(&reverse_first, &reverse_encoded, "reverse first frame")?;
    assert_bytes_eq_redacted(&reverse_fin, &reverse_fin_encoded, "reverse fin frame")?;
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
fn peer_sealed_transit_keeps_one_way_flow_alive_across_reverse_idle_timeout() -> Result<(), String>
{
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 211, b"first one way chunk");
    let second_encoded = encoded_frame(FrameKind::Data, 212, b"second one way chunk");
    let fin_encoded = encoded_frame(FrameKind::Fin, 213, b"");
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

    let second_to_write = second_encoded.clone();
    let fin_to_write = fin_encoded.clone();
    let writer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(5));
        source_writer.write_secure_payload(&second_to_write)?;
        thread::sleep(Duration::from_millis(5));
        source_writer.write_secure_payload(&fin_to_write)?;
        Ok::<_, String>(source_writer)
    });

    forward_peer_sealed_transit_to_next_hop_with_limits(
        source_reader,
        PeerTransitPolicy::AllowPoolNextHop,
        Some(pool),
        first_frame,
        TransitRelayLimits::new(10, 1024 * 1024, 25)?,
    )?;
    writer
        .join()
        .map_err(|_| "one-way writer thread panicked".to_string())??;

    assert_bytes_eq_redacted(
        &next_reader.read_secure_payload()?,
        &first_encoded,
        "one-way first frame",
    )?;
    assert_bytes_eq_redacted(
        &next_reader.read_secure_payload()?,
        &second_encoded,
        "one-way second frame",
    )?;
    assert_bytes_eq_redacted(
        &next_reader.read_secure_payload()?,
        &fin_encoded,
        "one-way fin frame",
    )?;
    Ok(())
}

#[test]
fn peer_sealed_transit_full_session_idle_fails_closed() -> Result<(), String> {
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let first_encoded = encoded_frame(FrameKind::Data, 221, b"first then idle");
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

    let error = match forward_peer_sealed_transit_to_next_hop_with_limits(
        source_reader,
        PeerTransitPolicy::AllowPoolNextHop,
        Some(pool),
        first_frame,
        TransitRelayLimits::new(10, 1024 * 1024, 20)?,
    ) {
        Ok(()) => return Err("full sealed transit idle must fail closed".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("idle timeout"), "{error}");
    assert_bytes_eq_redacted(
        &next_reader.read_secure_payload()?,
        &first_encoded,
        "full idle first frame",
    )?;
    assert!(!error.contains("first then idle"));
    Ok(())
}
