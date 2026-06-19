use crate::peer_egress::lane_binding::TransitLaneRegistration;
use crate::peer_egress::modes::handle_local_client_with_registrations_and_first_byte;
use crate::peer_egress::options::AeadSuite;
use crate::peer_egress::protocol::read_line_limited;
use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};
use crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher;
use std::io::Write;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;

fn binding(route: u64, lane: u16) -> TransitPathBinding {
    TransitPathBinding::new(
        TransitRouteId::new(route).unwrap_or_else(|e| unreachable!("{e}")),
        TransitLaneId::new(lane).unwrap_or_else(|e| unreachable!("{e}")),
    )
}

fn tcp_pair() -> Result<(TcpStream, TcpStream), String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("bind test listener failed: {error}"))?;
    let addr = listener
        .local_addr()
        .map_err(|error| format!("read listener addr failed: {error}"))?;
    let client =
        TcpStream::connect(addr).map_err(|error| format!("connect test client failed: {error}"))?;
    let (server, _) = listener
        .accept()
        .map_err(|error| format!("accept test server failed: {error}"))?;
    Ok((client, server))
}

fn test_peer_pair() -> Result<
    (
        crate::peer_egress::protocol::SecurePeerStream,
        crate::peer_egress::protocol::SecurePeerStream,
    ),
    String,
> {
    let transcript = chimera_crypto::TranscriptHash::from_messages(&[b"local-egress-test"]);
    let secrets = chimera_crypto::derive_traffic_secrets(
        chimera_crypto::SuiteId(AeadSuite::Chacha20Poly1305.suite_id()),
        &transcript,
        &[23_u8; 32],
    )
    .map_err(|error| format!("derive test secrets failed: {error}"))?;
    let (left, right) = tcp_pair()?;
    Ok((
        crate::peer_egress::protocol::SecurePeerStream {
            stream: left,
            send_secret: secrets.initiator_to_responder().clone(),
            recv_secret: secrets.responder_to_initiator().clone(),
            send_packet: 0,
            recv_packet: 0,
            aead: AeadSuite::Chacha20Poly1305,
        },
        crate::peer_egress::protocol::SecurePeerStream {
            stream: right,
            send_secret: secrets.responder_to_initiator().clone(),
            recv_secret: secrets.initiator_to_responder().clone(),
            send_packet: 0,
            recv_packet: 0,
            aead: AeadSuite::Chacha20Poly1305,
        },
    ))
}

#[test]
fn local_client_uses_binding_backed_dispatcher_when_registrations_exist() -> Result<(), String> {
    let (mut local_client, local_server) = tcp_pair()?;
    let (selected_peer_writer, mut selected_peer_reader) = test_peer_pair()?;
    let registrations = vec![TransitLaneRegistration::new(
        binding(301, 4),
        "198.51.100.44:443".to_string(),
    )?];
    let dispatcher = new_shared_transit_dispatcher();
    dispatcher.register(binding(301, 4), selected_peer_writer)?;

    local_client
        .set_read_timeout(Some(std::time::Duration::from_millis(250)))
        .map_err(|error| format!("set local timeout failed: {error}"))?;

    let (payload_tx, payload_rx) = mpsc::channel();
    let peer_worker = thread::spawn(move || -> Result<(), String> {
        let forwarded = selected_peer_reader.read_secure_payload()?;
        payload_tx
            .send(forwarded)
            .map_err(|_| "send forwarded payload failed".to_string())?;
        selected_peer_reader.write_secure_payload(b"OK\n")?;
        selected_peer_reader
            .stream
            .shutdown(Shutdown::Write)
            .map_err(|error| format!("shutdown peer writer failed: {error}"))?;
        Ok(())
    });
    let worker = thread::spawn(move || {
        handle_local_client_with_registrations_and_first_byte(
            local_server,
            &registrations,
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

    let forwarded = payload_rx
        .recv_timeout(std::time::Duration::from_millis(250))
        .map_err(|error| format!("receive forwarded payload failed: {error}"))?;
    assert_eq!(forwarded, b"CONNECT example.org 443\n");

    let ack = read_line_limited(&mut local_client, 16)?;
    assert_eq!(ack, "OK");

    peer_worker
        .join()
        .map_err(|_| "peer worker panicked".to_string())??;
    worker
        .join()
        .map_err(|_| "local ingress worker panicked".to_string())??;
    Ok(())
}

#[test]
fn local_client_fails_closed_when_reload_selects_new_binding() -> Result<(), String> {
    let (mut local_client, local_server) = tcp_pair()?;
    let (old_peer_writer, mut old_peer_reader) = test_peer_pair()?;
    let registrations = vec![TransitLaneRegistration::new(
        binding(302, 5),
        "198.51.100.45:443".to_string(),
    )?];
    let dispatcher = new_shared_transit_dispatcher();
    dispatcher.register(binding(301, 4), old_peer_writer)?;

    local_client
        .set_read_timeout(Some(std::time::Duration::from_millis(250)))
        .map_err(|error| format!("set local timeout failed: {error}"))?;
    old_peer_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(250)))
        .map_err(|error| format!("set old peer timeout failed: {error}"))?;

    let worker = thread::spawn(move || {
        handle_local_client_with_registrations_and_first_byte(
            local_server,
            &registrations,
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
        .map_err(|_| "local ingress worker panicked".to_string())?
    {
        Ok(()) => return Err("reload mismatch must fail closed".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("binding unavailable"));
    assert!(old_peer_reader.read_secure_payload().is_err());
    Ok(())
}
