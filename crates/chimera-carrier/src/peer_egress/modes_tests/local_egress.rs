use crate::peer_egress::lane_binding::TransitLaneRegistration;
use crate::peer_egress::modes::{
    handle_local_client_with_lane_document_and_first_byte,
    handle_local_client_with_registrations_and_first_byte,
};
use crate::peer_egress::options::AeadSuite;
use crate::peer_egress::pool::new_shared_pool;
use crate::peer_egress::protocol::read_line_limited;
use crate::peer_egress::transit::{PeerTransitPolicy, relay_local_sealed_transit_to_next_hop};
use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};
use crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher;
use chimera_session::{Frame, FrameKind};
use std::io::Write;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;

#[path = "local_egress_strict_document.rs"]
mod strict_document_tests;

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

#[test]
fn local_client_uses_document_backed_lane_selection_when_plan_snapshot_exists() -> Result<(), String>
{
    let (mut local_client, local_server) = tcp_pair()?;
    let (selected_peer_writer, mut selected_peer_reader) = test_peer_pair()?;
    let document = {
        let mut runtime = chimera_mesh::MeshRuntime::bootstrap("cef-public", "seed-a")?;
        runtime.merge_discovery(
            "seed-b",
            &[
                chimera_mesh::MeshDiscoveryRecord {
                    node_id: "node-a".to_string(),
                    endpoint: "198.51.100.31:443".to_string(),
                    region: "eu".to_string(),
                    load_score: 20,
                    reliability_score: 90,
                },
                chimera_mesh::MeshDiscoveryRecord {
                    node_id: "node-b".to_string(),
                    endpoint: "198.51.100.32:443".to_string(),
                    region: "eu".to_string(),
                    load_score: 22,
                    reliability_score: 91,
                },
            ],
        )?;
        let plan = runtime.plan_path_from_dps_payload(
            &chimera_mesh::MeshJoinRequest {
                namespace: "cef-public".to_string(),
                node_name: "node-client".to_string(),
                invite_token: None,
            },
            concat!(
                "mesh_allowed_regions=eu;",
                "mesh_max_peers=2;",
                "mesh_max_selected_per_region=2;",
                "mesh_multipath_mode=flow_shard;",
                "mesh_route_binding_id=7004"
            ),
        )?;
        crate::peer_egress::lane_binding::transit_lane_document_from_mesh_plan(&plan)?
    };
    let rendered = crate::peer_egress::lane_binding::render_transit_lane_document(&document)?;
    let mut rewritten = String::new();
    let mut data_line_index = 0usize;
    for line in rendered.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            rewritten.push_str(line);
            rewritten.push('\n');
            continue;
        }
        data_line_index += 1;
        let parts: Vec<&str> = trimmed.split(',').map(str::trim).collect();
        if parts.len() != 6 {
            return Err("document-backed test requires plan-snapshot lane rows".to_string());
        }
        let role = if data_line_index == 2 {
            "standby"
        } else {
            parts[3]
        };
        rewritten.push_str(&format!(
            "{},{},{},{},{},{}\n",
            parts[0], parts[1], parts[2], role, parts[4], parts[5]
        ));
    }
    if data_line_index < 2 {
        return Err("document-backed test requires at least two lane rows".to_string());
    }
    let document = crate::peer_egress::lane_binding::parse_transit_lane_document(&rewritten)?;
    local_client
        .set_read_timeout(Some(std::time::Duration::from_millis(250)))
        .map_err(|error| format!("set local timeout failed: {error}"))?;
    let plan = document
        .mesh_path_plan()?
        .ok_or_else(|| "plan snapshot missing".to_string())?;
    let plan_selection =
        crate::peer_egress::live_lane_selection::select_carrier_lane_from_mesh_plan(
            &plan,
            chimera_mesh::MeshMultipathFlowKey::from_opaque_flow_bytes(b"plan-binding-check")?,
        );
    let plan_binding = plan_selection
        .selected_binding
        .ok_or_else(|| "planned binding missing".to_string())?;

    let mut chosen_destination = None;
    let fallback_bindings = document.registrations();
    for index in 0..256usize {
        let destination_host = format!("document-backed-{index}.example.org");
        let destination = format!("{destination_host}:443");
        let key =
            chimera_mesh::MeshMultipathFlowKey::from_opaque_flow_bytes(destination.as_bytes())?;
        let plan_selection =
            crate::peer_egress::live_lane_selection::select_carrier_lane_from_mesh_plan(&plan, key);
        let fallback_selection =
            crate::peer_egress::live_lane_selection::select_carrier_lane_from_registrations(
                fallback_bindings,
                key,
            )?;
        if plan_selection.selected_binding == Some(plan_binding)
            && fallback_selection.selected_binding != Some(plan_binding)
        {
            chosen_destination = Some((destination_host, fallback_selection.selected_binding));
            break;
        }
    }
    let (destination_host, fallback_binding) = chosen_destination.ok_or_else(|| {
        "could not find destination matching plan and diverging from fallback".to_string()
    })?;
    assert_ne!(fallback_binding, Some(plan_binding));

    let dispatcher = new_shared_transit_dispatcher();
    dispatcher.register(plan_binding, selected_peer_writer)?;
    selected_peer_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(250)))
        .map_err(|error| format!("set selected timeout failed: {error}"))?;
    let peer_worker = thread::spawn(move || -> Result<(), String> {
        match crate::peer_egress::wire::read_peer_message(&mut selected_peer_reader, 512)? {
            crate::peer_egress::wire::PeerMessage::Connect(_) => {
                crate::peer_egress::wire::write_ack_ok(&mut selected_peer_reader)?;
                selected_peer_reader
                    .stream
                    .shutdown(Shutdown::Write)
                    .map_err(|error| format!("shutdown peer writer failed: {error}"))?;
                Ok(())
            }
            other => Err(format!("unexpected peer message: {other:?}")),
        }
    });
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
        .and_then(|_| {
            local_client.write_all(format!("CONNECT {destination_host} 443\n").as_bytes())
        })
        .map_err(|error| format!("write local connect failed: {error}"))?;
    local_client
        .shutdown(Shutdown::Write)
        .map_err(|error| format!("shutdown local writer failed: {error}"))?;

    let forwarded = read_line_limited(&mut local_client, 16)?;
    assert_eq!(forwarded, "OK");
    worker
        .join()
        .map_err(|_| "local ingress worker panicked".to_string())??;
    peer_worker
        .join()
        .map_err(|_| "peer worker panicked".to_string())??;
    Ok(())
}

