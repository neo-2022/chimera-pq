#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "chimera_reboot_persistence_smoke: $1" >&2
  exit 1
}

tmp_dir="$(mktemp -d)"
# shellcheck disable=SC2064
trap 'rm -rf "$tmp_dir"' EXIT

bin_dir="$tmp_dir/bin"
config_dir="$tmp_dir/config"
cache_dir="$tmp_dir/cache"
runtime_dir="$tmp_dir/runtime"
install_root="$tmp_dir/chimera-release"
home_dir="$tmp_dir/home"
unit_state_dir="$config_dir/systemd/user/default.target.wants"
active_units_file="$cache_dir/active-units.txt"
started_units_file="$cache_dir/started-units.txt"
# Hermetic synthetic release fixtures; keep session-process-guard independent of
# a prior build_release.sh run. The values are not asserted against real releases.
smoke_release_version="0.1.86"
smoke_release_bundle_sha="$(printf '%064d' 1)"

mkdir -p "$bin_dir" "$config_dir/chimera" "$cache_dir/chimera" \
  "$runtime_dir" "$install_root/bin" "$unit_state_dir" "$home_dir/.local/bin" \
  "$home_dir/.local/share/applications"

# Copy the runtime tree.
cp -r "$ROOT_DIR/scripts" "$install_root/"
cp -r "$ROOT_DIR/deploy" "$install_root/"
# Copy all example configs so the installer can seed defaults.
cp -r "$ROOT_DIR/configs" "$install_root/"
if [[ -d "$ROOT_DIR/bin" ]]; then
  cp -r "$ROOT_DIR/bin" "$install_root/"
fi

# Version/sha files expected by chimera-sh.
printf '%s\n' "$smoke_release_version" >"$install_root/.chimera_release_version"
printf '%s\n' "$smoke_release_bundle_sha" >"$install_root/.chimera_release_bundle.sha256"

# Fake chimera-cli: returns invite token for selected-invite-token, no-ops otherwise.
cat >"$install_root/bin/chimera-cli" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "mesh" && "${2:-}" == "nodes" && "${3:-}" == "selected-invite-token" ]]; then
  printf '%s\n' "test-token"
  exit 0
fi
exit 0
EOF
chmod +x "$install_root/bin/chimera-cli"

# Fake systemctl that records unit state.
cat >"$bin_dir/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
home="${HOME:-}"
cfg_root="${XDG_CONFIG_HOME:-$home/.config}"
wants_dir="$cfg_root/systemd/user/default.target.wants"
unit_dir="$cfg_root/systemd/user"
active_file="${CHIMERA_FAKE_SYSTEMCTL_ACTIVE_FILE:-/dev/null}"
started_file="${CHIMERA_FAKE_SYSTEMCTL_STARTED_FILE:-/dev/null}"
cache_root="${XDG_CACHE_HOME:-$home/.cache}"
case "${1:-}" in
  --user) shift ;;
esac
cmd="${1:-}"
unit="${2:-}"
# Strip common options so $unit holds the actual unit name.
while [[ "$unit" == --* ]]; do
  shift
  unit="${2:-}"
done
ensure_dirs() { mkdir -p "$wants_dir" "$cache_root/chimera"; }
record_active() {
  ensure_dirs
  touch "$active_file"
  if ! grep -qx "$1" "$active_file" 2>/dev/null; then
    printf '%s\n' "$1" >>"$active_file"
  fi
}
record_started() {
  ensure_dirs
  touch "$started_file"
  if ! grep -qx "$1" "$started_file" 2>/dev/null; then
    printf '%s\n' "$1" >>"$started_file"
  fi
}
is_active() {
  local u="$1"
  # Accept 'is-active --quiet <unit>'
  if [[ "$u" == --* ]]; then
    shift; u="${2:-}"
  fi
  if [[ -n "$u" && -f "$active_file" ]] && grep -qx "$u" "$active_file"; then
    echo "active"
    return 0
  fi
  echo "inactive"
  return 3
}
case "$cmd" in
  show-environment|daemon-reload|enable|disable|list-units|list-unit-files)
    if [[ "$cmd" == "enable" && -n "${unit:-}" ]]; then
      ensure_dirs
      ln -sfn "../$unit" "$wants_dir/$unit"
    fi
    if [[ "$cmd" == "disable" && -n "${unit:-}" ]]; then
      rm -f "$wants_dir/$unit"
    fi
    exit 0
    ;;
  start)
    if [[ -z "${unit:-}" ]]; then exit 1; fi
    record_started "$unit"
    # Keep node/datapath/runtime "active" so the orchestrator sees a stable boot.
    record_active "$unit"
    touch "$cache_root/chimera/${unit%.service}.log"
    exit 0
    ;;
  stop)
    if [[ -n "${unit:-}" && -f "$active_file" ]]; then
      grep -vx "$unit" "$active_file" >"$active_file.tmp" || true
      mv "$active_file.tmp" "$active_file"
    fi
    exit 0
    ;;
  is-active)
    is_active "$unit"
    ;;
  is-enabled)
    if [[ -L "$wants_dir/$unit" ]]; then
      echo "enabled"
    else
      echo "disabled"
    fi
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
EOF
chmod +x "$bin_dir/systemctl"

# Fake loginctl.
cat >"$bin_dir/loginctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  enable-linger) exit 0 ;;
  show-user)
    echo "Linger=yes"
    exit 0
    ;;
  *) exit 0 ;;
