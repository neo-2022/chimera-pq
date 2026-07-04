#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "chimera_start_contract_smoke: $1" >&2
  exit 1
}

run_case() {
  local case_name="$1"
  local node_ready="$2"
  local systemctl_mode="$3"
  local expected_reason="${4:-service_failure}"
  local expect_systemctl_start="${5:-1}"
  local tmp_dir systemctl_dir cache_dir config_dir runtime_dir node_conf output rc install_root bootstrap_marker bootstrap_script

  tmp_dir="$(mktemp -d)"
  systemctl_dir="$tmp_dir/bin"
  cache_dir="$tmp_dir/cache/chimera"
  config_dir="$tmp_dir/config/chimera"
  runtime_dir="$tmp_dir/runtime"
  install_root="$tmp_dir/chimera-release"
  mkdir -p "$systemctl_dir" "$cache_dir" "$config_dir" "$runtime_dir"
  mkdir -p "$install_root/scripts" "$install_root/bin" "$install_root/configs" "$install_root/deploy/systemd-user" "$install_root/deploy/desktop"
  cp "$ROOT_DIR/scripts/chimera-sh" "$install_root/scripts/chimera-sh"
  cp "$ROOT_DIR/scripts/chimera-update.sh" "$install_root/scripts/chimera-update.sh"
  cp "$ROOT_DIR/scripts/chimera-update-runtime-state.sh" "$install_root/scripts/chimera-update-runtime-state.sh"
  cp "$ROOT_DIR/scripts/chimera-update-rerun.sh" "$install_root/scripts/chimera-update-rerun.sh"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cp "$ROOT_DIR/scripts/install_desktop_control.sh" "$install_root/scripts/install_desktop_control.sh"
  cp "$ROOT_DIR/deploy/systemd-user/chimera-node.service" "$install_root/deploy/systemd-user/chimera-node.service"
  cp "$ROOT_DIR/deploy/systemd-user/chimera-datapath.service" "$install_root/deploy/systemd-user/chimera-datapath.service"
  cp "$ROOT_DIR/deploy/desktop/chimera-control-gui.desktop" "$install_root/deploy/desktop/chimera-control-gui.desktop"
  cp "$ROOT_DIR/configs/mesh-node.example.conf" "$install_root/configs/mesh-node.example.conf"
  printf '%s\n' "0.1.86" >"$install_root/.chimera_release_version"
  printf '%064d\n' 1 >"$install_root/.chimera_release_bundle.sha256"

  node_conf="$tmp_dir/mesh-node.conf"
  if [[ "$node_ready" == "1" ]]; then
    cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = carrier.mesh:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
  elif [[ "$node_ready" == "tcp_doc_placeholder" ]]; then
    cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = tcp://203.0.113.10:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
  else
    cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = 203.0.113.10:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
  fi

  cat >"$systemctl_dir/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
cache_root="${XDG_CACHE_HOME:-${HOME:-}/.cache}"
node_log="$cache_root/chimera/chimera_node.service.log"
datapath_log="$cache_root/chimera/chimera_datapath.service.log"
mode="__MODE__"
count_dir="${TMPDIR:-/tmp}/chimera-start-contract-counts"
mkdir -p "$count_dir"
case "${1:-}" in
  --user)
    shift
    ;;
esac
case "${1:-}" in
  show-environment|daemon-reload)
    exit 0
    ;;
  start)
    if [[ ! -f "$node_log" || ! -f "$datapath_log" ]]; then
      exit 209
    fi
    exit 0
    ;;
  is-active)
    local_unit="${2:-}"
    case "$mode" in
      node_flap)
        if [[ "$local_unit" == "chimera-node.service" ]]; then
          count_file="$count_dir/node_flap.count"
          count="0"
          if [[ -f "$count_file" ]]; then
            read -r count <"$count_file" 2>/dev/null || count="0"
          fi
          count=$((count + 1))
          printf '%s\n' "$count" >"$count_file"
          if (( count <= 2 )); then
            echo "active"
            exit 0
          fi
          echo "failed"
          exit 3
        fi
        ;;
    esac
    if [[ "$mode" == "node_fail" && "${2:-}" == "chimera-node.service" ]]; then
      echo "failed"
      exit 3
    fi
    if [[ "$mode" == "datapath_fail" && "${2:-}" == "chimera-datapath.service" ]]; then
      echo "failed"
      exit 3
    fi
    echo "active"
    exit 0
    ;;
  stop|list-units|list-unit-files)
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
EOF
  sed -i "s|__MODE__|$systemctl_mode|g" "$systemctl_dir/systemctl"
  chmod +x "$systemctl_dir/systemctl"

  bootstrap_marker="$tmp_dir/bootstrap_invoked"
  bootstrap_script="$tmp_dir/forbidden-runtime-bootstrap.sh"
  cat >"$bootstrap_script" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' invoked >"$bootstrap_marker"
exit 42
EOF
  chmod +x "$bootstrap_script"

  set +e
  output="$(
		    PATH="$systemctl_dir:$PATH" \
		    HOME="$tmp_dir/home" \
	    XDG_CACHE_HOME="$tmp_dir/cache" \
	    XDG_CONFIG_HOME="$config_dir" \
	    XDG_RUNTIME_DIR="$runtime_dir" \
    NODE_CONFIG_FILE="$node_conf" \
    RUNTIME_BOOTSTRAP_SCRIPT="$bootstrap_script" \
    CHIMERA_UPDATE_BOOTSTRAP_URL="http://127.0.0.1:9/chimera.sh" \
    CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URLS_DEFAULT="http://127.0.0.1:9/chimera.sh" \
    CHIMERA_UPDATE_DOWNLOAD_CONNECT_TIMEOUT_SEC=1 \
    CHIMERA_UPDATE_DOWNLOAD_MAX_TIME_SEC=1 \
	    CHIMERA_UPDATE_DOWNLOAD_RETRIES=0 \
	    CHIMERA_AUTOFIX_MAX_TIME=0 \
		    timeout 20s bash "$install_root/scripts/chimera-sh" -start 2>&1
		  )"
  rc=$?
  set -e

  [[ "$rc" -ne 124 ]] || fail "$case_name: launcher start timed out before contract result"
  [[ "$rc" -ne 0 ]] || fail "$case_name: expected non-zero exit"
  [[ "$output" == *"start_status=fail"* ]] || fail "$case_name: missing fail status"
  [[ "$output" != *"start_status=ok"* ]] || fail "$case_name: false ok status leaked"
  if [[ "$expect_systemctl_start" == "1" ]]; then
    [[ "$output" == *"systemctl_start_rc=0"* ]] || fail "$case_name: systemctl did not see prepared log targets"
  else
    [[ "$output" != *"systemctl_start_rc="* ]] || fail "$case_name: systemd start happened after preflight failure"
  fi
  case "$expected_reason" in
    service_failure)
      [[ "$output" == *"reason=node_service_failed"* || "$output" == *"reason=transparent_service_failed"* ]] || fail "$case_name: missing failure reason"
      ;;
    *)
      [[ "$output" == *"reason=$expected_reason"* ]] || fail "$case_name: missing failure reason=$expected_reason output=$output"
      ;;
  esac
  [[ ! -e "$bootstrap_marker" ]] || fail "$case_name: legacy third-party runtime bootstrap was invoked"
  [[ -f "$cache_dir/chimera_node.service.log" ]] || fail "$case_name: node log file missing"
  [[ -f "$cache_dir/chimera_datapath.service.log" ]] || fail "$case_name: datapath log file missing"

  rm -rf "$tmp_dir"
}

