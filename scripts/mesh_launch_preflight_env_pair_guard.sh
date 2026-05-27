#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: mesh_launch_preflight_env_pair_guard.sh <side_a_env_file> <side_b_env_file>"
  exit 2
fi

SIDE_A_ENV="$1"
SIDE_B_ENV="$2"
if [[ ! -f "$SIDE_A_ENV" ]]; then
  echo "mesh launch preflight env pair guard: missing side_a env file: $SIDE_A_ENV"
  exit 1
fi
if [[ ! -f "$SIDE_B_ENV" ]]; then
  echo "mesh launch preflight env pair guard: missing side_b env file: $SIDE_B_ENV"
  exit 1
fi

set -a
# shellcheck disable=SC1090
source "$SIDE_A_ENV"
set +a
V_ROLE="${CHIMERA_MESH_LOCAL_ROLE-}"
V_NS="${CHIMERA_MESH_NAMESPACE-}"
V_LOCAL_NODE="${CHIMERA_MESH_LOCAL_NODE-}"
V_REMOTE_NODE="${CHIMERA_MESH_REMOTE_NODE-}"
V_LOCAL_OUT="${CHIMERA_MESH_LOCAL_OUT-}"
V_REMOTE_OUT="${CHIMERA_MESH_REMOTE_OUT-}"

set -a
# shellcheck disable=SC1090
source "$SIDE_B_ENV"
set +a
L_ROLE="${CHIMERA_MESH_LOCAL_ROLE-}"
L_NS="${CHIMERA_MESH_NAMESPACE-}"
L_LOCAL_NODE="${CHIMERA_MESH_LOCAL_NODE-}"
L_REMOTE_NODE="${CHIMERA_MESH_REMOTE_NODE-}"
L_LOCAL_OUT="${CHIMERA_MESH_LOCAL_OUT-}"
L_REMOTE_OUT="${CHIMERA_MESH_REMOTE_OUT-}"

if [[ "$V_ROLE" != "side_a" ]]; then
  echo "mesh launch preflight env pair guard: side_a env role must be side_a"
  exit 1
fi
if [[ "$L_ROLE" != "side_b" ]]; then
  echo "mesh launch preflight env pair guard: side_b env role must be side_b"
  exit 1
fi
if [[ -z "${V_NS// }" || -z "${L_NS// }" ]]; then
  echo "mesh launch preflight env pair guard: namespace must be non-empty in both env files"
  exit 1
fi
if [[ "$V_NS" != "$L_NS" ]]; then
  echo "mesh launch preflight env pair guard: namespace mismatch between env files"
  exit 1
fi
if [[ "$V_LOCAL_NODE" != "$L_REMOTE_NODE" ]]; then
  echo "mesh launch preflight env pair guard: side_a local node must match side_b remote node"
  exit 1
fi
if [[ "$L_LOCAL_NODE" != "$V_REMOTE_NODE" ]]; then
  echo "mesh launch preflight env pair guard: side_b local node must match side_a remote node"
  exit 1
fi
if [[ "$V_LOCAL_OUT" != "$L_REMOTE_OUT" ]]; then
  echo "mesh launch preflight env pair guard: side_a local out must match side_b remote out"
  exit 1
fi
if [[ "$L_LOCAL_OUT" != "$V_REMOTE_OUT" ]]; then
  echo "mesh launch preflight env pair guard: side_b local out must match side_a remote out"
  exit 1
fi

echo "mesh launch preflight env pair guard: PASS"
