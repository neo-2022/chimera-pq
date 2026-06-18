use chimera_session::FrameKind;

use super::super::{BoundPeerTransitPolicy, forward_bound_peer_sealed_transit_to_next_hop};
use super::helpers::{
    assert_bound_payload, binding, bound_payload, read_first_bound_frame, test_peer_pair,
};

#[test]
fn bound_peer_transit_rejects_reverse_binding_change() -> Result<(), String> {
    let path_binding = binding(194, 1);
    let changed_binding = binding(194, 2);
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let (source_fin_payload, source_fin_sealed) =
        bound_payload(path_binding, FrameKind::Fin, 1901, b"")?;
    let (reverse_payload, reverse_sealed) = bound_payload(
        path_binding,
        FrameKind::Data,
        1902,
        b"reverse bound first payload",
    )?;
    let (changed_payload, _) = bound_payload(
        changed_binding,
        FrameKind::Data,
        1903,
        b"REVERSE_CHANGED_PAYLOAD_MARKER",
    )?;

    source_writer.write_secure_payload(&source_fin_payload)?;
    let mut source_reader = source_reader;
    let first_bound = read_first_bound_frame(&mut source_reader)?;
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(path_binding, next_writer)?;
    next_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set next timeout failed: {error}"))?;
    source_writer
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set source timeout failed: {error}"))?;

    next_reader.write_secure_payload(&reverse_payload)?;
    next_reader.write_secure_payload(&changed_payload)?;

    let error = match forward_bound_peer_sealed_transit_to_next_hop(
        source_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        Some(dispatcher),
        first_bound,
    ) {
        Ok(()) => return Err("reverse bound transit binding change must fail".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("binding changed"));
    assert!(!error.contains("REVERSE_CHANGED_PAYLOAD_MARKER"));
    assert_bound_payload(
        &next_reader.read_secure_payload()?,
        path_binding,
        &source_fin_sealed,
    )?;
    assert_bound_payload(
        &source_writer.read_secure_payload()?,
        path_binding,
        &reverse_sealed,
    )?;
    let changed_result = source_writer.read_secure_payload();
    assert!(changed_result.is_err());
    assert!(!format!("{changed_result:?}").contains("REVERSE_CHANGED_PAYLOAD_MARKER"));
    Ok(())
}

#[test]
fn bound_peer_transit_rejects_reverse_unbound_frame() -> Result<(), String> {
    let path_binding = binding(195, 1);
    let (mut source_writer, source_reader) = test_peer_pair()?;
    let (next_writer, mut next_reader) = test_peer_pair()?;
    let (source_fin_payload, source_fin_sealed) =
        bound_payload(path_binding, FrameKind::Fin, 2001, b"")?;
    let unbound =
        super::helpers::encoded_frame(FrameKind::Data, 2002, b"REVERSE_UNBOUND_PAYLOAD_MARKER");

    source_writer.write_secure_payload(&source_fin_payload)?;
    let mut source_reader = source_reader;
    let first_bound = read_first_bound_frame(&mut source_reader)?;
    let dispatcher = crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher();
    dispatcher.register(path_binding, next_writer)?;
    next_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set next timeout failed: {error}"))?;
    source_writer
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set source timeout failed: {error}"))?;

    next_reader.write_secure_payload(&unbound)?;

    let error = match forward_bound_peer_sealed_transit_to_next_hop(
        source_reader,
        BoundPeerTransitPolicy::AllowBoundNextHop,
        Some(dispatcher),
        first_bound,
    ) {
        Ok(()) => return Err("reverse bound transit must reject unbound frame".to_string()),
        Err(error) => error,
    };

    assert!(error.contains("unbound transit frame"));
    assert!(!error.contains("REVERSE_UNBOUND_PAYLOAD_MARKER"));
    assert_bound_payload(
        &next_reader.read_secure_payload()?,
        path_binding,
        &source_fin_sealed,
    )?;
    assert!(source_writer.read_secure_payload().is_err());
    Ok(())
}
