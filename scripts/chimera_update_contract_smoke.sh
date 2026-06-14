#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "chimera_update_contract_smoke: $1" >&2
  exit 1
}

source "$ROOT_DIR/scripts/chimera-sh"

case_update_sources_unreachable_continues() (
  read_local_runtime_version() { printf '%s\n' 0.1.0; }
  read_local_runtime_bundle_sha() { printf '%s\n' deadbeef; }
  load_update_peer_bootstrap_urls() { return 0; }
  try_update_from_bootstrap_source() { return 2; }

  local output rc
  output="$(auto_update_if_needed -start 2>&1)"
  rc=$?
  [[ "$rc" -eq 0 ]] || fail "expected rc=0 for update-source outage, got $rc"
  [[ "$output" == *"chimera_update=unavailable"* ]] || fail "missing unavailable diagnostic"
)

case_update_required_install_failure_blocks() (
  read_local_runtime_version() { printf '%s\n' 0.1.0; }
  read_local_runtime_bundle_sha() { printf '%s\n' deadbeef; }
  load_update_peer_bootstrap_urls() { return 0; }
  try_update_from_bootstrap_source() { return 3; }

  local output rc
  output="$(auto_update_if_needed -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  [[ "$rc" -ne 0 ]] || fail "expected non-zero rc when update is required but install fails"
  [[ "$output" != *"chimera_update=unavailable"* ]] || fail "unexpected outage diagnostic for real update failure"
)

case_semver_update_order() (
  is_remote_newer 0.1.84 0.1.85 || fail "expected 0.1.85 newer than 0.1.84"
  is_remote_newer v0.1.84 0.1.85 || fail "expected v-prefixed local version support"
  if is_remote_newer 0.1.85 0.1.84; then
    fail "older remote version accepted as newer"
  fi
  is_remote_newer 20260614-010203 0.1.85 || fail "invalid legacy local version should allow stable semver update"
  if is_remote_newer 0.1.84 20260614-010203; then
    fail "invalid remote timestamp version accepted as newer"
  fi
)

make_fake_release_archive() {
  local out_dir="${1:?out_dir_required}"
  local version="${2:?version_required}"
  local installer_body="${3:?installer_body_required}"
  local release_dir="$out_dir/chimera-release"
  mkdir -p "$release_dir/bin" "$release_dir/scripts"
  printf '%s\n' "$version" >"$release_dir/.chimera_release_version"
  printf '#!/usr/bin/env bash\n%s\n' "$installer_body" >"$release_dir/scripts/install_desktop_control.sh"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$release_dir/scripts/chimera.sh"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$release_dir/scripts/chimera-sh"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$release_dir/bin/chimera-bootstrap"
  chmod +x "$release_dir/scripts/"*.sh "$release_dir/bin/chimera-bootstrap"
  tar -czf "$out_dir/chimera-pq-release.tar.gz" -C "$out_dir" chimera-release
  (cd "$out_dir" && sha256sum chimera-pq-release.tar.gz > chimera-pq-release.tar.gz.sha256)
}

case_failed_install_restores_previous_release() (
  local tmp_dir old_home archive checksum rc
  tmp_dir="$(mktemp -d)"
  old_home="$tmp_dir/home/chimera"
  mkdir -p "$old_home/scripts" "$tmp_dir/bin"
  printf 'old-release\n' >"$old_home/old_marker"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$old_home/scripts/chimera.sh"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$old_home/scripts/chimera-sh"
  make_fake_release_archive "$tmp_dir" "0.1.99" "echo failing installer >&2; exit 42"
  archive="$tmp_dir/chimera-pq-release.tar.gz"
  checksum="$tmp_dir/chimera-pq-release.tar.gz.sha256"

  set +e
  CHIMERA_ALLOW_LOCAL_RELEASE_SOURCE=1 \
  CHIMERA_HOME="$old_home" \
  CHIMERA_LOCAL_BIN="$tmp_dir/bin" \
    bash "$ROOT_DIR/scripts/install_release.sh" "$archive" "$checksum" >/dev/null 2>&1
  rc=$?
  set -e
  [[ "$rc" -ne 0 ]] || fail "expected failing installer to return non-zero"
  [[ -f "$old_home/old_marker" ]] || fail "previous release was not restored after install failure"
  [[ ! -f "$old_home/.chimera_release_version" || "$(cat "$old_home/.chimera_release_version" 2>/dev/null)" != "0.1.99" ]] || fail "failed release remained installed"
  rm -rf "$tmp_dir"
)

case_control_requires_update_first_marker() (
  local tmp_dir control_stub output rc
  tmp_dir="$(mktemp -d)"
  control_stub="$tmp_dir/chimera-control.sh"
  cat >"$control_stub" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${CHIMERA_UPDATE_FIRST_CHECKED:-0}" != "1" ]]; then
  echo "direct control bypass blocked" >&2
  exit 7
fi
echo "control_ok"
EOF
  chmod +x "$control_stub"

  set +e
  output="$(CHIMERA_UPDATE_FIRST_CHECKED=0 "$control_stub" start 2>&1)"
  rc=$?
  set -e
  [[ "$rc" -eq 7 ]] || fail "direct control start should be blocked without marker"
  [[ "$output" == *"direct control bypass blocked"* ]] || fail "missing direct bypass diagnostic"
  rm -rf "$tmp_dir"
)

case_real_control_delegates_direct_start_and_mesh() (
  local tmp_dir launcher output rc
  tmp_dir="$(mktemp -d)"
  launcher="$tmp_dir/chimera-sh"
  cat >"$launcher" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'fake_launcher_args=%s\n' "$*"
exit 23
EOF
  chmod +x "$launcher"

  set +e
  output="$(CHIMERA_UPDATE_FIRST_LAUNCHER="$launcher" bash "$ROOT_DIR/scripts/chimera-control.sh" start 2>&1)"
  rc=$?
  set -e
  [[ "$rc" -eq 23 ]] || fail "direct real control start did not delegate to update-first launcher"
  [[ "$output" == *"fake_launcher_args=-start"* ]] || fail "direct real control start delegated wrong args"

  set +e
  output="$(CHIMERA_UPDATE_FIRST_LAUNCHER="$launcher" bash "$ROOT_DIR/scripts/chimera-control.sh" restart 2>&1)"
  rc=$?
  set -e
  [[ "$rc" -eq 23 ]] || fail "direct real control restart did not delegate to update-first launcher"
  [[ "$output" == *"fake_launcher_args=-restart"* ]] || fail "direct real control restart delegated wrong args"

  set +e
  output="$(CHIMERA_UPDATE_FIRST_LAUNCHER="$launcher" bash "$ROOT_DIR/scripts/chimera-control.sh" mesh nodes list 2>&1)"
  rc=$?
  set -e
  [[ "$rc" -eq 23 ]] || fail "direct real control mesh did not delegate to update-first launcher"
  [[ "$output" == *"fake_launcher_args=-mesh nodes list"* ]] || fail "direct real control mesh delegated wrong args"
  rm -rf "$tmp_dir"
)

case_update_sources_unreachable_continues
case_update_required_install_failure_blocks
case_semver_update_order
case_failed_install_restores_previous_release
case_control_requires_update_first_marker
case_real_control_delegates_direct_start_and_mesh

echo "chimera_update_contract_smoke=pass"