run_case "node_service_failure" "1" "node_fail"
run_case "node_flap_failure" "1" "node_flap"
run_case "datapath_service_failure" "1" "datapath_fail"
run_case "datapath_unconfigured_failure" "0" "ok" "datapath_unconfigured" "0"
run_case "datapath_unconfigured_tcp_doc_placeholder_failure" "tcp_doc_placeholder" "ok" "datapath_unconfigured" "0"

run_systemd_bound_transit_missing_authority_does_not_block_start_case() {
  local tmp_dir bin_dir cache_dir config_dir runtime_dir install_root node_conf output rc fake_systemctl systemctl_log

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  install_root="$tmp_dir/chimera-release"
  node_conf="$tmp_dir/mesh-node.conf"
  fake_systemctl="$bin_dir/systemctl"
  systemctl_log="$tmp_dir/systemctl.log"
  : >"$systemctl_log"

  mkdir -p "$bin_dir" "$cache_dir" "$config_dir/chimera" "$runtime_dir" "$install_root/scripts" "$install_root/configs"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cp "$ROOT_DIR/configs/mesh-node.example.conf" "$install_root/configs/mesh-node.example.conf"
  cat >"$node_conf" <<'EOF'
carrier.profile = tls
carrier.addr = ${CHIMERA_NODE_PEER_ENDPOINT}
carrier.server_name = ${CHIMERA_NODE_SERVER_NAME}
capture.mode = tun
capture.tun_supported = true
peer.listen_addr = ${CHIMERA_NODE_LISTEN_ADDR}
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF

  cat >"$config_dir/chimera/peer-egress.env" <<EOF
CHIMERA_PEER_EGRESS_MODE=node
CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:0
CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:0
CHIMERA_PEER_EGRESS_STATE_FILE=$tmp_dir/peer-egress.state
CHIMERA_MESH_PEER_EGRESS_STATE_PATH=$tmp_dir/peer-egress.state
CHIMERA_PEER_EGRESS_TOKEN=test-token
CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=true
EOF

  cat >"$fake_systemctl" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >>"$systemctl_log"
case "\${1:-}" in
  --user) shift ;;
esac
case "\${1:-}" in
  show-environment|daemon-reload|start)
    exit 0
    ;;
  is-active)
    echo "active"
    exit 0
    ;;
  stop|list-units|list-unit-files)
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
EOF
  chmod +x "$fake_systemctl"

  set +e
  output="$(
    PATH="$bin_dir:$PATH" \
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$cache_dir" \
    XDG_CONFIG_HOME="$config_dir" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    NODE_CONFIG_FILE="$node_conf" \
    CHIMERA_UPDATE_FIRST_CHECKED=1 \
    SITE_AUTOWATCH_ENABLED=0 \
    timeout 20s bash "$install_root/scripts/chimera-control.sh" start 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "systemd_bound_transit_missing_authority_start_progress: expected rc=0 output=$output"
  [[ "$output" == *"start_status=partial"* ]] || fail "systemd_bound_transit_missing_authority_start_progress: missing partial status output=$output"
  [[ "$output" == *"mode=listener_only"* ]] || fail "systemd_bound_transit_missing_authority_start_progress: missing listener_only mode output=$output"
  [[ "$output" == *"reason=node_endpoint_unconfigured_listener_only"* ]] || fail "systemd_bound_transit_missing_authority_start_progress: missing listener_only reason output=$output"
  [[ "$output" != *"reason=bound_transit_unready"* ]] || fail "systemd_bound_transit_missing_authority_start_progress: bound transit preflight still blocked clean start output=$output"
  grep -q '^--user start chimera-node.service$' "$systemctl_log" || fail "systemd_bound_transit_missing_authority_start_progress: node service was not started"
  ! grep -q '^--user start chimera-datapath.service$' "$systemctl_log" || fail "systemd_bound_transit_missing_authority_start_progress: datapath service should stay skipped in listener_only"

  rm -rf "$tmp_dir"
}

run_systemd_bound_transit_missing_authority_does_not_block_start_case

run_systemd_listener_only_unconfigured_endpoint_case() {
  local tmp_dir bin_dir cache_dir config_dir runtime_dir install_root node_conf output rc fake_systemctl systemctl_log

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  install_root="$tmp_dir/chimera-release"
  node_conf="$tmp_dir/mesh-node.conf"
  fake_systemctl="$bin_dir/systemctl"
  systemctl_log="$tmp_dir/systemctl.log"
  : >"$systemctl_log"

  mkdir -p "$bin_dir" "$cache_dir" "$config_dir/chimera" "$runtime_dir" "$install_root/scripts" "$install_root/configs"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cp "$ROOT_DIR/configs/mesh-node.example.conf" "$install_root/configs/mesh-node.example.conf"
  cat >"$node_conf" <<'EOF'
carrier.profile = tls
carrier.addr = ${CHIMERA_NODE_PEER_ENDPOINT}
carrier.server_name = ${CHIMERA_NODE_SERVER_NAME}
capture.mode = tun
capture.tun_supported = true
peer.listen_addr = ${CHIMERA_NODE_LISTEN_ADDR}
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF

  cat >"$config_dir/chimera/peer-egress.env" <<EOF
CHIMERA_PEER_EGRESS_MODE=node
CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:0
CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:0
CHIMERA_PEER_EGRESS_STATE_FILE=$tmp_dir/peer-egress.state
CHIMERA_MESH_PEER_EGRESS_STATE_PATH=$tmp_dir/peer-egress.state
CHIMERA_PEER_EGRESS_TOKEN=test-token
CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=false
EOF

  cat >"$fake_systemctl" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >>"$systemctl_log"
case "\${1:-}" in
  --user) shift ;;
esac
case "\${1:-}" in
  show-environment|daemon-reload|start)
    exit 0
    ;;
  is-active)
    echo "active"
    exit 0
    ;;
  stop|list-units|list-unit-files)
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
EOF
  chmod +x "$fake_systemctl"

  set +e
  output="$(
    PATH="$bin_dir:$PATH" \
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$cache_dir" \
    XDG_CONFIG_HOME="$config_dir" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    NODE_CONFIG_FILE="$node_conf" \
    CHIMERA_UPDATE_FIRST_CHECKED=1 \
    SITE_AUTOWATCH_ENABLED=0 \
    timeout 20s bash "$install_root/scripts/chimera-control.sh" start 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "systemd_listener_only_unconfigured_endpoint: expected rc=0 got $rc output=$output"
  [[ "$output" == *"start_status=partial"* ]] || fail "systemd_listener_only_unconfigured_endpoint: missing partial status output=$output"
  [[ "$output" == *"mode=listener_only"* ]] || fail "systemd_listener_only_unconfigured_endpoint: missing listener_only mode output=$output"
  [[ "$output" == *"mesh_ready=false"* ]] || fail "systemd_listener_only_unconfigured_endpoint: missing mesh_ready=false output=$output"
  [[ "$output" == *"reason=node_endpoint_unconfigured_listener_only"* ]] || fail "systemd_listener_only_unconfigured_endpoint: missing reason output=$output"
  [[ "$output" == *"transparent_runtime=skipped"* ]] || fail "systemd_listener_only_unconfigured_endpoint: missing transparent_runtime=skipped output=$output"
  [[ "$output" == *"datapath_apply=skipped"* ]] || fail "systemd_listener_only_unconfigured_endpoint: missing datapath_apply=skipped output=$output"
  grep -q '^--user start chimera-node.service$' "$systemctl_log" || fail "systemd_listener_only_unconfigured_endpoint: node service was not started"
  ! grep -q '^--user start chimera-datapath.service$' "$systemctl_log" || fail "systemd_listener_only_unconfigured_endpoint: datapath service should not start in listener_only mode"

  rm -rf "$tmp_dir"
}

