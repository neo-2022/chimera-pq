use std::collections::VecDeque;

use crate::nodes_grouping::compare_mesh_nodes_for_best;
use crate::nodes_model::{
    MeshNode, MeshNodeId, MeshNodeReasonCode, MeshNodeSwitchAction, MeshNodeSwitchDecision,
};
use crate::nodes_policy::MeshNodesPolicy;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshNodeSwitchEvent {
    pub from: Option<MeshNodeId>,
    pub to: MeshNodeId,
    pub at_tick: u64,
    pub reason: MeshNodeReasonCode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshNodeRuntimeState {
    pub current_node: Option<MeshNodeId>,
    pub pinned_node: Option<MeshNodeId>,
    pub autoconnect_enabled: bool,
    pub last_switch_tick: Option<u64>,
    pub switch_history: VecDeque<MeshNodeSwitchEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshNodeRuntime {
    pub policy: MeshNodesPolicy,
    pub state: MeshNodeRuntimeState,
}

impl MeshNodeRuntime {
    pub fn new(policy: MeshNodesPolicy) -> Result<Self, Vec<String>> {
        policy.validate()?;
        Ok(Self {
            state: MeshNodeRuntimeState {
                current_node: None,
                pinned_node: None,
                autoconnect_enabled: policy.autoconnect_enabled_by_default,
                last_switch_tick: None,
                switch_history: VecDeque::new(),
            },
            policy,
        })
    }

    pub fn set_autoconnect(&mut self, enabled: bool) {
        self.state.autoconnect_enabled = enabled;
    }

    pub fn pin(&mut self, node_id: MeshNodeId) -> MeshNodeSwitchDecision {
        self.state.pinned_node = Some(node_id.clone());
        MeshNodeSwitchDecision {
            action: MeshNodeSwitchAction::Pin,
            reason: MeshNodeReasonCode::PinnedNodeActive,
            current_node: self.state.current_node.clone(),
            candidate_node: Some(node_id),
            current_score: None,
            candidate_score: None,
            allowed: true,
            explain: "node pinned".to_string(),
        }
    }

    pub fn unpin(&mut self) -> MeshNodeSwitchDecision {
        let old = self.state.pinned_node.take();
        MeshNodeSwitchDecision {
            action: MeshNodeSwitchAction::Unpin,
            reason: MeshNodeReasonCode::BestSelected,
            current_node: self.state.current_node.clone(),
            candidate_node: old,
            current_score: None,
            candidate_score: None,
            allowed: true,
            explain: "node unpinned".to_string(),
        }
    }

    pub fn manual_connect(
        &mut self,
        nodes: &[MeshNode],
        node_id: &MeshNodeId,
        now_tick: u64,
    ) -> MeshNodeSwitchDecision {
        let Some(node) = nodes.iter().find(|node| &node.node_id == node_id) else {
            return MeshNodeSwitchDecision::noop(
                MeshNodeReasonCode::NoNodes,
                self.state.current_node.clone(),
                "requested node not found",
            );
        };
        if node.status.is_down() {
            return MeshNodeSwitchDecision::noop(
                MeshNodeReasonCode::AllNodesDown,
                self.state.current_node.clone(),
                "requested node is down",
            );
        }
        self.commit_connect(
            node,
            now_tick,
            MeshNodeReasonCode::BestSelected,
            MeshNodeSwitchAction::ManualConnect,
        )
    }

    pub fn auto_step(&mut self, nodes: &[MeshNode], now_tick: u64) -> MeshNodeSwitchDecision {
        self.prune_switch_history(now_tick);
        if nodes.is_empty() {
            return MeshNodeSwitchDecision::noop(
                MeshNodeReasonCode::NoNodes,
                self.state.current_node.clone(),
                "no mesh nodes configured",
            );
        }
        if let Some(pinned) = self.state.pinned_node.clone() {
            return self.handle_pinned(nodes, &pinned, now_tick);
        }
        let Some(best) = select_best_mesh_node(nodes) else {
            return MeshNodeSwitchDecision::noop(
                MeshNodeReasonCode::AllNodesDown,
                self.state.current_node.clone(),
                "all mesh nodes are down",
            );
        };
        if !self.state.autoconnect_enabled {
            return MeshNodeSwitchDecision {
                action: MeshNodeSwitchAction::Noop,
                reason: MeshNodeReasonCode::BestSelected,
                current_node: self.state.current_node.clone(),
                candidate_node: Some(best.node_id.clone()),
                current_score: current_mesh_node(nodes, self.state.current_node.as_ref())
                    .map(|node| node.score),
                candidate_score: Some(best.score),
                allowed: false,
                explain: "autoconnect is off; best node is reported only".to_string(),
            };
        }
        let Some(current_id) = self.state.current_node.clone() else {
            return self.commit_connect(
                best,
                now_tick,
                MeshNodeReasonCode::BestSelected,
                MeshNodeSwitchAction::Connect,
            );
        };
        let Some(current) = current_mesh_node(nodes, Some(&current_id)) else {
            return self.commit_connect(
                best,
                now_tick,
                MeshNodeReasonCode::CurrentDownEmergencySwitch,
                MeshNodeSwitchAction::Switch,
            );
        };
        if current.status.is_down() {
            return self.commit_connect(
                best,
                now_tick,
                MeshNodeReasonCode::CurrentDownEmergencySwitch,
                MeshNodeSwitchAction::Switch,
            );
        }
        if current.node_id == best.node_id {
            return MeshNodeSwitchDecision {
                action: MeshNodeSwitchAction::Noop,
                reason: MeshNodeReasonCode::CurrentStillGood,
                current_node: Some(current.node_id.clone()),
                candidate_node: Some(best.node_id.clone()),
                current_score: Some(current.score),
                candidate_score: Some(best.score),
                allowed: false,
                explain: "current node is already the best node".to_string(),
            };
        }
        self.decide_switch_between(current, best, now_tick)
    }

    fn handle_pinned(
        &mut self,
        nodes: &[MeshNode],
        pinned_id: &MeshNodeId,
        now_tick: u64,
    ) -> MeshNodeSwitchDecision {
        let Some(pinned) = current_mesh_node(nodes, Some(pinned_id)) else {
            let Some(best) = select_best_mesh_node(nodes) else {
                return MeshNodeSwitchDecision::noop(
                    MeshNodeReasonCode::AllNodesDown,
                    self.state.current_node.clone(),
                    "pinned node not found and no fallback candidate",
                );
            };
            return self.commit_connect(
                best,
                now_tick,
                MeshNodeReasonCode::PinnedNodeDownEmergencySwitch,
                MeshNodeSwitchAction::Switch,
            );
        };
        if pinned.status.is_down() {
            let Some(best) = select_best_mesh_node(nodes) else {
                return MeshNodeSwitchDecision::noop(
                    MeshNodeReasonCode::AllNodesDown,
                    self.state.current_node.clone(),
                    "pinned node is down and no fallback candidate is available",
                );
            };
            return self.commit_connect(
                best,
                now_tick,
                MeshNodeReasonCode::PinnedNodeDownEmergencySwitch,
                MeshNodeSwitchAction::Switch,
            );
        }
        if self.state.current_node.as_ref() != Some(&pinned.node_id)
            && self.state.autoconnect_enabled
        {
            return self.commit_connect(
                pinned,
                now_tick,
                MeshNodeReasonCode::PinnedNodeActive,
                MeshNodeSwitchAction::Connect,
            );
        }
        MeshNodeSwitchDecision {
            action: MeshNodeSwitchAction::Noop,
            reason: MeshNodeReasonCode::PinnedNodeActive,
            current_node: self.state.current_node.clone(),
            candidate_node: Some(pinned.node_id.clone()),
            current_score: current_mesh_node(nodes, self.state.current_node.as_ref())
                .map(|node| node.score),
            candidate_score: Some(pinned.score),
            allowed: false,
            explain: "pinned node is active; autoselect suppressed".to_string(),
        }
    }

    fn decide_switch_between(
        &mut self,
        current: &MeshNode,
        candidate: &MeshNode,
        now_tick: u64,
    ) -> MeshNodeSwitchDecision {
        if candidate.observation_count < self.policy.score.thresholds.min_observation_count {
            return switch_blocked(
                current,
                candidate,
                MeshNodeReasonCode::InsufficientObservations,
                "candidate has insufficient observations",
            );
        }
        let margin = candidate.score - current.score;
        if margin < f64::from(self.policy.anti_flap.hysteresis_margin) {
            return switch_blocked(
                current,
                candidate,
                MeshNodeReasonCode::CandidateNotBetterEnough,
                "candidate is not better enough",
            );
        }
        if let Some(last_tick) = self.state.last_switch_tick {
            let elapsed = now_tick.saturating_sub(last_tick);
            if elapsed < self.policy.anti_flap.hold_down_ticks {
                return switch_blocked(
                    current,
                    candidate,
                    MeshNodeReasonCode::HoldDownActive,
                    "hold-down is active",
                );
            }
        }
        if self.state.switch_history.len() >= self.policy.anti_flap.max_switches_per_window {
            return switch_blocked(
                current,
                candidate,
                MeshNodeReasonCode::MaxSwitchRateExceeded,
                "max switch rate exceeded",
            );
        }
        self.commit_connect(
            candidate,
            now_tick,
            MeshNodeReasonCode::CandidateBetterByMargin,
            MeshNodeSwitchAction::Switch,
        )
    }

    fn commit_connect(
        &mut self,
        node: &MeshNode,
        now_tick: u64,
        reason: MeshNodeReasonCode,
        action: MeshNodeSwitchAction,
    ) -> MeshNodeSwitchDecision {
        let from = self.state.current_node.clone();
        self.state.current_node = Some(node.node_id.clone());
        self.state.last_switch_tick = Some(now_tick);
        self.state.switch_history.push_back(MeshNodeSwitchEvent {
            from: from.clone(),
            to: node.node_id.clone(),
            at_tick: now_tick,
            reason,
        });
        self.prune_switch_history(now_tick);
        MeshNodeSwitchDecision {
            action,
            reason,
            current_node: from,
            candidate_node: Some(node.node_id.clone()),
            current_score: None,
            candidate_score: Some(node.score),
            allowed: true,
            explain: format!("{reason}: selected {}", node.node_id),
        }
    }

    fn prune_switch_history(&mut self, now_tick: u64) {
        while let Some(front) = self.state.switch_history.front() {
            if now_tick.saturating_sub(front.at_tick) > self.policy.anti_flap.switch_window_ticks {
                self.state.switch_history.pop_front();
            } else {
                break;
            }
        }
    }
}

pub fn select_best_mesh_node(nodes: &[MeshNode]) -> Option<&MeshNode> {
    nodes
        .iter()
        .filter(|node| node.is_selectable())
        .min_by(|left, right| compare_mesh_nodes_for_best(left, right))
}

fn current_mesh_node<'a>(nodes: &'a [MeshNode], id: Option<&MeshNodeId>) -> Option<&'a MeshNode> {
    let id = id?;
    nodes.iter().find(|node| &node.node_id == id)
}

fn switch_blocked(
    current: &MeshNode,
    candidate: &MeshNode,
    reason: MeshNodeReasonCode,
    explain: &str,
) -> MeshNodeSwitchDecision {
    MeshNodeSwitchDecision {
        action: MeshNodeSwitchAction::Noop,
        reason,
        current_node: Some(current.node_id.clone()),
        candidate_node: Some(candidate.node_id.clone()),
        current_score: Some(current.score),
        candidate_score: Some(candidate.score),
        allowed: false,
        explain: explain.to_string(),
    }
}
