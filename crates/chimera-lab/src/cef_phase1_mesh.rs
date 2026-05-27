#![forbid(unsafe_code)]

use chimera_config::parse_mesh_runtime_config_text;
use chimera_dht::SignedDiscoveryRecord;
use chimera_dps::{PolicyFragmentStore, SignedPolicyFragment};
use chimera_emergency::{EmergencyCarrierKind, EmergencyOffer};
use chimera_mesh::{
    MeshDiscoveryRecord, MeshFailoverEvent, MeshJoinMode, MeshJoinRequest, MeshPathPolicy,
    MeshPeerHealth, MeshPeerTablePolicy, MeshRuntime, evaluate_join_mode,
};
use chimera_relay::{RelayConsentMode, RelayPolicy};
use chimera_reputation::{ComplaintEvidence, ReputationState, apply_penalty};
use chimera_roaming::{RoamingCache, RoamingEntry};
use serde_json::json;

use crate::CefPhase1SmokeResult;

pub(crate) fn execute_cef_phase1_smoke() -> Result<CefPhase1SmokeResult, String> {
    let mut mesh_runtime = MeshRuntime::bootstrap("cef-public", "seed-bootstrap")
        .map_err(|error| format!("mesh bootstrap failed: {error}"))?;
    let table_policy = mesh_peer_table_policy_from_env()?;
    mesh_runtime
        .set_peer_table_policy(table_policy)
        .map_err(|error| format!("mesh peer table policy apply failed: {error}"))?;
    let mesh_records = vec![
        MeshDiscoveryRecord {
            node_id: "node-eu-1".to_string(),
            endpoint: "198.51.100.7:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 92,
        },
        MeshDiscoveryRecord {
            node_id: "node-eu-2".to_string(),
            endpoint: "198.51.100.8:443".to_string(),
            region: "eu".to_string(),
            load_score: 40,
            reliability_score: 88,
        },
    ];
    mesh_runtime
        .merge_discovery("seed-dht", &mesh_records)
        .map_err(|error| format!("mesh discovery merge failed: {error}"))?;

    let mesh_mode = evaluate_join_mode(&MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-cef-lab-01".to_string(),
        invite_token: Some("inv-cef-lab".to_string()),
    })
    .map_err(|error| format!("mesh join mode evaluation failed: {error}"))?;
    if mesh_mode != MeshJoinMode::InvitationOnly {
        return Err("mesh join mode mismatch for invitation token".to_string());
    }
    let dht_record = SignedDiscoveryRecord::parse(
        "namespace=cef-public;node_hint=198.51.100.7:443;epoch=42;signature=abcdef123456",
    )
    .map_err(|error| format!("dht discovery record parse failed: {error}"))?;
    dht_record
        .basic_verify()
        .map_err(|error| format!("dht discovery record verify failed: {error}"))?;

    let mut dps_store = PolicyFragmentStore::default();
    dps_store
        .insert(SignedPolicyFragment {
            issuer: "issuer-cef".to_string(),
            policy_id: "policy-main".to_string(),
            version: 1,
            payload: "allow=mesh;mesh_allowed_regions=eu;mesh_min_reliability=80;mesh_max_load=50;mesh_max_peers=1".to_string(),
            signature: "abcdef123456".to_string(),
        })
        .map_err(|error| format!("dps policy fragment insert failed: {error}"))?;
    let dps_latest = dps_store.latest_by_id("policy-main");
    let latest_fragment = match dps_latest {
        Some(value) => value,
        None => return Err("dps latest policy fragment resolve failed".to_string()),
    };
    if latest_fragment.version != 1 || !latest_fragment.payload.contains("allow=mesh") {
        return Err("dps latest policy fragment resolve failed".to_string());
    }
    let mesh_policy = MeshPathPolicy::from_dps_payload(&latest_fragment.payload)
        .map_err(|error| format!("mesh policy from dps failed: {error}"))?;
    let mesh_plan = mesh_runtime
        .plan_path(
            &MeshJoinRequest {
                namespace: "cef-public".to_string(),
                node_name: "node-cef-lab-01".to_string(),
                invite_token: Some("inv-cef-lab".to_string()),
            },
            &mesh_policy,
        )
        .map_err(|error| format!("mesh path plan failed: {error}"))?;
    if mesh_plan.selected_peers.len() != 1 {
        return Err("mesh path plan selected peer count mismatch".to_string());
    }
    if mesh_plan.selected_peers[0].node_id != "node-eu-1" {
        return Err("mesh path plan selected unexpected peer".to_string());
    }
    let failover_plan = mesh_runtime
        .failover_plan(
            &MeshJoinRequest {
                namespace: "cef-public".to_string(),
                node_name: "node-cef-lab-01".to_string(),
                invite_token: Some("inv-cef-lab".to_string()),
            },
            &mesh_policy,
            &MeshFailoverEvent {
                failed_node_id: "node-eu-1".to_string(),
                reason: "health_probe_timeout".to_string(),
            },
        )
        .map_err(|error| format!("mesh failover plan failed: {error}"))?;
    if failover_plan.selected_peers.len() != 1 {
        return Err("mesh failover plan selected peer count mismatch".to_string());
    }
    if failover_plan.selected_peers[0].node_id != "node-eu-2" {
        return Err("mesh failover plan selected unexpected peer".to_string());
    }
    mesh_runtime
        .update_health_state(&[MeshPeerHealth {
            node_id: "node-eu-1".to_string(),
            healthy: true,
            cooldown_active: true,
        }])
        .map_err(|error| format!("mesh health state persistence failed: {error}"))?;
    let reselection_plan = mesh_runtime
        .reselection_plan_with_health(
            &MeshJoinRequest {
                namespace: "cef-public".to_string(),
                node_name: "node-cef-lab-01".to_string(),
                invite_token: Some("inv-cef-lab".to_string()),
            },
            &mesh_policy,
            &[MeshPeerHealth {
                node_id: "node-eu-1".to_string(),
                healthy: true,
                cooldown_active: true,
            }],
        )
        .map_err(|error| format!("mesh health reselection failed: {error}"))?;
    if reselection_plan.selected_peers.len() != 1 {
        return Err("mesh reselection selected peer count mismatch".to_string());
    }
    if reselection_plan.selected_peers[0].node_id != "node-eu-2" {
        return Err("mesh reselection selected unexpected peer".to_string());
    }
    let reselection_with_persisted_state = mesh_runtime
        .reselection_plan_with_health(
            &MeshJoinRequest {
                namespace: "cef-public".to_string(),
                node_name: "node-cef-lab-01".to_string(),
                invite_token: Some("inv-cef-lab".to_string()),
            },
            &mesh_policy,
            &[],
        )
        .map_err(|error| format!("mesh persisted state reselection failed: {error}"))?;
    if reselection_with_persisted_state.selected_peers.len() != 1 {
        return Err("mesh persisted reselection selected peer count mismatch".to_string());
    }
    if reselection_with_persisted_state.selected_peers[0].node_id != "node-eu-2" {
        return Err("mesh persisted reselection selected unexpected peer".to_string());
    }

    let relay_policy = RelayPolicy {
        mode: RelayConsentMode::TrustedOnly,
        max_concurrent_flows: 2,
        max_mbps: 10,
    };
    relay_policy
        .validate()
        .map_err(|error| format!("relay policy validation failed: {error}"))?;
    if !relay_policy.allows_flow(1) || relay_policy.allows_flow(2) {
        return Err("relay policy flow admission check failed".to_string());
    }

    let offer = EmergencyOffer {
        kind: EmergencyCarrierKind::Qr,
        token: "cef_phase1_offer_token_123456".to_string(),
        ttl_seconds: 300,
    };
    offer
        .validate()
        .map_err(|error| format!("emergency validation failed: {error}"))?;

    let mut cache = RoamingCache::default();
    cache
        .insert(RoamingEntry {
            namespace: "cef-public".to_string(),
            gateway_hint: "198.51.100.25:443".to_string(),
            expires_epoch: 10_000,
        })
        .map_err(|error| format!("roaming cache insert failed: {error}"))?;
    let roaming_hit = cache.resolve_active("cef-public", 9_000).is_some();
    if !roaming_hit {
        return Err("roaming cache resolve_active returned none".to_string());
    }

    let evidence = ComplaintEvidence {
        node_id: "node-cef-01".to_string(),
        reason_code: "relay_misuse".to_string(),
        signature: "abcdef12345678".to_string(),
    };
    evidence
        .validate()
        .map_err(|error| format!("reputation evidence validation failed: {error}"))?;
    let mut state = ReputationState {
        node_id: "node-cef-01".to_string(),
        score: 100,
    };
    apply_penalty(&mut state, 10).map_err(|error| format!("penalty apply failed: {error}"))?;
    let penalty_applied = state.score == 90;
    if !penalty_applied {
        return Err("penalty did not reduce score as expected".to_string());
    }

    Ok(CefPhase1SmokeResult {
        mesh_join_mode_resolved: true,
        mesh_failover_reselection_verified: true,
        dht_discovery_record_verified: true,
        dps_policy_fragment_verified: true,
        relay_policy_verified: true,
        emergency_offer_valid: true,
        roaming_cache_active_hit: true,
        reputation_penalty_applied: true,
    })
}

