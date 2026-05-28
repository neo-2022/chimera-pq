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
}

pub type SharedPeerPool = Arc<PeerPool>;

pub fn new_shared_pool() -> SharedPeerPool {
    Arc::new(PeerPool::default())
}
