#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "chimera_runner_sudo_contract_smoke: $1" >&2
  exit 1
}

make_install_root() {
  local install_root="${1:?install_root_required}"
  mkdir -p "$install_root/scripts" "$install_root/bin"
  cp "$ROOT_DIR/scripts/chimera-runner.sh" "$install_root/scripts/chimera-runner.sh"
  chmod +x "$install_root/scripts/chimera-runner.sh"
}

write_fake_runtime() {
  local install_root="${1:?install_root_required}"
  local binary="${2:?binary_required}"
  local record="${3:?record_required}"
  cat >"$install_root/bin/$binary" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
{
  printf 'binary=%s\n' "__BINARY__"
  printf 'nft_mode=%s\n' "${CHIMERA_NFT_PRIVILEGE_MODE:-unset}"
  printf 'runner_sudo=%s\n' "${CHIMERA_RUNNER_USE_SUDO:-unset}"
  printf 'peer_token=%s\n' "${CHIMERA_PEER_EGRESS_TOKEN:-unset}"
  printf 'args=%s\n' "$*"
} >>"__RECORD__"
EOF
  sed -i -e "s|__BINARY__|$binary|g" -e "s|__RECORD__|$record|g" "$install_root/bin/$binary"
  chmod +x "$install_root/bin/$binary"
}

case_legacy_runner_sudo_maps_to_nft_sudo_mode() {
  local tmp_dir install_root record output rc
  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  record="$tmp_dir/record.log"
  touch "$record"
  make_install_root "$install_root"
  write_fake_runtime "$install_root" "chimera-transparent-runtime" "$record"

  set +e
  output="$(
    CHIMERA_RUNNER_USE_SUDO=1 \
    CHIMERA_PEER_EGRESS_TOKEN="not_a_runner_secret" \
      timeout 10s "$install_root/scripts/chimera-runner.sh" transparent-runtime --run-ms 1 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "legacy sudo mapping failed rc=$rc output=$output"
  rg -q '^binary=chimera-transparent-runtime$' "$record" || fail "transparent runtime did not run"
  rg -q '^nft_mode=sudo$' "$record" || fail "legacy CHIMERA_RUNNER_USE_SUDO did not map to nft sudo mode"
  rg -q '^runner_sudo=1$' "$record" || fail "legacy flag not visible to runtime"
  rg -q '^peer_token=not_a_runner_secret$' "$record" || fail "runner unexpectedly scrubbed user env"
  rg -q '^args=--run-ms 1$' "$record" || fail "transparent runtime args not preserved"

  rm -rf "$tmp_dir"
}

case_explicit_nft_mode_is_not_overwritten() {
  local tmp_dir install_root record output rc
  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  record="$tmp_dir/record.log"
  touch "$record"
  make_install_root "$install_root"
  write_fake_runtime "$install_root" "chimera-transparent-runtime" "$record"

  set +e
  output="$(
    CHIMERA_RUNNER_USE_SUDO=1 \
    CHIMERA_NFT_PRIVILEGE_MODE=direct \
      timeout 10s "$install_root/scripts/chimera-runner.sh" transparent-runtime 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "explicit nft direct mode failed rc=$rc output=$output"
  rg -q '^nft_mode=direct$' "$record" || fail "explicit CHIMERA_NFT_PRIVILEGE_MODE was overwritten"

  rm -rf "$tmp_dir"
}

case_sudo_flag_does_not_apply_to_other_targets() {
  local tmp_dir install_root record output rc
  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  record="$tmp_dir/record.log"
  touch "$record"
  make_install_root "$install_root"
  write_fake_runtime "$install_root" "chimera-cli" "$record"

  set +e
  output="$(
    CHIMERA_RUNNER_USE_SUDO=1 \
      timeout 10s "$install_root/scripts/chimera-runner.sh" cli --version 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "cli target failed rc=$rc output=$output"
  rg -q '^binary=chimera-cli$' "$record" || fail "cli runtime did not run"
  rg -q '^nft_mode=unset$' "$record" || fail "nft sudo mode leaked into cli target"

  rm -rf "$tmp_dir"
}

case_runner_contains_no_sudo_reexec() {
  if rg -n 'exec sudo|sudo -n env|sudo -n bash' "$ROOT_DIR/scripts/chimera-runner.sh" >/dev/null; then
    fail "runner must not sudo-reexec itself"
  fi
}

case_legacy_runner_sudo_maps_to_nft_sudo_mode
case_explicit_nft_mode_is_not_overwritten
case_sudo_flag_does_not_apply_to_other_targets
case_runner_contains_no_sudo_reexec

echo "chimera_runner_sudo_contract_smoke=pass"
