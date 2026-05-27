use super::mode::{resolve_mode_from_action, standby_ready_flags, standby_stage_source};

pub(crate) struct StandbyShadowDerived {
    pub(crate) mode: String,
    pub(crate) target: String,
    pub(crate) target_source: String,
    pub(crate) reason: String,
    pub(crate) source: String,
    pub(crate) warm_ready: bool,
    pub(crate) hot_ready: bool,
    pub(crate) stage_source: String,
    pub(crate) summary: String,
}

pub(crate) struct StandbyShadowDeriveInput<'a> {
    pub(crate) mode: &'a str,
    pub(crate) target: &'a str,
    pub(crate) target_source: &'a str,
    pub(crate) reason: &'a str,
    pub(crate) source: &'a str,
    pub(crate) warm_ready: bool,
    pub(crate) hot_ready: bool,
    pub(crate) stage_source: &'a str,
}

pub(crate) struct StandbyShadowStatus {
    pub(crate) mode: String,
    pub(crate) target: String,
    pub(crate) target_source: String,
    pub(crate) reason: String,
    pub(crate) source: String,
    pub(crate) warm_ready: bool,
    pub(crate) hot_ready: bool,
    pub(crate) stage_source: String,
    pub(crate) summary: String,
}

pub(crate) fn derive_standby_shadow_fields(
    input: StandbyShadowDeriveInput<'_>,
) -> StandbyShadowDerived {
    let summary = format!(
        "mode:{};target:{};target_source:{};reason:{};source:{};warm_ready:{};hot_ready:{};{}",
        input.mode,
        input.target,
        input.target_source,
        input.reason,
        input.source,
        input.warm_ready,
        input.hot_ready,
        input.stage_source
    );
    StandbyShadowDerived {
        mode: input.mode.to_string(),
        target: input.target.to_string(),
        target_source: input.target_source.to_string(),
        reason: input.reason.to_string(),
        source: input.source.to_string(),
        warm_ready: input.warm_ready,
        hot_ready: input.hot_ready,
        stage_source: input.stage_source.to_string(),
        summary,
    }
}

pub(crate) fn build_standby_shadow_status(
    stage: &str,
    trigger: &str,
    action: &str,
    switch_target: &str,
    should_prepare: bool,
    should_switch: bool,
    switch_mode_hint: &str,
) -> StandbyShadowStatus {
    let mode = resolve_mode_from_action(action).to_string();
    let target = if switch_target.trim().is_empty() {
        "none".to_string()
    } else {
        switch_target.to_string()
    };
    let target_source = if target == "none" {
        "none".to_string()
    } else {
        "switch_target".to_string()
    };
    let reason = if target == "none" {
        "no_candidate".to_string()
    } else if should_switch {
        "switch_recommended".to_string()
    } else if should_prepare {
        "prepare_threshold".to_string()
    } else {
        "no_action".to_string()
    };
    let source = if switch_mode_hint != "unknown" {
        "dps_multipath_policy".to_string()
    } else {
        "preemptive_shadow".to_string()
    };
    let (warm_ready, hot_ready) = standby_ready_flags(Some(stage), mode.as_str(), &target);
    let stage_source = standby_stage_source(stage, trigger);
    let derived = derive_standby_shadow_fields(StandbyShadowDeriveInput {
        mode: &mode,
        target: &target,
        target_source: &target_source,
        reason: &reason,
        source: &source,
        warm_ready,
        hot_ready,
        stage_source: &stage_source,
    });
    StandbyShadowStatus {
        mode: derived.mode,
        target: derived.target,
        target_source: derived.target_source,
        reason: derived.reason,
        source: derived.source,
        warm_ready: derived.warm_ready,
        hot_ready: derived.hot_ready,
        stage_source: derived.stage_source,
        summary: derived.summary,
    }
}
