use crate::{MeshDiscoveryRecord, MeshPeerTablePolicy, MeshRuntime};

#[test]
fn table_enforcement_summary_is_consistent_with_detail_fields() {
    let mut runtime =
        MeshRuntime::bootstrap("cef-public", "seed-a").unwrap_or_else(|e| unreachable!("{e}"));
    assert!(
        runtime
            .set_peer_table_policy(MeshPeerTablePolicy {
                max_entries: 2,
                max_entries_per_region: 2,
                stale_after_ticks: 32,
                target_distinct_regions: 1,
                replacement_min_score_delta: 1,
                degraded_replacement_min_score_delta: 1,
                max_replacements_per_window: 8,
                stability_window_ticks: 8,
                profile_hysteresis_ticks: 4,
                resilient_region_spread_bonus_weight: 10,
            })
            .is_ok()
    );

    let records = vec![
        MeshDiscoveryRecord {
            node_id: "node-a".to_string(),
            endpoint: "198.51.100.10:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 90,
        },
        MeshDiscoveryRecord {
            node_id: "node-b".to_string(),
            endpoint: "198.51.100.11:443".to_string(),
            region: "us".to_string(),
            load_score: 15,
            reliability_score: 88,
        },
        MeshDiscoveryRecord {
            node_id: "node-c".to_string(),
            endpoint: "198.51.100.12:443".to_string(),
            region: "ap".to_string(),
            load_score: 20,
            reliability_score: 85,
        },
    ];
    assert!(runtime.merge_discovery("seed-b", &records).is_ok());

    let status = runtime.status_explain();
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_policy_max_entries=2") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_policy_max_entries_per_region=2") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_policy_stale_after_ticks=32") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_policy_max_replacements_per_window=8") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_policy_stability_window_ticks=8") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_policy_replacement_min_score_delta=1") })
    );
    assert!(status.iter().any(|line| {
        line.contains("status_table_policy_degraded_replacement_min_score_delta=1")
    }));
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_policy_profile_hysteresis_ticks=4") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_policy_target_within_capacity=true") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_policy_region_quota_within_capacity=true") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_policy_limits_invariants_all_true=true") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_policy_consistency_all_true=true") })
    );
    assert!(status.iter().any(|line| {
        line.contains("status_table_policy_summary=")
            && line.contains("max_entries:2")
            && line.contains("max_entries_per_region:2")
            && line.contains("target_distinct_regions:1")
            && line.contains("stale_after_ticks:32")
            && line.contains("max_replacements_per_window:8")
            && line.contains("stability_window_ticks:8")
            && line.contains("replacement_min_score_delta:1")
            && line.contains("degraded_replacement_min_score_delta:1")
            && line.contains("profile_hysteresis_ticks:4")
            && line.contains("resilient_region_spread_bonus_weight:10")
            && line.contains("target_within_capacity:true")
            && line.contains("region_quota_within_capacity:true")
            && line.contains("limits_invariants_all_true:true")
            && line.contains("summary_matches_fields:true")
            && line.contains("runtime_consistency_all_true:true")
    }));
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_policy_summary_matches_fields=true") })
    );
    assert!(status.iter().any(|line| {
        line.contains("status_table_policy_invariants=")
            && line.contains("target_within_capacity:true")
            && line.contains("region_quota_within_capacity:true")
            && line.contains("limits_invariants_all_true:true")
            && line.contains("summary_matches_fields:true")
            && line.contains("policy_consistency_all_true:true")
            && line.contains("runtime_consistency_all_true:true")
    }));
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_enforcement_summary_matches_fields=true") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_summary_consistency_all_true=true") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_runtime_consistency_all_true=true") })
    );
    assert!(
        status
            .iter()
            .any(|line| { line.contains("status_table_runtime_consistency_gate=ok") })
    );
    assert!(status.iter().any(|line| {
        line.contains("status_table_runtime_consistency_summary=gate=ok;all_true=true")
    }));
    assert!(status.iter().any(|line| {
        line.contains("status_plan_setup_discovery_table_compact=")
            && line.contains("sources:")
            && line.contains("entries_after:")
            && line.contains("consistency_gate:ok")
            && line.contains("degraded:false")
    }));
    let setup_compact = status
        .iter()
        .find_map(|line| line.strip_prefix("status_plan_setup_discovery_table_compact="))
        .unwrap_or("");
    assert!(!setup_compact.is_empty());
    let sources = status
        .iter()
        .find_map(|line| line.strip_prefix("status_sources="))
        .unwrap_or("");
    let entries_after = status
        .iter()
        .find_map(|line| line.strip_prefix("status_table_enforcement_after="))
        .unwrap_or("");
    let consistency_gate = status
        .iter()
        .find_map(|line| line.strip_prefix("status_table_runtime_consistency_gate="))
        .unwrap_or("");
    let degraded = status
        .iter()
        .find_map(|line| line.strip_prefix("status_preemptive_shadow_degraded_path="))
        .unwrap_or("");
    assert!(setup_compact.contains(&format!("sources:{sources}")));
    assert!(setup_compact.contains(&format!("entries_after:{entries_after}")));
    assert!(setup_compact.contains(&format!("consistency_gate:{consistency_gate}")));
    assert!(setup_compact.contains(&format!("degraded:{degraded}")));
    let summary = status
        .iter()
        .find_map(|line| line.strip_prefix("status_table_enforcement_summary="))
        .unwrap_or("");
    let invariants = status
        .iter()
        .find_map(|line| line.strip_prefix("status_table_enforcement_invariants="))
        .unwrap_or("");
    let invariants_all_true = status
        .iter()
        .find_map(|line| line.strip_prefix("status_table_enforcement_invariants_all_true="))
        .unwrap_or("");
    let tick = status
        .iter()
        .find_map(|line| line.strip_prefix("status_table_enforcement_tick="))
        .unwrap_or("");
    let drop_delta = status
        .iter()
        .find_map(|line| line.strip_prefix("status_table_enforcement_drop_delta="))
        .unwrap_or("");
    let drop_components_sum = status
        .iter()
        .find_map(|line| line.strip_prefix("status_table_enforcement_drop_components_sum="))
        .unwrap_or("");
    let drop_delta_matches_total = status
        .iter()
        .find_map(|line| line.strip_prefix("status_table_enforcement_drop_delta_matches_total="))
        .unwrap_or("");
    let drop_accounting = status
        .iter()
        .find_map(|line| line.strip_prefix("status_table_enforcement_drop_accounting="))
        .unwrap_or("");
    let drop_accounting_matches = status
        .iter()
        .find_map(|line| line.strip_prefix("status_table_enforcement_drop_accounting_matches="))
        .unwrap_or("");
    let non_negative_drops = status
        .iter()
        .find_map(|line| line.strip_prefix("status_table_enforcement_non_negative_drops="))
        .unwrap_or("");
    let capacity_valid = status
        .iter()
        .find_map(|line| line.strip_prefix("status_table_enforcement_capacity_valid="))
        .unwrap_or("");
    assert!(!summary.is_empty());
    assert!(!invariants.is_empty());
    assert_eq!(invariants_all_true, "true");
    assert!(!tick.is_empty());
    assert_eq!(drop_delta, "1");
    assert_eq!(drop_components_sum, "1");
    assert_eq!(non_negative_drops, "true");
    assert_eq!(drop_delta_matches_total, "true");
    assert_eq!(capacity_valid, "true");
    assert_eq!(drop_accounting, "total:1;components:1;delta:1");
    assert_eq!(drop_accounting_matches, "true");
    assert!(invariants.contains("drop_breakdown_match:true"));
    assert!(invariants.contains("count_transition_valid:true"));
    assert!(invariants.contains("non_negative_drops:true"));
    assert!(invariants.contains("drop_delta_matches_total:true"));
    assert!(invariants.contains("drop_accounting_matches:true"));
    assert!(invariants.contains("capacity_valid:true"));
    assert!(invariants.contains("summary_matches_fields:true"));
    assert!(summary.contains("before:3"));
    assert!(summary.contains("after:2"));
    assert!(summary.contains("drop_total:1"));
    assert!(summary.contains("drop_region:0"));
    assert!(summary.contains("drop_global:1"));
    assert!(summary.contains("protected_region_skips:0"));
    assert!(summary.contains("profile:balanced"));
    assert!(summary.contains("target:1"));
    assert!(summary.contains("source:balanced:configured"));
    assert!(summary.contains("invariants_all_true:true"));
    assert!(summary.contains("summary_matches_fields:true"));
    assert!(summary.contains("runtime_consistency_all_true:true"));
    assert!(summary.contains(&format!("tick:{tick}")));
}
