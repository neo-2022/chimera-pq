#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "mesh control-plane env smoke: $1" >&2
  exit 1
}

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

export HOME="$tmp_dir/home"
export XDG_CONFIG_HOME="$tmp_dir/config"
export XDG_CACHE_HOME="$tmp_dir/cache"
export XDG_RUNTIME_DIR="$tmp_dir/runtime"
mkdir -p "$XDG_CONFIG_HOME/chimera" "$XDG_CACHE_HOME/chimera" "$XDG_RUNTIME_DIR"

export CHIMERA_MESH_NAMESPACE="cef-public"
export CHIMERA_MESH_LOCAL_NODE="node-a"
export CHIMERA_MESH_REMOTE_NODE="node-b"
export CHIMERA_MESH_REMOTE_ENDPOINT="198.51.100.22:443"
export CHIMERA_MESH_REMOTE_REGION="eu"
export CHIMERA_MESH_REMOTE_LOAD_SCORE="20"
export CHIMERA_MESH_REMOTE_RELIABILITY_SCORE="90"
export CHIMERA_MESH_POLICY_PAYLOAD="mesh_allowed_regions=eu;mesh_multipath_mode=flow_shard;mesh_route_binding_id=7401"

bash "$ROOT_DIR/scripts/mesh_control_plane_env_from_preflight.sh" >/dev/null
control_plane_env="$XDG_CONFIG_HOME/chimera/mesh-control-plane.env"
[[ -f "$control_plane_env" ]] || fail "control-plane env was not created"
grep -q '^CHIMERA_MESH_REMOTE_PEER_SPEC=' "$control_plane_env" \
  || fail "control-plane env missing remote peer spec"

skip_file="$tmp_dir/profile-only-control-plane.env"
skip_output="$(
  unset CHIMERA_MESH_POLICY_PAYLOAD
  export CHIMERA_MESH_TRAFFIC_PROFILE="high_speed_anonymous"
  bash "$ROOT_DIR/scripts/mesh_control_plane_env_from_preflight.sh" "$skip_file"
)"
[[ "$skip_output" == *"mesh_control_plane_env=skipped reason=missing_route_binding_id"* ]] \
  || fail "profile-only control-plane env did not skip safely: $skip_output"
[[ ! -f "$skip_file" ]] || fail "profile-only control-plane env created a false handoff"

peer_env="$XDG_CONFIG_HOME/chimera/peer-egress.env"
printf '%s\n' 'CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=true' >"$peer_env"
chmod 600 "$peer_env"

fake_runner="$tmp_dir/chimera-runner.sh"
cat >"$fake_runner" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "cli" ]]; then
  shift
fi

out_file=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --transit-lane-bindings-out)
      out_file="${2:-}"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done

if [[ -n "$out_file" ]]; then
  mkdir -p "$(dirname "$out_file")"
  cat >"$out_file" <<'DOC'
# chimera_transit_lane_document=v1
# chimera_plan_snapshot=v1
# chimera_plan_namespace=cef-public
# chimera_plan_mode=flow_shard
# chimera_plan_route_binding_id=7401
7401,0,198.51.100.22:443
DOC
fi
exit 0
EOF
chmod +x "$fake_runner"

output="$(
  CHIMERA_RUNNER="$fake_runner" \
  bash -c 'source "$1"; publish_peer_egress_transit_lane_bindings_from_control_plane' \
    _ "$ROOT_DIR/scripts/chimera-control.sh" 2>&1
)"

[[ "$output" == *"peer_egress_transit_lane_bindings_publish=ok"* ]] \
  || fail "publish did not report ok: $output"
grep -q '^CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE=' "$peer_env" \
  || fail "peer env missing generated lane bindings path"
lane_file="$XDG_CACHE_HOME/chimera/peer-egress-transit-lane-bindings.csv"
[[ -s "$lane_file" ]] || fail "lane bindings file missing"
grep -q '^7401,0,198.51.100.22:443$' "$lane_file" \
  || fail "lane bindings file content mismatch"

echo "mesh control-plane env smoke: PASS"
