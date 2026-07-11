use super::{
    active_fallback_bindings, handle_local_client_with_lane_document_and_first_byte,
    handle_local_client_with_peer_pool_and_first_byte,
};
use crate::peer_egress::lane_binding::TransitLaneDocument;
use crate::peer_egress::live_lane_selection::select_carrier_binding_from_multipath_schedule;
use crate::peer_egress::options::{AeadSuite, LOCAL_MAGIC};
use crate::peer_egress::pool::new_shared_pool;
use crate::peer_egress::protocol::{Destination, SecurePeerStream};
use crate::peer_egress::transit_dispatch::new_shared_transit_dispatcher;
use chimera_mesh::{
    MeshCarrierLaneBinding, MeshJoinMode, MeshMultipathLane, MeshMultipathLaneRole,
    MeshMultipathMode, MeshMultipathSchedule, MeshMultipathFlowKey, MeshPathPlan, MeshRouteBindingId,
};
use std::io::Write;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

fn tcp_pair() -> Result<(TcpStream, TcpStream), String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("bind test listener failed: {error}"))?;
    let addr = listener
        .local_addr()
        .map_err(|error| format!("read test listener addr failed: {error}"))?;
    let client =
        TcpStream::connect(addr).map_err(|error| format!("connect test client failed: {error}"))?;
    let (server, _) = listener
        .accept()
        .map_err(|error| format!("accept test server failed: {error}"))?;
    Ok((client, server))
}

fn test_peer_pair() -> Result<(SecurePeerStream, SecurePeerStream), String> {
    let transcript = chimera_crypto::TranscriptHash::from_messages(&[b"modes-local-ingress-test"]);
    let secrets = chimera_crypto::derive_traffic_secrets(
        chimera_crypto::SuiteId(AeadSuite::Chacha20Poly1305.suite_id()),
        &transcript,
        &[29_u8; 32],
    )
    .map_err(|error| format!("derive test secrets failed: {error}"))?;
    let (left, right) = tcp_pair()?;
    Ok((
        SecurePeerStream::new(left, secrets.initiator_to_responder().clone(), secrets.responder_to_initiator().clone(), AeadSuite::Chacha20Poly1305),
        SecurePeerStream::new(right, secrets.responder_to_initiator().clone(), secrets.initiator_to_responder().clone(), AeadSuite::Chacha20Poly1305),
    ))
}

fn dead_peer_stream() -> Result<SecurePeerStream, String> {
    let (left, right) = test_peer_pair()?;
    drop(right);
    Ok(left)
}

fn mesh_route_id() -> MeshRouteBindingId {
    MeshRouteBindingId::new(401).unwrap_or_else(|error| unreachable!("{error}"))
}

fn active_lane(lane_id: usize, weight_pct: u8, capacity_weight_pct: u8) -> MeshCarrierLaneBinding {
    let suffix = lane_id + 1;
    MeshCarrierLaneBinding {
        route_binding_id: mesh_route_id(),
        lane_id,
        peer_node_id: format!("peer-{suffix}"),
        carrier_endpoint: format!("192.0.2.{suffix}:443"),
        role: MeshMultipathLaneRole::Active,
        weight_pct,
        capacity_weight_pct,
    }
}

fn mesh_multipath_lane(lane_id: usize) -> MeshMultipathLane {
    MeshMultipathLane {
        lane_id,
        peer_node_id: format!("peer-{}", lane_id + 1),
        role: MeshMultipathLaneRole::Active,
        weight_pct: 40,
        capacity_weight_pct: 40,
    }
}