run_systemd_listener_only_unconfigured_endpoint_case

run_systemd_apply_failure_case() {
  local tmp_dir bin_dir cache_dir config_dir runtime_dir install_root node_conf output rc fake_runner fake_systemctl fake_ip runner_log rollback_marker systemctl_log state_file ip_log tun_marker

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  install_root="$tmp_dir/chimera-release"
  node_conf="$tmp_dir/mesh-node.conf"
  fake_runner="$tmp_dir/fake-runner.sh"
  fake_systemctl="$bin_dir/systemctl"
  fake_ip="$bin_dir/ip"
  runner_log="$tmp_dir/runner.log"
  rollback_marker="$tmp_dir/rollback_invoked"
  systemctl_log="$tmp_dir/systemctl.log"
  state_file="$tmp_dir/runtime_state.json"
  ip_log="$tmp_dir/ip.log"
  tun_marker="$tmp_dir/chimera0.present"

  mkdir -p "$bin_dir" "$cache_dir" "$config_dir" "$runtime_dir" "$install_root/scripts" "$install_root/configs"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cp "$ROOT_DIR/configs/mesh-node.example.conf" "$install_root/configs/mesh-node.example.conf"
  cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = carrier.mesh:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF

  cat >"$fake_systemctl" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >>"$systemctl_log"
case "\${1:-}" in
  --user) shift ;;
esac
case "\${1:-}" in
  show-environment|daemon-reload)
    exit 0
    ;;
  start)
    exit 0
    ;;
  is-active)
    echo "active"
    exit 0
    ;;
  stop|list-units|list-unit-files)
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
EOF
  chmod +x "$fake_systemctl"

  cat >"$fake_ip" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >>"$ip_log"
if [[ "\${1:-}" == "link" && "\${2:-}" == "show" && "\${3:-}" == "dev" && "\${4:-}" == "chimera0" ]]; then
  [[ -f "$tun_marker" ]] && exit 0
  exit 1
fi
if [[ "\${1:-}" == "link" && "\${2:-}" == "delete" && "\${3:-}" == "dev" && "\${4:-}" == "chimera0" ]]; then
  rm -f "$tun_marker"
  exit 0
fi
exit 0
EOF
  chmod +x "$fake_ip"

  cat >"$fake_runner" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >>"$runner_log"
case "\${1:-}" in
  cli)
    if [[ "\${2:-}" == "up" ]]; then
      state_file=""
      prev=""
      for arg in "\$@"; do
        if [[ "\$prev" == "--state-file" ]]; then
          state_file="\$arg"
          break
        fi
        prev="\$arg"
      done
      if [[ -n "\$state_file" ]]; then
        mkdir -p "\$(dirname "\$state_file")"
        printf '%s\n' '{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":false,"tun_device":"chimera-test","route_cidrs_applied":"10.0.0.0/8"}' >"\$state_file"
      fi
      printf '%s\n' present >"$tun_marker"
      exit 42
    fi
    if [[ "\${2:-}" == "rollback" && "\${3:-}" == "recover" ]]; then
      state_file=""
      prev=""
      for arg in "\$@"; do
        if [[ "\$prev" == "--state-file" ]]; then
          state_file="\$arg"
          break
        fi
        prev="\$arg"
      done
      printf '%s\n' invoked >"$rollback_marker"
      [[ -n "\$state_file" ]] && rm -f "\$state_file"
      exit 0
    fi
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
EOF
  chmod +x "$fake_runner"

  set +e
  output="$(
    PATH="$bin_dir:$PATH" \
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$cache_dir" \
    XDG_CONFIG_HOME="$config_dir" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    NODE_CONFIG_FILE="$node_conf" \
    STATE_FILE="$state_file" \
    CHIMERA_RUNNER="$fake_runner" \
    CHIMERA_UPDATE_FIRST_CHECKED=1 \
    SITE_AUTOWATCH_ENABLED=0 \
    timeout 20s bash "$install_root/scripts/chimera-control.sh" start 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -ne 124 ]] || fail "systemd_datapath_apply_failure: control start timed out before contract result"
  [[ "$rc" -ne 0 ]] || fail "systemd_datapath_apply_failure: expected non-zero exit"
  [[ "$output" == *"start_status=fail"* ]] || fail "systemd_datapath_apply_failure: missing fail status output=$output"
  [[ "$output" == *"mode=systemd_user"* ]] || fail "systemd_datapath_apply_failure: missing systemd mode output=$output"
  [[ "$output" == *"reason=datapath_apply_failed"* ]] || fail "systemd_datapath_apply_failure: missing datapath_apply_failed output=$output"
  [[ "$output" == *"datapath_apply=failed"* ]] || fail "systemd_datapath_apply_failure: missing datapath_apply=failed output=$output"
  [[ "$output" == *"apply_rc=42"* ]] || fail "systemd_datapath_apply_failure: missing apply_rc=42 output=$output"
  [[ "$output" == *"datapath_rollback=ok"* ]] || fail "systemd_datapath_apply_failure: missing datapath_rollback=ok output=$output"
  [[ "$output" == *"rollback_rc=0"* ]] || fail "systemd_datapath_apply_failure: missing rollback_rc=0 output=$output"
  [[ "$output" != *"start_status=ok"* ]] || fail "systemd_datapath_apply_failure: false ok status leaked"
  [[ -f "$rollback_marker" ]] || fail "systemd_datapath_apply_failure: rollback recover was not invoked after partial apply state"
  [[ ! -f "$state_file" ]] || fail "systemd_datapath_apply_failure: partial runtime state was not removed by rollback"
  [[ ! -f "$tun_marker" ]] || fail "systemd_datapath_apply_failure: stale tun marker was not removed after apply failure"
  grep -q '^cli up ' "$runner_log" || fail "systemd_datapath_apply_failure: fake cli up was not called"
  grep -q '^cli rollback recover ' "$runner_log" || fail "systemd_datapath_apply_failure: fake rollback recover was not called"
  grep -q '^--user stop chimera-datapath.service chimera-node.service$' "$systemctl_log" || fail "systemd_datapath_apply_failure: node/datapath units were not stopped after apply failure"
  grep -q '^link show dev chimera0$' "$ip_log" || fail "systemd_datapath_apply_failure: stale tun check did not run"
  grep -q '^link delete dev chimera0$' "$ip_log" || fail "systemd_datapath_apply_failure: stale tun cleanup did not run"

  rm -rf "$tmp_dir"
}

run_systemd_apply_failure_case

