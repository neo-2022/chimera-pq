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
  cp "$ROOT_DIR/deploy/systemd-user/chimera-runtime.service" "$install_root/deploy/systemd-user/chimera-runtime.service"
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
CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:18135
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

  [[ "$rc" -eq 2 ]] || fail "systemd_bound_transit_missing_authority_start_progress: expected rc=2 output=$output"
  [[ "$output" == *"start_status=partial"* ]] || fail "systemd_bound_transit_missing_authority_start_progress: missing partial status output=$output"
  [[ "$output" == *"mode=listener_only"* ]] || fail "systemd_bound_transit_missing_authority_start_progress: missing listener_only mode output=$output"
  [[ "$output" == *"reason=node_endpoint_unconfigured_listener_only"* ]] || fail "systemd_bound_transit_missing_authority_start_progress: missing listener_only reason output=$output"
  [[ "$output" == *"fail_closed=true"* ]] || fail "systemd_bound_transit_missing_authority_start_progress: missing fail_closed=true output=$output"
  [[ "$output" == *"node_runtime=stopped"* ]] || fail "systemd_bound_transit_missing_authority_start_progress: missing node_runtime=stopped output=$output"
  [[ "$output" != *"reason=bound_transit_unready"* ]] || fail "systemd_bound_transit_missing_authority_start_progress: bound transit preflight still blocked clean start output=$output"
  grep -q '^--user start chimera-node.service$' "$systemctl_log" || fail "systemd_bound_transit_missing_authority_start_progress: node service was not started"
  grep -q '^--user stop chimera-datapath.service chimera-node.service$' "$systemctl_log" || fail "systemd_bound_transit_missing_authority_start_progress: fail-closed stop was not invoked"
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
CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:18135
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

  [[ "$rc" -eq 2 ]] || fail "systemd_listener_only_unconfigured_endpoint: expected rc=2 got $rc output=$output"
  [[ "$output" == *"start_status=partial"* ]] || fail "systemd_listener_only_unconfigured_endpoint: missing partial status output=$output"
  [[ "$output" == *"mode=listener_only"* ]] || fail "systemd_listener_only_unconfigured_endpoint: missing listener_only mode output=$output"
  [[ "$output" == *"mesh_ready=false"* ]] || fail "systemd_listener_only_unconfigured_endpoint: missing mesh_ready=false output=$output"
  [[ "$output" == *"reason=node_endpoint_unconfigured_listener_only"* ]] || fail "systemd_listener_only_unconfigured_endpoint: missing reason output=$output"
  [[ "$output" == *"fail_closed=true"* ]] || fail "systemd_listener_only_unconfigured_endpoint: missing fail_closed=true output=$output"
  [[ "$output" == *"transparent_runtime=stopped"* ]] || fail "systemd_listener_only_unconfigured_endpoint: missing transparent_runtime=stopped output=$output"
  [[ "$output" == *"node_runtime=stopped"* ]] || fail "systemd_listener_only_unconfigured_endpoint: missing node_runtime=stopped output=$output"
  [[ "$output" == *"datapath_apply=skipped"* ]] || fail "systemd_listener_only_unconfigured_endpoint: missing datapath_apply=skipped output=$output"
  grep -q '^--user start chimera-node.service$' "$systemctl_log" || fail "systemd_listener_only_unconfigured_endpoint: node service was not started"
  grep -q '^--user stop chimera-datapath.service chimera-node.service$' "$systemctl_log" || fail "systemd_listener_only_unconfigured_endpoint: fail-closed stop was not invoked"
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
      if [[ -n "\$state_file" && -f "\$state_file" ]] && grep -q 'rollback_failure_marker' "\$state_file"; then
        exit 1
      fi
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

  [[ "$rc" -eq 2 ]] || fail "systemd_listener_only_self_loop: expected rc=2 got $rc output=$output"
  [[ "$output" == *"start_status=partial"* ]] || fail "systemd_listener_only_self_loop: missing partial status output=$output"
  [[ "$output" == *"mode=listener_only"* ]] || fail "systemd_listener_only_self_loop: missing listener_only mode output=$output"
  [[ "$output" == *"mesh_ready=false"* ]] || fail "systemd_listener_only_self_loop: missing mesh_ready=false output=$output"
  [[ "$output" == *"reason=self_loop_listener_only"* ]] || fail "systemd_listener_only_self_loop: missing self_loop_listener_only reason output=$output"
  [[ "$output" == *"fail_closed=true"* ]] || fail "systemd_listener_only_self_loop: missing fail_closed=true output=$output"
  [[ "$output" == *"transparent_runtime=stopped"* ]] || fail "systemd_listener_only_self_loop: missing transparent_runtime=stopped output=$output"
  [[ "$output" == *"node_runtime=stopped"* ]] || fail "systemd_listener_only_self_loop: missing node_runtime=stopped output=$output"
  [[ "$output" == *"datapath_apply=skipped"* ]] || fail "systemd_listener_only_self_loop: missing datapath_apply=skipped output=$output"
  grep -q '^--user start chimera-node.service$' "$systemctl_log" || fail "systemd_listener_only_self_loop: node service was not started"
  grep -q '^--user stop chimera-datapath.service chimera-node.service$' "$systemctl_log" || fail "systemd_listener_only_self_loop: fail-closed stop was not invoked"
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
      if [[ -n "\$state_file" && -f "\$state_file" ]] && grep -q 'rollback_failure_marker' "\$state_file"; then
        exit 1
      fi
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

run_prestart_saved_state_case() {
  local case_name="${1:?case_name_required}"
  local state_payload="${2:?state_payload_required}"
  local expected_rc="${3:?expected_rc_required}"
  local expected_status="${4:?expected_status_required}"
  local expected_reason="${5-}"
  local expected_recovery="${6:?expected_recovery_required}"
  local expected_proof="${7:?expected_proof_required}"
  local expect_rollback="${8:?expect_rollback_required}"
  local expect_up="${9:?expect_up_required}"
  local tmp_dir bin_dir cache_dir config_dir runtime_dir install_root node_conf output rc fake_runner fake_systemctl runner_log systemctl_log state_file rollback_marker proof_line rollback_line up_line

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
  systemctl_log="$tmp_dir/systemctl.log"
  state_file="$tmp_dir/runtime_state.json"
  rollback_marker="$tmp_dir/rollback.invoked"

  mkdir -p "$bin_dir" "$cache_dir/chimera" "$config_dir/chimera" "$runtime_dir" "$install_root/scripts" "$install_root/configs" "$install_root/docs"
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
  printf '%s\n' "$state_payload" >"$state_file"
  cat >"$config_dir/chimera/transparent-runtime.env" <<'EOF'
CHIMERA_RUNNER_USE_SUDO=0
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
      if grep -q 'duplicate_field_marker' "\$state_file"; then
        echo "datapath_proof=duplicate_field"
        exit 1
      fi
      if grep -q '"network_state":"not_modified"' "\$state_file"; then
        echo "datapath_proof=network_not_modified"
        exit 1
      fi
      if grep -q '"status":"up"' "\$state_file" \\
        && grep -q '"network_state":"modified"' "\$state_file" \\
        && grep -q '"rollback_ready":true' "\$state_file" \\
        && grep -q '"tun_applied":true' "\$state_file" \\
        && grep -q '"route_applied":true' "\$state_file" \\
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
      if [[ -n "\$state_file" && -f "\$state_file" ]] && grep -q 'rollback_failure_marker' "\$state_file"; then
        exit 1
      fi
      [[ -n "\$state_file" ]] && rm -f "\$state_file"
      exit 0
    fi
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
        printf '%s\n' '{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true,"tun_device":"chimera-test","route_cidrs_applied":"10.0.0.0/8"}' >"\$state_file"
      fi
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
  if [[ -n "$expected_reason" ]]; then
    [[ "$output" == *"reason=$expected_reason"* ]] || fail "$case_name: missing reason=$expected_reason output=$output"
  fi
  [[ "$output" == *"recovery_state=$expected_recovery"* ]] || fail "$case_name: missing recovery_state=$expected_recovery output=$output"
  [[ "$output" == *"datapath_proof=$expected_proof"* ]] || fail "$case_name: missing datapath_proof=$expected_proof output=$output"
  if [[ "$expect_rollback" == "1" ]]; then
    [[ -f "$rollback_marker" ]] || fail "$case_name: rollback recover was not invoked"
    grep -q '^cli rollback recover ' "$runner_log" || fail "$case_name: fake rollback recover was not called"
  else
    [[ ! -f "$rollback_marker" ]] || fail "$case_name: rollback unexpectedly invoked"
    ! grep -q '^cli rollback recover ' "$runner_log" 2>/dev/null || fail "$case_name: fake rollback recover was unexpectedly called"
  fi
  if [[ "$expect_up" == "1" ]]; then
    grep -q '^cli up ' "$runner_log" || fail "$case_name: fake cli up was not called"
    proof_line="$(grep -n '^cli state proof ' "$runner_log" | head -n1 | cut -d: -f1)"
    up_line="$(grep -n '^cli up ' "$runner_log" | head -n1 | cut -d: -f1)"
    [[ -n "$proof_line" && -n "$up_line" ]] || fail "$case_name: missing recovery call ordering markers"
    if [[ "$expect_rollback" == "1" ]]; then
      rollback_line="$(grep -n '^cli rollback recover ' "$runner_log" | head -n1 | cut -d: -f1)"
      [[ -n "$rollback_line" ]] || fail "$case_name: missing rollback ordering marker"
      (( proof_line < rollback_line && rollback_line < up_line )) || fail "$case_name: stale state was not recovered before fresh up"
    else
      (( proof_line < up_line )) || fail "$case_name: stale state proof did not run before fresh up"
    fi
  else
    ! grep -q '^cli up ' "$runner_log" 2>/dev/null || fail "$case_name: cli up should not run after invalid saved state"
    ! grep -q '^--user start chimera-node.service$' "$systemctl_log" 2>/dev/null || fail "$case_name: systemd start should not run after invalid saved state"
  fi

  rm -rf "$tmp_dir"
}

run_prestart_saved_state_case \
  "prestart_saved_state_recovery_ok" \
  '{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true}' \
  0 \
  "ok" \
  "" \
  "ok" \
  "ok" \
  1 \
  1

run_prestart_saved_state_case \
  "prestart_saved_state_partial_cleanup_allows_restart" \
  '{"status":"up","network_state":"not_modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true}' \
  0 \
  "ok" \
  "" \
  "ok" \
  "ok" \
  1 \
  1

run_prestart_saved_state_case \
  "prestart_saved_state_duplicate_field_recovers" \
  'duplicate_field_marker' \
  0 \
  "ok" \
  "" \
  "ok" \
  "ok" \
  1 \
  1

run_prestart_saved_state_case \
  "prestart_saved_state_unrecoverable_invalid_blocks_start" \
  'duplicate_field_marker rollback_failure_marker' \
  2 \
  "fail" \
  "saved_state_invalid" \
  "invalid" \
  "duplicate_field" \
  1 \
  0

run_invalid_bootstrap_env_preflight_case() {
  local tmp_dir bin_dir cache_dir config_dir runtime_dir install_root output rc fake_runner fake_systemctl bootstrap_env runner_log systemctl_log

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  install_root="$tmp_dir/chimera-release"
  fake_runner="$tmp_dir/fake-runner.sh"
  fake_systemctl="$bin_dir/systemctl"
  bootstrap_env="$config_dir/chimera/mesh_bootstrap.env"
  runner_log="$tmp_dir/runner.log"
  systemctl_log="$tmp_dir/systemctl.log"

  mkdir -p "$bin_dir" "$cache_dir/chimera" "$config_dir/chimera" "$runtime_dir" "$install_root/scripts"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cat >"$bootstrap_env" <<'EOF'
CHIMERA_MESH_REMOTE_PEER_SPEC=$(touch /tmp/chimera_should_not_run)
EOF

  cat >"$fake_systemctl" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >>"$systemctl_log"
case "\${1:-}" in
  --user) shift ;;
esac
case "\${1:-}" in
  show-environment|daemon-reload|start|stop|list-units|list-unit-files)
    exit 0
    ;;
  is-active)
    echo "active"
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
exit 0
EOF
  chmod +x "$fake_runner"

  set +e
  output="$(
    PATH="$bin_dir:$PATH" \
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$cache_dir" \
    XDG_CONFIG_HOME="$config_dir" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    CHIMERA_RUNNER="$fake_runner" \
    CHIMERA_UPDATE_FIRST_CHECKED=1 \
    SITE_AUTOWATCH_ENABLED=0 \
    timeout 20s bash "$install_root/scripts/chimera-control.sh" start 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 2 ]] || fail "invalid_bootstrap_env_preflight: expected rc=2 got rc=$rc output=$output"
  [[ "$output" == *"start_status=fail"* ]] || fail "invalid_bootstrap_env_preflight: missing fail status output=$output"
  [[ "$output" == *"reason=bootstrap_env_invalid"* ]] || fail "invalid_bootstrap_env_preflight: missing bootstrap_env_invalid output=$output"
  [[ ! -s "$runner_log" ]] || fail "invalid_bootstrap_env_preflight: runner should not execute for invalid bootstrap env"
  ! grep -q '^--user start ' "$systemctl_log" 2>/dev/null || fail "invalid_bootstrap_env_preflight: systemd start should not run for invalid bootstrap env"

  rm -rf "$tmp_dir"
}

run_invalid_bootstrap_env_preflight_case

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
  cat >"$bin_dir/ss" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
cat <<'OUT'
LISTEN 0 128 0.0.0.0:18179 0.0.0.0:*
OUT
EOF
  chmod +x "$bin_dir/ss"

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

