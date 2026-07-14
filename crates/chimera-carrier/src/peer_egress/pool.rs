use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use chimera_mesh::MeshMultipathFlowKey;

use crate::peer_egress::protocol::SecurePeerStream;

const DEFAULT_PEER_IDLE_TIMEOUT_MS: u64 = 30_000;

fn default_peer_idle_timeout() -> Duration {
    Duration::from_millis(DEFAULT_PEER_IDLE_TIMEOUT_MS)
}

#[derive(Debug)]
pub struct PeerPool {
    peers: Mutex<VecDeque<SecurePeerStream>>,
    ready: Condvar,
    idle_timeout: Duration,
}

impl PeerPool {
    pub fn new(idle_timeout: Duration) -> Self {
        Self {
            peers: Mutex::new(VecDeque::new()),
            ready: Condvar::new(),
            idle_timeout,
        }
    }

    pub fn push(&self, stream: SecurePeerStream) -> Result<(), String> {
        let mut peers = self
            .peers
            .lock()
            .map_err(|_| "peer pool lock poisoned".to_string())?;
        stream.touch();
        peers.push_back(stream);
        self.ready.notify_one();
        Ok(())
    }

    fn is_peer_usable(&self, peer: &SecurePeerStream) -> bool {
        peer.is_alive() && peer.idle_duration() <= self.idle_timeout
    }

    fn pop_front_alive(&self, peers: &mut VecDeque<SecurePeerStream>) -> Option<SecurePeerStream> {
        while let Some(peer) = peers.pop_front() {
            if self.is_peer_usable(&peer) {
                return Some(peer);
            }
        }
        None
    }

    fn remove_alive_at(
        &self,
        peers: &mut VecDeque<SecurePeerStream>,
        index: usize,
    ) -> Option<SecurePeerStream> {
        let peer = peers.remove(index)?;
        if self.is_peer_usable(&peer) {
            Some(peer)
        } else {
            self.pop_front_alive(peers)
        }
    }

    pub fn pop_wait(&self) -> Result<SecurePeerStream, String> {
        let mut peers = self
            .peers
            .lock()
            .map_err(|_| "peer pool lock poisoned".to_string())?;
        loop {
            if let Some(stream) = self.pop_front_alive(&mut peers) {
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
        Ok(self.pop_front_alive(&mut peers))
    }

    pub fn try_pop_unique(&self) -> Result<UniquePeerPop, String> {
        let mut peers = self
            .peers
            .lock()
            .map_err(|_| "peer pool lock poisoned".to_string())?;
        match peers.iter().filter(|p| self.is_peer_usable(p)).count() {
            0 => Ok(UniquePeerPop::Unavailable),
            1 => Ok(UniquePeerPop::Ready(
                self.pop_front_alive(&mut peers)
                    .ok_or_else(|| "peer pool unexpectedly empty".to_string())?,
            )),
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
        Ok(self.remove_alive_at(&mut peers, slot))
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
            if let Some(stream) = self.remove_alive_at(&mut peers, slot) {
                return Ok(stream);
            }
        }
    }

    pub fn pop_wait_timeout(&self, timeout: Duration) -> Result<Option<SecurePeerStream>, String> {
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| "peer pool wait timeout overflow".to_string())?;
        let mut peers = self
            .peers
            .lock()
            .map_err(|_| "peer pool lock poisoned".to_string())?;
        loop {
            if let Some(stream) = self.pop_front_alive(&mut peers) {
                return Ok(Some(stream));
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(None);
            }
            let (guard, wait_result) = self
                .ready
                .wait_timeout(peers, remaining.min(timeout))
                .map_err(|_| "peer pool wait_timeout poisoned".to_string())?;
            peers = guard;
            if wait_result.timed_out() && peers.is_empty() {
                return Ok(None);
            }
        }
    }

    pub fn pop_wait_timeout_for_flow_key(
        &self,
        flow_key: MeshMultipathFlowKey,
        timeout: Duration,
    ) -> Result<Option<SecurePeerStream>, String> {
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| "peer pool wait timeout overflow".to_string())?;
        let mut peers = self
            .peers
            .lock()
            .map_err(|_| "peer pool lock poisoned".to_string())?;
        loop {
            if !peers.is_empty() {
                let slot = flow_key.select_slot_index(peers.len())?;
                if let Some(stream) = self.remove_alive_at(&mut peers, slot) {
                    return Ok(Some(stream));
                }
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(None);
            }
            let (guard, wait_result) = self
                .ready
                .wait_timeout(peers, remaining.min(timeout))
                .map_err(|_| "peer pool wait_timeout poisoned".to_string())?;
            peers = guard;
            if wait_result.timed_out() && peers.is_empty() {
                return Ok(None);
            }
        }
    }
}

impl Default for PeerPool {
    fn default() -> Self {
        Self::new(default_peer_idle_timeout())
    }
}

pub type SharedPeerPool = Arc<PeerPool>;

pub fn new_shared_pool() -> SharedPeerPool {
    Arc::new(PeerPool::default())
}

pub fn new_shared_pool_with_timeout(idle_timeout: Duration) -> SharedPeerPool {
    Arc::new(PeerPool::new(idle_timeout))
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
    use std::time::Duration;

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
        SecurePeerStream::new(
            client,
            secrets.initiator_to_responder().clone(),
            secrets.responder_to_initiator().clone(),
            crate::peer_egress::options::AeadSuite::Chacha20Poly1305,
        )
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
        pool.push(SecurePeerStream::new(
            client,
            secrets.initiator_to_responder().clone(),
            secrets.responder_to_initiator().clone(),
            crate::peer_egress::options::AeadSuite::Chacha20Poly1305,
        ))?;
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

    #[test]
    fn try_pop_skips_marked_dead_peer() -> Result<(), String> {
        let pool = PeerPool::new(Duration::from_secs(60));
        let alive_port = push_test_peer(&pool, "alive-dead-test-alive")?;
        let dead_port = push_test_peer(&pool, "alive-dead-test-dead")?;
        {
            let mut peers = pool
                .peers
                .lock()
                .map_err(|_| "peer pool lock poisoned".to_string())?;
            for peer in peers.iter_mut() {
                if peer
                    .stream
                    .local_addr()
                    .map_err(|error| format!("local addr failed: {error}"))?
                    .port()
                    == dead_port
                {
                    peer.mark_dead();
                }
            }
        }
        let selected = pool
            .try_pop()?
            .ok_or_else(|| "expected an alive peer from pool".to_string())?;
        let selected_port = selected
            .stream
            .local_addr()
            .map_err(|error| format!("selected local addr failed: {error}"))?
            .port();
        assert_eq!(selected_port, alive_port);
        assert!(pool.try_pop()?.is_none());
        Ok(())
    }

    #[test]
    fn try_pop_skips_peer_idle_longer_than_timeout() -> Result<(), String> {
        let pool = PeerPool::new(Duration::from_millis(1));
        let _port = push_test_peer(&pool, "idle-timeout-test")?;
        std::thread::sleep(Duration::from_millis(5));
        assert!(
            pool.try_pop()?.is_none(),
            "idle peer must be treated as dead"
        );
        Ok(())
    }
}
