pub(crate) fn shadow_switch_guard_meta(reason: &str) -> (&'static str, &'static str) {
    match reason {
        "candidate_low_confidence" => ("confidence_guard", "switch_confidence_gate"),
        "confirmation_gate_blocked" => ("confirmation_guard", "confirmation_gate"),
        "no_candidate" | "no_candidate_for_switch" => ("candidate_guard", "candidate_selection"),
        "switch_budget_exceeded" => ("antiflap_guard", "switch_budget_gate"),
        _ => ("none", "none"),
    }
}

pub(crate) fn shadow_antiflap_meta(reason: &str) -> (bool, &'static str) {
    match reason {
        "switch_budget_exceeded" => (true, "switch_budget_exceeded"),
        _ => (false, "none"),
    }
}
