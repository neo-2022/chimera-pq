#[path = "standby_shadow_mode.rs"]
mod mode;
#[path = "standby_shadow_status.rs"]
mod status;
#[path = "standby_shadow_target.rs"]
mod target;

pub(super) use mode::{resolve_mode_from_action, standby_ready_flags, standby_stage_source};
pub(super) use status::{
    StandbyShadowDeriveInput, StandbyShadowStatus, build_standby_shadow_status,
    derive_standby_shadow_fields,
};
pub(super) use target::standby_target_for_multipath_mode;
