set shell := ["bash", "-cu"]

fmt:
    cargo fmt --all

lint:
    cargo clippy --workspace --all-targets -- -D warnings

deny:
    bash scripts/cargo_deny_guard.sh

anti-monolith-guard:
    bash scripts/anti_monolith_guard.sh

check:
    just anti-monolith-guard
    cargo check --workspace

test:
    cargo test --workspace

lab-smoke:
    cargo run -p chimera-lab --bin chimera-lab -- smoke

lab-doctor:
    cargo run -p chimera-lab --bin chimera-lab -- doctor --json --out docs/lab_doctor_latest.json

mvp-spec-check:
    cargo run -p chimera-lab --bin chimera-lab -- mvp-spec-check --json --out docs/mvp_spec_check_latest.json

mvp-spec-report:
    cargo run -p chimera-lab --bin chimera-lab -- mvp-spec-report --out docs/MVP_SPEC_COVERAGE.md

m5-artifacts-report:
    cargo run -p chimera-lab --bin chimera-lab -- m5-artifacts-report --out docs/M5_ARTIFACTS_REPORT.md

m6-artifacts-report:
    cargo run -p chimera-lab --bin chimera-lab -- m6-artifacts-report --out docs/M6_ARTIFACTS_REPORT.md

release-readiness-report:
    cargo run -p chimera-lab --bin chimera-lab -- release-readiness-report --out docs/RELEASE_READINESS_REPORT.md

release-readiness-report-ru:
    cargo run -p chimera-lab --bin chimera-lab -- --lang ru release-readiness-report --out docs/RELEASE_READINESS_REPORT_RU.md

release-readiness-report-json:
    cargo run -p chimera-lab --bin chimera-lab -- release-readiness-report --json --out docs/RELEASE_READINESS_REPORT.json

release-readiness-audit-json:
    cargo run -p chimera-lab --bin chimera-lab -- release-readiness-report --json --out docs/release_readiness_audit.json

report-pack:
    cargo run -p chimera-lab --bin chimera-lab -- report-pack --out docs/REPORT_PACK.md

report-pack-json:
    cargo run -p chimera-lab --bin chimera-lab -- report-pack --json --out docs/REPORT_PACK.json

artifact-audit:
    cargo run -p chimera-lab --bin chimera-lab -- artifact-audit --json --out docs/ARTIFACT_AUDIT.json

mvp-snapshot:
    cargo run -p chimera-lab --bin chimera-lab -- mvp-snapshot --json --out docs/MVP_SNAPSHOT.json

mvp-snapshot-text:
    cargo run -p chimera-lab --bin chimera-lab -- mvp-snapshot --text --out docs/MVP_SNAPSHOT.txt

mvp-verify:
    cargo run -p chimera-lab --bin chimera-lab -- mvp-verify --json --out docs/MVP_VERIFY.json

mvp-verify-refresh:
    cargo run -p chimera-lab --bin chimera-lab -- mvp-verify --refresh --json --out docs/MVP_VERIFY.json

config-smoke:
    cargo run -p chimera-lab --bin chimera-lab -- config-smoke

diag-export:
    cargo run -p chimera-cli -- diag export --config configs/client.example.conf --age 120 --packets 1 --out docs/diag_export_latest.json

route-explain-json:
    cargo run -p chimera-cli -- route explain example.org --json --out docs/route_explain_latest.json

probe-access-smoke:
    cargo run -p chimera-cli -- probe access --url-file configs/probe_targets.example.txt --timeout-sec 3 --fail-threshold 1 --json --out docs/probe_access_latest.json

probe-access-smoke-selfcheck:
    test -f configs/probe_targets.example.txt
    rg -q '^https://example.org$' configs/probe_targets.example.txt
    rg -q '^https://www.youtube.com$' configs/probe_targets.example.txt
    cargo run -q -p chimera-cli -- probe access --url-file configs/probe_targets.example.txt --timeout-sec 2 --fail-threshold 10 --json --out /tmp/chimera_probe_access_smoke.json
    rg -q '"kind":"probe_access"' /tmp/chimera_probe_access_smoke.json
    rg -Fq '"targets":[' /tmp/chimera_probe_access_smoke.json
    rg -Fq '"totals":{' /tmp/chimera_probe_access_smoke.json
    rm -f /tmp/chimera_probe_access_smoke.json

chimera-path-proof:
    bash scripts/chimera-path-proof.sh docs/CHIMERA_PATH_PROOF.json || true

chimera-path-proof-selfcheck:
    test -x scripts/chimera-path-proof.sh
    bash -n scripts/chimera-path-proof.sh
    tmp_out=$(mktemp) ; \
      bash scripts/chimera-path-proof.sh /tmp/chimera_path_proof_selfcheck.json > "$tmp_out" 2>/dev/null || true ; \
      rg -q '"kind":"chimera_path_proof"' "$tmp_out" ; \
      rm -f "$tmp_out"
    rg -Fq '"listener":{' /tmp/chimera_path_proof_selfcheck.json
    rg -Fq '"observed_public_ip":{' /tmp/chimera_path_proof_selfcheck.json
    rg -Fq '"results":[' /tmp/chimera_path_proof_selfcheck.json
    rg -Fq '"totals":{' /tmp/chimera_path_proof_selfcheck.json
    rm -f /tmp/chimera_path_proof_selfcheck.json

chimera-channel-audit:
    bash scripts/chimera_channel_audit.sh docs/CHIMERA_CHANNEL_AUDIT.json || true

chimera-channel-audit-selfcheck:
    test -x scripts/chimera_channel_audit.sh
    bash -n scripts/chimera_channel_audit.sh
    rg -q '"kind":"chimera_channel_audit"' scripts/chimera_channel_audit.sh
    rg -q '"network_state":"not_modified"' scripts/chimera_channel_audit.sh
    rg -q '"chimera"' scripts/chimera_channel_audit.sh
    rg -q '"path_proof"' scripts/chimera_channel_audit.sh
    rg -q '"selective_routing"' scripts/chimera_channel_audit.sh
    rg -q '"system_default_path"' scripts/chimera_channel_audit.sh

chimera-app-routes-selfcheck:
    test -f configs/chimera-app-routes.example.conf
    rg -q '^app:' configs/chimera-app-routes.example.conf
    rg -q '^service:' configs/chimera-app-routes.example.conf
    APP_ROUTES_FILE=configs/chimera-app-routes.example.conf bash scripts/chimera-control.sh app-routes-status | rg -q '^app_routes_count='
    APP_ROUTES_FILE=configs/chimera-app-routes.example.conf bash scripts/chimera-control.sh route-status | rg -q '^chimera_proxy_url='

chimera-runtime-verify:
    test -x scripts/chimera_runtime_verification.sh
    bash scripts/chimera_runtime_verification.sh

chimera-e2e-channel-gate:
    bash scripts/chimera_e2e_channel_gate.sh docs/CHIMERA_E2E_CHANNEL_GATE.json || true

chimera-e2e-channel-gate-selfcheck:
    test -x scripts/chimera_e2e_channel_gate.sh
    bash -n scripts/chimera_e2e_channel_gate.sh
    tmp_out=$(mktemp) ; \
      bash scripts/chimera_e2e_channel_gate.sh /tmp/chimera_e2e_channel_gate_selfcheck.json > "$tmp_out" 2>/dev/null || true ; \
      rg -q '"kind":"chimera_e2e_channel_gate"' "$tmp_out" ; \
      rm -f "$tmp_out"
    rg -Fq '"path_proof":{' /tmp/chimera_e2e_channel_gate_selfcheck.json
    rg -Fq '"channel_audit":{' /tmp/chimera_e2e_channel_gate_selfcheck.json
    rg -Fq '"selected_route_checks":{' /tmp/chimera_e2e_channel_gate_selfcheck.json
    rm -f /tmp/chimera_e2e_channel_gate_selfcheck.json

chimera-e2e-channel-gate-guard:
    test -x scripts/chimera_e2e_channel_gate_guard.sh
    bash scripts/chimera_e2e_channel_gate_guard.sh docs/CHIMERA_E2E_CHANNEL_GATE.json

chimera-e2e-channel-gate-guard-selfcheck:
    test -x scripts/chimera_e2e_channel_gate_guard.sh
    bash -n scripts/chimera_e2e_channel_gate_guard.sh
    rg -q 'status must be pass' scripts/chimera_e2e_channel_gate_guard.sh
    rg -q 'network_state must be not_modified' scripts/chimera_e2e_channel_gate_guard.sh
    rg -q 'stale artifact' scripts/chimera_e2e_channel_gate_guard.sh
    rg -q 'chimera e2e channel gate guard: PASS' scripts/chimera_e2e_channel_gate_guard.sh

upstream-resilience-smoke:
    bash scripts/upstream_resilience_smoke.sh docs/UPSTREAM_RESILIENCE_SMOKE.json

upstream-resilience-smoke-selfcheck:
    test -x scripts/upstream_resilience_smoke.sh
    bash -n scripts/upstream_resilience_smoke.sh
    tmp_json=$(mktemp) ; \
      bash scripts/upstream_resilience_smoke.sh "$tmp_json" >/dev/null ; \
      rg -q '"smoke":[[:space:]]*"upstream_resilience"' "$tmp_json" ; \
      rg -q '"outcome":[[:space:]]*"(pass|partial)"' "$tmp_json" ; \
      rg -Fq '"post": {' "$tmp_json" ; \
      rm -f "$tmp_json"

chimera-load-laptop:
    bash scripts/chimera_load_5m_laptop.sh docs/load

chimera-load-laptop-selfcheck:
    test -x scripts/chimera_load_5m_laptop.sh
    bash -n scripts/chimera_load_5m_laptop.sh
    rg -q 'CHIMERA_LAPTOP_HOST' scripts/chimera_load_5m_laptop.sh
    rg -q '__REMOTE_OUT__=' scripts/chimera_load_5m_laptop.sh
    rg -q 'local_artifact=' scripts/chimera_load_5m_laptop.sh

chimera-load-gate-laptop:
    bash scripts/chimera_load_gate_laptop.sh docs/CHIMERA_LOAD_GATE_LAPTOP.json

chimera-load-gate-laptop-selfcheck:
    test -x scripts/chimera_load_gate_laptop.sh
    bash -n scripts/chimera_load_gate_laptop.sh
    rg -q 'CHIMERA_LOAD_GATE_MIN_SUCCESS_RATE' scripts/chimera_load_gate_laptop.sh
    rg -q 'CHIMERA_LOAD_GATE_MIN_TOTAL_REQUESTS' scripts/chimera_load_gate_laptop.sh
    rg -q 'CHIMERA_LOAD_GATE_FORCE_FRESH' scripts/chimera_load_gate_laptop.sh
    rg -q 'CHIMERA_LOAD_GATE_MAX_AGE_SEC' scripts/chimera_load_gate_laptop.sh
    rg -q '"kind": "chimera_load_gate_laptop"' scripts/chimera_load_gate_laptop.sh

chimera-fresh-gate-report:
    bash scripts/chimera_fresh_gate_report.sh docs/CHIMERA_FRESH_GATE_REPORT.json

chimera-fresh-gate-report-selfcheck:
    test -x scripts/chimera_fresh_gate_report.sh
    bash -n scripts/chimera_fresh_gate_report.sh
    rg -q 'CHIMERA_PATH_PROOF_JSON' scripts/chimera_fresh_gate_report.sh
    rg -q 'CHIMERA_E2E_GATE_JSON' scripts/chimera_fresh_gate_report.sh
    rg -q 'CHIMERA_LOAD_GATE_JSON' scripts/chimera_fresh_gate_report.sh
    rg -q '"chimera_fresh_gate_report"' scripts/chimera_fresh_gate_report.sh

chimera-laptop-fresh-gate-sync:
    bash scripts/chimera_laptop_fresh_gate_sync.sh

chimera-laptop-fresh-gate-sync-selfcheck:
    test -x scripts/chimera_laptop_fresh_gate_sync.sh
    bash -n scripts/chimera_laptop_fresh_gate_sync.sh
    rg -q 'CHIMERA_LAPTOP_HOST' scripts/chimera_laptop_fresh_gate_sync.sh
    rg -q 'CHIMERA_LOAD_DURATION_SEC' scripts/chimera_laptop_fresh_gate_sync.sh
    rg -q 'CHIMERA_LOAD_GATE_MAX_AGE_SEC' scripts/chimera_laptop_fresh_gate_sync.sh
    rg -q 'sync_dir=' scripts/chimera_laptop_fresh_gate_sync.sh

chimera-desktop-hygiene-guard:
    bash scripts/chimera_desktop_hygiene_guard.sh

chimera-desktop-hygiene-guard-selfcheck:
    test -x scripts/chimera_desktop_hygiene_guard.sh
    bash -n scripts/chimera_desktop_hygiene_guard.sh
    rg -q 'third-party Chromium desktop file write detected' scripts/chimera_desktop_hygiene_guard.sh
    rg -q 'legacy chromium launcher hook detected' scripts/chimera_desktop_hygiene_guard.sh
    just chimera-desktop-hygiene-guard

chimera-ops-gate:
    just chimera-desktop-hygiene-guard-selfcheck
    just chimera-path-proof-selfcheck
    just chimera-channel-audit-selfcheck
    just chimera-e2e-channel-gate-selfcheck
    just chimera-e2e-channel-gate-guard-selfcheck
    just upstream-resilience-smoke-selfcheck
    just chimera-load-gate-laptop-selfcheck
    just chimera-runtime-verify
    just upstream-resilience-smoke
    just chimera-e2e-channel-gate
    just chimera-e2e-channel-gate-guard
    just chimera-load-gate-laptop

chimera-ops-gate-quiet:
    just chimera-desktop-hygiene-guard-selfcheck
    just chimera-path-proof-selfcheck
    just chimera-channel-audit-selfcheck
    just chimera-e2e-channel-gate-selfcheck
    just chimera-e2e-channel-gate-guard-selfcheck
    just upstream-resilience-smoke-selfcheck
    just chimera-load-gate-laptop-selfcheck
    CHIMERA_QUIET=1 just chimera-runtime-verify
    CHIMERA_QUIET=1 just upstream-resilience-smoke
    CHIMERA_QUIET=1 just chimera-e2e-channel-gate
    just chimera-e2e-channel-gate-guard
    just chimera-load-gate-laptop

chimera-ops-gate-fresh:
    just chimera-desktop-hygiene-guard-selfcheck
    just chimera-path-proof-selfcheck
    just chimera-channel-audit-selfcheck
    just chimera-e2e-channel-gate-selfcheck
    just chimera-e2e-channel-gate-guard-selfcheck
    just upstream-resilience-smoke-selfcheck
    just chimera-load-gate-laptop-selfcheck
    just chimera-fresh-gate-report-selfcheck
    CHIMERA_QUIET=1 just chimera-runtime-verify
    CHIMERA_QUIET=1 just upstream-resilience-smoke
    CHIMERA_QUIET=1 just chimera-e2e-channel-gate
    just chimera-e2e-channel-gate-guard
    CHIMERA_LOAD_GATE_FORCE_FRESH=1 just chimera-load-gate-laptop
    just chimera-fresh-gate-report

datapath-report-json:
    cargo run -p chimera-lab --bin chimera-lab -- datapath-report --json --out docs/datapath_latest.json

rollback-smoke:
    cargo run -p chimera-cli -- up --state-file docs/runtime_state_latest.json
    cargo run -p chimera-cli -- rollback status --state-file docs/runtime_state_latest.json
    cargo run -p chimera-cli -- rollback recover --state-file docs/runtime_state_latest.json
    cargo run -p chimera-cli -- rollback status --state-file docs/runtime_state_latest.json
    cargo run -p chimera-cli -- down --state-file docs/runtime_state_latest.json

rollback-json-smoke:
    cargo run -p chimera-cli -- up --state-file docs/runtime_state_latest.json
    cargo run -p chimera-cli -- rollback status --state-file docs/runtime_state_latest.json --json --out docs/rollback_status_latest.json
    cargo run -p chimera-cli -- rollback recover --state-file docs/runtime_state_latest.json --json --out docs/rollback_recover_latest.json
    cargo run -p chimera-cli -- rollback status --state-file docs/runtime_state_latest.json --json --out docs/rollback_status_after_recover_latest.json
    cargo run -p chimera-cli -- down --state-file docs/runtime_state_latest.json

runtime-apply-dns-smoke:
    printf '%s\n' 'carrier.profile = tls' 'gateway.listen_addr = 127.0.0.1:18443' 'rekey.max_age_seconds = 300' 'rekey.max_packets_per_key = 10000' > /tmp/chimera_gateway_smoke.conf
    printf '%s\n' 'carrier.profile = tls' 'carrier.addr = 127.0.0.1:18443' 'carrier.server_name = gateway.local' 'capture.mode = auto' 'capture.tun_supported = true' 'rekey.max_age_seconds = 300' 'rekey.max_packets_per_key = 10000' > /tmp/chimera_client_smoke.conf
    printf '%s\n' '# smoketest' 'nameserver 8.8.8.8' > /tmp/chimera_resolv_smoke.conf
    CHIMERA_GATEWAY_IDLE_EXIT_MS=6000 cargo run -q -p chimera-gateway -- run --config /tmp/chimera_gateway_smoke.conf > /tmp/chimera_gateway_smoke.log 2>&1 &
    sleep 0.5
    cargo run -q -p chimera-cli -- up --state-file /tmp/chimera_runtime_apply_state.json --config /tmp/chimera_client_smoke.conf --apply-dns true --dns-server 9.9.9.9 --resolv-conf /tmp/chimera_resolv_smoke.conf
    rg -q '"network_state":"modified"' /tmp/chimera_runtime_apply_state.json
    rg -q 'nameserver 9.9.9.9' /tmp/chimera_resolv_smoke.conf
    cargo run -q -p chimera-cli -- down --state-file /tmp/chimera_runtime_apply_state.json
    rg -q 'nameserver 8.8.8.8' /tmp/chimera_resolv_smoke.conf
    test ! -f /tmp/chimera_runtime_apply_state.json
    printf '%s\n' '{"status":"ok","kind":"runtime_apply_dns_smoke","message_en":"Runtime apply DNS smoke passed.","message_ru":"Smoke-проверка runtime apply DNS пройдена.","network_state":"modified","rollback_ok":true}' > docs/RUNTIME_APPLY_DNS_SMOKE.json

runtime-apply-route-smoke:
    bash scripts/runtime_apply_route_smoke.sh

runtime-apply-route-existing-tun-smoke:
    bash scripts/runtime_apply_route_existing_tun_smoke.sh

runtime-apply-route-multi-cidr-smoke:
    bash scripts/runtime_apply_route_multi_cidr_smoke.sh

runtime-route-policy-validation-smoke:
    bash scripts/runtime_route_policy_validation_smoke.sh

runtime-route-duplicate-cidr-validation-smoke:
    bash scripts/runtime_route_duplicate_cidr_validation_smoke.sh

runtime-tun-name-validation-smoke:
    bash scripts/runtime_tun_name_validation_smoke.sh

runtime-resolv-conf-validation-smoke:
    bash scripts/runtime_resolv_conf_validation_smoke.sh

runtime-datapath-multiflow-smoke:
    bash scripts/runtime_datapath_multiflow_smoke.sh

runtime-policy-precedence-smoke:
    bash scripts/runtime_policy_precedence_smoke.sh

runtime-forced-stop-rollback-smoke:
    bash scripts/runtime_forced_stop_rollback_smoke.sh

rust-no-hardcode-guard:
    cargo run -q -p chimera-lab --bin rust_no_hardcode_guard

