use crate::{
    MeshNode, MeshNodeCountry, MeshNodeCountryConfidence, MeshNodeCountrySource, MeshNodeId,
    MeshNodeListFilter, MeshNodeReasonCode, MeshNodeRuntime, MeshNodeStatus, MeshNodesPolicy,
    compute_mesh_node_score, group_mesh_nodes_by_country, refresh_mesh_node_scores,
    select_best_mesh_node,
};

fn test_node(
    id: &str,
    country_code: &str,
    country_name: &str,
    status: MeshNodeStatus,
    score: f64,
) -> MeshNode {
    let country = if country_code == MeshNodeCountry::UNKNOWN_CODE {
        MeshNodeCountry::unknown("test", 86_400)
    } else {
        MeshNodeCountry {
            country_code: country_code.to_string(),
            country_name: country_name.to_string(),
            country_source: MeshNodeCountrySource::NodeClaim,
            country_confidence: MeshNodeCountryConfidence::Low,
            country_updated_at: "test".to_string(),
            country_ttl_sec: 86_400,
            country_conflict: false,
            country_conflict_reason: None,
        }
    };
    MeshNode {
        node_id: MeshNodeId::new(id),
        endpoint: format!("{id}.test:443"),
        country,
        status,
        latency_ms: Some(50.0),
        jitter_ms: Some(5.0),
        loss_pct: Some(0.1),
        success_rate_5m: Some(99.0),
        success_rate_1h: Some(98.0),
        consecutive_failures: 0,
        observation_count: 10,
        score,
        explain_reason: "test".to_string(),
    }
}

#[test]
fn nodes_group_by_country_and_keep_unknown_last() {
    let nodes = vec![
        test_node(
            "node-x",
            "ZZ",
            MeshNodeCountry::UNKNOWN_NAME,
            MeshNodeStatus::Checking,
            20.0,
        ),
        test_node(
            "node-nl",
            "NL",
            "Netherlands",
            MeshNodeStatus::Healthy,
            88.0,
        ),
        test_node("node-de", "DE", "Germany", MeshNodeStatus::Healthy, 91.0),
    ];
    let groups = group_mesh_nodes_by_country(&nodes, &MeshNodeListFilter::default());
    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0].country_name, "Germany");
    assert_eq!(groups[1].country_name, "Netherlands");
    assert_eq!(groups[2].country_name, MeshNodeCountry::UNKNOWN_NAME);
}

#[test]
fn nodes_sort_inside_country_by_status_then_score_then_latency_then_id() {
    let mut low_latency = test_node("node-a", "DE", "Germany", MeshNodeStatus::Healthy, 90.0);
    low_latency.latency_ms = Some(20.0);
    let mut high_latency = test_node("node-b", "DE", "Germany", MeshNodeStatus::Healthy, 90.0);
    high_latency.latency_ms = Some(80.0);
    let nodes = vec![
        test_node("node-down", "DE", "Germany", MeshNodeStatus::Down, 100.0),
        test_node(
            "node-checking",
            "DE",
            "Germany",
            MeshNodeStatus::Checking,
            100.0,
        ),
        high_latency,
        low_latency,
    ];
    let groups = group_mesh_nodes_by_country(&nodes, &MeshNodeListFilter::default());
    let ids = groups[0]
        .nodes
        .iter()
        .map(|node| node.node_id.0.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["node-a", "node-b", "node-checking", "node-down"]);
}

#[test]
fn nodes_filter_country_status_available_and_search() {
    let nodes = vec![
        test_node("alpha-de", "DE", "Germany", MeshNodeStatus::Healthy, 90.0),
        test_node("beta-nl", "NL", "Netherlands", MeshNodeStatus::Down, 80.0),
    ];
    let mut filter = MeshNodeListFilter::default();
    filter.countries.insert("DE".to_string());
    filter.available_only = true;
    filter.search = Some("alpha".to_string());
    let groups = group_mesh_nodes_by_country(&nodes, &filter);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].country_code, "DE");
    assert_eq!(groups[0].nodes[0].node_id.0, "alpha-de");
}

#[test]
fn nodes_score_down_is_zero_and_low_observation_is_penalized() {
    let policy = MeshNodesPolicy::default();
    let down = test_node("down", "DE", "Germany", MeshNodeStatus::Down, 0.0);
    assert_eq!(compute_mesh_node_score(&down, &policy).final_score, 0.0);

    let mut low = test_node("low", "DE", "Germany", MeshNodeStatus::Healthy, 0.0);
    low.observation_count = 1;
    let high = test_node("high", "DE", "Germany", MeshNodeStatus::Healthy, 0.0);
    assert!(
        compute_mesh_node_score(&high, &policy).final_score
            > compute_mesh_node_score(&low, &policy).final_score
    );
}

