//! Phase 3 in-process sealed multi-hop transit proof.
//!
//! Topology: Alice (source) -> Bob (transit) -> Charlie (destination).
//! All three nodes are real threads with real TCP sockets and the same
//! CHIMERA peer-egress secure handshake used in production, but they run
//! on 127.0.0.1 inside one deterministic integration test.
//!
//! Evidence produced:
//! - The payload injected by Alice reaches Charlie byte-for-byte unchanged.
//! - The transit node (Bob) logs sealed-transit forwarding events but never
//!   logs the secret payload bytes.
//! - The planner/flow-key path is exercised because Alice and Bob each select
//!   the next hop from a shared peer pool keyed by the opaque sealed bytes.

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use chimera_carrier::peer_egress::handshake::{
    authenticate_peer, establish_secure_peer_client, establish_secure_peer_server,
};
use chimera_carrier::peer_egress::options::{AeadSuite, HANDSHAKE_MAGIC};
use chimera_carrier::peer_egress::pool::{PeerPool, SharedPeerPool};
use chimera_carrier::peer_egress::protocol::SecurePeerStream;
use chimera_carrier::peer_egress::relay_local_sealed_transit_to_next_hop;
use chimera_carrier::peer_egress::transit::{PeerTransitPolicy, validate_transit_relay_frame};
use chimera_mesh::MeshMultipathFlowKey;
use chimera_session::{Frame, FrameKind};

const TOKEN: &str = "phase3-sealed-multi-hop-test";
const SECRET_MARKER: &str = "PHASE3_SECRET_MARKER_7a9f_echo_reaches_charlie";
const CONNECT_TIMEOUT_MS: u64 = 5_000;

fn aead() -> AeadSuite {
    AeadSuite::Chacha20Poly1305
}

fn bind_local() -> Result<TcpListener, String> {
    TcpListener::bind("127.0.0.1:0").map_err(|e| format!("bind failed: {e}"))
}

fn local_addr(listener: &TcpListener) -> Result<SocketAddr, String> {
    listener
        .local_addr()
        .map_err(|e| format!("local_addr failed: {e}"))
}

fn write_peer_magic(stream: &mut TcpStream) -> Result<(), String> {
    stream
        .write_all(HANDSHAKE_MAGIC)
        .and_then(|_| stream.write_all(TOKEN.as_bytes()))
        .and_then(|_| stream.write_all(b"\n"))
        .map_err(|e| format!("write handshake failed: {e}"))
}

fn establish_client(stream: TcpStream) -> Result<SecurePeerStream, String> {
    let mut stream = stream;
    write_peer_magic(&mut stream)?;
    establish_secure_peer_client(stream, TOKEN, aead())
        .map_err(|e| format!("secure client handshake failed: {e}"))
}

fn establish_server(stream: TcpStream) -> Result<SecurePeerStream, String> {
    let mut stream = stream;
    authenticate_peer(&mut stream, TOKEN).map_err(|e| format!("peer auth failed: {e}"))?;
    establish_secure_peer_server(stream, TOKEN, aead())
        .map_err(|e| format!("secure server handshake failed: {e}"))
}

fn connect_client(addr: SocketAddr) -> Result<SecurePeerStream, String> {
    let stream = TcpStream::connect_timeout(&addr, Duration::from_millis(CONNECT_TIMEOUT_MS))
        .map_err(|e| format!("connect failed: {e}"))?;
    establish_client(stream)
}

fn build_sealed_flow(secret: &str) -> Result<Vec<u8>, String> {
    let data = Frame {
        kind: FrameKind::Data,
        packet_number: 1,
        payload: secret.as_bytes().to_vec(),
    }
    .encode()
    .map_err(|e| format!("encode data frame failed: {e}"))?;

    let fin = Frame {
        kind: FrameKind::Fin,
        packet_number: 2,
        payload: Vec::new(),
    }
    .encode()
    .map_err(|e| format!("encode fin frame failed: {e}"))?;

    let mut flow = data;
    flow.extend_from_slice(&fin);
    Ok(flow)
}

fn transit_node_forwarder(
    source: SecurePeerStream,
    next_hop_pool: SharedPeerPool,
) -> Result<(), String> {
    let mut source = source;
    let first_payload = source
        .read_secure_payload()
        .map_err(|e| format!("transit read first frame failed: {e}"))?;
    let first = validate_transit_relay_frame(&first_payload)
        .map_err(|e| format!("transit validate first frame failed: {e}"))?;
    let flow_key = MeshMultipathFlowKey::from_opaque_flow_bytes(first.sealed_bytes())
        .map_err(|e| format!("flow key failed: {e}"))?;
    let mut next = next_hop_pool
        .try_pop_for_flow_key(flow_key)
        .map_err(|e| format!("transit pop next hop failed: {e}"))?
        .ok_or_else(|| "transit next hop unavailable".to_string())?;

    next.write_secure_payload(first.sealed_bytes())
        .map_err(|e| format!("transit write first frame failed: {e}"))?;
    eprintln!("event=weave_peer_transit_frame_forwarded direction=source_to_next");

    while let Ok(payload) = source.read_secure_payload() {
        let frame = validate_transit_relay_frame(&payload)
            .map_err(|e| format!("transit validate frame failed: {e}"))?;
        next.write_secure_payload(frame.sealed_bytes())
            .map_err(|e| format!("transit write frame failed: {e}"))?;
        eprintln!("event=weave_peer_transit_frame_forwarded direction=source_to_next");
    }
    let _ = next.stream.shutdown(Shutdown::Write);
    Ok(())
}

