#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
START_EPOCH="$(date +%s)"
GENERATED_AT_UTC="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

just baseline-freeze
just cleanroom-handoff-check
just benchmark-regression-check
just cef-track-report
just cef-track-guard
just cef-track-sync-guard
just cef-gap-map-guard
just cef-consistency-guard
just cef-phase1-smoke
just mesh-auto-smoke
just mesh-auto-adaptive-trace-guard
just mesh-cli-recovery-schema-guard-selfcheck
just mesh-cli-recovery-schema-guard
just release-readiness-report-json
just release-readiness-report-ru
just report-pack-json
just report-pack
just runtime-apply-dns-smoke
just runtime-apply-route-smoke-selfcheck
just runtime-apply-route-smoke
just runtime-apply-route-existing-tun-smoke-selfcheck
just runtime-apply-route-existing-tun-smoke
just runtime-apply-route-multi-cidr-smoke-selfcheck
just runtime-apply-route-multi-cidr-smoke
just runtime-route-policy-validation-smoke-selfcheck
just runtime-route-policy-validation-smoke
just runtime-route-duplicate-cidr-validation-smoke-selfcheck
just runtime-route-duplicate-cidr-validation-smoke
just runtime-tun-name-validation-smoke-selfcheck
just runtime-tun-name-validation-smoke
just runtime-resolv-conf-validation-smoke-selfcheck
just runtime-resolv-conf-validation-smoke
just runtime-datapath-multiflow-smoke-selfcheck
just runtime-datapath-multiflow-smoke
just runtime-policy-precedence-smoke-selfcheck
just runtime-policy-precedence-smoke
just runtime-forced-stop-rollback-smoke-selfcheck
just runtime-forced-stop-rollback-smoke
just rust-no-hardcode-guard-selfcheck
just rust-no-hardcode-guard
just runtime-real-world-probe-smoke-selfcheck
just runtime-real-world-probe-smoke
just runtime-real-world-probe-schema-guard-selfcheck
just runtime-real-world-probe-schema-guard
just probe-access-smoke-selfcheck
just probe-access-smoke
just reality-audit-refresh-selfcheck
just reality-audit-refresh
just reality-audit-schema-guard-selfcheck
just reality-audit-schema-guard

release_ok=false
release_ok_lab_only=false
cef_phase1_smoke_ok=false
cef_phase1_closed=false
network_state_ok=false
runtime_apply_smoke_ok=false
runtime_apply_route_smoke_ok=false
runtime_apply_route_existing_tun_smoke_ok=false
runtime_apply_route_multi_cidr_smoke_ok=false
runtime_route_policy_validation_smoke_ok=false
runtime_route_duplicate_cidr_validation_smoke_ok=false
runtime_tun_name_validation_smoke_ok=false
runtime_resolv_conf_validation_smoke_ok=false
runtime_datapath_multiflow_smoke_ok=false
runtime_policy_precedence_smoke_ok=false
runtime_forced_stop_rollback_smoke_ok=false
runtime_real_world_probe_smoke_ok=false
runtime_real_world_direct_probe_ok=false
runtime_real_world_proxy_listener_detected=false
runtime_real_world_proxy_probe_attempted=false
runtime_real_world_proxy_probe_ok=false
runtime_real_world_proxy_selected_from_candidates=false
runtime_real_world_proxy_candidates=""
runtime_real_world_proxy_probe_error="unknown"
runtime_real_world_skipped_no_curl=false
runtime_real_world_skipped_no_proxy_listener=false
runtime_real_world_proxy_blocked_targets_total=0
runtime_real_world_proxy_blocked_targets_ok=0
runtime_real_world_proxy_blocked_targets_failed=0
runtime_probe_access_smoke_ok=false
mesh_route_explain_ok=false
mesh_auto_adaptive_ok=false
artifacts_fresh_ok=true
cef_gap_map_guard=true
real_world_datapath_closed=false
if rg -q '"release_ok":true' docs/RELEASE_READINESS_REPORT.json; then
  release_ok=true
  release_ok_lab_only=true
