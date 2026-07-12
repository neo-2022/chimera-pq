use std::collections::{BTreeMap, BTreeSet};

use crate::model::{
    MeshDiscoveryRecord, MeshFailoverEvent, MeshJoinMode, MeshJoinRequest, MeshPathPlan,
    MeshPathPlanCore, MeshPeerHealth, MeshPeerPerformance, MeshPeerState,
    MeshPublishedEndpointUpdate, peer_priority,
};
use crate::multipath_model::{MeshMultipathMode, MeshMultipathSchedule};
use crate::route_announcement::RouteAnnouncement;
use crate::policy::{MeshPathPolicy, MeshPathProfile, MeshPeerTablePolicy, MultipathMode};
use crate::preemptive::{
    evaluate_shadow_runtime_decision, format_confirmation_tuning, format_profile_tuning_thresholds,
    format_profile_tuning_weights, format_shadow_action, format_shadow_action_state,
    shadow_action_priority, shadow_pri_tuning_from_env,
};
mod candidate_filter;
mod connect_probe;
mod connect_retry_profile;
mod diagnostic_redaction;
mod dps_payload_explain;
mod health_state_utils;
mod join_mode;
mod multipath_aggregate;
#[cfg(test)]
mod multipath_aggregate_tests;
mod multipath_demand;
mod multipath_flow;
mod multipath_lane_admission;
mod multipath_rebuild_bridge;
mod multipath_rebuild_control;
mod multipath_rebuild_model;
mod multipath_rebuild_trigger;
mod multipath_schedule;
#[cfg(test)]
mod multipath_schedule_tests;
mod multipath_weights;
mod path_planner;
mod path_planner_finalize;
mod path_planner_recovery;
mod path_planner_recovery_explain;
mod path_planner_selection_explain;
mod path_planner_selection_metrics;
mod path_planner_setup;
mod payload_utils;
mod peer_discovery;
mod peer_endpoint_update;
mod peer_health_lifecycle;
mod peer_maintenance;
mod peer_performance;
mod plan_dps_adaptation;
mod plan_ops;
mod preemptive_antiflap;
mod preemptive_helpers;
mod preemptive_shadow_eval;
mod preemptive_shadow_explain;
mod preemptive_status;
mod preemptive_status_lines;
mod reports;
mod selection_policy;
mod selection_profile;
mod standby_shadow;
mod standby_shadow_explain;
mod standby_status_lines;
mod status_base_explain;
mod status_report_builder;
mod status_runtime;
mod status_shadow_snapshot;
mod table_consistency;
use candidate_filter::collect_candidates;
use dps_payload_explain::annotate_dps_payload_explain;
pub use join_mode::evaluate_join_mode;
pub use multipath_aggregate::{
    MeshMultipathAggregateAction, MeshMultipathAggregatePlan, MeshMultipathAggregateShard,
    plan_multipath_aggregate_object,
};
pub use multipath_flow::{
    MeshMultipathFlowAction, MeshMultipathFlowDecision, MeshMultipathFlowKey,
    MeshMultipathFlowPlan, plan_multipath_flow, plan_multipath_flow_decision,
};
use multipath_rebuild_control::MeshMultipathRebuildState;
pub use multipath_rebuild_model::{
    MeshMultipathRebuildAction, MeshMultipathRebuildDecision, MeshMultipathRebuildDirtyMetadata,
    MeshMultipathRebuildDirtyScope, MeshMultipathRebuildPolicy, MeshMultipathRebuildSignal,
    MeshMultipathRebuildSignalInput, MeshMultipathRebuildUrgency,
};
use multipath_rebuild_trigger::MeshMultipathRebuildTriggerCause;
use multipath_schedule::{
    build_multipath_schedule, replace_multipath_schedule, replace_multipath_schedule_core,
    schedule_mode_from_multipath_hint,
};
pub use reports::{
    MeshConnectAttempt, MeshConnectProbeReport, MeshPeerTableEnforcementReport,
    MeshRuntimeStatusReport,
};
use selection_policy::{normalize_region_key, validate_runtime_node_id, validate_source_name};
use selection_profile::{
    effective_target_distinct_regions, profile_label, resolve_path_profile,
    runtime_peer_signal_averages,
};
use status_runtime::{build_status_explain, build_status_report};

