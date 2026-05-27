use std::collections::BTreeMap;

use crate::preemptive::{ShadowPriTuningSource, shadow_pri_tuning_from_kv};

#[test]
fn shadow_pri_tuning_from_kv_applies_confirmation_n_keys() {
    let mut kv = BTreeMap::new();
    kv.insert(
        "CHIMERA_SHADOW_PRI_CONFIRM_FAST_PREPARE_N".to_string(),
        "2".to_string(),
    );
    kv.insert(
        "CHIMERA_SHADOW_PRI_CONFIRM_FAST_HOT_N".to_string(),
        "3".to_string(),
    );
    kv.insert(
        "CHIMERA_SHADOW_PRI_CONFIRM_FAST_SWITCH_N".to_string(),
        "4".to_string(),
    );
    kv.insert(
        "CHIMERA_SHADOW_PRI_CONFIRM_FAST_HARD_N".to_string(),
        "5".to_string(),
    );
    kv.insert(
        "CHIMERA_SHADOW_PRI_CONFIRM_BALANCED_PREPARE_N".to_string(),
        "2".to_string(),
    );
    kv.insert(
        "CHIMERA_SHADOW_PRI_CONFIRM_BALANCED_HOT_N".to_string(),
        "3".to_string(),
    );
    kv.insert(
        "CHIMERA_SHADOW_PRI_CONFIRM_BALANCED_SWITCH_N".to_string(),
        "4".to_string(),
    );
    kv.insert(
        "CHIMERA_SHADOW_PRI_CONFIRM_BALANCED_HARD_N".to_string(),
        "5".to_string(),
    );
    kv.insert(
        "CHIMERA_SHADOW_PRI_CONFIRM_RESILIENT_PREPARE_N".to_string(),
        "2".to_string(),
    );
    kv.insert(
        "CHIMERA_SHADOW_PRI_CONFIRM_RESILIENT_HOT_N".to_string(),
        "3".to_string(),
    );
    kv.insert(
        "CHIMERA_SHADOW_PRI_CONFIRM_RESILIENT_SWITCH_N".to_string(),
        "4".to_string(),
    );
    kv.insert(
        "CHIMERA_SHADOW_PRI_CONFIRM_RESILIENT_HARD_N".to_string(),
        "5".to_string(),
    );
    kv.insert("CHIMERA_SHADOW_PRI_CONFIRM_M".to_string(), "6".to_string());

    let tuning = shadow_pri_tuning_from_kv(|key| kv.get(key).cloned());
    assert_eq!(tuning.source, ShadowPriTuningSource::Env);
    assert_eq!(tuning.confirm_fast_prepare_n, 2);
    assert_eq!(tuning.confirm_fast_hot_n, 3);
    assert_eq!(tuning.confirm_fast_switch_n, 4);
    assert_eq!(tuning.confirm_fast_hard_n, 5);
    assert_eq!(tuning.confirm_balanced_prepare_n, 2);
    assert_eq!(tuning.confirm_balanced_hot_n, 3);
    assert_eq!(tuning.confirm_balanced_switch_n, 4);
    assert_eq!(tuning.confirm_balanced_hard_n, 5);
    assert_eq!(tuning.confirm_resilient_prepare_n, 2);
    assert_eq!(tuning.confirm_resilient_hot_n, 3);
    assert_eq!(tuning.confirm_resilient_switch_n, 4);
    assert_eq!(tuning.confirm_resilient_hard_n, 5);
    assert_eq!(tuning.confirm_m, 6);
}

#[test]
fn shadow_pri_tuning_from_kv_sanitizes_confirmation_n_by_confirm_m() {
    let mut kv = BTreeMap::new();
    kv.insert(
        "CHIMERA_SHADOW_PRI_CONFIRM_FAST_PREPARE_N".to_string(),
        "9".to_string(),
    );
    kv.insert(
        "CHIMERA_SHADOW_PRI_CONFIRM_BALANCED_HARD_N".to_string(),
        "8".to_string(),
    );
    kv.insert(
        "CHIMERA_SHADOW_PRI_CONFIRM_RESILIENT_SWITCH_N".to_string(),
        "7".to_string(),
    );
    kv.insert("CHIMERA_SHADOW_PRI_CONFIRM_M".to_string(), "2".to_string());

    let tuning = shadow_pri_tuning_from_kv(|key| kv.get(key).cloned());
    assert_eq!(tuning.confirm_m, 2);
    assert_eq!(tuning.confirm_fast_prepare_n, 2);
    assert_eq!(tuning.confirm_balanced_hard_n, 2);
    assert_eq!(tuning.confirm_resilient_switch_n, 2);
}