fn make_lane_document(active_lane_count: usize) -> Result<TransitLaneDocument, String> {
    if active_lane_count == 0 {
        return Err("test schedule must have at least one active lane".to_string());
    }
    let route_id = mesh_route_id();
    let lane_weight_pct = (100 / active_lane_count).min(40) as u8;
    let lane_capacity_weight_pct = lane_weight_pct;
    let active_lane_with_pct = |lane_id: usize| active_lane(lane_id, lane_weight_pct, lane_capacity_weight_pct);
    let carrier_lane_bindings: Vec<MeshCarrierLaneBinding> = (0..active_lane_count)
        .map(active_lane_with_pct)
        .collect();
    let lanes: Vec<MeshMultipathLane> = (0..active_lane_count).map(mesh_multipath_lane).collect();
    let total_capacity_weight_pct = (lane_weight_pct as u16) * (active_lane_count as u16);
    let transit_capacity_budget_pct = total_capacity_weight_pct;
    let local_traffic_reserve_pct = 100_u8.saturating_sub(transit_capacity_budget_pct as u8);

    let schedule = MeshMultipathSchedule {
        mode: MeshMultipathMode::FlowShard,
        route_binding_id: Some(route_id),
        lanes,
        carrier_lane_bindings,
        active_lane_count,
        standby_lane_count: 0,
        lane_admission_requested_active_lane_count: active_lane_count,
        lane_admission_admitted_active_lane_count: active_lane_count,
        lane_admission_rejected_active_lane_count: 0,
        lane_admission_capacity_status: "ok".to_string(),
        active_weight_sum_pct: total_capacity_weight_pct,
        active_capacity_sum_pct: total_capacity_weight_pct,
        local_traffic_reserve_pct,
        transit_capacity_budget_pct: total_capacity_weight_pct as u8,
        demand_policy: "none".to_string(),
        demand_policy_source: "none".to_string(),
        demand_requested_active_lane_count: active_lane_count,
        demand_planned_active_lane_count: active_lane_count,
        demand_admitted_lane_capacity_pct: total_capacity_weight_pct as u8,
        demand_unmet_lane_count: 0,
        demand_status: "ok".to_string(),
        demand_rebuild_recommended: false,
        fairness_policy: "weighted_round_robin_v1".to_string(),
        execution_status: "ok".to_string(),
        transit_payload_policy: "sealed_opaque_only".to_string(),
        planner_rebuild_reason: "none".to_string(),
    };
    let plan = MeshPathPlan {
        namespace: "test-stabilization-1A".to_string(),
        join_mode: MeshJoinMode::InvitationOnly,
        selected_peers: Vec::new(),
        multipath_schedule: schedule,
        explain: Vec::new(),
    };
    Ok(TransitLaneDocument::new(
        crate::peer_egress::lane_binding::transit_lane_registrations_from_mesh_plan(&plan)?,
        Some(crate::peer_egress::lane_binding::TransitLanePlanSnapshot::new(plan)),
    ))
}

#[test]
fn lane_document_retries_same_binding_when_fresh_peer_arrives() -> Result<(), String> {
    // Use the default handshake deadline; the same-lane retry succeeds well before
    // it expires because a fresh peer is injected after a short delay.
    let (mut local_client, local_server) = tcp_pair()?;
    let document = make_lane_document(1)?;
    let plan = document
        .require_mesh_path_plan_ref()
        .map_err(|error| format!("plan missing: {error}"))?;
    let destination = Destination {
        host: "same-lane-retry.example.org".to_string(),
        port: 443,
    };
    let flow_key =
        MeshMultipathFlowKey::from_opaque_flow_bytes(destination.connect_addr().as_bytes())?;
    let initial_binding = select_carrier_binding_from_multipath_schedule(&plan.multipath_schedule, flow_key)
        .map_err(|reason| format!("lane selection failed: {reason}"))?;

    let dispatcher = new_shared_transit_dispatcher();
    dispatcher.register(initial_binding, dead_peer_stream()?)?;

    let dispatcher_for_worker = dispatcher.clone();
    let document_for_worker = document.clone();
    let (live_peer, mut live_remote) = test_peer_pair()?;
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(80));
        dispatcher_for_worker.register(initial_binding, live_peer)
    });

    let ack_worker = thread::spawn(move || -> Result<(), String> {
        let forwarded = live_remote.read_secure_payload()?;
        assert!(forwarded.starts_with(b"CONNECT "));
        live_remote.write_line("OK")?;
        Ok(())
    });

    let worker = thread::spawn(move || {
        handle_local_client_with_lane_document_and_first_byte(
            local_server,
            &document_for_worker,
            dispatcher,
            LOCAL_MAGIC[0],
        )
    });

    local_client
        .set_read_timeout(Some(Duration::from_millis(400)))
        .map_err(|error| format!("set local timeout failed: {error}"))?;
    local_client
        .write_all(&LOCAL_MAGIC[1..])
        .and_then(|_| local_client.write_all(b"CONNECT same-lane-retry.example.org 443\n"))
        .map_err(|error| format!("write local connect failed: {error}"))?;
    local_client
        .shutdown(Shutdown::Write)
        .map_err(|error| format!("shutdown local writer failed: {error}"))?;

    let ack = crate::peer_egress::protocol::read_line_limited(&mut local_client, 16)?;
    assert_eq!(ack, "OK");

    worker
        .join()
        .map_err(|_| "local ingress worker panicked".to_string())??;
    ack_worker
        .join()
        .map_err(|_| "ack worker panicked".to_string())??;
    Ok(())
}