fn mesh_peer_table_policy_from_env() -> Result<MeshPeerTablePolicy, String> {
    let raw = std::env::var("CHIMERA_MESH_RUNTIME_CONFIG").unwrap_or_default();
    let config = parse_mesh_runtime_config_text(&raw)
        .map_err(|error| format!("mesh runtime config parse failed: {error}"))?;
    Ok(MeshPeerTablePolicy {
        max_entries: config.peer_table_max_entries,
        max_entries_per_region: config.peer_table_max_entries_per_region,
        stale_after_ticks: config.peer_table_stale_after_ticks,
        ..MeshPeerTablePolicy::default()
    })
}

pub(crate) fn render_mesh_runtime_trace_json() -> String {
    let mut mesh_runtime = match MeshRuntime::bootstrap("cef-public", "seed-bootstrap") {
        Ok(value) => value,
        Err(error) => {
            return json!({"status":"error","kind":"mesh_runtime_trace","error":error}).to_string();
        }
    };
    let table_policy = match mesh_peer_table_policy_from_env() {
        Ok(value) => value,
        Err(error) => {
            return json!({"status":"error","kind":"mesh_runtime_trace","error":error}).to_string();
        }
    };
    if let Err(error) = mesh_runtime.set_peer_table_policy(table_policy) {
        return json!({"status":"error","kind":"mesh_runtime_trace","error":error}).to_string();
    }
    let mesh_records = vec![
        MeshDiscoveryRecord {
            node_id: "node-eu-1".to_string(),
            endpoint: "198.51.100.7:443".to_string(),
            region: "eu".to_string(),
            load_score: 20,
            reliability_score: 92,
        },
        MeshDiscoveryRecord {
            node_id: "node-eu-2".to_string(),
            endpoint: "198.51.100.8:443".to_string(),
            region: "eu".to_string(),
            load_score: 40,
            reliability_score: 88,
        },
    ];
    if let Err(error) = mesh_runtime.merge_discovery("seed-dht", &mesh_records) {
        return json!({"status":"error","kind":"mesh_runtime_trace","error":error}).to_string();
    }
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-cef-lab-01".to_string(),
        invite_token: Some("inv-cef-lab".to_string()),
    };
    let policy = match MeshPathPolicy::from_dps_payload(
        "allow=mesh;mesh_allowed_regions=eu;mesh_min_reliability=80;mesh_max_load=50;mesh_max_peers=1",
    ) {
        Ok(value) => value,
        Err(error) => {
            return json!({"status":"error","kind":"mesh_runtime_trace","error":error}).to_string();
        }
    };
    let initial = match mesh_runtime.plan_path(&req, &policy) {
        Ok(value) => value,
        Err(error) => {
            return json!({"status":"error","kind":"mesh_runtime_trace","error":error}).to_string();
        }
    };
    let failover = match mesh_runtime.failover_plan(
        &req,
        &policy,
        &MeshFailoverEvent {
            failed_node_id: "node-eu-1".to_string(),
            reason: "health_probe_timeout".to_string(),
        },
    ) {
        Ok(value) => value,
        Err(error) => {
            return json!({"status":"error","kind":"mesh_runtime_trace","error":error}).to_string();
        }
    };
    if let Err(error) = mesh_runtime.update_health_state(&[MeshPeerHealth {
        node_id: "node-eu-1".to_string(),
        healthy: true,
        cooldown_active: true,
    }]) {
        return json!({"status":"error","kind":"mesh_runtime_trace","error":error}).to_string();
    }
    let reselection = match mesh_runtime.reselection_plan_with_health(
        &req,
        &policy,
        &[MeshPeerHealth {
            node_id: "node-eu-1".to_string(),
            healthy: true,
            cooldown_active: true,
        }],
    ) {
        Ok(value) => value,
        Err(error) => {
            return json!({"status":"error","kind":"mesh_runtime_trace","error":error}).to_string();
        }
    };
    let persisted = match mesh_runtime.reselection_plan_with_health(&req, &policy, &[]) {
        Ok(value) => value,
        Err(error) => {
            return json!({"status":"error","kind":"mesh_runtime_trace","error":error}).to_string();
        }
    };

    json!({
        "status": "ok",
        "kind": "mesh_runtime_trace",
        "namespace": "cef-public",
        "join_mode": format!("{:?}", initial.join_mode),
        "phases": {
            "initial": {
                "selected_peer": initial.selected_peers.first().map(|p| p.node_id.clone()).unwrap_or_default(),
                "explain": initial.explain,
            },
            "failover": {
                "selected_peer": failover.selected_peers.first().map(|p| p.node_id.clone()).unwrap_or_default(),
                "explain": failover.explain,
            },
            "reselection": {
                "selected_peer": reselection.selected_peers.first().map(|p| p.node_id.clone()).unwrap_or_default(),
                "explain": reselection.explain,
            },
            "persisted_state_reselection": {
                "selected_peer": persisted.selected_peers.first().map(|p| p.node_id.clone()).unwrap_or_default(),
                "explain": persisted.explain,
            }
        }
    })
    .to_string()
}