rust-no-hardcode-guard-selfcheck:
    rg -q '^#!\[forbid\(unsafe_code\)\]$' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    rg -q 'python source found in project tree' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    rg -q 'python usage found in runtime smoke scripts' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    rg -q 'python usage found in scripts/\*\.sh' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    rg -q 'machine/resource-specific literal found' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    rg -q 'python execution found in justfile' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    rg -q 'python command found in docs/README' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    rg -Fq 'resolve_non_empty_setting("CHIMERA_REAL_WORLD_DIRECT_URL"' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    rg -Fq 'resolve_non_empty_setting("CHIMERA_REAL_WORLD_BLOCKED_TARGETS"' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    rg -Fq 'resolve_non_empty_setting("CHIMERA_REAL_WORLD_PROXY_URL"' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    rg -q 'CHIMERA_REAL_WORLD_PROXY_CANDIDATES' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    rg -q 'CHIMERA_REAL_WORLD_DIRECT_TIMEOUT_SEC' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    rg -q 'CHIMERA_REAL_WORLD_PROXY_TIMEOUT_SEC' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    rg -q 'CHIMERA_REAL_WORLD_CONNECT_TIMEOUT_MS' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    rg -q 'baked runtime target found in runtime_real_world_probe.rs' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    rg -q 'baked URL/proxy endpoint found in runtime Rust bins' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    rg -q 'ws://' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    rg -q 'wss://' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    rg -q 'cargo run -q -p chimera-lab --bin -- lab health' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    rg -q 'ambiguous cargo run for chimera-lab \(missing --bin\)' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    rg -q 'rust/no-hardcode guard: PASS' crates/chimera-lab/src/bin/rust_no_hardcode_guard.rs
    cargo run -q -p chimera-lab --bin rust_no_hardcode_guard

peer-egress-perf-smoke bytes="67108864" pool="16":
    cargo run -q -p chimera-carrier --bin chimera-peer-egress -- --mode bench --token bench --pool "{{pool}}" --bench-bytes "{{bytes}}"

peer-egress-perf-smoke-selfcheck:
    cargo test -q -p chimera-carrier --bin chimera-peer-egress
    cargo run -q -p chimera-carrier --bin chimera-peer-egress -- --mode bench --token bench --pool 8 --bench-bytes 16777216 | rg -q 'chimera_peer_egress_bench=pass'

runtime-real-world-probe-smoke:
    bash scripts/runtime_real_world_probe_smoke.sh

runtime-real-world-probe-smoke-selfcheck:
    test -x scripts/runtime_real_world_probe_smoke.sh
    bash -n scripts/runtime_real_world_probe_smoke.sh
    rg -q 'cargo run -q -p chimera-lab --bin runtime_real_world_probe' scripts/runtime_real_world_probe_smoke.sh
    test -f crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q 'runtime_real_world_probe_smoke' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q '^fn resolve_non_empty_setting' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q '^fn format_blocked_targets_csv' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q '^fn is_supported_probe_url' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q '^fn is_supported_proxy_url' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q '^fn extract_authority' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q 'let authority = extract_authority\(rest\);' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q 'authority_has_non_empty_host\(authority\)' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q 'let auth = extract_authority\(rest\);' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q 'parse_blocked_targets_dedups_case_insensitive_and_trims' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q 'format_blocked_targets_csv_preserves_normalized_order' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q 'resolve_non_empty_setting_trims_and_rejects_empty' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q 'with_query = format!\("\{\}://\{\}", "socks5h", "127.0.0.1:11080\?via=1"\)' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q 'with_fragment = format!\("\{\}://\{\}", "socks5h", "127.0.0.1:11080#v1"\)' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q 'with_path_and_auth = format!\("\{\}://\{\}", "socks5h", "user:pass@localhost:1080/proxy"\)' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q 'extract_authority_stops_on_path_query_and_fragment' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q 'supported_probe_url_requires_http_or_https' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -Fq 'let ws_target = format!("{}://{}", "ws", "example.invalid");' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -Fq 'let wss_target = format!("{}://{}", "wss", "example.invalid");' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q 'supported_proxy_url_requires_scheme_and_authority' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q 'invalid_spaced_authority' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q 'proxy_blocked_targets_total' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q 'direct_timeout_sec' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q 'proxy_timeout_sec' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q 'connect_timeout_ms' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q '^fn parse_u64_setting_with_min' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q '^fn parse_proxy_candidates_csv' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q '^fn build_proxy_candidates' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q 'Command::new\("ss"\)' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    rg -q 'arg\("-ltnH"\)' crates/chimera-lab/src/bin/runtime_real_world_probe.rs
    test -f crates/chimera-lab/src/bin/runtime_real_world_probe_env.rs
    rg -q '^fn normalize_proxy_probe_error' crates/chimera-lab/src/bin/runtime_real_world_probe_env.rs
    rg -q 'normalize_proxy_probe_error_allows_only_known_values' crates/chimera-lab/src/bin/runtime_real_world_probe_env.rs
    rg -q '^fn normalize_blocked_target_totals' crates/chimera-lab/src/bin/runtime_real_world_probe_env.rs
    rg -q 'normalize_blocked_target_totals_clamps_negative_and_overflow' crates/chimera-lab/src/bin/runtime_real_world_probe_env.rs

runtime-real-world-probe-schema-guard:
    bash scripts/runtime_real_world_probe_schema_guard.sh docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json

runtime-real-world-probe-schema-guard-selfcheck:
    test -x scripts/runtime_real_world_probe_schema_guard.sh
    bash -n scripts/runtime_real_world_probe_schema_guard.sh
    rg -q 'cargo run -q -p chimera-lab --bin runtime_real_world_probe_schema_guard --' scripts/runtime_real_world_probe_schema_guard.sh
    test -f crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'probe keys mismatch' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'probe string is empty:' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'probe direct_url must use http/https' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'proxy blocked totals mismatch' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'direct_timeout_sec' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'proxy_timeout_sec' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'connect_timeout_ms' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'blocked_targets csv is not normalized' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'blocked_targets contains non-http/https url' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'proxy probe not attempted must have empty target rows' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q '^fn normalize_blocked_targets_csv' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q '^fn is_supported_probe_url' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q '^fn is_supported_proxy_url' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q '^fn extract_authority' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'let authority = extract_authority\(rest\);' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'authority_has_non_empty_host\(authority\)' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'validate_probe_rejects_non_normalized_blocked_targets_csv' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'validate_probe_rejects_not_attempted_with_non_empty_rows' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'extract_authority_stops_on_path_query_and_fragment' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'validate_probe_rejects_empty_required_strings' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'validate_probe_rejects_non_http_direct_url' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'ws://blocked1.example' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'wss://blocked1.example' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'validate_probe_rejects_non_http_blocked_target_url' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'validate_probe_rejects_proxy_url_with_empty_authority' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'supported_proxy_url_requires_non_blank_non_spaced_authority' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'https://bad host' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs
    rg -q 'runtime real-world probe schema guard: PASS' crates/chimera-lab/src/bin/runtime_real_world_probe_schema_guard.rs

mesh-cli-recovery-schema-guard:
    bash scripts/mesh_cli_recovery_schema_guard.sh

mesh-cli-recovery-schema-guard-selfcheck:
    test -x scripts/mesh_cli_recovery_schema_guard.sh
    bash -n scripts/mesh_cli_recovery_schema_guard.sh
    rg -q 'cargo test -q -p chimera-cli tests_json_operator_cross_contract' scripts/mesh_cli_recovery_schema_guard.sh
    rg -q 'cargo test -q -p chimera-cli tests_json_success_presence' scripts/mesh_cli_recovery_schema_guard.sh
    rg -q 'cargo test -q -p chimera-cli tests_json_error_contract' scripts/mesh_cli_recovery_schema_guard.sh
    rg -q 'docs/MESH_ROUTE_EXPLAIN_ERROR.json' scripts/mesh_cli_recovery_schema_guard.sh
    rg -q 'cargo run -q -p chimera-lab --bin mesh_cli_recovery_schema_guard -- docs/MESH_ROUTE_EXPLAIN.json' scripts/mesh_cli_recovery_schema_guard.sh
    rg -q 'cargo run -q -p chimera-lab --bin mesh_cli_recovery_schema_guard -- docs/MESH_ROUTE_EXPLAIN_ERROR.json' scripts/mesh_cli_recovery_schema_guard.sh
    test -f crates/chimera-lab/src/bin/mesh_cli_recovery_schema_guard.rs
    rg -q 'EXPECTED_SCHEMA_VERSION' crates/chimera-lab/src/bin/mesh_cli_recovery_schema_guard.rs
    rg -q 'EXPECTED_FIELDS_CHECKSUM' crates/chimera-lab/src/bin/mesh_cli_recovery_schema_guard.rs
    rg -q 'status must be ok|error' crates/chimera-lab/src/bin/mesh_cli_recovery_schema_guard.rs
    rg -q 'kind does not match status' crates/chimera-lab/src/bin/mesh_cli_recovery_schema_guard.rs
    rg -q 'connect_recovery_needed does not match operator action' crates/chimera-lab/src/bin/mesh_cli_recovery_schema_guard.rs
    rg -q 'connect_recovery_strategy does not match recovery-needed state' crates/chimera-lab/src/bin/mesh_cli_recovery_schema_guard.rs
    rg -q 'connect_recovery_projection_consistency is false' crates/chimera-lab/src/bin/mesh_cli_recovery_schema_guard.rs
    rg -q 'connect_recovery_projection_key mismatch' crates/chimera-lab/src/bin/mesh_cli_recovery_schema_guard.rs
    rg -q 'error envelope final result mismatch' crates/chimera-lab/src/bin/mesh_cli_recovery_schema_guard.rs
    rg -q 'validate_rejects_ok_without_explain' crates/chimera-lab/src/bin/mesh_cli_recovery_schema_guard.rs
    rg -q 'validate_rejects_kind_status_mismatch' crates/chimera-lab/src/bin/mesh_cli_recovery_schema_guard.rs
    cargo test -q -p chimera-lab --bin mesh_cli_recovery_schema_guard
    rg -q 'mesh cli recovery schema guard: PASS' scripts/mesh_cli_recovery_schema_guard.sh

probe-access-env-selfcheck:
    test -f crates/chimera-lab/src/bin/probe_access_env.rs
    rg -q '^#!\[forbid\(unsafe_code\)\]$' crates/chimera-lab/src/bin/probe_access_env.rs
    rg -q 'usage: probe_access_env <json_path>' crates/chimera-lab/src/bin/probe_access_env.rs
    rg -q 'runtime_probe_access_smoke_ok' crates/chimera-lab/src/bin/probe_access_env.rs
    rg -q 'exports_ok_when_contract_is_green' crates/chimera-lab/src/bin/probe_access_env.rs
    rg -q 'exports_false_when_threshold_exceeded' crates/chimera-lab/src/bin/probe_access_env.rs
    cargo test -q -p chimera-lab --bin probe_access_env

reality-audit-refresh:
    bash scripts/reality_audit_refresh.sh

reality-audit-refresh-selfcheck:
    test -x scripts/reality_audit_refresh.sh
    bash -n scripts/reality_audit_refresh.sh
    rg -q 'cargo run -q -p chimera-lab --bin reality_audit_refresh --' scripts/reality_audit_refresh.sh
    test -f crates/chimera-lab/src/bin/reality_audit_refresh.rs
    rg -q 'real_world_datapath_closed' crates/chimera-lab/src/bin/reality_audit_refresh.rs
    rg -q 'runtime_probe_blocked_targets_total' crates/chimera-lab/src/bin/reality_audit_refresh.rs
    rg -q 'runtime_probe_skipped_no_proxy_listener' crates/chimera-lab/src/bin/reality_audit_refresh.rs
    rg -q 'runtime_probe_blocked_targets_failed' crates/chimera-lab/src/bin/reality_audit_refresh.rs
    rg -q 'runtime_probe_proxy_error' crates/chimera-lab/src/bin/reality_audit_refresh.rs
    rg -q 'runtime_probe_path_ok' crates/chimera-lab/src/bin/reality_audit_refresh.rs

reality-audit-schema-guard:
    bash scripts/reality_audit_schema_guard.sh docs/REALITY_AUDIT_LATEST.json

reality-audit-schema-guard-selfcheck:
    test -x scripts/reality_audit_schema_guard.sh
    bash -n scripts/reality_audit_schema_guard.sh
    rg -q 'cargo run -q -p chimera-lab --bin reality_audit_schema_guard --' scripts/reality_audit_schema_guard.sh
    test -f crates/chimera-lab/src/bin/reality_audit_schema_guard.rs
    rg -q 'reality audit schema guard: PASS' crates/chimera-lab/src/bin/reality_audit_schema_guard.rs
    rg -q 'runtime_evidence_closed mismatch' crates/chimera-lab/src/bin/reality_audit_schema_guard.rs
    rg -q 'real_world_datapath_closed mismatch' crates/chimera-lab/src/bin/reality_audit_schema_guard.rs
    rg -q 'runtime_probe_blocked_targets totals mismatch' crates/chimera-lab/src/bin/reality_audit_schema_guard.rs
    rg -q 'runtime_probe_proxy_error invalid' crates/chimera-lab/src/bin/reality_audit_schema_guard.rs
    rg -q 'runtime_probe proxy attempted without listener' crates/chimera-lab/src/bin/reality_audit_schema_guard.rs
    rg -q 'runtime_probe listener detected but skipped_no_proxy_listener=true' crates/chimera-lab/src/bin/reality_audit_schema_guard.rs
    rg -q 'runtime_probe not attempted must set skipped_no_proxy_listener=true' crates/chimera-lab/src/bin/reality_audit_schema_guard.rs
    rg -q 'runtime_probe attempted must set skipped_no_proxy_listener=false' crates/chimera-lab/src/bin/reality_audit_schema_guard.rs
    rg -q 'runtime_probe proxy not attempted must be listener_not_found' crates/chimera-lab/src/bin/reality_audit_schema_guard.rs
    rg -q 'runtime_probe proxy attempted with listener_not_found' crates/chimera-lab/src/bin/reality_audit_schema_guard.rs
    rg -q 'runtime_probe proxy ok requires ok==total' crates/chimera-lab/src/bin/reality_audit_schema_guard.rs

reality-ship-sync-guard:
    bash scripts/reality_ship_sync_guard.sh \
      docs/REALITY_AUDIT_LATEST.json \
      docs/SHIP_READINESS_REPORT.json \
      docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json

reality-ship-sync-guard-selfcheck:
    test -x scripts/reality_ship_sync_guard.sh
    bash -n scripts/reality_ship_sync_guard.sh
    rg -q 'cargo run -q -p chimera-lab --bin reality_ship_sync_guard --' scripts/reality_ship_sync_guard.sh
    test -f crates/chimera-lab/src/bin/reality_ship_sync_guard.rs
    rg -q 'reality ship sync guard: PASS' crates/chimera-lab/src/bin/reality_ship_sync_guard.rs
    rg -q 'bool mismatch' crates/chimera-lab/src/bin/reality_ship_sync_guard.rs
    rg -q 'int mismatch' crates/chimera-lab/src/bin/reality_ship_sync_guard.rs
    rg -q 'str mismatch' crates/chimera-lab/src/bin/reality_ship_sync_guard.rs
    rg -q 'runtime_probe_blocked_targets_failed' crates/chimera-lab/src/bin/reality_ship_sync_guard.rs
    rg -q 'proxy attempted without listener' crates/chimera-lab/src/bin/reality_ship_sync_guard.rs
    rg -q 'proxy not attempted must be listener_not_found' crates/chimera-lab/src/bin/reality_ship_sync_guard.rs
    rg -q 'proxy attempted with listener_not_found' crates/chimera-lab/src/bin/reality_ship_sync_guard.rs
    rg -q 'proxy ok with failed targets' crates/chimera-lab/src/bin/reality_ship_sync_guard.rs
    rg -q 'proxy attempted with empty target totals' crates/chimera-lab/src/bin/reality_ship_sync_guard.rs
    rg -q 'proxy not attempted with non-zero totals' crates/chimera-lab/src/bin/reality_ship_sync_guard.rs
    rg -q 'proxy error value is invalid' crates/chimera-lab/src/bin/reality_ship_sync_guard.rs
    rg -q 'proxy totals mismatch' crates/chimera-lab/src/bin/reality_ship_sync_guard.rs

runtime-apply-route-smoke-selfcheck:
    test -x scripts/runtime_apply_route_smoke.sh
    bash -n scripts/runtime_apply_route_smoke.sh
    rg -q 'unshare -Urn' scripts/runtime_apply_route_smoke.sh
    rg -q 'apply_attempt_ok' scripts/runtime_apply_route_smoke.sh
    rg -q 'policy_rule_ok' scripts/runtime_apply_route_smoke.sh
    rg -q 'rollback_ok' scripts/runtime_apply_route_smoke.sh
    rg -q 'route-policy true' scripts/runtime_apply_route_smoke.sh
    rg -q 'route-table 60001' scripts/runtime_apply_route_smoke.sh
    rg -q 'route-rule-priority 12000' scripts/runtime_apply_route_smoke.sh
    rg -q 'ip rule show' scripts/runtime_apply_route_smoke.sh
    rg -q 'ip route show table 60001' scripts/runtime_apply_route_smoke.sh

runtime-apply-route-existing-tun-smoke-selfcheck:
    test -x scripts/runtime_apply_route_existing_tun_smoke.sh
    bash -n scripts/runtime_apply_route_existing_tun_smoke.sh
    rg -q 'unshare -Urn' scripts/runtime_apply_route_existing_tun_smoke.sh
    rg -q 'ip tuntap add dev chimera-pre0 mode tun' scripts/runtime_apply_route_existing_tun_smoke.sh
    rg -Fq 'rg -q "\"tun_applied\":false"' scripts/runtime_apply_route_existing_tun_smoke.sh
    rg -q 'preexisting_tun_used' scripts/runtime_apply_route_existing_tun_smoke.sh
    rg -q 'route-table 60002' scripts/runtime_apply_route_existing_tun_smoke.sh
    rg -q 'route-rule-priority 12010' scripts/runtime_apply_route_existing_tun_smoke.sh

runtime-apply-route-multi-cidr-smoke-selfcheck:
    test -x scripts/runtime_apply_route_multi_cidr_smoke.sh
    bash -n scripts/runtime_apply_route_multi_cidr_smoke.sh
    rg -q 'unshare -Urn' scripts/runtime_apply_route_multi_cidr_smoke.sh
    rg -q 'apply_attempt_ok' scripts/runtime_apply_route_multi_cidr_smoke.sh
    rg -q 'policy_rule_ok' scripts/runtime_apply_route_multi_cidr_smoke.sh
    rg -q 'rollback_ok' scripts/runtime_apply_route_multi_cidr_smoke.sh
    rg -q 'route-policy true' scripts/runtime_apply_route_multi_cidr_smoke.sh
    rg -q '203.0.113.0/24,198.51.100.0/24' scripts/runtime_apply_route_multi_cidr_smoke.sh
    rg -q 'route-table 60011' scripts/runtime_apply_route_multi_cidr_smoke.sh
    rg -q 'route-rule-priority 12110' scripts/runtime_apply_route_multi_cidr_smoke.sh
    rg -q 'ip rule show' scripts/runtime_apply_route_multi_cidr_smoke.sh
    rg -q 'ip route show table 60011' scripts/runtime_apply_route_multi_cidr_smoke.sh