#[test]
fn lane_document_fallbacks_to_other_active_lane_when_first_peer_is_dead() -> Result<(), String> {
    // Use the default handshake deadline; fallback succeeds well before it expires.
    let (mut local_client, local_server) = tcp_pair()?;
    let document = make_lane_document(2)?;
    let plan = document
        .require_mesh_path_plan_ref()
        .map_err(|error| format!("plan missing: {error}"))?;
    let destination = Destination {
        host: "lane-fallback.example.org".to_string(),
        port: 443,
    };
    let flow_key =
        MeshMultipathFlowKey::from_opaque_flow_bytes(destination.connect_addr().as_bytes())?;
    let initial_binding = select_carrier_binding_from_multipath_schedule(&plan.multipath_schedule, flow_key)
        .map_err(|reason| format!("lane selection failed: {reason}"))?;
    let fallback_bindings =
        active_fallback_bindings(&plan.multipath_schedule, flow_key, initial_binding)?;
    assert!(
        !fallback_bindings.is_empty(),
        "test setup must provide at least one fallback lane"
    );
    let fallback_binding = fallback_bindings[0];
    assert_ne!(initial_binding, fallback_binding);

    let dispatcher = new_shared_transit_dispatcher();
    dispatcher.register(initial_binding, dead_peer_stream()?)?;

    let dispatcher_for_worker = dispatcher.clone();
    let document_for_worker = document.clone();
    let (live_peer, mut live_remote) = test_peer_pair()?;
    dispatcher.register(fallback_binding, live_peer)?;

    let ack_worker = thread::spawn(move || -> Result<(), String> {
        let forwarded = live_remote.read_secure_payload()?;
        assert!(forwarded.starts_with(b"CONNECT "));
        live_remote.write_line("OK")?;
        Ok(())
    });

    let worker = thread::spawn(move || {
        handle_local_client_with_lane_document_and_first_byte(
            local_server,
            &document_for_worker,
            dispatcher_for_worker,
            LOCAL_MAGIC[0],
        )
    });

    local_client
        .set_read_timeout(Some(Duration::from_millis(400)))
        .map_err(|error| format!("set local timeout failed: {error}"))?;
    local_client
        .write_all(&LOCAL_MAGIC[1..])
        .and_then(|_| local_client.write_all(b"CONNECT lane-fallback.example.org 443\n"))
        .map_err(|error| format!("write local connect failed: {error}"))?;
    local_client
        .shutdown(Shutdown::Write)
        .map_err(|error| format!("shutdown local writer failed: {error}"))?;

    let ack = crate::peer_egress::protocol::read_line_limited(&mut local_client, 16)?;
    assert_eq!(ack, "OK");

    worker
        .join()
        .map_err(|_| "local ingress worker panicked".to_string())??;
    ack_worker
        .join()
        .map_err(|_| "ack worker panicked".to_string())??;
    Ok(())
}

