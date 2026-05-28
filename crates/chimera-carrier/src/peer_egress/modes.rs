use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::peer_egress::handshake::{authenticate_peer, establish_secure_peer_client, establish_secure_peer_server};
use crate::peer_egress::net::{
    bind_reuse_listener, connect_tcp, pipe_plain_with_secure_peer,
    pipe_secure_peer_with_plain, read_exact_bytes, tune_tcp, write_repeating_payload,
};
use crate::peer_egress::options::{
    enforce_min_throughput, split_host_port, write_resolved_state_file, AeadSuite,
    LOCAL_MAGIC, Mode, Options, SECURE_PLAINTEXT_CHUNK_LEN,
};
use crate::peer_egress::pool::{new_shared_pool, SharedPeerPool, PeerPool};
use crate::peer_egress::protocol::{
    parse_peer_connect_request, read_line_limited, read_native_connect_destination,
    read_socks5_connect_destination, write_socks5_success, SecurePeerStream,
};

pub fn run_vps(options: Options) -> Result<(), String> {
    let peer_listener = bind_reuse_listener(&options.peer_listen)
        .map_err(|error| format!("bind peer listener failed: {error}"))?;
    let local_listener = bind_reuse_listener(&options.local_listen)
        .map_err(|error| format!("bind local listener failed: {error}"))?;
    let resolved_peer_listen = peer_listener
        .local_addr()
        .map_err(|error| format!("resolve peer listener addr failed: {error}"))?
        .to_string();
    let resolved_local_listen = local_listener
        .local_addr()
        .map_err(|error| format!("resolve local listener addr failed: {error}"))?
        .to_string();
    if let Some(state_file) = &options.state_file
        && let Err(error) = write_resolved_state_file(
            state_file,
            &options.mode,
            &resolved_local_listen,
            &resolved_peer_listen,
        ) {
            eprintln!("event=peer_state_write_failed reason={error}");
        }
    let token = options.token.clone();
    let aead = options.aead;
    let reverse_connect = options.reverse_connect;
    if reverse_connect {
        let peer_pool = new_shared_pool();
        let r_pool = peer_pool.clone();
        let r_token = token.clone();
        thread::spawn(move || {
            for incoming in peer_listener.incoming() {
                let Ok(mut stream) = incoming else {
                    continue;
                };
                if let Err(error) = tune_tcp(&stream) {
                    eprintln!("event=peer_socket_tune_failed reason={error}");
                }
                match authenticate_peer(&mut stream, &r_token)
                    .and_then(|_| establish_secure_peer_server(stream, &r_token, aead))
                {
                    Ok(peer) => {
                        eprintln!("event=reverse_peer_authenticated");
                        let pool = r_pool.clone();
                        thread::spawn(move || {
                            if let Err(error) = handle_reverse_peer(peer, pool) {
                                eprintln!("event=reverse_peer_error reason={error}");
                            }
                        });
                    }
                    Err(error) => {
                        eprintln!("event=reverse_peer_auth_failed reason={error}");
                    }
                }
            }
        });
        println!(
            "chimera_peer_egress=vps_reverse_ready local={} peer={} resolved_local={} resolved_peer={}",
            options.local_listen,
            options.peer_listen,
            resolved_local_listen,
            resolved_peer_listen
        );
        for incoming in local_listener.incoming() {
            let Ok(local) = incoming else {
                continue;
            };
            eprintln!("event=reverse_local_ingress_accepted");
            thread::spawn(move || {
                if let Err(error) = handle_reverse_local_client(local) {
                    eprintln!("event=reverse_local_client_error reason={error}");
                }
            });
        }
    } else {
        let pool = Arc::new(PeerPool::default());
        let peer_pool = Arc::clone(&pool);
        thread::spawn(move || {
            for incoming in peer_listener.incoming() {
                let Ok(mut stream) = incoming else {
                    continue;
                };
                if let Err(error) = tune_tcp(&stream) {
                    eprintln!("event=peer_socket_tune_failed reason={error}");
                }
                match authenticate_peer(&mut stream, &token)
                    .and_then(|_| establish_secure_peer_server(stream, &token, aead))
                {
                    Ok(peer) => {
                        eprintln!("event=peer_authenticated");
                        let _ = peer_pool.push(peer);
                    }
                    Err(error) => {
                        eprintln!("event=peer_auth_failed reason={error}");
                    }
                }
            }
        });
        println!(
            "chimera_peer_egress=vps_ready local={} peer={} resolved_local={} resolved_peer={}",
            options.local_listen,
            options.peer_listen,
            resolved_local_listen,
            resolved_peer_listen
        );
        for incoming in local_listener.incoming() {
            let Ok(local) = incoming else {
                continue;
            };
            eprintln!("event=local_ingress_accepted");
            let Ok(peer) = pool.pop_wait() else {
                continue;
            };
            eprintln!("event=local_ingress_paired_with_peer");
            thread::spawn(move || {
                let _ = handle_local_client(local, peer);
            });
        }
    }
    Ok(())
}