runtime-route-policy-validation-smoke-selfcheck:
    test -x scripts/runtime_route_policy_validation_smoke.sh
    bash -n scripts/runtime_route_policy_validation_smoke.sh
    rg -q 'route-policy true' scripts/runtime_route_policy_validation_smoke.sh
    rg -q 'route-table main' scripts/runtime_route_policy_validation_smoke.sh
    rg -q 'apply_rejected' scripts/runtime_route_policy_validation_smoke.sh
    rg -q 'state_not_created' scripts/runtime_route_policy_validation_smoke.sh

runtime-route-duplicate-cidr-validation-smoke-selfcheck:
    test -x scripts/runtime_route_duplicate_cidr_validation_smoke.sh
    bash -n scripts/runtime_route_duplicate_cidr_validation_smoke.sh
    rg -q 'route-policy true' scripts/runtime_route_duplicate_cidr_validation_smoke.sh
    rg -q 'route-table 60012' scripts/runtime_route_duplicate_cidr_validation_smoke.sh
    rg -q 'route-rule-priority 12120' scripts/runtime_route_duplicate_cidr_validation_smoke.sh
    rg -q '203.0.113.0/24,203.0.113.0/24' scripts/runtime_route_duplicate_cidr_validation_smoke.sh
    rg -q 'apply_rejected' scripts/runtime_route_duplicate_cidr_validation_smoke.sh
    rg -q 'state_not_created' scripts/runtime_route_duplicate_cidr_validation_smoke.sh

runtime-tun-name-validation-smoke-selfcheck:
    test -x scripts/runtime_tun_name_validation_smoke.sh
    bash -n scripts/runtime_tun_name_validation_smoke.sh
    rg -q 'route-policy true' scripts/runtime_tun_name_validation_smoke.sh
    rg -q 'route-table 60003' scripts/runtime_tun_name_validation_smoke.sh
    rg -q 'route-rule-priority 12020' scripts/runtime_tun_name_validation_smoke.sh
    rg -q 'tun-name bad@if' scripts/runtime_tun_name_validation_smoke.sh
    rg -q 'apply_rejected' scripts/runtime_tun_name_validation_smoke.sh
    rg -q 'state_not_created' scripts/runtime_tun_name_validation_smoke.sh

runtime-resolv-conf-validation-smoke-selfcheck:
    test -x scripts/runtime_resolv_conf_validation_smoke.sh
    bash -n scripts/runtime_resolv_conf_validation_smoke.sh
    rg -q 'apply-dns true' scripts/runtime_resolv_conf_validation_smoke.sh
    rg -q 'dns-server 9.9.9.9' scripts/runtime_resolv_conf_validation_smoke.sh
    rg -q 'resolv-conf relative/not-allowed.conf' scripts/runtime_resolv_conf_validation_smoke.sh
    rg -q 'apply_rejected' scripts/runtime_resolv_conf_validation_smoke.sh
    rg -q 'state_not_created' scripts/runtime_resolv_conf_validation_smoke.sh

runtime-datapath-multiflow-smoke-selfcheck:
    test -x scripts/runtime_datapath_multiflow_smoke.sh
    bash -n scripts/runtime_datapath_multiflow_smoke.sh
    rg -q 'chimera-lab -- datapath-report --json' scripts/runtime_datapath_multiflow_smoke.sh
    rg -q '"gateway_explain":"matched rule' scripts/runtime_datapath_multiflow_smoke.sh
    rg -q '"block_explain":"matched rule' scripts/runtime_datapath_multiflow_smoke.sh
    rg -q '"direct_explain":"matched rule' scripts/runtime_datapath_multiflow_smoke.sh
    rg -q 'gateway_ok' scripts/runtime_datapath_multiflow_smoke.sh
    rg -q 'block_ok' scripts/runtime_datapath_multiflow_smoke.sh
    rg -q 'direct_ok' scripts/runtime_datapath_multiflow_smoke.sh

runtime-policy-precedence-smoke-selfcheck:
    test -x scripts/runtime_policy_precedence_smoke.sh
    bash -n scripts/runtime_policy_precedence_smoke.sh
    rg -q 'exact-direct = exact:video.example.org => direct' scripts/runtime_policy_precedence_smoke.sh
    rg -q 'suffix-gateway = suffix:example.org => gateway' scripts/runtime_policy_precedence_smoke.sh
    rg -q -- '--show-all-matches' scripts/runtime_policy_precedence_smoke.sh
    rg -q -- '--dns-bind-domain blocked.example.org' scripts/runtime_policy_precedence_smoke.sh
    rg -q -- '--dns-bind-ip 203.0.113.77' scripts/runtime_policy_precedence_smoke.sh
    rg -q 'precedence_ok' scripts/runtime_policy_precedence_smoke.sh
    rg -q 'all_matches_ok' scripts/runtime_policy_precedence_smoke.sh
    rg -q 'dns_binding_ok' scripts/runtime_policy_precedence_smoke.sh

runtime-forced-stop-rollback-smoke-selfcheck:
    test -x scripts/runtime_forced_stop_rollback_smoke.sh
    bash -n scripts/runtime_forced_stop_rollback_smoke.sh
    rg -q 'unshare -Urn' scripts/runtime_forced_stop_rollback_smoke.sh
    rg -q 'rollback recover' scripts/runtime_forced_stop_rollback_smoke.sh
    rg -q 'route-table 60010' scripts/runtime_forced_stop_rollback_smoke.sh
    rg -q 'route-rule-priority 12100' scripts/runtime_forced_stop_rollback_smoke.sh
    rg -q 'apply_attempt_ok' scripts/runtime_forced_stop_rollback_smoke.sh
    rg -q 'recover_ok' scripts/runtime_forced_stop_rollback_smoke.sh
    rg -q 'down_state_clean' scripts/runtime_forced_stop_rollback_smoke.sh

ship-readiness-selfcheck:
    test -x scripts/ship_readiness.sh
    bash -n scripts/ship_readiness.sh
    rg -q 'artifacts_fresh' scripts/ship_readiness.sh
    rg -q 'freshness_check' scripts/ship_readiness.sh
    rg -q 'runtime_apply_route_smoke_selfcheck' scripts/ship_readiness.sh
    rg -q 'runtime_apply_route_multi_cidr_smoke_selfcheck' scripts/ship_readiness.sh
    rg -q 'runtime_apply_route_multi_cidr_smoke' scripts/ship_readiness.sh
    rg -q 'runtime_route_duplicate_cidr_validation_smoke_selfcheck' scripts/ship_readiness.sh
    rg -q 'runtime_route_duplicate_cidr_validation_smoke' scripts/ship_readiness.sh
    rg -q 'just rust-no-hardcode-guard-selfcheck' scripts/ship_readiness.sh
    rg -q 'just rust-no-hardcode-guard' scripts/ship_readiness.sh
    rg -q 'just cef-phase1-smoke' scripts/ship_readiness.sh
    rg -q 'just benchmark-regression-check' scripts/ship_readiness.sh
    rg -q 'just cef-track-report' scripts/ship_readiness.sh
    rg -q 'just cef-track-guard' scripts/ship_readiness.sh
    rg -q 'just cef-track-sync-guard' scripts/ship_readiness.sh
    rg -q 'just cef-gap-map-guard' scripts/ship_readiness.sh
    rg -q 'just cef-consistency-guard' scripts/ship_readiness.sh
    line_track=$(grep -n '^just cef-track-guard$' scripts/ship_readiness.sh | cut -d: -f1); line_track_sync=$(grep -n '^just cef-track-sync-guard$' scripts/ship_readiness.sh | cut -d: -f1); line_gap=$(grep -n '^just cef-gap-map-guard$' scripts/ship_readiness.sh | cut -d: -f1); line_phase1=$(grep -n '^just cef-phase1-smoke$' scripts/ship_readiness.sh | cut -d: -f1); test -n "$line_track" && test -n "$line_track_sync" && test -n "$line_gap" && test -n "$line_phase1" && test "$line_track" -lt "$line_track_sync" && test "$line_track_sync" -lt "$line_gap" && test "$line_gap" -lt "$line_phase1"
    rg -q 'just release-readiness-report-ru' scripts/ship_readiness.sh
    rg -q 'just report-pack-json' scripts/ship_readiness.sh
    rg -q 'just report-pack$' scripts/ship_readiness.sh
    rg -q 'just reality-ship-sync-guard-selfcheck' scripts/ship_readiness.sh
    rg -q 'just reality-ship-sync-guard' scripts/ship_readiness.sh
    rg -q 'cef_phase1_smoke_ok' scripts/ship_readiness.sh
    rg -q '"report_pack_json":true' scripts/ship_readiness.sh
    rg -q '"cef_track_report":true' scripts/ship_readiness.sh
    rg -q '"cef_track_guard":true' scripts/ship_readiness.sh
    rg -q '"cef_track_sync_guard":true' scripts/ship_readiness.sh
    rg -q '"cef_gap_map_guard":' scripts/ship_readiness.sh
    rg -q '"cef_consistency_guard":true' scripts/ship_readiness.sh
    rg -qv 'cef_gap_map_present' scripts/ship_readiness.sh
    rg -q '&& "\$cef_gap_map_guard" == "true"' scripts/ship_readiness.sh
    rg -q '"benchmark_regression_gate":true' scripts/ship_readiness.sh
    rg -q '"release_readiness_report_ru":true' scripts/ship_readiness.sh
    rg -q '"report_pack_md":true' scripts/ship_readiness.sh
    rg -q '"cef_phase1_smoke":true' scripts/ship_readiness.sh
    rg -q 'just runtime-forced-stop-rollback-smoke-selfcheck' scripts/ship_readiness.sh
    rg -q 'just runtime-forced-stop-rollback-smoke' scripts/ship_readiness.sh
    rg -q 'just mesh-cli-recovery-schema-guard-selfcheck' scripts/ship_readiness.sh
    rg -q 'just mesh-cli-recovery-schema-guard' scripts/ship_readiness.sh
    rg -q '"runtime_forced_stop_rollback_smoke_ok":' scripts/ship_readiness.sh
    rg -q '"runtime_apply_route_multi_cidr_smoke_ok":' scripts/ship_readiness.sh
    rg -q '"runtime_forced_stop_rollback_smoke_selfcheck":true' scripts/ship_readiness.sh
    rg -q '"runtime_forced_stop_rollback_smoke":true' scripts/ship_readiness.sh
    rg -q '"reality_ship_sync_guard_selfcheck":true' scripts/ship_readiness.sh
    rg -q '"reality_ship_sync_guard":true' scripts/ship_readiness.sh
    rg -q '^deny:' justfile
    rg -q 'crates/chimera-mesh/src/policy_parse.rs:220' scripts/anti_monolith_guard.sh
    rg -q 'bash scripts/cargo_deny_guard.sh' justfile
    test -x scripts/cargo_deny_guard.sh
    bash -n scripts/cargo_deny_guard.sh
    rg -q 'CHIMERA_ADVISORY_SOURCE_ROOT' scripts/cargo_deny_guard.sh
    rg -q 'CHIMERA_ADVISORY_TARGET_ROOT' scripts/cargo_deny_guard.sh
    rg -q 'cargo deny check --disable-fetch' scripts/cargo_deny_guard.sh
    rg -q 'cargo-deny guard: PASS' scripts/cargo_deny_guard.sh
    rg -q 'runtime_real_world_probe_env' scripts/ship_readiness.sh
    test -f crates/chimera-lab/src/bin/runtime_real_world_probe_env.rs
    rg -q 'runtime_real_world_proxy_probe_error' crates/chimera-lab/src/bin/runtime_real_world_probe_env.rs
    rg -q 'runtime_real_world_proxy_blocked_targets_total' crates/chimera-lab/src/bin/runtime_real_world_probe_env.rs
    rg -q 'GENERATED_AT_UTC=' scripts/ship_readiness.sh
    rg -Fq '"generated_at":"${GENERATED_AT_UTC}"' scripts/ship_readiness.sh
    rg -Fq 'Generated at (UTC): \`${GENERATED_AT_UTC}\`' scripts/ship_readiness.sh
    rg -q 'docs/benchmark_latest.json' scripts/ship_readiness.sh
    rg -q '^truth-contract-check:' justfile
    just truth-contract-selfcheck
    rg -q 'just truth-contract-check' justfile
    rg -q 'benchmark-regression-selfcheck' justfile
    rg -q 'ship-readiness-json-guard-selfcheck' justfile
    rg -q 'ship-readiness-json-guard' justfile
    rg -q 'ship-readiness-freshness-guard-selfcheck' justfile
    rg -q 'ship-readiness-freshness-guard' justfile
    rg -q 'json-no-dupe-guard-selfcheck' justfile
    rg -q 'json-no-dupe-guard' justfile

cleanroom-handoff-selfcheck:
    test -x scripts/cleanroom_handoff_check.sh
    bash -n scripts/cleanroom_handoff_check.sh
    rg -q 'just handoff-check' scripts/cleanroom_handoff_check.sh
    rg -q 'just benchmark-regression-selfcheck' scripts/cleanroom_handoff_check.sh
    rg -q 'apply_attempt_ok' scripts/cleanroom_handoff_check.sh
    rg -q 'RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json' scripts/cleanroom_handoff_check.sh
    rg -q 'forced_stop_smoke_ok' scripts/cleanroom_handoff_check.sh
    rg -q 'forced_stop_recover' scripts/cleanroom_handoff_check.sh
    rg -q 'recover_ok' scripts/cleanroom_handoff_check.sh
    rg -q 'down_state_clean' scripts/cleanroom_handoff_check.sh

client-health:
    cargo run -p chimera-cli -- health --config configs/client.example.conf

client-doctor:
    cargo run -p chimera-cli -- doctor --config configs/client.example.conf --json --out docs/doctor_latest.json

gateway-health:
    cargo run -p chimera-gateway -- health --config configs/gateway.example.conf

gateway-doctor:
    cargo run -p chimera-gateway -- doctor --config configs/gateway.example.conf --json --out docs/gateway_doctor_latest.json

fuzz-smoke:
    cargo run -p chimera-lab --bin chimera-lab -- fuzz-smoke

fuzz-targets-check:
    cargo check --manifest-path fuzz/Cargo.toml

perf-smoke:
    cargo run -p chimera-lab --bin chimera-lab -- perf-smoke

net-sim:
    cargo run -p chimera-lab --bin chimera-lab -- net-sim

cef-track-report:
    bash scripts/cef_track_report.sh docs/CEF_TRACK_REPORT.json

cef-track-guard:
    bash scripts/cef_track_guard.sh docs/CEF_TRACK_REPORT.json docs/CEF_TRACK_REPORT.md

cef-track-sync-guard:
    bash scripts/cef_track_sync_guard.sh docs/CEF_TRACK_REPORT.json docs/CEF_TRACK_REPORT.md

cef-track-sync-guard-selfcheck:
    test -x scripts/cef_track_sync_guard.sh
    bash -n scripts/cef_track_sync_guard.sh
    rg -q 'mktemp -d' scripts/cef_track_sync_guard.sh
    rg -q 'cef_track_report.sh' scripts/cef_track_sync_guard.sh
    rg -q 'cmp -s' scripts/cef_track_sync_guard.sh

cef-track-guard-selfcheck:
    test -x scripts/cef_track_guard.sh
    bash -n scripts/cef_track_guard.sh
    rg -q 'cargo run -q -p chimera-lab --bin cef_track_guard --' scripts/cef_track_guard.sh
    test -f crates/chimera-lab/src/bin/cef_track_guard.rs
    rg -q 'cef track guard: PASS' crates/chimera-lab/src/bin/cef_track_guard.rs
    rg -q 'truth boundary mismatch' crates/chimera-lab/src/bin/cef_track_guard.rs
    rg -q 'gate_status mismatch' crates/chimera-lab/src/bin/cef_track_guard.rs

cef-gap-map-guard:
    bash scripts/cef_gap_map_guard.sh docs/CEF_GAP_MAP_2026-05-18.md

cef-consistency-guard:
    bash scripts/cef_consistency_guard.sh docs/CEF_TRACK_REPORT.json docs/CEF_TRACK_REPORT.md docs/CEF_GAP_MAP_2026-05-18.md

cef-consistency-guard-selfcheck:
    test -x scripts/cef_consistency_guard.sh
    bash -n scripts/cef_consistency_guard.sh
    rg -q 'cargo run -q -p chimera-lab --bin cef_consistency_guard --' scripts/cef_consistency_guard.sh
    test -f crates/chimera-lab/src/bin/cef_consistency_guard.rs
    rg -q 'cef consistency guard: PASS' crates/chimera-lab/src/bin/cef_consistency_guard.rs
    rg -q 'phase1_closed mismatch' crates/chimera-lab/src/bin/cef_consistency_guard.rs
    rg -q 'gap status count mismatch' crates/chimera-lab/src/bin/cef_consistency_guard.rs

cef-gap-map-guard-selfcheck:
    test -x scripts/cef_gap_map_guard.sh
    bash -n scripts/cef_gap_map_guard.sh
    rg -q 'Gap Matrix' scripts/cef_gap_map_guard.sh
    rg -q 'for n in 1 2 3 4 5 6 7' scripts/cef_gap_map_guard.sh
    rg -q 'Full cooperative mesh runtime' scripts/cef_gap_map_guard.sh
    rg -q 'DHT discovery' scripts/cef_gap_map_guard.sh
    rg -q 'Distributed Policy Store' scripts/cef_gap_map_guard.sh
    rg -q 'Cooperative relay participation/consent model' scripts/cef_gap_map_guard.sh
    rg -q 'Emergency/OOB carriers' scripts/cef_gap_map_guard.sh
    rg -q 'Roaming cache / distributed bootstrap continuation' scripts/cef_gap_map_guard.sh
    rg -q 'Reputation / complaint / relay credit subsystems' scripts/cef_gap_map_guard.sh
    rg -q 'rg -c '\''\^1\\\. Full cooperative mesh runtime\$'\''' scripts/cef_gap_map_guard.sh
    rg -q 'rg -c '\''\^2\\\. DHT discovery' scripts/cef_gap_map_guard.sh
    rg -q ' -eq 1' scripts/cef_gap_map_guard.sh
    rg -q 'count_current_fact=' scripts/cef_gap_map_guard.sh
    rg -q 'count_status=' scripts/cef_gap_map_guard.sh
    rg -q 'count_next_step=' scripts/cef_gap_map_guard.sh
    rg -q 'count_pdf_evidence=' scripts/cef_gap_map_guard.sh
    rg -q 'number_set=' scripts/cef_gap_map_guard.sh
    rg -q 'for n in 1 2 3 4 5 6 7; do' scripts/cef_gap_map_guard.sh
    rg -q 'start_line=' scripts/cef_gap_map_guard.sh
    rg -q 'end_line=' scripts/cef_gap_map_guard.sh
    rg -Fq 'sed -n "${start_line},${end_line}p" "$GAP_MAP_PATH" | rg -q '\''^- `CHIMERA.pdf` evidence:'\''' scripts/cef_gap_map_guard.sh
    rg -Fq 'sed -n "${start_line},${end_line}p" "$GAP_MAP_PATH" | rg -q '\''^- Current fact:'\''' scripts/cef_gap_map_guard.sh
    rg -Fq 'sed -n "${start_line},${end_line}p" "$GAP_MAP_PATH" | rg -q '\''^- Status:'\''' scripts/cef_gap_map_guard.sh
    rg -Fq 'sed -n "${start_line},${end_line}p" "$GAP_MAP_PATH" | rg -q '\''^- Next step:'\''' scripts/cef_gap_map_guard.sh
    rg -q 'evidence_line=' scripts/cef_gap_map_guard.sh
    rg -q 'current_line=' scripts/cef_gap_map_guard.sh
    rg -q 'status_line=' scripts/cef_gap_map_guard.sh
    rg -q 'next_step_line=' scripts/cef_gap_map_guard.sh
    rg -q 'test "\$evidence_line" -lt "\$current_line"' scripts/cef_gap_map_guard.sh
    rg -q 'test "\$current_line" -lt "\$status_line"' scripts/cef_gap_map_guard.sh
    rg -q 'test "\$status_line" -lt "\$next_step_line"' scripts/cef_gap_map_guard.sh
    rg -q 'test "\$number_set" = "1 2 3 4 5 6 7"' scripts/cef_gap_map_guard.sh
    rg -q 'test "\$count_current_fact" -ge 7' scripts/cef_gap_map_guard.sh
    rg -q 'test "\$count_status" -ge 7' scripts/cef_gap_map_guard.sh
    rg -q 'test "\$count_next_step" -ge 7' scripts/cef_gap_map_guard.sh
    rg -q 'test "\$count_pdf_evidence" -ge 7' scripts/cef_gap_map_guard.sh
    rg -q 'PARTIAL / NOT CLOSED' scripts/cef_gap_map_guard.sh

