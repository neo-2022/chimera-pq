use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

use chimera_mesh::MeshMultipathFlowKey;

use crate::peer_egress::protocol::SecurePeerStream;

#[derive(Debug, Default)]
pub struct PeerPool {
    peers: Mutex<VecDeque<SecurePeerStream>>,
    ready: Condvar,
}

impl PeerPool {
    pub fn push(&self, stream: SecurePeerStream) -> Result<(), String> {
        let mut peers = self
            .peers
            .lock()
            .map_err(|_| "peer pool lock poisoned".to_string())?;
        peers.push_back(stream);
        self.ready.notify_one();
        Ok(())
    }

    pub fn pop_wait(&self) -> Result<SecurePeerStream, String> {
        let mut peers = self
            .peers
            .lock()
            .map_err(|_| "peer pool lock poisoned".to_string())?;
        loop {
            if let Some(stream) = peers.pop_front() {
                return Ok(stream);
            }
            peers = self
                .ready
                .wait(peers)
                .map_err(|_| "peer pool wait poisoned".to_string())?;
        }
    }

    pub fn try_pop(&self) -> Result<Option<SecurePeerStream>, String> {
        let mut peers = self
            .peers
            .lock()
            .map_err(|_| "peer pool lock poisoned".to_string())?;
        Ok(peers.pop_front())
    }

    pub fn try_pop_unique(&self) -> Result<UniquePeerPop, String> {
        let mut peers = self
            .peers
            .lock()
            .map_err(|_| "peer pool lock poisoned".to_string())?;
        match peers.len() {
            0 => Ok(UniquePeerPop::Unavailable),
            1 => {
                Ok(UniquePeerPop::Ready(peers.pop_front().ok_or_else(
                    || "peer pool unexpectedly empty".to_string(),
                )?))
            }
            _ => Ok(UniquePeerPop::Ambiguous),
        }
    }

    pub fn try_pop_for_flow_key(
        &self,
        flow_key: MeshMultipathFlowKey,
    ) -> Result<Option<SecurePeerStream>, String> {
        let mut peers = self
            .peers
            .lock()
            .map_err(|_| "peer pool lock poisoned".to_string())?;
        if peers.is_empty() {
            return Ok(None);
        }
        let slot = flow_key.select_slot_index(peers.len())?;
        Ok(peers.remove(slot))
    }

    pub fn pop_wait_for_flow_key(
        &self,
        flow_key: MeshMultipathFlowKey,
    ) -> Result<SecurePeerStream, String> {
        let mut peers = self
            .peers
            .lock()
            .map_err(|_| "peer pool lock poisoned".to_string())?;
        loop {
            if peers.is_empty() {
                peers = self
                    .ready
                    .wait(peers)
                    .map_err(|_| "peer pool wait poisoned".to_string())?;
                continue;
            }
            let slot = flow_key.select_slot_index(peers.len())?;
            return peers
                .remove(slot)
                .ok_or_else(|| "peer pool unexpectedly empty".to_string());
        }
    }
}

pub type SharedPeerPool = Arc<PeerPool>;

pub fn new_shared_pool() -> SharedPeerPool {
    Arc::new(PeerPool::default())
}

#[derive(Debug)]
pub enum UniquePeerPop {
    Unavailable,
    Ambiguous,
    Ready(SecurePeerStream),
}

#[cfg(test)]
mod tests {
    use super::{PeerPool, SecurePeerStream};
    use chimera_mesh::MeshMultipathFlowKey;

    fn test_peer_stream() -> SecurePeerStream {
        let transcript = chimera_crypto::TranscriptHash::from_messages(&[b"peer-pool-test"]);
        let secrets = chimera_crypto::derive_traffic_secrets(
            chimera_crypto::SuiteId(
                crate::peer_egress::options::AeadSuite::Chacha20Poly1305.suite_id(),
            ),
            &transcript,
            &[9_u8; 32],
        )
        .unwrap_or_else(|error| unreachable!("test secrets must derive: {error}"));
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap_or_else(|error| unreachable!("listener bind failed: {error}"));
        let addr = listener
            .local_addr()
            .unwrap_or_else(|error| unreachable!("listener addr failed: {error}"));
        let client = std::net::TcpStream::connect(addr)
            .unwrap_or_else(|error| unreachable!("client connect failed: {error}"));
        let (server, _) = listener
            .accept()
            .unwrap_or_else(|error| unreachable!("server accept failed: {error}"));
        drop(server);
        SecurePeerStream {
            stream: client,
            send_secret: secrets.initiator_to_responder().clone(),
            recv_secret: secrets.responder_to_initiator().clone(),
            send_packet: 0,
            recv_packet: 0,
            aead: crate::peer_egress::options::AeadSuite::Chacha20Poly1305,
        }
    }

    #[test]
    fn try_pop_returns_peer_once_without_waiting() -> Result<(), String> {
        let pool = PeerPool::default();
        assert!(pool.try_pop()?.is_none());
        pool.push(test_peer_stream())?;
        assert!(pool.try_pop()?.is_some());
        assert!(pool.try_pop()?.is_none());
        Ok(())
    }