pub fn handle_reverse_peer(mut peer: SecurePeerStream, _pool: SharedPeerPool) -> Result<(), String> {
    let request = peer.read_line(512)?;
    eprintln!("event=reverse_peer_request_received request={request}");
    let target_addr = parse_peer_connect_request(&request)?;
    eprintln!("event=reverse_peer_target_connecting target={target_addr}");
    let target = connect_tcp(&target_addr, 10_000)
        .map_err(|error| format!("reverse connect target failed: {error}"))?;
    tune_tcp(&target)?;
    eprintln!("event=reverse_peer_target_connected target={target_addr}");
    peer.write_line("OK")?;
    eprintln!("event=reverse_peer_connect_ack_sent target={target_addr}");
    pipe_secure_peer_with_plain(peer, target)
}

pub fn handle_reverse_local_client(mut local: TcpStream) -> Result<(), String> {
    tune_tcp(&local)?;
    let mut first = [0_u8; 1];
    local
        .read_exact(&mut first)
        .map_err(|error| format!("read reverse local protocol byte failed: {error}"))?;
    let destination = if first[0] == LOCAL_MAGIC[0] {
        read_native_connect_destination(&mut local, first[0])?
    } else if first[0] == 5 {
        read_socks5_connect_destination(&mut local, first[0])?
    } else {
        return Err("unsupported reverse local ingress protocol".to_string());
    };
    eprintln!(
        "event=reverse_local_ingress_destination host={} port={}",
        destination.host, destination.port
    );
    let target_addr = format!("{}:{}", destination.host, destination.port);
    eprintln!("event=reverse_local_target_connecting target={target_addr}");
    let target = connect_tcp(&target_addr, 10_000)
        .map_err(|error| format!("reverse local connect target failed: {error}"))?;
    tune_tcp(&target)?;
    eprintln!("event=reverse_local_target_connected target={target_addr}");
    local
        .write_all(b"OK\n")
        .map_err(|error| format!("write reverse local ack failed: {error}"))?;
    eprintln!("event=reverse_local_ack_sent");
    crate::peer_egress::net::relay_plain(local, target)
}

pub fn run_laptop(options: Options) -> Result<(), String> {
    println!(
        "chimera_peer_egress=laptop_connecting server={} pool={}",
        options.server, options.pool
    );
    for _ in 0..options.pool {
        let worker = options.clone();
        thread::spawn(move || {
            loop {
                if let Err(error) = laptop_worker(&worker) {
                    eprintln!("worker_error={error}");
                    thread::sleep(Duration::from_secs(1));
                }
            }
        });
    }
    loop {
        thread::sleep(Duration::from_secs(3600));
    }
}