run_materialize_node_runtime_config_preserves_existing_case() {
  local tmp_dir install_root node_conf output rc

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  node_conf="$tmp_dir/mesh-node.conf"
  mkdir -p "$install_root/scripts"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"

  cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = old-peer.example:443
carrier.server_name = saved-node.example
capture.mode = auto
capture.tun_supported = true
peer.listen_addr = 9443
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF

  set +e
  output="$(
    NODE_CONFIG_FILE="$node_conf" \
    bash -lc 'source "'"$install_root/scripts/chimera-control.sh"'"; materialize_node_runtime_config "'"$node_conf"'" tcp://new-peer.example:18142 new-peer.example; cat "'"$node_conf"'"' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "materialize_node_runtime_config_preserves_existing_case: materialize failed output=$output"
  grep -Fxq 'carrier.addr = tcp://new-peer.example:18142' "$node_conf" || fail "materialize_node_runtime_config_preserves_existing_case: carrier addr not refreshed output=$output"
  grep -Fxq 'carrier.server_name = saved-node.example' "$node_conf" || fail "materialize_node_runtime_config_preserves_existing_case: existing server name not preserved output=$output"
  grep -Fxq 'peer.listen_addr = 9443' "$node_conf" || fail "materialize_node_runtime_config_preserves_existing_case: existing listen addr not preserved output=$output"

  rm -rf "$tmp_dir"
}

run_materialize_node_runtime_config_preserves_existing_case

run_peer_update_env_preserves_existing_listen_case() {
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
EOF
  cat >"$peer_env" <<'EOF'
# keep-this-comment
CHIMERA_PEER_UPDATE_BASE_URL=http://198.51.100.9
CHIMERA_PEER_UPDATE_LISTEN=0.0.0.0:9443
CHIMERA_EXTRA_OPERATOR_FLAG=keep-me
EOF

  set +e
  output="$(
    CHIMERA_BOOTSTRAP_ENV_FILE="$bootstrap_env" \
    PEER_UPDATE_ENV_FILE="$peer_env" \
    PEER_UPDATE_STATE_FILE="$state_file" \
    bash -lc 'source "'"$install_root/scripts/chimera-control.sh"'"; configure_peer_update_env; cat "'"$peer_env"'"' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "peer_update_env_preserves_existing_listen_case: configure_peer_update_env failed output=$output"
  grep -Fxq '# keep-this-comment' "$peer_env" || fail "peer_update_env_preserves_existing_listen_case: comment was not preserved output=$output"
  grep -Fxq 'CHIMERA_PEER_UPDATE_BASE_URL=http://198.51.100.10' "$peer_env" || fail "peer_update_env_preserves_existing_listen_case: base url not refreshed output=$output"
  grep -Fxq 'CHIMERA_PEER_UPDATE_LISTEN=0.0.0.0:9443' "$peer_env" || fail "peer_update_env_preserves_existing_listen_case: existing listen addr not preserved output=$output"
  grep -Fxq "CHIMERA_PEER_UPDATE_STATE_FILE=$state_file" "$peer_env" || fail "peer_update_env_preserves_existing_listen_case: missing state file path output=$output"
  grep -Fxq 'CHIMERA_EXTRA_OPERATOR_FLAG=keep-me' "$peer_env" || fail "peer_update_env_preserves_existing_listen_case: extra operator flag was not preserved output=$output"

  rm -rf "$tmp_dir"
}

run_peer_update_env_preserves_existing_listen_case

run_peer_update_runtime_restarts_stale_process_without_state_case() {
  local tmp_dir install_root config_dir cache_dir runtime_dir bootstrap_env peer_env state_file pid_file autofix_log fake_runner runner_log output rc stale_pid

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  config_dir="$tmp_dir/config/chimera"
  cache_dir="$tmp_dir/cache/chimera"
  runtime_dir="$tmp_dir/runtime"
  bootstrap_env="$config_dir/mesh_bootstrap.env"
  peer_env="$config_dir/peer-update.env"
  state_file="$cache_dir/peer-update.state.json"
  pid_file="$runtime_dir/chimera-peer-update.pid"
  autofix_log="$cache_dir/autofix.log"
  fake_runner="$tmp_dir/fake-runner.sh"
  runner_log="$tmp_dir/runner.log"

  mkdir -p "$install_root/scripts" "$config_dir" "$cache_dir" "$runtime_dir"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"

  cat >"$bootstrap_env" <<'EOF'
CHIMERA_PEER_UPDATE_BASE_URL=http://198.51.100.10
EOF
  cat >"$peer_env" <<'EOF'
CHIMERA_PEER_UPDATE_BASE_URL=http://198.51.100.9
CHIMERA_PEER_UPDATE_LISTEN=0.0.0.0:0
EOF
  cat >"$fake_runner" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$1" >>"$runner_log"
if [[ "\${1:-}" == "peer-update" ]]; then
  printf '%s\n' '{"kind":"chimera_peer_update_serve_state","status":"ready","listen":"0.0.0.0:45833","base_url":"http://198.51.100.10:45833","update_bootstrap_url":"http://198.51.100.10:45833/chimera.sh","version":"0.1.168","sha256":"test-sha","endpoint_epoch":1,"endpoint_generation":1}' >"$state_file"
  sleep 5
fi
EOF
  chmod +x "$fake_runner"

  bash -lc 'exec -a "fake-runner.sh peer-update" sleep 30' &
  stale_pid=$!
  printf '%s\n' "$stale_pid" >"$pid_file"

  set +e
  output="$(
    CHIMERA_BOOTSTRAP_ENV_FILE="$bootstrap_env" \
    CHIMERA_RUNNER="$fake_runner" \
    PEER_UPDATE_ENV_FILE="$peer_env" \
    PEER_UPDATE_STATE_FILE="$state_file" \
    PEER_UPDATE_PID_FILE="$pid_file" \
    AUTOFIX_LOG_FILE="$autofix_log" \
    XDG_CACHE_HOME="$tmp_dir/cache" \
    XDG_CONFIG_HOME="$tmp_dir/config" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    STALE_PID="$stale_pid" \
    bash -lc 'source "'"$install_root/scripts/chimera-control.sh"'"; start_peer_update_runtime; echo "start_rc=$?"; if kill -0 "$STALE_PID" >/dev/null 2>&1; then echo "stale_pid=alive"; else echo "stale_pid=gone"; fi' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "peer_update_runtime_restarts_stale_process_without_state_case: start failed output=$output"
  [[ "$output" == *"start_rc=0"* ]] || fail "peer_update_runtime_restarts_stale_process_without_state_case: start rc missing output=$output"
  [[ "$output" == *"stale_pid=gone"* ]] || fail "peer_update_runtime_restarts_stale_process_without_state_case: stale process was not restarted output=$output"
  [[ -f "$state_file" ]] || fail "peer_update_runtime_restarts_stale_process_without_state_case: state file missing"
  grep -Fxq 'peer-update' "$runner_log" || fail "peer_update_runtime_restarts_stale_process_without_state_case: fresh peer-update was not started"
  grep -q 'runtime_repair=peer_update_state_restart' "$autofix_log" || fail "peer_update_runtime_restarts_stale_process_without_state_case: autofix log missing stale restart"

  if [[ -f "$pid_file" ]]; then
    kill "$(tr -d '[:space:]' <"$pid_file" 2>/dev/null || true)" >/dev/null 2>&1 || true
  fi
  kill "$stale_pid" >/dev/null 2>&1 || true
  rm -rf "$tmp_dir"
}

run_peer_update_runtime_restarts_stale_process_without_state_case

run_direct_start_skips_datapath_when_node_fails_case() {
  local tmp_dir install_root config_dir cache_dir runtime_dir fake_bin node_conf peer_env datapath_env fake_runner runner_log output rc

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  config_dir="$tmp_dir/config/chimera"
  cache_dir="$tmp_dir/cache/chimera"
  runtime_dir="$tmp_dir/runtime"
  fake_bin="$tmp_dir/bin"
  node_conf="$tmp_dir/mesh-node.conf"
  peer_env="$config_dir/missing-peer-egress.env"
  datapath_env="$config_dir/transparent-runtime.env"
  fake_runner="$tmp_dir/fake-runner.sh"
  runner_log="$tmp_dir/runner.log"

  mkdir -p "$install_root/scripts" "$config_dir" "$cache_dir" "$runtime_dir" "$fake_bin"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"

  cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = carrier.mesh:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
  cat >"$datapath_env" <<'EOF'
CHIMERA_TRANSPARENT_RUNTIME_MODE=listen
EOF
  cat >"$fake_runner" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$1" >>"$runner_log"
exit 0
EOF
  chmod +x "$fake_runner"
  cat >"$fake_bin/systemctl" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
  chmod +x "$fake_bin/systemctl"

  set +e
  output="$(
    PATH="$fake_bin:/usr/bin:/bin" \
    CHIMERA_RUNNER="$fake_runner" \
    NODE_CONFIG_FILE="$node_conf" \
    PEER_EGRESS_ENV_FILE="$peer_env" \
    TRANSPARENT_RUNTIME_ENV_FILE="$datapath_env" \
    XDG_CACHE_HOME="$tmp_dir/cache" \
    XDG_CONFIG_HOME="$tmp_dir/config" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    bash -lc 'source "'"$install_root/scripts/chimera-control.sh"'"; start_runtime' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -ne 0 ]] || fail "direct_start_skips_datapath_when_node_fails_case: expected non-zero rc"
  [[ "$output" == *"reason=node_service_failed"* ]] || fail "direct_start_skips_datapath_when_node_fails_case: missing node failure reason output=$output"
  [[ "$output" == *"transparent_runtime=skipped"* ]] || fail "direct_start_skips_datapath_when_node_fails_case: transparent runtime should stay skipped output=$output"
  [[ ! -f "$runner_log" ]] || ! grep -Fxq 'transparent-runtime' "$runner_log" || fail "direct_start_skips_datapath_when_node_fails_case: transparent runtime started after node failure"
  [[ ! -e "$runtime_dir/chimera-peer-egress.pid" ]] || fail "direct_start_skips_datapath_when_node_fails_case: peer-egress pidfile survived fail-closed cleanup"
  [[ ! -e "$runtime_dir/chimera-transparent-runtime.pid" ]] || fail "direct_start_skips_datapath_when_node_fails_case: transparent pidfile survived fail-closed cleanup"

  rm -rf "$tmp_dir"
}

run_direct_start_skips_datapath_when_node_fails_case

run_node_peer_listen_heal_case() {
  local case_name="${1:?case_name_required}"
  local configured_listen="${2:?configured_listen_required}"
  local initial_peer_listen="${3:?initial_peer_listen_required}"
  local expected_peer_listen="${4:?expected_peer_listen_required}"
  local tmp_dir install_root config_dir node_conf peer_env output rc

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  config_dir="$tmp_dir/config/chimera"
  node_conf="$tmp_dir/mesh-node.conf"
  peer_env="$config_dir/peer-egress.env"

  mkdir -p "$install_root/scripts" "$config_dir"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cat >"$node_conf" <<EOF
carrier.profile = in-memory
carrier.addr = carrier.mesh:443
carrier.server_name = node.local
capture.mode = auto
capture.tun_supported = true
peer.listen_addr = $configured_listen
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
  cat >"$peer_env" <<EOF
CHIMERA_PEER_EGRESS_MODE=node
CHIMERA_PEER_EGRESS_PEER_LISTEN=$initial_peer_listen
EOF

  set +e
  output="$(
    NODE_CONFIG_FILE="$node_conf" \
    PEER_EGRESS_ENV_FILE="$peer_env" \
    bash -lc 'source "'"$install_root/scripts/chimera-control.sh"'"; heal_node_peer_egress_env_bindings; cat "'"$peer_env"'"' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "$case_name: heal failed output=$output"
  grep -Fxq "CHIMERA_PEER_EGRESS_PEER_LISTEN=$expected_peer_listen" "$peer_env" || fail "$case_name: expected peer listen $expected_peer_listen output=$output"

  rm -rf "$tmp_dir"
}

run_node_peer_listen_heal_case \
  "node_peer_listen_heal_auto_rewrites_legacy_fixed_port" \
  "auto" \
  "0.0.0.0:8443" \
  "0.0.0.0:0"

run_node_peer_listen_heal_case \
  "node_peer_listen_heal_preserves_configured_fixed_port" \
  "9443" \
  "0.0.0.0:8443" \
  "0.0.0.0:9443"

run_repair_node_listener_bindings_for_retry_case() {
  local tmp_dir install_root config_dir node_conf peer_env autofix_log override_file bin_dir output rc

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  config_dir="$tmp_dir/config/chimera"
  node_conf="$tmp_dir/mesh-node.conf"
  peer_env="$config_dir/peer-egress.env"
  autofix_log="$tmp_dir/autofix.log"
  override_file="$tmp_dir/runtime_listener_overrides.env"
  bin_dir="$tmp_dir/bin"

  mkdir -p "$install_root/scripts" "$config_dir" "$bin_dir"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = carrier.mesh:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
peer.listen_addr = 9443
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
  cat >"$peer_env" <<'EOF'
CHIMERA_PEER_EGRESS_MODE=node
CHIMERA_PEER_EGRESS_SERVER=carrier.mesh:443
CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:22180
CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:9443
EOF
  cat >"$bin_dir/ss" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
cat <<'OUT'
LISTEN 0 128 0.0.0.0:9443 0.0.0.0:*
LISTEN 0 128 127.0.0.1:22180 0.0.0.0:*
OUT
EOF
  chmod +x "$bin_dir/ss"

  set +e
  output="$(
    PATH="$bin_dir:$PATH" \
    NODE_CONFIG_FILE="$node_conf" \
    PEER_EGRESS_ENV_FILE="$peer_env" \
    RUNTIME_LISTENER_OVERRIDE_FILE="$override_file" \
    AUTOFIX_LOG_FILE="$autofix_log" \
    bash -lc 'source "'"$install_root/scripts/chimera-control.sh"'"; repair_node_listener_bindings_for_retry; printf "retry_rc=%s\n" "$?"; cat "'"$node_conf"'"; cat "'"$peer_env"'"' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "repair_node_listener_bindings_for_retry_case: helper failed output=$output"
  [[ "$output" == *"retry_rc=0"* ]] || fail "repair_node_listener_bindings_for_retry_case: helper did not report repair output=$output"
  grep -q '^peer.listen_addr = 9443$' "$node_conf" || fail "repair_node_listener_bindings_for_retry_case: operator node listen was overwritten"
  grep -q '^CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:22180$' "$peer_env" || fail "repair_node_listener_bindings_for_retry_case: operator local listen was overwritten"
  grep -q '^CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:9443$' "$peer_env" || fail "repair_node_listener_bindings_for_retry_case: operator peer listen was overwritten"
  grep -q '^CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:18135$' "$override_file" || fail "repair_node_listener_bindings_for_retry_case: runtime local listen override missing"
  grep -q '^CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:0$' "$override_file" || fail "repair_node_listener_bindings_for_retry_case: runtime peer listen override missing"
  grep -q 'runtime_repair=node_listener_reset' "$autofix_log" || fail "repair_node_listener_bindings_for_retry_case: autofix log missing node listener reset"

  rm -rf "$tmp_dir"
}

