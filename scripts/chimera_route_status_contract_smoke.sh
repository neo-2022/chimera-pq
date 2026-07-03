#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEBUG_CLI_BIN="$ROOT_DIR/target/debug/chimera-cli"
UNKNOWN_MODE_MARKER="datapath_mode=unknown"
TRANSPARENT_MODE_MARKER="datapath_mode=transparent"
UNVERIFIED_APPLY_MARKER="datapath_apply=unverified"
OK_APPLY_MARKER="datapath_apply=ok"

fail() {
  echo "chimera_route_status_contract_smoke=fail reason=$1" >&2
  exit 1
}

write_fake_systemctl() {
  local dest="${1:?dest_required}"
  cat >"$dest" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  --user) shift ;;
esac
case "${1:-}" in
  show-environment) exit 0 ;;
  is-active) echo active; exit 0 ;;
  *) exit 0 ;;
esac
EOF
  chmod +x "$dest"
}

write_actual_cli_runner() {
  local dest="${1:?dest_required}"
  local cli_bin="${2:?cli_bin_required}"
  cat >"$dest" <<EOF
#!/usr/bin/env bash
set -euo pipefail
case "\${1:-}" in
  cli)
    shift
    exec "$(printf '%q' "$cli_bin")" "\$@"
    ;;
  *)
    exit 0
    ;;
esac
EOF
  chmod +x "$dest"
}

run_case() {
  local case_name="${1:?case_name_required}"
  local state_payload="${2:-}"
  local flow_payload="${3:-}"
  local flow_touch_mode="${4:-}"
  local expected_mode="${5:?expected_mode_required}"
  local expected_apply="${6:?expected_apply_required}"
  local expected_proof="${7:?expected_proof_required}"
  local expected_flow_proof="${8:?expected_flow_proof_required}"
  local tmp_dir bin_dir runner state_file flow_file output datapath_output rc

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  runner="$tmp_dir/chimera-runner.sh"
  state_file="$tmp_dir/runtime_state.json"
  flow_file="${state_file}.flow.json"
  mkdir -p "$bin_dir" "$tmp_dir/cache" "$tmp_dir/config" "$tmp_dir/runtime"
  write_fake_systemctl "$bin_dir/systemctl"
  write_actual_cli_runner "$runner" "$DEBUG_CLI_BIN"

  if [[ -n "$state_payload" ]]; then
    printf '%s\n' "$state_payload" >"$state_file"
  fi
  if [[ -n "$flow_payload" ]]; then
    printf '%s\n' "$flow_payload" >"$flow_file"
    if [[ "$flow_touch_mode" == "stale" ]]; then
      touch -d '10 minutes ago' "$flow_file"
    fi
  fi

  set +e
  output="$(
    PATH="$bin_dir:$PATH" \
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$tmp_dir/cache" \
    XDG_CONFIG_HOME="$tmp_dir/config" \
    XDG_RUNTIME_DIR="$tmp_dir/runtime" \
    STATE_FILE="$state_file" \
    APP_ROUTES_FILE="$tmp_dir/app-routes.conf" \
    MANUAL_TRANSIT_DOMAINS_FILE="$tmp_dir/manual-transit.txt" \
    ADAPTIVE_DOMAINS_FILE="$tmp_dir/adaptive.txt" \
    NODE_CONFIG_FILE="$tmp_dir/missing-node.conf" \
    CHIMERA_RUNNER="$runner" \
    bash "$ROOT_DIR/scripts/chimera-control.sh" route-status 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "${case_name}:route_status_rc_${rc}:$output"
  grep -qx "datapath_mode=$expected_mode" <<<"$output" || fail "${case_name}:missing_datapath_mode_${expected_mode}:$output"
  grep -qx "datapath_apply=$expected_apply" <<<"$output" || fail "${case_name}:missing_datapath_apply_${expected_apply}:$output"
  grep -qx "datapath_proof=$expected_proof" <<<"$output" || fail "${case_name}:missing_datapath_proof_${expected_proof}:$output"
  grep -qx "datapath_flow_proof=$expected_flow_proof" <<<"$output" || fail "${case_name}:missing_datapath_flow_proof_${expected_flow_proof}:$output"
  if [[ "$expected_mode" != "transparent" ]]; then
    ! grep -qx 'datapath_mode=transparent' <<<"$output" || fail "${case_name}:false_transparent_datapath:$output"
  fi
  if [[ "$expected_apply" != "ok" ]]; then
    ! grep -qx 'datapath_apply=ok' <<<"$output" || fail "${case_name}:false_ok_apply:$output"
  fi
  if [[ "$expected_mode" == "transparent" ]]; then
    grep -qx 'datapath_proof=ok' <<<"$output" || fail "${case_name}:valid_state_missing_ok_proof:$output"
    grep -qx 'datapath_flow_proof=ok' <<<"$output" || fail "${case_name}:valid_state_missing_ok_flow_proof:$output"
    grep -qx 'runtime_state_status=up' <<<"$output" || fail "${case_name}:valid_state_missing_runtime_up:$output"
  fi

  set +e
  datapath_output="$(
    PATH="$bin_dir:$PATH" \
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$tmp_dir/cache" \
    XDG_CONFIG_HOME="$tmp_dir/config" \
    XDG_RUNTIME_DIR="$tmp_dir/runtime" \
    STATE_FILE="$state_file" \
    APP_ROUTES_FILE="$tmp_dir/app-routes.conf" \
    MANUAL_TRANSIT_DOMAINS_FILE="$tmp_dir/manual-transit.txt" \
    ADAPTIVE_DOMAINS_FILE="$tmp_dir/adaptive.txt" \
    NODE_CONFIG_FILE="$tmp_dir/missing-node.conf" \
    CHIMERA_RUNNER="$runner" \
    bash "$ROOT_DIR/scripts/chimera-control.sh" datapath-status 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "${case_name}:datapath_status_rc_${rc}:$datapath_output"
  grep -qx "datapath_mode=$expected_mode" <<<"$datapath_output" || fail "${case_name}:datapath_status_missing_datapath_mode_${expected_mode}:$datapath_output"
  grep -qx "datapath_apply=$expected_apply" <<<"$datapath_output" || fail "${case_name}:datapath_status_missing_datapath_apply_${expected_apply}:$datapath_output"
  grep -qx "datapath_proof=$expected_proof" <<<"$datapath_output" || fail "${case_name}:datapath_status_missing_datapath_proof_${expected_proof}:$datapath_output"
  grep -qx "datapath_flow_proof=$expected_flow_proof" <<<"$datapath_output" || fail "${case_name}:datapath_status_missing_datapath_flow_proof_${expected_flow_proof}:$datapath_output"
  grep -qx 'runtime_root_state=present' <<<"$datapath_output" || fail "${case_name}:datapath_status_missing_runtime_root_state:$datapath_output"
  grep -qx 'node_service_state=active' <<<"$datapath_output" || fail "${case_name}:datapath_status_missing_node_service_state:$datapath_output"
  grep -qx 'transparent_runtime_service_state=active' <<<"$datapath_output" || fail "${case_name}:datapath_status_missing_transparent_runtime_service_state:$datapath_output"
  grep -qx 'node_runtime=running' <<<"$datapath_output" || fail "${case_name}:datapath_status_missing_node_runtime:$datapath_output"
  grep -qx 'route_mode=split' <<<"$datapath_output" || fail "${case_name}:datapath_status_missing_route_mode:$datapath_output"
  grep -qx 'split_list_mode=allow' <<<"$datapath_output" || fail "${case_name}:datapath_status_missing_split_list_mode:$datapath_output"
  grep -qx 'node_config_ready=false' <<<"$datapath_output" || fail "${case_name}:datapath_status_missing_node_config_ready:$datapath_output"

  rm -rf "$tmp_dir"
}

