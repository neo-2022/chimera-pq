use crate::dps_payload_snapshot::MeshDpsPayloadSnapshot;

#[path = "dps_payload_explain_hints.rs"]
mod hints;
#[path = "dps_payload_explain_summary.rs"]
mod summary;

pub(super) fn annotate_dps_payload_explain(
    explain: &mut Vec<String>,
    snapshot: &MeshDpsPayloadSnapshot,
    origin: &str,
) {
    const EXPLAIN_CONTRACT_VERSION: &str = "mesh_explain_v1";

    hints::remove_explain_keys(explain, hints::HINT_EXPLAIN_KEYS);
    let summary_snapshot = summary::DpsPayloadExplainSnapshot::capture(explain);

    let mut dps_lines = Vec::with_capacity(64);
    dps_lines.push(format!(
        "dps_payload_explain_contract_version={EXPLAIN_CONTRACT_VERSION}"
    ));
    dps_lines.push("policy_source=dps_payload".to_string());
    dps_lines.push(format!("dps_payload_origin={origin}"));
    dps_lines.push(format!(
        "dps_payload_mesh_field_count={}",
        snapshot.mesh_field_count()
    ));
    dps_lines.push(format!(
        "dps_payload_mesh_keys={}",
        snapshot.mesh_policy_keys_fingerprint()
    ));

    let hints = snapshot.traffic_hints();
    match hints.traffic_class {
        Some(class) => {
            dps_lines.push(format!("dps_payload_traffic_class={}", class.as_str()));
            let profile = class.starter_profile();
            dps_lines.push(format!(
                "dps_payload_traffic_profile=lat_p95:{:.1},jit_p95:{:.1},loss:{:.3},pri_warm:{:.2},pri_switch:{:.2}",
                profile.latency_p95_ms,
                profile.jitter_p95_ms,
                profile.loss_pct,
                profile.pri_warm_threshold,
                profile.pri_switch_threshold
            ));
        }
        None => dps_lines.push("dps_payload_traffic_class=none".to_string()),
    }

    hints::append_hints_ok(&mut dps_lines, &hints);
    summary::append_decision_summaries(&mut dps_lines, &summary_snapshot);
    summary::append_standby_summaries(&mut dps_lines, &summary_snapshot);
    explain.extend(dps_lines);
}