cef-phase1-smoke:
    cargo run -p chimera-lab --bin chimera-lab -- cef-phase1-smoke --json --out docs/CEF_PHASE1_SMOKE.json

mesh-runtime-trace-guard:
    cargo run -q -p chimera-lab --bin mesh_runtime_trace_guard -- docs/MESH_RUNTIME_TRACE.json

mesh-runtime-trace-guard-selfcheck:
    test -f crates/chimera-lab/src/bin/mesh_runtime_trace_guard.rs
    rg -q 'mesh_runtime_trace' crates/chimera-lab/src/bin/mesh_runtime_trace_guard.rs
    rg -q 'persisted_state_reselection' crates/chimera-lab/src/bin/mesh_runtime_trace_guard.rs
    rg -q 'mesh runtime trace guard: PASS' crates/chimera-lab/src/bin/mesh_runtime_trace_guard.rs

mesh-route-explain-smoke:
    cargo run -q -p chimera-cli -- mesh route-explain --namespace cef-public --node node-client --invite-token inv-1 --policy-payload 'allow=mesh;mesh_allowed_regions=eu;mesh_min_reliability=70;mesh_max_load=60;mesh_max_peers=1' --peer 'node-eu-1@198.51.100.7:443@eu@20@92' --peer 'node-eu-2@198.51.100.8:443@eu@40@88' --failed-node node-eu-1 --cooldown-node node-eu-2 --json --out docs/MESH_ROUTE_EXPLAIN.json

mesh-route-explain-error-smoke:
    set +e; cargo run -q -p chimera-cli -- mesh route-explain --namespace cef-public --node node-client --invite-token inv-1 --policy-payload 'mesh_max_peers=0' --peer 'node-eu-1@198.51.100.7:443@eu@20@92' --json --out docs/MESH_ROUTE_EXPLAIN_ERROR.json; rc=$?; set -e; test "$rc" -eq 2

mesh-launch-preflight-verify-smoke:
    bash scripts/mesh_launch_preflight_verify_smoke.sh

mesh-launch-preflight-verify-smoke-selfcheck:
    test -x scripts/mesh_launch_preflight_verify_smoke.sh
    bash -n scripts/mesh_launch_preflight_verify_smoke.sh
    rg -q 'mesh launch-preflight-verify' scripts/mesh_launch_preflight_verify_smoke.sh
    rg -q '"status":"ready"' scripts/mesh_launch_preflight_verify_smoke.sh
    rg -q '"all_ready":true' scripts/mesh_launch_preflight_verify_smoke.sh
    rg -q 'mesh launch preflight verify smoke: PASS' scripts/mesh_launch_preflight_verify_smoke.sh

mesh-launch-preflight-verify-guard:
    cargo run -q -p chimera-lab --bin mesh_launch_preflight_verify_guard -- docs/MESH_LAUNCH_PREFLIGHT_VERIFY_SMOKE.json

mesh-launch-preflight-report-guard-side-a:
    cargo run -q -p chimera-lab --bin mesh_launch_preflight_report_guard -- "${CHIMERA_MESH_PREFLIGHT_VPS_JSON:-docs/MESH_LAUNCH_PREFLIGHT_VPS.json}" side_a

mesh-launch-preflight-report-guard-side-b:
    cargo run -q -p chimera-lab --bin mesh_launch_preflight_report_guard -- "${CHIMERA_MESH_PREFLIGHT_LAPTOP_JSON:-docs/MESH_LAUNCH_PREFLIGHT_LAPTOP.json}" side_b

mesh-launch-preflight-report-guard-selfcheck:
    test -f crates/chimera-lab/src/bin/mesh_launch_preflight_report_guard.rs
    rg -q 'status must be ready|blocked' crates/chimera-lab/src/bin/mesh_launch_preflight_report_guard.rs
    rg -q 'network_state must be not_modified' crates/chimera-lab/src/bin/mesh_launch_preflight_report_guard.rs
    rg -q 'ready_for_real_launch must match connect_probe_success' crates/chimera-lab/src/bin/mesh_launch_preflight_report_guard.rs
    rg -q 'expected role must be side_a|side_b' crates/chimera-lab/src/bin/mesh_launch_preflight_report_guard.rs
    rg -q 'mesh launch preflight report guard: PASS' crates/chimera-lab/src/bin/mesh_launch_preflight_report_guard.rs
    cargo test -q -p chimera-lab --bin mesh_launch_preflight_report_guard

mesh-launch-preflight-verify-guard-selfcheck:
    test -f crates/chimera-lab/src/bin/mesh_launch_preflight_verify_guard.rs
    rg -q 'status must be ready|blocked' crates/chimera-lab/src/bin/mesh_launch_preflight_verify_guard.rs
    rg -q 'all_ready=true requires empty blockers' crates/chimera-lab/src/bin/mesh_launch_preflight_verify_guard.rs
    rg -q 'blocked status requires blockers' crates/chimera-lab/src/bin/mesh_launch_preflight_verify_guard.rs
    rg -q 'mesh launch preflight verify guard: PASS' crates/chimera-lab/src/bin/mesh_launch_preflight_verify_guard.rs
    cargo test -q -p chimera-lab --bin mesh_launch_preflight_verify_guard

mesh-launch-preflight-pair:
    bash scripts/mesh_launch_preflight_pair.sh

mesh-launch-preflight-pair-selfcheck:
    test -x scripts/mesh_launch_preflight_pair.sh
    bash -n scripts/mesh_launch_preflight_pair.sh
    rg -q 'CHIMERA_MESH_LOCAL_ROLE' scripts/mesh_launch_preflight_pair.sh
    rg -q 'CHIMERA_MESH_ALLOW_REMOTE_MISSING' scripts/mesh_launch_preflight_pair.sh
    rg -q 'CHIMERA_MESH_NAMESPACE' scripts/mesh_launch_preflight_pair.sh
    rg -q 'CHIMERA_MESH_TRAFFIC_PROFILE' scripts/mesh_launch_preflight_pair.sh
    rg -q 'mesh launch-preflight' scripts/mesh_launch_preflight_pair.sh
    rg -q 'mesh launch-preflight-verify' scripts/mesh_launch_preflight_pair.sh
    rg -q 'VERIFY_VPS_REPORT' scripts/mesh_launch_preflight_pair.sh
    rg -q 'VERIFY_LAPTOP_REPORT' scripts/mesh_launch_preflight_pair.sh
    rg -q 'trim_ascii\(\)' scripts/mesh_launch_preflight_pair.sh
    rg -q 'validate_peer_spec\(\)' scripts/mesh_launch_preflight_pair.sh
    rg -q 'declare -A seen_peers=' scripts/mesh_launch_preflight_pair.sh
    rg -q 'invalid peer spec in' scripts/mesh_launch_preflight_pair.sh
    rg -q 'expected format: node@endpoint:port#region@load@reliability' scripts/mesh_launch_preflight_pair.sh
    rg -q 'verify skipped by CHIMERA_MESH_ALLOW_REMOTE_MISSING=1' scripts/mesh_launch_preflight_pair.sh
    rg -q 'missing remote artifact' scripts/mesh_launch_preflight_pair.sh

mesh-launch-preflight-env-guard-side-a:
    test -x scripts/mesh_launch_preflight_env_guard.sh
    test -f configs/mesh_launch_preflight.vps.env
    bash scripts/mesh_launch_preflight_env_guard.sh configs/mesh_launch_preflight.vps.env

mesh-launch-preflight-env-guard-side-b:
    test -x scripts/mesh_launch_preflight_env_guard.sh
    test -f configs/mesh_launch_preflight.laptop.env
    bash scripts/mesh_launch_preflight_env_guard.sh configs/mesh_launch_preflight.laptop.env

mesh-launch-preflight-endpoint-probe-side-a:
    test -x scripts/mesh_launch_preflight_endpoint_probe.sh
    test -f configs/mesh_launch_preflight.vps.env
    bash scripts/mesh_launch_preflight_endpoint_probe.sh configs/mesh_launch_preflight.vps.env

mesh-launch-preflight-endpoint-probe-side-b:
    test -x scripts/mesh_launch_preflight_endpoint_probe.sh
    test -f configs/mesh_launch_preflight.laptop.env
    bash scripts/mesh_launch_preflight_endpoint_probe.sh configs/mesh_launch_preflight.laptop.env

mesh-launch-preflight-ready-check:
    just mesh-launch-preflight-env-guard-side-a
    just mesh-launch-preflight-env-guard-side-b
    just mesh-launch-preflight-env-pair-guard
    just mesh-launch-preflight-endpoint-probe-side-a
    just mesh-launch-preflight-endpoint-probe-side-b

mesh-launch-preflight-ready-hint:
    test -x scripts/mesh_launch_preflight_ready_hint.sh
    bash scripts/mesh_launch_preflight_ready_hint.sh

mesh-launch-preflight-ready-hint-selfcheck:
    test -x scripts/mesh_launch_preflight_ready_hint.sh
    bash -n scripts/mesh_launch_preflight_ready_hint.sh
    rg -q 'mesh launch preflight ready hint: READY' scripts/mesh_launch_preflight_ready_hint.sh
    rg -q 'mesh launch preflight ready hint: NOT READY' scripts/mesh_launch_preflight_ready_hint.sh
    rg -q 'endpoint_probe_side_a' scripts/mesh_launch_preflight_ready_hint.sh

mesh-launch-preflight-status-summary:
    test -x scripts/mesh_launch_preflight_status_summary.sh
    bash scripts/mesh_launch_preflight_status_summary.sh

mesh-launch-preflight-status-summary-selfcheck:
    test -x scripts/mesh_launch_preflight_status_summary.sh
    bash -n scripts/mesh_launch_preflight_status_summary.sh
    rg -q 'status summary' scripts/mesh_launch_preflight_status_summary.sh
    rg -q 'side_a remote endpoint' scripts/mesh_launch_preflight_status_summary.sh
    rg -q 'verify artifact' scripts/mesh_launch_preflight_status_summary.sh
    rg -q 'readiness gate' scripts/mesh_launch_preflight_status_summary.sh

mesh-launch-preflight-print-unblock-cmd:
    test -x scripts/mesh_launch_preflight_print_unblock_cmd.sh
    bash scripts/mesh_launch_preflight_print_unblock_cmd.sh

mesh-launch-preflight-print-unblock-cmd-selfcheck:
    test -x scripts/mesh_launch_preflight_print_unblock_cmd.sh
    bash -n scripts/mesh_launch_preflight_print_unblock_cmd.sh
    rg -q 'mesh-launch-preflight-unblock-and-run' scripts/mesh_launch_preflight_print_unblock_cmd.sh
    rg -q 'CHIMERA_MESH_LOCAL_ENDPOINT' scripts/mesh_launch_preflight_print_unblock_cmd.sh
    rg -q 'documentation placeholder' scripts/mesh_launch_preflight_print_unblock_cmd.sh

mesh-launch-preflight-set-remote-endpoint side endpoint:
    test -x scripts/mesh_launch_preflight_set_remote_endpoint.sh
    bash scripts/mesh_launch_preflight_set_remote_endpoint.sh "{{side}}" "{{endpoint}}"

mesh-launch-preflight-set-remote-endpoint-selfcheck:
    test -x scripts/mesh_launch_preflight_set_remote_endpoint.sh
    bash -n scripts/mesh_launch_preflight_set_remote_endpoint.sh
    rg -q 'usage: mesh_launch_preflight_set_remote_endpoint.sh <side_a|side_b> <host:port>' scripts/mesh_launch_preflight_set_remote_endpoint.sh
    rg -q 'side must be side_a or side_b' scripts/mesh_launch_preflight_set_remote_endpoint.sh
    rg -q 'endpoint must be host:port' scripts/mesh_launch_preflight_set_remote_endpoint.sh
    rg -q 'port out of range' scripts/mesh_launch_preflight_set_remote_endpoint.sh

mesh-launch-preflight-set-local-endpoint side endpoint:
    test -x scripts/mesh_launch_preflight_set_local_endpoint.sh
    bash scripts/mesh_launch_preflight_set_local_endpoint.sh "{{side}}" "{{endpoint}}"

mesh-launch-preflight-set-local-endpoint-selfcheck:
    test -x scripts/mesh_launch_preflight_set_local_endpoint.sh
    bash -n scripts/mesh_launch_preflight_set_local_endpoint.sh
    rg -q 'usage: mesh_launch_preflight_set_local_endpoint.sh <side_a|side_b> <host:port>' scripts/mesh_launch_preflight_set_local_endpoint.sh
    rg -q 'side must be side_a or side_b' scripts/mesh_launch_preflight_set_local_endpoint.sh
    rg -q 'endpoint must be host:port' scripts/mesh_launch_preflight_set_local_endpoint.sh
    rg -q 'port out of range' scripts/mesh_launch_preflight_set_local_endpoint.sh

mesh-launch-preflight-set-real-endpoints laptop_endpoint vps_endpoint:
    just mesh-launch-preflight-set-remote-endpoint side_a "{{laptop_endpoint}}"
    just mesh-launch-preflight-set-local-endpoint side_b "{{laptop_endpoint}}"
    just mesh-launch-preflight-set-remote-endpoint side_b "{{vps_endpoint}}"
    just mesh-launch-preflight-set-local-endpoint side_a "{{vps_endpoint}}"
    just mesh-launch-preflight-ready-check

mesh-launch-preflight-set-real-endpoints-selfcheck:
    rg -q '^mesh-launch-preflight-set-real-endpoints laptop_endpoint vps_endpoint:$' justfile
    rg -q '^    just mesh-launch-preflight-set-remote-endpoint side_a ' justfile
    rg -q '^    just mesh-launch-preflight-set-local-endpoint side_b ' justfile
    rg -q '^    just mesh-launch-preflight-set-remote-endpoint side_b ' justfile
    rg -q '^    just mesh-launch-preflight-set-local-endpoint side_a ' justfile
    rg -q '^    just mesh-launch-preflight-ready-check$' justfile

mesh-launch-preflight-auto-bind vps_endpoint:
    test -x scripts/mesh_launch_preflight_auto_bind.sh
    bash scripts/mesh_launch_preflight_auto_bind.sh "{{vps_endpoint}}"

mesh-launch-preflight-auto-bind-selfcheck:
    test -x scripts/mesh_launch_preflight_auto_bind.sh
    bash -n scripts/mesh_launch_preflight_auto_bind.sh
    rg -q 'selected laptop endpoint' scripts/mesh_launch_preflight_auto_bind.sh
    rg -q 'mesh-launch-preflight-set-real-endpoints' scripts/mesh_launch_preflight_auto_bind.sh

mesh-launch-preflight-autopilot mode="staged" profile_set="core" vps_endpoint="":
    test -x scripts/mesh_launch_preflight_autopilot.sh
    bash scripts/mesh_launch_preflight_autopilot.sh "{{mode}}" "{{profile_set}}" "{{vps_endpoint}}"

mesh-launch-preflight-autopilot-selfcheck:
    test -x scripts/mesh_launch_preflight_autopilot.sh
    bash -n scripts/mesh_launch_preflight_autopilot.sh
    rg -q 'mode must be staged or full' scripts/mesh_launch_preflight_autopilot.sh
    rg -q 'profile set must be core or all' scripts/mesh_launch_preflight_autopilot.sh
    rg -q 'mesh-launch-preflight-auto-bind' scripts/mesh_launch_preflight_autopilot.sh
    rg -q 'mesh-launch-preflight-ready-check' scripts/mesh_launch_preflight_autopilot.sh
    rg -q 'mesh-launch-preflight-side-a-profile-staged' scripts/mesh_launch_preflight_autopilot.sh
    rg -q 'mesh-launch-preflight-side-b-profile-staged' scripts/mesh_launch_preflight_autopilot.sh
    rg -q 'mesh-launch-preflight-side-a-profile' scripts/mesh_launch_preflight_autopilot.sh
    rg -q 'mesh-launch-preflight-side-b-profile' scripts/mesh_launch_preflight_autopilot.sh
    rg -q 'mesh-launch-preflight-evidence-guard' scripts/mesh_launch_preflight_autopilot.sh
    rg -q 'mesh launch preflight autopilot: PASS' scripts/mesh_launch_preflight_autopilot.sh

mesh-launch-preflight-unblock-and-run laptop_endpoint:
    just mesh-launch-preflight-set-remote-endpoint side_a "{{laptop_endpoint}}"
    just mesh-launch-preflight-ready-check
    just mesh-launch-preflight-side-a
    just mesh-launch-preflight-side-b
    just mesh-launch-preflight-evidence-guard

mesh-launch-preflight-unblock-and-run-selfcheck:
    rg -q '^mesh-launch-preflight-unblock-and-run laptop_endpoint:$' justfile
    rg -q '^    just mesh-launch-preflight-set-remote-endpoint side_a ' justfile
    rg -q '^    just mesh-launch-preflight-ready-check$' justfile
    rg -q '^    just mesh-launch-preflight-side-a$' justfile
    rg -q '^    just mesh-launch-preflight-side-b$' justfile
    rg -q '^    just mesh-launch-preflight-evidence-guard$' justfile

mesh-launch-preflight-env-pair-guard:
    test -x scripts/mesh_launch_preflight_env_pair_guard.sh
    vps_env=configs/mesh_launch_preflight.vps.env; \
    laptop_env=configs/mesh_launch_preflight.laptop.env; \
    if [[ ! -f "$vps_env" ]]; then vps_env=configs/mesh_launch_preflight.vps.env.example; fi; \
    if [[ ! -f "$laptop_env" ]]; then laptop_env=configs/mesh_launch_preflight.laptop.env.example; fi; \
    bash scripts/mesh_launch_preflight_env_pair_guard.sh "$vps_env" "$laptop_env"

mesh-launch-preflight-env-pair-guard-selfcheck:
    test -x scripts/mesh_launch_preflight_env_pair_guard.sh
    bash -n scripts/mesh_launch_preflight_env_pair_guard.sh
    rg -q 'usage: mesh_launch_preflight_env_pair_guard.sh <side_a_env_file> <side_b_env_file>' scripts/mesh_launch_preflight_env_pair_guard.sh
    rg -q 'side_a env role must be side_a' scripts/mesh_launch_preflight_env_pair_guard.sh
    rg -q 'side_b env role must be side_b' scripts/mesh_launch_preflight_env_pair_guard.sh
    rg -q 'namespace mismatch between env files' scripts/mesh_launch_preflight_env_pair_guard.sh
    rg -q 'mesh launch preflight env pair guard: PASS' scripts/mesh_launch_preflight_env_pair_guard.sh