run_repair_node_listener_bindings_for_retry_case

run_node_service_preflight_heals_blocked_fixed_listener_case() {
  local tmp_dir bin_dir cache_dir config_dir runtime_dir install_root node_conf peer_env bootstrap_env autofix_log override_file output rc

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  install_root="$tmp_dir/chimera-release"
  node_conf="$tmp_dir/mesh-node.conf"
  peer_env="$config_dir/chimera/peer-egress.env"
  bootstrap_env="$config_dir/chimera/mesh_bootstrap.env"
  autofix_log="$cache_dir/chimera/autofix.log"
  override_file="$cache_dir/chimera/runtime_listener_overrides.env"

  mkdir -p "$bin_dir" "$cache_dir/chimera" "$config_dir/chimera" "$runtime_dir" "$install_root/scripts" "$install_root/configs"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cp "$ROOT_DIR/configs/mesh-node.example.conf" "$install_root/configs/mesh-node.example.conf"
  cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = old-peer.example:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
peer.listen_addr = 9443
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
  cat >"$peer_env" <<'EOF'
CHIMERA_PEER_EGRESS_MODE=node
CHIMERA_PEER_EGRESS_SERVER=old-peer.example:443
CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:9444
CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:9443
EOF
  cat >"$bootstrap_env" <<'EOF'
CHIMERA_MESH_REMOTE_ENDPOINT=new-peer.example:18142
EOF
  cat >"$bin_dir/ss" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
cat <<'OUT'
LISTEN 0 128 0.0.0.0:9443 0.0.0.0:*
LISTEN 0 128 127.0.0.1:9444 0.0.0.0:*
OUT
EOF
  chmod +x "$bin_dir/ss"

  set +e
  output="$(
    PATH="$bin_dir:$PATH" \
    CHIMERA_BOOTSTRAP_ENV_FILE="$bootstrap_env" \
    NODE_CONFIG_FILE="$node_conf" \
    PEER_EGRESS_ENV_FILE="$peer_env" \
    RUNTIME_LISTENER_OVERRIDE_FILE="$override_file" \
    AUTOFIX_LOG_FILE="$autofix_log" \
    XDG_CACHE_HOME="$cache_dir" \
    XDG_CONFIG_HOME="$config_dir" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    timeout 20s bash "$install_root/scripts/chimera-control.sh" __service-preflight-node 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "node_service_preflight_heals_blocked_fixed_listener_case: helper failed rc=$rc output=$output"
  grep -Fxq 'peer.listen_addr = 9443' "$node_conf" || fail "node_service_preflight_heals_blocked_fixed_listener_case: operator node listen was overwritten"
  grep -Fxq 'carrier.addr = tcp://new-peer.example:18142' "$node_conf" || fail "node_service_preflight_heals_blocked_fixed_listener_case: node endpoint not refreshed"
  grep -Fxq 'CHIMERA_PEER_EGRESS_SERVER=new-peer.example:18142' "$peer_env" || fail "node_service_preflight_heals_blocked_fixed_listener_case: peer env endpoint not refreshed"
  grep -Fxq 'CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:9444' "$peer_env" || fail "node_service_preflight_heals_blocked_fixed_listener_case: operator local listen was overwritten"
  grep -Fxq 'CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:9443' "$peer_env" || fail "node_service_preflight_heals_blocked_fixed_listener_case: operator peer listen was overwritten"
  grep -Fxq 'CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:18135' "$override_file" || fail "node_service_preflight_heals_blocked_fixed_listener_case: runtime local override missing"
  grep -Fxq 'CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:0' "$override_file" || fail "node_service_preflight_heals_blocked_fixed_listener_case: runtime peer override missing"
  grep -q 'runtime_repair=node_listener_reset' "$autofix_log" || fail "node_service_preflight_heals_blocked_fixed_listener_case: autofix log missing node listener reset"

  rm -rf "$tmp_dir"
}

run_node_service_preflight_heals_blocked_fixed_listener_case

run_node_service_preflight_clears_stale_listener_override_case() {
  local tmp_dir bin_dir cache_dir config_dir runtime_dir install_root node_conf peer_env override_file output rc

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  install_root="$tmp_dir/chimera-release"
  node_conf="$tmp_dir/mesh-node.conf"
  peer_env="$config_dir/chimera/peer-egress.env"
  override_file="$cache_dir/chimera/runtime_listener_overrides.env"

  mkdir -p "$bin_dir" "$cache_dir/chimera" "$config_dir/chimera" "$runtime_dir" "$install_root/scripts" "$install_root/configs"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cp "$ROOT_DIR/configs/mesh-node.example.conf" "$install_root/configs/mesh-node.example.conf"
  cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = carrier.mesh:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
peer.listen_addr = 9443
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
  cat >"$peer_env" <<'EOF'
CHIMERA_PEER_EGRESS_MODE=node
CHIMERA_PEER_EGRESS_SERVER=carrier.mesh:443
CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:9444
CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:9443
EOF
  cat >"$override_file" <<'EOF'
CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:18135
CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:0
EOF
  cat >"$bin_dir/ss" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exit 0
EOF
  chmod +x "$bin_dir/ss"

  set +e
  output="$(
    PATH="$bin_dir:$PATH" \
    NODE_CONFIG_FILE="$node_conf" \
    PEER_EGRESS_ENV_FILE="$peer_env" \
    RUNTIME_LISTENER_OVERRIDE_FILE="$override_file" \
    XDG_CACHE_HOME="$cache_dir" \
    XDG_CONFIG_HOME="$config_dir" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    timeout 20s bash "$install_root/scripts/chimera-control.sh" __service-preflight-node 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "node_service_preflight_clears_stale_listener_override_case: helper failed rc=$rc output=$output"
  [[ ! -f "$override_file" ]] || fail "node_service_preflight_clears_stale_listener_override_case: stale override was not cleared"
  grep -Fxq 'peer.listen_addr = 9443' "$node_conf" || fail "node_service_preflight_clears_stale_listener_override_case: operator node listen changed"
  grep -Fxq 'CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:9444' "$peer_env" || fail "node_service_preflight_clears_stale_listener_override_case: operator local listen changed"
  grep -Fxq 'CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:9443' "$peer_env" || fail "node_service_preflight_clears_stale_listener_override_case: operator peer listen changed"

  rm -rf "$tmp_dir"
}

run_node_service_preflight_clears_stale_listener_override_case

run_public_start_second_attempt_repairs_fixed_peer_update_listen_case() {
  local tmp_dir bin_dir cache_dir config_dir runtime_dir install_root node_conf bootstrap_env peer_env transparent_env fake_runner fake_systemctl runner_log state_file autofix_log override_file output rc

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  install_root="$tmp_dir/chimera-release"
  node_conf="$tmp_dir/mesh-node.conf"
  bootstrap_env="$config_dir/chimera/mesh_bootstrap.env"
  peer_env="$config_dir/chimera/peer-egress.env"
  transparent_env="$config_dir/chimera/transparent-runtime.env"
  fake_runner="$tmp_dir/fake-runner.sh"
  fake_systemctl="$bin_dir/systemctl"
  runner_log="$tmp_dir/runner.log"
  state_file="$tmp_dir/runtime_state.json"
  autofix_log="$cache_dir/chimera/autofix.log"
  override_file="$cache_dir/chimera/runtime_listener_overrides.env"

  mkdir -p "$bin_dir" "$cache_dir/chimera" "$config_dir/chimera" "$runtime_dir" "$install_root/scripts"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = carrier.mesh:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
peer.listen_addr = auto
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
  cat >"$bootstrap_env" <<'EOF'
CHIMERA_PEER_UPDATE_BASE_URL=http://node.example:18179
CHIMERA_PEER_UPDATE_LISTEN=0.0.0.0:18179
EOF
  cat >"$peer_env" <<'EOF'
CHIMERA_PEER_EGRESS_MODE=node
CHIMERA_PEER_EGRESS_SERVER=carrier.mesh:443
CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:18135
CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:0
EOF
  cat >"$transparent_env" <<'EOF'
CHIMERA_RUNNER_USE_SUDO=0
EOF

  cat >"$fake_systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  --user) shift ;;
esac
case "${1:-}" in
  show-environment) exit 1 ;;
  stop|list-units|list-unit-files|daemon-reload) exit 0 ;;
  *) exit 0 ;;
esac
EOF
  chmod +x "$fake_systemctl"
  cat >"$bin_dir/ss" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
cat <<'OUT'
LISTEN 0 128 0.0.0.0:9443 0.0.0.0:*
LISTEN 0 128 127.0.0.1:9444 0.0.0.0:*
OUT
EOF
  chmod +x "$bin_dir/ss"

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
        printf '%s\n' '{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true}' >"\$state_file"
      fi
      exit 0
    fi
    if [[ "\${2:-}" == "state" && "\${3:-}" == "proof" ]]; then
      echo "datapath_proof=ok"
      exit 0
    fi
    if [[ "\${2:-}" == "down" || ( "\${2:-}" == "rollback" && "\${3:-}" == "recover" ) ]]; then
      state_file=""
      prev=""
      for arg in "\$@"; do
        if [[ "\$prev" == "--state-file" ]]; then
          state_file="\$arg"
          break
        fi
        prev="\$arg"
      done
      [[ -n "\$state_file" ]] && rm -f "\$state_file"
      exit 0
    fi
    exit 0
    ;;
  peer-egress|transparent-runtime)
    sleep 60
    ;;
  peer-update)
    if [[ "\${CHIMERA_PEER_UPDATE_LISTEN:-}" != "0.0.0.0:0" ]]; then
      exit 1
    fi
    mkdir -p "\$(dirname "\${CHIMERA_PEER_UPDATE_STATE_FILE:?}")"
    printf '%s\n' '{"status":"ready"}' >"\${CHIMERA_PEER_UPDATE_STATE_FILE:?}"
    sleep 60
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
    CHIMERA_PEER_UPDATE_BASE_URL= \
    CHIMERA_PEER_UPDATE_LISTEN= \
    NODE_CONFIG_FILE="$node_conf" \
    CHIMERA_BOOTSTRAP_ENV_FILE="$bootstrap_env" \
    PEER_EGRESS_ENV_FILE="$peer_env" \
    TRANSPARENT_RUNTIME_ENV_FILE="$transparent_env" \
    STATE_FILE="$state_file" \
    RUNTIME_LISTENER_OVERRIDE_FILE="$override_file" \
    CHIMERA_RUNNER="$fake_runner" \
    AUTOFIX_LOG_FILE="$autofix_log" \
    CHIMERA_UPDATE_FIRST_CHECKED=1 \
    SITE_AUTOWATCH_ENABLED=0 \
    timeout 20s bash "$install_root/scripts/chimera-control.sh" start 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -ne 124 ]] || fail "public_start_second_attempt_repairs_fixed_peer_update_listen_case: start timed out"
  [[ "$rc" -eq 0 ]] || fail "public_start_second_attempt_repairs_fixed_peer_update_listen_case: start did not self-heal output=$output"
  [[ "$output" == *"start_status=ok mode=direct"* ]] || fail "public_start_second_attempt_repairs_fixed_peer_update_listen_case: start not ok output=$output"
  [[ "$output" == *"peer_update_publish=ok"* ]] || fail "public_start_second_attempt_repairs_fixed_peer_update_listen_case: peer update publish not ok output=$output"
  grep -q '^CHIMERA_PEER_UPDATE_LISTEN=0.0.0.0:18179$' "$bootstrap_env" || fail "public_start_second_attempt_repairs_fixed_peer_update_listen_case: operator bootstrap listen was overwritten"
  grep -q '^CHIMERA_PEER_UPDATE_LISTEN=0.0.0.0:18179$' "$config_dir/chimera/peer-update.env" || fail "public_start_second_attempt_repairs_fixed_peer_update_listen_case: runtime peer-update env was overwritten"
  grep -q '^CHIMERA_PEER_UPDATE_LISTEN=0.0.0.0:0$' "$override_file" || fail "public_start_second_attempt_repairs_fixed_peer_update_listen_case: runtime peer-update override missing"
  [[ "$(grep -c '^peer-update$' "$runner_log")" -eq 2 ]] || fail "public_start_second_attempt_repairs_fixed_peer_update_listen_case: peer-update was not retried exactly once"
  grep -q 'runtime_repair=peer_update_listener_reset' "$autofix_log" || fail "public_start_second_attempt_repairs_fixed_peer_update_listen_case: autofix log missing peer-update listener reset"

  PATH="$bin_dir:$PATH" \
  HOME="$tmp_dir/home" \
  XDG_CACHE_HOME="$cache_dir" \
  XDG_CONFIG_HOME="$config_dir" \
  XDG_RUNTIME_DIR="$runtime_dir" \
  NODE_CONFIG_FILE="$node_conf" \
  CHIMERA_BOOTSTRAP_ENV_FILE="$bootstrap_env" \
  PEER_EGRESS_ENV_FILE="$peer_env" \
  TRANSPARENT_RUNTIME_ENV_FILE="$transparent_env" \
  STATE_FILE="$state_file" \
  CHIMERA_RUNNER="$fake_runner" \
  timeout 20s bash "$install_root/scripts/chimera-control.sh" stop >/dev/null 2>&1 || true

  rm -rf "$tmp_dir"
}

