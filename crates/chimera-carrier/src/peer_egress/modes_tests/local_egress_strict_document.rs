use std::io::Write;
use std::net::Shutdown;
use std::thread;

use crate::peer_egress::lane_binding::{TransitLaneDocument, TransitLaneRegistration};
use crate::peer_egress::modes::handle_local_client_with_lane_document_and_first_byte;
use crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher;
use crate::peer_egress::transit_local::relay_local_sealed_transit_with_lane_document_and_first_byte;

use super::{binding, tcp_pair};

#[test]
fn local_client_rejects_registration_only_lane_document() -> Result<(), String> {
    let (mut local_client, local_server) = tcp_pair()?;
    let document = TransitLaneDocument::new(
        vec![TransitLaneRegistration::new(
            binding(7091, 1),
            "198.51.100.91:443".to_string(),
        )?],
        None,
    );
    let dispatcher = new_shared_transit_dispatcher();
    let worker = thread::spawn(move || {
        handle_local_client_with_lane_document_and_first_byte(
            local_server,
            &document,
            dispatcher,
            crate::peer_egress::options::LOCAL_MAGIC[0],
        )
    });

    local_client
        .write_all(&crate::peer_egress::options::LOCAL_MAGIC[1..])
        .and_then(|_| local_client.write_all(b"CONNECT example.org 443\n"))
        .map_err(|error| format!("write local connect failed: {error}"))?;
    local_client
        .shutdown(Shutdown::Write)
        .map_err(|error| format!("shutdown local writer failed: {error}"))?;

    let error = match worker
        .join()
        .map_err(|_| "local document worker panicked".to_string())?
    {
        Ok(()) => return Err("registration-only local document must fail closed".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("mesh plan snapshot"));
    Ok(())
}

#[test]
fn local_sealed_transit_rejects_registration_only_lane_document() -> Result<(), String> {
    let (_local_client, local_server) = tcp_pair()?;
    let document = TransitLaneDocument::new(
        vec![TransitLaneRegistration::new(
            binding(7092, 1),
            "198.51.100.92:443".to_string(),
        )?],
        None,
    );
    let error = match relay_local_sealed_transit_with_lane_document_and_first_byte(
        local_server,
        &document,
        Some(new_shared_transit_dispatcher()),
        chimera_session::FRAME_VERSION,
    ) {
        Ok(()) => return Err("registration-only local transit must fail closed".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("mesh plan snapshot"));
    Ok(())
}
