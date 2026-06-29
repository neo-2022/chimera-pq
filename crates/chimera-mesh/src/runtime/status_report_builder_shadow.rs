use super::status_shadow_snapshot::ShadowStatusSnapshot;
use std::fmt::Write as _;

pub(super) fn preemptive_shadow_risk_summary(snapshot: &ShadowStatusSnapshot) -> String {
    let mut out = String::with_capacity(64);
    let _ = write!(
        &mut out,
        "pri={:.2};stage={};trigger={}",
        snapshot.shadow.report.risk.pri * 100.0,
        snapshot.shadow.report.stage,
        snapshot.shadow.report.trigger
    );
    out
}

pub(super) fn preemptive_shadow_switch_guard_summary(snapshot: &ShadowStatusSnapshot) -> String {
    let mut out = String::with_capacity(
        snapshot
            .switch_guard
            .len()
            .saturating_add(snapshot.switch_guard_source.len())
            .saturating_add(1),
    );
    out.push_str(&snapshot.switch_guard);
    out.push('|');
    out.push_str(&snapshot.switch_guard_source);
    out
}

pub(super) fn preemptive_shadow_confirm_state(snapshot: &ShadowStatusSnapshot) -> String {
    let mut out = String::with_capacity(64);
    let _ = write!(
        &mut out,
        "hits={}/{};need={};missing={};passed={}",
        snapshot.shadow.confirmation.signal_hits,
        snapshot.shadow.confirmation.confirm_m,
        snapshot.shadow.confirmation.confirm_n,
        snapshot.confirm_missing_signals,
        snapshot.shadow.confirmation.passed
    );
    out
}

pub(super) fn preemptive_shadow_confirm_summary(snapshot: &ShadowStatusSnapshot) -> String {
    let mut out = String::with_capacity(64);
    let _ = write!(
        &mut out,
        "hits={}/need={};stage={};trigger={}",
        snapshot.shadow.confirmation.signal_hits,
        snapshot.shadow.confirmation.confirm_n,
        snapshot.shadow.confirmation.stage,
        snapshot.shadow.confirmation.trigger
    );
    out
}