run_public_start_second_attempt_repairs_fixed_peer_update_listen_case

run_systemd_start_retry_heals_fixed_listener_case() {
  local tmp_dir bin_dir cache_dir config_dir runtime_dir install_root node_conf peer_env transparent_env fake_runner fake_systemctl runner_log systemctl_log state_file autofix_log override_file output rc

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  install_root="$tmp_dir/chimera-release"
  node_conf="$tmp_dir/mesh-node.conf"
  peer_env="$config_dir/chimera/peer-egress.env"
  transparent_env="$config_dir/chimera/transparent-runtime.env"
  fake_runner="$tmp_dir/fake-runner.sh"
  fake_systemctl="$bin_dir/systemctl"
  runner_log="$tmp_dir/runner.log"
  systemctl_log="$tmp_dir/systemctl.log"
  state_file="$tmp_dir/runtime_state.json"
  autofix_log="$cache_dir/chimera/autofix.log"
  override_file="$cache_dir/chimera/runtime_listener_overrides.env"

  mkdir -p "$bin_dir" "$cache_dir/chimera" "$config_dir/chimera" "$runtime_dir" "$install_root/scripts"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = carrier.mesh:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
peer.listen_addr = 9443
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
  cat >"$peer_env" <<'EOF'
CHIMERA_PEER_EGRESS_MODE=node
CHIMERA_PEER_EGRESS_SERVER=carrier.mesh:443
CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:9444
CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:9443
CHIMERA_PEER_EGRESS_TOKEN=test-token
EOF
  cat >"$transparent_env" <<'EOF'
CHIMERA_RUNNER_USE_SUDO=0
EOF
  cat >"$bin_dir/ss" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
cat <<'OUT'
LISTEN 0 128 0.0.0.0:9443 0.0.0.0:*
LISTEN 0 128 127.0.0.1:9444 0.0.0.0:*
OUT
EOF
  chmod +x "$bin_dir/ss"

  cat >"$fake_systemctl" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >>"$systemctl_log"
case "\${1:-}" in
  --user) shift ;;
esac
case "\${1:-}" in
  show-environment|daemon-reload|start|stop|list-units|list-unit-files)
    exit 0
    ;;
  is-active)
    unit="\${2:-}"
    case "\$unit" in
      chimera-node.service)
        if grep -Fxq 'peer.listen_addr = 9443' "$node_conf" \
          && grep -Fxq 'CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:9444' "$peer_env" \
          && grep -Fxq 'CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:9443' "$peer_env" \
          && grep -Fxq 'CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:18135' "$override_file" \
          && grep -Fxq 'CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:0' "$override_file"; then
          echo active
        else
          echo failed
        fi
        ;;
      chimera-datapath.service)
        echo active
        ;;
      *)
        echo inactive
        ;;
    esac
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
        printf '%s\n' '{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true}' >"\$state_file"
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

  set +e
  output="$(
    PATH="$bin_dir:$PATH" \
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$cache_dir" \
    XDG_CONFIG_HOME="$config_dir" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    NODE_CONFIG_FILE="$node_conf" \
    PEER_EGRESS_ENV_FILE="$peer_env" \
    TRANSPARENT_RUNTIME_ENV_FILE="$transparent_env" \
    STATE_FILE="$state_file" \
    RUNTIME_LISTENER_OVERRIDE_FILE="$override_file" \
    AUTOFIX_LOG_FILE="$autofix_log" \
    CHIMERA_RUNNER="$fake_runner" \
    CHIMERA_UPDATE_FIRST_CHECKED=1 \
    SITE_AUTOWATCH_ENABLED=0 \
    timeout 20s bash "$install_root/scripts/chimera-control.sh" start 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -ne 124 ]] || fail "systemd_start_retry_heals_fixed_listener_case: start timed out"
  [[ "$rc" -eq 0 ]] || fail "systemd_start_retry_heals_fixed_listener_case: expected success got rc=$rc output=$output"
  [[ "$output" == *"start_status=ok mode=systemd_user"* ]] || fail "systemd_start_retry_heals_fixed_listener_case: start not ok output=$output"
  grep -Fxq 'peer.listen_addr = 9443' "$node_conf" || fail "systemd_start_retry_heals_fixed_listener_case: operator node listen was overwritten output=$output"
  grep -Fxq 'CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:9444' "$peer_env" || fail "systemd_start_retry_heals_fixed_listener_case: operator local listen was overwritten output=$output"
  grep -Fxq 'CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:9443' "$peer_env" || fail "systemd_start_retry_heals_fixed_listener_case: operator peer listen was overwritten output=$output"
  grep -Fxq 'CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:18135' "$override_file" || fail "systemd_start_retry_heals_fixed_listener_case: runtime local override missing output=$output"
  grep -Fxq 'CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:0' "$override_file" || fail "systemd_start_retry_heals_fixed_listener_case: runtime peer override missing output=$output"
  [[ "$(grep -c '^--user start chimera-node.service$' "$systemctl_log")" -eq 1 ]] || fail "systemd_start_retry_heals_fixed_listener_case: node service should start cleanly after preflight repair output=$output"
  grep -q 'runtime_repair=node_listener_reset' "$autofix_log" || fail "systemd_start_retry_heals_fixed_listener_case: autofix log missing node listener reset"

  rm -rf "$tmp_dir"
}

run_systemd_start_retry_heals_fixed_listener_case

run_node_peer_endpoint_refresh_case() {
  local tmp_dir install_root config_dir cache_dir runtime_dir node_conf peer_env bootstrap_env output rc

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  config_dir="$tmp_dir/config/chimera"
  cache_dir="$tmp_dir/cache/chimera"
  runtime_dir="$tmp_dir/runtime"
  node_conf="$tmp_dir/mesh-node.conf"
  peer_env="$config_dir/peer-egress.env"
  bootstrap_env="$config_dir/mesh_bootstrap.env"

  mkdir -p "$install_root/scripts" "$install_root/configs" "$config_dir" "$cache_dir" "$runtime_dir"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = old-peer.example:443
carrier.server_name = node.local
capture.mode = auto
capture.tun_supported = true
peer.listen_addr = auto
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
  cat >"$peer_env" <<'EOF'
CHIMERA_PEER_EGRESS_MODE=node
CHIMERA_PEER_EGRESS_SERVER=old-peer.example:443
CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:18135
CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:0
EOF
  cat >"$bootstrap_env" <<'EOF'
CHIMERA_MESH_REMOTE_ENDPOINT=new-peer.example:18142
EOF

  set +e
  output="$(
    CHIMERA_BOOTSTRAP_ENV_FILE="$bootstrap_env" \
    NODE_CONFIG_FILE="$node_conf" \
    PEER_EGRESS_ENV_FILE="$peer_env" \
    XDG_CACHE_HOME="$tmp_dir/cache" \
    XDG_CONFIG_HOME="$tmp_dir/config" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    bash -lc 'source "'"$install_root/scripts/chimera-control.sh"'"; refresh_node_peer_target_from_bootstrap >/dev/null; cat "'"$peer_env"'"; cat "'"$node_conf"'"' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "node_peer_endpoint_refresh_case: refresh failed output=$output"
  grep -Fxq 'CHIMERA_PEER_EGRESS_SERVER=new-peer.example:18142' "$peer_env" || fail "node_peer_endpoint_refresh_case: peer env server not refreshed output=$output"
  grep -Fxq 'carrier.addr = tcp://new-peer.example:18142' "$node_conf" || fail "node_peer_endpoint_refresh_case: node config carrier addr not refreshed output=$output"

  rm -rf "$tmp_dir"
}

run_node_peer_endpoint_refresh_case

run_peer_update_publication_heals_fixed_listen_once_case() {
  local tmp_dir cache_dir config_dir runtime_dir install_root bootstrap_env peer_env state_file fake_runner runner_log autofix_log override_file output rc

  tmp_dir="$(mktemp -d)"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  install_root="$tmp_dir/chimera-release"
  bootstrap_env="$config_dir/chimera/mesh_bootstrap.env"
  peer_env="$config_dir/chimera/peer-update.env"
  state_file="$cache_dir/chimera/peer-update.state.json"
  fake_runner="$tmp_dir/fake-runner.sh"
  runner_log="$tmp_dir/runner.log"
  autofix_log="$cache_dir/chimera/autofix.log"
  override_file="$cache_dir/chimera/runtime_listener_overrides.env"

  mkdir -p "$cache_dir/chimera" "$config_dir/chimera" "$runtime_dir" "$install_root/scripts"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cat >"$bootstrap_env" <<'EOF'
CHIMERA_PEER_UPDATE_BASE_URL=http://node.example:18179
CHIMERA_PEER_UPDATE_LISTEN=0.0.0.0:18179
EOF
  cat >"$fake_runner" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${RUNNER_LOG_FILE:?}"
case "${1:-}" in
  peer-update)
    if [[ "${CHIMERA_PEER_UPDATE_LISTEN:-}" != "0.0.0.0:0" ]]; then
      exit 1
    fi
    mkdir -p "$(dirname "${CHIMERA_PEER_UPDATE_STATE_FILE:?}")"
    printf '%s\n' '{"status":"ready"}' >"${CHIMERA_PEER_UPDATE_STATE_FILE:?}"
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
EOF
  chmod +x "$fake_runner"
  cat >"$tmp_dir/ss" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
cat <<'OUT'
LISTEN 0 128 0.0.0.0:18179 0.0.0.0:*
OUT
EOF
  chmod +x "$tmp_dir/ss"

  set +e
  output="$(
    PATH="$tmp_dir:$PATH" \
    CHIMERA_BOOTSTRAP_ENV_FILE="$bootstrap_env" \
    PEER_UPDATE_ENV_FILE="$peer_env" \
    PEER_UPDATE_STATE_FILE="$state_file" \
    CHIMERA_PEER_UPDATE_BASE_URL= \
    CHIMERA_PEER_UPDATE_LISTEN= \
    CHIMERA_RUNNER="$fake_runner" \
    RUNTIME_LISTENER_OVERRIDE_FILE="$override_file" \
    AUTOFIX_LOG_FILE="$autofix_log" \
    RUNNER_LOG_FILE="$runner_log" \
    XDG_CACHE_HOME="$cache_dir" \
    XDG_CONFIG_HOME="$config_dir" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    bash -lc 'source "'"$install_root/scripts/chimera-control.sh"'"; if refresh_runtime_publication_after_node_start; then rc=0; else rc=$?; fi; echo "peer_update_publish=$START_RUNTIME_PEER_UPDATE_STATUS"; echo "discovery_publish=$START_RUNTIME_DISCOVERY_PUBLISH_STATUS"; cat "'"$peer_env"'"; exit "$rc"' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "peer_update_publication_heals_fixed_listen_once_case: publication did not recover output=$output"
  [[ "$output" == *"peer_update_publish=ok"* ]] || fail "peer_update_publication_heals_fixed_listen_once_case: peer update status not ok output=$output"
  grep -q '^CHIMERA_PEER_UPDATE_LISTEN=0.0.0.0:18179$' "$peer_env" || fail "peer_update_publication_heals_fixed_listen_once_case: operator env listen was overwritten"
  grep -q '^CHIMERA_PEER_UPDATE_LISTEN=0.0.0.0:18179$' "$bootstrap_env" || fail "peer_update_publication_heals_fixed_listen_once_case: bootstrap listen was overwritten"
  grep -q '^CHIMERA_PEER_UPDATE_LISTEN=0.0.0.0:0$' "$override_file" || fail "peer_update_publication_heals_fixed_listen_once_case: runtime override missing"
  [[ -f "$state_file" ]] || fail "peer_update_publication_heals_fixed_listen_once_case: peer update state file missing after retry"
  [[ "$(grep -c '^peer-update$' "$runner_log")" -eq 1 ]] || fail "peer_update_publication_heals_fixed_listen_once_case: peer-update should self-heal before first start"
  grep -q 'runtime_repair=peer_update_listener_reset' "$autofix_log" || fail "peer_update_publication_heals_fixed_listen_once_case: autofix log missing peer-update listener reset"

  rm -rf "$tmp_dir"
}

run_peer_update_publication_heals_fixed_listen_once_case

run_peer_update_invalid_base_url_preserves_existing_env_case() {
  local tmp_dir cache_dir config_dir runtime_dir install_root bootstrap_env peer_env output rc

  tmp_dir="$(mktemp -d)"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  install_root="$tmp_dir/chimera-release"
  bootstrap_env="$config_dir/chimera/mesh_bootstrap.env"
  peer_env="$config_dir/chimera/peer-update.env"

  mkdir -p "$cache_dir/chimera" "$config_dir/chimera" "$runtime_dir" "$install_root/scripts"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cat >"$bootstrap_env" <<'EOF'
CHIMERA_PEER_UPDATE_BASE_URL=not-a-url
EOF
  cat >"$peer_env" <<'EOF'
CHIMERA_PEER_UPDATE_BASE_URL=http://last-known-good.example:18179
CHIMERA_PEER_UPDATE_LISTEN=0.0.0.0:0
CHIMERA_PEER_UPDATE_STATE_FILE=/tmp/last-known-good.state.json
EOF

  set +e
  output="$(
    CHIMERA_BOOTSTRAP_ENV_FILE="$bootstrap_env" \
    PEER_UPDATE_ENV_FILE="$peer_env" \
    XDG_CACHE_HOME="$cache_dir" \
    XDG_CONFIG_HOME="$config_dir" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    bash -lc 'source "'"$install_root/scripts/chimera-control.sh"'"; configure_peer_update_env' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -ne 0 ]] || fail "peer_update_invalid_base_url_preserves_existing_env_case: invalid url unexpectedly accepted"
  grep -Fxq 'CHIMERA_PEER_UPDATE_BASE_URL=http://last-known-good.example:18179' "$peer_env" || fail "peer_update_invalid_base_url_preserves_existing_env_case: last-known-good env was deleted"
  grep -Fxq 'CHIMERA_PEER_UPDATE_LISTEN=0.0.0.0:0' "$peer_env" || fail "peer_update_invalid_base_url_preserves_existing_env_case: listener env changed on invalid input"
  [[ -z "$output" ]] || true

  rm -rf "$tmp_dir"
}