run_systemd_cli_privilege_case() {
  local tmp_dir bin_dir cache_dir config_dir runtime_dir install_root node_conf output rc fake_runner fake_systemctl fake_sudo fake_rm runner_log sudo_log rm_log state_file env_file

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  install_root="$tmp_dir/chimera-release"
  node_conf="$tmp_dir/mesh-node.conf"
  fake_runner="$tmp_dir/fake-runner.sh"
  fake_systemctl="$bin_dir/systemctl"
  fake_sudo="$bin_dir/sudo"
  fake_rm="$bin_dir/rm"
  runner_log="$tmp_dir/runner.log"
  sudo_log="$tmp_dir/sudo.log"
  rm_log="$tmp_dir/rm.log"
  state_file="$tmp_dir/runtime_state.json"
  env_file="$config_dir/chimera/transparent-runtime.env"

  mkdir -p "$bin_dir" "$cache_dir" "$config_dir/chimera" "$runtime_dir" "$install_root/scripts" "$install_root/configs"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cp "$ROOT_DIR/configs/mesh-node.example.conf" "$install_root/configs/mesh-node.example.conf"
  cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = carrier.mesh:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF

  cat >"$fake_systemctl" <<EOF
#!/usr/bin/env bash
set -euo pipefail
case "\${1:-}" in
  --user) shift ;;
esac
case "\${1:-}" in
  show-environment|daemon-reload|start)
    exit 0
    ;;
  is-active)
    echo "active"
    exit 0
    ;;
  stop|list-units|list-unit-files)
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
EOF
  chmod +x "$fake_systemctl"

  cat >"$fake_runner" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >>"$runner_log"
case "\${1:-}" in
  cli)
    if [[ "\${2:-}" == "up" ]]; then
      state_file=""
      prev=""
      for arg in "\$@"; do
        if [[ "\$prev" == "--state-file" ]]; then
          state_file="\$arg"
          break
        fi
        prev="\$arg"
      done
      if [[ -n "\$state_file" ]]; then
        mkdir -p "\$(dirname "\$state_file")"
        printf '%s\n' '{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true,"tun_device":"chimera-test","route_cidrs_applied":"10.0.0.0/8"}' >"\$state_file"
      fi
      exit 0
    fi
    if [[ "\${2:-}" == "state" && "\${3:-}" == "proof" ]]; then
      echo "datapath_proof=ok"
      exit 0
    fi
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
EOF
  chmod +x "$fake_runner"

  cat >"$fake_sudo" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf 'sudo %s\n' "\$*" >>"$sudo_log"
if [[ "\${1:-}" == "-n" ]]; then
  shift
fi
CHIMERA_FAKE_SUDO_CLEAN=1 exec "\$@"
EOF
  chmod +x "$fake_sudo"

  cat >"$fake_rm" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf 'rm %s\n' "\$*" >>"$rm_log"
if [[ "\${CHIMERA_FAKE_SUDO_CLEAN:-0}" != "1" ]]; then
  exit 1
fi
exec /bin/rm "\$@"
EOF
  chmod +x "$fake_rm"

  cat >"$env_file" <<'EOF'
CHIMERA_RUNNER_USE_SUDO=1
CHIMERA_NFT_PRIVILEGE_MODE=sudo
EOF
  printf '%s\n' '{"status":"old"}' >"$state_file"

  set +e
  output="$(
    PATH="$bin_dir:$PATH" \
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$cache_dir" \
    XDG_CONFIG_HOME="$config_dir" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    NODE_CONFIG_FILE="$node_conf" \
    STATE_FILE="$state_file" \
    CHIMERA_RUNNER="$fake_runner" \
    CHIMERA_UPDATE_FIRST_CHECKED=1 \
    SITE_AUTOWATCH_ENABLED=0 \
    timeout 20s bash "$install_root/scripts/chimera-control.sh" start 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "systemd_cli_privileged_up_path: expected rc=0 got $rc output=$output"
  [[ "$output" == *"start_status=ok"* ]] || fail "systemd_cli_privileged_up_path: missing ok status output=$output"
  [[ "$output" == *"datapath_apply=ok"* ]] || fail "systemd_cli_privileged_up_path: missing datapath_apply=ok output=$output"
  [[ "$output" == *"datapath_proof=ok"* ]] || fail "systemd_cli_privileged_up_path: missing datapath_proof=ok output=$output"
  grep -q '^sudo -n env .*CHIMERA_RUNNER_USE_SUDO=1 .* cli up ' "$sudo_log" || fail "systemd_cli_privileged_up_path: cli up did not use sudo env wrapper"
  grep -q "^sudo -n rm -f $state_file\$" "$sudo_log" || fail "systemd_cli_privileged_up_path: state cleanup did not use sudo"
  grep -q "^rm -f $state_file\$" "$rm_log" || fail "systemd_cli_privileged_up_path: rm wrapper did not see state cleanup"
  grep -q '^cli up ' "$runner_log" || fail "systemd_cli_privileged_up_path: fake cli up was not called"
  grep -q '^cli state proof ' "$runner_log" || fail "systemd_cli_privileged_up_path: fake state proof was not called"

  rm -rf "$tmp_dir"
}

run_systemd_cli_privilege_case

run_systemd_stale_tun_preflight_cleanup_case() {
  local tmp_dir bin_dir cache_dir config_dir runtime_dir install_root node_conf output rc fake_runner fake_systemctl fake_sudo fake_rm fake_ip runner_log sudo_log rm_log ip_log state_file env_file tun_marker

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  install_root="$tmp_dir/chimera-release"
  node_conf="$tmp_dir/mesh-node.conf"
  fake_runner="$tmp_dir/fake-runner.sh"
  fake_systemctl="$bin_dir/systemctl"
  fake_sudo="$bin_dir/sudo"
  fake_rm="$bin_dir/rm"
  fake_ip="$bin_dir/ip"
  runner_log="$tmp_dir/runner.log"
  sudo_log="$tmp_dir/sudo.log"
  rm_log="$tmp_dir/rm.log"
  ip_log="$tmp_dir/ip.log"
  state_file="$tmp_dir/runtime_state.json"
  env_file="$config_dir/chimera/transparent-runtime.env"
  tun_marker="$tmp_dir/chimera0.present"

  mkdir -p "$bin_dir" "$cache_dir" "$config_dir/chimera" "$runtime_dir" "$install_root/scripts" "$install_root/configs"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cp "$ROOT_DIR/configs/mesh-node.example.conf" "$install_root/configs/mesh-node.example.conf"
  cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = carrier.mesh:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF

  cat >"$fake_systemctl" <<EOF
#!/usr/bin/env bash
set -euo pipefail
case "\${1:-}" in
  --user) shift ;;
esac
case "\${1:-}" in
  show-environment|daemon-reload|start)
    exit 0
    ;;
  is-active)
    echo "active"
    exit 0
    ;;
  stop|list-units|list-unit-files)
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
EOF
  chmod +x "$fake_systemctl"

  cat >"$fake_runner" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >>"$runner_log"
case "\${1:-}" in
  cli)
    if [[ "\${2:-}" == "up" ]]; then
      state_file=""
      prev=""
      for arg in "\$@"; do
        if [[ "\$prev" == "--state-file" ]]; then
          state_file="\$arg"
          break
        fi
        prev="\$arg"
      done
      if [[ -n "\$state_file" ]]; then
        mkdir -p "\$(dirname "\$state_file")"
        printf '%s\n' '{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true,"tun_device":"chimera0","route_cidrs_applied":"10.0.0.0/8"}' >"\$state_file"
      fi
      exit 0
    fi
    if [[ "\${2:-}" == "state" && "\${3:-}" == "proof" ]]; then
      echo "datapath_proof=ok"
      exit 0
    fi
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
EOF
  chmod +x "$fake_runner"

  cat >"$fake_sudo" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf 'sudo %s\n' "\$*" >>"$sudo_log"
if [[ "\${1:-}" == "-n" ]]; then
  shift