pub(crate) fn render_mesh_auto_adaptive_trace_json() -> String {
    let mut mesh_runtime = match MeshRuntime::bootstrap("cef-public", "seed-bootstrap") {
        Ok(value) => value,
        Err(error) => {
            return json!({"status":"error","kind":"mesh_auto_adaptive_trace","error":error})
                .to_string();
        }
    };
    let table_policy = match mesh_peer_table_policy_from_env() {
        Ok(value) => value,
        Err(error) => {
            return json!({"status":"error","kind":"mesh_auto_adaptive_trace","error":error})
                .to_string();
        }
    };
    if let Err(error) = mesh_runtime.set_peer_table_policy(table_policy) {
        return json!({"status":"error","kind":"mesh_auto_adaptive_trace","error":error})
            .to_string();
    }
    let mesh_records = vec![
        MeshDiscoveryRecord {
            node_id: "node-fast-low-lat".to_string(),
            endpoint: "198.51.100.21:443".to_string(),
            region: "eu".to_string(),
            load_score: 10,
            reliability_score: 97,
        },
        MeshDiscoveryRecord {
            node_id: "node-stable-backup".to_string(),
            endpoint: "198.51.100.22:443".to_string(),
            region: "us".to_string(),
            load_score: 14,
            reliability_score: 96,
        },
    ];
    if let Err(error) = mesh_runtime.merge_discovery("seed-dht", &mesh_records) {
        return json!({"status":"error","kind":"mesh_auto_adaptive_trace","error":error})
            .to_string();
    }
    let req = MeshJoinRequest {
        namespace: "cef-public".to_string(),
        node_name: "node-cef-lab-auto".to_string(),
        invite_token: None,
    };
    let auto_policy = MeshPathPolicy::default_auto();
    let auto_plan = match mesh_runtime.plan_path(&req, &auto_policy) {
        Ok(value) => value,
        Err(error) => {
            return json!({"status":"error","kind":"mesh_auto_adaptive_trace","error":error})
                .to_string();
        }
    };
    if let Err(error) = mesh_runtime.update_health_state(&[MeshPeerHealth {
        node_id: "node-fast-low-lat".to_string(),
        healthy: true,
        cooldown_active: true,
    }]) {
        return json!({"status":"error","kind":"mesh_auto_adaptive_trace","error":error})
            .to_string();
    }
    let degraded_plan = match mesh_runtime.plan_path(&req, &auto_policy) {
        Ok(value) => value,
        Err(error) => {
            return json!({"status":"error","kind":"mesh_auto_adaptive_trace","error":error})
                .to_string();
        }
    };
    let manual_policy = MeshPathPolicy {
        allowed_regions: vec!["eu".to_string()],
        blocked_node_ids: Vec::new(),
        require_min_reliability: 96,
        max_load_score: 25,
        max_peers: 1,
        prefer_region_diversity: true,
        max_selected_per_region: 1,
        min_distinct_regions: 1,
        path_profile_override: None,
        connect_fallback_ports: vec![443, 8443],
    };
    let manual_plan = match mesh_runtime.plan_path(&req, &manual_policy) {
        Ok(value) => value,
        Err(error) => {
            return json!({"status":"error","kind":"mesh_auto_adaptive_trace","error":error})
                .to_string();
        }
    };

    json!({
        "status": "ok",
        "kind": "mesh_auto_adaptive_trace",
        "namespace": "cef-public",
        "plans": {
            "auto_baseline": {
                "selected_peer": auto_plan.selected_peers.first().map(|p| p.node_id.clone()).unwrap_or_default(),
                "explain": auto_plan.explain,
            },
            "auto_degraded": {
                "selected_peer": degraded_plan.selected_peers.first().map(|p| p.node_id.clone()).unwrap_or_default(),
                "explain": degraded_plan.explain,
            },
            "manual_override": {
                "selected_peer": manual_plan.selected_peers.first().map(|p| p.node_id.clone()).unwrap_or_default(),
                "explain": manual_plan.explain,
            }
        },
        "network_state": "not_modified"
    })
    .to_string()
}