#[test]
fn peer_pool_discards_dead_peer_and_does_not_retry_same_stream() -> Result<(), String> {
    // Use the default handshake deadline; the pool path succeeds as soon as a
    // live peer is injected after the dead peer is discarded.
    let (mut local_client, local_server) = tcp_pair()?;
    let pool = new_shared_pool();
    pool.push(dead_peer_stream()?)?;

    let pool_for_register = pool.clone();
    let (live_peer, mut live_remote) = test_peer_pair()?;
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(80));
        pool_for_register.push(live_peer)
    });

    let ack_worker = thread::spawn(move || -> Result<(), String> {
        let forwarded = live_remote.read_secure_payload()?;
        assert!(forwarded.starts_with(b"CONNECT "));
        live_remote.write_line("OK")?;
        Ok(())
    });

    let worker = thread::spawn(move || {
        handle_local_client_with_peer_pool_and_first_byte(
            local_server,
            pool,
            LOCAL_MAGIC[0],
        )
    });

    local_client
        .set_read_timeout(Some(Duration::from_millis(400)))
        .map_err(|error| format!("set local timeout failed: {error}"))?;
    local_client
        .write_all(&LOCAL_MAGIC[1..])
        .and_then(|_| local_client.write_all(b"CONNECT pool-fallback.example.org 443\n"))
        .map_err(|error| format!("write local connect failed: {error}"))?;
    local_client
        .shutdown(Shutdown::Write)
        .map_err(|error| format!("shutdown local writer failed: {error}"))?;

    let ack = crate::peer_egress::protocol::read_line_limited(&mut local_client, 16)?;
    assert_eq!(ack, "OK");

    worker
        .join()
        .map_err(|_| "local ingress worker panicked".to_string())??;
    ack_worker
        .join()
        .map_err(|_| "ack worker panicked".to_string())??;
    Ok(())
}


#[test]
fn lane_document_repaths_through_all_admitted_bindings_until_live_peer() -> Result<(), String> {
    let (mut local_client, local_server) = tcp_pair()?;
    let document = make_lane_document(3)?;
    let plan = document
        .require_mesh_path_plan_ref()
        .map_err(|error| format!("plan missing: {error}"))?;
    let destination = Destination {
        host: "lane-repath.example.org".to_string(),
        port: 443,
    };
    let flow_key =
        MeshMultipathFlowKey::from_opaque_flow_bytes(destination.connect_addr().as_bytes())?;
    let initial_binding = select_carrier_binding_from_multipath_schedule(&plan.multipath_schedule, flow_key)
        .map_err(|reason| format!("lane selection failed: {reason}"))?;
    let fallback_bindings =
        active_fallback_bindings(&plan.multipath_schedule, flow_key, initial_binding)?;
    assert!(
        fallback_bindings.len() >= 2,
        "test needs at least two fallback lanes"
    );

    let dispatcher = new_shared_transit_dispatcher();
    dispatcher.register(initial_binding, dead_peer_stream()?)?;
    dispatcher.register(fallback_bindings[0], dead_peer_stream()?)?;
    let (live_peer, mut live_remote) = test_peer_pair()?;
    dispatcher.register(fallback_bindings[1], live_peer)?;

    let ack_worker = thread::spawn(move || -> Result<(), String> {
        let forwarded = live_remote.read_secure_payload()?;
        assert!(forwarded.starts_with(b"CONNECT "));
        live_remote.write_line("OK")?;
        Ok(())
    });

    let dispatcher_for_worker = dispatcher.clone();
    let document_for_worker = document.clone();
    let worker = thread::spawn(move || {
        handle_local_client_with_lane_document_and_first_byte(
            local_server,
            &document_for_worker,
            dispatcher_for_worker,
            LOCAL_MAGIC[0],
        )
    });

    local_client
        .set_read_timeout(Some(Duration::from_millis(600)))
        .map_err(|error| format!("set local timeout failed: {error}"))?;
    local_client
        .write_all(&LOCAL_MAGIC[1..])
        .and_then(|_| local_client.write_all(b"CONNECT lane-repath.example.org 443
"))
        .map_err(|error| format!("write local connect failed: {error}"))?;
    local_client
        .shutdown(Shutdown::Write)
        .map_err(|error| format!("shutdown local writer failed: {error}"))?;

    let ack = crate::peer_egress::protocol::read_line_limited(&mut local_client, 16)?;
    assert_eq!(ack, "OK");

    worker
        .join()
        .map_err(|_| "local ingress worker panicked".to_string())??;
    ack_worker
        .join()
        .map_err(|_| "ack worker panicked".to_string())??;
    Ok(())
}