fi
CHIMERA_FAKE_SUDO_CLEAN=1 exec "\$@"
EOF
  chmod +x "$fake_sudo"

  cat >"$fake_rm" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf 'rm %s\n' "\$*" >>"$rm_log"
if [[ "\${CHIMERA_FAKE_SUDO_CLEAN:-0}" != "1" ]]; then
  exit 1
fi
exec /bin/rm "\$@"
EOF
  chmod +x "$fake_rm"

  cat >"$fake_ip" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >>"$ip_log"
if [[ "\${1:-}" == "link" && "\${2:-}" == "show" && "\${3:-}" == "dev" && "\${4:-}" == "chimera0" ]]; then
  [[ -f "$tun_marker" ]] && exit 0
  exit 1
fi
if [[ "\${1:-}" == "link" && "\${2:-}" == "delete" && "\${3:-}" == "dev" && "\${4:-}" == "chimera0" ]]; then
  rm -f "$tun_marker"
  exit 0
fi
exit 0
EOF
  chmod +x "$fake_ip"

  cat >"$env_file" <<'EOF'
CHIMERA_RUNNER_USE_SUDO=1
CHIMERA_NFT_PRIVILEGE_MODE=sudo
EOF
  printf '%s\n' present >"$tun_marker"

  set +e
  output="$(
    PATH="$bin_dir:$PATH" \
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$cache_dir" \
    XDG_CONFIG_HOME="$config_dir" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    NODE_CONFIG_FILE="$node_conf" \
    STATE_FILE="$state_file" \
    CHIMERA_RUNNER="$fake_runner" \
    CHIMERA_UPDATE_FIRST_CHECKED=1 \
    SITE_AUTOWATCH_ENABLED=0 \
    timeout 20s bash "$install_root/scripts/chimera-control.sh" start 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "systemd_stale_tun_preflight_cleanup: expected rc=0 got $rc output=$output"
  [[ "$output" == *"start_status=ok"* ]] || fail "systemd_stale_tun_preflight_cleanup: missing ok status output=$output"
  [[ "$output" == *"datapath_apply=ok"* ]] || fail "systemd_stale_tun_preflight_cleanup: missing datapath_apply=ok output=$output"
  [[ "$output" == *"datapath_proof=ok"* ]] || fail "systemd_stale_tun_preflight_cleanup: missing datapath_proof=ok output=$output"
  [[ ! -f "$tun_marker" ]] || fail "systemd_stale_tun_preflight_cleanup: stale tun marker was not removed before apply"
  grep -q '^sudo -n ip link show dev chimera0$' "$sudo_log" || fail "systemd_stale_tun_preflight_cleanup: stale tun show did not use sudo"
  grep -q '^sudo -n ip link delete dev chimera0$' "$sudo_log" || fail "systemd_stale_tun_preflight_cleanup: stale tun delete did not use sudo"
  grep -q '^sudo -n rm -f '"$state_file"'$' "$sudo_log" || fail "systemd_stale_tun_preflight_cleanup: state cleanup did not use sudo"
  grep -q '^cli up ' "$runner_log" || fail "systemd_stale_tun_preflight_cleanup: fake cli up was not called"
  grep -q '^cli state proof ' "$runner_log" || fail "systemd_stale_tun_preflight_cleanup: fake state proof was not called"
  grep -q '^link show dev chimera0$' "$ip_log" || fail "systemd_stale_tun_preflight_cleanup: ip show did not run"
  grep -q '^link delete dev chimera0$' "$ip_log" || fail "systemd_stale_tun_preflight_cleanup: ip delete did not run"

  rm -rf "$tmp_dir"
}

run_systemd_stale_tun_preflight_cleanup_case

run_systemd_listener_only_self_loop_case() {
  local tmp_dir bin_dir cache_dir config_dir runtime_dir install_root node_conf output rc fake_systemctl fake_ip fake_hostname systemctl_log

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  install_root="$tmp_dir/chimera-release"
  node_conf="$tmp_dir/mesh-node.conf"
  fake_systemctl="$bin_dir/systemctl"
  fake_ip="$bin_dir/ip"
  fake_hostname="$bin_dir/hostname"
  systemctl_log="$tmp_dir/systemctl.log"

  mkdir -p "$bin_dir" "$cache_dir" "$config_dir" "$runtime_dir" "$install_root/scripts" "$install_root/configs"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cp "$ROOT_DIR/configs/mesh-node.example.conf" "$install_root/configs/mesh-node.example.conf"
  cat >"$node_conf" <<'EOF'
carrier.profile = tls
carrier.addr = tcp://10.10.10.10:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF

  cat >"$fake_systemctl" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >>"$systemctl_log"
case "\${1:-}" in
  --user) shift ;;
esac
case "\${1:-}" in
  show-environment|daemon-reload|start)
    exit 0
    ;;
  is-active)
    echo "active"
    exit 0
    ;;
  stop|list-units|list-unit-files)
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
EOF
  chmod +x "$fake_systemctl"

  cat >"$fake_ip" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  link)
    exit 1
    ;;
  -o)
    if [[ "${2:-}" == "addr" ]]; then
      printf '%s\n' '2: eth0    inet 10.10.10.10/24 brd 10.10.10.255 scope global eth0'
      exit 0
    fi
    ;;
  route)
    printf '%s\n' '1.1.1.1 via 10.10.10.1 dev eth0 src 10.10.10.10 uid 1000'
    exit 0
    ;;
esac
exit 0
EOF
  chmod +x "$fake_ip"

  cat >"$fake_hostname" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "-I" ]]; then
  printf '%s\n' '10.10.10.10'
  exit 0
fi
exit 0
EOF
  chmod +x "$fake_hostname"

  set +e
  output="$(
    PATH="$bin_dir:$PATH" \
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$cache_dir" \
    XDG_CONFIG_HOME="$config_dir" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    NODE_CONFIG_FILE="$node_conf" \
    CHIMERA_UPDATE_FIRST_CHECKED=1 \
    SITE_AUTOWATCH_ENABLED=0 \
    timeout 20s bash "$install_root/scripts/chimera-control.sh" start 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "systemd_listener_only_self_loop: expected rc=0 got $rc output=$output"
  [[ "$output" == *"start_status=partial"* ]] || fail "systemd_listener_only_self_loop: missing partial status output=$output"
  [[ "$output" == *"mode=listener_only"* ]] || fail "systemd_listener_only_self_loop: missing listener_only mode output=$output"
  [[ "$output" == *"mesh_ready=false"* ]] || fail "systemd_listener_only_self_loop: missing mesh_ready=false output=$output"
  [[ "$output" == *"reason=self_loop_listener_only"* ]] || fail "systemd_listener_only_self_loop: missing self_loop_listener_only reason output=$output"
  [[ "$output" == *"transparent_runtime=skipped"* ]] || fail "systemd_listener_only_self_loop: missing transparent_runtime=skipped output=$output"
  [[ "$output" == *"datapath_apply=skipped"* ]] || fail "systemd_listener_only_self_loop: missing datapath_apply=skipped output=$output"
  grep -q '^--user start chimera-node.service$' "$systemctl_log" || fail "systemd_listener_only_self_loop: node service was not started"
  ! grep -q '^--user start chimera-datapath.service$' "$systemctl_log" || fail "systemd_listener_only_self_loop: datapath service should not start in listener_only mode"

  rm -rf "$tmp_dir"
}

run_systemd_listener_only_self_loop_case