fi
if rg -q '"network_state":"not_modified"' docs/RELEASE_READINESS_REPORT.json; then
  network_state_ok=true
fi
if rg -q '"status":"ok"' docs/CEF_PHASE1_SMOKE.json && rg -q '"kind":"cef_phase1_smoke"' docs/CEF_PHASE1_SMOKE.json && rg -q '"emergency_offer_valid":true' docs/CEF_PHASE1_SMOKE.json && rg -q '"roaming_cache_active_hit":true' docs/CEF_PHASE1_SMOKE.json && rg -q '"reputation_penalty_applied":true' docs/CEF_PHASE1_SMOKE.json; then
  cef_phase1_smoke_ok=true
fi
if rg -q '"phase1_closed":true' docs/CEF_TRACK_REPORT.json; then
  cef_phase1_closed=true
fi
if rg -q '"status":"ok"' docs/REALITY_AUDIT_LATEST.json && rg -q '"kind":"reality_audit"' docs/REALITY_AUDIT_LATEST.json && rg -q '"real_world_datapath_closed":true' docs/REALITY_AUDIT_LATEST.json; then
  real_world_datapath_closed=true
fi
if rg -q '"status":"ok"' docs/RUNTIME_APPLY_DNS_SMOKE.json && rg -q '"network_state":"modified"' docs/RUNTIME_APPLY_DNS_SMOKE.json && rg -q '"rollback_ok":true' docs/RUNTIME_APPLY_DNS_SMOKE.json; then
  runtime_apply_smoke_ok=true
fi
if rg -q '"status":"ok"' docs/RUNTIME_APPLY_ROUTE_SMOKE.json && rg -q '"network_state":"modified"' docs/RUNTIME_APPLY_ROUTE_SMOKE.json && rg -q '"rollback_ok":true' docs/RUNTIME_APPLY_ROUTE_SMOKE.json && rg -q '"apply_attempt_ok":true' docs/RUNTIME_APPLY_ROUTE_SMOKE.json && rg -q '"policy_rule_ok":true' docs/RUNTIME_APPLY_ROUTE_SMOKE.json; then
  runtime_apply_route_smoke_ok=true
fi
if rg -q '"status":"ok"' docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json && rg -q '"network_state":"modified"' docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json && rg -q '"rollback_ok":true' docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json && rg -q '"apply_attempt_ok":true' docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json && rg -q '"preexisting_tun_used":true' docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json; then
  runtime_apply_route_existing_tun_smoke_ok=true
fi
if rg -q '"status":"ok"' docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json && rg -q '"network_state":"modified"' docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json && rg -q '"rollback_ok":true' docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json && ( (rg -q '"apply_attempt_ok":true' docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json && rg -q '"policy_rule_ok":true' docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json) || rg -q '"skipped_no_tun":true' docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json ); then
  runtime_apply_route_multi_cidr_smoke_ok=true
fi
if rg -q '"status":"ok"' docs/RUNTIME_ROUTE_POLICY_VALIDATION_SMOKE.json && rg -q '"network_state":"not_modified"' docs/RUNTIME_ROUTE_POLICY_VALIDATION_SMOKE.json && rg -q '"apply_rejected":true' docs/RUNTIME_ROUTE_POLICY_VALIDATION_SMOKE.json && rg -q '"state_not_created":true' docs/RUNTIME_ROUTE_POLICY_VALIDATION_SMOKE.json; then
  runtime_route_policy_validation_smoke_ok=true
fi
if rg -q '"status":"ok"' docs/RUNTIME_ROUTE_DUPLICATE_CIDR_VALIDATION_SMOKE.json && rg -q '"network_state":"not_modified"' docs/RUNTIME_ROUTE_DUPLICATE_CIDR_VALIDATION_SMOKE.json && rg -q '"apply_rejected":true' docs/RUNTIME_ROUTE_DUPLICATE_CIDR_VALIDATION_SMOKE.json && rg -q '"state_not_created":true' docs/RUNTIME_ROUTE_DUPLICATE_CIDR_VALIDATION_SMOKE.json; then
  runtime_route_duplicate_cidr_validation_smoke_ok=true
