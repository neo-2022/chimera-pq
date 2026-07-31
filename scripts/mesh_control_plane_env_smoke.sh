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
profile_output="$(
  unset CHIMERA_MESH_POLICY_PAYLOAD
  export CHIMERA_MESH_TRAFFIC_PROFILE="high_speed_anonymous"
  bash "$ROOT_DIR/scripts/mesh_control_plane_env_from_preflight.sh" "$skip_file"
)"
[[ "$profile_output" == *"mesh_control_plane_env=ok"* ]] \
  || fail "profile-only control-plane env did not materialize: $profile_output"
[[ -f "$skip_file" ]] || fail "profile-only control-plane env was not created"
grep -q '^CHIMERA_MESH_TRAFFIC_PROFILE=high_speed_anonymous$' "$skip_file" \
  || fail "profile-only control-plane env missing traffic profile"

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

bootstrap_env="$XDG_CONFIG_HOME/chimera/mesh_bootstrap.env"
cat >"$bootstrap_env" <<'EOF'
CHIMERA_MESH_NAMESPACE=cef-public
CHIMERA_MESH_TRAFFIC_PROFILE=high_speed_anonymous
CHIMERA_MESH_REMOTE_PEER_SPEC=node-b@198.51.100.22:443@eu@20@90
EOF
chmod 600 "$bootstrap_env"

partial_control_plane_env="$tmp_dir/partial-control-plane.env"
cat >"$partial_control_plane_env" <<'EOF'
CHIMERA_MESH_LOCAL_NODE=node-from-control-plane
EOF

printf '%s\n' 'CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=true' >"$peer_env"
rm -f "$lane_file"
partial_output="$(
  unset CHIMERA_MESH_NAMESPACE \
    CHIMERA_MESH_LOCAL_NODE \
    CHIMERA_MESH_REMOTE_NODE \
    CHIMERA_MESH_REMOTE_ENDPOINT \
    CHIMERA_MESH_REMOTE_REGION \
    CHIMERA_MESH_REMOTE_LOAD_SCORE \
    CHIMERA_MESH_REMOTE_RELIABILITY_SCORE \
    CHIMERA_MESH_REMOTE_PEER_SPEC \
    CHIMERA_MESH_EXTRA_PEERS \
    CHIMERA_MESH_POLICY_PAYLOAD \
    CHIMERA_MESH_TRAFFIC_PROFILE
  CHIMERA_RUNNER="$fake_runner" \
  PEER_EGRESS_ENV_FILE="$peer_env" \
  CHIMERA_MESH_CONTROL_PLANE_ENV_FILE="$partial_control_plane_env" \
  bash -c 'source "$1"; publish_peer_egress_transit_lane_bindings_from_control_plane strict' \
    _ "$ROOT_DIR/scripts/chimera-control.sh" 2>&1
)"

[[ "$partial_output" == *"peer_egress_transit_lane_bindings_publish=ok"* ]] \
  || fail "partial control-plane env did not fall back to bootstrap: $partial_output"
[[ -s "$lane_file" ]] || fail "partial control-plane env bootstrap fallback did not create lane file"
rm -f "$bootstrap_env"

printf '%s\n' 'CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=true' >"$peer_env"
rm -f "$lane_file"
command_output="$(
  CHIMERA_RUNNER="$fake_runner" \
  PEER_EGRESS_ENV_FILE="$peer_env" \
  CHIMERA_MESH_CONTROL_PLANE_ENV_FILE="$control_plane_env" \
  bash "$ROOT_DIR/scripts/chimera-control.sh" mesh-bind-control-plane --strict 2>&1
)"
[[ "$command_output" == *"mesh_control_plane_env=ok"* ]] \
  || fail "strict command did not generate control-plane env: $command_output"
[[ "$command_output" == *"peer_egress_transit_lane_bindings_publish=ok"* ]] \
  || fail "strict command did not publish lane bindings: $command_output"
grep -q '^CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE=' "$peer_env" \
  || fail "strict command did not write lane bindings env"
[[ -s "$lane_file" ]] || fail "strict command lane bindings file missing"

printf '%s\n' 'CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=true' >"$peer_env"
missing_peer_env="$tmp_dir/missing-peer-egress.env"
set +e
missing_peer_output="$(
  CHIMERA_RUNNER="$fake_runner" \
  PEER_EGRESS_ENV_FILE="$missing_peer_env" \
  CHIMERA_MESH_CONTROL_PLANE_ENV_FILE="$control_plane_env" \
  bash "$ROOT_DIR/scripts/chimera-control.sh" mesh-bind-control-plane --strict 2>&1
)"
missing_peer_rc=$?
set -e
[[ "$missing_peer_rc" -ne 0 ]] \
  || fail "strict command unexpectedly accepted missing peer env"
