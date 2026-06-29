#[derive(Default)]
pub(in crate::runtime::dps_payload_explain) struct DpsPayloadExplainSnapshot<'a> {
    pub(in crate::runtime::dps_payload_explain::summary) switch_reason: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) confirm_n: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) confirm_hits: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) confirm_stage: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) confirm_trigger: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) pri: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) stage: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) trigger: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) switch_candidate_confidence:
        Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) switch_confidence: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) switch_confidence_gate_min:
        Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) switch_confidence_gate_passed:
        Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) switch_candidate_sample_age_ticks:
        Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) candidate_readiness_summary:
        Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) selection_pressure_summary:
        Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) selection_pressure_level: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) selection_pressure_score: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) selection_pressure_dominant:
        Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) selection_pressure_action_hint:
        Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) selection_pressure_compact:
        Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) selection_pressure_reason: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) tuning_source: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) tuning_weights: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) tuning_thresholds: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) tuning_confirmation: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) degraded_path: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) degraded_reason: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) consistency_gate: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) consistency_all_true: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) setup_compact: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) setup_compact_consistency_from_plan:
        Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) plan_setup_compact_consistency_match:
        Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) standby_mode: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) standby_target: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) standby_target_source: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) standby_reason: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) standby_source: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) standby_stage_source: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) standby_warm: Option<&'a str>,
    pub(in crate::runtime::dps_payload_explain::summary) standby_hot: Option<&'a str>,
}

