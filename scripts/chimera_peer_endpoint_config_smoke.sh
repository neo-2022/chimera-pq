#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "chimera_peer_endpoint_config_smoke: $1" >&2
  exit 1
}

tmp_dir="$(mktemp -d)"
# shellcheck disable=SC2064
trap 'rm -rf "$tmp_dir"' EXIT

bin_dir="$tmp_dir/bin"
config_a="$tmp_dir/config-a"
config_b="$tmp_dir/config-b"
cache_a="$tmp_dir/cache-a"
cache_b="$tmp_dir/cache-b"
home_a="$tmp_dir/home-a"
home_b="$tmp_dir/home-b"
install_a="$tmp_dir/install-a"
install_b="$tmp_dir/install-b"
peer_port="19142"
peer_endpoint="127.0.0.1:$peer_port"

mkdir -p "$bin_dir" \
  "$config_a/chimera" "$cache_a/chimera" "$home_a/.local/bin" "$home_a/.local/share/applications" \
  "$config_b/chimera" "$cache_b/chimera" "$home_b/.local/bin" "$home_b/.local/share/applications"

# Copy a minimal install root.
for root in "$install_a" "$install_b"; do
  mkdir -p "$root/scripts" "$root/bin" "$root/deploy/systemd-user" "$root/configs"
  cp -r "$ROOT_DIR/scripts" "$root/"
  cp -r "$ROOT_DIR/deploy" "$root/"
  cp -r "$ROOT_DIR/configs" "$root/"
  printf '%s\n' "0.1.170" >"$root/.chimera_release_version"
  printf '%s\n' "b35795d0b0852c61204488f297953dfcdc816172a551facaa658fea22f9d2426" >"$root/.chimera_release_bundle.sha256"
done

# Fake systemctl/loginctl (boot recovery is not the focus here).
cat >"$bin_dir/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
cfg="${XDG_CONFIG_HOME:-${HOME:-}/.config}"
wants="$cfg/systemd/user/default.target.wants"
case "${1:-}" in --user) shift;; esac
cmd="${1:-}"; unit="${2:-}"
case "$cmd" in
  show-environment|daemon-reload|list-units|list-unit-files|start|stop) exit 0;;
  enable) mkdir -p "$wants"; ln -sfn "../$unit" "$wants/$unit"; exit 0;;
  disable) rm -f "$wants/$unit"; exit 0;;
  is-enabled) [[ -L "$wants/$unit" ]] && echo enabled || echo disabled; exit 0;;
  is-active) echo active; exit 0;;
  *) exit 0;;
esac
EOF
chmod +x "$bin_dir/systemctl"

cat >"$bin_dir/loginctl" <<'EOF'
#!/usr/bin/env bash
case "${1:-}" in enable-linger) exit 0;; show-user) echo "Linger=yes"; exit 0;; *) exit 0;; esac
EOF
chmod +x "$bin_dir/loginctl"

# Fake chimera-cli for selected-invite-token.
cat >"$install_a/bin/chimera-cli" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "mesh" && "${2:-}" == "nodes" && "${3:-}" == "selected-invite-token" ]]; then
  echo "test-token-a"; exit 0
fi
exit 0
EOF
chmod +x "$install_a/bin/chimera-cli"

cat >"$install_b/bin/chimera-cli" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "mesh" && "${2:-}" == "nodes" && "${3:-}" == "selected-invite-token" ]]; then
  echo "test-token-b"; exit 0
fi
exit 0
EOF
chmod +x "$install_b/bin/chimera-cli"

# Install node A: listener-only, fixed peer port.
if ! env \
  HOME="$home_a" \
  XDG_CONFIG_HOME="$config_a" \
  XDG_CACHE_HOME="$cache_a" \
  PATH="$bin_dir:$PATH" \
  CHIMERA_NODE_PEER_LISTEN_ADDR="0.0.0.0:$peer_port" \
  CHIMERA_PEER_EGRESS_TOKEN="test-token-a" \
  bash "$install_a/scripts/install_desktop_control.sh" >"$tmp_dir/install-a.log" 2>&1; then
  cat "$tmp_dir/install-a.log" >&2
  fail "node A installer failed"
fi

# Verify node A configured itself listener-only.
[[ -f "$config_a/chimera/peer-egress.env" ]] || fail "node A peer-egress.env missing"
grep -q "^CHIMERA_PEER_EGRESS_PEER_LISTEN=0\.0\.0\.0:$peer_port\$" "$config_a/chimera/peer-egress.env" \
  || fail "node A peer listen not configured; $(cat "$config_a/chimera/peer-egress.env")"

# Install node B: should consume node A's endpoint and configure peer egress.
if ! env \
  HOME="$home_b" \
  XDG_CONFIG_HOME="$config_b" \
  XDG_CACHE_HOME="$cache_b" \
  PATH="$bin_dir:$PATH" \
  CHIMERA_NODE_ENDPOINT="$peer_endpoint" \
  CHIMERA_PEER_EGRESS_TOKEN="test-token-b" \
  bash "$install_b/scripts/install_desktop_control.sh" >"$tmp_dir/install-b.log" 2>&1; then
  cat "$tmp_dir/install-b.log" >&2
  fail "node B installer failed"
fi

# Verify node B configured peer endpoint in node config and peer egress env.
node_b_conf="$install_b/configs/mesh-node.conf"
peer_b_env="$config_b/chimera/peer-egress.env"

[[ -f "$node_b_conf" ]] || fail "node B mesh-node.conf missing"
[[ -f "$peer_b_env" ]] || fail "node B peer-egress.env missing"

# The installer normalizes the endpoint into the node config; it may add a
# tcp:// prefix depending on the carrier profile.
grep -Eq "^(carrier\.addr *= *(tcp://)?$peer_endpoint|carrier\.addr *= *$peer_endpoint)\$" "$node_b_conf" \
  || fail "node B carrier.addr not set to peer endpoint; $(cat "$node_b_conf")"
grep -q "^CHIMERA_PEER_EGRESS_SERVER=$peer_endpoint\$" "$peer_b_env" \
  || fail "node B peer egress server not set; $(cat "$peer_b_env")"
# Node B should have received a valid listen address (auto or explicit).
grep -Eq "^peer\.listen_addr *= *(0\.0\.0\.0:[0-9]+|auto)\$" "$node_b_conf" \
  || fail "node B peer listen not configured; $(cat "$node_b_conf")"

echo "chimera_peer_endpoint_config_smoke=pass"
