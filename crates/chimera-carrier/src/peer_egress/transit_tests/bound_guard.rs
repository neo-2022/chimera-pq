use chimera_session::FrameKind;

use super::super::{
    BoundPeerTransitPolicy, forward_bound_peer_sealed_transit_to_next_hop_with_limits,
};
use super::helpers::{
    assert_bound_payload, binding, bound_payload, read_first_bound_frame, test_peer_pair,
};
use crate::peer_egress::transit_guard::TransitRelayLimits;

#[test]
fn peer_bound_transit_frame_budget_fails_closed_without_payload_leak() -> Result<(), String> {
    let path_binding = binding(901, 7);
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let (first_payload, first_sealed) =
        bound_payload(path_binding, FrameKind::Data, 9011, b"bound frame first")?;
    let (second_payload, _) = bound_payload(
        path_binding,
        FrameKind::Data,
        9012,
        b"SECRET_BOUND_FRAME_BUDGET",
    )?;
    source_writer.write_secure_payload(&first_payload)?;
    source_writer.write_secure_payload(&second_payload)?;
    let mut source_reader = source_reader;
    let first_bound = read_first_bound_frame(&mut source_reader)?;
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(path_binding, next_writer)?;

    let error = match forward_bound_peer_sealed_transit_to_next_hop_with_limits(
        source_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        Some(dispatcher),
        first_bound,
        TransitRelayLimits::new(1, 1024 * 1024, 25)?,
    ) {
        Ok(()) => return Err("bound transit frame budget must fail closed".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("frame budget") || error.contains("read secure frame"));
    assert!(!error.contains("SECRET_BOUND_FRAME_BUDGET"));
    assert_bound_payload(
        &next_reader.read_secure_payload()?,
        path_binding,
        &first_sealed,
    )?;
    next_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set next reader timeout failed: {error}"))?;
    assert!(next_reader.read_secure_payload().is_err());
    Ok(())
}

#[test]
fn peer_bound_transit_byte_budget_fails_closed_before_forwarding_payload() -> Result<(), String> {
    let path_binding = binding(902, 7);
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let (first_payload, _) = bound_payload(
        path_binding,
        FrameKind::Data,
        9021,
        b"SECRET_BOUND_BYTE_BUDGET",
    )?;
    source_writer.write_secure_payload(&first_payload)?;
    let mut source_reader = source_reader;
    let first_bound = read_first_bound_frame(&mut source_reader)?;
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(path_binding, next_writer)?;

    let error = match forward_bound_peer_sealed_transit_to_next_hop_with_limits(
        source_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        Some(dispatcher),
        first_bound,
        TransitRelayLimits::new(10, first_payload.len() as u64 - 1, 25)?,
    ) {
        Ok(()) => return Err("bound transit byte budget must fail closed".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("byte budget"));
    assert!(!error.contains("SECRET_BOUND_BYTE_BUDGET"));
    next_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set next reader timeout failed: {error}"))?;
    assert!(next_reader.read_secure_payload().is_err());
    Ok(())
}

#[test]
fn peer_bound_transit_full_session_idle_fails_closed() -> Result<(), String> {
    let path_binding = binding(903, 7);
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let (first_payload, first_sealed) = bound_payload(
        path_binding,
        FrameKind::Data,
        9031,
        b"bound first then idle",
    )?;
    source_writer.write_secure_payload(&first_payload)?;
    let mut source_reader = source_reader;
    let first_bound = read_first_bound_frame(&mut source_reader)?;
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(path_binding, next_writer)?;

    let error = match forward_bound_peer_sealed_transit_to_next_hop_with_limits(
        source_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        Some(dispatcher),
        first_bound,
        TransitRelayLimits::new(10, 1024 * 1024, 20)?,
    ) {
        Ok(()) => return Err("bound transit idle session must fail closed".to_string()),
        Err(error) => error,
    };

    assert!(
        error.contains("bound transit session idle timeout"),
        "{error}"
    );
    assert!(!error.contains("bound first then idle"));
    assert_bound_payload(
        &next_reader.read_secure_payload()?,
        path_binding,
        &first_sealed,
    )?;
    Ok(())
}
