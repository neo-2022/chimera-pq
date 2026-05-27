use super::status_shadow_snapshot::ShadowStatusSnapshot;

pub(super) fn preemptive_shadow_risk_summary(snapshot: &ShadowStatusSnapshot) -> String {
    format!(
        "pri={:.2};stage={};trigger={}",
        snapshot.shadow.report.risk.pri * 100.0,
        snapshot.shadow.report.stage,
        snapshot.shadow.report.trigger
    )
}

pub(super) fn preemptive_shadow_switch_guard_summary(snapshot: &ShadowStatusSnapshot) -> String {
    format!("{}|{}", snapshot.switch_guard, snapshot.switch_guard_source)
}

pub(super) fn preemptive_shadow_confirm_state(snapshot: &ShadowStatusSnapshot) -> String {
    format!(
        "hits={}/{};need={};missing={};passed={}",
        snapshot.shadow.confirmation.signal_hits,
        snapshot.shadow.confirmation.confirm_m,
        snapshot.shadow.confirmation.confirm_n,
        snapshot.confirm_missing_signals,
        snapshot.shadow.confirmation.passed
    )
}

pub(super) fn preemptive_shadow_confirm_summary(snapshot: &ShadowStatusSnapshot) -> String {
    format!(
        "hits={}/need={};stage={};trigger={}",
        snapshot.shadow.confirmation.signal_hits,
        snapshot.shadow.confirmation.confirm_n,
        snapshot.shadow.confirmation.stage,
        snapshot.shadow.confirmation.trigger
    )
}
