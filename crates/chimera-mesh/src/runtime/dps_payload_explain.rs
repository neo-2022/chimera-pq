use super::payload_utils::{count_mesh_policy_fields, mesh_policy_keys_fingerprint};
use super::*;

#[path = "dps_payload_explain_hints.rs"]
mod hints;
#[path = "dps_payload_explain_summary.rs"]
mod summary;

pub(super) fn annotate_dps_payload_explain(explain: &mut Vec<String>, payload: &str, origin: &str) {
    const EXPLAIN_CONTRACT_VERSION: &str = "mesh_explain_v1";
    explain.push(format!(
        "dps_payload_explain_contract_version={EXPLAIN_CONTRACT_VERSION}"
    ));
    explain.push("policy_source=dps_payload".to_string());
    explain.push(format!("dps_payload_origin={origin}"));
    explain.push(format!(
        "dps_payload_mesh_field_count={}",
        count_mesh_policy_fields(payload)
    ));
    explain.push(format!(
        "dps_payload_mesh_keys={}",
        mesh_policy_keys_fingerprint(payload)
    ));

    match traffic_class_from_dps_payload(payload) {
        Ok(Some(class)) => {
            explain.push(format!("dps_payload_traffic_class={}", class.as_str()));
            let profile = class.starter_profile();
            explain.push(format!(
                "dps_payload_traffic_profile=lat_p95:{:.1},jit_p95:{:.1},loss:{:.3},pri_warm:{:.2},pri_switch:{:.2}",
                profile.latency_p95_ms,
                profile.jitter_p95_ms,
                profile.loss_pct,
                profile.pri_warm_threshold,
                profile.pri_switch_threshold
            ));
        }
        Ok(None) => explain.push("dps_payload_traffic_class=none".to_string()),
        Err(_) => explain.push("dps_payload_traffic_class=invalid".to_string()),
    }

    match traffic_hints_from_dps_payload(payload) {
        Ok(parsed_hints) => {
            hints::remove_explain_keys(explain, hints::HINT_EXPLAIN_KEYS);
            hints::append_hints_ok(explain, &parsed_hints);
        }
        Err(_) => {
            hints::remove_explain_keys(explain, hints::HINT_EXPLAIN_KEYS);
            hints::append_hints_invalid(explain);
        }
    }

    summary::append_decision_summaries(explain);
    summary::append_standby_summaries(explain);
}