pub fn laptop_worker(options: &Options) -> Result<(), String> {
    let mut peer = connect_tcp(&options.server, options.connect_timeout_ms)
        .map_err(|error| format!("connect peer server failed: {error}"))?;
    tune_tcp(&peer)?;
    eprintln!("event=laptop_peer_connected");
    peer.write_all(b"CHIMERA-PEER-EGRESS/1\n")
        .map_err(|error| format!("write handshake failed: {error}"))?;
    peer.write_all(options.token.as_bytes())
        .and_then(|_| peer.write_all(b"\n"))
        .map_err(|error| format!("write token failed: {error}"))?;
    let mut peer = establish_secure_peer_client(peer, &options.token, options.aead)?;
    let request = peer.read_line(512)?;
    eprintln!("event=laptop_peer_request_received request={request}");
    let target_addr = parse_peer_connect_request(&request)?;
    eprintln!("event=laptop_target_connecting target={target_addr}");
    let target = connect_tcp(&target_addr, options.connect_timeout_ms)
        .map_err(|error| format!("connect target failed: {error}"))?;
    tune_tcp(&target)?;
    eprintln!("event=laptop_target_connected target={target_addr}");
    peer.write_line("OK")?;
    eprintln!("event=laptop_peer_connect_ack_sent target={target_addr}");
    pipe_secure_peer_with_plain(peer, target)
}

pub fn handle_local_client(mut local: TcpStream, mut peer: SecurePeerStream) -> Result<(), String> {
    tune_tcp(&local)?;
    let mut first = [0_u8; 1];
    local
        .read_exact(&mut first)
        .map_err(|error| format!("read local protocol byte failed: {error}"))?;
    let (destination, native_client) = if first[0] == LOCAL_MAGIC[0] {
        (read_native_connect_destination(&mut local, first[0])?, true)
    } else if first[0] == 5 {
        (
            read_socks5_connect_destination(&mut local, first[0])?,
            false,
        )
    } else {
        return Err("unsupported local ingress protocol".to_string());
    };
    eprintln!(
        "event=local_ingress_destination host={} port={} native_client={}",
        destination.host, destination.port, native_client
    );
    let request = format!("CONNECT {} {}", destination.host, destination.port);
    peer.write_line(&request)?;
    eprintln!("event=peer_connect_request_sent request={request}");
    let ack = peer.read_line(16)?;
    if ack != "OK" {
        return Err("peer connect failed".to_string());
    }
    eprintln!("event=peer_connect_ack_received");
    if native_client {
        local
            .write_all(b"OK\n")
            .map_err(|error| format!("write native local ack failed: {error}"))?;
    } else {
        write_socks5_success(&mut local)?;
    }
    pipe_plain_with_secure_peer(local, peer)
}

pub fn run_bench(options: Options) -> Result<(), String> {
    let token = if options.token.is_empty() {
        "bench-token".to_string()
    } else {
        options.token
    };
    let target_listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("bind bench target failed: {error}"))?;
    let target_addr = target_listener
        .local_addr()
        .map_err(|error| format!("read bench target addr failed: {error}"))?;
    thread::spawn(move || {
        for incoming in target_listener.incoming() {
            let Ok(mut stream) = incoming else {
                continue;
            };
            thread::spawn(move || {
                let mut total = 0usize;
                let mut buf = vec![0_u8; SECURE_PLAINTEXT_CHUNK_LEN];
                loop {
                    match stream.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => total += n,
                        Err(_) => return,
                    }
                }
                let _ = stream.write_all(format!("OK {total}\n").as_bytes());
            });
        }
    });

    let peer_listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("bind bench peer failed: {error}"))?;
    let local_listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("bind bench local failed: {error}"))?;
    let peer_addr = peer_listener
        .local_addr()
        .map_err(|error| format!("read bench peer addr failed: {error}"))?;
    let local_addr = local_listener
        .local_addr()
        .map_err(|error| format!("read bench local addr failed: {error}"))?;
    start_vps_runtime(peer_listener, local_listener, token.clone(), options.aead);

    let worker_options = Options {
        mode: Mode::Laptop,
        local_listen: String::new(),
        peer_listen: String::new(),
        state_file: None,
        server: peer_addr.to_string(),
        token,
        pool: options.pool,
        bench_bytes: options.bench_bytes,
        target: String::new(),
        connect_timeout_ms: options.connect_timeout_ms,
        min_throughput_mib_s: options.min_throughput_mib_s,
        connections: 1,
        aead: options.aead,
        reverse_connect: false,
    };
    for _ in 0..options.pool {
        let worker = worker_options.clone();
        thread::spawn(move || {
            loop {
                if laptop_worker(&worker).is_err() {
                    thread::sleep(Duration::from_millis(10));
                }
            }
        });
    }
    thread::sleep(Duration::from_millis(50));

    let mut local = TcpStream::connect(local_addr)
        .map_err(|error| format!("connect bench local ingress failed: {error}"))?;
    tune_tcp(&local)?;
    local
        .write_all(LOCAL_MAGIC)
        .and_then(|_| {
            local.write_all(
                format!("CONNECT {} {}\n", target_addr.ip(), target_addr.port()).as_bytes(),
            )
        })
        .map_err(|error| format!("write bench local request failed: {error}"))?;
    let ack = read_line_limited(&mut local, 16)?;
    if ack != "OK" {
        return Err("bench local connect failed".to_string());
    }
    let started = Instant::now();
    write_repeating_payload(&mut local, options.bench_bytes)?;
    local
        .shutdown(Shutdown::Write)
        .map_err(|error| format!("shutdown bench writer failed: {error}"))?;
    let reply = read_line_limited(&mut local, 64)?;
    let elapsed = started.elapsed();
    let expected = format!("OK {}", options.bench_bytes);
    if reply != expected {
        return Err(format!(
            "bench byte mismatch: got {reply}, expected {expected}"
        ));
    }
    let seconds = elapsed.as_secs_f64().max(0.000_001);
    let mib = options.bench_bytes as f64 / 1024.0 / 1024.0;
    let throughput_mib_s = mib / seconds;
    enforce_min_throughput(throughput_mib_s, options.min_throughput_mib_s)?;
    println!(
        "chimera_peer_egress_bench=pass bytes={} elapsed_ms={} throughput_mib_s={:.2}",
        options.bench_bytes,
        elapsed.as_millis(),
        throughput_mib_s
    );
    Ok(())
}

