use core::fmt;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Mutex};

use crate::peer_egress::aggregate_reassembly::{
    AggregateTransitObjectReassembler, AggregateTransitReassemblyLimits,
    AggregateTransitReassemblyStatus,
};
use crate::peer_egress::aggregate_wire::{AggregateObjectId, AggregateTransitShardFrame};
use crate::peer_egress::transit::TransitRelayFrame;
use crate::peer_egress::transit_binding::TransitRouteId;

const DEFAULT_MAX_ACTIVE_AGGREGATE_INGRESS_OBJECTS: usize = 1024;
const DEFAULT_MAX_COMPLETED_AGGREGATE_INGRESS_OBJECTS: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AggregateTransitIngressLimits {
    pub reassembly: AggregateTransitReassemblyLimits,
    pub max_active_objects: usize,
    pub max_completed_objects: usize,
}

impl Default for AggregateTransitIngressLimits {
    fn default() -> Self {
        Self {
            reassembly: AggregateTransitReassemblyLimits::default(),
            max_active_objects: DEFAULT_MAX_ACTIVE_AGGREGATE_INGRESS_OBJECTS,
            max_completed_objects: DEFAULT_MAX_COMPLETED_AGGREGATE_INGRESS_OBJECTS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AggregateTransitIngressStatus {
    Pending,
    Complete(TransitRelayFrame),
}

pub(crate) struct AggregateTransitIngressRegistry {
    state: Mutex<AggregateIngressState>,
}

impl Default for AggregateTransitIngressRegistry {
    fn default() -> Self {
        Self::new_session_scoped()
    }
}

impl fmt::Debug for AggregateTransitIngressRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.state.lock() {
            Ok(state) => f
                .debug_struct("AggregateTransitIngressRegistry")
                .field("active_objects", &state.active.len())
                .field("completed_objects", &state.completed.len())
                .field("limits", &state.limits)
                .field("keys", &"<opaque>")
                .finish(),
            Err(_) => f
                .debug_struct("AggregateTransitIngressRegistry")
                .field("state", &"<unavailable>")
                .field("lock_poisoned", &true)
                .field("keys", &"<opaque>")
                .finish(),
        }
    }
}

impl AggregateTransitIngressRegistry {
    pub(crate) fn new(limits: AggregateTransitIngressLimits) -> Result<Self, String> {
        validate_ingress_limits(limits)?;
        Ok(Self {
            state: Mutex::new(AggregateIngressState::new(limits)),
        })
    }

    pub(crate) fn new_session_scoped() -> Self {
        Self {
            state: Mutex::new(AggregateIngressState::new(
                AggregateTransitIngressLimits::default(),
            )),
        }
    }

    pub(crate) fn accept_shard(
        &self,
        shard: AggregateTransitShardFrame,
    ) -> Result<AggregateTransitIngressStatus, String> {
        let key = AggregateIngressKey::from_shard(&shard);
        let mut state = self
            .state
            .lock()
            .map_err(|_| "aggregate ingress registry lock poisoned".to_string())?;
        if state.completed.contains(&key) {
            return Err("aggregate ingress object already complete".to_string());
        }

        if !state.active.contains_key(&key) {
            if state.active.len() >= state.limits.max_active_objects {
                return Err("aggregate ingress active object limit exceeded".to_string());
            }
            let reassembler = AggregateTransitObjectReassembler::new(state.limits.reassembly)
                .map_err(|error| format!("aggregate ingress reassembly limits invalid: {error}"))?;
            state.active.insert(key, reassembler);
        }

        let reassembler = state
            .active
            .get_mut(&key)
            .ok_or_else(|| "aggregate ingress reassembly state unavailable".to_string())?;
        match reassembler.accept(shard) {
            Ok(AggregateTransitReassemblyStatus::Pending) => {
                Ok(AggregateTransitIngressStatus::Pending)
            }
            Ok(AggregateTransitReassemblyStatus::Complete(frame)) => {
                state.active.remove(&key);
                record_completed_key(&mut state, key);
                Ok(AggregateTransitIngressStatus::Complete(frame))
            }
            Err(error) => {
                state.active.remove(&key);
                Err(format!("aggregate ingress reassembly failed: {error}"))
            }
        }
    }
}

pub(crate) type SharedAggregateTransitIngressRegistry = Arc<AggregateTransitIngressRegistry>;

pub(crate) fn new_shared_aggregate_transit_ingress_registry(
    limits: AggregateTransitIngressLimits,
) -> Result<SharedAggregateTransitIngressRegistry, String> {
    Ok(Arc::new(AggregateTransitIngressRegistry::new(limits)?))
}

struct AggregateIngressState {
    limits: AggregateTransitIngressLimits,
    active: BTreeMap<AggregateIngressKey, AggregateTransitObjectReassembler>,
    completed: BTreeSet<AggregateIngressKey>,
    completed_order: VecDeque<AggregateIngressKey>,
}

impl AggregateIngressState {
    fn new(limits: AggregateTransitIngressLimits) -> Self {
        Self {
            limits,
            active: BTreeMap::new(),
            completed: BTreeSet::new(),
            completed_order: VecDeque::new(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct AggregateIngressKey {
    route_id: TransitRouteId,
    aggregate_id: AggregateObjectId,
}

impl fmt::Debug for AggregateIngressKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AggregateIngressKey(<opaque>)")
    }
}

impl AggregateIngressKey {
    fn from_shard(shard: &AggregateTransitShardFrame) -> Self {
        Self {
            route_id: shard.binding().route_id(),
            aggregate_id: shard.aggregate_id(),
        }
    }
}

fn validate_ingress_limits(limits: AggregateTransitIngressLimits) -> Result<(), String> {
    if limits.max_active_objects == 0 {
        return Err("aggregate ingress active object limit invalid".to_string());
    }
    if limits.max_completed_objects == 0 {
        return Err("aggregate ingress completed object limit invalid".to_string());
    }
    let _ = AggregateTransitObjectReassembler::new(limits.reassembly)
        .map_err(|error| format!("aggregate ingress reassembly limits invalid: {error}"))?;
    Ok(())
}

fn record_completed_key(state: &mut AggregateIngressState, key: AggregateIngressKey) {
    if state.completed.insert(key) {
        state.completed_order.push_back(key);
    }
    trim_completed_keys(state);
}

fn trim_completed_keys(state: &mut AggregateIngressState) {
    while state.completed.len() > state.limits.max_completed_objects {
        let Some(oldest) = state.completed_order.pop_front() else {
            state.completed.clear();
            return;
        };
        state.completed.remove(&oldest);
    }
}

#[cfg(test)]
#[path = "aggregate_ingress_tests.rs"]
mod tests;