run_peer_update_invalid_base_url_preserves_existing_env_case

run_peer_update_publication_failure_degrades_start_case() {
  local tmp_dir bin_dir cache_dir config_dir runtime_dir install_root node_conf bootstrap_env peer_env transparent_env output rc fake_runner fake_systemctl runner_log systemctl_log

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  install_root="$tmp_dir/chimera-release"
  node_conf="$tmp_dir/mesh-node.conf"
  bootstrap_env="$config_dir/chimera/mesh_bootstrap.env"
  peer_env="$config_dir/chimera/peer-egress.env"
  transparent_env="$config_dir/chimera/transparent-runtime.env"
  fake_runner="$tmp_dir/fake-runner.sh"
  fake_systemctl="$bin_dir/systemctl"
  runner_log="$tmp_dir/runner.log"
  systemctl_log="$tmp_dir/systemctl.log"

  mkdir -p "$bin_dir" "$cache_dir/chimera" "$config_dir/chimera" "$runtime_dir" "$install_root/scripts" "$install_root/configs"
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
  cat >"$bootstrap_env" <<'EOF'
CHIMERA_PEER_UPDATE_BASE_URL=http://node.example:18179
EOF
  cat >"$peer_env" <<'EOF'
CHIMERA_PEER_EGRESS_MODE=node
CHIMERA_PEER_EGRESS_SERVER=carrier.mesh:443
CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:18135
CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:0
EOF
  cat >"$transparent_env" <<'EOF'
CHIMERA_RUNNER_USE_SUDO=0
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
      if [[ -n "\$state_file" ]]; then
        mkdir -p "\$(dirname "\$state_file")"
        printf '%s\n' '{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true}' >"\$state_file"
      fi
      exit 0
    fi
    if [[ "\${2:-}" == "state" && "\${3:-}" == "proof" ]]; then
      echo "datapath_proof=ok"
      exit 0
    fi
    exit 0
    ;;
  peer-update)
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
    CHIMERA_BOOTSTRAP_ENV_FILE="$bootstrap_env" \
    CHIMERA_RUNNER="$fake_runner" \
    CHIMERA_UPDATE_FIRST_CHECKED=1 \
    SITE_AUTOWATCH_ENABLED=0 \
    timeout 20s bash "$install_root/scripts/chimera-control.sh" start 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -ne 124 ]] || fail "peer_update_publication_failure_degrades_start_case: start timed out"
  [[ "$rc" -eq 2 ]] || fail "peer_update_publication_failure_degrades_start_case: expected rc=2 got rc=$rc output=$output"
  [[ "$output" == *"start_status=partial"* ]] || fail "peer_update_publication_failure_degrades_start_case: missing partial status output=$output"
  [[ "$output" == *"reason=runtime_publication_unready"* ]] || fail "peer_update_publication_failure_degrades_start_case: missing publication reason output=$output"
  [[ "$output" == *"peer_update_publish=failed"* ]] || fail "peer_update_publication_failure_degrades_start_case: missing peer update failed status output=$output"
  [[ "$output" == *"discovery_publish=skipped"* ]] || fail "peer_update_publication_failure_degrades_start_case: missing discovery skipped status output=$output"
  [[ "$output" == *"fail_closed=true"* ]] || fail "peer_update_publication_failure_degrades_start_case: missing fail_closed=true output=$output"
  [[ "$output" == *"datapath_apply=ok"* ]] || fail "peer_update_publication_failure_degrades_start_case: datapath apply should still succeed output=$output"
  [[ "$output" == *"datapath_rollback=ok"* ]] || fail "peer_update_publication_failure_degrades_start_case: rollback should close partial datapath output=$output"
  grep -q '^cli rollback recover ' "$runner_log" || fail "peer_update_publication_failure_degrades_start_case: rollback recover was not invoked"
  grep -q '^peer-update$' "$runner_log" || fail "peer_update_publication_failure_degrades_start_case: peer-update runtime was not started"

  rm -rf "$tmp_dir"
}

run_peer_update_publication_failure_degrades_start_case

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

run_mesh_discovery_invite_token_env_case() {
  local tmp_dir install_root config_dir cache_dir state_file peer_env fake_runner token_capture output rc

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  config_dir="$tmp_dir/config/chimera"
  cache_dir="$tmp_dir/cache/chimera"
  state_file="$cache_dir/peer-egress.state"
  peer_env="$config_dir/peer-egress.env"
  fake_runner="$tmp_dir/fake-runner.sh"
  token_capture="$tmp_dir/advertise-token.txt"

  mkdir -p "$install_root/scripts" "$config_dir" "$cache_dir"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cat >"$state_file" <<'EOF'
mode=node
resolved_local_listen=127.0.0.1:45678
resolved_peer_listen=198.51.100.44:45678
EOF
  cat >"$peer_env" <<'EOF'
CHIMERA_PEER_EGRESS_TOKEN=test-invite-token
EOF
  cat >"$fake_runner" <<EOF
#!/usr/bin/env bash
set -euo pipefail
if [[ "\${1:-}" == "cli" ]]; then
  shift
fi
printf '%s\n' "\${CHIMERA_MESH_ADVERTISE_INVITE_TOKEN:-}" > "$token_capture"
while [[ \$# -gt 0 ]]; do
  case "\$1" in
    --out)
      printf '%s\n' '{}' > "\${2:?}"
      shift 2
      ;;
    --pubkey-out)
      printf '%s\n' 'PUBKEY' > "\${2:?}"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
EOF
  chmod +x "$fake_runner"

  set +e
  output="$(
    CHIMERA_RUNNER="$fake_runner" \
    PEER_EGRESS_ENV_FILE="$peer_env" \
    PEER_EGRESS_STATE_FILE="$state_file" \
    XDG_CACHE_HOME="$tmp_dir/cache" \
    bash -lc 'source "'"$install_root/scripts/chimera-control.sh"'"; publish_mesh_discovery_snapshot' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "mesh_discovery_invite_token_env_case: publish failed output=$output"
  [[ -f "$token_capture" ]] || fail "mesh_discovery_invite_token_env_case: token capture missing"
  grep -Fxq 'test-invite-token' "$token_capture" || fail "mesh_discovery_invite_token_env_case: invite token env not forwarded"

  rm -rf "$tmp_dir"
}

run_mesh_discovery_invite_token_env_case

run_mesh_bootstrap_keyring_runtime_case() {
  local tmp_dir install_root config_dir cache_dir runtime_dir bootstrap_env fake_runner cli_log output rc

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  config_dir="$tmp_dir/config/chimera"
  cache_dir="$tmp_dir/cache/chimera"
  runtime_dir="$tmp_dir/runtime"
  bootstrap_env="$config_dir/mesh_bootstrap.env"
  fake_runner="$tmp_dir/fake-runner.sh"
  cli_log="$tmp_dir/cli.log"

  mkdir -p "$install_root/scripts" "$install_root/configs" "$config_dir" "$cache_dir" "$runtime_dir"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cat >"$install_root/configs/mesh-node.example.conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = 203.0.113.10:443
carrier.server_name = node.local
capture.mode = auto
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
  cat >"$bootstrap_env" <<'EOF'
CHIMERA_MESH_NODES_DISCOVERY_URL=https://seed.example/mesh_nodes.discovery.json
CHIMERA_MESH_NODES_DISCOVERY_KEYRING=key-a:pubkey-a,key-b:pubkey-b
CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS=1200
CHIMERA_MESH_NAMESPACE=test-mesh
CHIMERA_MESH_LOCAL_NODE=test-local-node
CHIMERA_MESH_TRAFFIC_PROFILE=high_speed_anonymous
EOF
  cat >"$fake_runner" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >>"$cli_log"
if [[ "\${1:-}" == "cli" ]]; then
  shift
fi
case "\${1:-}" in
  mesh)
    case "\${2:-}" in
      nodes)
        case "\${3:-}" in
          select)
            shift 3
            while [[ \$# -gt 0 ]]; do
              case "\$1" in
                --id)
                  exit 0
                  ;;
              esac
              shift
            done
            echo "mesh nodes select error: interactive selection requires a terminal or GUI; use --id <node_id>" >&2
            exit 2
            ;;
          selected-endpoint)
            printf '%s\n' '198.51.100.77:443'
            exit 0
            ;;
          selected-peer-spec)
            printf '%s\n' 'seed-node@198.51.100.77:443@eu@42@99'
            exit 0
            ;;
          best)
            printf '%s\n' 'node_id=seed-node'
            exit 0
            ;;
        esac
        ;;
    esac
    ;;
esac
exit 0
EOF
  chmod +x "$fake_runner"

  set +e
  output="$(
    CHIMERA_BOOTSTRAP_ENV_FILE="$bootstrap_env" \
    CHIMERA_RUNNER="$fake_runner" \
    XDG_CACHE_HOME="$tmp_dir/cache" \
    XDG_CONFIG_HOME="$tmp_dir/config" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    bash -lc 'source "'"$install_root/scripts/chimera-control.sh"'"; seed_mesh_control_plane_authority_from_bootstrap --strict; printf "peer_spec=%s\n" "$(selected_mesh_remote_peer_spec_from_inventory)"; refresh_node_peer_target_from_bootstrap; cat "'"$bootstrap_env"'"; cat "'"$install_root/configs/mesh-node.example.conf"'"; printf "endpoint_file=%s\n" "$(cat "'"$install_root/configs/chimera_runtime_endpoint.txt"'")"' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "mesh_bootstrap_keyring_runtime_case: bootstrap refresh failed output=$output"
  [[ "$output" == *"mesh_control_plane_seed=ok"* ]] || fail "mesh_bootstrap_keyring_runtime_case: strict seed did not succeed output=$output"
  [[ "$output" == *"peer_spec=seed-node@198.51.100.77:443@eu@42@99"* ]] || fail "mesh_bootstrap_keyring_runtime_case: selected peer spec missing output=$output"
  [[ "$output" == *"CHIMERA_MESH_REMOTE_PEER_SPEC=seed-node@198.51.100.77:443@eu@42@99"* ]] || fail "mesh_bootstrap_keyring_runtime_case: remote peer spec not persisted output=$output"
  [[ "$output" == *"carrier.addr = tcp://198.51.100.77:443"* ]] || fail "mesh_bootstrap_keyring_runtime_case: node config not refreshed output=$output"
  [[ "$output" == *"endpoint_file=198.51.100.77:443"* ]] || fail "mesh_bootstrap_keyring_runtime_case: runtime endpoint file missing output=$output"
  grep -q -- '--discovery-keyring key-a:pubkey-a,key-b:pubkey-b' "$cli_log" || fail "mesh_bootstrap_keyring_runtime_case: keyring not forwarded to cli"

  rm -rf "$tmp_dir"
}

run_mesh_bootstrap_keyring_runtime_case

run_mesh_bootstrap_discovery_urls_runtime_case() {
  local tmp_dir install_root config_dir cache_dir runtime_dir bootstrap_env fake_runner cli_log output rc

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  config_dir="$tmp_dir/config/chimera"
  cache_dir="$tmp_dir/cache/chimera"
  runtime_dir="$tmp_dir/runtime"
  bootstrap_env="$config_dir/mesh_bootstrap.env"
  fake_runner="$tmp_dir/fake-runner.sh"
  cli_log="$tmp_dir/cli.log"

  mkdir -p "$install_root/scripts" "$install_root/configs" "$config_dir" "$cache_dir" "$runtime_dir"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cp "$ROOT_DIR/configs/mesh-node.example.conf" "$install_root/configs/mesh-node.example.conf"
  cat >"$bootstrap_env" <<'EOF'
CHIMERA_MESH_NODES_DISCOVERY_URLS=https://seed-a.example/mesh_nodes.discovery.json,https://seed-b.example/mesh_nodes.discovery.json
CHIMERA_MESH_NODES_DISCOVERY_KEYRING=key-a:pubkey-a,key-b:pubkey-b
CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS=1200
CHIMERA_MESH_NAMESPACE=test-mesh
CHIMERA_MESH_LOCAL_NODE=test-local-node
CHIMERA_MESH_TRAFFIC_PROFILE=high_speed_anonymous
EOF
  cat >"$fake_runner" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >>"$cli_log"
if [[ "\${1:-}" == "cli" ]]; then
  shift
fi
case "\${1:-}" in
  mesh)
    case "\${2:-}" in
      nodes)
        case "\${3:-}" in
          select)
            printf '%s\n' 'selected'
            exit 0
            ;;
          selected-peer-spec)
            printf '%s\n' 'seed-node@198.51.100.88:443@eu@42@99'
            exit 0
            ;;
          selected-endpoint)
            printf '%s\n' '198.51.100.88:443'
            exit 0
            ;;
          best)
            printf '%s\n' 'node_id=seed-node'
            exit 0
            ;;
        esac
        ;;
    esac
    ;;