pub fn run_echo(options: Options) -> Result<(), String> {
    let listener = bind_reuse_listener(&options.local_listen)
        .map_err(|error| format!("bind echo listener failed: {error}"))?;
    println!(
        "chimera_peer_egress=echo_ready listen={}",
        options.local_listen
    );
    for incoming in listener.incoming() {
        let Ok(mut stream) = incoming else {
            continue;
        };
        if let Err(error) = tune_tcp(&stream) {
            eprintln!("event=echo_socket_tune_failed reason={error}");
        }
        thread::spawn(move || {
            let mut total = 0usize;
            let mut buf = vec![0_u8; SECURE_PLAINTEXT_CHUNK_LEN];
            loop {
                match stream.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => total += n,
                    Err(_) => return,
                }
            }
            let _ = stream.write_all(format!("OK {total}\n").as_bytes());
        });
    }
    Ok(())
}

pub fn run_download_echo(options: Options) -> Result<(), String> {
    let listener = bind_reuse_listener(&options.local_listen)
        .map_err(|error| format!("bind download echo listener failed: {error}"))?;
    println!(
        "chimera_peer_egress=download_echo_ready listen={}",
        options.local_listen
    );
    for incoming in listener.incoming() {
        let Ok(mut stream) = incoming else {
            continue;
        };
        if let Err(error) = tune_tcp(&stream) {
            eprintln!("event=download_echo_socket_tune_failed reason={error}");
        }
        thread::spawn(move || {
            let Ok(request) = read_line_limited(&mut stream, 64) else {
                return;
            };
            let mut parts = request.split_whitespace();
            if parts.next() != Some("SEND") {
                return;
            }
            let Some(bytes_raw) = parts.next() else {
                return;
            };
            if parts.next().is_some() {
                return;
            }
            let Ok(bytes) = bytes_raw.parse::<usize>() else {
                return;
            };
            let _ = write_repeating_payload(&mut stream, bytes);
        });
    }
    Ok(())
}

pub fn run_probe(options: Options) -> Result<(), String> {
    let (target_host, target_port) = split_host_port(&options.target)?;
    if options.connections > 1 {
        return run_probe_parallel(options, target_host, target_port);
    }
    run_probe_connection(
        &options.server,
        &target_host,
        target_port,
        options.bench_bytes,
        options.connect_timeout_ms,
        options.min_throughput_mib_s,
        "chimera_peer_egress_probe",
    )
}

