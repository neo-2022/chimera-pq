#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BLOCKED_PORT="18142"

fail() {
  echo "chimera_port_conflict_recovery_smoke: $1" >&2
  exit 1
}

tmp_dir="$(mktemp -d)"
bin_dir="$tmp_dir/bin"
cache_dir="$tmp_dir/cache"
config_dir="$tmp_dir/config"
runtime_dir="$tmp_dir/runtime"
install_root="$tmp_dir/chimera-release"
node_conf="$tmp_dir/mesh-node.conf"
peer_env="$config_dir/chimera/peer-egress.env"
autofix_log="$cache_dir/chimera/autofix.log"
override_env="$cache_dir/chimera/runtime_listener_overrides.env"

mkdir -p "$bin_dir" "$cache_dir/chimera" "$config_dir/chimera" "$runtime_dir" \
  "$install_root/scripts"

cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
cp "$ROOT_DIR/scripts/chimera-control-cleanup.inc" "$install_root/scripts/chimera-control-cleanup.inc"
# Hermetic synthetic release fixture; keep session-process-guard independent of
# a prior build_release.sh run. The value is not asserted against real releases.
smoke_release_version="0.1.86"
printf '%s\n' "$smoke_release_version" >"$install_root/.chimera_release_version"

# Node config with a fixed peer listen port to trigger the conflict path.
cat >"$node_conf" <<EOF
carrier.profile = in-memory
carrier.addr = 203.0.113.10:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
peer.listen_addr = 0.0.0.0:$BLOCKED_PORT
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF

# Peer egress env with the same fixed port.
cat >"$peer_env" <<EOF
CHIMERA_PEER_EGRESS_MODE=node
CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:18135
CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:$BLOCKED_PORT
CHIMERA_PEER_EGRESS_STATE_FILE=$cache_dir/chimera/peer-egress.state
CHIMERA_MESH_PEER_EGRESS_STATE_PATH=$cache_dir/chimera/peer-egress.state
CHIMERA_PEER_UPDATE_STATE_FILE=$cache_dir/chimera/peer-update.state.json
CHIMERA_PEER_EGRESS_TOKEN=test-token
CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=true
EOF

# Fake systemctl: pretend all units start and stay active.
cat >"$bin_dir/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  --user) shift ;;
esac
case "${1:-}" in
  show-environment|daemon-reload|start|stop|enable|disable|is-enabled|list-units|list-unit-files)
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
chmod +x "$bin_dir/systemctl"

# Fake ss: report the blocked port as listening when scanning TCP listeners.
cat >"$bin_dir/ss" <<EOF
#!/usr/bin/env bash
set -euo pipefail
if [[ "\${*:-}" == *"-H -ltn"* ]]; then
  printf 'LISTEN 0 4096 0.0.0.0:$BLOCKED_PORT 0.0.0.0:*\n'
fi
exit 0
EOF
chmod +x "$bin_dir/ss"

set +e
output="$(
  PATH="$bin_dir:$PATH" \
  HOME="$tmp_dir/home" \
  XDG_CACHE_HOME="$cache_dir" \
  XDG_CONFIG_HOME="$config_dir" \
  XDG_RUNTIME_DIR="$runtime_dir" \
  NODE_CONFIG_FILE="$node_conf" \
  CHIMERA_UPDATE_FIRST_CHECKED=1 \
  CHIMERA_FAIL_CLOSED_ON_PARTIAL_START=0 \
  timeout 20s bash "$install_root/scripts/chimera-control.sh" start 2>&1
)"
rc=$?
set -e

[[ "$rc" -ne 124 ]] || fail "smoke timed out before contract result"
[[ -f "$override_env" ]] || fail "runtime listener override file not created; output=$output"
[[ "$(cat "$override_env")" == *"CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:0"* ]] \
  || fail "peer listen override did not reset to auto-listen"
[[ "$(cat "$override_env")" == *"CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:18135"* ]] \
  || fail "local listen override did not reset to auto-listen"
[[ -f "$autofix_log" ]] || fail "autofix log not created"
[[ "$(cat "$autofix_log")" == *"node_listener_reset"* ]] \
  || fail "autofix log missing node_listener_reset event"

# The env file itself should retain the original fixed port (repair uses runtime
# overrides, not the persisted env), and the node config remains unchanged.
[[ -f "$peer_env" ]] || fail "peer egress env file missing"
grep -q "^CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:$BLOCKED_PORT$" "$peer_env" \
  || fail "peer egress env was unexpectedly mutated"
grep -q "peer.listen_addr = 0.0.0.0:$BLOCKED_PORT$" "$node_conf" \
  || fail "node config was unexpectedly mutated"

rm -rf "$tmp_dir"

echo "chimera_port_conflict_recovery_smoke=pass"