cargo build -q -p chimera-cli --bin chimera-cli
[[ -x "$DEBUG_CLI_BIN" ]] || fail "debug_chimera_cli_missing_after_build"

valid_state_payload='{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true}'
valid_flow_payload='{"status":"ok","kind":"chimera_datapath_flow_proof","flow_id":"flow#1","path_kind":"local_egress_via_secure_peer","transparent_flow_observed":true,"counter_delta_ok":true,"secure_peer_egress_observed":true,"secure_peer_bytes_delta_ok":true,"network_state":"modified"}'

run_case "missing_state_negative_control" "" "" "" "unknown" "unverified" "missing_state" "skipped_apply_unverified"
run_case "invalid_json_negative_control" "{not json" "" "" "unknown" "unverified" "state_invalid_json" "skipped_apply_unverified"
run_case \
  "duplicate_key_negative_control" \
  '{"status":"down","status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true}' \
  "" \
  "" \
  "unknown" \
  "unverified" \
  "duplicate_field" \
  "skipped_apply_unverified"
run_case \
  "network_not_modified_negative_control" \
  '{"status":"up","network_state":"not_modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true}' \
  "" \
  "" \
  "unknown" \
  "unverified" \
  "network_not_modified" \
  "skipped_apply_unverified"
run_case \
  "tun_not_applied_negative_control" \
  '{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":false,"route_applied":true,"dns_applied":true}' \
  "" \
  "" \
  "unknown" \
  "unverified" \
  "tun_not_applied" \
  "skipped_apply_unverified"
run_case \
  "route_not_applied_negative_control" \
  '{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":false,"dns_applied":true}' \
  "" \
  "" \
  "unknown" \
  "unverified" \
  "route_not_applied" \
  "skipped_apply_unverified"
run_case \
  "dns_not_applied_negative_control" \
  '{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":false}' \
  "" \
  "" \
  "unknown" \
  "unverified" \
  "dns_not_applied" \
  "skipped_apply_unverified"
run_case \
  "valid_state_without_flow_proof_negative_control" \
  "$valid_state_payload" \
  "" \
  "" \
  "unknown" \
  "ok" \
  "ok" \
  "missing_flow_proof"
run_case \
  "invalid_flow_proof_negative_control" \
  "$valid_state_payload" \
  "{not json" \
  "" \
  "unknown" \
  "ok" \
  "ok" \
  "flow_invalid_json"
run_case \
  "stale_flow_proof_negative_control" \
  "$valid_state_payload" \
  "$valid_flow_payload" \
  "stale" \
  "unknown" \
  "ok" \
  "ok" \
  "flow_stale"
run_case \
  "valid_state_positive_control" \
  "$valid_state_payload" \
  "$valid_flow_payload" \
  "" \
  "transparent" \
  "ok" \
  "ok" \
  "ok"

echo "chimera_route_status_contract_smoke=pass"
