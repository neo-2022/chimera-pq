use crate::{
    TrafficClass, continuity_policy_from_dps_payload, multipath_demand_from_dps_payload,
    multipath_mode_from_dps_payload, traffic_class_from_dps_payload,
    traffic_hints_from_dps_payload,
};

#[test]
fn traffic_class_from_dps_payload_parses_valid_values() {
    let value = traffic_class_from_dps_payload("mesh_traffic_class=gaming_fps")
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(value, Some(TrafficClass::GamingFps));

    let value = traffic_class_from_dps_payload("allow=mesh;mesh_traffic_class=BUFFERED_STREAMING")
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(value, Some(TrafficClass::BufferedStreaming));

    let value = traffic_class_from_dps_payload("mesh_traffic_class=bulk_download")
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(value, Some(TrafficClass::BulkTransfer));

    let value = traffic_class_from_dps_payload("mesh_traffic_class=dns")
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(value, Some(TrafficClass::ControlDns));
}

#[test]
fn traffic_class_from_dps_payload_rejects_invalid_or_duplicate_values() {
    assert!(traffic_class_from_dps_payload("mesh_traffic_class=").is_err());
    assert!(traffic_class_from_dps_payload("mesh_traffic_class=unknown_class").is_err());
    assert!(
        traffic_class_from_dps_payload(
            "mesh_traffic_class=gaming_fps;MESH_TRAFFIC_CLASS=bulk_download"
        )
        .is_err()
    );
}

#[test]
fn multipath_mode_from_dps_payload_parses_valid_values() {
    let value = multipath_mode_from_dps_payload("mesh_multipath_mode=flow_shard")
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(value.map(|v| v.as_str()), Some("flow_shard"));

    let value =
        multipath_mode_from_dps_payload("allow=mesh;mesh_multipath_mode=AGGREGATE_BUFFERED")
            .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(value.map(|v| v.as_str()), Some("aggregate_buffered"));
}

#[test]
fn multipath_mode_from_dps_payload_rejects_invalid_or_duplicate_values() {
    assert!(multipath_mode_from_dps_payload("mesh_multipath_mode=").is_err());
    assert!(multipath_mode_from_dps_payload("mesh_multipath_mode=invalid_mode").is_err());
    assert!(
        multipath_mode_from_dps_payload("mesh_multipath_mode=off;MESH_MULTIPATH_MODE=flow_shard")
            .is_err()
    );
}

#[test]
fn multipath_demand_from_dps_payload_parses_valid_values() {
    let value = multipath_demand_from_dps_payload("mesh_multipath_demand=high")
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(value.map(|v| v.as_str()), Some("high"));

    let value = multipath_demand_from_dps_payload("allow=mesh;mesh_multipath_demand=BULK")
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(value.map(|v| v.as_str()), Some("bulk"));
}

#[test]
fn multipath_demand_from_dps_payload_rejects_invalid_or_duplicate_values() {
    assert!(multipath_demand_from_dps_payload("mesh_multipath_demand=").is_err());
    assert!(multipath_demand_from_dps_payload("mesh_multipath_demand=100mbps").is_err());
    assert!(
        multipath_demand_from_dps_payload("mesh_multipath_demand=low;MESH_MULTIPATH_DEMAND=bulk")
            .is_err()
    );
}

#[test]
fn continuity_policy_from_dps_payload_parses_valid_values() {
    let value = continuity_policy_from_dps_payload("mesh_continuity_policy=allow_flow_drain")
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(value.map(|v| v.as_str()), Some("allow_flow_drain"));

    let value = continuity_policy_from_dps_payload(
        "allow=mesh;mesh_continuity_policy=ALLOW_HARD_REBIND_ONLY",
    )
    .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(value.map(|v| v.as_str()), Some("allow_hard_rebind_only"));
}

#[test]
fn continuity_policy_from_dps_payload_rejects_invalid_or_duplicate_values() {
    assert!(continuity_policy_from_dps_payload("mesh_continuity_policy=").is_err());
    assert!(continuity_policy_from_dps_payload("mesh_continuity_policy=unknown_policy").is_err());
    assert!(
        continuity_policy_from_dps_payload(
            "mesh_continuity_policy=same_egress_only;mesh_continuity_policy=allow_flow_drain"
        )
        .is_err()
    );
}

#[test]
fn traffic_hints_from_dps_payload_parses_shadow_mode_with_continuity_priority() {
    let hints = traffic_hints_from_dps_payload(
        "mesh_traffic_class=gaming_fps;mesh_multipath_mode=standby_only;mesh_continuity_policy=allow_flow_drain",
    )
    .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(hints.traffic_class.map(|v| v.as_str()), Some("gaming_fps"));
    assert_eq!(
        hints.multipath_mode.map(|v| v.as_str()),
        Some("standby_only")
    );
    assert_eq!(hints.multipath_demand.map(|v| v.as_str()), None);
    assert_eq!(
        hints.continuity_policy.map(|v| v.as_str()),
        Some("allow_flow_drain")
    );
    assert_eq!(hints.shadow_switch_mode.as_str(), "flow_drain");
}

#[test]
fn traffic_hints_from_dps_payload_falls_back_to_multipath_mode() {
    let hints = traffic_hints_from_dps_payload("mesh_multipath_mode=standby_only")
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(hints.shadow_switch_mode.as_str(), "transport_only");
}

#[test]
fn traffic_hints_from_dps_payload_includes_coarse_multipath_demand() {
    let hints = traffic_hints_from_dps_payload(
        "mesh_multipath_mode=aggregate_buffered;mesh_multipath_demand=bulk",
    )
    .unwrap_or_else(|e| unreachable!("{e}"));

    assert_eq!(
        hints.multipath_mode.map(|v| v.as_str()),
        Some("aggregate_buffered")
    );
    assert_eq!(hints.multipath_demand.map(|v| v.as_str()), Some("bulk"));
    assert!(hints.has_any_hint());
}

#[test]
fn traffic_hints_from_dps_payload_ignores_non_hint_mesh_fields_in_one_pass() {
    let hints = traffic_hints_from_dps_payload(
        "allow=mesh;mesh_unknown_future_key=1;mesh_max_peers=2;mesh_multipath_mode=flow_shard",
    )
    .unwrap_or_else(|e| unreachable!("{e}"));

    assert_eq!(hints.multipath_mode.map(|v| v.as_str()), Some("flow_shard"));
    assert_eq!(hints.shadow_switch_mode.as_str(), "flow_drain");
}

#[test]
fn traffic_hints_from_dps_payload_allows_payload_without_mesh_hints() {
    let hints =
        traffic_hints_from_dps_payload("allow=mesh").unwrap_or_else(|e| unreachable!("{e}"));

    assert!(!hints.has_any_hint());
    assert_eq!(hints.shadow_switch_mode.as_str(), "unknown");
}

#[test]
fn traffic_hints_from_dps_payload_rejects_malformed_or_duplicate_hint_in_one_pass() {
    assert!(traffic_hints_from_dps_payload("mesh_multipath_mode").is_err());
    assert!(
        traffic_hints_from_dps_payload(
            "mesh_traffic_class=dns;allow=mesh;MESH_TRAFFIC_CLASS=gaming_fps"
        )
        .is_err()
    );
}