#[derive(Clone, PartialEq, Eq)]
struct MeshPeerMeta {
    identity_marker: u64,
    last_seen_tick: u64,
    update_events: u64,
    replacement_events: u64,
    hold_events: u64,
    degraded_events: u64,
    churn_block_events: u64,
    threshold_block_events: u64,
    last_effective_replacement_threshold: i32,
    endpoint_generation: u64,
    update_bootstrap_url: Option<String>,
}

impl std::fmt::Debug for MeshPeerMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeshPeerMeta")
            .field("identity_marker", &"<redacted>")
            .field("last_seen_tick", &self.last_seen_tick)
            .field("update_events", &self.update_events)
            .field("replacement_events", &self.replacement_events)
            .field("hold_events", &self.hold_events)
            .field("degraded_events", &self.degraded_events)
            .field("churn_block_events", &self.churn_block_events)
            .field("threshold_block_events", &self.threshold_block_events)
            .field(
                "last_effective_replacement_threshold",
                &self.last_effective_replacement_threshold,
            )
            .field("endpoint_generation", &self.endpoint_generation)
            .field(
                "update_bootstrap_url",
                &self.update_bootstrap_url.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MeshHealthMeta {
    health: MeshPeerHealth,
    last_updated_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MeshProfileState {
    active_profile: MeshPathProfile,
    degrade_cleared_since_tick: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CandidateStats {
    rejected_blocked: usize,
    rejected_health: usize,
    rejected_region: usize,
    rejected_reliability: usize,
    rejected_load: usize,
    accepted_count: usize,
}

#[derive(Clone)]
struct CandidateFilter<'a> {
    blocked: &'a BTreeSet<&'a str>,
    health_blocked: &'a BTreeSet<&'a str>,
    allowed_regions: &'a BTreeSet<String>,
    min_reliability: u8,
    max_load: u8,
    profile: MeshPathProfile,
}

#[derive(Clone)]
struct CandidateSlot<'a> {
    peer: &'a MeshPeerState,
    normalized_region: String,
    selection_score: i32,
}

impl std::fmt::Debug for CandidateFilter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CandidateFilter")
            .field("blocked_count", &self.blocked.len())
            .field("health_blocked_count", &self.health_blocked.len())
            .field("allowed_region_count", &self.allowed_regions.len())
            .field("min_reliability", &self.min_reliability)
            .field("max_load", &self.max_load)
            .field("profile", &self.profile)
            .finish()
    }
}

impl std::fmt::Debug for CandidateSlot<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CandidateSlot")
            .field("peer", &"<redacted>")
            .field("normalized_region", &self.normalized_region)
            .field("selection_score", &self.selection_score)
            .finish()
    }
}

impl CandidateStats {
    fn rejected_total(self) -> usize {
        self.rejected_blocked
            .saturating_add(self.rejected_health)
            .saturating_add(self.rejected_region)
            .saturating_add(self.rejected_reliability)
            .saturating_add(self.rejected_load)
    }
}

impl CandidateSlot<'_> {
    fn materialize_peer(&self) -> MeshPeerState {
        let mut peer = self.peer.clone();
        peer.selection_score = self.selection_score;
        peer
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct MeshRuntime {
    namespace: String,
    peers: BTreeMap<String, MeshPeerState>,
    peer_meta: BTreeMap<String, MeshPeerMeta>,
    sources: BTreeSet<String>,
    health_state: BTreeMap<String, MeshHealthMeta>,
    table_policy: MeshPeerTablePolicy,
    profile_state: MeshProfileState,
    multipath_rebuild_state: MeshMultipathRebuildState,
    pending_multipath_rebuild: Option<MeshMultipathRebuildSignal>,
    runtime_announcements: Vec<RouteAnnouncement>,
    next_peer_identity_marker: u64,
    last_table_enforcement_report: MeshPeerTableEnforcementReport,
    tick: u64,
}

impl std::fmt::Debug for MeshRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeshRuntime")
            .field("namespace", &"<redacted>")
            .field("source_count", &self.sources.len())
            .field("peer_count", &self.peers.len())
            .field("health_state_count", &self.health_state.len())
            .field("table_policy", &self.table_policy)
            .field("profile_state", &self.profile_state)
            .field("multipath_rebuild_state", &self.multipath_rebuild_state)
            .field("pending_multipath_rebuild", &self.pending_multipath_rebuild)
            .field(
                "last_table_enforcement_report",
                &self.last_table_enforcement_report,
            )
            .field("tick", &self.tick)
            .finish()
    }
}

