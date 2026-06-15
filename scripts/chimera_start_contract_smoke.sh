#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "chimera_start_contract_smoke: $1" >&2
  exit 1
}

run_case() {
  local case_name="$1"
  local client_ready="$2"
  local systemctl_mode="$3"
  local tmp_dir systemctl_dir cache_dir config_dir runtime_dir client_conf output rc install_root

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
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cp "$ROOT_DIR/scripts/install_desktop_control.sh" "$install_root/scripts/install_desktop_control.sh"
  cp "$ROOT_DIR/scripts/chimera_runtime_bootstrap.sh" "$install_root/scripts/chimera_runtime_bootstrap.sh"
  cp "$ROOT_DIR/deploy/systemd-user/chimera-gateway.service" "$install_root/deploy/systemd-user/chimera-gateway.service"
  cp "$ROOT_DIR/deploy/systemd-user/chimera-client.service" "$install_root/deploy/systemd-user/chimera-client.service"
  cp "$ROOT_DIR/deploy/desktop/chimera-control-gui.desktop" "$install_root/deploy/desktop/chimera-control-gui.desktop"
  cp "$ROOT_DIR/configs/client.example.conf" "$install_root/configs/client.example.conf"
  cp "$ROOT_DIR/configs/gateway.example.conf" "$install_root/configs/gateway.example.conf"
  printf '%s\n' "0.1.86" >"$install_root/.chimera_release_version"
  printf '%064d\n' 1 >"$install_root/.chimera_release_bundle.sha256"

  client_conf="$tmp_dir/client.conf"
  if [[ "$client_ready" == "1" ]]; then
    cat >"$client_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = 198.51.100.10:443
carrier.server_name = gateway.local
capture.mode = tun
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
  else
    cat >"$client_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = 203.0.113.10:443
carrier.server_name = gateway.local
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
gateway_log="$cache_root/chimera/chimera_gateway.service.log"
client_log="$cache_root/chimera/chimera_client.service.log"
mode="__MODE__"
count_dir="${TMPDIR:-/tmp}/chimera-start-contract-counts"
mkdir -p "$count_dir"
case "\${1:-}" in
  --user)
    shift
    ;;
esac
case "\${1:-}" in
  show-environment|daemon-reload)
    exit 0
    ;;
  start)
    if [[ ! -f "\$gateway_log" || ! -f "\$client_log" ]]; then
      exit 209
    fi
    exit 0
    ;;
  is-active)
    local_unit="${2:-}"
    case "\$mode" in
      node_flap)
        if [[ "$local_unit" == "chimera-gateway.service" ]]; then
          count_file="$count_dir/node_flap.count"
          count="0"
          if [[ -f "$count_file" ]]; then
            read -r count <"$count_file" 2>/dev/null || count="0"
          fi
          count=$((count + 1))
          printf '%s\n' "$count" >"$count_file"
          if (( count <= 2 )); then
            exit 0
          fi
          exit 3
        fi
        ;;
    esac
    if [[ "$mode" == "node_fail" && "${2:-}" == "chimera-gateway.service" ]]; then
      exit 3
    fi
    if [[ "$mode" == "client_fail" && "${2:-}" == "chimera-client.service" ]]; then
      exit 3
    fi
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

  set +e
  output="$(
	    PATH="$systemctl_dir:$PATH" \
	    HOME="$tmp_dir/home" \
	    XDG_CACHE_HOME="$tmp_dir/cache" \
	    XDG_CONFIG_HOME="$config_dir" \
	    XDG_RUNTIME_DIR="$runtime_dir" \
	    CLIENT_CONFIG_FILE="$client_conf" \
	    CHIMERA_UPDATE_BOOTSTRAP_URL="https://127.0.0.1.invalid/chimera.sh" \
	    CHIMERA_AUTOFIX_MAX_TIME=0 \
	    bash "$install_root/scripts/chimera-sh" -start 2>&1
	  )"
  rc=$?
  set -e

  [[ "$rc" -ne 0 ]] || fail "$case_name: expected non-zero exit"
  [[ "$output" == *"start_status=fail"* ]] || fail "$case_name: missing fail status"
  [[ "$output" != *"start_status=ok"* ]] || fail "$case_name: false ok status leaked"
  [[ "$output" == *"systemctl_start_rc=0"* ]] || fail "$case_name: systemctl did not see prepared log targets"
  [[ "$output" == *"reason=node_service_failed"* || "$output" == *"reason=transparent_service_failed"* ]] || fail "$case_name: missing failure reason"
  [[ -f "$cache_dir/chimera_gateway.service.log" ]] || fail "$case_name: gateway log file missing"
  [[ -f "$cache_dir/chimera_client.service.log" ]] || fail "$case_name: client log file missing"

  rm -rf "$tmp_dir"
}

run_case "node_service_failure" "1" "node_fail"
run_case "node_flap_failure" "1" "node_flap"
run_case "client_service_failure" "1" "client_fail"

echo "chimera_start_contract_smoke=pass"