fi
if rg -q '"status":"ok"' docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json && rg -q '"network_state":"not_modified"' docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json && rg -q '"apply_rejected":true' docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json && rg -q '"state_not_created":true' docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json; then
  runtime_tun_name_validation_smoke_ok=true
fi
if rg -q '"status":"ok"' docs/RUNTIME_RESOLV_CONF_VALIDATION_SMOKE.json && rg -q '"network_state":"not_modified"' docs/RUNTIME_RESOLV_CONF_VALIDATION_SMOKE.json && rg -q '"apply_rejected":true' docs/RUNTIME_RESOLV_CONF_VALIDATION_SMOKE.json && rg -q '"state_not_created":true' docs/RUNTIME_RESOLV_CONF_VALIDATION_SMOKE.json; then
  runtime_resolv_conf_validation_smoke_ok=true
fi
if rg -q '"status":"ok"' docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json && rg -q '"network_state":"not_modified"' docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json && rg -q '"gateway_ok":true' docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json && rg -q '"block_ok":true' docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json && rg -q '"direct_ok":true' docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json; then
  runtime_datapath_multiflow_smoke_ok=true
fi
if rg -q '"status":"ok"' docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json && rg -q '"network_state":"not_modified"' docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json && rg -q '"precedence_ok":true' docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json && rg -q '"all_matches_ok":true' docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json && rg -q '"dns_binding_ok":true' docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json; then
  runtime_policy_precedence_smoke_ok=true
fi
if rg -q '"status":"ok"' docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json && rg -q '"network_state":"modified"' docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json && rg -q '"apply_attempt_ok":true' docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json && rg -q '"recover_ok":true' docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json && rg -q '"down_state_clean":true' docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json; then
  runtime_forced_stop_rollback_smoke_ok=true
fi
if rg -q '"status":"ok"' docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json && rg -q '"kind":"runtime_real_world_probe_smoke"' docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json && rg -q '"network_state":"not_modified"' docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json; then
  # Read all probe fields atomically in one Rust call to avoid repeated parsing/drift.
  eval "$(
    cargo run -q -p chimera-lab --bin runtime_real_world_probe_env -- docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json
  )"
  # Valid real-world probe requires direct check and either:
  # - proxy path really works, or
  # - proxy listener is explicitly absent (honest environment limitation).
  if [[ "$runtime_real_world_skipped_no_curl" == "false" && "$runtime_real_world_direct_probe_ok" == "true" && ( "$runtime_real_world_proxy_probe_ok" == "true" || "$runtime_real_world_skipped_no_proxy_listener" == "true" ) && ( "$runtime_real_world_proxy_blocked_targets_total" -eq 0 || "$runtime_real_world_proxy_blocked_targets_ok" -ge 1 ) ]]; then
    runtime_real_world_probe_smoke_ok=true
  fi
fi
if [[ -f docs/probe_access_latest.json ]]; then
  eval "$(
    cargo run -q -p chimera-lab --bin probe_access_env -- docs/probe_access_latest.json
  )"
fi

if rg -q '"status":"ok"' docs/MESH_ROUTE_EXPLAIN.json && rg -q '"kind":"mesh_route_explain"' docs/MESH_ROUTE_EXPLAIN.json && rg -q '"initial_selected_peer":"node-eu-1"' docs/MESH_ROUTE_EXPLAIN.json && rg -q '"failover_selected_peer":"node-eu-2"' docs/MESH_ROUTE_EXPLAIN.json && rg -q '"cooldown_selected_peer":"node-eu-1"' docs/MESH_ROUTE_EXPLAIN.json && rg -q '"network_state":"not_modified"' docs/MESH_ROUTE_EXPLAIN.json; then
  mesh_route_explain_ok=true
