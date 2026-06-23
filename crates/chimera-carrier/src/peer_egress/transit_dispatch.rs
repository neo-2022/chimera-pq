use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex};

use crate::peer_egress::protocol::SecurePeerStream;
use crate::peer_egress::transit_binding::TransitPathBinding;

#[derive(Default)]
pub struct TransitNextHopDispatcher {
    state: Mutex<TransitNextHopState>,
}

#[derive(Default)]
struct TransitNextHopState {
    peers: BTreeMap<TransitPathBinding, VecDeque<TransitNextHopEntry>>,
    next_ticket_id: u64,
}

struct TransitNextHopEntry {
    ticket: TransitNextHopTicket,
    peer: SecurePeerStream,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TransitNextHopTicket {
    binding: TransitPathBinding,
    id: u64,
}

impl fmt::Debug for TransitNextHopTicket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransitNextHopTicket")
            .field("binding", &"<opaque>")
            .field("id", &"<opaque>")
            .finish()
    }
}

impl fmt::Debug for TransitNextHopDispatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (bindings, streams) = self
            .state
            .lock()
            .map(|state| {
                (
                    state.peers.len(),
                    state.peers.values().map(VecDeque::len).sum::<usize>(),
                )
            })
            .unwrap_or_default();
        f.debug_struct("TransitNextHopDispatcher")
            .field("bindings", &bindings)
            .field("streams", &streams)
            .field("peers", &"<redacted>")
            .finish()
    }
}

