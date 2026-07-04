use std::fmt;

use crate::policy::MeshPeerTablePolicy;

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

#[derive(Clone, PartialEq, Eq)]
pub struct MeshConnectProbeReport {
    pub namespace: String,
    pub selected_peers: Vec<String>,
    pub connected_peer: String,
    pub connected_endpoint: String,
    pub connected_endpoint_raw: String,
    pub success: bool,
    pub attempts: Vec<MeshConnectAttempt>,
    pub explain: Vec<String>,
}

impl MeshConnectProbeReport {
    pub fn proof_endpoint(&self) -> &str {
        if self.connected_endpoint_raw.is_empty() {
            &self.connected_endpoint
        } else {
            &self.connected_endpoint_raw
        }
    }
}

impl fmt::Debug for MeshConnectProbeReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MeshConnectProbeReport")
            .field("namespace", &self.namespace)
            .field("selected_peers", &self.selected_peers)
            .field("connected_peer", &self.connected_peer)
            .field("connected_endpoint", &self.connected_endpoint)
            .field("success", &self.success)
            .field("attempts", &self.attempts)
            .field("explain", &self.explain)
            .finish()
    }
}
