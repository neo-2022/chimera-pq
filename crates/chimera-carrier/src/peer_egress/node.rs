use std::io::Read;
use std::thread;
use std::time::Duration;

use crate::peer_egress::handshake::{authenticate_peer, establish_secure_peer_server};
use crate::peer_egress::modes::{
    handle_local_client, handle_local_client_with_first_byte, outbound_peer_worker,
};
use crate::peer_egress::net::{bind_reuse_listener, tune_tcp};
use crate::peer_egress::options::{Options, write_resolved_state_file};
use crate::peer_egress::pool::new_shared_pool;
use crate::peer_egress::protocol::redacted_log_reason;
use crate::peer_egress::startup_contract::validate_node_startup_contract;
use crate::peer_egress::transit::relay_local_sealed_transit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalIngressBranch {
    SealedTransit,
    NativeConnect,
}

fn classify_local_ingress(first_byte: u8) -> LocalIngressBranch {
    if first_byte == chimera_session::FRAME_VERSION {
        LocalIngressBranch::SealedTransit
    } else {
        LocalIngressBranch::NativeConnect
    }
}

pub fn run_node(options: Options) -> Result<(), String> {
    let startup_contract = validate_node_startup_contract(&options)?;
    let peer_listen = startup_contract.peer_listen.clone();
    let local_listen = startup_contract.local_listen.clone();
    let peer_listener = bind_reuse_listener(&peer_listen)
        .map_err(|error| format!("bind WEAVE peer ingress failed: {error}"))?;
    let local_listener = bind_reuse_listener(&local_listen)
        .map_err(|error| format!("bind WEAVE local ingress failed: {error}"))?;
    let resolved_peer_listen = peer_listener
        .local_addr()
        .map_err(|error| format!("resolve WEAVE peer ingress addr failed: {error}"))?
        .to_string();
    let resolved_local_listen = local_listener
        .local_addr()
        .map_err(|error| format!("resolve WEAVE local ingress addr failed: {error}"))?
        .to_string();
    if let Some(state_file) = &options.state_file
        && let Err(error) = write_resolved_state_file(
            state_file,
            &options.mode,
            &resolved_local_listen,
            &resolved_peer_listen,
        )
    {
        eprintln!("event=weave_node_state_write_failed reason={error}");
    }

    let token = options.token.clone();
    let aead = options.aead;
    let peer_pool = new_shared_pool();
    let ingress_pool = peer_pool.clone();
    thread::spawn(move || {
        for incoming in peer_listener.incoming() {
            let Ok(mut stream) = incoming else {
                continue;
            };
            if let Err(error) = tune_tcp(&stream) {
                eprintln!("event=weave_peer_socket_tune_failed reason={error}");
            }
            match authenticate_peer(&mut stream, &token)
                .and_then(|_| establish_secure_peer_server(stream, &token, aead))
            {
                Ok(peer) => {
                    eprintln!("event=weave_peer_ingress_authenticated");
                    if let Err(error) = ingress_pool.push(peer) {
                        eprintln!("event=weave_peer_pool_push_failed reason={error}");
                    }
                }
                Err(error) => {
                    eprintln!("event=weave_peer_ingress_auth_failed reason={error}");
                }
            }
        }
    });

    if startup_contract.outbound_bootstrap_configured {
        for _ in 0..options.pool {
            let worker = options.clone();
            thread::spawn(move || {
                loop {
                    if let Err(error) = outbound_peer_worker(&worker) {
                        eprintln!(
                            "event=weave_outbound_peer_worker_error reason_class={}",
                            redacted_log_reason(&error)
                        );
                        thread::sleep(Duration::from_secs(1));
                    }
                }
            });
        }
    }

    println!(
        "chimera_peer_egress=node_ready local={} peer={} resolved_local={} resolved_peer={} outbound_bootstrap_configured={} capabilities={}",
        startup_contract.local_listen,
        startup_contract.peer_listen,
        resolved_local_listen,
        resolved_peer_listen,
        startup_contract.outbound_bootstrap_configured,
        startup_contract.capability_names().join(",")
    );
    for incoming in local_listener.incoming() {
        let Ok(local) = incoming else {
            continue;
        };
        eprintln!("event=weave_local_ingress_accepted");
        let Ok(peer) = peer_pool.pop_wait() else {
            continue;
        };
        let mut local = local;
        let mut first = [0_u8; 1];
        match local.read_exact(&mut first) {
            Ok(())
                if matches!(
                    classify_local_ingress(first[0]),
                    LocalIngressBranch::SealedTransit
                ) =>
            {
                eprintln!("event=weave_local_ingress_transit_branch");
                thread::spawn(move || {
                    if let Err(error) = relay_local_sealed_transit(local, peer, first[0]) {
                        eprintln!(
                            "event=weave_local_ingress_transit_error reason_class={}",
                            redacted_log_reason(&error)
                        );
                    }
                });
            }
            Ok(()) => {
                eprintln!("event=weave_local_ingress_paired_with_peer");
                thread::spawn(move || {
                    if let Err(error) = handle_local_client_with_first_byte(local, peer, first[0]) {
                        eprintln!(
                            "event=weave_local_ingress_error reason_class={}",
                            redacted_log_reason(&error)
                        );
                    }
                });
            }
            Err(error) => {
                eprintln!("event=weave_local_ingress_peek_failed reason={error}");
                thread::spawn(move || {
                    if let Err(error) = handle_local_client(local, peer) {
                        eprintln!(
                            "event=weave_local_ingress_error reason_class={}",
                            redacted_log_reason(&error)
                        );
                    }
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{LocalIngressBranch, classify_local_ingress};
    use crate::peer_egress::options::LOCAL_MAGIC;

    #[test]
    fn transit_branch_is_reserved_for_frame_version() {
        assert_eq!(
            classify_local_ingress(chimera_session::FRAME_VERSION),
            LocalIngressBranch::SealedTransit
        );
    }

    #[test]
    fn native_connect_branch_handles_local_magic_prefix() {
        assert_eq!(
            classify_local_ingress(LOCAL_MAGIC[0]),
            LocalIngressBranch::NativeConnect
        );
    }

    #[test]
    fn native_connect_branch_handles_other_bytes() {
        assert_eq!(
            classify_local_ingress(b'X'),
            LocalIngressBranch::NativeConnect
        );
    }
}