mesh-launch-preflight-env-guard-selfcheck:
    test -x scripts/mesh_launch_preflight_env_guard.sh
    bash -n scripts/mesh_launch_preflight_env_guard.sh
    rg -q 'CHIMERA_MESH_LOCAL_ROLE' scripts/mesh_launch_preflight_env_guard.sh
    rg -q 'CHIMERA_MESH_ALLOW_REMOTE_MISSING' scripts/mesh_launch_preflight_env_guard.sh
    rg -q 'CHIMERA_MESH_NAMESPACE' scripts/mesh_launch_preflight_env_guard.sh
    rg -q 'CHIMERA_MESH_TRAFFIC_PROFILE' scripts/mesh_launch_preflight_env_guard.sh
    rg -q 'CHIMERA_MESH_LOCAL_NODE' scripts/mesh_launch_preflight_env_guard.sh
    rg -q 'CHIMERA_MESH_REMOTE_ENDPOINT' scripts/mesh_launch_preflight_env_guard.sh
    rg -q 'CHIMERA_MESH_LOCAL_ENDPOINT' scripts/mesh_launch_preflight_env_guard.sh
    rg -q 'must be 0 or 1' scripts/mesh_launch_preflight_env_guard.sh
    rg -q 'must be side_a or side_b' scripts/mesh_launch_preflight_env_guard.sh
    rg -q 'local and remote node must differ' scripts/mesh_launch_preflight_env_guard.sh
    rg -q 'remote endpoint must be host:port' scripts/mesh_launch_preflight_env_guard.sh
    rg -q 'local endpoint must be host:port' scripts/mesh_launch_preflight_env_guard.sh
    rg -q 'local and remote out filenames must differ' scripts/mesh_launch_preflight_env_guard.sh
    rg -q 'remote endpoint port out of range' scripts/mesh_launch_preflight_env_guard.sh
    rg -q 'local endpoint port out of range' scripts/mesh_launch_preflight_env_guard.sh
    rg -q 'remote endpoint uses test placeholder range' scripts/mesh_launch_preflight_env_guard.sh
    rg -q 'local endpoint uses test placeholder range' scripts/mesh_launch_preflight_env_guard.sh
    rg -q 'mesh launch preflight env guard: PASS' scripts/mesh_launch_preflight_env_guard.sh

mesh-launch-preflight-endpoint-probe-selfcheck:
    test -x scripts/mesh_launch_preflight_endpoint_probe.sh
    bash -n scripts/mesh_launch_preflight_endpoint_probe.sh
    rg -q 'CHIMERA_MESH_REMOTE_ENDPOINT' scripts/mesh_launch_preflight_endpoint_probe.sh
    rg -q 'CHIMERA_MESH_ALLOW_REMOTE_MISSING' scripts/mesh_launch_preflight_endpoint_probe.sh
    rg -q 'CHIMERA_MESH_ENDPOINT_PROBE_TIMEOUT_SEC' scripts/mesh_launch_preflight_endpoint_probe.sh
    rg -q 'endpoint must be host:port' scripts/mesh_launch_preflight_endpoint_probe.sh
    rg -q 'mesh launch preflight endpoint probe: PASS' scripts/mesh_launch_preflight_endpoint_probe.sh
    rg -q 'mesh launch preflight endpoint probe: FAIL' scripts/mesh_launch_preflight_endpoint_probe.sh

mesh-launch-preflight-side-a:
    just mesh-launch-preflight-env-guard-side-a
    just mesh-launch-preflight-endpoint-probe-side-a
    just mesh-launch-preflight-env-pair-guard
    set -a; source configs/mesh_launch_preflight.vps.env; set +a; bash scripts/mesh_launch_preflight_pair.sh

mesh-launch-preflight-side-b:
    just mesh-launch-preflight-env-guard-side-b
    just mesh-launch-preflight-endpoint-probe-side-b
    just mesh-launch-preflight-env-pair-guard
    set -a; source configs/mesh_launch_preflight.laptop.env; set +a; bash scripts/mesh_launch_preflight_pair.sh

mesh-launch-preflight-side-a-profile profile:
    just mesh-launch-preflight-env-guard-side-a
    just mesh-launch-preflight-endpoint-probe-side-a
    just mesh-launch-preflight-env-pair-guard
    set -a; source configs/mesh_launch_preflight.vps.env; CHIMERA_MESH_TRAFFIC_PROFILE="{{profile}}"; unset CHIMERA_MESH_POLICY_PAYLOAD; set +a; bash scripts/mesh_launch_preflight_pair.sh

mesh-launch-preflight-side-b-profile profile:
    just mesh-launch-preflight-env-guard-side-b
    just mesh-launch-preflight-endpoint-probe-side-b
    just mesh-launch-preflight-env-pair-guard
    set -a; source configs/mesh_launch_preflight.laptop.env; CHIMERA_MESH_TRAFFIC_PROFILE="{{profile}}"; unset CHIMERA_MESH_POLICY_PAYLOAD; set +a; bash scripts/mesh_launch_preflight_pair.sh

mesh-launch-preflight-side-a-profile-staged profile:
    just mesh-launch-preflight-env-guard-side-a
    just mesh-launch-preflight-env-pair-guard
    set -a; source configs/mesh_launch_preflight.vps.env; CHIMERA_MESH_ALLOW_REMOTE_MISSING=1; CHIMERA_MESH_TRAFFIC_PROFILE="{{profile}}"; unset CHIMERA_MESH_POLICY_PAYLOAD; set +a; bash scripts/mesh_launch_preflight_pair.sh

mesh-launch-preflight-side-b-profile-staged profile:
    just mesh-launch-preflight-env-guard-side-b
    just mesh-launch-preflight-env-pair-guard
    set -a; source configs/mesh_launch_preflight.laptop.env; CHIMERA_MESH_ALLOW_REMOTE_MISSING=1; CHIMERA_MESH_TRAFFIC_PROFILE="{{profile}}"; unset CHIMERA_MESH_POLICY_PAYLOAD; set +a; bash scripts/mesh_launch_preflight_pair.sh

mesh-launch-preflight-profile-staged-selfcheck:
    rg -q '^mesh-launch-preflight-side-a-profile-staged profile:' justfile
    rg -q '^mesh-launch-preflight-side-b-profile-staged profile:' justfile
    rg -q 'CHIMERA_MESH_ALLOW_REMOTE_MISSING=1' justfile
    rg -q 'CHIMERA_MESH_TRAFFIC_PROFILE=' justfile
    rg -q 'unset CHIMERA_MESH_POLICY_PAYLOAD' justfile
    rg -q '^    set -a; source configs/mesh_launch_preflight.vps.env; CHIMERA_MESH_ALLOW_REMOTE_MISSING=1; CHIMERA_MESH_TRAFFIC_PROFILE="\{\{profile\}\}"; unset CHIMERA_MESH_POLICY_PAYLOAD; set \+a; bash scripts/mesh_launch_preflight_pair.sh$' justfile
    rg -q '^    set -a; source configs/mesh_launch_preflight.laptop.env; CHIMERA_MESH_ALLOW_REMOTE_MISSING=1; CHIMERA_MESH_TRAFFIC_PROFILE="\{\{profile\}\}"; unset CHIMERA_MESH_POLICY_PAYLOAD; set \+a; bash scripts/mesh_launch_preflight_pair.sh$' justfile

mesh-launch-preflight-side-a-staged:
    just mesh-launch-preflight-env-guard-side-a
    just mesh-launch-preflight-endpoint-probe-side-a
    just mesh-launch-preflight-env-pair-guard
    set -a; source configs/mesh_launch_preflight.vps.env; CHIMERA_MESH_ALLOW_REMOTE_MISSING=1; set +a; bash scripts/mesh_launch_preflight_pair.sh

mesh-launch-preflight-side-b-staged:
    just mesh-launch-preflight-env-guard-side-b
    just mesh-launch-preflight-endpoint-probe-side-b
    just mesh-launch-preflight-env-pair-guard
    set -a; source configs/mesh_launch_preflight.laptop.env; CHIMERA_MESH_ALLOW_REMOTE_MISSING=1; set +a; bash scripts/mesh_launch_preflight_pair.sh

mesh-launch-preflight-freshness-guard:
    bash scripts/mesh_launch_preflight_freshness_guard.sh

mesh-launch-preflight-cross-artifact-guard:
    cargo run -q -p chimera-lab --bin mesh_launch_preflight_cross_artifact_guard -- "${CHIMERA_MESH_PREFLIGHT_VPS_JSON:-docs/MESH_LAUNCH_PREFLIGHT_VPS.json}" "${CHIMERA_MESH_PREFLIGHT_LAPTOP_JSON:-docs/MESH_LAUNCH_PREFLIGHT_LAPTOP.json}" "${CHIMERA_MESH_PREFLIGHT_VERIFY_JSON:-docs/MESH_LAUNCH_PREFLIGHT_VERIFY.json}"

mesh-launch-preflight-cross-artifact-guard-selfcheck:
    test -f crates/chimera-lab/src/bin/mesh_launch_preflight_cross_artifact_guard.rs
    rg -q 'verify namespace mismatch' crates/chimera-lab/src/bin/mesh_launch_preflight_cross_artifact_guard.rs
    rg -q 'verify vps_ready mismatch' crates/chimera-lab/src/bin/mesh_launch_preflight_cross_artifact_guard.rs
    rg -q 'mesh launch preflight cross artifact guard: PASS' crates/chimera-lab/src/bin/mesh_launch_preflight_cross_artifact_guard.rs
    cargo test -q -p chimera-lab --bin mesh_launch_preflight_cross_artifact_guard

mesh-launch-preflight-freshness-guard-selfcheck:
    test -x scripts/mesh_launch_preflight_freshness_guard.sh
    bash -n scripts/mesh_launch_preflight_freshness_guard.sh
    rg -q 'CHIMERA_MESH_PREFLIGHT_MAX_AGE_SEC' scripts/mesh_launch_preflight_freshness_guard.sh
    rg -q 'CHIMERA_MESH_PREFLIGHT_VPS_JSON' scripts/mesh_launch_preflight_freshness_guard.sh
    rg -q 'CHIMERA_MESH_PREFLIGHT_LAPTOP_JSON' scripts/mesh_launch_preflight_freshness_guard.sh
    rg -q 'CHIMERA_MESH_PREFLIGHT_VERIFY_JSON' scripts/mesh_launch_preflight_freshness_guard.sh
    rg -q 'stale artifact' scripts/mesh_launch_preflight_freshness_guard.sh
    rg -q 'verify artifact older than peer artifacts' scripts/mesh_launch_preflight_freshness_guard.sh
    rg -q 'mesh launch preflight freshness guard: PASS' scripts/mesh_launch_preflight_freshness_guard.sh

mesh-launch-preflight-evidence-smoke:
    bash scripts/mesh_launch_preflight_evidence_smoke.sh

mesh-launch-preflight-profile-smoke:
    bash scripts/mesh_launch_preflight_profile_smoke.sh

mesh-launch-preflight-profile-smoke-selfcheck:
    test -x scripts/mesh_launch_preflight_profile_smoke.sh
    bash -n scripts/mesh_launch_preflight_profile_smoke.sh
    rg -q 'high_speed_anonymous' scripts/mesh_launch_preflight_profile_smoke.sh
    rg -q 'privacy_first' scripts/mesh_launch_preflight_profile_smoke.sh
    rg -q 'speed_first' scripts/mesh_launch_preflight_profile_smoke.sh
    rg -q 'low_latency_private' scripts/mesh_launch_preflight_profile_smoke.sh
    rg -q 'CHIMERA_MESH_TRAFFIC_PROFILE' scripts/mesh_launch_preflight_profile_smoke.sh
    rg -q 'unset CHIMERA_MESH_POLICY_PAYLOAD' scripts/mesh_launch_preflight_profile_smoke.sh
    rg -q 'side_a_rc=0' scripts/mesh_launch_preflight_profile_smoke.sh
    rg -q 'side_b_rc=0' scripts/mesh_launch_preflight_profile_smoke.sh
    bash -c 'a=$(rg -n "side_a_rc=0" scripts/mesh_launch_preflight_profile_smoke.sh | head -n1 | cut -d: -f1); b=$(rg -n "side_b_rc=0" scripts/mesh_launch_preflight_profile_smoke.sh | head -n1 | cut -d: -f1); test -n "$a" && test -n "$b" && test "$a" -lt "$b"'
    rg -q 'mesh launch preflight profile smoke: PASS' scripts/mesh_launch_preflight_profile_smoke.sh

mesh-launch-preflight-doc-two-phase-selfcheck:
    rg -q '^Copy-paste staged two-phase block \(strict first-host sequencing\):$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^just mesh-launch-preflight-side-a-profile-staged high_speed_anonymous$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^just mesh-launch-preflight-side-b-profile-staged high_speed_anonymous$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^just mesh-launch-preflight-side-a-profile-staged privacy_first$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^just mesh-launch-preflight-side-b-profile-staged privacy_first$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^just mesh-launch-preflight-side-a-profile-staged speed_first$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^just mesh-launch-preflight-side-b-profile-staged speed_first$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^just mesh-launch-preflight-side-a-profile-staged low_latency_private$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^just mesh-launch-preflight-side-b-profile-staged low_latency_private$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md

mesh-launch-preflight-doc-quickcheck-selfcheck:
    rg -q '^Daily quick check \(without full gate\):$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^just mesh-launch-preflight-profile-two-phase-fastcheck-selfcheck$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^just mesh-launch-preflight-profile-two-phase-fastcheck$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^just mesh-launch-preflight-ready-check$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^just mesh-launch-preflight-ready-hint$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^just mesh-launch-preflight-status-summary$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^just mesh-launch-preflight-unblock-and-run <laptop_host:port>$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^just mesh-launch-preflight-set-real-endpoints <laptop_host:port> <vps_host:port>$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^just mesh-launch-preflight-set-remote-endpoint side_a <laptop_host:port>$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^just mesh-launch-preflight-side-a && just mesh-launch-preflight-side-b && just mesh-launch-preflight-evidence-guard$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^Full gate note:$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^- `just mesh-launch-gate-selfcheck` now includes this fast fail-fast block before heavier smoke runs\.$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md

mesh-launch-preflight-doc-contract-selfcheck:
    rg -q '^Route-explain operator contract quick check:$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^just mesh-route-explain-operator-contract-selfcheck$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^just mesh-route-explain-operator-contract-check$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md
    rg -q '^Use this before real launch attempts when routing/diagnostics contract code was touched\.$' docs/MESH_FIRST_LAUNCH_EXECUTION_GATE.md

mesh-launch-preflight-doc-fast-selfcheck:
    just mesh-launch-preflight-doc-two-phase-selfcheck
    just mesh-launch-preflight-doc-quickcheck-selfcheck
    just mesh-launch-preflight-doc-contract-selfcheck

mesh-launch-preflight-profile-two-phase-fastcheck:
    just mesh-launch-preflight-profile-smoke-selfcheck
    just mesh-launch-preflight-doc-two-phase-selfcheck
    just mesh-launch-preflight-profile-smoke

mesh-launch-preflight-profile-two-phase-fastcheck-selfcheck:
    rg -q '^mesh-launch-preflight-profile-two-phase-fastcheck:$' justfile
    rg -q '^    just mesh-launch-preflight-profile-smoke-selfcheck$' justfile
    rg -q '^    just mesh-launch-preflight-doc-two-phase-selfcheck$' justfile
    rg -q '^    just mesh-launch-preflight-profile-smoke$' justfile
    awk 'BEGIN{ok=0;state=0;n=0} /^mesh-launch-preflight-profile-two-phase-fastcheck:/{state=1;next} state==1 && /^[^[:space:]]/{state=0} state==1 && /^    just /{n++; line[n]=$0} END{if(n>=3 && line[1]=="    just mesh-launch-preflight-profile-smoke-selfcheck" && line[2]=="    just mesh-launch-preflight-doc-two-phase-selfcheck" && line[3]=="    just mesh-launch-preflight-profile-smoke"){ok=1} exit(ok?0:1)}' justfile

mesh-launch-preflight-evidence-smoke-selfcheck:
    test -x scripts/mesh_launch_preflight_evidence_smoke.sh
    bash -n scripts/mesh_launch_preflight_evidence_smoke.sh
    rg -q 'mesh launch-preflight-verify' scripts/mesh_launch_preflight_evidence_smoke.sh
    rg -q 'trap cleanup EXIT' scripts/mesh_launch_preflight_evidence_smoke.sh
    rg -q 'mesh launch preflight evidence smoke: temp artifact cleanup failed' scripts/mesh_launch_preflight_evidence_smoke.sh
    rg -q 'CHIMERA_MESH_PREFLIGHT_MAX_AGE_SEC=300' scripts/mesh_launch_preflight_evidence_smoke.sh
    rg -q 'CHIMERA_MESH_PREFLIGHT_VPS_JSON=' scripts/mesh_launch_preflight_evidence_smoke.sh
    rg -q 'CHIMERA_MESH_PREFLIGHT_LAPTOP_JSON=' scripts/mesh_launch_preflight_evidence_smoke.sh
    rg -q 'CHIMERA_MESH_PREFLIGHT_VERIFY_JSON=' scripts/mesh_launch_preflight_evidence_smoke.sh
    rg -q 'just mesh-launch-preflight-evidence-guard' scripts/mesh_launch_preflight_evidence_smoke.sh
    rg -q 'mesh launch preflight evidence smoke: PASS' scripts/mesh_launch_preflight_evidence_smoke.sh

mesh-launch-preflight-evidence-smoke-docs-preserve-selfcheck:
    tmp_before=$(mktemp); tmp_after=$(mktemp); \
    for p in docs/MESH_LAUNCH_PREFLIGHT_VPS.json docs/MESH_LAUNCH_PREFLIGHT_LAPTOP.json docs/MESH_LAUNCH_PREFLIGHT_VERIFY.json; do \
      if [[ -f "$$p" ]]; then \
        printf '%s %s\n' "$$p" "$(sha256sum "$$p" | awk '{print $1}')" >> "$$tmp_before"; \
      else \
        printf '%s %s\n' "$$p" "__MISSING__" >> "$$tmp_before"; \
      fi; \
    done; \
    just mesh-launch-preflight-evidence-smoke; \
    for p in docs/MESH_LAUNCH_PREFLIGHT_VPS.json docs/MESH_LAUNCH_PREFLIGHT_LAPTOP.json docs/MESH_LAUNCH_PREFLIGHT_VERIFY.json; do \
      if [[ -f "$$p" ]]; then \
        printf '%s %s\n' "$$p" "$(sha256sum "$$p" | awk '{print $1}')" >> "$$tmp_after"; \
      else \
        printf '%s %s\n' "$$p" "__MISSING__" >> "$$tmp_after"; \
      fi; \
    done; \
    diff -u "$$tmp_before" "$$tmp_after"; \
    rm -f "$$tmp_before" "$$tmp_after"

mesh-launch-preflight-evidence-guard:
    just mesh-launch-preflight-freshness-guard
    just mesh-launch-preflight-report-guard-side-a
    just mesh-launch-preflight-report-guard-side-b
    cargo run -q -p chimera-lab --bin mesh_launch_preflight_verify_guard -- "${CHIMERA_MESH_PREFLIGHT_VERIFY_JSON:-docs/MESH_LAUNCH_PREFLIGHT_VERIFY.json}"
    just mesh-launch-preflight-cross-artifact-guard