impl MeshRuntime {
    pub fn bootstrap(namespace: &str, source: &str) -> Result<Self, String> {
        let namespace = namespace.trim();
        let source = source.trim();
        if namespace.is_empty() {
            return Err("mesh runtime namespace is empty".to_string());
        }
        validate_source_name(source, "mesh runtime bootstrap source")?;
        let mut sources = BTreeSet::new();
        sources.insert(source.to_string());
        Ok(Self {
            namespace: namespace.to_string(),
            peers: BTreeMap::new(),
            peer_meta: BTreeMap::new(),
            sources,
            health_state: BTreeMap::new(),
            table_policy: MeshPeerTablePolicy::default(),
            profile_state: MeshProfileState {
                active_profile: MeshPathProfile::Balanced,
                degrade_cleared_since_tick: None,
            },
            multipath_rebuild_state: MeshMultipathRebuildState::default(),
            pending_multipath_rebuild: None,
            runtime_announcements: Vec::new(),
            next_peer_identity_marker: 1,
            last_table_enforcement_report: MeshPeerTableEnforcementReport {
                tick: 0,
                total_peers_before: 0,
                total_peers_after: 0,
                dropped_total: 0,
                dropped_by_region_cap: 0,
                dropped_by_global_cap: 0,
                protected_region_skips: 0,
                effective_profile: "balanced".to_string(),
                effective_target_distinct_regions: 0,
                effective_target_source: "boot".to_string(),
            },
            tick: 0,
        })
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn source_count(&self) -> usize {
        self.sources.len()
    }

    fn remember_source(&mut self, source: &str) {
        if !self.sources.contains(source) {
            self.sources.insert(source.to_string());
        }
    }

    pub fn source_list(&self) -> Vec<String> {
        self.sources.iter().cloned().collect()
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    pub fn peer_snapshot(&self) -> Vec<MeshPeerState> {
        self.peers.values().cloned().collect()
    }

    pub fn runtime_announcements(&self) -> &[RouteAnnouncement] {
        &self.runtime_announcements
    }

    pub fn merge_runtime_announcements(
        &mut self,
        source: &str,
        announcements: &[RouteAnnouncement],
    ) -> Result<bool, String> {
        if announcements.is_empty() {
            return Ok(false);
        }
        self.remember_source(source);
        let now = std::time::SystemTime::now();
        let mut keys: std::collections::BTreeSet<String> = self
            .runtime_announcements
            .iter()
            .map(runtime_announcement_key)
            .collect();
        let before = self.rebuild_trigger_fingerprint();
        let mut added = 0usize;
        for announcement in announcements {
            if announcement.is_expired(now) {
                continue;
            }
            let key = runtime_announcement_key(announcement);
            if keys.insert(key) {
                self.runtime_announcements.push(announcement.clone());
                added = added.saturating_add(1);
            }
        }
        if added == 0 {
            return Ok(false);
        }
        self.mark_pending_multipath_rebuild_with_dirty_scope(
            MeshMultipathRebuildTriggerCause::RuntimeAnnouncementsChanged,
            before,
            MeshMultipathRebuildDirtyScope::RuntimeAnnouncements,
            added,
        )?;
        Ok(true)
    }

    pub fn health_state_count(&self) -> usize {
        self.health_state.len()
    }

    pub fn health_snapshot(&self) -> Vec<MeshPeerHealth> {
        self.health_state
            .values()
            .map(|meta| meta.health.clone())
            .collect()
    }

    pub fn peer_table_last_enforcement_report(&self) -> MeshPeerTableEnforcementReport {
        self.last_table_enforcement_report.clone()
    }

    fn region_distribution_counts(&self) -> BTreeMap<String, usize> {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for peer in self.peers.values() {
            *counts
                .entry(normalize_region_key(&peer.region))
                .or_insert(0) += 1;
        }
        counts
    }

    pub fn region_distribution(&self) -> Vec<(String, usize)> {
        self.region_distribution_counts().into_iter().collect()
    }

    pub fn set_peer_table_policy(&mut self, policy: MeshPeerTablePolicy) -> Result<(), String> {
        policy.validate()?;
        self.table_policy = policy;
        self.enforce_peer_table_limits();
        Ok(())
    }

    pub fn set_peer_table_policy_from_dps_payload(&mut self, payload: &str) -> Result<(), String> {
        let policy = MeshPeerTablePolicy::from_dps_payload(payload)?;
        self.set_peer_table_policy(policy)
    }

    pub fn peer_table_policy_snapshot(&self) -> MeshPeerTablePolicy {
        self.table_policy.clone()
    }

    pub(super) fn allocate_peer_identity_marker(&mut self) -> u64 {
        let marker = self.next_peer_identity_marker;
        self.next_peer_identity_marker = self.next_peer_identity_marker.saturating_add(1);
        marker
    }

    pub fn status_report(&self) -> MeshRuntimeStatusReport {
        self.status_report_with_optional_dps_payload(None)
    }

    pub fn status_report_with_dps_payload(&self, payload: &str) -> MeshRuntimeStatusReport {
        self.status_report_with_optional_dps_payload(Some(payload))
    }

    fn status_report_with_optional_dps_payload(
        &self,
        payload: Option<&str>,
    ) -> MeshRuntimeStatusReport {
        build_status_report(self, payload)
    }

    pub fn status_explain(&self) -> Vec<String> {
        let report = self.status_report();
        self.status_explain_from_report(&report)
    }

    pub fn status_explain_with_dps_payload(&self, payload: &str) -> Vec<String> {
        let report = self.status_report_with_dps_payload(payload);
        self.status_explain_from_report(&report)
    }

    fn status_explain_from_report(&self, report: &MeshRuntimeStatusReport) -> Vec<String> {
        build_status_explain(self, report)
    }

    pub(super) fn rebuild_plan_snapshot_from_runtime_state(
        &self,
        join_mode: MeshJoinMode,
        policy: &MeshPathPolicy,
    ) -> Result<MeshPathPlan, String> {
        path_planner::build_plan_from_runtime_state(self, join_mode, policy)
    }

    pub(super) fn rebuild_plan_snapshot_core_from_runtime_state(
        &self,
        join_mode: MeshJoinMode,
        policy: &MeshPathPolicy,
    ) -> Result<MeshPathPlanCore, String> {
        path_planner::build_plan_core_from_runtime_state(self, join_mode, policy)
    }
}

fn runtime_announcement_key(announcement: &RouteAnnouncement) -> String {
    format!(
        "{}|{}|{}",
        announcement.destination().to_wire_string(),
        announcement.via().as_str(),
        announcement.route_binding_id().get()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_announcements_create_transit_carrier_binding_in_plan() -> Result<(), String> {
        let mut runtime = MeshRuntime::bootstrap("stand", "test")?;
        runtime.merge_discovery(
            "test-discovery",
            &[crate::model::MeshDiscoveryRecord {
                node_id: "vdsina".to_string(),
                endpoint: "198.51.100.1:443".to_string(),
                region: "ru".to_string(),
                load_score: 0,
                reliability_score: 100,
            }],
        )?;

        let announcements = crate::route_announcement::parse_route_announcements(
            "static,cidr/127.0.0.1/32,vdsina,3600,11",
        )?;
        let changed = runtime.merge_runtime_announcements("test-peer", &announcements)?;
        assert!(changed, "registry should change after first merge");

        let request = crate::model::MeshJoinRequest {
            namespace: "stand".to_string(),
            node_name: "amai".to_string(),
            invite_token: None,
        };
        let payload = concat!(
            "allow=mesh;",
            "mesh_multipath_mode=off;",
            "mesh_route_binding_id=11;",
            "mesh_max_peers=1;",
            "mesh_max_selected_per_region=1"
        );

        let plan = runtime.plan_path_from_dps_payload_with_announcements(
            &request,
            payload,
            runtime.runtime_announcements(),
        )?;

        assert_eq!(plan.multipath_schedule.active_lane_count, 1);
        assert_eq!(plan.multipath_schedule.carrier_lane_bindings.len(), 1);
        assert_eq!(
            plan.multipath_schedule.carrier_lane_bindings[0].peer_node_id,
            "vdsina"
        );
        assert_eq!(
            plan.multipath_schedule.execution_status,
            "carrier_lane_binding_contract_ready"
        );
        Ok(())
    }
}