    #[test]
    fn try_pop_unique_requires_single_candidate() -> Result<(), String> {
        let pool = PeerPool::default();
        assert!(matches!(
            pool.try_pop_unique()?,
            super::UniquePeerPop::Unavailable
        ));

        pool.push(test_peer_stream())?;
        assert!(matches!(
            pool.try_pop_unique()?,
            super::UniquePeerPop::Ready(_)
        ));

        pool.push(test_peer_stream())?;
        pool.push(test_peer_stream())?;
        assert!(matches!(
            pool.try_pop_unique()?,
            super::UniquePeerPop::Ambiguous
        ));
        assert!(pool.try_pop()?.is_some());
        assert!(pool.try_pop()?.is_some());
        Ok(())
    }

    fn push_test_peer(pool: &PeerPool, label: &str) -> Result<u16, String> {
        let transcript = chimera_crypto::TranscriptHash::from_messages(&[label.as_bytes()]);
        let secrets = chimera_crypto::derive_traffic_secrets(
            chimera_crypto::SuiteId(
                crate::peer_egress::options::AeadSuite::Chacha20Poly1305.suite_id(),
            ),
            &transcript,
            &[11_u8; 32],
        )
        .map_err(|error| format!("test secrets derive failed: {error}"))?;
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|error| format!("listener bind failed: {error}"))?;
        let addr = listener
            .local_addr()
            .map_err(|error| format!("listener addr failed: {error}"))?;
        let client = std::net::TcpStream::connect(addr)
            .map_err(|error| format!("client connect failed: {error}"))?;
        let (server, _) = listener
            .accept()
            .map_err(|error| format!("server accept failed: {error}"))?;
        drop(server);
        let port = client
            .local_addr()
            .map_err(|error| format!("client local addr failed: {error}"))?
            .port();
        pool.push(SecurePeerStream {
            stream: client,
            send_secret: secrets.initiator_to_responder().clone(),
            recv_secret: secrets.responder_to_initiator().clone(),
            send_packet: 0,
            recv_packet: 0,
            aead: crate::peer_egress::options::AeadSuite::Chacha20Poly1305,
        })?;
        Ok(port)
    }

    #[test]
    fn try_pop_for_flow_key_returns_none_when_pool_is_empty() -> Result<(), String> {
        let pool = PeerPool::default();
        let key = MeshMultipathFlowKey::from_opaque_flow_id("flow-a")?;
        assert!(pool.try_pop_for_flow_key(key)?.is_none());
        Ok(())
    }

    #[test]
    fn try_pop_for_flow_key_selects_expected_slot() -> Result<(), String> {
        let pool = PeerPool::default();
        let ports = [
            push_test_peer(&pool, "peer-0")?,
            push_test_peer(&pool, "peer-1")?,
            push_test_peer(&pool, "peer-2")?,
        ];
        let key = MeshMultipathFlowKey::from_opaque_flow_id("flow-slot-test")?;
        let slot = key.select_slot_index(ports.len())?;
        let selected = pool
            .try_pop_for_flow_key(key)?
            .ok_or_else(|| "flow pop should select a peer".to_string())?;
        let selected_port = selected
            .stream
            .local_addr()
            .map_err(|error| format!("selected peer local addr failed: {error}"))?
            .port();

        assert_eq!(selected_port, ports[slot]);
        Ok(())
    }

    #[test]
    fn flow_key_selection_spreads_across_slots() -> Result<(), String> {
        let mut slots = std::collections::BTreeSet::new();
        for index in 0..64 {
            let pool = PeerPool::default();
            let ports = [
                push_test_peer(&pool, &format!("spread-{index}-a"))?,
                push_test_peer(&pool, &format!("spread-{index}-b"))?,
            ];
            let key = MeshMultipathFlowKey::from_opaque_flow_id(&format!("opaque-flow-{index}"))?;
            let slot = key.select_slot_index(ports.len())?;
            let selected = pool
                .try_pop_for_flow_key(key)?
                .ok_or_else(|| "flow pop should select a peer".to_string())?;
            let selected_port = selected
                .stream
                .local_addr()
                .map_err(|error| format!("selected peer local addr failed: {error}"))?
                .port();
            assert_eq!(selected_port, ports[slot]);
            slots.insert(slot);
        }
        assert!(slots.len() >= 2);
        Ok(())
    }

    #[test]
    fn pop_wait_for_flow_key_waits_until_peer_is_available() -> Result<(), String> {
        let pool = std::sync::Arc::new(PeerPool::default());
        let key = MeshMultipathFlowKey::from_opaque_flow_id("wait-flow")?;
        let worker_pool = pool.clone();
        let handle = std::thread::spawn(move || worker_pool.pop_wait_for_flow_key(key));
        std::thread::sleep(std::time::Duration::from_millis(50));
        let port = push_test_peer(&pool, "waiting-peer")?;
        let selected = handle
            .join()
            .map_err(|_| "flow wait thread panicked".to_string())??;
        let selected_port = selected
            .stream
            .local_addr()
            .map_err(|error| format!("selected peer local addr failed: {error}"))?
            .port();
        assert_eq!(selected_port, port);
        Ok(())
    }
}