mesh-launch-gate-selfcheck:
    just mesh-launch-preflight-env-guard-selfcheck
    just mesh-launch-preflight-ready-hint-selfcheck
    just mesh-launch-preflight-status-summary-selfcheck
    just mesh-launch-preflight-print-unblock-cmd-selfcheck
    just mesh-launch-preflight-set-local-endpoint-selfcheck
    just mesh-launch-preflight-set-real-endpoints-selfcheck
    just mesh-launch-preflight-auto-bind-selfcheck
    just mesh-launch-preflight-unblock-and-run-selfcheck
    just mesh-launch-preflight-env-pair-guard-selfcheck
    just mesh-launch-preflight-env-pair-guard
    just mesh-launch-preflight-pair-selfcheck
    just mesh-launch-preflight-profile-staged-selfcheck
    just mesh-launch-preflight-profile-two-phase-fastcheck-selfcheck
    just mesh-launch-preflight-report-guard-selfcheck
    just mesh-launch-preflight-freshness-guard-selfcheck
    just mesh-launch-preflight-cross-artifact-guard-selfcheck
    just mesh-launch-preflight-evidence-smoke-selfcheck
    just mesh-launch-preflight-profile-smoke-selfcheck
    just mesh-launch-preflight-doc-fast-selfcheck
    just mesh-launch-preflight-profile-smoke
    just mesh-launch-preflight-evidence-smoke-docs-preserve-selfcheck
    just mesh-launch-preflight-verify-smoke-selfcheck
    just mesh-launch-preflight-verify-smoke
    just mesh-launch-preflight-verify-guard-selfcheck
    just mesh-launch-preflight-verify-guard

mesh-auto-smoke:
    cargo run -q -p chimera-lab --bin chimera-lab -- mesh-auto-smoke --json --out docs/MESH_AUTO_ADAPTIVE_TRACE.json

mesh-route-explain-guard:
    cargo run -q -p chimera-lab --bin mesh_route_explain_guard -- docs/MESH_ROUTE_EXPLAIN.json

mesh-route-explain-guard-selfcheck:
    test -f crates/chimera-lab/src/bin/mesh_route_explain_guard.rs
    rg -q 'mesh_route_explain' crates/chimera-lab/src/bin/mesh_route_explain_guard.rs
    rg -q 'mesh route explain guard: PASS' crates/chimera-lab/src/bin/mesh_route_explain_guard.rs

mesh-route-explain-error-guard:
    cargo run -q -p chimera-lab --bin mesh_route_explain_error_guard -- docs/MESH_ROUTE_EXPLAIN_ERROR.json

mesh-route-explain-error-guard-selfcheck:
    test -f crates/chimera-lab/src/bin/mesh_route_explain_error_guard.rs
    rg -q 'mesh_route_explain_error' crates/chimera-lab/src/bin/mesh_route_explain_error_guard.rs
    rg -q 'mesh route explain error guard: PASS' crates/chimera-lab/src/bin/mesh_route_explain_error_guard.rs
    rg -q 'auto_recovery_final_result' crates/chimera-lab/src/bin/mesh_route_explain_error_guard.rs
    rg -q 'connect_retry_budget_exhausted' crates/chimera-lab/src/bin/mesh_route_explain_error_guard.rs
    rg -q 'validate_accepts_valid_error_contract' crates/chimera-lab/src/bin/mesh_route_explain_error_guard.rs
    rg -q 'validate_rejects_mismatched_projection_key' crates/chimera-lab/src/bin/mesh_route_explain_error_guard.rs
    rg -q 'validate_rejects_recovery_schema_mismatch' crates/chimera-lab/src/bin/mesh_route_explain_error_guard.rs
    rg -q 'validate_rejects_blank_error_stage' crates/chimera-lab/src/bin/mesh_route_explain_error_guard.rs
    cargo test -q -p chimera-lab --bin mesh_route_explain_error_guard

mesh-route-explain-operator-contract-check:
    cargo test -q -p chimera-cli tests_json_operator_cross_contract
    cargo test -q -p chimera-cli tests_json_operator_cross_contract_matrix
    cargo test -q -p chimera-cli tests_json_error_contract
    cargo test -q -p chimera-cli tests_route_explain_error_internal_matrix

mesh-route-explain-operator-contract-selfcheck:
    rg -q '^mesh-route-explain-operator-contract-check:' justfile
    rg -q 'cargo test -q -p chimera-cli tests_json_operator_cross_contract' justfile
    rg -q 'cargo test -q -p chimera-cli tests_json_operator_cross_contract_matrix' justfile
    rg -q 'cargo test -q -p chimera-cli tests_json_error_contract' justfile
    rg -q 'cargo test -q -p chimera-cli tests_route_explain_error_internal_matrix' justfile

mesh-launch-connect-contract-check:
    cargo test -q -p chimera-cli tests_connect_probe_json
    cargo test -q -p chimera-cli tests_launch_preflight_json

mesh-launch-connect-contract-selfcheck:
    rg -q '^mesh-launch-connect-contract-check:' justfile
    rg -q 'cargo test -q -p chimera-cli tests_connect_probe_json' justfile
    rg -q 'cargo test -q -p chimera-cli tests_launch_preflight_json' justfile

mesh-auto-adaptive-trace-guard:
    cargo run -q -p chimera-lab --bin mesh_auto_adaptive_trace_guard -- docs/MESH_AUTO_ADAPTIVE_TRACE.json

mesh-auto-adaptive-trace-guard-selfcheck:
    test -f crates/chimera-lab/src/bin/mesh_auto_adaptive_trace_guard.rs
    rg -q 'mesh_auto_adaptive_trace' crates/chimera-lab/src/bin/mesh_auto_adaptive_trace_guard.rs
    rg -q 'auto:fast_signals' crates/chimera-lab/src/bin/mesh_auto_adaptive_trace_guard.rs
    rg -q 'auto:degraded_active' crates/chimera-lab/src/bin/mesh_auto_adaptive_trace_guard.rs
    rg -q 'mesh auto adaptive trace guard: PASS' crates/chimera-lab/src/bin/mesh_auto_adaptive_trace_guard.rs

hardening-smoke:
    cargo run -p chimera-lab --bin chimera-lab -- hardening-smoke

benchmark-report:
    cargo run -p chimera-lab --bin chimera-lab -- benchmark-report --out docs/benchmark_latest.json

benchmark-regression-check:
    bash scripts/benchmark_regression_check.sh

benchmark-regression-selfcheck:
    test -x scripts/benchmark_regression_check.sh
    bash -n scripts/benchmark_regression_check.sh
    rg -q 'max_attempts=2' scripts/benchmark_regression_check.sh
    rg -q 'benchmark-report --baseline' scripts/benchmark_regression_check.sh
    rg -q 'BENCHMARK_REGRESSION_GATE.json' scripts/benchmark_regression_check.sh
    rg -q '"kind":"benchmark_regression_gate"' scripts/benchmark_regression_check.sh

baseline-verify:
    sha256sum -c docs/V1_MVP_BASELINE.sha256

baseline-freeze:
    bash scripts/baseline_freeze.sh

cleanroom-handoff-check:
    bash scripts/cleanroom_handoff_check.sh

json-message-contract-check:
    rg -q '"message_en":"' docs/doctor_latest.json
    rg -q '"message_ru":"' docs/doctor_latest.json
    rg -q '"message_en":"' docs/gateway_doctor_latest.json
    rg -q '"message_ru":"' docs/gateway_doctor_latest.json
    rg -q '"message_en":"' docs/lab_doctor_latest.json
    rg -q '"message_ru":"' docs/lab_doctor_latest.json
    rg -q '"message_en":"' docs/datapath_latest.json
    rg -q '"message_ru":"' docs/datapath_latest.json
    rg -q '"kind":"datapath_report"' docs/datapath_latest.json
    rg -q '"gateway_explain":"' docs/datapath_latest.json
    rg -q '"block_explain":"' docs/datapath_latest.json
    rg -q '"direct_explain":"' docs/datapath_latest.json
    rg -q '"message_en":"' docs/route_explain_latest.json
    rg -q '"message_ru":"' docs/route_explain_latest.json
    rg -q '"message_en":"' docs/rollback_status_latest.json
    rg -q '"message_ru":"' docs/rollback_status_latest.json
    rg -q '"message_en":"' docs/rollback_recover_latest.json
    rg -q '"message_ru":"' docs/rollback_recover_latest.json
    rg -q '"message_en":"' docs/rollback_status_after_recover_latest.json
    rg -q '"message_ru":"' docs/rollback_status_after_recover_latest.json
    rg -q '"message_en":"' docs/diag_export_latest.json
    rg -q '"message_ru":"' docs/diag_export_latest.json
    rg -q '"message_en":"' docs/RUNTIME_APPLY_DNS_SMOKE.json
    rg -q '"message_ru":"' docs/RUNTIME_APPLY_DNS_SMOKE.json
    rg -q '"message_en":"' docs/RUNTIME_APPLY_ROUTE_SMOKE.json
    rg -q '"message_ru":"' docs/RUNTIME_APPLY_ROUTE_SMOKE.json
    rg -q '"network_state":"(not_modified|modified)"' docs/doctor_latest.json
    rg -q '"network_state":"(not_modified|modified)"' docs/gateway_doctor_latest.json
    rg -q '"network_state":"(not_modified|modified)"' docs/lab_doctor_latest.json
    rg -q '"network_state":"(not_modified|modified)"' docs/datapath_latest.json
    rg -q '"network_state":"(not_modified|modified)"' docs/route_explain_latest.json
    rg -q '"network_state":"(not_modified|modified)"' docs/rollback_status_latest.json
    rg -q '"network_state":"(not_modified|modified)"' docs/rollback_recover_latest.json
    rg -q '"network_state":"(not_modified|modified)"' docs/rollback_status_after_recover_latest.json
    rg -q '"network_state":"(not_modified|modified)"' docs/diag_export_latest.json
    rg -q '"network_state":"modified"' docs/RUNTIME_APPLY_DNS_SMOKE.json
    rg -q '"rollback_ok":true' docs/RUNTIME_APPLY_DNS_SMOKE.json
    rg -q '"network_state":"modified"' docs/RUNTIME_APPLY_ROUTE_SMOKE.json
    rg -q '"apply_attempt_ok":true' docs/RUNTIME_APPLY_ROUTE_SMOKE.json
    rg -q '"policy_rule_ok":true' docs/RUNTIME_APPLY_ROUTE_SMOKE.json
    rg -q '"rollback_ok":true' docs/RUNTIME_APPLY_ROUTE_SMOKE.json
    rg -q '"status":"ok"' docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json
    rg -q '"kind":"runtime_apply_route_existing_tun_smoke"' docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json
    rg -q '"apply_attempt_ok":true' docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json
    rg -q '"preexisting_tun_used":true' docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json
    rg -q '"rollback_ok":true' docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json
    rg -q '"status":"ok"' docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json
    rg -q '"kind":"runtime_apply_route_multi_cidr_smoke"' docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json
    (rg -q '"apply_attempt_ok":true' docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json && rg -q '"policy_rule_ok":true' docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json) || rg -q '"skipped_no_tun":true' docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json
    rg -q '"rollback_ok":true' docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json
    rg -q '"status":"ok"' docs/RUNTIME_ROUTE_POLICY_VALIDATION_SMOKE.json
    rg -q '"kind":"runtime_route_policy_validation_smoke"' docs/RUNTIME_ROUTE_POLICY_VALIDATION_SMOKE.json
    rg -q '"network_state":"not_modified"' docs/RUNTIME_ROUTE_POLICY_VALIDATION_SMOKE.json
    rg -q '"apply_rejected":true' docs/RUNTIME_ROUTE_POLICY_VALIDATION_SMOKE.json
    rg -q '"state_not_created":true' docs/RUNTIME_ROUTE_POLICY_VALIDATION_SMOKE.json
    rg -q '"status":"ok"' docs/RUNTIME_ROUTE_DUPLICATE_CIDR_VALIDATION_SMOKE.json
    rg -q '"kind":"runtime_route_duplicate_cidr_validation_smoke"' docs/RUNTIME_ROUTE_DUPLICATE_CIDR_VALIDATION_SMOKE.json
    rg -q '"network_state":"not_modified"' docs/RUNTIME_ROUTE_DUPLICATE_CIDR_VALIDATION_SMOKE.json
    rg -q '"apply_rejected":true' docs/RUNTIME_ROUTE_DUPLICATE_CIDR_VALIDATION_SMOKE.json
    rg -q '"state_not_created":true' docs/RUNTIME_ROUTE_DUPLICATE_CIDR_VALIDATION_SMOKE.json
    rg -q '"status":"ok"' docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json
    rg -q '"kind":"runtime_tun_name_validation_smoke"' docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json
    rg -q '"network_state":"not_modified"' docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json
    rg -q '"apply_rejected":true' docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json
    rg -q '"state_not_created":true' docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json
    rg -q '"status":"ok"' docs/RUNTIME_RESOLV_CONF_VALIDATION_SMOKE.json
    rg -q '"kind":"runtime_resolv_conf_validation_smoke"' docs/RUNTIME_RESOLV_CONF_VALIDATION_SMOKE.json
    rg -q '"network_state":"not_modified"' docs/RUNTIME_RESOLV_CONF_VALIDATION_SMOKE.json
    rg -q '"apply_rejected":true' docs/RUNTIME_RESOLV_CONF_VALIDATION_SMOKE.json
    rg -q '"state_not_created":true' docs/RUNTIME_RESOLV_CONF_VALIDATION_SMOKE.json
    rg -q '"status":"ok"' docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json
    rg -q '"kind":"runtime_datapath_multiflow_smoke"' docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json
    rg -q '"network_state":"not_modified"' docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json
    rg -q '"gateway_ok":true' docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json
    rg -q '"block_ok":true' docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json
    rg -q '"direct_ok":true' docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json
    rg -q '"status":"ok"' docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json
    rg -q '"kind":"runtime_policy_precedence_smoke"' docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json
    rg -q '"network_state":"not_modified"' docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json
    rg -q '"precedence_ok":true' docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json
    rg -q '"all_matches_ok":true' docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json
    rg -q '"dns_binding_ok":true' docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json

rollback-json-contract-check:
    rg -q '"kind":"rollback"' docs/rollback_status_latest.json
    rg -q '"kind":"rollback"' docs/rollback_recover_latest.json
    rg -q '"kind":"rollback"' docs/rollback_status_after_recover_latest.json
    rg -q -P '"tun_applied":(true|false)' docs/rollback_status_latest.json
    rg -q -P '"route_applied":(true|false)' docs/rollback_status_latest.json
    rg -q -P '"dns_applied":(true|false)' docs/rollback_status_latest.json
    rg -q -P '"network_state":"(not_modified|modified)"' docs/rollback_status_latest.json
    rg -q -P '"tun_applied":(true|false)' docs/rollback_recover_latest.json
    rg -q -P '"route_applied":(true|false)' docs/rollback_recover_latest.json
    rg -q -P '"dns_applied":(true|false)' docs/rollback_recover_latest.json
    rg -q -P '"network_state":"(not_modified|modified)"' docs/rollback_recover_latest.json

truth-contract-check:
    just truth-contract-fast-precheck
    just json-no-dupe-guard-selfcheck
    just json-no-dupe-guard
    just cef-track-guard-selfcheck
    just cef-track-guard
    just cef-track-sync-guard-selfcheck
    just cef-track-sync-guard
    just mesh-runtime-trace-guard-selfcheck
    just mesh-runtime-trace-guard
    just mesh-route-explain-smoke
    just mesh-route-explain-error-smoke
    just mesh-route-explain-guard-selfcheck
    just mesh-route-explain-guard
    just mesh-route-explain-error-guard-selfcheck
    just mesh-route-explain-error-guard
    just mesh-route-explain-operator-contract-selfcheck
    just mesh-route-explain-operator-contract-check
    just cef-gap-map-guard-selfcheck
    just cef-gap-map-guard
    just cef-consistency-guard-selfcheck
    just cef-consistency-guard
    rg -qv 'Current implementation is M0/M1 skeleton only' docs/ARCHITECTURE.md
    rg -qv 'listener is not started' docs/MVP_SPEC_DEEP_AUDIT_2026-05-18.md
    rg -q 'Lab/proof/report contour: PASS.' docs/REALITY_AUDIT_2026-05-18.md
    rg -q 'Real OS-level datapath closure for strict M4/M5: PARTIAL / NOT CLOSED.' docs/REALITY_AUDIT_2026-05-18.md
    rg -q '"kind":"cef_track_report"' docs/CEF_TRACK_REPORT.json
    rg -q '"kind":"cef_phase1_smoke"' docs/CEF_PHASE1_SMOKE.json
    rg -q '"status":"ok"' docs/CEF_PHASE1_SMOKE.json
    rg -q '"network_state":"not_modified"' docs/CEF_PHASE1_SMOKE.json
    rg -q '"mvp_lab_ready":true' docs/CEF_TRACK_REPORT.json
    rg -q '"full_cef_closed":false' docs/CEF_TRACK_REPORT.json
    rg -q '"phase1_closed":true' docs/CEF_TRACK_REPORT.json
    rg -q '"mesh_runtime":\{"implemented":true,"code":true,"api":true,"tests":true,"runtime_wired":(true|false),"gate_status":"(ready|partial)"\}' docs/CEF_TRACK_REPORT.json
    rg -q '"dht_discovery":\{"implemented":true,"code":true,"api":true,"tests":true,"runtime_wired":(true|false),"gate_status":"(ready|partial)"\}' docs/CEF_TRACK_REPORT.json
    rg -q '"distributed_policy_store":\{"implemented":true,"code":true,"api":true,"tests":true,"runtime_wired":(true|false),"gate_status":"(ready|partial)"\}' docs/CEF_TRACK_REPORT.json
    rg -q '"cooperative_relay_model":\{"implemented":true,"code":true,"api":true,"tests":true,"runtime_wired":(true|false),"gate_status":"(ready|partial)"\}' docs/CEF_TRACK_REPORT.json
    rg -q '"emergency_oob_carriers":\{"implemented":true,"code":true,"api":true,"tests":true,"runtime_wired":(true|false),"gate_status":"(ready|partial)"\}' docs/CEF_TRACK_REPORT.json
    rg -q '"roaming_cache":\{"implemented":true,"code":true,"api":true,"tests":true,"runtime_wired":(true|false),"gate_status":"(ready|partial)"\}' docs/CEF_TRACK_REPORT.json
    rg -q '"reputation_complaint_credit":\{"implemented":true,"code":true,"api":true,"tests":true,"runtime_wired":(true|false),"gate_status":"(ready|partial)"\}' docs/CEF_TRACK_REPORT.json
    rg -q 'Status: \*\*PARTIAL / NOT CLOSED\*\*' docs/CEF_TRACK_REPORT.md
    rg -q 'Full CEF closed: `false`' docs/CEF_TRACK_REPORT.md
    rg -q 'runtime_wired: `(true|false)`, gate_status: `(ready|partial)`' docs/CEF_TRACK_REPORT.md
    test -f docs/CEF_PHASE1_GATES.md
    test -f docs/ADR/0002-full-cef-track-gates.md
    rg -q 'Gate Contract \(per block\)' docs/CEF_PHASE1_GATES.md
    rg -q 'Full CEF Track Gates' docs/ADR/0002-full-cef-track-gates.md
    rg -q 'just benchmark-regression-selfcheck' docs/SECOND_MACHINE_REPORT.md
    rg -q 'Benchmark regression gate script selfcheck: PASS' docs/SECOND_MACHINE_REPORT.md
    rg -q 'runtime forced-stop rollback smoke: true' docs/SECOND_MACHINE_REPORT.md
    rg -q 'runtime forced-stop recover\+cleanup: true' docs/SECOND_MACHINE_REPORT.md
    line_bench=$(grep -n '^1\. .*benchmark-regression-selfcheck' docs/SECOND_MACHINE_REPORT.md | cut -d: -f1); line_handoff=$(grep -n '^2\. .*handoff-check' docs/SECOND_MACHINE_REPORT.md | cut -d: -f1); test -n "$line_bench" && test -n "$line_handoff" && test "$line_bench" -lt "$line_handoff"

