use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use crate::peer_egress::protocol::SecurePeerStream;
use crate::peer_egress::transit_binding::TransitPathBinding;

#[derive(Default)]
pub struct TransitNextHopDispatcher {
    peers: Mutex<BTreeMap<TransitPathBinding, SecurePeerStream>>,
}

impl fmt::Debug for TransitNextHopDispatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bindings = self
            .peers
            .lock()
            .map(|peers| peers.len())
            .unwrap_or_default();
        f.debug_struct("TransitNextHopDispatcher")
            .field("bindings", &bindings)
            .field("peers", &"<redacted>")
            .finish()
    }
}

impl TransitNextHopDispatcher {
    pub fn register(
        &self,
        binding: TransitPathBinding,
        peer: SecurePeerStream,
    ) -> Result<(), String> {
        let mut peers = self
            .peers
            .lock()
            .map_err(|_| "sealed transit binding dispatcher lock poisoned".to_string())?;
        if peers.contains_key(&binding) {
            return Err("sealed transit path binding ambiguous".to_string());
        }
        peers.insert(binding, peer);
        Ok(())
    }

    pub fn pop_for(&self, binding: TransitPathBinding) -> Result<SecurePeerStream, String> {
        let mut peers = self
            .peers
            .lock()
            .map_err(|_| "sealed transit binding dispatcher lock poisoned".to_string())?;
        peers
            .remove(&binding)
            .ok_or_else(|| "sealed transit path binding unavailable".to_string())
    }

    pub fn contains_binding(&self, binding: TransitPathBinding) -> Result<bool, String> {
        let peers = self
            .peers
            .lock()
            .map_err(|_| "sealed transit binding dispatcher lock poisoned".to_string())?;
        Ok(peers.contains_key(&binding))
    }
}

pub type SharedTransitNextHopDispatcher = Arc<TransitNextHopDispatcher>;

pub fn new_shared_transit_dispatcher() -> SharedTransitNextHopDispatcher {
    Arc::new(TransitNextHopDispatcher::default())
}

#[cfg(test)]
mod tests {
    use super::TransitNextHopDispatcher;
    use crate::peer_egress::options::AeadSuite;
    use crate::peer_egress::protocol::SecurePeerStream;
    use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};

    fn binding(route: u64, lane: u16) -> TransitPathBinding {
        TransitPathBinding::new(
            TransitRouteId::new(route).unwrap_or_else(|e| unreachable!("{e}")),
            TransitLaneId::new(lane).unwrap_or_else(|e| unreachable!("{e}")),
        )
    }

    fn test_peer_stream() -> Result<SecurePeerStream, String> {
        let transcript = chimera_crypto::TranscriptHash::from_messages(&[b"dispatch-test"]);
        let secrets = chimera_crypto::derive_traffic_secrets(
            chimera_crypto::SuiteId(AeadSuite::Chacha20Poly1305.suite_id()),
            &transcript,
            &[19_u8; 32],
        )
        .map_err(|error| format!("test secrets derive failed: {error}"))?;
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|error| format!("bind test listener failed: {error}"))?;
        let addr = listener
            .local_addr()
            .map_err(|error| format!("read listener addr failed: {error}"))?;
        let client = std::net::TcpStream::connect(addr)
            .map_err(|error| format!("connect test client failed: {error}"))?;
        let (server, _) = listener
            .accept()
            .map_err(|error| format!("accept test peer failed: {error}"))?;
        drop(server);
        Ok(SecurePeerStream {
            stream: client,
            send_secret: secrets.initiator_to_responder().clone(),
            recv_secret: secrets.responder_to_initiator().clone(),
            send_packet: 0,
            recv_packet: 0,
            aead: AeadSuite::Chacha20Poly1305,
        })
    }

    #[test]
    fn dispatcher_rejects_duplicate_binding() -> Result<(), String> {
        let dispatcher = TransitNextHopDispatcher::default();
        let binding = binding(1, 1);
        dispatcher.register(binding, test_peer_stream()?)?;
        let error = match dispatcher.register(binding, test_peer_stream()?) {
            Ok(()) => return Err("duplicate binding must fail".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("ambiguous"));
        Ok(())
    }

    #[test]
    fn dispatcher_pops_only_matching_binding() -> Result<(), String> {
        let dispatcher = TransitNextHopDispatcher::default();
        let first = binding(1, 1);
        let second = binding(1, 2);
        dispatcher.register(first, test_peer_stream()?)?;
        dispatcher.register(second, test_peer_stream()?)?;

        let peer = dispatcher.pop_for(second)?;
        drop(peer);
        assert!(dispatcher.contains_binding(first)?);
        assert!(!dispatcher.contains_binding(second)?);
        assert!(dispatcher.pop_for(binding(9, 9)).is_err());
        Ok(())
    }

    #[test]
    fn dispatcher_allows_fresh_registration_after_binding_is_claimed() -> Result<(), String> {
        let dispatcher = TransitNextHopDispatcher::default();
        let binding = binding(7, 1);

        for _ in 0..3 {
            dispatcher.register(binding, test_peer_stream()?)?;
            assert!(dispatcher.contains_binding(binding)?);

            let error = match dispatcher.register(binding, test_peer_stream()?) {
                Ok(()) => return Err("occupied binding must fail before claim".to_string()),
                Err(error) => error,
            };
            assert!(error.contains("ambiguous"));

            let claimed_peer = dispatcher.pop_for(binding)?;
            drop(claimed_peer);
            assert!(!dispatcher.contains_binding(binding)?);
        }

        assert!(dispatcher.pop_for(binding).is_err());
        Ok(())
    }
}