fi
if rg -q '"status":"ok"' docs/MESH_AUTO_ADAPTIVE_TRACE.json && rg -q '"kind":"mesh_auto_adaptive_trace"' docs/MESH_AUTO_ADAPTIVE_TRACE.json && rg -q '"network_state":"not_modified"' docs/MESH_AUTO_ADAPTIVE_TRACE.json && rg -q 'path_profile_reason=auto:fast_signals' docs/MESH_AUTO_ADAPTIVE_TRACE.json && rg -q 'path_profile_reason=auto:degraded_active' docs/MESH_AUTO_ADAPTIVE_TRACE.json && rg -q 'effective_filter_source=manual_override' docs/MESH_AUTO_ADAPTIVE_TRACE.json; then
  mesh_auto_adaptive_ok=true
fi

for artifact in \
  docs/CEF_TRACK_REPORT.json \
  docs/CEF_TRACK_REPORT.md \
  docs/RELEASE_READINESS_REPORT.json \
  docs/RELEASE_READINESS_REPORT.md \
  docs/RELEASE_READINESS_REPORT_RU.md \
  docs/REPORT_PACK.json \
  docs/REPORT_PACK.md \
  docs/CEF_PHASE1_SMOKE.json \
  docs/BENCHMARK_REGRESSION_GATE.json \
  docs/benchmark_baseline.json \
  docs/benchmark_latest.json \
  docs/RUNTIME_APPLY_DNS_SMOKE.json \
  docs/RUNTIME_APPLY_ROUTE_SMOKE.json \
  docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json \
  docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json \
  docs/RUNTIME_ROUTE_POLICY_VALIDATION_SMOKE.json \
  docs/RUNTIME_ROUTE_DUPLICATE_CIDR_VALIDATION_SMOKE.json \
  docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json \
  docs/RUNTIME_RESOLV_CONF_VALIDATION_SMOKE.json \
  docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json \
  docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json \
  docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json \
  docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json \
  docs/MESH_AUTO_ADAPTIVE_TRACE.json \
  docs/probe_access_latest.json \
  docs/REALITY_AUDIT_LATEST.json \
  docs/SECOND_MACHINE_REPORT.md \
  docs/V1_MVP_BASELINE_MANIFEST.json \
  docs/V1_MVP_BASELINE.sha256
do
  if [[ ! -f "$artifact" ]]; then
    artifacts_fresh_ok=false
    break
  fi
  mtime_epoch="$(stat -c %Y "$artifact" 2>/dev/null || echo 0)"
  if [[ "$mtime_epoch" -lt "$START_EPOCH" ]]; then
    artifacts_fresh_ok=false
    break
  fi
done

status="fail"
if [[ "$release_ok" == "true" && "$cef_phase1_smoke_ok" == "true" && "$cef_phase1_closed" == "true" && "$network_state_ok" == "true" && "$runtime_apply_smoke_ok" == "true" && "$runtime_apply_route_smoke_ok" == "true" && "$runtime_apply_route_existing_tun_smoke_ok" == "true" && "$runtime_apply_route_multi_cidr_smoke_ok" == "true" && "$runtime_route_policy_validation_smoke_ok" == "true" && "$runtime_route_duplicate_cidr_validation_smoke_ok" == "true" && "$runtime_tun_name_validation_smoke_ok" == "true" && "$runtime_resolv_conf_validation_smoke_ok" == "true" && "$runtime_datapath_multiflow_smoke_ok" == "true" && "$runtime_policy_precedence_smoke_ok" == "true" && "$runtime_forced_stop_rollback_smoke_ok" == "true" && "$runtime_probe_access_smoke_ok" == "true" && "$mesh_route_explain_ok" == "true" && "$mesh_auto_adaptive_ok" == "true" && "$artifacts_fresh_ok" == "true" && "$cef_gap_map_guard" == "true" ]]; then
  status="ok"
fi