pub fn run_download_probe(options: Options) -> Result<(), String> {
    let (target_host, target_port) = split_host_port(&options.target)?;
    if options.connections > 1 {
        return run_download_probe_parallel(options, target_host, target_port);
    }
    run_download_probe_connection(
        &options.server,
        &target_host,
        target_port,
        options.bench_bytes,
        options.connect_timeout_ms,
        options.min_throughput_mib_s,
        "chimera_peer_egress_download_probe",
    )
}

fn run_download_probe_parallel(
    options: Options,
    target_host: String,
    target_port: u16,
) -> Result<(), String> {
    let started = Instant::now();
    let base = options.bench_bytes / options.connections;
    let remainder = options.bench_bytes % options.connections;
    let mut workers = Vec::with_capacity(options.connections);
    for index in 0..options.connections {
        let server = options.server.clone();
        let target_host = target_host.clone();
        let bytes = base + usize::from(index < remainder);
        let connect_timeout_ms = options.connect_timeout_ms;
        workers.push(thread::spawn(move || {
            run_download_probe_connection(
                &server,
                &target_host,
                target_port,
                bytes,
                connect_timeout_ms,
                0,
                "chimera_peer_egress_download_lane",
            )
        }));
    }
    for worker in workers {
        worker
            .join()
            .map_err(|_| "download probe worker panicked".to_string())??;
    }
    let elapsed = started.elapsed();
    let seconds = elapsed.as_secs_f64().max(0.000_001);
    let mib = options.bench_bytes as f64 / 1024.0 / 1024.0;
    let throughput_mib_s = mib / seconds;
    enforce_min_throughput(throughput_mib_s, options.min_throughput_mib_s)?;
    println!(
        "chimera_peer_egress_download_probe=pass bytes={} connections={} elapsed_ms={} throughput_mib_s={:.2}",
        options.bench_bytes,
        options.connections,
        elapsed.as_millis(),
        throughput_mib_s
    );
    Ok(())
}

fn run_download_probe_connection(
    server: &str,
    target_host: &str,
    target_port: u16,
    bench_bytes: usize,
    connect_timeout_ms: u64,
    min_throughput_mib_s: u64,
    event_name: &str,
) -> Result<(), String> {
    let mut local = connect_tcp(server, connect_timeout_ms)
        .map_err(|error| format!("connect native local ingress failed: {error}"))?;
    tune_tcp(&local)?;
    local
        .write_all(LOCAL_MAGIC)
        .and_then(|_| local.write_all(format!("CONNECT {target_host} {target_port}\n").as_bytes()))
        .map_err(|error| format!("write native download request failed: {error}"))?;
    let ack = read_line_limited(&mut local, 16)?;
    if ack != "OK" {
        return Err("native download connect failed".to_string());
    }
    let started = Instant::now();
    local
        .write_all(format!("SEND {bench_bytes}\n").as_bytes())
        .map_err(|error| format!("write download echo request failed: {error}"))?;
    read_exact_bytes(&mut local, bench_bytes)?;
    let elapsed = started.elapsed();
    let seconds = elapsed.as_secs_f64().max(0.000_001);
    let mib = bench_bytes as f64 / 1024.0 / 1024.0;
    let throughput_mib_s = mib / seconds;
    enforce_min_throughput(throughput_mib_s, min_throughput_mib_s)?;
    println!(
        "{event_name}=pass bytes={} elapsed_ms={} throughput_mib_s={:.2}",
        bench_bytes,
        elapsed.as_millis(),
        throughput_mib_s
    );
    Ok(())
}