truth-contract-fast-precheck:
    just mesh-route-explain-operator-contract-selfcheck
    just mesh-route-explain-operator-contract-check
    just mesh-launch-connect-contract-selfcheck
    just mesh-launch-connect-contract-check
    just mesh-launch-preflight-doc-fast-selfcheck
    just mesh-launch-preflight-profile-two-phase-fastcheck-selfcheck

truth-contract-selfcheck:
    rg -q '^truth-contract-check:' justfile
    rg -q '^truth-contract-fast-precheck:' justfile
    rg -q 'just truth-contract-fast-precheck' justfile
    rg -q 'mesh-route-explain-error-smoke' justfile
    rg -q 'mesh-route-explain-error-guard-selfcheck' justfile
    rg -q 'mesh-route-explain-error-guard' justfile
    rg -q 'mesh-route-explain-operator-contract-selfcheck' justfile
    rg -q 'mesh-route-explain-operator-contract-check' justfile
    rg -q 'mesh-launch-connect-contract-selfcheck' justfile
    rg -q 'mesh-launch-connect-contract-check' justfile
    rg -q 'M0/M1 skeleton only' justfile
    rg -q 'listener is not started' justfile
    rg -q 'PARTIAL / NOT CLOSED' justfile
    rg -q 'cef-track-guard-selfcheck' justfile
    rg -q 'cef-track-guard' justfile
    rg -q 'cef-track-sync-guard-selfcheck' justfile
    rg -q 'cef-track-sync-guard' justfile
    rg -q 'cef-gap-map-guard-selfcheck' justfile
    rg -q 'cef-gap-map-guard' justfile
    rg -q 'cef-consistency-guard-selfcheck' justfile
    rg -q 'cef-consistency-guard' justfile
    rg -q 'prev_line=0' justfile
    rg -Fq 'rg -c "^- ${block}: "' justfile
    rg -q ' -eq 1' justfile
    rg -Fq 'test "\$block_line" -gt "\$prev_line"' justfile
    rg -Fq 'block_line="$(rg -n "^- ${block}: "' justfile
    rg -q 'Full cooperative mesh runtime' justfile
    rg -q 'DHT discovery' justfile
    rg -q 'Distributed Policy Store' justfile
    rg -q 'Cooperative relay participation/consent model' justfile
    rg -q 'Emergency/OOB carriers' justfile
    rg -q 'Roaming cache / distributed bootstrap continuation' justfile
    rg -q 'Reputation / complaint / relay credit subsystems' justfile
    rg -q 'count_current_fact=' justfile
    rg -q 'count_status=' justfile
    rg -q 'count_next_step=' justfile
    rg -q 'count_pdf_evidence=' justfile
    rg -q 'start_line=' justfile
    rg -q 'end_line=' justfile
    rg -Fq 'sed -n "${start_line},${end_line}p"' justfile
    rg -q 'evidence_line=' justfile
    rg -q 'current_line=' justfile
    rg -q 'status_line=' justfile
    rg -q 'next_step_line=' justfile
    rg -Fq 'test "\$evidence_line" -lt "\$current_line"' justfile
    rg -Fq 'test "\$current_line" -lt "\$status_line"' justfile
    rg -Fq 'test "\$status_line" -lt "\$next_step_line"' justfile
    rg -q 'test "\\\$count_current_fact" -ge 7' justfile
    rg -q 'test "\\\$count_status" -ge 7' justfile
    rg -q 'test "\\\$count_next_step" -ge 7' justfile
    rg -q 'test "\\\$count_pdf_evidence" -ge 7' justfile
    rg -q 'line_track=.*cef-track-guard' justfile
    rg -q 'line_track_sync=.*cef-track-sync-guard' justfile
    rg -q 'line_gap=.*cef-gap-map-guard' justfile
    rg -q 'line_phase1=.*cef-phase1-smoke' justfile
    line_mesh_ok="$(rg -n '^\s*just mesh-route-explain-guard$' justfile | head -n 1 | cut -d: -f1)"; line_mesh_err="$(rg -n '^\s*just mesh-route-explain-error-guard$' justfile | head -n 1 | cut -d: -f1)"; test -n "$line_mesh_ok" && test -n "$line_mesh_err" && test "$line_mesh_ok" -lt "$line_mesh_err"
    rg -Fq 'runtime_wired":(true|false),"gate_status":"(ready|partial)"' justfile
    rg -q 'benchmark-regression-selfcheck' justfile
    rg -q 'BENCHMARK_REGRESSION_GATE.json' justfile
    rg -q 'benchmark_regression_gate' justfile
    rg -q 'cef_gap_map_guard' justfile
    rg -q "rg -qv 'cef_gap_map_present' scripts/ship_readiness.sh" justfile
    rg -q "rg -qv 'cef_gap_map_present' docs/SHIP_READINESS_REPORT.json" justfile
    rg -q "rg -qv 'gap map present' docs/SHIP_READINESS_REPORT.md" justfile
    rg -q 'V1_MVP_BASELINE_MANIFEST.json' justfile
    rg -q 'Baseline integrity .*just baseline-verify.*PASS' justfile
    rg -q "grep -n '\\^1\\\\\\. .*benchmark-regression-selfcheck'" justfile
    rg -q "grep -n '\\^2\\\\\\. .*handoff-check'" justfile

ship-report-contract-check:
    just truth-contract-check
    just reality-truth-guard-selfcheck
    just reality-truth-guard
    just runtime-real-world-probe-schema-guard-selfcheck
    just runtime-real-world-probe-schema-guard
    just mesh-cli-recovery-schema-guard-selfcheck
    just mesh-cli-recovery-schema-guard
    just reality-audit-schema-guard-selfcheck
    just reality-audit-schema-guard
    just json-no-dupe-guard-selfcheck
    just json-no-dupe-guard
    just mvp-snapshot-verify-guard-selfcheck
    just mvp-snapshot-verify-guard
    just release-ru-guard-selfcheck
    just release-ru-guard
    just report-pack-guard-selfcheck
    just report-pack-guard
    just release-pack-schema-guard-selfcheck
    just release-pack-schema-guard
    just ship-structure-guard-selfcheck
    just ship-structure-guard
    just ship-nonregression-guard-selfcheck
    just ship-nonregression-guard
    just ship-readiness-json-guard-selfcheck
    just ship-readiness-json-guard
    just ship-readiness-freshness-guard-selfcheck
    just ship-readiness-freshness-guard
    test -f docs/CEF_TRACK_REPORT.json
    test -f docs/benchmark_baseline.json
    test -f docs/benchmark_latest.json
    rg -q '"status":"ok"' docs/SHIP_READINESS_REPORT.json
    rg -q '"kind":"ship_readiness_report"' docs/SHIP_READINESS_REPORT.json
    test "$(rg -c '"generated_at":"' docs/SHIP_READINESS_REPORT.json)" -eq 1
    rg -q '"generated_at":"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z"' docs/SHIP_READINESS_REPORT.json
    rg -q '"release_ok":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"release_ok_lab_only":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"cef_phase1_smoke_ok":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"cef_phase1_closed":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"mesh_auto_adaptive_ok":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"route_explain_recovery_schema_version":"mesh_recovery_v5"' docs/MESH_ROUTE_EXPLAIN.json
    rg -q '"route_explain_recovery_fields_checksum":"auto_recovery_attempts\|auto_recovery_final_result\|connect_retry_budget_exhausted\|connect_recovery_needed\|connect_recovery_strategy\|connect_recovery_projection_consistency\|connect_recovery_projection_key"' docs/MESH_ROUTE_EXPLAIN.json
    rg -q '"kind":"mesh_route_explain_error"' docs/MESH_ROUTE_EXPLAIN_ERROR.json
    rg -q '"route_explain_recovery_schema_version":"mesh_recovery_v5"' docs/MESH_ROUTE_EXPLAIN_ERROR.json
    rg -q '"route_explain_recovery_fields_checksum":"auto_recovery_attempts\|auto_recovery_final_result\|connect_retry_budget_exhausted\|connect_recovery_needed\|connect_recovery_strategy\|connect_recovery_projection_consistency\|connect_recovery_projection_key"' docs/MESH_ROUTE_EXPLAIN_ERROR.json
    rg -q '"auto_recovery_final_result":"not_applicable_error"' docs/MESH_ROUTE_EXPLAIN_ERROR.json
    rg -q '"connect_retry_budget_exhausted":"unknown"' docs/MESH_ROUTE_EXPLAIN_ERROR.json
    rg -q '"cef_phase1_smoke":true' docs/RELEASE_READINESS_REPORT.json
    rg -q '"network_state_not_modified":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_apply_smoke_modified":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_apply_route_smoke_modified":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_apply_route_existing_tun_smoke_modified":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_apply_route_multi_cidr_smoke_ok":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_route_policy_validation_smoke_ok":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_route_duplicate_cidr_validation_smoke_ok":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_tun_name_validation_smoke_ok":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_resolv_conf_validation_smoke_ok":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_datapath_multiflow_smoke_ok":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_policy_precedence_smoke_ok":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_forced_stop_rollback_smoke_ok":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_real_world_probe_smoke_ok":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_real_world_proxy_listener_detected":(true|false)' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_real_world_proxy_probe_attempted":(true|false)' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_real_world_proxy_probe_ok":(true|false)' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_real_world_proxy_probe_error":"(none|proxy_listener_not_found|proxy_connect_or_upstream_failed|unknown)"' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_real_world_direct_probe_ok":(true|false)' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_real_world_skipped_no_curl":(true|false)' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_real_world_skipped_no_proxy_listener":(true|false)' docs/SHIP_READINESS_REPORT.json
    rg -q '"artifacts_fresh":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"baseline_freeze":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"cleanroom_handoff_check":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"cef_track_report":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"cef_track_guard":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"cef_track_sync_guard":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"cef_gap_map_guard":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"cef_consistency_guard":true' docs/SHIP_READINESS_REPORT.json
    rg -qv 'cef_gap_map_present' docs/SHIP_READINESS_REPORT.json
    rg -q '"cef_gap_map_guard":true' docs/MVP_HANDOFF_CHECKLIST.md
    rg -qv 'cef_gap_map_present' docs/MVP_HANDOFF_CHECKLIST.md
    rg -q 'CEF gap map guard:' docs/SHIP_READINESS_REPORT.md
    rg -qv 'gap map present' docs/SHIP_READINESS_REPORT.md
    rg -q '"benchmark_regression_gate":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"release_readiness_report_json":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"release_readiness_report_ru":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"report_pack_json":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"report_pack_md":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"cef_phase1_smoke":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"mesh_auto_smoke":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"mesh_auto_adaptive_trace_guard":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_apply_dns_smoke":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_apply_route_smoke_selfcheck":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_apply_route_smoke":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_apply_route_existing_tun_smoke_selfcheck":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_apply_route_existing_tun_smoke":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_apply_route_multi_cidr_smoke_selfcheck":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_apply_route_multi_cidr_smoke":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_route_policy_validation_smoke_selfcheck":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_route_policy_validation_smoke":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_route_duplicate_cidr_validation_smoke_selfcheck":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_route_duplicate_cidr_validation_smoke":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_tun_name_validation_smoke_selfcheck":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_tun_name_validation_smoke":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_resolv_conf_validation_smoke_selfcheck":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_resolv_conf_validation_smoke":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_datapath_multiflow_smoke_selfcheck":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_datapath_multiflow_smoke":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_policy_precedence_smoke_selfcheck":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_policy_precedence_smoke":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_forced_stop_rollback_smoke_selfcheck":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_forced_stop_rollback_smoke":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"rust_no_hardcode_guard_selfcheck":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"rust_no_hardcode_guard":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_real_world_probe_smoke_selfcheck":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_real_world_probe_smoke":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"freshness_check":true' docs/SHIP_READINESS_REPORT.json
    rg -q '"runtime_apply_dns_verified":true' docs/RELEASE_READINESS_REPORT.json
    rg -q '"runtime_apply_route_verified":true' docs/RELEASE_READINESS_REPORT.json
    rg -q '"runtime_route_policy_validation_verified":true' docs/RELEASE_READINESS_REPORT.json
    rg -q '"runtime_tun_name_validation_verified":true' docs/RELEASE_READINESS_REPORT.json
    rg -q '"runtime_forced_stop_rollback_verified":true' docs/RELEASE_READINESS_REPORT.json
    rg -q '"status":"ok"' docs/BENCHMARK_REGRESSION_GATE.json
    rg -q '"kind":"benchmark_regression_gate"' docs/BENCHMARK_REGRESSION_GATE.json
    rg -q '"attempt":' docs/BENCHMARK_REGRESSION_GATE.json
    rg -q '"max_attempts":2' docs/BENCHMARK_REGRESSION_GATE.json
    rg -q '"baseline_file":"docs/benchmark_' docs/BENCHMARK_REGRESSION_GATE.json
    rg -q '"output_file":"docs/benchmark_latest.json"' docs/BENCHMARK_REGRESSION_GATE.json
    rg -q '"path": "docs/BENCHMARK_REGRESSION_GATE.json"' docs/V1_MVP_BASELINE_MANIFEST.json
    rg -q 'Baseline integrity \(`just baseline-verify`\): PASS' docs/SECOND_MACHINE_REPORT.md
    rg -q '"cef_phase1_smoke":true' docs/REPORT_PACK.json
    rg -q 'Truth boundary:' docs/REPORT_PACK.md
    rg -q '"status":"ok"' docs/RUNTIME_APPLY_DNS_SMOKE.json
    rg -q '"kind":"runtime_apply_dns_smoke"' docs/RUNTIME_APPLY_DNS_SMOKE.json
    rg -q '"network_state":"modified"' docs/RUNTIME_APPLY_DNS_SMOKE.json
    rg -q '"rollback_ok":true' docs/RUNTIME_APPLY_DNS_SMOKE.json
    rg -q '"status":"ok"' docs/RUNTIME_APPLY_ROUTE_SMOKE.json
    rg -q '"kind":"runtime_apply_route_smoke"' docs/RUNTIME_APPLY_ROUTE_SMOKE.json
    rg -q '"network_state":"modified"' docs/RUNTIME_APPLY_ROUTE_SMOKE.json
    rg -q '"apply_attempt_ok":true' docs/RUNTIME_APPLY_ROUTE_SMOKE.json
    rg -q '"policy_rule_ok":true' docs/RUNTIME_APPLY_ROUTE_SMOKE.json
    rg -q '"rollback_ok":true' docs/RUNTIME_APPLY_ROUTE_SMOKE.json
    rg -q '"status":"ok"' docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json
    rg -q '"kind":"runtime_apply_route_existing_tun_smoke"' docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json
    rg -q '"network_state":"modified"' docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json
    rg -q '"apply_attempt_ok":true' docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json
    rg -q '"preexisting_tun_used":true' docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json
    rg -q '"rollback_ok":true' docs/RUNTIME_APPLY_ROUTE_EXISTING_TUN_SMOKE.json
    rg -q '"status":"ok"' docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json
    rg -q '"kind":"runtime_apply_route_multi_cidr_smoke"' docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json
    rg -q '"network_state":"modified"' docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json
    (rg -q '"apply_attempt_ok":true' docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json && rg -q '"policy_rule_ok":true' docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json) || rg -q '"skipped_no_tun":true' docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json
    rg -q '"rollback_ok":true' docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json
    rg -q '"status":"ok"' docs/RUNTIME_ROUTE_POLICY_VALIDATION_SMOKE.json
    rg -q '"kind":"runtime_route_policy_validation_smoke"' docs/RUNTIME_ROUTE_POLICY_VALIDATION_SMOKE.json
    rg -q '"network_state":"not_modified"' docs/RUNTIME_ROUTE_POLICY_VALIDATION_SMOKE.json
    rg -q '"apply_rejected":true' docs/RUNTIME_ROUTE_POLICY_VALIDATION_SMOKE.json
    rg -q '"state_not_created":true' docs/RUNTIME_ROUTE_POLICY_VALIDATION_SMOKE.json
    rg -q '"status":"ok"' docs/RUNTIME_ROUTE_DUPLICATE_CIDR_VALIDATION_SMOKE.json
    rg -q '"kind":"runtime_route_duplicate_cidr_validation_smoke"' docs/RUNTIME_ROUTE_DUPLICATE_CIDR_VALIDATION_SMOKE.json
    rg -q '"network_state":"not_modified"' docs/RUNTIME_ROUTE_DUPLICATE_CIDR_VALIDATION_SMOKE.json
    rg -q '"apply_rejected":true' docs/RUNTIME_ROUTE_DUPLICATE_CIDR_VALIDATION_SMOKE.json
    rg -q '"state_not_created":true' docs/RUNTIME_ROUTE_DUPLICATE_CIDR_VALIDATION_SMOKE.json
    rg -q '"status":"ok"' docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json
    rg -q '"kind":"runtime_tun_name_validation_smoke"' docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json
    rg -q '"network_state":"not_modified"' docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json
    rg -q '"apply_rejected":true' docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json
    rg -q '"state_not_created":true' docs/RUNTIME_TUN_NAME_VALIDATION_SMOKE.json
    rg -q '"status":"ok"' docs/RUNTIME_RESOLV_CONF_VALIDATION_SMOKE.json
    rg -q '"kind":"runtime_resolv_conf_validation_smoke"' docs/RUNTIME_RESOLV_CONF_VALIDATION_SMOKE.json
    rg -q '"network_state":"not_modified"' docs/RUNTIME_RESOLV_CONF_VALIDATION_SMOKE.json
    rg -q '"apply_rejected":true' docs/RUNTIME_RESOLV_CONF_VALIDATION_SMOKE.json
    rg -q '"state_not_created":true' docs/RUNTIME_RESOLV_CONF_VALIDATION_SMOKE.json
    rg -q '"status":"ok"' docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json
    rg -q '"kind":"runtime_datapath_multiflow_smoke"' docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json
    rg -q '"network_state":"not_modified"' docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json
    rg -q '"gateway_ok":true' docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json
    rg -q '"block_ok":true' docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json
    rg -q '"direct_ok":true' docs/RUNTIME_DATAPATH_MULTIFLOW_SMOKE.json
    rg -q '"status":"ok"' docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json
    rg -q '"kind":"runtime_policy_precedence_smoke"' docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json
    rg -q '"network_state":"not_modified"' docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json
    rg -q '"precedence_ok":true' docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json
    rg -q '"all_matches_ok":true' docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json
    rg -q '"dns_binding_ok":true' docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json
    rg -q '"status":"ok"' docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json
    rg -q '"kind":"runtime_forced_stop_rollback_smoke"' docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json
    rg -q '"network_state":"modified"' docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json
    rg -q '"apply_attempt_ok":true' docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json
    rg -q '"recover_ok":true' docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json
    rg -q '"down_state_clean":true' docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json
    rg -q '"status":"ok"' docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json
    rg -q '"kind":"runtime_real_world_probe_smoke"' docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json
    rg -q '"network_state":"not_modified"' docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json
    rg -q 'just benchmark-regression-selfcheck' docs/SECOND_MACHINE_REPORT.md
    rg -q 'Benchmark regression gate script selfcheck: PASS' docs/SECOND_MACHINE_REPORT.md
    rg -q 'Release gate is lab-only' docs/SHIP_READINESS_REPORT.md
    rg -q '^Generated at \(UTC\): `[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z`$' docs/SHIP_READINESS_REPORT.md
    rg -q 'CEF phase1 smoke:' docs/SHIP_READINESS_REPORT.md
    rg -q 'Mesh auto adaptive trace:' docs/SHIP_READINESS_REPORT.md
    rg -q 'CEF phase1 smoke:' docs/REPORT_PACK.md
    rg -q 'Truth boundary:' docs/SHIP_READINESS_REPORT.md
    rg -q 'Runtime real-world proxy listener detected:' docs/SHIP_READINESS_REPORT.md
    rg -q 'Runtime real-world proxy probe attempted:' docs/SHIP_READINESS_REPORT.md
    rg -q 'Runtime real-world proxy probe ok:' docs/SHIP_READINESS_REPORT.md
    rg -q 'Runtime real-world proxy probe error:' docs/SHIP_READINESS_REPORT.md
    rg -q 'Runtime real-world proxy blocked targets total:' docs/SHIP_READINESS_REPORT.md
    rg -q 'Runtime real-world proxy blocked targets ok:' docs/SHIP_READINESS_REPORT.md
    rg -q 'Runtime real-world proxy blocked targets failed:' docs/SHIP_READINESS_REPORT.md
    rg -q 'Runtime real-world direct probe ok:' docs/SHIP_READINESS_REPORT.md
    rg -q 'Runtime real-world skipped no curl:' docs/SHIP_READINESS_REPORT.md
    rg -q 'Runtime real-world skipped no proxy listener:' docs/SHIP_READINESS_REPORT.md
    rg -q 'MVP готов только к расширенным лабораторным тестам' docs/RELEASE_READINESS_REPORT_RU.md
    rg -q 'не означает закрытие real-world datapath' docs/RELEASE_READINESS_REPORT_RU.md
    rg -q 'CEF phase1 smoke:' docs/RELEASE_READINESS_REPORT_RU.md