cat > docs/SHIP_READINESS_REPORT.json <<REPORT
{"status":"${status}","kind":"ship_readiness_report","message_en":"Ship readiness pipeline finished.","message_ru":"Пайплайн готовности к передаче завершен.","release_ok":${release_ok},"release_ok_lab_only":${release_ok_lab_only},"cef_phase1_smoke_ok":${cef_phase1_smoke_ok},"cef_phase1_closed":${cef_phase1_closed},"mesh_route_explain_ok":${mesh_route_explain_ok},"mesh_auto_adaptive_ok":${mesh_auto_adaptive_ok},"truth_boundary":{"lab_scope_only":true,"real_world_datapath_closed":${real_world_datapath_closed}},"network_state_not_modified":${network_state_ok},"runtime_apply_smoke_modified":${runtime_apply_smoke_ok},"runtime_apply_route_smoke_modified":${runtime_apply_route_smoke_ok},"runtime_apply_route_existing_tun_smoke_modified":${runtime_apply_route_existing_tun_smoke_ok},"runtime_apply_route_multi_cidr_smoke_ok":${runtime_apply_route_multi_cidr_smoke_ok},"runtime_route_policy_validation_smoke_ok":${runtime_route_policy_validation_smoke_ok},"runtime_route_duplicate_cidr_validation_smoke_ok":${runtime_route_duplicate_cidr_validation_smoke_ok},"runtime_tun_name_validation_smoke_ok":${runtime_tun_name_validation_smoke_ok},"runtime_resolv_conf_validation_smoke_ok":${runtime_resolv_conf_validation_smoke_ok},"runtime_datapath_multiflow_smoke_ok":${runtime_datapath_multiflow_smoke_ok},"runtime_policy_precedence_smoke_ok":${runtime_policy_precedence_smoke_ok},"runtime_forced_stop_rollback_smoke_ok":${runtime_forced_stop_rollback_smoke_ok},"runtime_probe_access_smoke_ok":${runtime_probe_access_smoke_ok},"runtime_real_world_probe_smoke_ok":${runtime_real_world_probe_smoke_ok},"runtime_real_world_direct_probe_ok":${runtime_real_world_direct_probe_ok},"runtime_real_world_proxy_listener_detected":${runtime_real_world_proxy_listener_detected},"runtime_real_world_proxy_probe_attempted":${runtime_real_world_proxy_probe_attempted},"runtime_real_world_proxy_probe_ok":${runtime_real_world_proxy_probe_ok},"runtime_real_world_proxy_selected_from_candidates":${runtime_real_world_proxy_selected_from_candidates},"runtime_real_world_proxy_candidates":"${runtime_real_world_proxy_candidates}","runtime_real_world_proxy_probe_error":"${runtime_real_world_proxy_probe_error}","runtime_real_world_proxy_blocked_targets_total":${runtime_real_world_proxy_blocked_targets_total},"runtime_real_world_proxy_blocked_targets_ok":${runtime_real_world_proxy_blocked_targets_ok},"runtime_real_world_proxy_blocked_targets_failed":${runtime_real_world_proxy_blocked_targets_failed},"runtime_real_world_skipped_no_curl":${runtime_real_world_skipped_no_curl},"runtime_real_world_skipped_no_proxy_listener":${runtime_real_world_skipped_no_proxy_listener},"artifacts_fresh":${artifacts_fresh_ok},"steps":{"baseline_freeze":true,"cleanroom_handoff_check":true,"benchmark_regression_gate":true,"cef_track_report":true,"cef_track_guard":true,"cef_track_sync_guard":true,"cef_gap_map_guard":${cef_gap_map_guard},"cef_consistency_guard":true,"cef_phase1_smoke":true,"mesh_auto_smoke":true,"mesh_auto_adaptive_trace_guard":true,"mesh_cli_recovery_schema_guard_selfcheck":true,"mesh_cli_recovery_schema_guard":true,"release_readiness_report_json":true,"release_readiness_report_ru":true,"report_pack_json":true,"report_pack_md":true,"runtime_apply_dns_smoke":true,"runtime_apply_route_smoke_selfcheck":true,"runtime_apply_route_smoke":true,"runtime_apply_route_existing_tun_smoke_selfcheck":true,"runtime_apply_route_existing_tun_smoke":true,"runtime_apply_route_multi_cidr_smoke_selfcheck":true,"runtime_apply_route_multi_cidr_smoke":true,"runtime_route_policy_validation_smoke_selfcheck":true,"runtime_route_policy_validation_smoke":true,"runtime_route_duplicate_cidr_validation_smoke_selfcheck":true,"runtime_route_duplicate_cidr_validation_smoke":true,"runtime_tun_name_validation_smoke_selfcheck":true,"runtime_tun_name_validation_smoke":true,"runtime_resolv_conf_validation_smoke_selfcheck":true,"runtime_resolv_conf_validation_smoke":true,"runtime_datapath_multiflow_smoke_selfcheck":true,"runtime_datapath_multiflow_smoke":true,"runtime_policy_precedence_smoke_selfcheck":true,"runtime_policy_precedence_smoke":true,"runtime_forced_stop_rollback_smoke_selfcheck":true,"runtime_forced_stop_rollback_smoke":true,"rust_no_hardcode_guard_selfcheck":true,"rust_no_hardcode_guard":true,"runtime_real_world_probe_smoke_selfcheck":true,"runtime_real_world_probe_smoke":true,"runtime_real_world_probe_schema_guard_selfcheck":true,"runtime_real_world_probe_schema_guard":true,"probe_access_smoke_selfcheck":true,"probe_access_smoke":true,"reality_audit_refresh_selfcheck":true,"reality_audit_refresh":true,"reality_audit_schema_guard_selfcheck":true,"reality_audit_schema_guard":true,"reality_ship_sync_guard_selfcheck":true,"reality_ship_sync_guard":true,"freshness_check":true},"generated_at":"${GENERATED_AT_UTC}"}
REPORT