[[ "$missing_peer_output" == *"peer_egress_transit_lane_bindings_publish=skipped reason=peer_env_missing"* ]] \
  || fail "strict command missing peer env reason mismatch: $missing_peer_output"

printf '%s\n' 'CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=true' >"$peer_env"
malicious_control_plane_env="$tmp_dir/malicious-control-plane.env"
marker="$tmp_dir/control-plane-env-injection-marker"
cat >"$malicious_control_plane_env" <<EOF
CHIMERA_MESH_NAMESPACE=\$(touch "$marker")
CHIMERA_MESH_LOCAL_NODE=node-a
CHIMERA_MESH_POLICY_PAYLOAD=mesh_route_binding_id=7401
CHIMERA_MESH_REMOTE_PEER_SPEC=node-b@198.51.100.22:443@eu@20@90
EOF
set +e
malicious_output="$(
  CHIMERA_RUNNER="$fake_runner" \
  PEER_EGRESS_ENV_FILE="$peer_env" \
  CHIMERA_MESH_CONTROL_PLANE_ENV_FILE="$malicious_control_plane_env" \
  bash -c 'source "$1"; publish_peer_egress_transit_lane_bindings_from_control_plane strict' \
    _ "$ROOT_DIR/scripts/chimera-control.sh" 2>&1
)"
malicious_rc=$?
set -e
[[ "$malicious_rc" -ne 0 ]] \
  || fail "strict publish unexpectedly accepted malicious control-plane env"
[[ "$malicious_output" == *"peer_egress_transit_lane_bindings_publish=skipped reason=invalid_control_plane_env"* ]] \
  || fail "malicious control-plane env reason mismatch: $malicious_output"
[[ ! -e "$marker" ]] \
  || fail "malicious control-plane env executed shell syntax"

stale_lane_file="$XDG_CACHE_HOME/chimera/stale-transit-lane-bindings.csv"
mkdir -p "$(dirname "$stale_lane_file")"
cat >"$stale_lane_file" <<'DOC'
# chimera_transit_lane_document=v1
# chimera_plan_snapshot=v1
# chimera_plan_namespace=cef-public
# chimera_plan_mode=flow_shard
# chimera_plan_route_binding_id=7001
7001,0,198.51.100.99:443
DOC
{
  printf '%s\n' 'CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=true'
  printf 'CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE=%q\n' "$stale_lane_file"
} >"$peer_env"
rm -f "$lane_file"
stale_output="$(
  CHIMERA_RUNNER="$fake_runner" \
  PEER_EGRESS_ENV_FILE="$peer_env" \
  CHIMERA_MESH_CONTROL_PLANE_ENV_FILE="$control_plane_env" \
  bash "$ROOT_DIR/scripts/chimera-control.sh" mesh-bind-control-plane --strict 2>&1
)"
[[ "$stale_output" == *"peer_egress_transit_lane_bindings_publish=ok"* ]] \
  || fail "strict command did not replace stale lane bindings: $stale_output"
grep -q '^CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE=' "$peer_env" \
  || fail "strict command removed lane bindings env"
grep -q '^7401,0,198.51.100.22:443$' "$lane_file" \
  || fail "strict command did not regenerate current lane file"

printf '%s\n' 'CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=true' >"$peer_env"
strict_skip_env="$tmp_dir/strict-skip-control-plane.env"
set +e
strict_skip_output="$(
  unset CHIMERA_MESH_POLICY_PAYLOAD
  export CHIMERA_MESH_TRAFFIC_PROFILE=""
  CHIMERA_RUNNER="$fake_runner" \
  PEER_EGRESS_ENV_FILE="$peer_env" \
  CHIMERA_MESH_CONTROL_PLANE_ENV_FILE="$strict_skip_env" \
  bash "$ROOT_DIR/scripts/chimera-control.sh" mesh-bind-control-plane --strict 2>&1
)"
strict_skip_rc=$?
set -e
[[ "$strict_skip_rc" -ne 0 ]] \
  || fail "strict command unexpectedly accepted missing route binding"
[[ "$strict_skip_output" == *"mesh_control_plane_env=skipped reason=missing_authoritative_policy"* ]] \
  || fail "strict command route binding reason mismatch: $strict_skip_output"
[[ ! -f "$strict_skip_env" ]] \
  || fail "strict command created false control-plane handoff"

echo "mesh control-plane env smoke: PASS"