esac
exit 0
EOF
  chmod +x "$fake_runner"

  set +e
  output="$(
    CHIMERA_BOOTSTRAP_ENV_FILE="$bootstrap_env" \
    CHIMERA_RUNNER="$fake_runner" \
    XDG_CACHE_HOME="$tmp_dir/cache" \
    XDG_CONFIG_HOME="$tmp_dir/config" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    bash -lc 'source "'"$install_root/scripts/chimera-control.sh"'"; seed_mesh_control_plane_authority_from_bootstrap --strict; printf "peer_spec=%s\n" "$(selected_mesh_remote_peer_spec_from_inventory)"; refresh_node_peer_target_from_bootstrap; cat "'"$bootstrap_env"'"; cat "'"$install_root/configs/mesh-node.example.conf"'"' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "mesh_bootstrap_discovery_urls_runtime_case: bootstrap refresh failed output=$output"
  [[ "$output" == *"mesh_control_plane_seed=ok"* ]] || fail "mesh_bootstrap_discovery_urls_runtime_case: strict seed did not succeed output=$output"
  [[ "$output" == *"peer_spec=seed-node@198.51.100.88:443@eu@42@99"* ]] || fail "mesh_bootstrap_discovery_urls_runtime_case: selected peer spec missing output=$output"
  [[ "$output" == *"CHIMERA_MESH_REMOTE_PEER_SPEC=seed-node@198.51.100.88:443@eu@42@99"* ]] || fail "mesh_bootstrap_discovery_urls_runtime_case: remote peer spec not persisted output=$output"
  grep -q -- '--discovery-keyring key-a:pubkey-a,key-b:pubkey-b' "$cli_log" || fail "mesh_bootstrap_discovery_urls_runtime_case: keyring not forwarded to cli"
  grep -q '^CHIMERA_MESH_NODES_DISCOVERY_URLS=https://seed-a\.example/mesh_nodes\.discovery\.json,https://seed-b\.example/mesh_nodes\.discovery\.json$' "$bootstrap_env" || fail "mesh_bootstrap_discovery_urls_runtime_case: discovery source list not persisted"

  rm -rf "$tmp_dir"
}

run_mesh_bootstrap_discovery_urls_runtime_case

run_mesh_bootstrap_direct_peer_spec_runtime_case() {
  local tmp_dir install_root config_dir cache_dir runtime_dir bootstrap_env output rc

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  config_dir="$tmp_dir/config/chimera"
  cache_dir="$tmp_dir/cache/chimera"
  runtime_dir="$tmp_dir/runtime"
  bootstrap_env="$config_dir/mesh_bootstrap.env"

  mkdir -p "$install_root/scripts" "$install_root/configs" "$config_dir" "$cache_dir" "$runtime_dir"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cp "$ROOT_DIR/configs/mesh-node.example.conf" "$install_root/configs/mesh-node.example.conf"
  cat >"$bootstrap_env" <<'EOF'
CHIMERA_MESH_NAMESPACE=test-mesh
CHIMERA_MESH_LOCAL_NODE=test-local-node
CHIMERA_MESH_TRAFFIC_PROFILE=high_speed_anonymous
CHIMERA_MESH_REMOTE_PEER_SPEC=seed-node@198.51.100.66:443@eu@42@99
EOF

  set +e
  output="$(
    CHIMERA_BOOTSTRAP_ENV_FILE="$bootstrap_env" \
    XDG_CACHE_HOME="$tmp_dir/cache" \
    XDG_CONFIG_HOME="$tmp_dir/config" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    bash -lc 'source "'"$install_root/scripts/chimera-control.sh"'"; seed_mesh_control_plane_authority_from_bootstrap --strict; refresh_node_peer_target_from_bootstrap; cat "'"$bootstrap_env"'"; cat "'"$install_root/configs/mesh-node.example.conf"'"; printf "endpoint_file=%s\n" "$(cat "'"$install_root/configs/chimera_runtime_endpoint.txt"'")"' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "mesh_bootstrap_direct_peer_spec_runtime_case: bootstrap refresh failed output=$output"
  [[ "$output" == *"mesh_control_plane_seed=ok"* ]] || fail "mesh_bootstrap_direct_peer_spec_runtime_case: strict seed did not succeed output=$output"
  [[ "$output" == *"CHIMERA_MESH_REMOTE_PEER_SPEC=seed-node@198.51.100.66:443@eu@42@99"* ]] || fail "mesh_bootstrap_direct_peer_spec_runtime_case: remote peer spec not persisted output=$output"
  [[ "$output" == *"carrier.addr = tcp://198.51.100.66:443"* ]] || fail "mesh_bootstrap_direct_peer_spec_runtime_case: node config not refreshed output=$output"
  [[ "$output" == *"endpoint_file=198.51.100.66:443"* ]] || fail "mesh_bootstrap_direct_peer_spec_runtime_case: runtime endpoint file missing output=$output"

  rm -rf "$tmp_dir"
}

run_mesh_bootstrap_direct_peer_spec_runtime_case

run_mesh_bind_uses_bootstrap_env_case() {
  local tmp_dir install_root config_dir cache_dir runtime_dir bootstrap_env peer_env fake_runner cli_log output rc bindings_file

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  config_dir="$tmp_dir/config/chimera"
  cache_dir="$tmp_dir/cache/chimera"
  runtime_dir="$tmp_dir/runtime"
  bootstrap_env="$config_dir/mesh_bootstrap.env"
  peer_env="$config_dir/peer-egress.env"
  fake_runner="$tmp_dir/fake-runner.sh"
  cli_log="$tmp_dir/cli.log"
  bindings_file="$cache_dir/peer-egress-transit-lane-bindings.csv"

  mkdir -p "$install_root/scripts" "$config_dir" "$cache_dir" "$runtime_dir"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cat >"$bootstrap_env" <<'EOF'
CHIMERA_MESH_NAMESPACE=cef-public
CHIMERA_MESH_LOCAL_NODE=test-local-node
CHIMERA_MESH_TRAFFIC_PROFILE=high_speed_anonymous
CHIMERA_MESH_REMOTE_PEER_SPEC=seed-node@198.51.100.88:443@eu@42@99
EOF
  cat >"$peer_env" <<'EOF'
CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=true
EOF
  cat >"$fake_runner" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >>"$cli_log"
if [[ "\${1:-}" == "cli" ]]; then
  shift
fi
if [[ "\${1:-}" == "mesh" && "\${2:-}" == "route-explain" ]]; then
  while [[ \$# -gt 0 ]]; do
    case "\$1" in
      --transit-lane-bindings-out)
        printf '%s\n' 'lane-a,bound' > "\${2:?}"
        shift 2
        ;;
      *)
        shift
        ;;
    esac
  done
  exit 0
fi
exit 0
EOF
  chmod +x "$fake_runner"

  set +e
  output="$(
    CHIMERA_BOOTSTRAP_ENV_FILE="$bootstrap_env" \
    CHIMERA_MESH_NAMESPACE="cef-public" \
    CHIMERA_MESH_TRAFFIC_PROFILE="high_speed_anonymous" \
    CHIMERA_RUNNER="$fake_runner" \
    CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE="$bindings_file" \
    PEER_EGRESS_ENV_FILE="$peer_env" \
    XDG_CACHE_HOME="$tmp_dir/cache" \
    XDG_CONFIG_HOME="$tmp_dir/config" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    bash -lc 'source "'"$install_root/scripts/chimera-control.sh"'"; mesh_bind_control_plane --strict; cat "'"$peer_env"'"' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "mesh_bind_uses_bootstrap_env_case: bind failed output=$output"
  [[ "$output" == *"peer_egress_transit_lane_bindings_publish=ok"* ]] || fail "mesh_bind_uses_bootstrap_env_case: bind did not publish output=$output"
  [[ -s "$bindings_file" ]] || fail "mesh_bind_uses_bootstrap_env_case: bindings file missing"
  grep -q '^CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE=' "$peer_env" || fail "mesh_bind_uses_bootstrap_env_case: bindings env not persisted"
  grep -q -- 'mesh route-explain --namespace cef-public --node test-local-node --traffic-profile high_speed_anonymous --peer seed-node@198.51.100.88:443@eu@42@99 --transit-lane-bindings-out' "$cli_log" || fail "mesh_bind_uses_bootstrap_env_case: bootstrap env was not used for route explain"

  rm -rf "$tmp_dir"
}

run_mesh_bind_uses_bootstrap_env_case

run_publish_discovery_uses_bootstrap_keyring_case() {
  local tmp_dir install_root config_dir cache_dir runtime_dir bootstrap_env peer_env peer_state update_state fake_runner cli_log output rc

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  config_dir="$tmp_dir/config/chimera"
  cache_dir="$tmp_dir/cache/chimera"
  runtime_dir="$tmp_dir/runtime"
  bootstrap_env="$config_dir/mesh_bootstrap.env"
  peer_env="$config_dir/peer-egress.env"
  peer_state="$cache_dir/peer-egress.state"
  update_state="$cache_dir/peer-update.state.json"
  fake_runner="$tmp_dir/fake-runner.sh"
  cli_log="$tmp_dir/cli.log"

  mkdir -p "$install_root/scripts" "$config_dir" "$cache_dir" "$runtime_dir"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cat >"$bootstrap_env" <<'EOF'
CHIMERA_MESH_NODES_DISCOVERY_KEYRING=default:key-a
EOF
  cat >"$peer_env" <<'EOF'
CHIMERA_PEER_EGRESS_TOKEN=invite-token-123
EOF
  cat >"$peer_state" <<'EOF'
mode=node
resolved_local_listen=127.0.0.1:43169
resolved_peer_listen=0.0.0.0:35609
EOF
  cat >"$update_state" <<'EOF'
{"kind":"chimera_peer_update_serve_state","status":"ready","listen":"0.0.0.0:45833","base_url":"http://198.51.100.44:45833","update_bootstrap_url":"http://198.51.100.44:45833/chimera.sh","version":"0.1.167","sha256":"test-sha","endpoint_epoch":1,"endpoint_generation":1}
EOF
  cat >"$fake_runner" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf 'keyring=%s token=%s self=%s args=%s\n' "\${CHIMERA_MESH_NODES_DISCOVERY_KEYRING:-}" "\${CHIMERA_MESH_ADVERTISE_INVITE_TOKEN:-}" "\${CHIMERA_MESH_SELF_NODE_ID:-}" "\$*" >>"$cli_log"
if [[ "\${1:-}" == "cli" ]]; then
  shift
fi
if [[ "\${1:-}" == "mesh" && "\${2:-}" == "nodes" && "\${3:-}" == "advertise" ]]; then
  out=""
  pubkey=""
  shift 3
  while [[ \$# -gt 0 ]]; do
    case "\$1" in
      --out)
        out="\${2:?}"
        shift 2
        ;;
      --pubkey-out)
        pubkey="\${2:?}"
        shift 2
        ;;
      *)
        shift
        ;;
    esac
  done
  printf '%s\n' '{"ok":true}' > "\$out"
  printf '%s\n' 'pubkey-a' > "\$pubkey"
  exit 0
fi
exit 0
EOF
  chmod +x "$fake_runner"

  set +e
  output="$(
    CHIMERA_BOOTSTRAP_ENV_FILE="$bootstrap_env" \
    CHIMERA_RUNNER="$fake_runner" \
    PEER_EGRESS_ENV_FILE="$peer_env" \
    CHIMERA_PEER_UPDATE_STATE_FILE="$update_state" \
    XDG_CACHE_HOME="$tmp_dir/cache" \
    XDG_CONFIG_HOME="$tmp_dir/config" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    bash -lc 'source "'"$install_root/scripts/chimera-control.sh"'"; publish_mesh_discovery_snapshot; cat "'"$tmp_dir/cache/chimera/mesh_nodes.discovery.json"'"; cat "'"$tmp_dir/cache/chimera/mesh_nodes.discovery.pubkey"'"' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "publish_discovery_uses_bootstrap_keyring_case: publish failed output=$output"
  [[ "$output" == *'{"ok":true}'* ]] || fail "publish_discovery_uses_bootstrap_keyring_case: discovery output missing output=$output"
  [[ "$output" == *'pubkey-a'* ]] || fail "publish_discovery_uses_bootstrap_keyring_case: pubkey output missing output=$output"
  grep -q 'keyring=default:key-a' "$cli_log" || fail "publish_discovery_uses_bootstrap_keyring_case: bootstrap keyring not forwarded"
  grep -q 'token=invite-token-123' "$cli_log" || fail "publish_discovery_uses_bootstrap_keyring_case: invite token not forwarded"

  rm -rf "$tmp_dir"
}

run_publish_discovery_uses_bootstrap_keyring_case

run_publish_discovery_strict_missing_state_clears_stale_case() {
  local tmp_dir install_root cache_dir runtime_dir output rc discovery_out pubkey_out

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  cache_dir="$tmp_dir/cache/chimera"
  runtime_dir="$tmp_dir/runtime"
  discovery_out="$cache_dir/mesh_nodes.discovery.json"
  pubkey_out="$cache_dir/mesh_nodes.discovery.pubkey"

  mkdir -p "$install_root/scripts" "$cache_dir" "$runtime_dir"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  printf '%s\n' '{"stale":true}' >"$discovery_out"
  printf '%s\n' 'stale-pubkey' >"$pubkey_out"

  set +e
  output="$(
    XDG_CACHE_HOME="$tmp_dir/cache" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    bash -lc 'source "'"$install_root/scripts/chimera-control.sh"'"; publish_mesh_discovery_snapshot strict' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -ne 0 ]] || fail "publish_discovery_strict_missing_state_clears_stale_case: expected non-zero rc"
  [[ "$output" == *"discovery_snapshot_publish=skipped reason=peer_state_missing"* ]] || fail "publish_discovery_strict_missing_state_clears_stale_case: missing peer_state_missing output=$output"
  [[ ! -e "$discovery_out" ]] || fail "publish_discovery_strict_missing_state_clears_stale_case: stale discovery file not removed"
  [[ ! -e "$pubkey_out" ]] || fail "publish_discovery_strict_missing_state_clears_stale_case: stale pubkey file not removed"

  rm -rf "$tmp_dir"
}

run_publish_discovery_strict_missing_state_clears_stale_case

