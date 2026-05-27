use std::collections::{BTreeMap, BTreeSet};

use crate::model::{
    MeshDiscoveryRecord, MeshFailoverEvent, MeshJoinMode, MeshJoinRequest, MeshPathPlan,
    MeshPeerHealth, MeshPeerState, peer_priority,
};
use crate::policy::{
    MeshPathPolicy, MeshPathProfile, MeshPeerTablePolicy, traffic_class_from_dps_payload,
    traffic_hints_from_dps_payload,
};
use crate::preemptive::{
    evaluate_shadow_runtime_decision, format_confirmation_tuning, format_profile_tuning_thresholds,
    format_profile_tuning_weights, format_shadow_action, format_shadow_action_state,
    shadow_action_priority, shadow_pri_tuning_from_env,
};
mod candidate_filter;
mod connect_probe;
mod connect_retry_profile;
mod dps_payload_explain;
mod health_state_utils;
mod join_mode;
mod path_planner;
mod path_planner_finalize;
mod path_planner_recovery;
mod path_planner_recovery_explain;
mod path_planner_selection_explain;
mod path_planner_selection_metrics;
mod path_planner_setup;
mod payload_utils;
mod peer_discovery;
mod peer_health_lifecycle;
mod peer_maintenance;
mod plan_dps_adaptation;
mod plan_ops;
mod preemptive_antiflap;
mod preemptive_helpers;
mod preemptive_shadow_eval;
mod preemptive_shadow_explain;
mod preemptive_status;
mod preemptive_status_lines;
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
use selection_policy::{normalize_region_key, validate_runtime_node_id, validate_source_name};
use selection_profile::{
    effective_target_distinct_regions, profile_label, resolve_path_profile,
    runtime_peer_signal_averages,
};
use status_runtime::{build_status_explain, build_status_report};