run_systemd_apply_state_proof_case() {
  local case_name="$1"
  local state_payload="$2"
  local expected_rc="$3"
  local expected_status="$4"
  local expected_proof="$5"
  local expect_rollback="$6"
  local tmp_dir bin_dir cache_dir config_dir runtime_dir install_root node_conf output rc fake_runner fake_systemctl runner_log rollback_marker systemctl_log state_file

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  install_root="$tmp_dir/chimera-release"
  node_conf="$tmp_dir/mesh-node.conf"
  fake_runner="$tmp_dir/fake-runner.sh"
  fake_systemctl="$bin_dir/systemctl"
  runner_log="$tmp_dir/runner.log"
  rollback_marker="$tmp_dir/rollback_invoked"
  systemctl_log="$tmp_dir/systemctl.log"
  state_file="$tmp_dir/runtime_state.json"

  mkdir -p "$bin_dir" "$cache_dir" "$config_dir" "$runtime_dir" "$install_root/scripts" "$install_root/configs"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cp "$ROOT_DIR/configs/mesh-node.example.conf" "$install_root/configs/mesh-node.example.conf"
  cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = carrier.mesh:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF

  cat >"$fake_systemctl" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >>"$systemctl_log"
case "\${1:-}" in
  --user) shift ;;
esac
case "\${1:-}" in
  show-environment|daemon-reload|start)
    exit 0
    ;;
  is-active)
    echo "active"
    exit 0
    ;;
  stop|list-units|list-unit-files)
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
EOF
  chmod +x "$fake_systemctl"

  cat >"$fake_runner" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >>"$runner_log"
case "\${1:-}" in
  cli)
    if [[ "\${2:-}" == "up" ]]; then
      state_file=""
      prev=""
      for arg in "\$@"; do
        if [[ "\$prev" == "--state-file" ]]; then
          state_file="\$arg"
          break
        fi
        prev="\$arg"
      done
      if [[ -n "\$state_file" && -n "$state_payload" ]]; then
        mkdir -p "\$(dirname "\$state_file")"
        printf '%s\n' '$state_payload' >"\$state_file"
      fi
      exit 0
    fi
    if [[ "\${2:-}" == "state" && "\${3:-}" == "proof" ]]; then
      state_file=""
      prev=""
      for arg in "\$@"; do
        if [[ "\$prev" == "--state-file" ]]; then
          state_file="\$arg"
          break
        fi
        prev="\$arg"
      done
      if [[ -z "\$state_file" || ! -f "\$state_file" ]]; then
        echo "datapath_proof=missing_state"
        exit 1
      fi
      if grep -q '"network_state":"not_modified"' "\$state_file"; then
        echo "datapath_proof=network_not_modified"
        exit 1
      fi
      if grep -q '"status":"up"' "\$state_file" \
        && grep -q '"network_state":"modified"' "\$state_file" \
        && grep -q '"rollback_ready":true' "\$state_file" \
        && grep -q '"tun_applied":true' "\$state_file" \
        && grep -q '"route_applied":true' "\$state_file" \
        && grep -q '"dns_applied":true' "\$state_file"; then
        echo "datapath_proof=ok"
        exit 0
      fi
      echo "datapath_proof=proof_command_failed"
      exit 1
    fi
    if [[ "\${2:-}" == "rollback" && "\${3:-}" == "recover" ]]; then
      state_file=""
      prev=""
      for arg in "\$@"; do
        if [[ "\$prev" == "--state-file" ]]; then
          state_file="\$arg"
          break
        fi
        prev="\$arg"
      done
      printf '%s\n' invoked >"$rollback_marker"
      [[ -n "\$state_file" ]] && rm -f "\$state_file"
      exit 0
    fi
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
EOF
  chmod +x "$fake_runner"

  set +e
  output="$(
    PATH="$bin_dir:$PATH" \
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$cache_dir" \
    XDG_CONFIG_HOME="$config_dir" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    NODE_CONFIG_FILE="$node_conf" \
    STATE_FILE="$state_file" \
    CHIMERA_RUNNER="$fake_runner" \
    CHIMERA_UPDATE_FIRST_CHECKED=1 \
    SITE_AUTOWATCH_ENABLED=0 \
    timeout 20s bash "$install_root/scripts/chimera-control.sh" start 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -ne 124 ]] || fail "$case_name: control start timed out before contract result"
  [[ "$rc" -eq "$expected_rc" ]] || fail "$case_name: expected rc=$expected_rc got rc=$rc output=$output"
  [[ "$output" == *"start_status=$expected_status"* ]] || fail "$case_name: missing start_status=$expected_status output=$output"
  [[ "$output" == *"mode=systemd_user"* ]] || fail "$case_name: missing systemd mode output=$output"
  [[ "$output" == *"datapath_proof=$expected_proof"* ]] || fail "$case_name: missing datapath_proof=$expected_proof output=$output"
  if [[ "$expected_status" == "fail" ]]; then
    [[ "$output" == *"datapath_apply=unverified"* ]] || fail "$case_name: missing datapath_apply=unverified output=$output"
    [[ "$output" == *"reason=datapath_proof_failed"* ]] || fail "$case_name: missing datapath_proof_failed output=$output"
    [[ "$output" != *"start_status=ok"* ]] || fail "$case_name: false ok status leaked"
  else
    [[ "$output" == *"datapath_apply=ok"* ]] || fail "$case_name: missing datapath_apply=ok output=$output"
  fi
  if [[ "$expect_rollback" == "1" ]]; then
    [[ -f "$rollback_marker" ]] || fail "$case_name: rollback recover was not invoked"
    grep -q '^cli rollback recover ' "$runner_log" || fail "$case_name: fake rollback recover was not called"
    grep -q '^--user stop chimera-datapath.service chimera-node.service$' "$systemctl_log" || fail "$case_name: node/datapath units were not stopped"
  else
    [[ ! -f "$rollback_marker" ]] || fail "$case_name: rollback was invoked for valid state"
    ! grep -q '^cli rollback recover ' "$runner_log" 2>/dev/null || fail "$case_name: fake rollback recover was called for valid state"
  fi
  grep -q '^cli up ' "$runner_log" || fail "$case_name: fake cli up was not called"

  rm -rf "$tmp_dir"
}

run_systemd_apply_state_proof_case \
  "systemd_apply_rc0_state_missing" \
  "" \
  1 \
  "fail" \
  "missing_state" \
  1

run_systemd_apply_state_proof_case \
  "systemd_apply_rc0_network_not_modified" \
  '{"status":"up","network_state":"not_modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true}' \
  1 \
  "fail" \
  "network_not_modified" \
  1

run_systemd_apply_state_proof_case \
  "systemd_apply_rc0_valid_state_allows_ok" \
  '{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true}' \
  0 \
  "ok" \
  "ok" \
  0

run_route_status_case() {
  local case_name="${1:?case_name_required}"
  local state_payload="${2:-}"
  local flow_payload="${3:-}"
  local flow_touch_mode="${4:-}"
  local expected_mode="${5:?expected_mode_required}"
  local expected_apply="${6:?expected_apply_required}"
  local expected_proof="${7:?expected_proof_required}"
  local expected_flow_proof="${8:?expected_flow_proof_required}"
  local tmp_dir bin_dir output rc fake_runner state_file flow_file

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  fake_runner="$tmp_dir/fake-runner.sh"
  state_file="$tmp_dir/runtime_state.json"
  flow_file="${state_file}.flow.json"
  mkdir -p "$bin_dir" "$tmp_dir/config" "$tmp_dir/cache" "$tmp_dir/runtime"
  cat >"$bin_dir/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  --user) shift ;;
esac
case "${1:-}" in
  show-environment) exit 1 ;;
  *) exit 0 ;;
