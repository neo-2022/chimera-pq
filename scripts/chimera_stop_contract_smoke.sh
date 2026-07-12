#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "chimera_stop_contract_smoke: $1" >&2
  exit 1
}

make_install_root() {
  local install_root="${1:?install_root_required}"
  mkdir -p "$install_root/scripts" "$install_root/bin" "$install_root/configs" "$install_root/deploy/systemd-user" "$install_root/deploy/desktop" "$install_root/docs"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cp "$ROOT_DIR/scripts/chimera-control-cleanup.inc" "$install_root/scripts/chimera-control-cleanup.inc"
  cp "$ROOT_DIR/scripts/chimera-runner.sh" "$install_root/scripts/chimera-runner.sh"
  cp "$ROOT_DIR/configs/mesh-node.example.conf" "$install_root/configs/mesh-node.example.conf"
}

write_fake_systemctl() {
  local bin_dir="${1:?bin_dir_required}"
  local ready="${2:?ready_required}"
  cat >"$bin_dir/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
ready="__READY__"
case "${1:-}" in
  --user)
    shift
    ;;
esac
case "${1:-}" in
  show-environment)
    [[ "$ready" == "1" ]] && exit 0
    exit 1
    ;;
  stop|is-active|list-units|list-unit-files|daemon-reload)
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
EOF
  sed -i "s|__READY__|$ready|g" "$bin_dir/systemctl"
  chmod +x "$bin_dir/systemctl"
}

write_fake_nft() {
  local bin_dir="${1:?bin_dir_required}"
  local mode="${2:?mode_required}"
  local record="${3:?record_required}"
  cat >"$bin_dir/nft" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"__RECORD__"
case "__MODE__" in
  ok)
    exit 0
    ;;
  missing)
    echo "Error: No such file or directory" >&2
    exit 1
    ;;
  denied)
    echo "Error: permission denied" >&2
    exit 1
    ;;
  *)
    exit 2
    ;;
esac
EOF
  sed -i -e "s|__MODE__|$mode|g" -e "s|__RECORD__|$record|g" "$bin_dir/nft"
  chmod +x "$bin_dir/nft"
}

write_fake_sudo() {
  local bin_dir="${1:?bin_dir_required}"
  local mode="${2:?mode_required}"
  local record="${3:?record_required}"
  cat >"$bin_dir/sudo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'sudo %s\n' "$*" >>"__RECORD__"
if [[ "__MODE__" == "missing" ]]; then
  exit 127
fi
if [[ "${1:-}" == "-n" ]]; then
  shift
fi
exec "$@"
EOF
  sed -i -e "s|__MODE__|$mode|g" -e "s|__RECORD__|$record|g" "$bin_dir/sudo"
  chmod +x "$bin_dir/sudo"
}

write_fake_runner() {
  local path="${1:?path_required}"
  local mode="${2:?mode_required}"
  local record="${3:?record_required}"
  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'runner %s\n' "$*" >>"__RECORD__"
if [[ "${1:-}" == "cli" && "${2:-}" == "down" ]]; then
  case "__MODE__" in
    ok) exit 0 ;;
    fail) exit 42 ;;
  esac
fi
exit 0
EOF
  sed -i -e "s|__MODE__|$mode|g" -e "s|__RECORD__|$record|g" "$path"
  chmod +x "$path"
}

