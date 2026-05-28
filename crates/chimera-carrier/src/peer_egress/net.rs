use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::thread;
use std::time::Duration;

use socket2::{Domain, Protocol, SockRef, Socket, Type};

use crate::peer_egress::options::{SECURE_PLAINTEXT_CHUNK_LEN, TCP_BUFFER_BYTES};
use crate::peer_egress::protocol::SecurePeerStream;

pub fn connect_tcp(target: &str, timeout_ms: u64) -> Result<TcpStream, String> {
    let timeout = Duration::from_millis(timeout_ms);
    let addrs: Vec<SocketAddr> = target
        .to_socket_addrs()
        .map_err(|error| format!("resolve target failed: {error}"))?
        .collect();
    if addrs.is_empty() {
        return Err("target resolved to no socket addresses".to_string());
    }
    let mut last_error = String::new();
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(stream) => return Ok(stream),
            Err(error) => last_error = format!("{addr}: {error}"),
        }
    }
    Err(last_error)
}

pub fn tune_tcp(stream: &TcpStream) -> Result<(), String> {
    stream
        .set_nodelay(true)
        .map_err(|error| format!("set TCP_NODELAY failed: {error}"))?;
    tune_tcp_buffers(stream);
    Ok(())
}

fn tune_tcp_buffers(stream: &TcpStream) {
    let socket = SockRef::from(stream);
    let _ = socket.set_recv_buffer_size(TCP_BUFFER_BYTES);
    let _ = socket.set_send_buffer_size(TCP_BUFFER_BYTES);
}

pub fn bind_reuse_listener(addr: &str) -> Result<TcpListener, String> {
    let listen_addr: SocketAddr = addr
        .parse()
        .map_err(|error| format!("parse listen address failed: {error}"))?;
    let domain = if listen_addr.is_ipv4() {
        Domain::IPV4
    } else {
        Domain::IPV6
    };
    let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))
        .map_err(|error| format!("create listener socket failed: {error}"))?;
    socket
        .set_reuse_address(true)
        .map_err(|error| format!("set SO_REUSEADDR failed: {error}"))?;
    socket
        .bind(&listen_addr.into())
        .map_err(|error| format!("bind listener failed: {error}"))?;
    socket
        .listen(1024)
        .map_err(|error| format!("listen failed: {error}"))?;
    Ok(TcpListener::from(socket))
}

pub fn relay_plain(left: TcpStream, right: TcpStream) -> Result<(), String> {
    let mut left_read = left
        .try_clone()
        .map_err(|error| format!("clone left stream failed: {error}"))?;
    let mut right_write = right
        .try_clone()
        .map_err(|error| format!("clone right stream failed: {error}"))?;
    let mut right_read = right;
    let mut left_write = left;

    let a = thread::spawn(move || copy_until_eof(&mut left_read, &mut right_write));
    let b = thread::spawn(move || copy_until_eof(&mut right_read, &mut left_write));
    let _ = a.join().map_err(|_| "left relay panicked".to_string())?;
    let _ = b.join().map_err(|_| "right relay panicked".to_string())?;
    Ok(())
}

fn copy_until_eof(reader: &mut TcpStream, writer: &mut TcpStream) -> Result<(), String> {
    let mut buf = vec![0_u8; SECURE_PLAINTEXT_CHUNK_LEN];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => {
                let _ = writer.shutdown(Shutdown::Write);
                return Ok(());
            }
            Ok(n) => writer
                .write_all(&buf[..n])
                .map_err(|error| format!("relay write failed: {error}"))?,
            Err(error) => return Err(format!("relay read failed: {error}")),
        }
    }
}

pub fn pipe_plain_with_secure_peer(
    mut plain: TcpStream,
    mut peer: SecurePeerStream,
) -> Result<(), String> {
    let mut plain_read = plain
        .try_clone()
        .map_err(|error| format!("clone plain failed: {error}"))?;
    let mut peer_write = SecurePeerStream {
        stream: peer
            .stream
            .try_clone()
            .map_err(|error| format!("clone peer failed: {error}"))?,
        send_secret: peer.send_secret.clone(),
        recv_secret: peer.recv_secret.clone(),
        send_packet: peer.send_packet,
        recv_packet: peer.recv_packet,
        aead: peer.aead,
    };
    let a = thread::spawn(move || {
        let mut buf = vec![0_u8; SECURE_PLAINTEXT_CHUNK_LEN];
        loop {
            match plain_read.read(&mut buf) {
                Ok(0) => {
                    let _ = peer_write.write_secure_payload(&[]);
                    break;
                }
                Ok(n) => {
                    if peer_write.write_secure_payload(&buf[..n]).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    let b = thread::spawn(move || {
        loop {
            match peer.read_secure_payload() {
                Ok(payload) if payload.is_empty() => break,
                Ok(payload) => {
                    if plain.write_all(&payload).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = plain.shutdown(Shutdown::Write);
    });
    let _ = a.join();
    let _ = b.join();
    Ok(())
}

pub fn pipe_secure_peer_with_plain(peer: SecurePeerStream, plain: TcpStream) -> Result<(), String> {
    pipe_plain_with_secure_peer(plain, peer)
}

pub fn write_repeating_payload(stream: &mut TcpStream, bytes: usize) -> Result<(), String> {
    let chunk = vec![0xA5_u8; SECURE_PLAINTEXT_CHUNK_LEN];
    let mut remaining = bytes;
    while remaining > 0 {
        let n = remaining.min(chunk.len());
        stream
            .write_all(&chunk[..n])
            .map_err(|error| format!("write bench payload failed: {error}"))?;
        remaining -= n;
    }
    Ok(())
}

pub fn read_exact_bytes(stream: &mut TcpStream, bytes: usize) -> Result<(), String> {
    let mut buf = vec![0_u8; SECURE_PLAINTEXT_CHUNK_LEN];
    let mut remaining = bytes;
    while remaining > 0 {
        let n = remaining.min(buf.len());
        stream
            .read_exact(&mut buf[..n])
            .map_err(|error| format!("read download payload failed: {error}"))?;
        remaining -= n;
    }
    Ok(())
}
