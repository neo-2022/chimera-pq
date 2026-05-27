use super::auto_recovery::windowed_peer_meta_counters;
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ShadowAntiFlapGuard {
    pub(super) blocked: bool,
    pub(super) reason: &'static str,
    pub(super) replacements_window: u64,
    pub(super) replacements_limit: u64,
}

pub(super) fn apply_shadow_antiflap_guard(
    shadow: &mut ShadowRuntimeDecision,
    peer_meta: &BTreeMap<String, MeshPeerMeta>,
    tick: u64,
    table_policy: &MeshPeerTablePolicy,
) -> ShadowAntiFlapGuard {
    let replacements_window = peer_meta
        .values()
        .map(|meta| {
            windowed_peer_meta_counters(Some(meta), tick, table_policy.stability_window_ticks)
                .replacements
        })
        .max()
        .unwrap_or(0);
    let replacements_limit = table_policy.max_replacements_per_window;
    if shadow.switch.should_switch && replacements_window >= replacements_limit {
        shadow.switch.should_switch = false;
        shadow.switch.reason = "switch_budget_exceeded".to_string();
        shadow.action = if shadow.switch.should_prepare {
            ShadowAction::KeepHotStandby
        } else {
            ShadowAction::Hold
        };
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