impl<'a> DpsPayloadExplainSnapshot<'a> {
    pub(in crate::runtime::dps_payload_explain) fn capture(explain: &'a [String]) -> Self {
        const TRACKED_FIELD_COUNT: usize = 40;
        let mut snapshot = Self::default();
        let mut remaining = TRACKED_FIELD_COUNT;
        for line in explain {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "preemptive_shadow_switch_reason" => {
                    Self::assign(&mut snapshot.switch_reason, value, &mut remaining)
                }
                "preemptive_shadow_confirm_n" => {
                    Self::assign(&mut snapshot.confirm_n, value, &mut remaining)
                }
                "preemptive_shadow_confirm_signal_hits" => {
                    Self::assign(&mut snapshot.confirm_hits, value, &mut remaining)
                }
                "preemptive_shadow_confirm_stage" => {
                    Self::assign(&mut snapshot.confirm_stage, value, &mut remaining)
                }
                "preemptive_shadow_confirm_trigger" => {
                    Self::assign(&mut snapshot.confirm_trigger, value, &mut remaining)
                }
                "preemptive_shadow_pri" => Self::assign(&mut snapshot.pri, value, &mut remaining),
                "preemptive_shadow_stage" => {
                    Self::assign(&mut snapshot.stage, value, &mut remaining)
                }
                "preemptive_shadow_trigger" => {
                    Self::assign(&mut snapshot.trigger, value, &mut remaining)
                }
                "preemptive_shadow_switch_candidate_confidence" => Self::assign(
                    &mut snapshot.switch_candidate_confidence,
                    value,
                    &mut remaining,
                ),
                "preemptive_shadow_switch_confidence" => {
                    Self::assign(&mut snapshot.switch_confidence, value, &mut remaining)
                }
                "preemptive_shadow_switch_confidence_gate_min" => Self::assign(
                    &mut snapshot.switch_confidence_gate_min,
                    value,
                    &mut remaining,
                ),
                "preemptive_shadow_switch_confidence_gate_passed" => Self::assign(
                    &mut snapshot.switch_confidence_gate_passed,
                    value,
                    &mut remaining,
                ),
                "preemptive_shadow_switch_candidate_sample_age_ticks" => Self::assign(
                    &mut snapshot.switch_candidate_sample_age_ticks,
                    value,
                    &mut remaining,
                ),
                "preemptive_shadow_candidate_readiness_summary" => Self::assign(
                    &mut snapshot.candidate_readiness_summary,
                    value,
                    &mut remaining,
                ),
                "selection_pressure_summary" => Self::assign(
                    &mut snapshot.selection_pressure_summary,
                    value,
                    &mut remaining,
                ),
                "selection_pressure_level" => Self::assign(
                    &mut snapshot.selection_pressure_level,
                    value,
                    &mut remaining,
                ),
                "selection_pressure_score" => Self::assign(
                    &mut snapshot.selection_pressure_score,
                    value,
                    &mut remaining,
                ),
                "selection_pressure_dominant" => Self::assign(
                    &mut snapshot.selection_pressure_dominant,
                    value,
                    &mut remaining,
                ),
                "selection_pressure_action_hint" => Self::assign(
                    &mut snapshot.selection_pressure_action_hint,
                    value,
                    &mut remaining,
                ),
                "selection_pressure_compact" => Self::assign(
                    &mut snapshot.selection_pressure_compact,
                    value,
                    &mut remaining,
                ),
                "selection_pressure_reason" => Self::assign(
                    &mut snapshot.selection_pressure_reason,
                    value,
                    &mut remaining,
                ),
                "preemptive_shadow_tuning_source" => {
                    Self::assign(&mut snapshot.tuning_source, value, &mut remaining)
                }
                "preemptive_shadow_tuning_weights" => {
                    Self::assign(&mut snapshot.tuning_weights, value, &mut remaining)
                }
                "preemptive_shadow_tuning_thresholds" => {
                    Self::assign(&mut snapshot.tuning_thresholds, value, &mut remaining)
                }
                "preemptive_shadow_tuning_confirmation" => {
                    Self::assign(&mut snapshot.tuning_confirmation, value, &mut remaining)
                }
                "preemptive_shadow_degraded_path" => {
                    Self::assign(&mut snapshot.degraded_path, value, &mut remaining)
                }
                "preemptive_shadow_degraded_reason" => {
                    Self::assign(&mut snapshot.degraded_reason, value, &mut remaining)
                }
                "peer_table_runtime_consistency_gate" => {
                    Self::assign(&mut snapshot.consistency_gate, value, &mut remaining)
                }
                "peer_table_runtime_consistency_all_true" => {
                    Self::assign(&mut snapshot.consistency_all_true, value, &mut remaining)
                }
                "plan_setup_discovery_table_compact" => {
                    Self::assign(&mut snapshot.setup_compact, value, &mut remaining)
                }
                "plan_setup_discovery_table_compact_consistency" => Self::assign(
                    &mut snapshot.setup_compact_consistency_from_plan,
                    value,
                    &mut remaining,
                ),
                "plan_setup_discovery_table_compact_consistency_match" => Self::assign(
                    &mut snapshot.plan_setup_compact_consistency_match,
                    value,
                    &mut remaining,
                ),
                "standby_shadow_mode" => {
                    Self::assign(&mut snapshot.standby_mode, value, &mut remaining)
                }
                "standby_shadow_target" => {
                    Self::assign(&mut snapshot.standby_target, value, &mut remaining)
                }
                "standby_shadow_target_source" => {
                    Self::assign(&mut snapshot.standby_target_source, value, &mut remaining)
                }
                "standby_shadow_reason" => {
                    Self::assign(&mut snapshot.standby_reason, value, &mut remaining)
                }
                "standby_shadow_source" => {
                    Self::assign(&mut snapshot.standby_source, value, &mut remaining)
                }
                "standby_shadow_stage_source" => {
                    Self::assign(&mut snapshot.standby_stage_source, value, &mut remaining)
                }
                "standby_shadow_warm_ready" => {
                    Self::assign(&mut snapshot.standby_warm, value, &mut remaining)
                }
                "standby_shadow_hot_ready" => {
                    Self::assign(&mut snapshot.standby_hot, value, &mut remaining)
                }
                _ => {}
            }
            if remaining == 0 {
                break;
            }
        }
        snapshot
    }

    #[inline]
    fn assign(slot: &mut Option<&'a str>, value: &'a str, remaining: &mut usize) {
        if slot.is_none() {
            *slot = Some(value);
            *remaining = remaining.saturating_sub(1);
        }
    }
}