esac
EOF
  chmod +x "$bin_dir/systemctl"
  cat >"$fake_runner" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  cli)
    if [[ "${2:-}" == "state" && "${3:-}" == "proof" ]]; then
      state_file=""
      flow_file=""
      require_flow="false"
      max_flow_age_sec="300"
      prev=""
      for arg in "$@"; do
        if [[ "$prev" == "--state-file" ]]; then
          state_file="$arg"
        elif [[ "$prev" == "--flow-file" ]]; then
          flow_file="$arg"
        elif [[ "$prev" == "--require-flow" ]]; then
          require_flow="$arg"
        elif [[ "$prev" == "--max-flow-age-sec" ]]; then
          max_flow_age_sec="$arg"
        fi
        prev="$arg"
      done
      if [[ -z "$state_file" || ! -f "$state_file" ]]; then
        echo "datapath_proof=missing_state"
        exit 1
      fi
      if grep -q '"network_state":"not_modified"' "$state_file"; then
        echo "datapath_proof=network_not_modified"
        exit 1
      fi
      if grep -q '"status":"up"' "$state_file" \
        && grep -q '"network_state":"modified"' "$state_file" \
        && grep -q '"rollback_ready":true' "$state_file" \
        && grep -q '"tun_applied":true' "$state_file" \
        && grep -q '"route_applied":true' "$state_file" \
        && grep -q '"dns_applied":true' "$state_file"; then
        if [[ "$require_flow" == "true" ]]; then
          [[ -n "$flow_file" ]] || flow_file="${state_file}.flow.json"
          if [[ ! -f "$flow_file" ]]; then
            echo "datapath_proof=missing_flow_proof"
            exit 1
          fi
          if [[ "${FLOW_TOUCH_MODE:-}" == "stale" ]]; then
            echo "datapath_proof=flow_stale"
            exit 1
          fi
          if [[ "${FLOW_PAYLOAD_KIND:-valid}" == "invalid_json" ]]; then
            echo "datapath_proof=flow_invalid_json"
            exit 1
          fi
        fi
        echo "datapath_proof=ok"
        exit 0
      fi
      echo "datapath_proof=proof_command_failed"
      exit 1
    fi
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
EOF
  chmod +x "$fake_runner"

  if [[ -n "$state_payload" ]]; then
    printf '%s\n' "$state_payload" >"$state_file"
  fi
  if [[ -n "$flow_payload" ]]; then
    printf '%s\n' "$flow_payload" >"$flow_file"
  fi

  set +e
  output="$(
    PATH="$bin_dir:$PATH" \
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$tmp_dir/cache" \
    XDG_CONFIG_HOME="$tmp_dir/config" \
    XDG_RUNTIME_DIR="$tmp_dir/runtime" \
    STATE_FILE="$state_file" \
    FLOW_PAYLOAD_KIND="$([[ "$flow_payload" == "{not json" ]] && echo invalid_json || echo valid)" \
    FLOW_TOUCH_MODE="$flow_touch_mode" \
    APP_ROUTES_FILE="$tmp_dir/app-routes.conf" \
    MANUAL_TRANSIT_DOMAINS_FILE="$tmp_dir/manual-transit.txt" \
    ADAPTIVE_DOMAINS_FILE="$tmp_dir/adaptive.txt" \
    CHIMERA_RUNNER="$fake_runner" \
    bash "$ROOT_DIR/scripts/chimera-control.sh" route-status 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "$case_name: expected rc=0 got rc=$rc output=$output"
  [[ "$output" == *"datapath_mode=$expected_mode"* ]] || fail "$case_name: missing datapath_mode=$expected_mode output=$output"
  [[ "$output" == *"datapath_apply=$expected_apply"* ]] || fail "$case_name: missing datapath_apply=$expected_apply output=$output"
  [[ "$output" == *"datapath_proof=$expected_proof"* ]] || fail "$case_name: missing datapath_proof=$expected_proof output=$output"
  [[ "$output" == *"datapath_flow_proof=$expected_flow_proof"* ]] || fail "$case_name: missing datapath_flow_proof=$expected_flow_proof output=$output"
  if [[ "$expected_mode" != "transparent" ]]; then
    [[ "$output" != *"datapath_mode=transparent"* ]] || fail "$case_name: false transparent datapath leaked"
  fi
  if [[ "$expected_apply" != "ok" ]]; then
    [[ "$output" != *"datapath_apply=ok"* ]] || fail "$case_name: false ok apply leaked"
  fi

  rm -rf "$tmp_dir"
}

valid_state_payload='{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true}'
valid_flow_payload='{"status":"ok","kind":"chimera_datapath_flow_proof","flow_id":"flow#1","path_kind":"local_egress_via_secure_peer","transparent_flow_observed":true,"counter_delta_ok":true,"secure_peer_egress_observed":true,"secure_peer_bytes_delta_ok":true,"network_state":"modified"}'

run_route_status_case "route_status_without_proof" "" "" "" "unknown" "unverified" "missing_state" "skipped_apply_unverified"
run_route_status_case "route_status_valid_apply_without_flow_proof" "$valid_state_payload" "" "" "unknown" "ok" "ok" "missing_flow_proof"
run_route_status_case "route_status_invalid_flow_proof" "$valid_state_payload" "{not json" "" "unknown" "ok" "ok" "flow_invalid_json"
run_route_status_case "route_status_stale_flow_proof" "$valid_state_payload" "$valid_flow_payload" "stale" "unknown" "ok" "ok" "flow_stale"
run_route_status_case "route_status_valid_flow_proof" "$valid_state_payload" "$valid_flow_payload" "" "transparent" "ok" "ok" "ok"

run_direct_apply_failure_case() {
  local tmp_dir bin_dir cache_dir config_dir runtime_dir install_root node_conf output rc fake_runner fake_systemctl runner_log rollback_marker node_env datapath_env node_pid datapath_pid state_file

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  install_root="$tmp_dir/chimera-release"
  node_conf="$tmp_dir/mesh-node.conf"
  fake_runner="$tmp_dir/fake-runner.sh"
  fake_systemctl="$bin_dir/systemctl"
  runner_log="$tmp_dir/runner.log"
  rollback_marker="$tmp_dir/rollback_invoked"
  node_env="$config_dir/chimera/peer-egress.env"
  datapath_env="$config_dir/chimera/transparent-runtime.env"
  node_pid="$runtime_dir/chimera-peer-egress.pid"
  datapath_pid="$runtime_dir/chimera-transparent-runtime.pid"
  state_file="$tmp_dir/runtime_state.json"

  mkdir -p "$bin_dir" "$cache_dir" "$config_dir/chimera" "$runtime_dir" "$install_root/scripts" "$install_root/configs"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cp "$ROOT_DIR/configs/mesh-node.example.conf" "$install_root/configs/mesh-node.example.conf"
  cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = carrier.mesh:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
  printf '%s\n' 'CHIMERA_PEER_EGRESS_MODE=listen' >"$node_env"
  printf '%s\n' 'CHIMERA_TRANSPARENT_RUNTIME_MODE=listen' >"$datapath_env"

  cat >"$fake_systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  --user) shift ;;
esac
case "${1:-}" in
  show-environment) exit 1 ;;
  *) exit 0 ;;
esac
EOF
  chmod +x "$fake_systemctl"

  cat >"$fake_runner" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >>"$runner_log"