esac
EOF
chmod +x "$bin_dir/loginctl"

# Run install with boot recovery explicitly enabled.
install_log="$tmp_dir/install.log"
if ! env \
  HOME="$home_dir" \
  XDG_CONFIG_HOME="$config_dir" \
  XDG_CACHE_HOME="$cache_dir" \
  XDG_RUNTIME_DIR="$runtime_dir" \
  PATH="$bin_dir:$PATH" \
  CHIMERA_INSTALL_ENABLE_BOOT_RECOVERY=true \
  CHIMERA_PEER_EGRESS_TOKEN=test-token \
  CHIMERA_FAKE_SYSTEMCTL_ACTIVE_FILE="$active_units_file" \
  CHIMERA_FAKE_SYSTEMCTL_STARTED_FILE="$started_units_file" \
  CHIMERA_RUNTIME_SERVICE_UNIT=chimera-runtime.service \
  CHIMERA_UPDATE_FIRST_CHECKED=1 \
  bash "$install_root/scripts/install_desktop_control.sh" >"$install_log" 2>&1; then
  cat "$install_log" >&2
  fail "installer failed"
fi

# Verify the runtime unit was installed and enabled for boot.
[[ -f "$config_dir/systemd/user/chimera-runtime.service" ]] || fail "runtime unit not installed"
[[ -L "$unit_state_dir/chimera-runtime.service" ]] || fail "runtime unit not enabled for boot"

# Simulate a reboot: wipe transient runtime state but keep installed unit + config.
rm -rf "$runtime_dir"/* 2>/dev/null || true
rm -f "$cache_dir/chimera"/*.log \
      "$cache_dir/chimera/peer-egress.state" \
      "$cache_dir/chimera/peer-update.state.json" \
      "$cache_dir/chimera/mesh_nodes.discovery.json" \
      "$cache_dir/chimera/runtime_state_latest.json" 2>/dev/null || true
rm -f "$active_units_file" "$started_units_file" 2>/dev/null || true
mkdir -p "$runtime_dir" "$cache_dir/chimera"

# Mark runtime as active because we are about to invoke its ExecStart (boot semantic).
mkdir -p "$cache_dir"
printf '%s\n' "chimera-runtime.service" >"$active_units_file"

# Invoke the runtime unit ExecStart exactly as installed.
set +e
output="$(
  cd "$install_root" && \
  HOME="$home_dir" \
  XDG_CONFIG_HOME="$config_dir" \
  XDG_CACHE_HOME="$cache_dir" \
  XDG_RUNTIME_DIR="$runtime_dir" \
  PATH="$bin_dir:$PATH" \
  CHIMERA_FAIL_CLOSED_ON_PARTIAL_START=0 \
  CHIMERA_FAKE_SYSTEMCTL_ACTIVE_FILE="$active_units_file" \
  CHIMERA_FAKE_SYSTEMCTL_STARTED_FILE="$started_units_file" \
  CHIMERA_UPDATE_FIRST_CHECKED=1 \
  CHIMERA_CLI_BIN="$install_root/bin/chimera-cli" \
  timeout 30s /usr/bin/env bash -lc 'exec ./scripts/chimera-control.sh start' 2>&1
)"
rc=$?
set -e

[[ "$rc" -ne 124 ]] || fail "smoke timed out before contract result"
# With CHIMERA_FAIL_CLOSED_ON_PARTIAL_START=0 the listener-only node returns 0.
[[ "$rc" -eq 0 ]] || fail "unexpected start exit=$rc output=$output"

[[ "$output" == *"start_status=partial"* ]] || fail "expected partial start status; output=$output"
[[ "$output" == *"reason=node_endpoint_unconfigured_listener_only"* ]] || fail "expected listener-only reason; output=$output"

# Query runtime status and verify the boot recovery contract.
set +e
status_output="$(
  cd "$install_root" && \
  HOME="$home_dir" \
  XDG_CONFIG_HOME="$config_dir" \
  XDG_CACHE_HOME="$cache_dir" \
  XDG_RUNTIME_DIR="$runtime_dir" \
  PATH="$bin_dir:$PATH" \
  CHIMERA_FAKE_SYSTEMCTL_ACTIVE_FILE="$active_units_file" \
  CHIMERA_FAKE_SYSTEMCTL_STARTED_FILE="$started_units_file" \
  bash ./scripts/chimera-control.sh status 2>&1
)"
status_rc=$?
set -e
[[ "$status_rc" -eq 0 ]] || fail "status command failed rc=$status_rc output=$status_output"

[[ "$status_output" == *"runtime_boot_service_state=active"* ]] || fail "runtime not active: $status_output"
[[ "$status_output" == *"runtime_boot_enabled_state=enabled"* ]] || fail "runtime not enabled: $status_output"
[[ "$status_output" == *"node_service_state=active"* ]] || fail "node service not active: $status_output"
[[ "$status_output" == *"node_runtime=running"* ]] || fail "node runtime not running: $status_output"
[[ "$status_output" == *"runtime_state_status=up"* ]] || fail "runtime state not up: $status_output"
[[ "$status_output" == *"node_config_ready=false"* ]] || fail "expected unconfigured listener-only node: $status_output"

[[ -f "$started_units_file" ]] || fail "no units were started"
grep -qx "chimera-node.service" "$started_units_file" || fail "node unit was not started during boot"

echo "chimera_reboot_persistence_smoke=pass"