impl TransitNextHopDispatcher {
    pub fn register(
        &self,
        binding: TransitPathBinding,
        peer: SecurePeerStream,
    ) -> Result<TransitNextHopTicket, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "sealed transit binding dispatcher lock poisoned".to_string())?;
        let id = state
            .next_ticket_id
            .checked_add(1)
            .ok_or_else(|| "sealed transit dispatch ticket overflow".to_string())?;
        state.next_ticket_id = id;
        let ticket = TransitNextHopTicket { binding, id };
        state
            .peers
            .entry(binding)
            .or_default()
            .push_back(TransitNextHopEntry { ticket, peer });
        Ok(ticket)
    }

    pub fn pop_for(&self, binding: TransitPathBinding) -> Result<SecurePeerStream, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "sealed transit binding dispatcher lock poisoned".to_string())?;
        let (entry, remove_binding) = {
            let queue = state
                .peers
                .get_mut(&binding)
                .ok_or_else(|| "sealed transit path binding unavailable".to_string())?;
            let entry = queue
                .pop_front()
                .ok_or_else(|| "sealed transit path binding unavailable".to_string())?;
            (entry, queue.is_empty())
        };
        if remove_binding {
            state.peers.remove(&binding);
        }
        Ok(entry.peer)
    }

    pub fn pop_many_for(
        &self,
        bindings: &[TransitPathBinding],
    ) -> Result<Vec<(TransitPathBinding, SecurePeerStream)>, String> {
        if bindings.is_empty() {
            return Err("sealed transit path binding set empty".to_string());
        }
        let mut unique = BTreeSet::new();
        if bindings.iter().any(|binding| !unique.insert(*binding)) {
            return Err("sealed transit path binding set ambiguous".to_string());
        }

        let mut state = self
            .state
            .lock()
            .map_err(|_| "sealed transit binding dispatcher lock poisoned".to_string())?;
        if bindings
            .iter()
            .any(|binding| state.peers.get(binding).is_none_or(VecDeque::is_empty))
        {
            return Err("sealed transit path binding set unavailable".to_string());
        }

        let mut claimed = Vec::with_capacity(bindings.len());
        for binding in bindings {
            let (entry, remove_binding) = {
                let queue = state
                    .peers
                    .get_mut(binding)
                    .ok_or_else(|| "sealed transit path binding set unavailable".to_string())?;
                let entry = queue
                    .pop_front()
                    .ok_or_else(|| "sealed transit path binding set unavailable".to_string())?;
                (entry, queue.is_empty())
            };
            if remove_binding {
                state.peers.remove(binding);
            }
            claimed.push((*binding, entry.peer));
        }
        Ok(claimed)
    }

    pub fn contains_binding(&self, binding: TransitPathBinding) -> Result<bool, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "sealed transit binding dispatcher lock poisoned".to_string())?;
        Ok(state
            .peers
            .get(&binding)
            .is_some_and(|queue| !queue.is_empty()))
    }

    pub fn contains_ticket(&self, ticket: TransitNextHopTicket) -> Result<bool, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "sealed transit binding dispatcher lock poisoned".to_string())?;
        Ok(state
            .peers
            .get(&ticket.binding)
            .is_some_and(|queue| queue.iter().any(|entry| entry.ticket == ticket)))
    }

    pub fn clear_binding(&self, binding: TransitPathBinding) -> Result<usize, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "sealed transit binding dispatcher lock poisoned".to_string())?;
        Ok(state.peers.remove(&binding).map_or(0, |queue| queue.len()))
    }

    pub fn binding_depth(&self, binding: TransitPathBinding) -> Result<usize, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "sealed transit binding dispatcher lock poisoned".to_string())?;
        Ok(state.peers.get(&binding).map_or(0, VecDeque::len))
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
    use std::sync::Barrier;
    use std::thread;

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
    fn dispatcher_allows_parallel_streams_per_binding() -> Result<(), String> {
        let dispatcher = TransitNextHopDispatcher::default();
        let binding = binding(1, 1);
        let first = dispatcher.register(binding, test_peer_stream()?)?;
        let second = dispatcher.register(binding, test_peer_stream()?)?;

        assert_ne!(first, second);
        assert!(dispatcher.contains_ticket(first)?);
        assert!(dispatcher.contains_ticket(second)?);
        assert_eq!(dispatcher.binding_depth(binding)?, 2);
        drop(dispatcher.pop_for(binding)?);
        assert_eq!(dispatcher.binding_depth(binding)?, 1);
        assert!(!dispatcher.contains_ticket(first)?);
        assert!(dispatcher.contains_ticket(second)?);
        drop(dispatcher.pop_for(binding)?);
        assert!(!dispatcher.contains_binding(binding)?);
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

        let ticket = dispatcher.register(binding, test_peer_stream()?)?;
        assert!(dispatcher.contains_binding(binding)?);
        assert!(dispatcher.contains_ticket(ticket)?);

        let claimed_peer = dispatcher.pop_for(binding)?;
        drop(claimed_peer);
        assert!(!dispatcher.contains_binding(binding)?);
        assert!(!dispatcher.contains_ticket(ticket)?);

        dispatcher.register(binding, test_peer_stream()?)?;
        assert!(dispatcher.contains_binding(binding)?);
        let claimed_again = dispatcher.pop_for(binding)?;
        drop(claimed_again);
        assert!(dispatcher.pop_for(binding).is_err());
        Ok(())
    }

    #[test]
    fn dispatcher_pops_many_bindings_atomically() -> Result<(), String> {
        let dispatcher = TransitNextHopDispatcher::default();
        let first = binding(7, 1);
        let second = binding(7, 2);
        let missing = binding(7, 3);
        dispatcher.register(first, test_peer_stream()?)?;
        dispatcher.register(second, test_peer_stream()?)?;

        let error = match dispatcher.pop_many_for(&[first, missing]) {
            Ok(_) => return Err("partial binding set must fail".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("unavailable"));
        assert!(dispatcher.contains_binding(first)?);
        assert!(dispatcher.contains_binding(second)?);

        let claimed = dispatcher.pop_many_for(&[first, second])?;
        assert_eq!(claimed.len(), 2);
        assert!(!dispatcher.contains_binding(first)?);
        assert!(!dispatcher.contains_binding(second)?);
        Ok(())
    }

    #[test]
    fn dispatcher_rejects_duplicate_many_binding_claim() -> Result<(), String> {
        let dispatcher = TransitNextHopDispatcher::default();
        let binding = binding(7, 1);
        dispatcher.register(binding, test_peer_stream()?)?;

        let error = match dispatcher.pop_many_for(&[binding, binding]) {
            Ok(_) => return Err("duplicate binding set must fail".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("ambiguous"));
        assert!(dispatcher.contains_binding(binding)?);
        Ok(())
    }

    #[test]
    fn dispatcher_clear_binding_removes_all_parallel_streams() -> Result<(), String> {
        let dispatcher = TransitNextHopDispatcher::default();
        let binding = binding(8, 1);
        let ticket = dispatcher.register(binding, test_peer_stream()?)?;
        dispatcher.register(binding, test_peer_stream()?)?;

        assert_eq!(dispatcher.clear_binding(binding)?, 2);
        assert!(!dispatcher.contains_binding(binding)?);
        assert!(!dispatcher.contains_ticket(ticket)?);
        assert!(dispatcher.pop_for(binding).is_err());
        Ok(())
    }

    #[test]
    fn dispatcher_allows_only_one_concurrent_claim_for_binding() -> Result<(), String> {
        let dispatcher = std::sync::Arc::new(TransitNextHopDispatcher::default());
        let binding = binding(9, 1);
        dispatcher.register(binding, test_peer_stream()?)?;

        let barrier = std::sync::Arc::new(Barrier::new(3));
        let mut handles = Vec::new();
        for _ in 0..2 {
            let dispatcher = dispatcher.clone();
            let barrier = barrier.clone();
            handles.push(thread::spawn(move || {
                barrier.wait();
                dispatcher.pop_for(binding).is_ok()
            }));
        }

        barrier.wait();
        let successes = handles
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .map_err(|_| "claim thread panicked".to_string())
            })
            .collect::<Result<Vec<bool>, String>>()?
            .into_iter()
            .filter(|claimed| *claimed)
            .count();

        assert_eq!(successes, 1);
        assert!(dispatcher.pop_for(binding).is_err());
        Ok(())
    }
}