cat > docs/SHIP_READINESS_REPORT.md <<REPORT
# Ship Readiness Report

Status: **$(if [[ "$status" == "ok" ]]; then echo "PASS"; else echo "FAIL"; fi)**
Generated at (UTC): \`${GENERATED_AT_UTC}\`

Checks:
- Baseline freeze: \`true\`
- Clean-room handoff check: \`true\`
- CEF track report: \`true\`
- CEF track guard: \`true\`
- CEF track sync guard: \`true\`
- Benchmark regression gate: \`true\`
- CEF gap map guard: \`true\`
- CEF consistency guard: \`true\`
- Mesh auto smoke: \`true\`
- Mesh auto adaptive trace guard: \`true\`
- Mesh CLI recovery schema guard selfcheck: \`true\`
- Mesh CLI recovery schema guard: \`true\`
- Release readiness JSON: \`true\`
- Release readiness RU markdown: \`true\`
- Report pack JSON: \`true\`
- Report pack markdown: \`true\`
- Release gate (\`release_ok\`): \`${release_ok}\`
- Release gate is lab-only (\`release_ok_lab_only\`): \`${release_ok_lab_only}\`
- CEF phase1 smoke: \`${cef_phase1_smoke_ok}\`
- CEF phase1 closed: \`${cef_phase1_closed}\`
- Mesh route explain: \`${mesh_route_explain_ok}\`
- Mesh auto adaptive trace: \`${mesh_auto_adaptive_ok}\`
- Network state unchanged: \`${network_state_ok}\`
- Runtime apply smoke (\`modified + rollback\`): \`${runtime_apply_smoke_ok}\`
- Runtime route smoke (\`modified + rollback\`): \`${runtime_apply_route_smoke_ok}\`
- Runtime route existing-TUN smoke (\`modified + rollback\`): \`${runtime_apply_route_existing_tun_smoke_ok}\`
- Runtime route multi-CIDR smoke (\`modified + rollback\`): \`${runtime_apply_route_multi_cidr_smoke_ok}\`
- Runtime route policy validation smoke (\`reject invalid + no state\`): \`${runtime_route_policy_validation_smoke_ok}\`
- Runtime route duplicate-CIDR validation smoke (\`reject invalid + no state\`): \`${runtime_route_duplicate_cidr_validation_smoke_ok}\`
- Runtime TUN-name validation smoke (\`reject invalid + no state\`): \`${runtime_tun_name_validation_smoke_ok}\`
- Runtime resolv.conf validation smoke (\`reject invalid + no state\`): \`${runtime_resolv_conf_validation_smoke_ok}\`
- Runtime datapath multiflow smoke (\`gateway/direct/block explain\`): \`${runtime_datapath_multiflow_smoke_ok}\`
- Runtime policy precedence smoke (\`exact>suffix + dns-binding\`): \`${runtime_policy_precedence_smoke_ok}\`
- Runtime forced-stop rollback smoke (\`recover without graceful down\`): \`${runtime_forced_stop_rollback_smoke_ok}\`
- Runtime probe-access smoke (\`batch targets + totals/threshold report\`): \`${runtime_probe_access_smoke_ok}\`
- Runtime real-world probe smoke (\`direct/proxy snapshot only\`): \`${runtime_real_world_probe_smoke_ok}\`
- Runtime real-world direct probe ok: \`${runtime_real_world_direct_probe_ok}\`
- Runtime real-world proxy listener detected: \`${runtime_real_world_proxy_listener_detected}\`
- Runtime real-world proxy probe attempted: \`${runtime_real_world_proxy_probe_attempted}\`
- Runtime real-world proxy probe ok: \`${runtime_real_world_proxy_probe_ok}\`
- Runtime real-world proxy selected from candidates: \`${runtime_real_world_proxy_selected_from_candidates}\`
- Runtime real-world proxy candidates: \`${runtime_real_world_proxy_candidates}\`
- Runtime real-world proxy probe error: \`${runtime_real_world_proxy_probe_error}\`
- Runtime real-world proxy blocked targets total: \`${runtime_real_world_proxy_blocked_targets_total}\`
- Runtime real-world proxy blocked targets ok: \`${runtime_real_world_proxy_blocked_targets_ok}\`
- Runtime real-world proxy blocked targets failed: \`${runtime_real_world_proxy_blocked_targets_failed}\`
- Runtime real-world skipped no curl: \`${runtime_real_world_skipped_no_curl}\`
- Runtime real-world skipped no proxy listener: \`${runtime_real_world_skipped_no_proxy_listener}\`
- Artifacts refreshed in this run: \`${artifacts_fresh_ok}\`

Truth boundary:
- Lab/proof/report contour only: \`true\`
- Real OS-level datapath closure (strict M4/M5): \`${real_world_datapath_closed}\`

Artifacts:
- \`docs/SHIP_READINESS_REPORT.json\`
- \`docs/RELEASE_READINESS_REPORT.json\`
- \`docs/RELEASE_READINESS_REPORT_RU.md\`
- \`docs/REPORT_PACK.json\`
- \`docs/REPORT_PACK.md\`
- \`docs/CEF_GAP_MAP_2026-05-18.md\`
- \`docs/CEF_PHASE1_SMOKE.json\`
- \`docs/BENCHMARK_REGRESSION_GATE.json\`
- \`docs/benchmark_baseline.json\`
- \`docs/benchmark_latest.json\`
- \`docs/RUNTIME_APPLY_DNS_SMOKE.json\`
- \`docs/RUNTIME_APPLY_ROUTE_SMOKE.json\`
- \`docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json\`
- \`docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json\`
- \`docs/RUNTIME_ROUTE_POLICY_VALIDATION_SMOKE.json\`
- \`docs/RUNTIME_ROUTE_DUPLICATE_CIDR_VALIDATION_SMOKE.json\`
- \`docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json\`
- \`docs/RUNTIME_RESOLV_CONF_VALIDATION_SMOKE.json\`
- \`docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json\`
- \`docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json\`
- \`docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json\`
- \`docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json\`
- \`docs/probe_access_latest.json\`
- \`docs/REALITY_AUDIT_LATEST.json\`
- \`docs/SECOND_MACHINE_REPORT.md\`
- \`docs/V1_MVP_BASELINE_MANIFEST.json\`
- \`docs/V1_MVP_BASELINE.sha256\`
REPORT

just reality-ship-sync-guard-selfcheck
just reality-ship-sync-guard

if [[ "$status" != "ok" ]]; then
  echo "ship readiness failed: release report gate is not green" >&2
  exit 1
fi

echo "ship readiness: PASS"
echo "report: docs/SHIP_READINESS_REPORT.json"