case "\${1:-}" in
  cli)
    if [[ "\${2:-}" == "up" ]]; then
      for _ in {1..20}; do
        if grep -q '^peer-egress$' "$runner_log" 2>/dev/null && grep -q '^transparent-runtime$' "$runner_log" 2>/dev/null; then
          break
        fi
        sleep 0.05
      done
      state_file=""
      prev=""
      for arg in "\$@"; do
        if [[ "\$prev" == "--state-file" ]]; then
          state_file="\$arg"
          break
        fi
        prev="\$arg"
      done
      if [[ -n "\$state_file" ]]; then
        mkdir -p "\$(dirname "\$state_file")"
        printf '%s\n' '{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":false,"tun_device":"chimera-test","route_cidrs_applied":"10.0.0.0/8"}' >"\$state_file"
      fi
      exit 42
    fi
    if [[ "\${2:-}" == "rollback" && "\${3:-}" == "recover" ]]; then
      state_file=""
      prev=""
      for arg in "\$@"; do
        if [[ "\$prev" == "--state-file" ]]; then
          state_file="\$arg"
          break
        fi
        prev="\$arg"
      done
      printf '%s\n' invoked >"$rollback_marker"
      [[ -n "\$state_file" ]] && rm -f "\$state_file"
      exit 0
    fi
    exit 0
    ;;
  peer-egress|transparent-runtime)
    sleep 60
    ;;
  *)
    exit 2
    ;;
esac
EOF
  chmod +x "$fake_runner"

  set +e
  output="$(
    PATH="$bin_dir:$PATH" \
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$cache_dir" \
    XDG_CONFIG_HOME="$config_dir" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    NODE_CONFIG_FILE="$node_conf" \
    STATE_FILE="$state_file" \
    CHIMERA_RUNNER="$fake_runner" \
    CHIMERA_UPDATE_FIRST_CHECKED=1 \
    SITE_AUTOWATCH_ENABLED=0 \
    timeout 20s bash "$install_root/scripts/chimera-control.sh" start 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -ne 124 ]] || fail "direct_datapath_apply_failure: control start timed out before contract result"
  [[ "$rc" -ne 0 ]] || fail "direct_datapath_apply_failure: expected non-zero exit"
  [[ "$output" == *"start_status=fail"* ]] || fail "direct_datapath_apply_failure: missing fail status output=$output"
  [[ "$output" == *"reason=datapath_apply_failed"* ]] || fail "direct_datapath_apply_failure: missing datapath_apply_failed output=$output"
  [[ "$output" == *"datapath_apply=failed"* ]] || fail "direct_datapath_apply_failure: missing datapath_apply=failed output=$output"
  [[ "$output" == *"apply_rc=42"* ]] || fail "direct_datapath_apply_failure: missing apply_rc=42 output=$output"
  [[ "$output" == *"datapath_rollback=ok"* ]] || fail "direct_datapath_apply_failure: missing datapath_rollback=ok output=$output"
  [[ "$output" == *"rollback_rc=0"* ]] || fail "direct_datapath_apply_failure: missing rollback_rc=0 output=$output"
  [[ "$output" != *"start_status=ok"* ]] || fail "direct_datapath_apply_failure: false ok status leaked"
  [[ -f "$rollback_marker" ]] || fail "direct_datapath_apply_failure: rollback recover was not invoked after partial apply state"
  [[ ! -f "$state_file" ]] || fail "direct_datapath_apply_failure: partial runtime state was not removed by rollback"
  [[ ! -f "$node_pid" ]] || fail "direct_datapath_apply_failure: node pidfile not cleaned after apply failure"
  [[ ! -f "$datapath_pid" ]] || fail "direct_datapath_apply_failure: datapath pidfile not cleaned after apply failure"
  grep -q '^cli up ' "$runner_log" || fail "direct_datapath_apply_failure: fake cli up was not called"
  grep -q '^cli rollback recover ' "$runner_log" || fail "direct_datapath_apply_failure: fake rollback recover was not called"
  grep -q '^peer-egress$' "$runner_log" || fail "direct_datapath_apply_failure: fake peer runner was not called"
  grep -q '^transparent-runtime$' "$runner_log" || fail "direct_datapath_apply_failure: fake transparent runner was not called"

  rm -rf "$tmp_dir"
}

run_direct_apply_failure_case

run_peer_update_env_write_case() {
  local tmp_dir install_root config_dir cache_dir bootstrap_env peer_env state_file output rc

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  config_dir="$tmp_dir/config/chimera"
  cache_dir="$tmp_dir/cache/chimera"
  mkdir -p "$install_root/scripts" "$config_dir" "$cache_dir"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"

  bootstrap_env="$config_dir/mesh_bootstrap.env"
  peer_env="$config_dir/peer-update.env"
  state_file="$cache_dir/peer-update.state.json"
  cat >"$bootstrap_env" <<'EOF'
CHIMERA_PEER_UPDATE_BASE_URL=http://198.51.100.10
CHIMERA_PEER_UPDATE_LISTEN=0.0.0.0:0
EOF

  set +e
  output="$(
    CHIMERA_BOOTSTRAP_ENV_FILE="$bootstrap_env" \
    PEER_UPDATE_ENV_FILE="$peer_env" \
    PEER_UPDATE_STATE_FILE="$state_file" \
    bash -lc 'source "'"$install_root/scripts/chimera-control.sh"'"; configure_peer_update_env' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "peer_update_env_write_case: configure_peer_update_env failed output=$output"
  [[ -f "$peer_env" ]] || fail "peer_update_env_write_case: peer-update env file missing"
  grep -Fxq 'CHIMERA_PEER_UPDATE_BASE_URL=http://198.51.100.10' "$peer_env" || fail "peer_update_env_write_case: missing base url"
  grep -Fxq 'CHIMERA_PEER_UPDATE_LISTEN=0.0.0.0:0' "$peer_env" || fail "peer_update_env_write_case: missing listen addr"
  grep -Fxq "CHIMERA_PEER_UPDATE_STATE_FILE=$state_file" "$peer_env" || fail "peer_update_env_write_case: missing state file path"

  rm -rf "$tmp_dir"
}

run_peer_update_env_write_case

run_mesh_discovery_default_path_case() {
  local tmp_dir install_root output rc expected_out expected_pub

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  mkdir -p "$install_root/scripts" "$tmp_dir/cache"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"

  expected_out="$tmp_dir/cache/chimera/mesh_nodes.discovery.json"
  expected_pub="$tmp_dir/cache/chimera/mesh_nodes.discovery.pubkey"

  set +e
  output="$(
    XDG_CACHE_HOME="$tmp_dir/cache" \
    bash -lc 'unset MESH_DISCOVERY_OUT_FILE MESH_DISCOVERY_PUBKEY_OUT_FILE; source "'"$install_root/scripts/chimera-control.sh"'"; printf "out=%s\npub=%s\n" "$(mesh_discovery_out_path)" "$(mesh_discovery_pubkey_out_path)"' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "mesh_discovery_default_path_case: path probe failed output=$output"
  [[ "$output" == *"out=$expected_out"* ]] || fail "mesh_discovery_default_path_case: wrong discovery path output=$output"
  [[ "$output" == *"pub=$expected_pub"* ]] || fail "mesh_discovery_default_path_case: wrong pubkey path output=$output"

  rm -rf "$tmp_dir"
}

run_mesh_discovery_default_path_case

echo "chimera_start_contract_smoke=pass"
