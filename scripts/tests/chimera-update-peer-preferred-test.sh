#!/usr/bin/env bash
# Peer-preferred update authority tests for chimera-update.sh
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=scripts/chimera-update.sh
source "$ROOT_DIR/scripts/chimera-update.sh"

fail=0
pass=0

assert_eq() {
  local expected="$1"
  local actual="$2"
  local label="${3:-}"
  if [[ "$expected" == "$actual" ]]; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    echo "FAIL: $label: expected '$expected', got '$actual'" >&2
  fi
}

assert_true() {
  local label="$1"
  shift
  if "$@"; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    echo "FAIL: $label returned non-zero" >&2
  fi
}

assert_false() {
  local label="$1"
  shift
  if "$@"; then
    fail=$((fail + 1))
    echo "FAIL: $label returned zero" >&2
  else
    pass=$((pass + 1))
  fi
}

# release_version_to_sortable
assert_eq "000001000002000003" "$(release_version_to_sortable "1.2.3")" "release_version_to_sortable 1.2.3"
assert_eq "000000000001000009" "$(release_version_to_sortable "0.1.9")" "release_version_to_sortable 0.1.9"
assert_eq "000000000001001000" "$(release_version_to_sortable "0.1.1000")" "release_version_to_sortable 0.1.1000"
assert_eq "000000000001000222" "$(release_version_to_sortable "v0.1.222")" "release_version_to_sortable v0.1.222"

# is_remote_newer: strict upgrade only, never downgrade
assert_true "upgrade patch" is_remote_newer "0.1.222" "0.1.223"
assert_true "upgrade minor" is_remote_newer "0.1.999" "0.2.0"
assert_true "upgrade with v prefix" is_remote_newer "0.1.222" "v0.1.223"
assert_false "same version not newer" is_remote_newer "0.1.222" "0.1.222"
assert_false "downgrade not newer" is_remote_newer "0.1.223" "0.1.222"
assert_true "large patch" is_remote_newer "0.1.999" "0.1.1000"

# mesh_connect_args_from_launcher_args extracts -connect args
connect_args="$(mesh_connect_args_from_launcher_args -connect peer-one --token x)" || true
assert_eq "peer-one
--token
x" "$connect_args" "mesh_connect_args_from_launcher_args -connect"
other_args="$(mesh_connect_args_from_launcher_args restart)" || true
assert_eq "" "$other_args" "mesh_connect_args_from_launcher_args non-connect"

# update_source_trust_rank ordering
assert_eq "1" "$(update_source_trust_rank github)" "trust rank github"
assert_eq "2" "$(update_source_trust_rank gitvers)" "trust rank gitvers"
assert_eq "3" "$(update_source_trust_rank peer)" "trust rank peer"

# register_update_source_version: consensus on same version
reset_update_source_authority
assert_true "register github current" register_update_source_version "github" "0.1.222"
assert_true "register gitvers same version" register_update_source_version "gitvers" "0.1.222"
assert_eq "github" "$UPDATE_AUTHORITY_SOURCE" "authority stays with highest trust source"
assert_eq "0.1.222" "$UPDATE_AUTHORITY_VERSION" "authority version preserved"

# Divergence between same-trust sources is a block
reset_update_source_authority
register_update_source_version "github" "0.1.222" || true
assert_false "divergent gitvers blocked" register_update_source_version "gitvers" "0.1.223"

# Higher-trust source overrides authority
reset_update_source_authority
register_update_source_version "peer" "0.1.222" || true
assert_true "github overrides peer" register_update_source_version "github" "0.1.223"
assert_eq "github" "$UPDATE_AUTHORITY_SOURCE" "authority upgraded to github"
assert_eq "0.1.223" "$UPDATE_AUTHORITY_VERSION" "authority version upgraded"

# Checksum authority consensus
reset_update_source_authority
register_update_source_version "github" "0.1.222" || true
register_update_source_checksum "github" "abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234" || true
assert_true "checksum same version ok" register_update_source_checksum "gitvers" "abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234"

reset_update_source_authority
register_update_source_version "github" "0.1.222" || true
register_update_source_checksum "github" "abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234" || true
assert_false "checksum divergence blocked" register_update_source_checksum "gitvers" "0000000000000000000000000000000000000000000000000000000000000000"

# Stale lower-trust source diverges when explicitly registered; production code
# guards this by only registering current-or-newer candidate sources.
reset_update_source_authority
register_update_source_version "github" "0.1.222" || true
assert_false "stale peer explicitly registered is blocked" register_update_source_version "peer" "0.1.221"

echo "chimera-update-peer-preferred-test: pass=$pass fail=$fail"
exit "$fail"