run_control_case() {
  local case_name="${1:?case_name_required}"
  local control_cmd="${2:?control_cmd_required}"
  local systemd_ready="${3:?systemd_ready_required}"
  local nft_mode="${4:?nft_mode_required}"
  local table_name="${5:-}"
  local sudo_mode="${6:-ok}"
  local expect_rc="${7:?expect_rc_required}"
  local expected_status="${8:?expected_status_required}"
  local expect_delete="${9:?expect_delete_required}"
  local expected_table="${10:-chimera_redirect}"
  local expect_generated_cleanup="${11:-skip}"
  local down_mode="${12:-ok}"
  local tmp_dir bin_dir install_root cache_dir config_dir runtime_dir record output rc env_file fake_runner
  local state_file peer_state peer_update_state discovery_file discovery_pubkey_file

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  install_root="$tmp_dir/chimera-release"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  record="$tmp_dir/commands.log"
  mkdir -p "$bin_dir" "$cache_dir" "$config_dir/chimera" "$runtime_dir"
  touch "$record"
  make_install_root "$install_root"
  write_fake_systemctl "$bin_dir" "$systemd_ready"
  write_fake_nft "$bin_dir" "$nft_mode" "$record"
  write_fake_sudo "$bin_dir" "$sudo_mode" "$record"
  fake_runner="$tmp_dir/fake-runner.sh"
  write_fake_runner "$fake_runner" "$down_mode" "$record"

  state_file="$install_root/docs/runtime_state_latest.json"
  peer_state="$cache_dir/chimera/peer-egress.state"
  peer_update_state="$cache_dir/chimera/peer-update.state.json"
  discovery_file="$cache_dir/chimera/mesh_nodes.discovery.json"
  discovery_pubkey_file="$cache_dir/chimera/mesh_nodes.discovery.pubkey"
  mkdir -p "$(dirname "$state_file")" "$(dirname "$peer_state")"
  printf '%s\n' '{"status":"up"}' >"$state_file"
  printf '%s\n' present >"$peer_state"
  printf '%s\n' present >"$peer_update_state"
  printf '%s\n' present >"$discovery_file"
  printf '%s\n' present >"$discovery_pubkey_file"

  env_file="$config_dir/chimera/transparent-runtime.env"
  if [[ -n "$table_name" ]]; then
    printf 'CHIMERA_REDIRECT_TABLE=%s\n' "$table_name" >"$env_file"
  fi

  set +e
  output="$(
    PATH="$bin_dir:$PATH" \
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$cache_dir" \
    XDG_CONFIG_HOME="$config_dir" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    NFT_BIN="$bin_dir/nft" \
    CHIMERA_RUNNER="$fake_runner" \
    CHIMERA_ALLOW_TEST_NFT_BIN=1 \
    CHIMERA_UPDATE_FIRST_CHECKED=1 \
    timeout 10s bash "$install_root/scripts/chimera-control.sh" "$control_cmd" 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -ne 124 ]] || fail "$case_name: stop timed out"
  if [[ "$expect_rc" == "0" ]]; then
    [[ "$rc" -eq 0 ]] || fail "$case_name: expected rc=0, got $rc output=$output"
  else
    [[ "$rc" -ne 0 ]] || fail "$case_name: expected non-zero rc"
  fi
  [[ "$output" == *"$expected_status"* ]] || fail "$case_name: missing status '$expected_status' in output=$output"
  if [[ "$expect_delete" == "1" ]]; then
    rg -q '^sudo -n .*/nft delete table inet ' "$record" || fail "$case_name: nft delete was not invoked through sudo"
    rg -q "delete table inet $expected_table" "$record" || fail "$case_name: expected table delete missing"
  else
    if rg -q 'delete table inet' "$record"; then
      fail "$case_name: unexpected nft delete invocation"
    fi
  fi
  case "$expect_generated_cleanup" in
    1)
      [[ ! -e "$state_file" ]] || fail "$case_name: state file was not cleared"
      [[ ! -e "$peer_state" ]] || fail "$case_name: peer state was not cleared"
      [[ ! -e "$peer_update_state" ]] || fail "$case_name: peer update state was not cleared"
      [[ ! -e "$discovery_file" ]] || fail "$case_name: discovery snapshot was not cleared"
      [[ ! -e "$discovery_pubkey_file" ]] || fail "$case_name: discovery pubkey was not cleared"
      ;;
    0)
      [[ -e "$state_file" ]] || fail "$case_name: state file was unexpectedly cleared"
      ;;
  esac

  rm -rf "$tmp_dir"
}

run_rejects_non_nft_override_case() {
  local tmp_dir bin_dir install_root cache_dir config_dir runtime_dir record output rc fake_nft fake_runner

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  install_root="$tmp_dir/chimera-release"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  record="$tmp_dir/commands.log"
  mkdir -p "$bin_dir" "$cache_dir" "$config_dir/chimera" "$runtime_dir"
  touch "$record"
  make_install_root "$install_root"
  write_fake_systemctl "$bin_dir" "1"
  write_fake_sudo "$bin_dir" "ok" "$record"
  fake_runner="$tmp_dir/fake-runner.sh"
  write_fake_runner "$fake_runner" "ok" "$record"
  fake_nft="$bin_dir/not-nft"
  cat >"$fake_nft" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "unexpected non-nft override execution" >&2
exit 0
EOF
  chmod +x "$fake_nft"

  set +e
  output="$(
    PATH="$bin_dir:$PATH" \
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$cache_dir" \
    XDG_CONFIG_HOME="$config_dir" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    NFT_BIN="$fake_nft" \
    CHIMERA_RUNNER="$fake_runner" \
    timeout 10s bash "$install_root/scripts/chimera-control.sh" stop 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "non_nft_override: expected no-op rc=0 for unavailable nft override, got $rc output=$output"
  [[ "$output" == *"transparent_redirect_cleanup=skipped reason=nft_missing"* ]] || fail "non_nft_override: missing skipped diagnostic"
  [[ "$output" != *"unexpected non-nft override execution"* ]] || fail "non_nft_override: unsafe override was executed"
  ! rg -q '^sudo ' "$record" || fail "non_nft_override: sudo should not be invoked"

  rm -rf "$tmp_dir"
}

