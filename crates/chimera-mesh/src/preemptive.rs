use std::collections::{BTreeMap, BTreeSet};

use crate::model::{MeshPeerState, PreemptiveRisk, SwitchDecision};
use crate::policy::MeshPathProfile;

#[path = "preemptive_engine.rs"]
mod preemptive_engine;
#[path = "preemptive_env.rs"]
mod preemptive_env;
#[path = "preemptive_types.rs"]
mod preemptive_types;

#[allow(unused_imports)]
pub(crate) use preemptive_engine::{
    evaluate_shadow_runtime_decision, evaluate_shadow_switch_decision,
};
#[allow(unused_imports)]
pub(crate) use preemptive_env::{
    format_confirmation_tuning, shadow_pri_tuning_from_env, shadow_pri_tuning_from_kv,
};
pub(crate) use preemptive_types::{
    ShadowAction, ShadowPriTuning, ShadowPriTuningSource, ShadowRuntimeDecision,
    format_profile_tuning_thresholds, format_profile_tuning_weights, format_shadow_action,
    format_shadow_action_state, format_tuning_source, profile_min_switch_conf,
    shadow_action_priority,
};

#[allow(dead_code)]
fn _preemptive_module_contract(
    _peers: &BTreeMap<String, MeshPeerState>,
    _unhealthy: &BTreeSet<String>,
    _profile: MeshPathProfile,
    _risk: PreemptiveRisk,
    _decision: SwitchDecision,
) {
}
