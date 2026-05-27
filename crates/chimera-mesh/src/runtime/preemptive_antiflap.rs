use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ShadowAntiFlapGuard {
    pub(super) blocked: bool,
    pub(super) reason: &'static str,
    pub(super) replacements_window: u64,
    pub(super) replacements_limit: u64,
}

pub(super) fn apply_preemptive_antiflap(
    shadow: &mut crate::preemptive::ShadowRuntimeDecision,
    peer_meta: &BTreeMap<String, MeshPeerMeta>,
    tick: u64,
    table_policy: &MeshPeerTablePolicy,
) -> ShadowAntiFlapGuard {
    let replacements_window = peer_meta
        .values()
        .map(|meta| {
            let age_since_seen = tick.saturating_sub(meta.last_seen_tick);
            if age_since_seen <= table_policy.stability_window_ticks {
                meta.replacement_events
            } else {
                0
            }
        })
        .max()
        .unwrap_or(0);
    let replacements_limit = table_policy.max_replacements_per_window;
    if shadow.switch.should_switch && replacements_window >= replacements_limit {
        shadow.switch.should_switch = false;
        shadow.switch.reason = "switch_budget_exceeded".to_string();
        shadow.action = crate::preemptive::ShadowAction::KeepHotStandby;
        shadow.action_reason = "switch_budget_exceeded";
        return ShadowAntiFlapGuard {
            blocked: true,
            reason: "switch_budget_exceeded",
            replacements_window,
            replacements_limit,
        };
    }
    ShadowAntiFlapGuard {
        blocked: false,
        reason: "none",
        replacements_window,
        replacements_limit,
    }
}