ship-readiness-json-guard:
    bash scripts/ship_readiness_json_guard.sh docs/SHIP_READINESS_REPORT.json docs/SHIP_READINESS_REPORT.md

ship-readiness-json-guard-selfcheck:
    test -x scripts/ship_readiness_json_guard.sh
    bash -n scripts/ship_readiness_json_guard.sh
    rg -q 'cargo run -q -p chimera-lab --bin ship_readiness_json_guard --' scripts/ship_readiness_json_guard.sh
    test -f crates/chimera-lab/src/bin/ship_readiness_json_guard.rs
    rg -q 'ship readiness json guard: PASS' crates/chimera-lab/src/bin/ship_readiness_json_guard.rs
    rg -q 'missing truth_boundary' crates/chimera-lab/src/bin/ship_readiness_json_guard.rs
    rg -q 'invalid CEF line order' crates/chimera-lab/src/bin/ship_readiness_json_guard.rs
    rg -q 'runtime real-world totals mismatch' crates/chimera-lab/src/bin/ship_readiness_json_guard.rs
    rg -q 'proxy probe attempted with empty totals' crates/chimera-lab/src/bin/ship_readiness_json_guard.rs
    rg -q 'proxy probe not attempted with non-zero totals' crates/chimera-lab/src/bin/ship_readiness_json_guard.rs
    rg -q 'proxy probe ok with failed targets' crates/chimera-lab/src/bin/ship_readiness_json_guard.rs
    rg -q 'proxy attempted without listener' crates/chimera-lab/src/bin/ship_readiness_json_guard.rs
    rg -q 'listener detected but skipped_no_proxy_listener=true' crates/chimera-lab/src/bin/ship_readiness_json_guard.rs
    rg -q 'proxy not attempted must set skipped_no_proxy_listener=true' crates/chimera-lab/src/bin/ship_readiness_json_guard.rs
    rg -q 'proxy attempted must set skipped_no_proxy_listener=false' crates/chimera-lab/src/bin/ship_readiness_json_guard.rs
    rg -q 'proxy not attempted must be listener_not_found' crates/chimera-lab/src/bin/ship_readiness_json_guard.rs
    rg -q 'proxy attempted with listener_not_found' crates/chimera-lab/src/bin/ship_readiness_json_guard.rs

ship-readiness-freshness-guard:
    bash scripts/ship_readiness_freshness_guard.sh docs/SHIP_READINESS_REPORT.json 1800

ship-readiness-freshness-guard-selfcheck:
    test -x scripts/ship_readiness_freshness_guard.sh
    bash -n scripts/ship_readiness_freshness_guard.sh
    rg -q 'cargo run -q -p chimera-lab --bin ship_readiness_freshness_guard --' scripts/ship_readiness_freshness_guard.sh
    test -f crates/chimera-lab/src/bin/ship_readiness_freshness_guard.rs
    rg -q 'ship readiness freshness guard: PASS' crates/chimera-lab/src/bin/ship_readiness_freshness_guard.rs
    rg -q 'missing required artifact' crates/chimera-lab/src/bin/ship_readiness_freshness_guard.rs
    rg -q 'stale/out-of-window artifact' crates/chimera-lab/src/bin/ship_readiness_freshness_guard.rs

ship-nonregression-guard:
    bash scripts/ship_nonregression_guard.sh \
      docs/SHIP_READINESS_REPORT.json \
      docs/RELEASE_READINESS_REPORT.json \
      docs/REPORT_PACK.json \
      docs/RUNTIME_APPLY_DNS_SMOKE.json \
      docs/RUNTIME_APPLY_ROUTE_SMOKE.json \
      docs/RUNTIME_APPLY_ROUTE_MULTI_CIDR_SMOKE.json \
      docs/RUNTIME_FORCED_STOP_ROLLBACK_SMOKE.json \
      docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json

ship-nonregression-guard-selfcheck:
    test -x scripts/ship_nonregression_guard.sh
    bash -n scripts/ship_nonregression_guard.sh
    rg -q 'cargo run -q -p chimera-lab --bin ship_nonregression_guard --' scripts/ship_nonregression_guard.sh
    test -f crates/chimera-lab/src/bin/ship_nonregression_guard.rs
    rg -q 'ship nonregression guard: PASS' crates/chimera-lab/src/bin/ship_nonregression_guard.rs
    rg -q 'truth_boundary mismatch' crates/chimera-lab/src/bin/ship_nonregression_guard.rs
    rg -q 'release_ok mismatch' crates/chimera-lab/src/bin/ship_nonregression_guard.rs
    rg -q 'proxy attempted without listener' crates/chimera-lab/src/bin/ship_nonregression_guard.rs
    rg -q 'proxy not attempted must be listener_not_found' crates/chimera-lab/src/bin/ship_nonregression_guard.rs
    rg -q 'proxy attempted with listener_not_found' crates/chimera-lab/src/bin/ship_nonregression_guard.rs
    rg -q 'proxy ok with failed targets' crates/chimera-lab/src/bin/ship_nonregression_guard.rs
    rg -q 'proxy attempted with empty target totals' crates/chimera-lab/src/bin/ship_nonregression_guard.rs
    rg -q 'proxy not attempted with non-zero totals' crates/chimera-lab/src/bin/ship_nonregression_guard.rs
    rg -q 'proxy error value is invalid' crates/chimera-lab/src/bin/ship_nonregression_guard.rs
    rg -q 'proxy totals mismatch' crates/chimera-lab/src/bin/ship_nonregression_guard.rs

reality-truth-guard:
    bash scripts/reality_truth_guard.sh \
      docs/REALITY_AUDIT_LATEST.json \
      docs/SHIP_READINESS_REPORT.json \
      docs/RELEASE_READINESS_REPORT.json \
      docs/REPORT_PACK.json \
      docs/MVP_SNAPSHOT.json \
      docs/MVP_VERIFY.json \
      docs/release_readiness_audit.json \
      docs/SHIP_READINESS_REPORT.md \
      docs/REPORT_PACK.md \
      docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json

reality-truth-guard-selfcheck:
    test -x scripts/reality_truth_guard.sh
    bash -n scripts/reality_truth_guard.sh
    rg -q 'cargo run -q -p chimera-lab --bin reality_truth_guard --' scripts/reality_truth_guard.sh
    test -f crates/chimera-lab/src/bin/reality_truth_guard.rs
    rg -q 'reality truth guard: PASS' crates/chimera-lab/src/bin/reality_truth_guard.rs
    rg -q 'truth boundary mismatch' crates/chimera-lab/src/bin/reality_truth_guard.rs
    rg -q 'markdown truth boundary mismatch' crates/chimera-lab/src/bin/reality_truth_guard.rs
    rg -q 'runtime_probe_proxy_error' crates/chimera-lab/src/bin/reality_truth_guard.rs
    rg -q 'str mismatch' crates/chimera-lab/src/bin/reality_truth_guard.rs
    rg -q 'proxy attempted without listener' crates/chimera-lab/src/bin/reality_truth_guard.rs
    rg -q 'proxy not attempted must be listener_not_found' crates/chimera-lab/src/bin/reality_truth_guard.rs
    rg -q 'proxy attempted with listener_not_found' crates/chimera-lab/src/bin/reality_truth_guard.rs
    rg -q 'proxy ok with failed targets' crates/chimera-lab/src/bin/reality_truth_guard.rs
    rg -q 'proxy error value is invalid' crates/chimera-lab/src/bin/reality_truth_guard.rs
    rg -q 'proxy totals mismatch' crates/chimera-lab/src/bin/reality_truth_guard.rs

ship-structure-guard:
    bash scripts/ship_structure_guard.sh \
      docs/SHIP_READINESS_REPORT.json \
      docs/SHIP_READINESS_REPORT.md

ship-structure-guard-selfcheck:
    test -x scripts/ship_structure_guard.sh
    bash -n scripts/ship_structure_guard.sh
    rg -q 'cargo run -q -p chimera-lab --bin ship_structure_guard --' scripts/ship_structure_guard.sh
    test -f crates/chimera-lab/src/bin/ship_structure_guard.rs
    rg -q 'ship structure guard: PASS' crates/chimera-lab/src/bin/ship_structure_guard.rs
    rg -q 'missing steps:' crates/chimera-lab/src/bin/ship_structure_guard.rs
    rg -q 'unexpected steps:' crates/chimera-lab/src/bin/ship_structure_guard.rs
    rg -q 'non-true steps:' crates/chimera-lab/src/bin/ship_structure_guard.rs

release-pack-schema-guard:
    bash scripts/release_pack_schema_guard.sh \
      docs/RELEASE_READINESS_REPORT.json \
      docs/REPORT_PACK.json

release-pack-schema-guard-selfcheck:
    test -x scripts/release_pack_schema_guard.sh
    bash -n scripts/release_pack_schema_guard.sh
    rg -q 'cargo run -q -p chimera-lab --bin release_pack_schema_guard --' scripts/release_pack_schema_guard.sh
    test -f crates/chimera-lab/src/bin/release_pack_schema_guard.rs
    rg -q 'release-pack schema guard: PASS' crates/chimera-lab/src/bin/release_pack_schema_guard.rs
    rg -q 'release top-level keys mismatch' crates/chimera-lab/src/bin/release_pack_schema_guard.rs
    rg -q 'pack top-level keys mismatch' crates/chimera-lab/src/bin/release_pack_schema_guard.rs

report-pack-guard:
    bash scripts/report_pack_guard.sh \
      docs/REPORT_PACK.json \
      docs/REPORT_PACK.md

report-pack-guard-selfcheck:
    test -x scripts/report_pack_guard.sh
    bash -n scripts/report_pack_guard.sh
    rg -q 'cargo run -q -p chimera-lab --bin report_pack_guard --' scripts/report_pack_guard.sh
    test -f crates/chimera-lab/src/bin/report_pack_guard.rs
    rg -q 'report pack guard: PASS' crates/chimera-lab/src/bin/report_pack_guard.rs
    rg -q 'truth_boundary mismatch' crates/chimera-lab/src/bin/report_pack_guard.rs
    rg -q 'section order mismatch' crates/chimera-lab/src/bin/report_pack_guard.rs

release-ru-guard:
    bash scripts/release_ru_guard.sh \
      docs/RELEASE_READINESS_REPORT.json \
      docs/RELEASE_READINESS_REPORT_RU.md

release-ru-guard-selfcheck:
    test -x scripts/release_ru_guard.sh
    bash -n scripts/release_ru_guard.sh
    rg -q 'cargo run -q -p chimera-lab --bin release_ru_guard --' scripts/release_ru_guard.sh
    test -f crates/chimera-lab/src/bin/release_ru_guard.rs
    rg -q 'release ru guard: PASS' crates/chimera-lab/src/bin/release_ru_guard.rs
    rg -q 'runtime order mismatch' crates/chimera-lab/src/bin/release_ru_guard.rs
    rg -q 'artifacts order mismatch' crates/chimera-lab/src/bin/release_ru_guard.rs

mvp-snapshot-verify-guard:
    bash scripts/mvp_snapshot_verify_guard.sh \
      docs/MVP_SNAPSHOT.json \
      docs/MVP_VERIFY.json

mvp-snapshot-verify-guard-selfcheck:
    test -x scripts/mvp_snapshot_verify_guard.sh
    bash -n scripts/mvp_snapshot_verify_guard.sh
    rg -q 'cargo run -q -p chimera-lab --bin mvp_snapshot_verify_guard --' scripts/mvp_snapshot_verify_guard.sh
    test -f crates/chimera-lab/src/bin/mvp_snapshot_verify_guard.rs
    rg -q 'mvp snapshot/verify guard: PASS' crates/chimera-lab/src/bin/mvp_snapshot_verify_guard.rs
    rg -q 'snapshot top-level keys mismatch' crates/chimera-lab/src/bin/mvp_snapshot_verify_guard.rs
    rg -q 'verify top-level keys mismatch' crates/chimera-lab/src/bin/mvp_snapshot_verify_guard.rs

json-no-dupe-guard:
    bash scripts/json_no_dupe_guard.sh \
      docs/SHIP_READINESS_REPORT.json \
      docs/CEF_TRACK_REPORT.json \
      docs/RELEASE_READINESS_REPORT.json \
      docs/REPORT_PACK.json \
      docs/MVP_SNAPSHOT.json \
      docs/MVP_VERIFY.json \
      docs/BENCHMARK_REGRESSION_GATE.json

json-no-dupe-guard-selfcheck:
    test -x scripts/json_no_dupe_guard.sh
    bash -n scripts/json_no_dupe_guard.sh
    rg -q 'cargo run -q -p chimera-lab --bin json_no_dupe_guard --' scripts/json_no_dupe_guard.sh
    test -f crates/chimera-lab/src/bin/json_no_dupe_guard.rs
    rg -q 'duplicate key' crates/chimera-lab/src/bin/json_no_dupe_guard.rs
    rg -q 'usage:' scripts/json_no_dupe_guard.sh

mvp-check:
    just check
    just test
    just lint
    just deny
    just rollback-smoke
    just rollback-json-smoke
    just runtime-apply-dns-smoke
    just runtime-apply-route-smoke-selfcheck
    just runtime-apply-route-smoke
    just runtime-apply-route-existing-tun-smoke-selfcheck
    just runtime-apply-route-existing-tun-smoke
    just runtime-route-policy-validation-smoke-selfcheck
    just runtime-route-policy-validation-smoke
    just runtime-tun-name-validation-smoke-selfcheck
    just runtime-tun-name-validation-smoke
    just runtime-resolv-conf-validation-smoke-selfcheck
    just runtime-resolv-conf-validation-smoke
    just runtime-datapath-multiflow-smoke-selfcheck
    just runtime-datapath-multiflow-smoke
    just runtime-policy-precedence-smoke-selfcheck
    just runtime-policy-precedence-smoke
    just fuzz-targets-check
    just diag-export
    just route-explain-json
    just datapath-report-json
    just client-doctor
    just gateway-doctor
    just lab-doctor
    just mvp-spec-check
    just mvp-spec-report
    just cef-track-report
    just cef-phase1-smoke
    just m5-artifacts-report
    just m6-artifacts-report
    just release-readiness-report
    just release-readiness-report-ru
    just release-readiness-audit-json
    just report-pack-json
    just report-pack
    just artifact-audit
    just mvp-snapshot
    just mvp-verify
    just hardening-smoke
    just benchmark-regression-selfcheck
    just benchmark-regression-check
    just json-message-contract-check
    just rollback-json-contract-check

handoff-check:
    just baseline-verify
    just mvp-check
    just release-readiness-report-json

ship-readiness:
    just ship-readiness-selfcheck
    just cleanroom-handoff-selfcheck
    bash scripts/ship_readiness.sh
    just ship-report-contract-check

automation-debt-guard:
    bash scripts/automation_debt_guard.sh docs/AUTOMATION_DEBT_REGISTER.md

automation-debt-guard-selfcheck:
    test -x scripts/automation_debt_guard.sh
    bash -n scripts/automation_debt_guard.sh
    rg -q 'automation debt guard: FAIL' scripts/automation_debt_guard.sh
    rg -q 'unresolved_items=' scripts/automation_debt_guard.sh

absolute-completion-lock-guard report_path="docs/COMMAND_EXECUTION_LOCK.json":
    bash scripts/absolute_completion_lock_guard.sh "{{report_path}}"

absolute-completion-lock-guard-selfcheck:
    test -x scripts/absolute_completion_lock_guard.sh
    bash -n scripts/absolute_completion_lock_guard.sh
    rg -q 'command_exhaustive_not_true' scripts/absolute_completion_lock_guard.sh
    rg -q 'verification_exhaustive_not_true' scripts/absolute_completion_lock_guard.sh
    rg -q 'evidence_exhaustive_not_true' scripts/absolute_completion_lock_guard.sh
    rg -q 'status_not_final' scripts/absolute_completion_lock_guard.sh
    pass_json="$(mktemp)"; \
    fail_json="$(mktemp)"; \
    printf '%s\n' '{"status":"pass","command_exhaustive":true,"verification_exhaustive":true,"evidence_exhaustive":true}' > "$pass_json"; \
    printf '%s\n' '{"status":"partial","command_exhaustive":true,"verification_exhaustive":true,"evidence_exhaustive":false}' > "$fail_json"; \
    bash scripts/absolute_completion_lock_guard.sh "$pass_json" | rg -q 'absolute completion lock guard: PASS'; \
    ! bash scripts/absolute_completion_lock_guard.sh "$fail_json" >/dev/null 2>&1; \
    rm -f "$pass_json" "$fail_json"

chimera-autonomous-nat-guard:
    bash scripts/chimera_autonomous_nat_guard.sh docs/CHIMERA_AUTONOMOUS_NAT_GUARD.json

chimera-autonomous-nat-guard-selfcheck:
    test -x scripts/chimera_autonomous_nat_guard.sh
    bash -n scripts/chimera_autonomous_nat_guard.sh
    rg -q 'chimera_autonomous_nat_guard' scripts/chimera_autonomous_nat_guard.sh
    rg -q 'upstream_adaptation_possible' scripts/chimera_autonomous_nat_guard.sh

chimera-upstream-autobootstrap-selfcheck:
    test -x scripts/chimera_upstream_autobootstrap.sh
    bash -n scripts/chimera_upstream_autobootstrap.sh
    rg -q 'upstream-autobootstrap: selected=' scripts/chimera_upstream_autobootstrap.sh
    rg -q 'CHIMERA_UPSTREAM_ENDPOINTS_CSV=' scripts/chimera_upstream_autobootstrap.sh