run_stop_case() {
  run_control_case "$1" "stop" "${@:2}"
}

run_stop_case "systemd_stop_deletes_default_redirect_table" "1" "ok" "" "ok" "0" "stop_status=ok mode=systemd_user" "1"
run_stop_case "systemd_stop_missing_redirect_table_is_idempotent" "1" "missing" "" "ok" "0" "stop_status=ok mode=systemd_user" "1"
run_stop_case "direct_stop_deletes_default_redirect_table" "0" "ok" "" "ok" "0" "stop_status=ok mode=direct" "1"
run_stop_case "systemd_stop_clears_generated_runtime_state" "1" "ok" "" "ok" "0" "stop_status=ok mode=systemd_user" "1" "chimera_redirect" "1"
run_stop_case "env_file_chimera_owned_redirect_table_is_allowed" "1" "ok" "chimera_redirect_stoptest" "ok" "0" "stop_status=ok mode=systemd_user" "1" "chimera_redirect_stoptest"
run_stop_case "invalid_redirect_table_fails_closed" "1" "ok" "bad;name" "ok" "1" "reason=transparent_redirect_cleanup_failed" "0"
run_stop_case "foreign_valid_table_fails_closed" "1" "ok" "filter" "ok" "1" "reason=transparent_redirect_cleanup_failed" "0"
run_stop_case "sudo_execution_failure_fails_stop" "1" "ok" "" "missing" "1" "reason=transparent_redirect_cleanup_failed" "1"
run_stop_case "datapath_down_failure_fails_closed" "1" "ok" "" "ok" "1" "reason=datapath_down_failed" "1" "chimera_redirect" "0" "fail"
run_rejects_non_nft_override_case

run_control_case "restart_does_not_hide_cleanup_failure" "restart" "1" "denied" "" "ok" "1" "restart_status=fail reason=stop_failed" "1"
run_control_case "uninstall_does_not_hide_cleanup_failure" "uninstall" "1" "denied" "" "ok" "1" "uninstall_status=fail reason=stop_failed" "1"

run_stop_rejects_direct_fallback_when_systemd_units_present_case() {
  local tmp_dir bin_dir install_root cache_dir config_dir runtime_dir record output rc fake_runner

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  install_root="$tmp_dir/chimera-release"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  record="$tmp_dir/commands.log"
  mkdir -p "$bin_dir" "$cache_dir/chimera" "$config_dir/chimera" "$config_dir/systemd/user" "$runtime_dir"
  touch "$record"
  make_install_root "$install_root"
  write_fake_systemctl "$bin_dir" "0"
  write_fake_nft "$bin_dir" "ok" "$record"
  write_fake_sudo "$bin_dir" "ok" "$record"
  fake_runner="$tmp_dir/fake-runner.sh"
  write_fake_runner "$fake_runner" "ok" "$record"
  printf '%s\n' '[Unit]' >"$config_dir/systemd/user/chimera-runtime.service"

  set +e
  output="$(
    PATH="$bin_dir:$PATH" \
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$cache_dir" \
    XDG_CONFIG_HOME="$config_dir" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    NFT_BIN="$bin_dir/nft" \
    CHIMERA_RUNNER="$fake_runner" \
    CHIMERA_ALLOW_TEST_NFT_BIN=1 \
    CHIMERA_UPDATE_FIRST_CHECKED=1 \
    timeout 10s bash "$install_root/scripts/chimera-control.sh" stop 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -ne 124 ]] || fail "stop_rejects_direct_fallback_when_systemd_units_present_case: stop timed out"
  [[ "$rc" -ne 0 ]] || fail "stop_rejects_direct_fallback_when_systemd_units_present_case: expected non-zero rc"
  [[ "$output" == *"stop_status=fail mode=preflight reason=user_systemd_session_unavailable units_on_disk=true"* ]] || fail "stop_rejects_direct_fallback_when_systemd_units_present_case: missing fail-closed status output=$output"
  if rg -q 'delete table inet' "$record"; then
    fail "stop_rejects_direct_fallback_when_systemd_units_present_case: nft cleanup should not run"
  fi
  if rg -q '^runner ' "$record"; then
    fail "stop_rejects_direct_fallback_when_systemd_units_present_case: runner should not run"
  fi

  rm -rf "$tmp_dir"
}

run_stop_rejects_direct_fallback_when_systemd_units_present_case

echo "chimera_stop_contract_smoke=pass"