run_site_auto_watch_publishes_discovery_case() {
  local tmp_dir install_root watch_log output rc

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  watch_log="$tmp_dir/watch.log"

  mkdir -p "$install_root/scripts"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"

  set +e
  output="$(
    WATCH_LOG="$watch_log" \
    bash -lc '
      source "'"$install_root/scripts/chimera-control.sh"'"
      site_auto_discover_run() { echo discover >>"$WATCH_LOG"; }
      site_auto_bootstrap_run() { echo bootstrap >>"$WATCH_LOG"; }
      publish_peer_egress_transit_lane_bindings_from_control_plane() { echo bindings >>"$WATCH_LOG"; }
      publish_mesh_discovery_snapshot() { echo discovery >>"$WATCH_LOG"; }
      site_auto_watch_run_once
      cat "$WATCH_LOG"
    ' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "site_auto_watch_publishes_discovery_case: watch run failed output=$output"
  [[ "$output" == *"site_auto_watch_run_once=ok"* ]] || fail "site_auto_watch_publishes_discovery_case: status missing output=$output"
  grep -Fxq 'discover' "$watch_log" || fail "site_auto_watch_publishes_discovery_case: discover step missing"
  grep -Fxq 'bootstrap' "$watch_log" || fail "site_auto_watch_publishes_discovery_case: bootstrap step missing"
  grep -Fxq 'bindings' "$watch_log" || fail "site_auto_watch_publishes_discovery_case: bindings publish missing"
  grep -Fxq 'discovery' "$watch_log" || fail "site_auto_watch_publishes_discovery_case: discovery publish missing"

  rm -rf "$tmp_dir"
}

run_site_auto_watch_publishes_discovery_case

run_site_auto_watch_reports_publication_failure_case() {
  local tmp_dir install_root watch_log output rc

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  watch_log="$tmp_dir/watch.log"
  mkdir -p "$install_root/scripts"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"

  set +e
  output="$(
    WATCH_LOG="$watch_log" \
    bash -lc '
      source "'"$install_root/scripts/chimera-control.sh"'"
      site_auto_discover_run() { echo discover >>"$WATCH_LOG"; }
      site_auto_bootstrap_run() { echo bootstrap >>"$WATCH_LOG"; }
      publish_peer_egress_transit_lane_bindings_from_control_plane() { echo bindings >>"$WATCH_LOG"; }
      publish_mesh_discovery_snapshot() { echo discovery >>"$WATCH_LOG"; return 1; }
      site_auto_watch_run_once
      cat "$WATCH_LOG"
    ' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -ne 0 ]] || fail "site_auto_watch_reports_publication_failure_case: expected non-zero rc"
  [[ "$output" == *"site_auto_watch_run_once=partial"* ]] || fail "site_auto_watch_reports_publication_failure_case: partial status missing output=$output"
  [[ "$output" == *"discovery_publish=failed"* ]] || fail "site_auto_watch_reports_publication_failure_case: discovery failure detail missing output=$output"
  grep -Fxq 'discover' "$watch_log" || fail "site_auto_watch_reports_publication_failure_case: discover step missing"
  grep -Fxq 'bootstrap' "$watch_log" || fail "site_auto_watch_reports_publication_failure_case: bootstrap step missing"
  grep -Fxq 'bindings' "$watch_log" || fail "site_auto_watch_reports_publication_failure_case: bindings publish missing"
  grep -Fxq 'discovery' "$watch_log" || fail "site_auto_watch_reports_publication_failure_case: discovery publish missing"

  rm -rf "$tmp_dir"
}

run_site_auto_watch_reports_publication_failure_case

run_site_auto_watch_uses_strict_publication_for_bound_transit_case() {
  local tmp_dir install_root watch_log output rc

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  watch_log="$tmp_dir/watch.log"
  mkdir -p "$install_root/scripts"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"

  set +e
  output="$(
    WATCH_LOG="$watch_log" \
    bash -lc '
      source "'"$install_root/scripts/chimera-control.sh"'"
      peer_egress_bound_transit_requested() { return 0; }
      site_auto_discover_run() { :; }
      site_auto_bootstrap_run() { :; }
      refresh_node_peer_target_from_bootstrap() { :; }
      publish_peer_egress_transit_lane_bindings_from_control_plane() { printf "bindings_mode=%s\n" "${1:-}" >>"$WATCH_LOG"; return 1; }
      publish_mesh_discovery_snapshot() { printf "discovery_mode=%s\n" "${1:-}" >>"$WATCH_LOG"; }
      site_auto_watch_run_once
    ' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -ne 0 ]] || fail "site_auto_watch_uses_strict_publication_for_bound_transit_case: expected non-zero rc"
  [[ "$output" == *"site_auto_watch_run_once=partial"* ]] || fail "site_auto_watch_uses_strict_publication_for_bound_transit_case: missing partial status output=$output"
  [[ "$output" == *"transit_lane_bindings_publish=failed"* ]] || fail "site_auto_watch_uses_strict_publication_for_bound_transit_case: missing bindings failure output=$output"
  grep -Fxq 'bindings_mode=strict' "$watch_log" || fail "site_auto_watch_uses_strict_publication_for_bound_transit_case: bindings did not use strict mode"
  grep -Fxq 'discovery_mode=strict' "$watch_log" || fail "site_auto_watch_uses_strict_publication_for_bound_transit_case: discovery did not use strict mode"

  rm -rf "$tmp_dir"
}

run_site_auto_watch_uses_strict_publication_for_bound_transit_case

run_refresh_runtime_publication_reports_bound_transit_failure_case() {
  local tmp_dir install_root output rc

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  mkdir -p "$install_root/scripts"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"

  set +e
  output="$(
    bash -lc '
      source "'"$install_root/scripts/chimera-control.sh"'"
      BINDINGS_MODE_LOG=""
      peer_update_runtime_configured() { return 1; }
      peer_egress_bound_transit_requested() { return 0; }
      mesh_discovery_source_present() { return 1; }
      publish_peer_egress_transit_lane_bindings_from_control_plane() { BINDINGS_MODE_LOG="${1:-}"; return 1; }
      set +e
      refresh_runtime_publication_after_node_start
      rc=$?
      printf "bindings_mode=%s\n" "$BINDINGS_MODE_LOG"
      printf "bindings_status=%s\n" "$START_RUNTIME_TRANSIT_LANE_BINDINGS_STATUS"
      exit "$rc"
    ' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -ne 0 ]] || fail "refresh_runtime_publication_reports_bound_transit_failure_case: expected non-zero rc"
  [[ "$output" == *"bindings_mode=strict"* ]] || fail "refresh_runtime_publication_reports_bound_transit_failure_case: strict mode missing output=$output"
  [[ "$output" == *"bindings_status=failed"* ]] || fail "refresh_runtime_publication_reports_bound_transit_failure_case: failed status missing output=$output"

  rm -rf "$tmp_dir"
}

run_refresh_runtime_publication_reports_bound_transit_failure_case

run_site_auto_watch_loop_escalates_after_repeated_failure_case() {
  local tmp_dir install_root watch_log output rc

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  watch_log="$tmp_dir/watch.log"
  mkdir -p "$install_root/scripts"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"

  set +e
  output="$(
    WATCH_LOG="$watch_log" \
    SITE_AUTOWATCH_INTERVAL_SEC=0 \
    SITE_AUTOWATCH_FAILURE_BUDGET=2 \
    timeout 5s bash -lc '
      set -euo pipefail
      source "'"$install_root/scripts/chimera-control.sh"'"
      peer_update_runtime_configured() { return 1; }
      site_auto_discover_run() { printf "%s\n" discover >>"$WATCH_LOG"; return 0; }
      site_auto_bootstrap_run() { printf "%s\n" bootstrap >>"$WATCH_LOG"; return 0; }
      publish_peer_egress_transit_lane_bindings_from_control_plane() { printf "%s\n" bindings >>"$WATCH_LOG"; return 0; }
      publish_mesh_discovery_snapshot() { printf "%s\n" discovery >>"$WATCH_LOG"; return 1; }
      site_auto_watch_loop
    ' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -ne 124 ]] || fail "site_auto_watch_loop_escalates_after_repeated_failure_case: watch loop timed out"
  [[ "$rc" -ne 0 ]] || fail "site_auto_watch_loop_escalates_after_repeated_failure_case: expected non-zero rc"
  [[ "$output" == *"site_auto_watch_loop=fail consecutive_failures=2 failure_budget=2"* ]] || fail "site_auto_watch_loop_escalates_after_repeated_failure_case: missing failure budget output=$output"
  [[ "$(grep -c '^discover$' "$watch_log" 2>/dev/null || true)" == "2" ]] || fail "site_auto_watch_loop_escalates_after_repeated_failure_case: discover should run twice"
  [[ "$(grep -c '^bootstrap$' "$watch_log" 2>/dev/null || true)" == "2" ]] || fail "site_auto_watch_loop_escalates_after_repeated_failure_case: bootstrap should run twice"
  [[ "$(grep -c '^bindings$' "$watch_log" 2>/dev/null || true)" == "2" ]] || fail "site_auto_watch_loop_escalates_after_repeated_failure_case: bindings should run twice"
  [[ "$(grep -c '^discovery$' "$watch_log" 2>/dev/null || true)" == "2" ]] || fail "site_auto_watch_loop_escalates_after_repeated_failure_case: discovery should run twice"

  rm -rf "$tmp_dir"
}

run_site_auto_watch_loop_escalates_after_repeated_failure_case

run_start_rejects_direct_fallback_when_systemd_units_present_case() {
  local tmp_dir bin_dir cache_dir config_dir runtime_dir install_root node_conf fake_runner runner_log output rc

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  install_root="$tmp_dir/chimera-release"
  node_conf="$tmp_dir/mesh-node.conf"
  fake_runner="$tmp_dir/fake-runner.sh"
  runner_log="$tmp_dir/runner.log"

  mkdir -p "$bin_dir" "$cache_dir/chimera" "$config_dir/chimera" "$config_dir/systemd/user" "$runtime_dir" "$install_root/scripts" "$install_root/configs"
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

  cat >"$bin_dir/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  --user) shift ;;
esac
case "${1:-}" in
  show-environment) exit 1 ;;
  stop|list-units|list-unit-files|daemon-reload) exit 0 ;;
  *) exit 0 ;;
esac
EOF
  chmod +x "$bin_dir/systemctl"

  cat >"$fake_runner" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >>"$runner_log"
exit 0
EOF
  chmod +x "$fake_runner"

  printf '%s\n' '[Unit]' >"$config_dir/systemd/user/chimera-runtime.service"

  set +e
  output="$(
    PATH="$bin_dir:$PATH" \
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$cache_dir" \
    XDG_CONFIG_HOME="$config_dir" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    NODE_CONFIG_FILE="$node_conf" \
    CHIMERA_RUNNER="$fake_runner" \
    CHIMERA_UPDATE_FIRST_CHECKED=1 \
    SITE_AUTOWATCH_ENABLED=0 \
    timeout 20s bash "$install_root/scripts/chimera-control.sh" start 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 2 ]] || fail "start_rejects_direct_fallback_when_systemd_units_present_case: expected rc=2 output=$output"
  [[ "$output" == *"start_status=fail mode=preflight"* ]] || fail "start_rejects_direct_fallback_when_systemd_units_present_case: missing preflight fail output=$output"
  [[ "$output" == *"reason=user_systemd_session_unavailable"* ]] || fail "start_rejects_direct_fallback_when_systemd_units_present_case: missing systemd session reason output=$output"
  [[ "$output" == *"units_on_disk=true"* ]] || fail "start_rejects_direct_fallback_when_systemd_units_present_case: missing units_on_disk output=$output"
  [[ ! -s "$runner_log" ]] || fail "start_rejects_direct_fallback_when_systemd_units_present_case: runner should not execute"

  rm -rf "$tmp_dir"
}

run_start_rejects_direct_fallback_when_systemd_units_present_case

run_mesh_launch_preflight_prefers_published_runtime_state_over_snapshot_case() {
  local tmp_dir fake_bin just_log cargo_marker output rc
  local side_a_env side_b_env side_a_discovery side_b_discovery side_a_state side_b_state

  tmp_dir="$(mktemp -d)"
  fake_bin="$tmp_dir/bin"
  just_log="$tmp_dir/just.log"
  cargo_marker="$tmp_dir/cargo.called"
  side_a_env="$tmp_dir/side_a.env"
  side_b_env="$tmp_dir/side_b.env"
  side_a_discovery="$tmp_dir/side_a.discovery.json"
  side_b_discovery="$tmp_dir/side_b.discovery.json"
  side_a_state="$tmp_dir/side_a.state.json"
  side_b_state="$tmp_dir/side_b.state.json"

  mkdir -p "$fake_bin"
  cat >"$fake_bin/just" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" > "$just_log"
exit 0
EOF
  chmod +x "$fake_bin/just"
  cat >"$fake_bin/cargo" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' called > "$cargo_marker"
exit 99
EOF
  chmod +x "$fake_bin/cargo"

  cat >"$side_a_env" <<'EOF'
CHIMERA_MESH_LOCAL_NODE=node-a
EOF
  cat >"$side_b_env" <<'EOF'
CHIMERA_MESH_LOCAL_NODE=node-b
EOF
  cat >"$side_a_discovery" <<'EOF'
{"nodes":[{"node_id":"node-a","endpoint":"11.0.0.10:1111"}],"signature":"sig","key_id":"key-a"}
EOF
  cat >"$side_b_discovery" <<'EOF'
{"nodes":[{"node_id":"node-b","endpoint":"12.0.0.20:3333"}],"signature":"sig","key_id":"key-a"}
EOF
  cat >"$side_a_state" <<'EOF'
{"node_id":"node-a","published_endpoint":"11.0.0.10:2222"}
EOF
  cat >"$side_b_state" <<'EOF'
{"node_id":"node-b","published_endpoint":"12.0.0.20:4444"}
EOF

  set +e
  output="$(
    PATH="$fake_bin:$PATH" \
    CHIMERA_MESH_SIDE_A_ENV_FILE="$side_a_env" \
    CHIMERA_MESH_SIDE_B_ENV_FILE="$side_b_env" \
    CHIMERA_MESH_SIDE_A_DISCOVERY_SNAPSHOT="$side_a_discovery" \
    CHIMERA_MESH_SIDE_B_DISCOVERY_SNAPSHOT="$side_b_discovery" \
    CHIMERA_MESH_SIDE_A_PUBLISHED_RUNTIME_STATE="$side_a_state" \
    CHIMERA_MESH_SIDE_B_PUBLISHED_RUNTIME_STATE="$side_b_state" \
    bash "$ROOT_DIR/scripts/mesh_launch_preflight_auto_bind.sh" 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "mesh_launch_preflight_prefers_published_runtime_state_over_snapshot_case: script failed output=$output"
  [[ "$output" == *"selected side_b endpoint 12.0.0.20:4444"* ]] || fail "mesh_launch_preflight_prefers_published_runtime_state_over_snapshot_case: published runtime state did not win output=$output"
  [[ -f "$just_log" ]] || fail "mesh_launch_preflight_prefers_published_runtime_state_over_snapshot_case: just was not called"
  grep -Fxq 'mesh-launch-preflight-set-real-endpoints 12.0.0.20:4444 11.0.0.10:2222' "$just_log" || fail "mesh_launch_preflight_prefers_published_runtime_state_over_snapshot_case: just received wrong endpoints"
  [[ ! -f "$cargo_marker" ]] || fail "mesh_launch_preflight_prefers_published_runtime_state_over_snapshot_case: inventory fallback unexpectedly ran"

  rm -rf "$tmp_dir"
}