#[test]
fn local_sealed_transit_fails_closed_when_pool_transit_denied() -> Result<(), String> {
    let (mut local_client, local_server) = tcp_pair()?;
    let (peer_writer, _peer_reader) = test_peer_pair()?;
    let pool = crate::peer_egress::pool::new_shared_pool();
    let _ = pool.push(peer_writer);

    let worker = thread::spawn(move || {
        relay_local_sealed_transit_to_next_hop(
            local_server,
            PeerTransitPolicy::DenyPoolNextHop,
            pool,
            chimera_session::FRAME_VERSION,
        )
    });

    let frame = chimera_session::Frame {
        kind: chimera_session::FrameKind::Data,
        packet_number: 7,
        payload: vec![b'P'; 32],
    }
    .encode()
    .map_err(|error| format!("encode transit frame failed: {error}"))?;
    local_client
        .write_all(&frame[1..])
        .map_err(|error| format!("write transit frame failed: {error}"))?;
    local_client
        .shutdown(Shutdown::Write)
        .map_err(|error| format!("shutdown local writer failed: {error}"))?;

    let error = match worker
        .join()
        .map_err(|_| "local sealed transit worker panicked".to_string())?
    {
        Ok(()) => return Err("pool-denied sealed transit must fail closed".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("denied by policy"));
    Ok(())
}

#[test]
fn local_sealed_transit_uses_flow_key_to_select_pool_next_hop() -> Result<(), String> {
    let (mut local_client, local_server) = tcp_pair()?;
    let (first_peer_writer, mut first_peer_reader) = test_peer_pair()?;
    let (second_peer_writer, mut second_peer_reader) = test_peer_pair()?;
    let pool = new_shared_pool();
    pool.push(first_peer_writer)?;
    pool.push(second_peer_writer)?;
    first_peer_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set first peer timeout failed: {error}"))?;
    second_peer_reader
        .stream
        .set_read_timeout(Some(std::time::Duration::from_millis(50)))
        .map_err(|error| format!("set second peer timeout failed: {error}"))?;

    let frame = Frame {
        kind: FrameKind::Data,
        packet_number: 17,
        payload: b"local opaque transit payload".to_vec(),
    }
    .encode()
    .map_err(|error| format!("encode transit frame failed: {error}"))?;
    let flow_key = chimera_mesh::MeshMultipathFlowKey::from_opaque_flow_bytes(&frame)?;
    let expected_slot = flow_key.select_slot_index(2)?;

    let worker = thread::spawn(move || {
        relay_local_sealed_transit_to_next_hop(
            local_server,
            PeerTransitPolicy::AllowPoolNextHop,
            pool,
            chimera_session::FRAME_VERSION,
        )
    });

    local_client
        .write_all(&frame[1..])
        .map_err(|error| format!("write local transit frame failed: {error}"))?;
    local_client
        .shutdown(Shutdown::Write)
        .map_err(|error| format!("shutdown local writer failed: {error}"))?;

    match expected_slot {
        0 => {
            assert_eq!(first_peer_reader.read_secure_payload()?, frame);
            assert!(second_peer_reader.read_secure_payload().is_err());
        }
        1 => {
            assert!(first_peer_reader.read_secure_payload().is_err());
            assert_eq!(second_peer_reader.read_secure_payload()?, frame);
        }
        _ => return Err("unexpected slot index".to_string()),
    }

    worker
        .join()
        .map_err(|_| "local sealed transit worker panicked".to_string())??;
    Ok(())
}
