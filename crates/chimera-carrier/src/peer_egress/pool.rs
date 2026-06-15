use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

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

    pub fn try_pop_index(
        &self,
        zero_based_index: usize,
    ) -> Result<Option<SecurePeerStream>, String> {
        let mut peers = self
            .peers
            .lock()
            .map_err(|_| "peer pool lock poisoned".to_string())?;
        if zero_based_index >= peers.len() {
            return Ok(None);
        }
        Ok(peers.remove(zero_based_index))
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

    #[test]
    fn try_pop_index_removes_only_selected_lane_candidate() -> Result<(), String> {
        let pool = PeerPool::default();
        pool.push(test_peer_stream())?;
        pool.push(test_peer_stream())?;

        assert!(pool.try_pop_index(1)?.is_some());
        assert!(pool.try_pop_index(1)?.is_none());
        assert!(pool.try_pop_index(0)?.is_some());
        assert!(pool.try_pop_index(0)?.is_none());
        Ok(())
    }
}