run_mesh_launch_preflight_prefers_published_runtime_state_over_snapshot_case

run_node_service_poststart_waits_for_datapath_proof_case() {
  local tmp_dir install_root watch_log output rc

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  watch_log="$tmp_dir/watch.log"
  mkdir -p "$install_root/scripts"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"

  set +e
  output="$(
    WATCH_LOG="$watch_log" \
    bash -lc 'source "'"$install_root/scripts/chimera-control.sh"'";
      datapath_apply_proof_ok() { return 1; }
      site_auto_watch_run_once() { echo run_once >>"'"$watch_log"'"; }
      site_auto_watch_start() { echo start >>"'"$watch_log"'"; }
      node_service_poststart_reconcile' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "node_service_poststart_waits_for_datapath_proof_case: helper failed output=$output"
  [[ "$output" == *"node_poststart_reconcile=deferred"* ]] || fail "node_service_poststart_waits_for_datapath_proof_case: missing deferred status output=$output"
  [[ ! -f "$watch_log" ]] || fail "node_service_poststart_waits_for_datapath_proof_case: watch should not run before proof"

  rm -rf "$tmp_dir"
}

run_node_service_poststart_waits_for_datapath_proof_case

run_clear_runtime_generated_state_clears_transit_lane_bindings_case() {
  local tmp_dir install_root config_dir lane_file peer_env output rc

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  config_dir="$tmp_dir/config/chimera"
  lane_file="$tmp_dir/cache/chimera/lanes.csv"
  peer_env="$config_dir/peer-egress.env"
  mkdir -p "$install_root/scripts" "$config_dir" "$(dirname "$lane_file")"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cat >"$peer_env" <<EOF
CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE=$lane_file
EOF
  printf '%s\n' lane >"$lane_file"

  set +e
  output="$(
    PEER_EGRESS_ENV_FILE="$peer_env" \
    CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE="$lane_file" \
    bash -lc 'source "'"$install_root/scripts/chimera-control.sh"'"; clear_runtime_generated_state; cat "'"$peer_env"'" 2>/dev/null || true' 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "clear_runtime_generated_state_clears_transit_lane_bindings_case: helper failed output=$output"
  [[ ! -f "$lane_file" ]] || fail "clear_runtime_generated_state_clears_transit_lane_bindings_case: lane file not removed"
  [[ "$output" != *"CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE="* ]] || fail "clear_runtime_generated_state_clears_transit_lane_bindings_case: env pointer not removed"

  rm -rf "$tmp_dir"
}

run_clear_runtime_generated_state_clears_transit_lane_bindings_case

run_node_service_preflight_heals_blocked_fixed_listener_without_ss_case() {
  local tmp_dir bin_dir cache_dir config_dir runtime_dir install_root node_conf peer_env override_file output rc

  tmp_dir="$(mktemp -d)"
  bin_dir="$tmp_dir/bin"
  cache_dir="$tmp_dir/cache"
  config_dir="$tmp_dir/config"
  runtime_dir="$tmp_dir/runtime"
  install_root="$tmp_dir/chimera-release"
  node_conf="$tmp_dir/mesh-node.conf"
  peer_env="$config_dir/chimera/peer-egress.env"
  override_file="$cache_dir/chimera/runtime_listener_overrides.env"

  mkdir -p "$bin_dir" "$cache_dir/chimera" "$config_dir/chimera" "$runtime_dir" "$install_root/scripts" "$install_root/configs"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cp "$ROOT_DIR/configs/mesh-node.example.conf" "$install_root/configs/mesh-node.example.conf"
  cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = carrier.mesh:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
peer.listen_addr = 9443
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
  cat >"$peer_env" <<'EOF'
CHIMERA_PEER_EGRESS_MODE=node
CHIMERA_PEER_EGRESS_SERVER=carrier.mesh:443
CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:9444
CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:9443
EOF
  cat >"$bin_dir/ss" <<'EOF'
#!/usr/bin/env bash
exit 127
EOF
  cat >"$bin_dir/lsof" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$*" == *":9443"* || "$*" == *":9444"* ]]; then
  exit 0
fi
exit 1
EOF
  chmod +x "$bin_dir/ss" "$bin_dir/lsof"

  set +e
  output="$(
    PATH="$bin_dir:$PATH" \
    NODE_CONFIG_FILE="$node_conf" \
    PEER_EGRESS_ENV_FILE="$peer_env" \
    RUNTIME_LISTENER_OVERRIDE_FILE="$override_file" \
    XDG_CACHE_HOME="$cache_dir" \
    XDG_CONFIG_HOME="$config_dir" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    timeout 20s bash "$install_root/scripts/chimera-control.sh" __service-preflight-node 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "node_service_preflight_heals_blocked_fixed_listener_without_ss_case: helper failed rc=$rc output=$output"
  grep -Fxq 'CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:18135' "$override_file" || fail "node_service_preflight_heals_blocked_fixed_listener_without_ss_case: runtime local override missing"
  grep -Fxq 'CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:0' "$override_file" || fail "node_service_preflight_heals_blocked_fixed_listener_without_ss_case: runtime peer override missing"

  rm -rf "$tmp_dir"
}

run_node_service_preflight_heals_blocked_fixed_listener_without_ss_case

run_mesh_launch_preflight_falls_back_to_published_runtime_state_case() {
  local tmp_dir fake_bin just_log cargo_marker output rc
  local side_a_env side_b_env side_a_state side_b_state

  tmp_dir="$(mktemp -d)"
  fake_bin="$tmp_dir/bin"
  just_log="$tmp_dir/just.log"
  cargo_marker="$tmp_dir/cargo.called"
  side_a_env="$tmp_dir/side_a.env"
  side_b_env="$tmp_dir/side_b.env"
  side_a_state="$tmp_dir/side_a.state.json"
  side_b_state="$tmp_dir/side_b.state.json"

  mkdir -p "$fake_bin"
  cat >"$fake_bin/just" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" > "$just_log"
exit 0
EOF
  chmod +x "$fake_bin/just"
  cat >"$fake_bin/cargo" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' called > "$cargo_marker"
exit 99
EOF
  chmod +x "$fake_bin/cargo"

  cat >"$side_a_env" <<'EOF'
CHIMERA_MESH_LOCAL_NODE=node-a
EOF
  cat >"$side_b_env" <<'EOF'
CHIMERA_MESH_LOCAL_NODE=node-b
EOF
  cat >"$side_a_state" <<'EOF'
{"node_id":"node-a","published_endpoint":"11.0.0.10:2222"}
EOF
  cat >"$side_b_state" <<'EOF'
{"node_id":"node-b","published_endpoint":"12.0.0.20:4444"}
EOF

  set +e
  output="$(
    PATH="$fake_bin:$PATH" \
    CHIMERA_MESH_SIDE_A_ENV_FILE="$side_a_env" \
    CHIMERA_MESH_SIDE_B_ENV_FILE="$side_b_env" \
    CHIMERA_MESH_SIDE_A_PUBLISHED_RUNTIME_STATE="$side_a_state" \
    CHIMERA_MESH_SIDE_B_PUBLISHED_RUNTIME_STATE="$side_b_state" \
    bash "$ROOT_DIR/scripts/mesh_launch_preflight_auto_bind.sh" 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "mesh_launch_preflight_falls_back_to_published_runtime_state_case: script failed output=$output"
  [[ "$output" == *"selected side_b endpoint 12.0.0.20:4444"* ]] || fail "mesh_launch_preflight_falls_back_to_published_runtime_state_case: published runtime state did not win output=$output"
  [[ -f "$just_log" ]] || fail "mesh_launch_preflight_falls_back_to_published_runtime_state_case: just was not called"
  grep -Fxq 'mesh-launch-preflight-set-real-endpoints 12.0.0.20:4444 11.0.0.10:2222' "$just_log" || fail "mesh_launch_preflight_falls_back_to_published_runtime_state_case: just received wrong endpoints"
  [[ ! -f "$cargo_marker" ]] || fail "mesh_launch_preflight_falls_back_to_published_runtime_state_case: inventory fallback unexpectedly ran"

  rm -rf "$tmp_dir"
}

run_mesh_launch_preflight_falls_back_to_published_runtime_state_case

run_mesh_launch_preflight_inventory_without_cargo_case() {
  local tmp_dir fake_bin just_log cargo_marker runner_log side_a_env side_b_env mesh_nodes_config output rc

  tmp_dir="$(mktemp -d)"
  fake_bin="$tmp_dir/bin"
  just_log="$tmp_dir/just.log"
  cargo_marker="$tmp_dir/cargo.was.used"
  runner_log="$tmp_dir/runner.log"
  side_a_env="$tmp_dir/side_a.env"
  side_b_env="$tmp_dir/side_b.env"
  mesh_nodes_config="$tmp_dir/mesh_nodes.conf"
  mkdir -p "$fake_bin"

  cat >"$side_a_env" <<'EOF'
CHIMERA_MESH_LOCAL_NODE=node-a
EOF
  cat >"$side_b_env" <<'EOF'
CHIMERA_MESH_LOCAL_NODE=node-b
EOF
  printf '%s\n' '# test config' >"$mesh_nodes_config"

  cat >"$fake_bin/just" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >>"$just_log"
EOF
  chmod +x "$fake_bin/just"

  cat >"$fake_bin/cargo" <<EOF
#!/usr/bin/env bash
set -euo pipefail
touch "$cargo_marker"
exit 99
EOF
  chmod +x "$fake_bin/cargo"

  cat >"$tmp_dir/fake-runner.sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >>"$runner_log"
if [[ "\${1:-}" == "cli" ]]; then
  shift
fi
case "\${1:-} \${2:-} \${3:-}" in
  "mesh nodes select")
    runtime_state=""
    node_id=""
    prev=""
    for arg in "\$@"; do
      if [[ "\$prev" == "--runtime-state" ]]; then
        runtime_state="\$arg"
      elif [[ "\$prev" == "--id" ]]; then
        node_id="\$arg"
      fi
      prev="\$arg"
    done
    [[ -n "\$runtime_state" && -n "\$node_id" ]] || exit 2
    printf '%s\n' "\$node_id" >"\$runtime_state"
    exit 0
    ;;
  "mesh nodes selected-endpoint")
    runtime_state=""
    prev=""
    for arg in "\$@"; do
      if [[ "\$prev" == "--runtime-state" ]]; then
        runtime_state="\$arg"
      fi
      prev="\$arg"
    done
    node_id="\$(cat "\$runtime_state" 2>/dev/null || true)"
    case "\$node_id" in
      node-a) printf '%s\n' '11.0.0.10:1111' ;;
      node-b) printf '%s\n' '12.0.0.20:3333' ;;
      *) exit 2 ;;
    esac
    exit 0
    ;;
esac
exit 2
EOF
  chmod +x "$tmp_dir/fake-runner.sh"

  set +e
  output="$(
    PATH="$fake_bin:$PATH" \
    CHIMERA_RUNNER="$tmp_dir/fake-runner.sh" \
    CHIMERA_MESH_NODES_CONFIG="$mesh_nodes_config" \
    CHIMERA_MESH_SIDE_A_ENV_FILE="$side_a_env" \
    CHIMERA_MESH_SIDE_B_ENV_FILE="$side_b_env" \
    bash "$ROOT_DIR/scripts/mesh_launch_preflight_auto_bind.sh" 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "mesh_launch_preflight_inventory_without_cargo_case: script failed output=$output"
  [[ "$output" == *"selected side_b endpoint 12.0.0.20:3333"* ]] || fail "mesh_launch_preflight_inventory_without_cargo_case: wrong side_b endpoint output=$output"
  [[ -f "$just_log" ]] || fail "mesh_launch_preflight_inventory_without_cargo_case: just was not called"
  grep -Fxq 'mesh-launch-preflight-set-real-endpoints 12.0.0.20:3333 11.0.0.10:1111' "$just_log" || fail "mesh_launch_preflight_inventory_without_cargo_case: just received wrong endpoints"
  [[ ! -f "$cargo_marker" ]] || fail "mesh_launch_preflight_inventory_without_cargo_case: cargo fallback was used"

  rm -rf "$tmp_dir"
}

run_mesh_launch_preflight_inventory_without_cargo_case

echo "chimera_start_contract_smoke=pass"
