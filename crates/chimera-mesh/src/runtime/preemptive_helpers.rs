#[path = "preemptive_helpers_common.rs"]
mod common;
#[path = "preemptive_helpers_guards.rs"]
mod guards;
#[path = "preemptive_helpers_hints.rs"]
mod hints;

pub(super) use common::{explain_value, tuning_source_label};
pub(super) use guards::{shadow_antiflap_meta, shadow_switch_guard_meta};
#[cfg(test)]
pub(super) use hints::format_hints_summary;
pub(super) use hints::{
    format_hints_summary_with_source, hints_reason_from_presence, hints_source_from_status,
    shadow_hints_meta_from_payload,
};

#[cfg(test)]
mod tests {
    use super::{format_hints_summary, format_hints_summary_with_source, hints_source_from_status};

    #[test]
    fn hints_source_from_status_maps_known_states() {
        assert_eq!(hints_source_from_status("unknown"), "none");
        assert_eq!(hints_source_from_status("invalid"), "invalid_payload");
        assert_eq!(hints_source_from_status("ok"), "dps_payload");
        assert_eq!(hints_source_from_status("custom"), "dps_payload");
    }

    #[test]
    fn hints_summary_with_source_wraps_base_summary() {
        let base = format_hints_summary("ok", true, "dps_payload_parsed", "flow_shard", "none");
        let with_source = format_hints_summary_with_source(
            "ok",
            true,
            "dps_payload_parsed",
            "flow_shard",
            "none",
        );
        assert_eq!(with_source, format!("{base};source=dps_payload"));
    }

    #[test]
    fn hints_summary_with_source_uses_none_and_invalid_sources() {
        let unknown = format_hints_summary_with_source(
            "unknown",
            false,
            "no_payload_context",
            "unknown",
            "unknown",
        );
        assert!(unknown.ends_with(";source=none"));

        let invalid = format_hints_summary_with_source(
            "invalid",
            false,
            "dps_payload_invalid",
            "invalid",
            "invalid",
        );
        assert!(invalid.ends_with(";source=invalid_payload"));
    }
}