fn run_probe_parallel(
    options: Options,
    target_host: String,
    target_port: u16,
) -> Result<(), String> {
    let started = Instant::now();
    let base = options.bench_bytes / options.connections;
    let remainder = options.bench_bytes % options.connections;
    let mut workers = Vec::with_capacity(options.connections);
    for index in 0..options.connections {
        let server = options.server.clone();
        let target_host = target_host.clone();
        let bytes = base + usize::from(index < remainder);
        let connect_timeout_ms = options.connect_timeout_ms;
        workers.push(thread::spawn(move || {
            run_probe_connection(
                &server,
                &target_host,
                target_port,
                bytes,
                connect_timeout_ms,
                0,
                "chimera_peer_egress_probe_lane",
            )
        }));
    }
    for worker in workers {
        worker
            .join()
            .map_err(|_| "probe worker panicked".to_string())??;
    }
    let elapsed = started.elapsed();
    let seconds = elapsed.as_secs_f64().max(0.000_001);
    let mib = options.bench_bytes as f64 / 1024.0 / 1024.0;
    let throughput_mib_s = mib / seconds;
    enforce_min_throughput(throughput_mib_s, options.min_throughput_mib_s)?;
    println!(
        "chimera_peer_egress_probe=pass bytes={} connections={} elapsed_ms={} throughput_mib_s={:.2}",
        options.bench_bytes,
        options.connections,
        elapsed.as_millis(),
        throughput_mib_s
    );
    Ok(())
}

fn run_probe_connection(
    server: &str,
    target_host: &str,
    target_port: u16,
    bench_bytes: usize,
    connect_timeout_ms: u64,
    min_throughput_mib_s: u64,
    event_name: &str,
) -> Result<(), String> {
    let mut local = connect_tcp(server, connect_timeout_ms)
        .map_err(|error| format!("connect native local ingress failed: {error}"))?;
    tune_tcp(&local)?;
    local
        .write_all(LOCAL_MAGIC)
        .and_then(|_| local.write_all(format!("CONNECT {target_host} {target_port}\n").as_bytes()))
        .map_err(|error| format!("write native probe request failed: {error}"))?;
    let ack = read_line_limited(&mut local, 16)?;
    if ack != "OK" {
        return Err("native probe connect failed".to_string());
    }
    let started = Instant::now();
    write_repeating_payload(&mut local, bench_bytes)?;
    local
        .shutdown(Shutdown::Write)
        .map_err(|error| format!("shutdown native probe writer failed: {error}"))?;
    let reply = read_line_limited(&mut local, 64)?;
    let elapsed = started.elapsed();
    let expected = format!("OK {bench_bytes}");
    if reply != expected {
        return Err(format!(
            "native probe byte mismatch: got {reply}, expected {expected}"
        ));
    }
    let seconds = elapsed.as_secs_f64().max(0.000_001);
    let mib = bench_bytes as f64 / 1024.0 / 1024.0;
    let throughput_mib_s = mib / seconds;
    enforce_min_throughput(throughput_mib_s, min_throughput_mib_s)?;
    println!(
        "{event_name}=pass bytes={} elapsed_ms={} throughput_mib_s={:.2}",
        bench_bytes,
        elapsed.as_millis(),
        throughput_mib_s
    );
    Ok(())
}

pub fn start_vps_runtime(
    peer_listener: TcpListener,
    local_listener: TcpListener,
    token: String,
    aead: AeadSuite,
) {
    let pool = Arc::new(PeerPool::default());
    let peer_pool = Arc::clone(&pool);
    thread::spawn(move || {
        for incoming in peer_listener.incoming() {
            let Ok(mut stream) = incoming else {
                continue;
            };
            match tune_tcp(&stream)
                .and_then(|_| authenticate_peer(&mut stream, &token))
                .and_then(|_| establish_secure_peer_server(stream, &token, aead))
            {
                Ok(peer) => {
                    eprintln!("event=bench_peer_authenticated");
                    let _ = peer_pool.push(peer);
                }
                Err(error) => {
                    eprintln!("event=bench_peer_auth_failed reason={error}");
                }
            }
        }
    });
    thread::spawn(move || {
        for incoming in local_listener.incoming() {
            let Ok(local) = incoming else {
                continue;
            };
            eprintln!("event=bench_local_ingress_accepted");
            let Ok(peer) = pool.pop_wait() else {
                continue;
            };
            eprintln!("event=bench_local_ingress_paired_with_peer");
            thread::spawn(move || {
                let _ = handle_local_client(local, peer);
            });
        }
    });
}