#[test]
fn nodes_best_is_deterministic_and_excludes_down() {
    let nodes = vec![
        test_node("b-node", "DE", "Germany", MeshNodeStatus::Healthy, 90.0),
        test_node("a-node", "DE", "Germany", MeshNodeStatus::Healthy, 90.0),
        test_node("down", "DE", "Germany", MeshNodeStatus::Down, 100.0),
    ];
    let best = select_best_mesh_node(&nodes)
        .unwrap_or_else(|| unreachable!("test nodes contain selectable peers"));
    assert_eq!(best.node_id.0, "a-node");
}

#[test]
fn nodes_runtime_autoconnect_connects_without_current_node() {
    let policy = MeshNodesPolicy {
        autoconnect_enabled_by_default: true,
        ..MeshNodesPolicy::default()
    };
    let mut runtime =
        MeshNodeRuntime::new(policy).unwrap_or_else(|err| unreachable!("policy valid: {err:?}"));
    let node = test_node("best", "DE", "Germany", MeshNodeStatus::Healthy, 90.0);
    let decision = runtime.auto_step(&[node], 10);
    assert!(decision.allowed);
    assert_eq!(decision.reason, MeshNodeReasonCode::BestSelected);
}

#[test]
fn nodes_runtime_blocks_minor_improvement_and_hold_down() {
    let policy = MeshNodesPolicy {
        autoconnect_enabled_by_default: true,
        ..MeshNodesPolicy::default()
    };
    let mut runtime =
        MeshNodeRuntime::new(policy).unwrap_or_else(|err| unreachable!("policy valid: {err:?}"));
    runtime.state.current_node = Some(MeshNodeId::new("current"));
    let current = test_node("current", "DE", "Germany", MeshNodeStatus::Healthy, 80.0);
    let candidate = test_node(
        "candidate",
        "NL",
        "Netherlands",
        MeshNodeStatus::Healthy,
        84.0,
    );
    let decision = runtime.auto_step(&[current.clone(), candidate], 10);
    assert_eq!(
        decision.reason,
        MeshNodeReasonCode::CandidateNotBetterEnough
    );

    runtime.state.last_switch_tick = Some(9);
    let better = test_node("better", "NL", "Netherlands", MeshNodeStatus::Healthy, 99.0);
    let decision = runtime.auto_step(&[current, better], 10);
    assert_eq!(decision.reason, MeshNodeReasonCode::HoldDownActive);
}

#[test]
fn nodes_runtime_pinned_live_blocks_better_candidate_but_down_switches() {
    let policy = MeshNodesPolicy {
        autoconnect_enabled_by_default: true,
        ..MeshNodesPolicy::default()
    };
    let mut runtime =
        MeshNodeRuntime::new(policy).unwrap_or_else(|err| unreachable!("policy valid: {err:?}"));
    runtime.pin(MeshNodeId::new("pinned"));
    let pinned = test_node("pinned", "DE", "Germany", MeshNodeStatus::Healthy, 40.0);
    let better = test_node("better", "NL", "Netherlands", MeshNodeStatus::Healthy, 99.0);
    let decision = runtime.auto_step(&[pinned, better.clone()], 10);
    assert_eq!(decision.reason, MeshNodeReasonCode::PinnedNodeActive);

    let pinned_down = test_node("pinned", "DE", "Germany", MeshNodeStatus::Down, 0.0);
    let decision = runtime.auto_step(&[pinned_down, better], 100);
    assert_eq!(
        decision.reason,
        MeshNodeReasonCode::PinnedNodeDownEmergencySwitch
    );
    assert!(decision.allowed);
}

#[test]
fn nodes_policy_validation_rejects_bad_weights() {
    let mut policy = MeshNodesPolicy::default();
    policy.score.weights.latency = 0.99;
    assert!(policy.validate().is_err());
}

#[test]
fn nodes_refresh_scores_keeps_score_in_range() {
    let policy = MeshNodesPolicy::default();
    let mut nodes = vec![test_node(
        "node-de",
        "DE",
        "Germany",
        MeshNodeStatus::Healthy,
        0.0,
    )];
    refresh_mesh_node_scores(&mut nodes, &policy);
    assert!((0.0..=100.0).contains(&nodes[0].score));
}