#[derive(Debug, Clone, PartialEq, Eq)]
struct MeshPeerMeta {
    last_seen_tick: u64,
    update_events: u64,
    replacement_events: u64,
    hold_events: u64,
    degraded_events: u64,
    churn_block_events: u64,
    threshold_block_events: u64,
    last_effective_replacement_threshold: i32,
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

#[derive(Debug, Clone)]
struct CandidateFilter<'a> {
    blocked: &'a BTreeSet<&'a str>,
    health_blocked: &'a BTreeSet<&'a str>,
    allowed_regions: &'a BTreeSet<String>,
    min_reliability: u8,
    max_load: u8,
    profile: MeshPathProfile,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshRuntime {
    namespace: String,
    peers: BTreeMap<String, MeshPeerState>,
    peer_meta: BTreeMap<String, MeshPeerMeta>,
    sources: BTreeSet<String>,
    health_state: BTreeMap<String, MeshHealthMeta>,
    table_policy: MeshPeerTablePolicy,
    profile_state: MeshProfileState,
    last_table_enforcement_report: MeshPeerTableEnforcementReport,
    tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshPeerTableEnforcementReport {
    pub tick: u64,
    pub total_peers_before: usize,
    pub total_peers_after: usize,
    pub dropped_total: usize,
    pub dropped_by_region_cap: usize,
    pub dropped_by_global_cap: usize,
    pub protected_region_skips: usize,
    pub effective_profile: String,
    pub effective_target_distinct_regions: usize,
    pub effective_target_source: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshRuntimeStatusReport {
    pub namespace: String,
    pub source_count: usize,
    pub peer_count: usize,
    pub health_state_count: usize,
    pub active_profile: String,
    pub table_policy: MeshPeerTablePolicy,
    pub table_enforcement: MeshPeerTableEnforcementReport,
    pub preemptive_shadow_pri: f32,
    pub preemptive_shadow_instant_risk: f32,
    pub preemptive_shadow_trend_risk: f32,
    pub preemptive_shadow_stage: String,
    pub preemptive_shadow_trigger: String,
    pub preemptive_shadow_risk_summary: String,
    pub preemptive_shadow_switch_prepare: bool,
    pub preemptive_shadow_switch_recommend: bool,
    pub preemptive_shadow_switch_reason: String,
    pub preemptive_shadow_switch_guard: String,
    pub preemptive_shadow_switch_guard_source: String,
    pub preemptive_shadow_switch_guard_summary: String,
    pub preemptive_shadow_switch_confidence: f64,
    pub preemptive_shadow_switch_candidate_confidence: f64,
    pub preemptive_shadow_switch_confidence_gate_min: f64,
    pub preemptive_shadow_switch_confidence_gate_passed: bool,
    pub preemptive_shadow_switch_candidate_sample_age_ticks: String,
    pub preemptive_shadow_switch_target: String,
    pub preemptive_shadow_switch_mode: String,
    pub preemptive_shadow_hints_status: String,
    pub preemptive_shadow_hints_source: String,
    pub preemptive_shadow_hints_reason: String,
    pub preemptive_shadow_hints_present: bool,
    pub preemptive_shadow_hints_multipath_mode: String,
    pub preemptive_shadow_hints_continuity_policy: String,
    pub preemptive_shadow_hints_summary: String,
    pub preemptive_shadow_action: String,
    pub preemptive_shadow_action_reason: String,
    pub preemptive_shadow_action_state: String,
    pub preemptive_shadow_action_priority: u8,
    pub preemptive_shadow_confirm_passed: bool,
    pub preemptive_shadow_confirm_n: u8,
    pub preemptive_shadow_confirm_m: u8,
    pub preemptive_shadow_confirm_signal_hits: u8,
    pub preemptive_shadow_confirm_ratio: f32,
    pub preemptive_shadow_confirm_missing_signals: u8,
    pub preemptive_shadow_confirm_state: String,
    pub preemptive_shadow_confirm_signal_labels: String,
    pub preemptive_shadow_confirm_stage: String,
    pub preemptive_shadow_confirm_trigger: String,
    pub preemptive_shadow_confirm_summary: String,
    pub preemptive_shadow_risk_valid: bool,
    pub preemptive_shadow_switch_valid: bool,
    pub preemptive_shadow_eligible_candidates: usize,
    pub preemptive_shadow_health_blocked_count: usize,
    pub preemptive_shadow_antiflap_blocked: bool,
    pub preemptive_shadow_antiflap_reason: String,
    pub preemptive_shadow_antiflap_replacements_window: u64,
    pub preemptive_shadow_antiflap_replacements_limit: u64,
    pub preemptive_shadow_tuning_source: String,
    pub preemptive_shadow_tuning_confirmation: String,
    pub preemptive_shadow_tuning_weights: String,
    pub preemptive_shadow_tuning_thresholds: String,
    pub table_runtime_consistency_all_true: bool,
    pub table_runtime_consistency_gate: String,
    pub table_runtime_consistency_summary: String,
    pub plan_setup_discovery_table_compact: String,
    pub plan_setup_discovery_table_compact_consistency: String,
    pub setup_compact_consistency_match: bool,
    pub setup_compact_consistency_match_source: String,
    pub plan_setup_compact_consistency_match_source: String,
    pub preemptive_shadow_degraded_path: bool,
    pub preemptive_shadow_degraded_reason: String,
    pub preemptive_shadow_degraded_summary: String,
    pub standby_shadow_mode: String,
    pub standby_shadow_target: String,
    pub standby_shadow_target_source: String,
    pub standby_shadow_reason: String,
    pub standby_shadow_source: String,
    pub standby_shadow_warm_ready: bool,
    pub standby_shadow_hot_ready: bool,
    pub standby_shadow_stage_source: String,
    pub standby_shadow_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshConnectAttempt {
    pub peer_id: String,
    pub endpoint: String,
    pub success: bool,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshConnectProbeReport {
    pub namespace: String,
    pub selected_peers: Vec<String>,
    pub connected_peer: String,
    pub connected_endpoint: String,
    pub success: bool,
    pub attempts: Vec<MeshConnectAttempt>,
    pub explain: Vec<String>,
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

    pub fn source_list(&self) -> Vec<String> {
        self.sources.iter().cloned().collect()
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    pub fn peer_snapshot(&self) -> Vec<MeshPeerState> {
        self.peers.values().cloned().collect()
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

    pub fn region_distribution(&self) -> Vec<(String, usize)> {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for peer in self.peers.values() {
            *counts
                .entry(normalize_region_key(&peer.region))
                .or_insert(0) += 1;
        }
        counts.into_iter().collect()
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
}