#[test]
fn sealed_multi_hop_logs_do_not_leak_secret() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe failed: {e}"))?;
    let output = Command::new(&exe)
        .args([
            "--nocapture",
            "--exact",
            "sealed_multi_hop_transit_reaches_destination_unchanged",
        ])
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .output()
        .map_err(|e| format!("subprocess test failed to run: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("subprocess test failed: {stderr}"));
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.contains("event=weave_peer_transit_frame_forwarded") {
        return Err("missing transit forwarding event in subprocess logs".to_string());
    }
    if !stderr.contains("event=weave_transit_frame_forwarded") {
        return Err("missing local ingress forwarding event in subprocess logs".to_string());
    }
    if stderr.contains(SECRET_MARKER) {
        return Err("secret payload leaked into transit logs".to_string());
    }
    Ok(())
}

fn destination_deliver(mut peer: SecurePeerStream, target_addr: SocketAddr) -> Result<(), String> {
    let mut target =
        TcpStream::connect_timeout(&target_addr, Duration::from_millis(CONNECT_TIMEOUT_MS))
            .map_err(|e| format!("destination connect target failed: {e}"))?;
    while let Ok(payload) = peer.read_secure_payload() {
        if payload.is_empty() {
            continue;
        }
        let frame = validate_transit_relay_frame(&payload)
            .map_err(|e| format!("destination validate frame failed: {e}"))?;
        target
            .write_all(frame.sealed_bytes())
            .map_err(|e| format!("destination write frame failed: {e}"))?;
    }
    let _ = target.shutdown(Shutdown::Write);
    let _ = peer.stream.shutdown(Shutdown::Both);
    Ok(())
}

#[test]
fn sealed_multi_hop_transit_reaches_destination_unchanged() -> Result<(), String> {
    // Destination echo server (Charlie's local target).
    let dest_listener = bind_local()?;
    let dest_addr = local_addr(&dest_listener)?;
    let (dest_tx, dest_rx) = mpsc::channel::<Vec<u8>>();
    thread::spawn(move || {
        let (mut stream, _) = match dest_listener.accept() {
            Ok(v) => v,
            Err(_) => {
                let _ = dest_tx.send(Vec::new());
                return;
            }
        };
        let mut buf = Vec::new();
        let _ = stream.read_to_end(&mut buf);
        let _ = dest_tx.send(buf);
    });

    // Charlie: peer listener. Accepts Bob and delivers sealed frames to the destination target.
    let charlie_listener = bind_local()?;
    let charlie_peer_addr = local_addr(&charlie_listener)?;
    let charlie_handle = thread::spawn(move || {
        let (stream, _) = charlie_listener
            .accept()
            .map_err(|e| format!("charlie accept failed: {e}"))?;
        let peer = establish_server(stream)?;
        destination_deliver(peer, dest_addr)
    });

    // Bob: pre-connect to Charlie so the next-hop pool is ready before Alice arrives.
    let bob_to_charlie = connect_client(charlie_peer_addr)?;
    let bob_pool: SharedPeerPool = PeerPool::default().into();
    bob_pool
        .push(bob_to_charlie)
        .map_err(|e| format!("bob pool push failed: {e}"))?;

    // Bob: peer listener, accepts Alice and forwards sealed frames to Charlie.
    let bob_listener = bind_local()?;
    let bob_peer_addr = local_addr(&bob_listener)?;
    let bob_handle = thread::spawn(move || {
        let (stream, _) = bob_listener
            .accept()
            .map_err(|e| format!("bob accept failed: {e}"))?;
        let peer = establish_server(stream)?;
        transit_node_forwarder(peer, bob_pool)
    });

    // Alice: pre-connect to Bob so the egress pool is ready.
    let alice_to_bob = connect_client(bob_peer_addr)?;
    let alice_pool: SharedPeerPool = PeerPool::default().into();
    alice_pool
        .push(alice_to_bob)
        .map_err(|e| format!("alice pool push failed: {e}"))?;

    // Alice: local ingress listener. Accepts our client and forwards to Bob.
    let alice_local = bind_local()?;
    let alice_local_addr = local_addr(&alice_local)?;
    let alice_handle = thread::spawn(move || {
        let (mut stream, _) = alice_local
            .accept()
            .map_err(|e| format!("alice local accept failed: {e}"))?;
        let mut first = [0_u8; 1];
        stream
            .read_exact(&mut first)
            .map_err(|e| format!("alice read first byte failed: {e}"))?;
        relay_local_sealed_transit_to_next_hop(
            stream,
            PeerTransitPolicy::AllowPoolNextHop,
            alice_pool,
            first[0],
        )
    });

    // Client: write the sealed flow into Alice's local ingress.
    let sealed_flow = build_sealed_flow(SECRET_MARKER)?;
    let mut client =
        TcpStream::connect_timeout(&alice_local_addr, Duration::from_millis(CONNECT_TIMEOUT_MS))
            .map_err(|e| format!("client connect failed: {e}"))?;
    client
        .write_all(&sealed_flow)
        .map_err(|e| format!("client write failed: {e}"))?;
    client
        .shutdown(Shutdown::Write)
        .map_err(|e| format!("client shutdown failed: {e}"))?;

    // Wait for all hops.
    alice_handle
        .join()
        .map_err(|_| "alice thread panicked".to_string())??;
    bob_handle
        .join()
        .map_err(|_| "bob thread panicked".to_string())??;
    charlie_handle
        .join()
        .map_err(|_| "charlie thread panicked".to_string())??;

    let received = dest_rx
        .recv_timeout(Duration::from_secs(5))
        .map_err(|e| format!("destination receive timeout: {e}"))?;

    if received != sealed_flow {
        return Err(format!(
            "payload mismatch: sent {} bytes, received {} bytes",
            sealed_flow.len(),
            received.len()
        ));
    }

    Ok(())
}
