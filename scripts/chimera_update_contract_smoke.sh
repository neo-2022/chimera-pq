#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "chimera_update_contract_smoke: $1" >&2
  exit 1
}

source "$ROOT_DIR/scripts/chimera-sh"

case_gitverse_repo_root_normalizes_to_raw_bootstrap() (
  local expected
  expected="https://gitverse.ru/api/repos/ArtReg/chimera/raw/branch/main/chimera.sh"
  [[ "$(normalize_update_bootstrap_url "https://gitverse.ru/ArtReg/chimera")" == "$expected" ]] \
    || fail "update bootstrap did not normalize GitVerse repo root"
)

case_gitverse_content_viewer_normalizes_to_raw_bootstrap() (
  local expected
  expected="https://gitverse.ru/api/repos/ArtReg/chimera/raw/branch/main/chimera.sh"
  [[ "$(normalize_update_bootstrap_url "https://gitverse.ru/ArtReg/chimera/content/main/chimera.sh")" == "$expected" ]] \
    || fail "update bootstrap did not normalize GitVerse viewer URL"
)

case_gitverse_archive_viewer_normalizes_to_raw_bootstrap() (
  local expected
  expected="https://gitverse.ru/api/repos/ArtReg/chimera/raw/branch/main/chimera.sh"
  [[ "$(normalize_update_bootstrap_url "https://gitverse.ru/ArtReg/chimera/content/main/chimera-pq-release.tar.gz.sha256")" == "$expected" ]] \
    || fail "update bootstrap did not normalize GitVerse checksum viewer URL"
)

case_gitverse_default_source_loads_without_operator_config() (
  local tmp_dir expected output
  tmp_dir="$(mktemp -d)"
  expected="http://gitvers-default.invalid/chimera.sh"
  unset CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URL CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URLS
  HOME="$tmp_dir/home"
  XDG_CONFIG_HOME="$tmp_dir/xdg-config"
  UPDATE_GITVERS_BOOTSTRAP_URLS_FILE="$tmp_dir/missing.list"
  UPDATE_GITVERS_BOOTSTRAP_URLS_DEFAULT="$expected"

  output="$(load_update_gitvers_bootstrap_urls)"
  [[ "$output" == "$expected" ]] || fail "gitvers default source did not load without operator config"
  rm -rf "$tmp_dir"
)

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

case_update_download_uses_bounded_bootstrap_timeouts() (
  local tmp_dir helper out record
  tmp_dir="$(mktemp -d)"
  helper="$tmp_dir/chimera-bootstrap"
  out="$tmp_dir/out"
  record="$tmp_dir/record"
  cat >"$helper" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
out=""
while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --output)
      out="${2:-}"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
printf 'connect=%s max=%s http_proxy=%s https_proxy=%s\n' \
  "${CHIMERA_BOOTSTRAP_CONNECT_TIMEOUT_SEC:-}" \
  "${CHIMERA_BOOTSTRAP_DOWNLOAD_TIMEOUT_SEC:-}" \
  "${HTTP_PROXY:-unset}" \
  "${HTTPS_PROXY:-unset}" >"__RECORD__"
printf '%s\n' ok >"$out"
EOF
  sed -i "s|__RECORD__|$record|g" "$helper"
  chmod +x "$helper"

  HTTP_PROXY=http://proxy.invalid \
  HTTPS_PROXY=http://proxy.invalid \
  CHIMERA_BOOTSTRAP_BIN="$helper" \
  CHIMERA_UPDATE_DOWNLOAD_CONNECT_TIMEOUT_SEC=2 \
  CHIMERA_UPDATE_DOWNLOAD_MAX_TIME_SEC=4 \
  UPDATE_DOWNLOAD_CONNECT_TIMEOUT_SEC=2 \
  UPDATE_DOWNLOAD_MAX_TIME_SEC=4 \
    download_url_to_file "http://example.invalid/chimera.sh" "$out" >/dev/null 2>&1 \
      || fail "bounded bootstrap download should succeed through helper"

  [[ -f "$out" ]] || fail "bounded bootstrap download did not create output"
  [[ "$(cat "$record")" == *"connect=2"* ]] || fail "bootstrap connect timeout not passed"
  [[ "$(cat "$record")" == *"max=4"* ]] || fail "bootstrap download timeout not passed"
  [[ "$(cat "$record")" == *"http_proxy=unset"* ]] || fail "bootstrap HTTP proxy was not cleared"
  [[ "$(cat "$record")" == *"https_proxy=unset"* ]] || fail "bootstrap HTTPS proxy was not cleared"
  rm -rf "$tmp_dir"
)

case_update_download_uses_bounded_curl_args() (
  local tmp_dir fake_bin out record
  tmp_dir="$(mktemp -d)"
  fake_bin="$tmp_dir/bin"
  out="$tmp_dir/out"
  record="$tmp_dir/record"
  mkdir -p "$fake_bin"
  cat >"$fake_bin/curl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'args=%s\nhttp_proxy=%s\nhttps_proxy=%s\n' "$*" "${HTTP_PROXY:-unset}" "${HTTPS_PROXY:-unset}" >"__RECORD__"
out=""
while [[ "$#" -gt 0 ]]; do
  case "$1" in
    -o)
      out="${2:-}"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
printf '%s\n' ok >"$out"
EOF
  sed -i "s|__RECORD__|$record|g" "$fake_bin/curl"
  chmod +x "$fake_bin/curl"

  HTTP_PROXY=http://proxy.invalid \
  HTTPS_PROXY=http://proxy.invalid \
  CHIMERA_BOOTSTRAP_BIN="$tmp_dir/missing-bootstrap" \
  PATH="$fake_bin:$PATH" \
  CHIMERA_UPDATE_DOWNLOAD_CONNECT_TIMEOUT_SEC=2 \
  CHIMERA_UPDATE_DOWNLOAD_MAX_TIME_SEC=4 \
  CHIMERA_UPDATE_DOWNLOAD_RETRIES=1 \
  UPDATE_DOWNLOAD_CONNECT_TIMEOUT_SEC=2 \
  UPDATE_DOWNLOAD_MAX_TIME_SEC=4 \
  UPDATE_DOWNLOAD_RETRIES=1 \
    download_url_to_file "http://example.invalid/chimera.sh" "$out" >/dev/null 2>&1 \
      || fail "bounded curl download should succeed through fake curl"

  [[ "$(cat "$record")" == *"--retry 1"* ]] || fail "curl retry bound missing"
  [[ "$(cat "$record")" == *"--disable"* ]] || fail "curl disable rcfile flag missing"
  [[ "$(cat "$record")" == *"--connect-timeout 2"* ]] || fail "curl connect timeout bound missing"
  [[ "$(cat "$record")" == *"--max-time 4"* ]] || fail "curl max time bound missing"
  [[ "$(cat "$record")" == *"http_proxy=unset"* ]] || fail "curl HTTP proxy was not cleared"
  [[ "$(cat "$record")" == *"https_proxy=unset"* ]] || fail "curl HTTPS proxy was not cleared"
  rm -rf "$tmp_dir"
)

case_update_download_timeout_bounds_slow_helper() (
  local tmp_dir helper fake_bin out start elapsed rc
  tmp_dir="$(mktemp -d)"
  helper="$tmp_dir/chimera-bootstrap"
  fake_bin="$tmp_dir/bin"
  out="$tmp_dir/out"
  mkdir -p "$fake_bin"
  cat >"$helper" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
sleep 5
EOF
  chmod +x "$helper"
  cat >"$fake_bin/curl" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
  cat >"$fake_bin/wget" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
  chmod +x "$fake_bin/curl" "$fake_bin/wget"

  start="$(date +%s)"
  set +e
  ROOT_DIR="$ROOT_DIR" \
  CHIMERA_BOOTSTRAP_BIN="$helper" \
  CHIMERA_UPDATE_DOWNLOAD_CONNECT_TIMEOUT_SEC=1 \
  CHIMERA_UPDATE_DOWNLOAD_MAX_TIME_SEC=1 \
  PATH="$fake_bin:$PATH" \
    timeout 4s bash -lc '
      source "$ROOT_DIR/scripts/chimera-sh"
      download_url_to_file "http://example.invalid/chimera.sh" "$1"
    ' bash "$out" >/dev/null 2>&1
  rc=$?
  set -e
  elapsed=$(( $(date +%s) - start ))

  [[ "$rc" -ne 0 ]] || fail "slow helper should not succeed"
  [[ ! -f "$out" ]] || fail "slow helper should not create output"
  [[ "$elapsed" -lt 4 ]] || fail "slow helper was not bounded"
  rm -rf "$tmp_dir"
)

case_newer_release_with_unreachable_checksum_continues() (
  local tmp_dir test_root output rc
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  mkdir -p "$test_root/scripts"
  cat >"$test_root/scripts/install_release.sh" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  chmod +x "$test_root/scripts/install_release.sh"
  ROOT_DIR="$test_root"
  read_local_runtime_version() { printf '%s\n' 0.1.0; }
  read_local_runtime_bundle_sha() { printf '%s\n' deadbeef; }
  load_update_peer_bootstrap_urls_for_args() { return 0; }
  read_release_metadata_from_source() {
    printf '%s\n%s\n%s\n' \
      0.1.99 \
      http://github.invalid/chimera-pq-release.tar.gz \
      http://github.invalid/chimera-pq-release.tar.gz.sha256
  }
  remote_archive_sha256() { return 2; }

  output="$(auto_update_if_needed -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  [[ "$rc" -eq 0 ]] || fail "unreachable checksum should fall through as source unavailable, got rc=$rc"
  [[ "$output" == *"reason=checksum_unreachable"* ]] || fail "missing checksum unreachable diagnostic"
  [[ "$output" == *"chimera_update=unavailable"* ]] || fail "unreachable checksum was not downgraded to soft outage"
  rm -rf "$tmp_dir"
)

case_newer_release_with_unreachable_checksum_falls_back_to_peer() (
  local tmp_dir test_root output rc calls installed_marker rerun_args
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  calls="$tmp_dir/calls"
  installed_marker="$tmp_dir/installed"
  rerun_args="$tmp_dir/rerun_args"
  expected_version="0.1.100"
  expected_sha="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  mkdir -p "$test_root/scripts"
  cat >"$test_root/scripts/install_release.sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' install >>"$calls"
touch "$installed_marker"
exit 0
EOF
  chmod +x "$test_root/scripts/install_release.sh"
  ROOT_DIR="$test_root"
  read_local_runtime_version() {
    if [[ -f "$installed_marker" ]]; then
      printf '%s\n' "$expected_version"
    else
      printf '%s\n' 0.1.0
    fi
  }
  read_local_runtime_bundle_sha() {
    if [[ -f "$installed_marker" ]]; then
      printf '%s\n' "$expected_sha"
    else
      printf '%s\n' deadbeef
    fi
  }
  read_release_metadata_from_source() {
    case "${1:-}" in
      github)
        printf '%s\n%s\n%s\n' \
          0.1.99 \
          http://github.invalid/chimera-pq-release.tar.gz \
          http://github.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      peer)
        printf '%s\n%s\n%s\n' \
          0.1.100 \
          http://peer.invalid/chimera-pq-release.tar.gz \
          http://peer.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *)
        return 2
        ;;
    esac
  }
  load_update_peer_bootstrap_urls_for_args() {
    printf '%s\n' http://peer.invalid/chimera.sh
  }
  remote_archive_sha256() {
    case "${1:-}" in
      http://peer.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$expected_sha"
        ;;
      *)
        return 2
        ;;
    esac
  }
  rerun_after_update() {
    printf '%s\n' "rerun:$*" >>"$calls"
    printf '%s\n' "$*" >"$rerun_args"
    return 0
  }

  output="$(auto_update_if_needed -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  [[ "$rc" -eq 0 ]] || fail "peer fallback after github checksum outage should continue"
  [[ "$output" == *"reason=checksum_unreachable"* ]] || fail "missing github checksum unreachable diagnostic"
  [[ "$output" == *"chimera_update=available source=peer"* ]] || fail "peer fallback was not reached"
  [[ -f "$installed_marker" ]] || fail "peer fallback installer was not invoked"
  [[ "$(cat "$rerun_args")" == "-restart" ]] || fail "original command was not restarted after peer update"
  [[ "$(cat "$calls")" == $'install\nrerun:-restart' ]] || fail "unexpected peer fallback call order"
  rm -rf "$tmp_dir"
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

case_confirmed_update_missing_local_installer_blocks() (
  local tmp_dir old_root test_root output rc
  tmp_dir="$(mktemp -d)"
  old_root="$ROOT_DIR"
  test_root="$tmp_dir/root"
  mkdir -p "$test_root/scripts"
  ROOT_DIR="$test_root"
  remote_archive_sha256() {
    printf '%s\n' "3333333333333333333333333333333333333333333333333333333333333333"
  }

  output="$(install_update_from_release_metadata \
    peer \
    0.1.99 \
    http://peer.invalid/chimera-pq-release.tar.gz \
    http://peer.invalid/chimera-pq-release.tar.gz.sha256 \
    "" \
    0.1.0 \
    deadbeef \
    -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  ROOT_DIR="$old_root"
  [[ "$rc" -ne 0 ]] || fail "missing local installer should block confirmed update"
  [[ "$output" == *"reason=missing_local_installer"* ]] || fail "missing local installer diagnostic absent"
  [[ "$output" == *"action=block"* ]] || fail "missing local installer was not blocking"
  rm -rf "$tmp_dir"
)

case_peer_confirmed_update_failure_blocks_start() (
  read_local_runtime_version() { printf '%s\n' 0.1.0; }
  read_local_runtime_bundle_sha() { printf '%s\n' deadbeef; }
  read_release_metadata_from_source() {
    case "${1:-}" in
      github) return 2 ;;
      peer)
        printf '%s\n%s\n%s\n' \
          0.1.99 \
          http://peer.invalid/chimera-pq-release.tar.gz \
          http://peer.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *) return 2 ;;
    esac
  }
  load_update_peer_bootstrap_urls_for_args() {
    printf '%s\n' http://peer.invalid/chimera.sh
  }
  install_update_from_release_metadata() {
    [[ "${1:-}" == "peer" ]] || return 2
    echo "chimera_update=install_failed source=peer latest_version=${2:-unknown} action=block reason=missing_local_installer" >&2
    return 3
  }

  local output rc
  output="$(auto_update_if_needed -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  [[ "$rc" -ne 0 ]] || fail "peer confirmed update install failure should block start"
  [[ "$output" == *"reason=missing_local_installer"* ]] || fail "missing peer install failure diagnostic"
  [[ "$output" != *"chimera_update=unavailable"* ]] || fail "peer install failure was downgraded to soft outage"
)

case_github_install_failure_is_not_masked_by_peer_noop() (
  read_local_runtime_version() { printf '%s\n' 0.1.0; }
  read_local_runtime_bundle_sha() { printf '%s\n' deadbeef; }
  load_update_peer_bootstrap_urls() { printf '%s\n' http://peer.invalid/chimera.sh; }
  try_update_from_bootstrap_source() {
    case "${1:-}" in
      github) return 3 ;;
      peer) return 0 ;;
      *) return 2 ;;
    esac
  }

  local output rc
  output="$(auto_update_if_needed -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  [[ "$rc" -ne 0 ]] || fail "github install failure was masked by peer no-op"
  [[ "$output" != *"chimera_update=unavailable"* ]] || fail "unexpected outage diagnostic for masked install failure"
)

case_github_invalid_does_not_try_peer_fallback() (
  read_local_runtime_version() { printf '%s\n' 0.1.0; }
  read_local_runtime_bundle_sha() { printf '%s\n' deadbeef; }
  load_update_peer_bootstrap_urls_for_args() {
    printf '%s\n' http://peer.invalid/chimera.sh
  }
  try_update_from_bootstrap_source() {
    case "${1:-}" in
      github) return 3 ;;
      peer) fail "peer fallback must not run after invalid github response" ;;
      *) return 2 ;;
    esac
  }

  local output rc
  output="$(auto_update_if_needed -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  [[ "$rc" -ne 0 ]] || fail "expected invalid github response to block"
  [[ "$output" != *"chimera_update=unavailable"* ]] || fail "invalid github was reported as outage"
)

case_github_invalid_bootstrap_parse_does_not_try_peer_fallback() (
  local tmp_dir github_url peer_url calls output rc
  tmp_dir="$(mktemp -d)"
  github_url="http://github.invalid/chimera.sh"
  peer_url="http://peer.invalid/chimera.sh"
  calls="$tmp_dir/calls"

  read_local_runtime_version() { printf '%s\n' 0.1.0; }
  read_local_runtime_bundle_sha() { printf '%s\n' deadbeef; }
  load_update_peer_bootstrap_urls_for_args() {
    printf '%s\n' "$peer_url"
  }
  download_url_to_file() {
    printf '%s\n' "$1" >>"$calls"
    case "$1" in
      "$github_url"?*)
        printf '%s\n' 'VERSION="not-semver"' >"$2"
        printf '%s\n' 'ARCHIVE_URL_DEFAULT="http://github.invalid/chimera-pq-release.tar.gz"' >>"$2"
        printf '%s\n' 'CHECKSUM_URL_DEFAULT="http://github.invalid/chimera-pq-release.tar.gz.sha256"' >>"$2"
        ;;
      "$peer_url"?*)
        fail "peer bootstrap should not be fetched after invalid github bootstrap"
        ;;
      *)
        return 1
        ;;
    esac
  }

  UPDATE_BOOTSTRAP_URL="$github_url"
  output="$(auto_update_if_needed -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  [[ "$rc" -ne 0 ]] || fail "invalid github bootstrap parse should block"
  [[ "$output" == *"source=github"* ]] || fail "missing github invalid diagnostic"
  [[ "$(cat "$calls")" == *"$github_url"* ]] || fail "github bootstrap was not fetched"
  [[ "$(cat "$calls")" != *"$peer_url"* ]] || fail "peer was tried after invalid github bootstrap"
  rm -rf "$tmp_dir"
)

case_github_unavailable_gitvers_newer_updates_before_peer() (
  local tmp_dir test_root old_root calls rerun_args output rc expected_sha
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  calls="$tmp_dir/calls"
  rerun_args="$tmp_dir/rerun_args"
  expected_sha="9999999999999999999999999999999999999999999999999999999999999999"
  mkdir -p "$test_root/scripts"

  cat >"$test_root/scripts/install_release.sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' install:gitvers >>"$calls"
touch "$tmp_dir/gitvers_installed"
exit 0
EOF
  chmod +x "$test_root/scripts/install_release.sh"

  ROOT_DIR="$test_root"
  read_local_runtime_version() {
    if [[ -f "$tmp_dir/gitvers_installed" ]]; then
      printf '%s\n' 0.1.200
    else
      printf '%s\n' 0.1.100
    fi
  }
  read_local_runtime_bundle_sha() {
    if [[ -f "$tmp_dir/gitvers_installed" ]]; then
      printf '%s\n' "$expected_sha"
    else
      printf '%s\n' deadbeef
    fi
  }
  read_release_metadata_from_source() {
    case "${1:-}" in
      github)
        printf '%s\n' meta:github >>"$calls"
        return 2
        ;;
      gitvers)
        printf '%s\n' meta:gitvers >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.200 \
          http://gitvers.invalid/chimera-pq-release.tar.gz \
          http://gitvers.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      peer)
        printf '%s\n' meta:peer >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.201 \
          http://peer.invalid/chimera-pq-release.tar.gz \
          http://peer.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *)
        return 2
        ;;
    esac
  }
  load_update_gitvers_bootstrap_urls() {
    printf '%s\n' http://gitvers.invalid/chimera.sh
  }
  load_update_peer_bootstrap_urls_for_args() {
    printf '%s\n' http://peer.invalid/chimera.sh
  }
  remote_archive_sha256() {
    case "${1:-}" in
      http://gitvers.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$expected_sha"
        ;;
      http://peer.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
        ;;
      *)
        return 2
        ;;
    esac
  }
  rerun_after_update() {
    printf '%s\n' "$*" >"$rerun_args"
    printf '%s\n' "rerun:$*" >>"$calls"
    return 0
  }

  output="$(auto_update_if_needed -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  ROOT_DIR="$old_root"
  [[ "$rc" -eq 0 ]] || fail "github outage should allow gitvers update, got rc=$rc"
  [[ "$output" == *"chimera_update=available source=gitvers"* ]] || fail "missing gitvers update diagnostic"
  [[ "$(cat "$rerun_args")" == "-restart" ]] || fail "gitvers update did not rerun original command"
  [[ "$(cat "$calls")" == $'meta:github\nmeta:gitvers\ninstall:gitvers\nrerun:-restart' ]] || fail "peer was used before gitvers or call order changed"
  rm -rf "$tmp_dir"
)

case_github_unavailable_default_gitvers_newer_updates_without_operator_config() (
  local tmp_dir test_root old_root calls rerun_args output rc expected_sha
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  calls="$tmp_dir/calls"
  rerun_args="$tmp_dir/rerun_args"
  expected_sha="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  mkdir -p "$test_root/scripts"

  cat >"$test_root/scripts/install_release.sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' install:gitvers >>"$calls"
touch "$tmp_dir/gitvers_installed"
exit 0
EOF
  chmod +x "$test_root/scripts/install_release.sh"

  ROOT_DIR="$test_root"
  unset CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URL CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URLS
  UPDATE_GITVERS_BOOTSTRAP_URLS_FILE="$tmp_dir/missing.list"
  CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URLS_DEFAULT="http://gitvers-default.invalid/chimera.sh"
  read_local_runtime_version() {
    if [[ -f "$tmp_dir/gitvers_installed" ]]; then
      printf '%s\n' 0.1.200
    else
      printf '%s\n' 0.1.100
    fi
  }
  read_local_runtime_bundle_sha() {
    if [[ -f "$tmp_dir/gitvers_installed" ]]; then
      printf '%s\n' "$expected_sha"
    else
      printf '%s\n' deadbeef
    fi
  }
  read_release_metadata_from_source() {
    case "${1:-}" in
      github)
        printf '%s\n' meta:github >>"$calls"
        return 2
        ;;
      gitvers)
        printf '%s\n' meta:gitvers >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.200 \
          http://gitvers.invalid/chimera-pq-release.tar.gz \
          http://gitvers.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      peer)
        printf '%s\n' meta:peer >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.201 \
          http://peer.invalid/chimera-pq-release.tar.gz \
          http://peer.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *)
        return 2
        ;;
    esac
  }
  load_update_peer_bootstrap_urls_for_args() {
    printf '%s\n' http://peer.invalid/chimera.sh
  }
  remote_archive_sha256() {
    case "${1:-}" in
      http://gitvers.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$expected_sha"
        ;;
      http://peer.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
        ;;
      *)
        return 2
        ;;
    esac
  }
  rerun_after_update() {
    printf '%s\n' "$*" >"$rerun_args"
    printf '%s\n' "rerun:$*" >>"$calls"
    return 0
  }

  output="$(auto_update_if_needed -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  ROOT_DIR="$old_root"
  [[ "$rc" -eq 0 ]] || fail "default gitvers source should update after github outage, got rc=$rc"
  [[ "$output" == *"chimera_update=available source=gitvers"* ]] || fail "missing default gitvers update diagnostic"
  [[ "$(cat "$rerun_args")" == "-restart" ]] || fail "default gitvers update did not rerun original command"
  [[ "$(cat "$calls")" == $'meta:github\nmeta:gitvers\ninstall:gitvers\nrerun:-restart' ]] || fail "default gitvers source order changed or peer was consulted"
  rm -rf "$tmp_dir"
)

case_github_unavailable_gitvers_unavailable_falls_back_to_peer() (
  local tmp_dir test_root old_root calls rerun_args output rc expected_sha
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  calls="$tmp_dir/calls"
  rerun_args="$tmp_dir/rerun_args"
  expected_sha="7777777777777777777777777777777777777777777777777777777777777777"
  mkdir -p "$test_root/scripts"

  cat >"$test_root/scripts/install_release.sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' install:peer >>"$calls"
touch "$tmp_dir/peer_installed"
exit 0
EOF
  chmod +x "$test_root/scripts/install_release.sh"

  ROOT_DIR="$test_root"
  read_local_runtime_version() {
    if [[ -f "$tmp_dir/peer_installed" ]]; then
      printf '%s\n' 0.1.201
    else
      printf '%s\n' 0.1.100
    fi
  }
  read_local_runtime_bundle_sha() {
    if [[ -f "$tmp_dir/peer_installed" ]]; then
      printf '%s\n' "$expected_sha"
    else
      printf '%s\n' deadbeef
    fi
  }
  read_release_metadata_from_source() {
    case "${1:-}" in
      github)
        printf '%s\n' meta:github >>"$calls"
        return 2
        ;;
      gitvers)
        printf '%s\n' meta:gitvers >>"$calls"
        return 2
        ;;
      peer)
        printf '%s\n' meta:peer >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.201 \
          http://peer.invalid/chimera-pq-release.tar.gz \
          http://peer.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *)
        return 2
        ;;
    esac
  }
  load_update_gitvers_bootstrap_urls() {
    printf '%s\n' http://gitvers.invalid/chimera.sh
  }
  load_update_peer_bootstrap_urls_for_args() {
    printf '%s\n' http://peer.invalid/chimera.sh
  }
  remote_archive_sha256() {
    case "${1:-}" in
      http://peer.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$expected_sha"
        ;;
      *)
        return 2
        ;;
    esac
  }
  rerun_after_update() {
    printf '%s\n' "$*" >"$rerun_args"
    printf '%s\n' "rerun:$*" >>"$calls"
    return 0
  }

  output="$(auto_update_if_needed -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  ROOT_DIR="$old_root"
  [[ "$rc" -eq 0 ]] || fail "gitvers outage should fall back to peer, got rc=$rc"
  [[ "$output" == *"chimera_update=available source=peer"* ]] || fail "missing peer update diagnostic after gitvers outage"
  [[ "$(cat "$rerun_args")" == "-restart" ]] || fail "peer update did not rerun original command"
  [[ "$(cat "$calls")" == $'meta:github\nmeta:gitvers\nmeta:peer\ninstall:peer\nrerun:-restart' ]] || fail "unexpected source order through gitvers outage"
  rm -rf "$tmp_dir"
)

case_github_unavailable_gitvers_current_stops_before_peer_newer_update() (
  local tmp_dir test_root old_root calls output rc current_sha peer_sha
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  calls="$tmp_dir/calls"
  current_sha="5656565656565656565656565656565656565656565656565656565656565656"
  peer_sha="7878787878787878787878787878787878787878787878787878787878787878"
  mkdir -p "$test_root/scripts"

  cat >"$test_root/scripts/install_release.sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' install:unexpected >>"$calls"
exit 1
EOF
  chmod +x "$test_root/scripts/install_release.sh"

  ROOT_DIR="$test_root"
  read_local_runtime_version() { printf '%s\n' 0.1.141; }
  read_local_runtime_bundle_sha() { printf '%s\n' "$current_sha"; }
  read_release_metadata_from_source() {
    case "${1:-}" in
      github)
        printf '%s\n' meta:github >>"$calls"
        return 2
        ;;
      gitvers)
        printf '%s\n' meta:gitvers >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.141 \
          http://gitvers.invalid/chimera-pq-release.tar.gz \
          http://gitvers.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      peer)
        printf '%s\n' meta:peer >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.142 \
          http://peer.invalid/chimera-pq-release.tar.gz \
          http://peer.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *)
        return 2
        ;;
    esac
  }
  load_update_gitvers_bootstrap_urls() {
    printf '%s\n' http://gitvers.invalid/chimera.sh
  }
  load_update_peer_bootstrap_urls_for_args() {
    printf '%s\n' http://peer.invalid/chimera.sh
  }
  remote_archive_sha256() {
    case "${1:-}" in
      http://gitvers.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$current_sha"
        ;;
      http://peer.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$peer_sha"
        ;;
      *)
        return 2
        ;;
    esac
  }

  output="$(auto_update_if_needed -restart 2>&1)"
  rc=$?
  ROOT_DIR="$old_root"
  [[ "$rc" -eq 0 ]] || fail "verified gitvers current result should stop lower-trust newer peer update, got rc=$rc"
  [[ "$output" == *"chimera_update=source_current source=gitvers"* ]] || fail "missing gitvers current diagnostic before stop"
  [[ "$output" == *"chimera_update=no_newer_release current_version=0.1.141 action=continue reason=highest_available_not_newer"* ]] || fail "missing final no-newer-release diagnostic after verified gitvers current result"
  [[ "$output" != *"source=peer"* ]] || fail "peer should not be consulted after verified gitvers current result"
  [[ "$(cat "$calls")" == $'meta:github\nmeta:gitvers' ]] || fail "unexpected lower-trust activity after verified gitvers current result"
  rm -rf "$tmp_dir"
)

case_github_unavailable_gitvers_invalid_does_not_try_peer() (
  local tmp_dir calls output rc
  tmp_dir="$(mktemp -d)"
  calls="$tmp_dir/calls"
  read_local_runtime_version() { printf '%s\n' 0.1.0; }
  read_local_runtime_bundle_sha() { printf '%s\n' deadbeef; }
  read_release_metadata_from_source() {
    case "${1:-}" in
      github)
        printf '%s\n' github >>"$calls"
        return 2
        ;;
      gitvers)
        printf '%s\n' gitvers >>"$calls"
        printf '%s\n%s\n%s\n' \
          invalid-version \
          http://gitvers.invalid/chimera-pq-release.tar.gz \
          http://gitvers.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      peer)
        printf '%s\n' peer >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.2 \
          http://peer.invalid/chimera-pq-release.tar.gz \
          http://peer.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *)
        return 2
        ;;
    esac
  }
  load_update_gitvers_bootstrap_urls() {
    printf '%s\n' http://gitvers.invalid/chimera.sh
  }
  load_update_peer_bootstrap_urls_for_args() {
    printf '%s\n' http://peer.invalid/chimera.sh
  }

  set +e
  output="$(auto_update_if_needed -start 2>&1)"
  rc=$?
  set -e
  [[ "$rc" -ne 0 ]] || fail "invalid gitvers metadata should fail closed"
  [[ "$output" == *"chimera_update=source_invalid source=gitvers"* ]] || fail "missing invalid gitvers diagnostic"
  [[ "$(tr '\n' ',' <"$calls" | sed 's/,$//')" == "github,gitvers" ]] || fail "peer fallback was attempted after invalid gitvers metadata"
  rm -rf "$tmp_dir"
)

case_github_stale_gitvers_newer_updates_before_peer() (
  local tmp_dir test_root old_root calls rerun_args output rc expected_sha github_sha
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  calls="$tmp_dir/calls"
  rerun_args="$tmp_dir/rerun_args"
  expected_sha="abababababababababababababababababababababababababababababababab"
  github_sha="cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"
  mkdir -p "$test_root/scripts"

  cat >"$test_root/scripts/install_release.sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' install:gitvers >>"$calls"
touch "$tmp_dir/gitvers_installed"
exit 0
EOF
  chmod +x "$test_root/scripts/install_release.sh"

  ROOT_DIR="$test_root"
  read_local_runtime_version() {
    if [[ -f "$tmp_dir/gitvers_installed" ]]; then
      printf '%s\n' 0.1.141
    else
      printf '%s\n' 0.1.140
    fi
  }
  read_local_runtime_bundle_sha() {
    if [[ -f "$tmp_dir/gitvers_installed" ]]; then
      printf '%s\n' "$expected_sha"
    else
      printf '%s\n' deadbeef
    fi
  }
  read_release_metadata_from_source() {
    case "${1:-}" in
      github)
        printf '%s\n' meta:github >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.139 \
          http://github.invalid/chimera-pq-release.tar.gz \
          http://github.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      gitvers)
        printf '%s\n' meta:gitvers >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.141 \
          http://gitvers.invalid/chimera-pq-release.tar.gz \
          http://gitvers.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      peer)
        printf '%s\n' meta:peer >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.142 \
          http://peer.invalid/chimera-pq-release.tar.gz \
          http://peer.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *)
        return 2
        ;;
    esac
  }
  load_update_gitvers_bootstrap_urls() {
    printf '%s\n' http://gitvers.invalid/chimera.sh
  }
  load_update_peer_bootstrap_urls_for_args() {
    printf '%s\n' http://peer.invalid/chimera.sh
  }
  remote_archive_sha256() {
    case "${1:-}" in
      http://github.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$github_sha"
        ;;
      http://gitvers.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$expected_sha"
        ;;
      http://peer.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
        ;;
      *)
        return 2
        ;;
    esac
  }
  rerun_after_update() {
    printf '%s\n' "$*" >"$rerun_args"
    printf '%s\n' "rerun:$*" >>"$calls"
    return 0
  }

  output="$(auto_update_if_needed -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  ROOT_DIR="$old_root"
  [[ "$rc" -eq 0 ]] || fail "verified github stale result should stop lower-trust newer update, got rc=$rc"
  [[ "$output" == *"chimera_update=source_stale source=github"* ]] || fail "missing stale github diagnostic before stop"
  [[ "$output" == *"chimera_update=no_newer_release current_version=0.1.140 action=continue reason=highest_available_not_newer"* ]] || fail "missing final no-newer-release diagnostic after verified github stale result"
  [[ "$output" != *"source=gitvers"* ]] || fail "gitvers should not be consulted after verified github stale result"
  [[ "$(cat "$calls")" == $'meta:github' ]] || fail "unexpected lower-trust activity after verified github stale result"
  rm -rf "$tmp_dir"
)

case_github_stale_gitvers_current_reports_no_newer_release() (
  local tmp_dir test_root old_root calls output rc current_sha github_sha
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  calls="$tmp_dir/calls"
  current_sha="1212121212121212121212121212121212121212121212121212121212121212"
  github_sha="3434343434343434343434343434343434343434343434343434343434343434"
  mkdir -p "$test_root/scripts"

  cat >"$test_root/scripts/install_release.sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' install:unexpected >>"$calls"
exit 1
EOF
  chmod +x "$test_root/scripts/install_release.sh"

  ROOT_DIR="$test_root"
  read_local_runtime_version() { printf '%s\n' 0.1.141; }
  read_local_runtime_bundle_sha() { printf '%s\n' "$current_sha"; }
  read_release_metadata_from_source() {
    case "${1:-}" in
      github)
        printf '%s\n' meta:github >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.140 \
          http://github.invalid/chimera-pq-release.tar.gz \
          http://github.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      gitvers)
        printf '%s\n' meta:gitvers >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.141 \
          http://gitvers.invalid/chimera-pq-release.tar.gz \
          http://gitvers.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *)
        return 2
        ;;
    esac
  }
  load_update_gitvers_bootstrap_urls() {
    printf '%s\n' http://gitvers.invalid/chimera.sh
  }
  load_update_peer_bootstrap_urls_for_args() { return 0; }
  remote_archive_sha256() {
    case "${1:-}" in
      http://github.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$github_sha"
        ;;
      http://gitvers.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$current_sha"
        ;;
      *)
        return 2
        ;;
    esac
  }

  output="$(auto_update_if_needed -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  ROOT_DIR="$old_root"
  [[ "$rc" -eq 0 ]] || fail "verified github stale result should stop before gitvers corroboration, got rc=$rc"
  [[ "$output" == *"chimera_update=source_stale source=github"* ]] || fail "missing stale github diagnostic in verified-stop path"
  [[ "$output" == *"chimera_update=no_newer_release current_version=0.1.141 action=continue reason=highest_available_not_newer"* ]] || fail "missing final no-newer-release diagnostic"
  [[ "$output" != *"chimera_update=unavailable"* ]] || fail "no-newer-release path was misreported as source outage"
  [[ "$output" != *"source=gitvers"* ]] || fail "gitvers should not be consulted after verified github stale result"
  [[ "$(cat "$calls")" == $'meta:github' ]] || fail "unexpected install or lower-trust usage in verified-stop path"
  rm -rf "$tmp_dir"
)

case_github_current_gitvers_current_peer_newer_updates_from_peer() (
  local tmp_dir test_root old_root calls rerun_args output rc current_sha peer_sha
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  calls="$tmp_dir/calls"
  rerun_args="$tmp_dir/rerun_args"
  current_sha="5656565656565656565656565656565656565656565656565656565656565656"
  peer_sha="7878787878787878787878787878787878787878787878787878787878787878"
  mkdir -p "$test_root/scripts"

  cat >"$test_root/scripts/install_release.sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
archive="\${1:-}"
case "\$archive" in
  *peer.invalid*)
    printf '%s\n' install:peer >>"$calls"
    touch "$tmp_dir/peer_installed"
    exit 0
    ;;
  *)
    printf '%s\n' install:unexpected >>"$calls"
    exit 1
    ;;
esac
EOF
  chmod +x "$test_root/scripts/install_release.sh"

  ROOT_DIR="$test_root"
  read_local_runtime_version() {
    if [[ -f "$tmp_dir/peer_installed" ]]; then
      printf '%s\n' 0.1.142
    else
      printf '%s\n' 0.1.141
    fi
  }
  read_local_runtime_bundle_sha() {
    if [[ -f "$tmp_dir/peer_installed" ]]; then
      printf '%s\n' "$peer_sha"
    else
      printf '%s\n' "$current_sha"
    fi
  }
  read_release_metadata_from_source() {
    case "${1:-}" in
      github)
        printf '%s\n' meta:github >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.141 \
          http://github.invalid/chimera-pq-release.tar.gz \
          http://github.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      gitvers)
        printf '%s\n' meta:gitvers >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.141 \
          http://gitvers.invalid/chimera-pq-release.tar.gz \
          http://gitvers.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      peer)
        printf '%s\n' meta:peer >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.142 \
          http://peer.invalid/chimera-pq-release.tar.gz \
          http://peer.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *)
        return 2
        ;;
    esac
  }
  load_update_gitvers_bootstrap_urls() {
    printf '%s\n' http://gitvers.invalid/chimera.sh
  }
  load_update_peer_bootstrap_urls_for_args() {
    printf '%s\n' http://peer.invalid/chimera.sh
  }
  remote_archive_sha256() {
    case "${1:-}" in
      http://github.invalid/chimera-pq-release.tar.gz|http://gitvers.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$current_sha"
        ;;
      http://peer.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$peer_sha"
        ;;
      *)
        return 2
        ;;
    esac
  }
  rerun_after_update() {
    printf '%s\n' "rerun:$*" >>"$calls"
    printf '%s\n' "$*" >"$rerun_args"
    return 0
  }

  output="$(auto_update_if_needed -restart 2>&1)"
  rc=$?
  ROOT_DIR="$old_root"
  [[ "$rc" -eq 0 ]] || fail "verified github current result should stop lower-trust newer update, got rc=$rc"
  [[ "$output" == *"chimera_update=source_current source=github"* ]] || fail "missing github current diagnostic before stop"
  [[ "$output" == *"chimera_update=no_newer_release current_version=0.1.141 action=continue reason=highest_available_not_newer"* ]] || fail "missing final no-newer-release diagnostic after verified github current result"
  [[ "$output" != *"source=gitvers"* && "$output" != *"source=peer"* ]] || fail "lower-trust sources should not be consulted after verified github current result"
  [[ "$(cat "$calls")" == $'meta:github' ]] || fail "unexpected lower-trust activity after verified github current result"
  rm -rf "$tmp_dir"
)

case_github_stale_gitvers_stale_peer_newer_updates_from_peer() (
  local tmp_dir test_root old_root calls rerun_args output rc github_sha gitvers_sha local_sha peer_sha
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  calls="$tmp_dir/calls"
  rerun_args="$tmp_dir/rerun_args"
  github_sha="9191919191919191919191919191919191919191919191919191919191919191"
  gitvers_sha="9292929292929292929292929292929292929292929292929292929292929292"
  local_sha="9393939393939393939393939393939393939393939393939393939393939393"
  peer_sha="9494949494949494949494949494949494949494949494949494949494949494"
  mkdir -p "$test_root/scripts"

  cat >"$test_root/scripts/install_release.sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
archive="\${1:-}"
case "\$archive" in
  *peer.invalid*)
    printf '%s\n' install:peer >>"$calls"
    touch "$tmp_dir/peer_installed"
    exit 0
    ;;
  *)
    printf '%s\n' install:unexpected >>"$calls"
    exit 1
    ;;
esac
EOF
  chmod +x "$test_root/scripts/install_release.sh"

  ROOT_DIR="$test_root"
  read_local_runtime_version() {
    if [[ -f "$tmp_dir/peer_installed" ]]; then
      printf '%s\n' 0.1.142
    else
      printf '%s\n' 0.1.141
    fi
  }
  read_local_runtime_bundle_sha() {
    if [[ -f "$tmp_dir/peer_installed" ]]; then
      printf '%s\n' "$peer_sha"
    else
      printf '%s\n' "$local_sha"
    fi
  }
  read_release_metadata_from_source() {
    case "${1:-}" in
      github)
        printf '%s\n' meta:github >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.140 \
          http://github.invalid/chimera-pq-release.tar.gz \
          http://github.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      gitvers)
        printf '%s\n' meta:gitvers >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.139 \
          http://gitvers.invalid/chimera-pq-release.tar.gz \
          http://gitvers.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      peer)
        printf '%s\n' meta:peer >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.142 \
          http://peer.invalid/chimera-pq-release.tar.gz \
          http://peer.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *)
        return 2
        ;;
    esac
  }
  load_update_gitvers_bootstrap_urls() {
    printf '%s\n' http://gitvers.invalid/chimera.sh
  }
  load_update_peer_bootstrap_urls_for_args() {
    printf '%s\n' http://peer.invalid/chimera.sh
  }
  remote_archive_sha256() {
    case "${1:-}" in
      http://github.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$github_sha"
        ;;
      http://gitvers.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$gitvers_sha"
        ;;
      http://peer.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$peer_sha"
        ;;
      *)
        return 2
        ;;
    esac
  }
  rerun_after_update() {
    printf '%s\n' "rerun:$*" >>"$calls"
    printf '%s\n' "$*" >"$rerun_args"
    return 0
  }

  output="$(auto_update_if_needed -restart 2>&1)"
  rc=$?
  ROOT_DIR="$old_root"
  [[ "$rc" -eq 0 ]] || fail "verified github stale result should stop lower-trust newer update, got rc=$rc"
  [[ "$output" == *"chimera_update=source_stale source=github"* ]] || fail "missing stale github diagnostic before stop"
  [[ "$output" == *"chimera_update=no_newer_release current_version=0.1.141 action=continue reason=highest_available_not_newer"* ]] || fail "missing final no-newer-release diagnostic after verified github stale result"
  [[ "$output" != *"source=gitvers"* && "$output" != *"source=peer"* ]] || fail "lower-trust sources should not be consulted after verified github stale result"
  [[ "$(cat "$calls")" == $'meta:github' ]] || fail "unexpected lower-trust activity after verified github stale result"
  rm -rf "$tmp_dir"
)

case_github_unavailable_gitvers_same_version_checksum_mismatch_blocks_before_peer() (
  local tmp_dir test_root old_root calls output rc current_sha
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  calls="$tmp_dir/calls"
  current_sha="abababababababababababababababababababababababababababababababab"
  mkdir -p "$test_root/scripts"

  cat >"$test_root/scripts/install_release.sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' install:unexpected >>"$calls"
exit 1
EOF
  chmod +x "$test_root/scripts/install_release.sh"

  ROOT_DIR="$test_root"
  read_local_runtime_version() { printf '%s\n' 0.1.141; }
  read_local_runtime_bundle_sha() { printf '%s\n' "$current_sha"; }
  read_release_metadata_from_source() {
    case "${1:-}" in
      github)
        printf '%s\n' meta:github >>"$calls"
        return 2
        ;;
      gitvers)
        printf '%s\n' meta:gitvers >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.141 \
          http://gitvers.invalid/chimera-pq-release.tar.gz \
          http://gitvers.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      peer)
        printf '%s\n' meta:peer >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.142 \
          http://peer.invalid/chimera-pq-release.tar.gz \
          http://peer.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *)
        return 2
        ;;
    esac
  }
  load_update_gitvers_bootstrap_urls() {
    printf '%s\n' http://gitvers.invalid/chimera.sh
  }
  load_update_peer_bootstrap_urls_for_args() {
    printf '%s\n' http://peer.invalid/chimera.sh
  }
  remote_archive_sha256() {
    case "${1:-}" in
      http://gitvers.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"
        ;;
      http://peer.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "efefefefefefefefefefefefefefefefefefefefefefefefefefefefefefefef"
        ;;
      *)
        return 2
        ;;
    esac
  }

  output="$(auto_update_if_needed -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  ROOT_DIR="$old_root"
  [[ "$rc" -ne 0 ]] || fail "same-version checksum mismatch on gitvers should block before peer"
  [[ "$output" == *"chimera_update=source_inconsistent source=gitvers"* ]] || fail "missing gitvers same-version mismatch diagnostic"
  [[ "$output" == *"reason=same_version_checksum_mismatch"* ]] || fail "missing gitvers mismatch reason"
  [[ "$(cat "$calls")" == $'meta:github\nmeta:gitvers' ]] || fail "peer fallback was attempted after invalid gitvers same-version mismatch"
  rm -rf "$tmp_dir"
)

case_missing_local_version_blocks_when_update_unavailable() (
  read_local_runtime_version() { printf '\n'; }
  read_local_runtime_bundle_sha() { printf '%s\n' deadbeef; }
  load_update_peer_bootstrap_urls() { return 0; }
  try_update_from_bootstrap_source() { return 2; }

  local output rc
  output="$(auto_update_if_needed -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  [[ "$rc" -ne 0 ]] || fail "expected missing local version to block when update source is unavailable"
  [[ "$output" == *"reason=local_version_unverified"* ]] || fail "missing local version repair diagnostic"
)

case_missing_local_version_uses_update_repair() (
  read_local_runtime_version() { printf '%s\n' 0.0.0; }
  read_local_runtime_bundle_sha() { printf '%s\n' deadbeef; }
  load_update_peer_bootstrap_urls() { return 0; }
  try_update_from_bootstrap_source() {
    [[ "${3:-}" == "0.0.0" ]] || fail "missing local version was not normalized to repair baseline"
    return 0
  }

  auto_update_if_needed -start >/dev/null 2>&1 || fail "expected update repair path to continue"
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
  mkdir -p "$release_dir/bin" "$release_dir/configs" "$release_dir/scripts"
  printf '%s\n' "$version" >"$release_dir/.chimera_release_version"
  printf '#!/usr/bin/env bash\n%s\n' "$installer_body" >"$release_dir/scripts/install_desktop_control.sh"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$release_dir/scripts/chimera.sh"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$release_dir/scripts/chimera-sh"
  printf '#!/usr/bin/env bash\nreturn 0 2>/dev/null || exit 0\n' >"$release_dir/scripts/chimera-update.sh"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$release_dir/bin/chimera-bootstrap"
  printf '%s\n' 'https://gitverse.ru/api/repos/ArtReg/chimera/raw/branch/main/chimera.sh' \
    >"$release_dir/configs/update_gitvers_bootstrap_urls.example.list"
  chmod +x "$release_dir/scripts/"*.sh "$release_dir/bin/chimera-bootstrap"
  tar -czf "$out_dir/chimera-pq-release.tar.gz" -C "$out_dir" chimera-release
  (cd "$out_dir" && sha256sum chimera-pq-release.tar.gz > chimera-pq-release.tar.gz.sha256)
}

make_fake_release_archive_with_current_installer() {
  local out_dir="${1:?out_dir_required}"
  local version="${2:?version_required}"
  local release_dir="$out_dir/chimera-release"
  mkdir -p \
    "$release_dir/bin" \
    "$release_dir/configs" \
    "$release_dir/deploy/desktop" \
    "$release_dir/deploy/systemd-user" \
    "$release_dir/scripts"
  printf '%s\n' "$version" >"$release_dir/.chimera_release_version"
  cp "$ROOT_DIR/scripts/install_desktop_control.sh" "$release_dir/scripts/install_desktop_control.sh"
  cat >"$release_dir/scripts/chimera-control.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  preflight-perms)
    echo "preflight_status=ok"
    ;;
  grant-perms)
    echo "grant_perms=skipped"
    ;;
esac
EOF
  cat >"$release_dir/scripts/chimera.sh" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  cat >"$release_dir/scripts/chimera-sh" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  cat >"$release_dir/scripts/chimera-update.sh" <<'EOF'
#!/usr/bin/env bash
return 0 2>/dev/null || exit 0
EOF
  cat >"$release_dir/scripts/chimera-control-tray.sh" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  cat >"$release_dir/scripts/chimera-control-launcher.sh" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  cat >"$release_dir/bin/chimera-bootstrap" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  cat >"$release_dir/deploy/desktop/chimera-control-gui.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=CHIMERA
Exec=__CHIMERA_ROOT__/scripts/chimera-sh
EOF
  cat >"$release_dir/deploy/systemd-user/chimera-node.service" <<'EOF'
[Service]
ExecStart=__CHIMERA_ROOT__/scripts/chimera-sh -start
EOF
  cat >"$release_dir/deploy/systemd-user/chimera-datapath.service" <<'EOF'
[Service]
ExecStart=__CHIMERA_ROOT__/scripts/chimera-sh -start
EOF
  printf '%s\n' 'CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS=4000' \
    >"$release_dir/configs/mesh_bootstrap.env.example"
  printf '%s\n' 'https://gitverse.ru/api/repos/ArtReg/chimera/raw/branch/main/chimera.sh' \
    >"$release_dir/configs/update_gitvers_bootstrap_urls.example.list"
  chmod +x "$release_dir/scripts/"*.sh "$release_dir/bin/chimera-bootstrap"
  tar -czf "$out_dir/chimera-pq-release.tar.gz" -C "$out_dir" chimera-release
  (cd "$out_dir" && sha256sum chimera-pq-release.tar.gz > chimera-pq-release.tar.gz.sha256)
}

case_auto_update_preserves_bound_transit_env() (
  local tmp_dir old_home xdg_config xdg_cache xdg_data local_bin fake_bin archive checksum env_file output rc
  tmp_dir="$(mktemp -d)"
  old_home="$tmp_dir/home/chimera"
  xdg_config="$tmp_dir/xdg-config"
  xdg_cache="$tmp_dir/xdg-cache"
  xdg_data="$tmp_dir/xdg-data"
  local_bin="$tmp_dir/bin"
  fake_bin="$tmp_dir/fake-bin"
  env_file="$xdg_config/chimera/peer-egress.env"
  mkdir -p "$old_home/scripts" "$xdg_config/chimera" "$xdg_cache" "$xdg_data" "$local_bin" "$fake_bin"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$old_home/scripts/chimera.sh"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$old_home/scripts/chimera-sh"
  printf '%s\n' \
    'CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=true' \
    'CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE=/safe/test/lanes.csv' \
    >"$env_file"
  cat >"$fake_bin/systemctl" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
  cat >"$fake_bin/nft" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  chmod +x "$fake_bin/systemctl" "$fake_bin/nft"

  make_fake_release_archive_with_current_installer "$tmp_dir" "0.1.99"
  archive="$tmp_dir/chimera-pq-release.tar.gz"
  checksum="$tmp_dir/chimera-pq-release.tar.gz.sha256"

  set +e
  output="$(CHIMERA_ALLOW_LOCAL_RELEASE_SOURCE=1 \
    CHIMERA_INSTALL_NODE_ROLE=node \
    CHIMERA_HOME="$old_home" \
    CHIMERA_LOCAL_BIN="$local_bin" \
    HOME="$tmp_dir/home/user" \
    XDG_CONFIG_HOME="$xdg_config" \
    XDG_CACHE_HOME="$xdg_cache" \
    XDG_DATA_HOME="$xdg_data" \
    PATH="$fake_bin:$PATH" \
    CHIMERA_PEER_EGRESS_TOKEN=test-token \
    bash "$ROOT_DIR/scripts/install_release.sh" "$archive" "$checksum" 2>&1)"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "auto-update preserve env install failed: $output"
  grep -q '^CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=true$' "$env_file" \
    || fail "auto-update lost bound transit allow env"
  grep -q '^CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE=/safe/test/lanes.csv$' "$env_file" \
    || fail "auto-update lost transit lane bindings env"
  [[ "$output" == *"peer_egress_transit_lane_bindings_file_configured=true"* ]] \
    || fail "auto-update did not report lane bindings configured"
  [[ "$output" != *"/safe/test/lanes.csv"* ]] || fail "auto-update leaked lane bindings path"
  rm -rf "$tmp_dir"
)

case_auto_update_heals_legacy_bound_transit_default() (
  local tmp_dir old_home xdg_config xdg_cache xdg_data local_bin fake_bin archive checksum env_file output rc
  tmp_dir="$(mktemp -d)"
  old_home="$tmp_dir/home/chimera"
  xdg_config="$tmp_dir/xdg-config"
  xdg_cache="$tmp_dir/xdg-cache"
  xdg_data="$tmp_dir/xdg-data"
  local_bin="$tmp_dir/bin"
  fake_bin="$tmp_dir/fake-bin"
  env_file="$xdg_config/chimera/peer-egress.env"
  mkdir -p "$old_home/scripts" "$xdg_config/chimera" "$xdg_cache" "$xdg_data" "$local_bin" "$fake_bin"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$old_home/scripts/chimera.sh"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$old_home/scripts/chimera-sh"
  printf '%s\n' \
    'CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=false' \
    'CHIMERA_PEER_EGRESS_ALLOW_POOL_TRANSIT=false' \
    >"$env_file"
  cat >"$fake_bin/systemctl" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
  cat >"$fake_bin/nft" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  chmod +x "$fake_bin/systemctl" "$fake_bin/nft"

  make_fake_release_archive_with_current_installer "$tmp_dir" "0.1.99"
  archive="$tmp_dir/chimera-pq-release.tar.gz"
  checksum="$tmp_dir/chimera-pq-release.tar.gz.sha256"

  set +e
  output="$(CHIMERA_ALLOW_LOCAL_RELEASE_SOURCE=1 \
    CHIMERA_INSTALL_NODE_ROLE=node \
    CHIMERA_HOME="$old_home" \
    CHIMERA_LOCAL_BIN="$local_bin" \
    HOME="$tmp_dir/home/user" \
    XDG_CONFIG_HOME="$xdg_config" \
    XDG_CACHE_HOME="$xdg_cache" \
    XDG_DATA_HOME="$xdg_data" \
    PATH="$fake_bin:$PATH" \
    CHIMERA_PEER_EGRESS_TOKEN=test-token \
    bash "$ROOT_DIR/scripts/install_release.sh" "$archive" "$checksum" 2>&1)"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "auto-update heal legacy default install failed: $output"
  grep -q '^CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=true$' "$env_file" \
    || fail "auto-update did not heal legacy bound transit default"
  grep -q '^CHIMERA_PEER_EGRESS_ALLOW_POOL_TRANSIT=false$' "$env_file" \
    || fail "auto-update changed pool transit while healing bound transit"
  rm -rf "$tmp_dir"
)

case_peer_egress_env_shell_quotes_lane_bindings_path() (
  local tmp_dir old_home xdg_config xdg_cache xdg_data local_bin fake_bin archive checksum env_file injected_path marker output rc
  tmp_dir="$(mktemp -d)"
  old_home="$tmp_dir/home/chimera"
  xdg_config="$tmp_dir/xdg-config"
  xdg_cache="$tmp_dir/xdg-cache"
  xdg_data="$tmp_dir/xdg-data"
  local_bin="$tmp_dir/bin"
  fake_bin="$tmp_dir/fake-bin"
  env_file="$xdg_config/chimera/peer-egress.env"
  marker="$tmp_dir/injection-ran"
  injected_path="/safe/test/lanes.csv;touch $marker & echo \$HOME"
  mkdir -p "$old_home/scripts" "$xdg_config/chimera" "$xdg_cache" "$xdg_data" "$local_bin" "$fake_bin"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$old_home/scripts/chimera.sh"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$old_home/scripts/chimera-sh"
  cat >"$fake_bin/systemctl" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
  cat >"$fake_bin/nft" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  chmod +x "$fake_bin/systemctl" "$fake_bin/nft"

  make_fake_release_archive_with_current_installer "$tmp_dir" "0.1.99"
  archive="$tmp_dir/chimera-pq-release.tar.gz"
  checksum="$tmp_dir/chimera-pq-release.tar.gz.sha256"

  set +e
  output="$(CHIMERA_ALLOW_LOCAL_RELEASE_SOURCE=1 \
    CHIMERA_INSTALL_NODE_ROLE=node \
    CHIMERA_HOME="$old_home" \
    CHIMERA_LOCAL_BIN="$local_bin" \
    HOME="$tmp_dir/home/user" \
    XDG_CONFIG_HOME="$xdg_config" \
    XDG_CACHE_HOME="$xdg_cache" \
    XDG_DATA_HOME="$xdg_data" \
    PATH="$fake_bin:$PATH" \
    CHIMERA_PEER_EGRESS_TOKEN=test-token \
    CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=true \
    CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE="$injected_path" \
    bash "$ROOT_DIR/scripts/install_release.sh" "$archive" "$checksum" 2>&1)"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "peer env shell quoting install failed: $output"
  [[ ! -f "$marker" ]] || fail "lane bindings path injection executed during installer source"
  grep -q '^CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=true$' "$env_file" \
    || fail "shell quoting test lost bound transit allow env"
  grep -q '^CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE=' "$env_file" \
    || fail "shell quoting test did not write lane bindings env"
  set -a
  # shellcheck disable=SC1090
  source "$env_file"
  set +a
  [[ "${CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE:-}" == "$injected_path" ]] \
    || fail "shell quoted lane bindings path did not round-trip"
  [[ ! -f "$marker" ]] || fail "lane bindings path injection executed during explicit source"
  rm -rf "$tmp_dir"
)

case_auto_update_preserves_quoted_lane_bindings_env() (
  local tmp_dir old_home xdg_config xdg_cache xdg_data local_bin fake_bin archive checksum env_file injected_path marker output rc
  tmp_dir="$(mktemp -d)"
  old_home="$tmp_dir/home/chimera"
  xdg_config="$tmp_dir/xdg-config"
  xdg_cache="$tmp_dir/xdg-cache"
  xdg_data="$tmp_dir/xdg-data"
  local_bin="$tmp_dir/bin"
  fake_bin="$tmp_dir/fake-bin"
  env_file="$xdg_config/chimera/peer-egress.env"
  marker="$tmp_dir/quoted-preserve-ran"
  injected_path="/safe/test/lanes.csv;touch $marker"
  mkdir -p "$old_home/scripts" "$xdg_config/chimera" "$xdg_cache" "$xdg_data" "$local_bin" "$fake_bin"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$old_home/scripts/chimera.sh"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$old_home/scripts/chimera-sh"
  {
    printf '%s\n' 'CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=true'
    printf 'CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE=%q\n' "$injected_path"
  } >"$env_file"
  cat >"$fake_bin/systemctl" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
  cat >"$fake_bin/nft" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  chmod +x "$fake_bin/systemctl" "$fake_bin/nft"

  make_fake_release_archive_with_current_installer "$tmp_dir" "0.1.99"
  archive="$tmp_dir/chimera-pq-release.tar.gz"
  checksum="$tmp_dir/chimera-pq-release.tar.gz.sha256"

  set +e
  output="$(CHIMERA_ALLOW_LOCAL_RELEASE_SOURCE=1 \
    CHIMERA_INSTALL_NODE_ROLE=node \
    CHIMERA_HOME="$old_home" \
    CHIMERA_LOCAL_BIN="$local_bin" \
    HOME="$tmp_dir/home/user" \
    XDG_CONFIG_HOME="$xdg_config" \
    XDG_CACHE_HOME="$xdg_cache" \
    XDG_DATA_HOME="$xdg_data" \
    PATH="$fake_bin:$PATH" \
    CHIMERA_PEER_EGRESS_TOKEN=test-token \
    bash "$ROOT_DIR/scripts/install_release.sh" "$archive" "$checksum" 2>&1)"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "quoted preserve env install failed: $output"
  [[ ! -f "$marker" ]] || fail "quoted preserved lane bindings path executed during install"
  grep -q '^CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=true$' "$env_file" \
    || fail "quoted preserve lost bound transit allow env"
  grep -q '^CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE=' "$env_file" \
    || fail "quoted preserve did not write lane bindings env"
  set -a
  # shellcheck disable=SC1090
  source "$env_file"
  set +a
  [[ "${CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE:-}" == "$injected_path" ]] \
    || fail "quoted preserved lane bindings path did not round-trip"
  [[ ! -f "$marker" ]] || fail "quoted preserved lane bindings path executed during explicit source"
  rm -rf "$tmp_dir"
)

case_peer_token_stays_in_private_peer_env() (
  local tmp_dir old_home xdg_config xdg_cache xdg_data local_bin fake_bin archive checksum bootstrap_env peer_env injected_token marker output rc peer_env_mode bootstrap_env_mode
  tmp_dir="$(mktemp -d)"
  old_home="$tmp_dir/home/chimera"
  xdg_config="$tmp_dir/xdg-config"
  xdg_cache="$tmp_dir/xdg-cache"
  xdg_data="$tmp_dir/xdg-data"
  local_bin="$tmp_dir/bin"
  fake_bin="$tmp_dir/fake-bin"
  bootstrap_env="$xdg_config/chimera/mesh_bootstrap.env"
  peer_env="$xdg_config/chimera/peer-egress.env"
  marker="$tmp_dir/upstream-token-injection-ran"
  injected_token="test-token;touch $marker"
  mkdir -p "$old_home/scripts" "$xdg_config/chimera" "$xdg_cache" "$xdg_data" "$local_bin" "$fake_bin"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$old_home/scripts/chimera.sh"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$old_home/scripts/chimera-sh"
  cat >"$fake_bin/systemctl" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
  cat >"$fake_bin/nft" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  chmod +x "$fake_bin/systemctl" "$fake_bin/nft"

  make_fake_release_archive_with_current_installer "$tmp_dir" "0.1.99"
  archive="$tmp_dir/chimera-pq-release.tar.gz"
  checksum="$tmp_dir/chimera-pq-release.tar.gz.sha256"

  set +e
  output="$(CHIMERA_ALLOW_LOCAL_RELEASE_SOURCE=1 \
    CHIMERA_INSTALL_NODE_ROLE=node \
    CHIMERA_HOME="$old_home" \
    CHIMERA_LOCAL_BIN="$local_bin" \
    HOME="$tmp_dir/home/user" \
    XDG_CONFIG_HOME="$xdg_config" \
    XDG_CACHE_HOME="$xdg_cache" \
    XDG_DATA_HOME="$xdg_data" \
    PATH="$fake_bin:$PATH" \
    CHIMERA_PEER_EGRESS_TOKEN="$injected_token" \
    bash "$ROOT_DIR/scripts/install_release.sh" "$archive" "$checksum" 2>&1)"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "mesh bootstrap env shell quoting install failed: $output"
  [[ ! -f "$marker" ]] || fail "peer token injection executed during install"
  ! grep -q '^CHIMERA_PEER_EGRESS_TOKEN=' "$bootstrap_env" \
    || fail "mesh bootstrap env leaked peer token"
  grep -q '^CHIMERA_PEER_EGRESS_TOKEN=' "$peer_env" \
    || fail "peer env did not receive peer token"
  bootstrap_env_mode="$(stat -c '%a' "$bootstrap_env")"
  [[ "$bootstrap_env_mode" == "600" ]] || fail "mesh bootstrap env mode was not private"
  peer_env_mode="$(stat -c '%a' "$peer_env")"
  [[ "$peer_env_mode" == "600" ]] || fail "peer env mode was not private"
  set -a
  # shellcheck disable=SC1090
  source "$peer_env"
  set +a
  [[ "${CHIMERA_PEER_EGRESS_TOKEN:-}" == "$injected_token" ]] \
    || fail "shell quoted private peer token did not round-trip"
  [[ ! -f "$marker" ]] || fail "peer token injection executed during explicit source"
  rm -rf "$tmp_dir"
)

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

case_failed_launcher_link_restores_previous_release() (
  local tmp_dir old_home archive checksum rc fake_bin
  tmp_dir="$(mktemp -d)"
  old_home="$tmp_dir/home/chimera"
  fake_bin="$tmp_dir/fake-bin"
  mkdir -p "$old_home/scripts" "$tmp_dir/bin" "$fake_bin"
  cat >"$fake_bin/ln" <<'EOF'
#!/usr/bin/env bash
for arg in "$@"; do
  if [[ "$arg" == */bin/chimera ]]; then
    exit 77
  fi
done
exec /usr/bin/ln "$@"
EOF
  chmod +x "$fake_bin/ln"
  printf 'old-release\n' >"$old_home/old_marker"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$old_home/scripts/chimera.sh"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$old_home/scripts/chimera-sh"
  make_fake_release_archive "$tmp_dir" "0.1.99" "exit 0"
  archive="$tmp_dir/chimera-pq-release.tar.gz"
  checksum="$tmp_dir/chimera-pq-release.tar.gz.sha256"

  set +e
  CHIMERA_ALLOW_LOCAL_RELEASE_SOURCE=1 \
  CHIMERA_HOME="$old_home" \
  CHIMERA_LOCAL_BIN="$tmp_dir/bin" \
  PATH="$fake_bin:$PATH" \
    bash "$ROOT_DIR/scripts/install_release.sh" "$archive" "$checksum" >/dev/null 2>&1
  rc=$?
  set -e
  [[ "$rc" -ne 0 ]] || fail "expected launcher link failure to return non-zero"
  [[ -f "$old_home/old_marker" ]] || fail "previous release was not restored after launcher link failure"
  [[ ! -f "$old_home/.chimera_release_version" || "$(cat "$old_home/.chimera_release_version" 2>/dev/null)" != "0.1.99" ]] || fail "failed release remained installed after launcher link failure"
  rm -rf "$tmp_dir"
)

case_peer_update_metadata_does_not_execute_peer_bootstrap() (
  local tmp_dir peer_dir metadata_url bootstrap_url calls helper rc
  tmp_dir="$(mktemp -d)"
  peer_dir="$tmp_dir/peer"
  helper="$tmp_dir/chimera-bootstrap"
  mkdir -p "$peer_dir"
  printf '%s\n' '{"status":"ok","kind":"chimera_peer_update_metadata","version":"0.1.99","archive":"http://peer.invalid/chimera-pq-release.tar.gz","checksum":"http://peer.invalid/chimera-pq-release.tar.gz.sha256","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}' >"$peer_dir/metadata.json"
  printf '#!/usr/bin/env bash\ntouch "%s/executed"\nVERSION="0.1.99"\nARCHIVE_URL_DEFAULT="http://evil.invalid/archive.tar.gz"\nCHECKSUM_URL_DEFAULT="http://evil.invalid/archive.tar.gz.sha256"\n' "$tmp_dir" >"$peer_dir/chimera.sh"
  cat >"$helper" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "${1:-}" == "parse-peer-metadata" ]] || exit 64
printf '%s\n%s\n%s\n%s\n' \
  0.1.99 \
  http://peer.invalid/chimera-pq-release.tar.gz \
  http://peer.invalid/chimera-pq-release.tar.gz.sha256 \
  aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
EOF
  chmod +x "$helper"
  metadata_url="http://peer.invalid/metadata.json"
  bootstrap_url="http://peer.invalid/chimera.sh"
  calls="$tmp_dir/calls"
  download_url_to_file() {
    printf '%s\n' "$1" >>"$calls"
    case "$1" in
      "$metadata_url"?*)
        cp "$peer_dir/metadata.json" "$2"
        ;;
      "$bootstrap_url"?*)
        cp "$peer_dir/chimera.sh" "$2"
        ;;
      *)
        return 1
        ;;
    esac
  }
  remote_archive_sha256() {
    printf '%064d\n' 1
  }
  install_update_from_release_metadata() {
    [[ "$1" == "peer" ]] || fail "expected peer source"
    [[ "$2" == "0.1.99" ]] || fail "peer metadata version not parsed"
    [[ "$3" == "http://peer.invalid/chimera-pq-release.tar.gz" ]] || fail "peer metadata archive not parsed"
    [[ "$4" == "http://peer.invalid/chimera-pq-release.tar.gz.sha256" ]] || fail "peer metadata checksum not parsed"
    return 0
  }

  CHIMERA_BOOTSTRAP_BIN="$helper" try_update_from_bootstrap_source "peer" "$bootstrap_url" "0.1.0" "" -start >/dev/null 2>&1 || rc=$?
  rc="${rc:-0}"
  [[ "$rc" -eq 0 ]] || fail "expected peer metadata update path to succeed"
  [[ ! -f "$tmp_dir/executed" ]] || fail "peer bootstrap script was executed"
  [[ "$(cat "$calls")" == *"$metadata_url"* ]] || fail "peer metadata endpoint was not used"
  [[ "$(cat "$calls")" != *"$bootstrap_url"* ]] || fail "peer bootstrap endpoint should not be downloaded for peer metadata"
  rm -rf "$tmp_dir"
)

case_peer_metadata_sha_mismatch_blocks_install() (
  local tmp_dir test_root old_root output rc
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  mkdir -p "$test_root/scripts"
  cat >"$test_root/scripts/install_release.sh" <<'EOF'
#!/usr/bin/env bash
echo "installer_must_not_run" >&2
exit 90
EOF
  chmod +x "$test_root/scripts/install_release.sh"
  ROOT_DIR="$test_root"
  remote_archive_sha256() {
    printf '%s\n' "1111111111111111111111111111111111111111111111111111111111111111"
  }

  output="$(install_update_from_release_metadata \
    peer \
    0.1.99 \
    http://peer.invalid/chimera-pq-release.tar.gz \
    http://peer.invalid/chimera-pq-release.tar.gz.sha256 \
    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa \
    0.1.0 \
    deadbeef \
    -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  ROOT_DIR="$old_root"
  [[ "$rc" -eq 3 ]] || fail "expected metadata checksum mismatch to block, got rc=$rc"
  [[ "$output" == *"reason=metadata_checksum_mismatch"* ]] || fail "missing metadata checksum mismatch diagnostic"
  [[ "$output" != *"installer_must_not_run"* ]] || fail "installer ran despite metadata checksum mismatch"
  rm -rf "$tmp_dir"
)

case_same_version_checksum_mismatch_blocks() (
  local tmp_dir test_root old_root output rc
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  mkdir -p "$test_root/scripts"
  cat >"$test_root/scripts/install_release.sh" <<'EOF'
#!/usr/bin/env bash
echo "installer_must_not_run" >&2
exit 91
EOF
  chmod +x "$test_root/scripts/install_release.sh"
  ROOT_DIR="$test_root"
  remote_archive_sha256() {
    printf '%s\n' "2222222222222222222222222222222222222222222222222222222222222222"
  }

  output="$(install_update_from_release_metadata \
    github \
    0.1.99 \
    http://github.invalid/chimera-pq-release.tar.gz \
    http://github.invalid/chimera-pq-release.tar.gz.sha256 \
    "" \
    0.1.99 \
    1111111111111111111111111111111111111111111111111111111111111111 \
    -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  ROOT_DIR="$old_root"
  [[ "$rc" -eq 3 ]] || fail "expected same-version checksum mismatch to block, got rc=$rc"
  [[ "$output" == *"reason=same_version_checksum_mismatch"* ]] || fail "missing same-version checksum mismatch diagnostic"
  [[ "$output" == *"action=block"* ]] || fail "same-version checksum mismatch was not blocking"
  [[ "$output" != *"installer_must_not_run"* ]] || fail "installer ran despite same-version checksum mismatch"
  rm -rf "$tmp_dir"
)

case_same_version_missing_local_checksum_blocks() (
  local tmp_dir test_root old_root output rc
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  mkdir -p "$test_root/scripts"
  cat >"$test_root/scripts/install_release.sh" <<'EOF'
#!/usr/bin/env bash
echo "installer_must_not_run" >&2
exit 92
EOF
  chmod +x "$test_root/scripts/install_release.sh"
  ROOT_DIR="$test_root"
  remote_archive_sha256() {
    printf '%s\n' "2222222222222222222222222222222222222222222222222222222222222222"
  }

  output="$(install_update_from_release_metadata \
    github \
    0.1.99 \
    http://github.invalid/chimera-pq-release.tar.gz \
    http://github.invalid/chimera-pq-release.tar.gz.sha256 \
    "" \
    0.1.99 \
    "" \
    -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  ROOT_DIR="$old_root"
  [[ "$rc" -eq 3 ]] || fail "expected same-version missing local checksum to block, got rc=$rc"
  [[ "$output" == *"reason=local_checksum_missing"* ]] || fail "missing local checksum diagnostic"
  [[ "$output" == *"action=block"* ]] || fail "missing local checksum was not blocking"
  [[ "$output" != *"installer_must_not_run"* ]] || fail "installer ran despite missing local checksum"
  rm -rf "$tmp_dir"
)

case_missing_root_checksum_self_heals_from_released_bundle() (
  local tmp_dir test_root old_root output rc archive checksum expected_sha
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  mkdir -p "$test_root/releases"
  printf '%s\n' 0.1.99 >"$test_root/.chimera_release_version"
  archive="$test_root/releases/chimera-pq-release.tar.gz"
  checksum="$test_root/releases/chimera-pq-release.tar.gz.sha256"
  printf '%s\n' "bundle" >"$archive"
  expected_sha="$(sha256sum "$archive" | awk '{print $1}')"
  printf '%s  chimera-pq-release.tar.gz\n' "$expected_sha" >"$checksum"
  ROOT_DIR="$test_root"
  remote_archive_sha256() {
    printf '%s\n' "$expected_sha"
  }

  output="$(read_local_runtime_bundle_sha 2>&1)" || rc=$?
  rc="${rc:-0}"
  ROOT_DIR="$old_root"
  [[ "$rc" -eq 0 ]] || fail "self-heal from released bundle should not fail, got rc=$rc"
  [[ "$output" == "$expected_sha" ]] || fail "self-heal did not return expected checksum"
  [[ "$(cat "$test_root/.chimera_release_bundle.sha256")" == "$expected_sha" ]] || fail "self-heal did not persist root checksum"
  rm -rf "$tmp_dir"
)

case_invalid_local_version_self_heals_from_released_bundle() (
  local tmp_dir test_root old_root archive checksum output rc
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  mkdir -p "$test_root/releases" "$tmp_dir/archive"
  printf '%s\n' 20260614-010203 >"$test_root/.chimera_release_version"
  make_fake_release_archive "$tmp_dir/archive" 0.1.99 "exit 0"
  archive="$tmp_dir/archive/chimera-pq-release.tar.gz"
  checksum="$tmp_dir/archive/chimera-pq-release.tar.gz.sha256"
  cp "$archive" "$test_root/releases/chimera-pq-release.tar.gz"
  cp "$checksum" "$test_root/releases/chimera-pq-release.tar.gz.sha256"
  ROOT_DIR="$test_root"

  output="$(read_local_runtime_version_from_release_bundle 2>&1)" || rc=$?
  rc="${rc:-0}"
  ROOT_DIR="$old_root"
  [[ "$rc" -eq 0 ]] || fail "version self-heal from released bundle should not fail, got rc=$rc"
  [[ "$output" == "0.1.99" ]] || fail "version self-heal did not return expected version"
  [[ "$(cat "$test_root/.chimera_release_version")" == "0.1.99" ]] || fail "version self-heal did not persist repaired version"
  rm -rf "$tmp_dir"
)

case_invalid_local_version_uses_repaired_version_before_update_choice() (
  local tmp_dir test_root old_root archive checksum output rc
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  mkdir -p "$test_root/releases" "$tmp_dir/archive"
  printf '%s\n' 20260614-010203 >"$test_root/.chimera_release_version"
  make_fake_release_archive "$tmp_dir/archive" 0.1.99 "exit 0"
  archive="$tmp_dir/archive/chimera-pq-release.tar.gz"
  checksum="$tmp_dir/archive/chimera-pq-release.tar.gz.sha256"
  cp "$archive" "$test_root/releases/chimera-pq-release.tar.gz"
  cp "$checksum" "$test_root/releases/chimera-pq-release.tar.gz.sha256"
  ROOT_DIR="$test_root"
  read_release_metadata_from_source() {
    case "${1:-}" in
      github)
        printf '%s\n%s\n%s\n' \
          0.1.98 \
          http://github.invalid/chimera-pq-release.tar.gz \
          http://github.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *)
        return 2
        ;;
    esac
  }
  load_update_gitvers_bootstrap_urls() { return 0; }
  load_update_peer_bootstrap_urls_for_args() { return 0; }
  remote_archive_sha256() {
    printf '%s\n' "9999999999999999999999999999999999999999999999999999999999999999"
  }

  output="$(auto_update_if_needed -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  ROOT_DIR="$old_root"
  [[ "$rc" -eq 0 ]] || fail "repaired local version should prevent downgrade, got rc=$rc"
  [[ "$output" == *"chimera_update=no_newer_release current_version=0.1.99 action=continue reason=highest_available_not_newer"* ]] || fail "repaired local version was not used for update choice"
  [[ "$(cat "$test_root/.chimera_release_version")" == "0.1.99" ]] || fail "repaired local version was not persisted before update choice"
  rm -rf "$tmp_dir"
)

case_chimera_sh_sources_update_runtime_state_helper() (
  rg -n 'source "\$ROOT_DIR/scripts/chimera-update-runtime-state\.sh"' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null \
    || fail "update script does not source runtime state helper"
  rg -n 'read_local_runtime_bundle_sha' "$ROOT_DIR/scripts/chimera-update-runtime-state.sh" >/dev/null \
    || fail "runtime state helper missing bundle sha reader"
  rg -n 'sha256_file' "$ROOT_DIR/scripts/chimera-update-runtime-state.sh" >/dev/null \
    || fail "runtime state helper missing sha helper"
  rg -n 'read_local_runtime_version_from_release_bundle' "$ROOT_DIR/scripts/chimera-update-runtime-state.sh" >/dev/null \
    || fail "runtime state helper missing release bundle version reader"
)

case_github_unavailable_peer_newer_updates_and_reruns() (
  local tmp_dir test_root old_root calls installed_marker rerun_args expected_sha output rc
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  calls="$tmp_dir/calls"
  installed_marker="$tmp_dir/installed"
  rerun_args="$tmp_dir/rerun_args"
  expected_sha="1111111111111111111111111111111111111111111111111111111111111111"
  mkdir -p "$test_root/scripts"

  cat >"$test_root/scripts/install_release.sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' install >>"$calls"
touch "$installed_marker"
exit 0
EOF
  chmod +x "$test_root/scripts/install_release.sh"

  ROOT_DIR="$test_root"
  read_local_runtime_version() {
    if [[ -f "$installed_marker" ]]; then
      printf '%s\n' 0.1.99
    else
      printf '%s\n' 0.1.0
    fi
  }
  read_local_runtime_bundle_sha() {
    if [[ -f "$installed_marker" ]]; then
      printf '%s\n' "$expected_sha"
    else
      printf '%s\n' deadbeef
    fi
  }
  read_release_metadata_from_source() {
    case "${1:-}" in
      github)
        printf '%s\n' meta:github >>"$calls"
        return 2
        ;;
      peer)
        printf '%s\n' meta:peer >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.99 \
          http://peer.invalid/chimera-pq-release.tar.gz \
          http://peer.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *)
        return 2
        ;;
    esac
  }
  load_update_peer_bootstrap_urls() {
    printf '%s\n' http://peer.invalid/chimera.sh
  }
  remote_archive_sha256() {
    printf '%s\n' "$expected_sha"
  }
  rerun_after_update() {
    printf '%s\n' "rerun:$*" >>"$calls"
    printf '%s\n' "$*" >"$rerun_args"
    return 0
  }

  output="$(auto_update_if_needed -start 2>&1)"
  rc=$?
  ROOT_DIR="$old_root"
  [[ "$rc" -eq 0 ]] || fail "expected peer update after github outage to continue"
  [[ "$output" == *"chimera_update=available source=peer"* ]] || fail "missing peer update diagnostic"
  [[ -f "$installed_marker" ]] || fail "fake installer was not invoked"
  [[ "$(cat "$rerun_args")" == "-restart" ]] || fail "original command was not restarted after update"
  [[ "$(cat "$calls")" == $'meta:github\nmeta:peer\ninstall\nrerun:-restart' ]] || fail "unexpected peer update call order"
  rm -rf "$tmp_dir"
)

case_github_current_stops_before_peer_newer_update() (
  local tmp_dir test_root old_root calls rerun_args output rc github_sha peer_sha
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  calls="$tmp_dir/calls"
  rerun_args="$tmp_dir/rerun_args"
  github_sha="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  peer_sha="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
  mkdir -p "$test_root/scripts"

  cat >"$test_root/scripts/install_release.sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
archive="\${1:-}"
case "\$archive" in
  *peer.invalid*)
    printf '%s\n' install:peer >>"$calls"
    touch "$tmp_dir/peer_installed"
    exit 0
    ;;
  *)
    printf '%s\n' install:unexpected >>"$calls"
    exit 1
    ;;
esac
EOF
  chmod +x "$test_root/scripts/install_release.sh"

  ROOT_DIR="$test_root"
  read_local_runtime_version() {
    if [[ -f "$tmp_dir/peer_installed" ]]; then
      printf '%s\n' 0.1.100
    else
      printf '%s\n' 0.1.99
    fi
  }
  read_local_runtime_bundle_sha() {
    if [[ -f "$tmp_dir/peer_installed" ]]; then
      printf '%s\n' "$peer_sha"
    else
      printf '%s\n' "$github_sha"
    fi
  }
  read_release_metadata_from_source() {
    case "${1:-}" in
      github)
        printf '%s\n' meta:github >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.99 \
          http://github.invalid/chimera-pq-release.tar.gz \
          http://github.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      peer)
        printf '%s\n' meta:peer >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.100 \
          http://peer.invalid/chimera-pq-release.tar.gz \
          http://peer.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *)
        return 2
        ;;
    esac
  }
  load_update_peer_bootstrap_urls_for_args() {
    printf '%s\n' http://peer.invalid/chimera.sh
  }
  remote_archive_sha256() {
    case "${1:-}" in
      http://github.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$github_sha"
        ;;
      http://peer.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$peer_sha"
        ;;
      *)
        return 2
        ;;
    esac
  }
  rerun_after_update() {
    printf '%s\n' "rerun:$*" >>"$calls"
    printf '%s\n' "$*" >"$rerun_args"
    return 0
  }

  output="$(auto_update_if_needed -start 2>&1)"
  rc=$?
  ROOT_DIR="$old_root"
  [[ "$rc" -eq 0 ]] || fail "verified github current result should stop lower-trust newer update, got rc=$rc"
  [[ "$output" == *"chimera_update=source_current source=github"* ]] || fail "missing github current diagnostic"
  [[ "$output" == *"chimera_update=no_newer_release current_version=0.1.99 action=continue reason=highest_available_not_newer"* ]] || fail "missing final no-newer-release diagnostic"
  [[ "$output" != *"source=peer"* ]] || fail "peer should not be consulted after verified github current result"
  [[ "$(cat "$calls")" == $'meta:github' ]] || fail "unexpected lower-trust activity after verified github current result"
  rm -rf "$tmp_dir"
)

case_github_stale_stops_before_peer_newer_update() (
  local tmp_dir test_root old_root calls rerun_args output rc github_sha local_sha peer_sha
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  calls="$tmp_dir/calls"
  rerun_args="$tmp_dir/rerun_args"
  github_sha="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  local_sha="cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
  peer_sha="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
  mkdir -p "$test_root/scripts"

  cat >"$test_root/scripts/install_release.sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
archive="\${1:-}"
case "\$archive" in
  *peer.invalid*)
    printf '%s\n' install:peer >>"$calls"
    touch "$tmp_dir/peer_installed"
    exit 0
    ;;
  *)
    printf '%s\n' install:unexpected >>"$calls"
    exit 1
    ;;
esac
EOF
  chmod +x "$test_root/scripts/install_release.sh"

  ROOT_DIR="$test_root"
  read_local_runtime_version() {
    if [[ -f "$tmp_dir/peer_installed" ]]; then
      printf '%s\n' 0.1.101
    else
      printf '%s\n' 0.1.100
    fi
  }
  read_local_runtime_bundle_sha() {
    if [[ -f "$tmp_dir/peer_installed" ]]; then
      printf '%s\n' "$peer_sha"
    else
      printf '%s\n' "$local_sha"
    fi
  }
  read_release_metadata_from_source() {
    case "${1:-}" in
      github)
        printf '%s\n' meta:github >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.99 \
          http://github.invalid/chimera-pq-release.tar.gz \
          http://github.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      peer)
        printf '%s\n' meta:peer >>"$calls"
        printf '%s\n%s\n%s\n' \
          0.1.101 \
          http://peer.invalid/chimera-pq-release.tar.gz \
          http://peer.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *)
        return 2
        ;;
    esac
  }
  load_update_peer_bootstrap_urls_for_args() {
    printf '%s\n' http://peer.invalid/chimera.sh
  }
  remote_archive_sha256() {
    case "${1:-}" in
      http://github.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$github_sha"
        ;;
      http://peer.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$peer_sha"
        ;;
      *)
        return 2
        ;;
    esac
  }
  rerun_after_update() {
    printf '%s\n' "rerun:$*" >>"$calls"
    printf '%s\n' "$*" >"$rerun_args"
    return 0
  }

  output="$(auto_update_if_needed -start 2>&1)"
  rc=$?
  ROOT_DIR="$old_root"
  [[ "$rc" -eq 0 ]] || fail "verified github stale result should stop lower-trust newer update, got rc=$rc"
  [[ "$output" == *"chimera_update=source_stale source=github"* ]] || fail "missing stale github diagnostic"
  [[ "$output" == *"chimera_update=no_newer_release current_version=0.1.100 action=continue reason=highest_available_not_newer"* ]] || fail "missing final no-newer-release diagnostic"
  [[ "$output" != *"source=peer"* ]] || fail "peer should not be consulted after verified github stale result"
  [[ "$(cat "$calls")" == $'meta:github' ]] || fail "unexpected lower-trust activity after verified github stale result"
  rm -rf "$tmp_dir"
)

case_github_install_source_unavailable_falls_back_to_peer() (
  local tmp_dir test_root old_root rerun_args output rc github_sha peer_archive peer_checksum peer_sha fake_bin
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  rerun_args="$tmp_dir/rerun_args"
  fake_bin="$tmp_dir/fake-bin"
  mkdir -p "$test_root/scripts" "$fake_bin" "$tmp_dir/home/chimera" "$tmp_dir/bin"
  make_fake_release_archive_with_current_installer "$tmp_dir" "0.1.100"
  peer_archive="$tmp_dir/chimera-pq-release.tar.gz"
  peer_checksum="$tmp_dir/chimera-pq-release.tar.gz.sha256"
  peer_sha="$(awk '{print $1}' "$peer_checksum")"
  github_sha="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
  mkdir -p "$test_root/scripts"
  cat >"$test_root/scripts/install_release.sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
archive="\${1:-}"
case "\$archive" in
  *github.invalid*)
    echo "error: release archive download unavailable" >&2
    exit 2
    ;;
  *peer.invalid*)
    mkdir -p "\${CHIMERA_HOME:-$tmp_dir/home/chimera}"
    printf '%s\n' 0.1.100 >"\${CHIMERA_HOME:-$tmp_dir/home/chimera}/.chimera_release_version"
    printf '%s\n' "$peer_sha" >"\${CHIMERA_HOME:-$tmp_dir/home/chimera}/.chimera_release_bundle.sha256"
    exit 0
    ;;
  *)
    exit 1
    ;;
esac
EOF
  chmod +x "$test_root/scripts/install_release.sh"
  cat >"$fake_bin/curl" <<EOF
#!/usr/bin/env bash
set -euo pipefail
dest=""
url=""
while [[ "\$#" -gt 0 ]]; do
  case "\$1" in
    -o)
      dest="\${2:-}"
      shift 2
      ;;
    --disable|-fsSL)
      shift
      ;;
    --retry|--connect-timeout|--max-time)
      shift 2
      ;;
    *)
      url="\$1"
      shift
      ;;
  esac
done
case "\$url" in
  *github.invalid*)
    case "\$url" in
      *.sha256)
        printf '%s  chimera-pq-release.tar.gz\n' "$github_sha" >"\$dest"
        exit 0
        ;;
      *)
        exit 1
        ;;
    esac
    ;;
  *peer.invalid*)
    case "\$url" in
      *.sha256)
        cp "$peer_checksum" "\$dest"
        ;;
      *)
        cp "$peer_archive" "\$dest"
        ;;
    esac
    ;;
  *)
    exit 1
    ;;
esac
EOF
  chmod +x "$fake_bin/curl"

  ROOT_DIR="$test_root"
  CHIMERA_HOME="$tmp_dir/home/chimera"
  CHIMERA_LOCAL_BIN="$tmp_dir/bin"
  HOME="$tmp_dir/home/user"
  PATH="$fake_bin:$PATH"
  cat >"$fake_bin/systemctl" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
  cat >"$fake_bin/nft" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  chmod +x "$fake_bin/systemctl" "$fake_bin/nft"
  read_local_runtime_version() {
    if [[ -f "$CHIMERA_HOME/.chimera_release_version" ]]; then
      cat "$CHIMERA_HOME/.chimera_release_version"
    else
      printf '%s\n' 0.1.0
    fi
  }
  read_local_runtime_bundle_sha() {
    if [[ -f "$CHIMERA_HOME/.chimera_release_bundle.sha256" ]]; then
      cat "$CHIMERA_HOME/.chimera_release_bundle.sha256"
    else
      printf '%s\n' deadbeef
    fi
  }
  read_release_metadata_from_source() {
    case "${1:-}" in
      github)
        printf '%s\n%s\n%s\n' \
          0.1.100 \
          http://github.invalid/chimera-pq-release.tar.gz \
          http://github.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      peer)
        printf '%s\n%s\n%s\n' \
          0.1.100 \
          http://peer.invalid/chimera-pq-release.tar.gz \
          http://peer.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *)
        return 2
        ;;
    esac
  }
  load_update_peer_bootstrap_urls_for_args() {
    printf '%s\n' http://peer.invalid/chimera.sh
  }
  remote_archive_sha256() {
    case "${1:-}" in
      http://github.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$peer_sha"
        ;;
      http://peer.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$peer_sha"
        ;;
      *)
        return 2
        ;;
    esac
  }
  rerun_after_update() {
    printf '%s\n' "$*" >"$rerun_args"
    return 0
  }

  output="$(CHIMERA_HOME="$CHIMERA_HOME" CHIMERA_LOCAL_BIN="$CHIMERA_LOCAL_BIN" HOME="$HOME" auto_update_if_needed -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  [[ "$rc" -eq 0 ]] || fail "github install source unavailable should fall back to same verified peer release, got rc=$rc"
  [[ "$output" == *"reason=install_source_unavailable"* ]] || fail "missing github install source unavailable diagnostic"
  [[ "$output" == *"chimera_update=available source=peer"* ]] || fail "peer fallback was not reached after github install source unavailable"
  [[ "$(cat "$rerun_args")" == "-restart" ]] || fail "original command was not restarted after peer fallback"
  [[ "$(cat "$CHIMERA_HOME/.chimera_release_version")" == "0.1.100" ]] || fail "peer fallback did not install expected version"
  [[ "$(cat "$CHIMERA_HOME/.chimera_release_bundle.sha256")" == "$(awk '{print $1}' "$peer_checksum")" ]] || fail "peer fallback did not install expected checksum"
  rm -rf "$tmp_dir"
)

case_github_install_source_unavailable_blocks_peer_newer_release() (
  local tmp_dir test_root old_root calls output rc github_sha peer_sha
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  calls="$tmp_dir/calls"
  github_sha="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  peer_sha="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
  mkdir -p "$test_root/scripts"

  cat >"$test_root/scripts/install_release.sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
archive="\${1:-}"
case "\$archive" in
  *github.invalid*)
    printf '%s\n' install:github >>"$calls"
    exit 2
    ;;
  *peer.invalid*)
    printf '%s\n' install:peer >>"$calls"
    exit 0
    ;;
  *)
    exit 1
    ;;
esac
EOF
  chmod +x "$test_root/scripts/install_release.sh"

  ROOT_DIR="$test_root"
  read_local_runtime_version() { printf '%s\n' 0.1.0; }
  read_local_runtime_bundle_sha() { printf '%s\n' deadbeef; }
  read_release_metadata_from_source() {
    case "${1:-}" in
      github)
        printf '%s\n%s\n%s\n' \
          0.1.100 \
          http://github.invalid/chimera-pq-release.tar.gz \
          http://github.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      peer)
        printf '%s\n%s\n%s\n' \
          0.1.101 \
          http://peer.invalid/chimera-pq-release.tar.gz \
          http://peer.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *)
        return 2
        ;;
    esac
  }
  load_update_peer_bootstrap_urls_for_args() {
    printf '%s\n' http://peer.invalid/chimera.sh
  }
  remote_archive_sha256() {
    case "${1:-}" in
      http://github.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$github_sha"
        ;;
      http://peer.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$peer_sha"
        ;;
      *)
        return 2
        ;;
    esac
  }

  output="$(auto_update_if_needed -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  ROOT_DIR="$old_root"
  [[ "$rc" -ne 0 ]] || fail "divergent peer release should block after github install source outage"
  [[ "$output" == *"reason=install_source_unavailable"* ]] || fail "missing install source unavailable diagnostic before divergence block"
  [[ "$output" == *"reason=trusted_version_divergence"* ]] || fail "missing trusted version divergence block"
  [[ "$(cat "$calls")" == $'install:github' ]] || fail "peer install should not run after divergent version block"
  rm -rf "$tmp_dir"
)

case_github_install_source_unavailable_blocks_peer_checksum_divergence() (
  local tmp_dir test_root old_root calls output rc github_sha peer_sha
  tmp_dir="$(mktemp -d)"
  test_root="$tmp_dir/root"
  old_root="$ROOT_DIR"
  calls="$tmp_dir/calls"
  github_sha="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  peer_sha="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
  mkdir -p "$test_root/scripts"

  cat >"$test_root/scripts/install_release.sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
archive="\${1:-}"
case "\$archive" in
  *github.invalid*)
    printf '%s\n' install:github >>"$calls"
    exit 2
    ;;
  *peer.invalid*)
    printf '%s\n' install:peer >>"$calls"
    exit 0
    ;;
  *)
    exit 1
    ;;
esac
EOF
  chmod +x "$test_root/scripts/install_release.sh"

  ROOT_DIR="$test_root"
  read_local_runtime_version() { printf '%s\n' 0.1.0; }
  read_local_runtime_bundle_sha() { printf '%s\n' deadbeef; }
  read_release_metadata_from_source() {
    case "${1:-}" in
      github)
        printf '%s\n%s\n%s\n' \
          0.1.100 \
          http://github.invalid/chimera-pq-release.tar.gz \
          http://github.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      peer)
        printf '%s\n%s\n%s\n' \
          0.1.100 \
          http://peer.invalid/chimera-pq-release.tar.gz \
          http://peer.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *)
        return 2
        ;;
    esac
  }
  load_update_peer_bootstrap_urls_for_args() {
    printf '%s\n' http://peer.invalid/chimera.sh
  }
  remote_archive_sha256() {
    case "${1:-}" in
      http://github.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$github_sha"
        ;;
      http://peer.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$peer_sha"
        ;;
      *)
        return 2
        ;;
    esac
  }

  output="$(auto_update_if_needed -start 2>&1)" || rc=$?
  rc="${rc:-0}"
  ROOT_DIR="$old_root"
  [[ "$rc" -ne 0 ]] || fail "peer checksum divergence should block after github install source outage"
  [[ "$output" == *"reason=install_source_unavailable"* ]] || fail "missing install source unavailable diagnostic before checksum divergence block"
  [[ "$output" == *"reason=trusted_checksum_divergence"* ]] || fail "missing trusted checksum divergence block"
  [[ "$(cat "$calls")" == $'install:github' ]] || fail "peer install should not run after divergent checksum block"
  rm -rf "$tmp_dir"
)

case_gitvers_same_tier_divergence_blocks_after_first_current_result() (
  local tmp_dir calls output rc current_sha
  tmp_dir="$(mktemp -d)"
  calls="$tmp_dir/calls"
  current_sha="5656565656565656565656565656565656565656565656565656565656565656"
  read_local_runtime_version() { printf '%s\n' 0.1.141; }
  read_local_runtime_bundle_sha() { printf '%s\n' "$current_sha"; }
  read_release_metadata_from_source() {
    printf '%s\n' "${2:-}" >>"$calls"
    case "${2:-}" in
      http://gitvers-a.invalid/chimera.sh)
        printf '%s\n%s\n%s\n' \
          0.1.141 \
          http://gitvers-a.invalid/chimera-pq-release.tar.gz \
          http://gitvers-a.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      http://gitvers-b.invalid/chimera.sh)
        printf '%s\n%s\n%s\n' \
          0.1.142 \
          http://gitvers-b.invalid/chimera-pq-release.tar.gz \
          http://gitvers-b.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *)
        return 2
        ;;
    esac
  }
  load_update_gitvers_bootstrap_urls() {
    printf '%s\n' http://gitvers-a.invalid/chimera.sh
    printf '%s\n' http://gitvers-b.invalid/chimera.sh
  }
  load_update_peer_bootstrap_urls_for_args() { return 0; }
  remote_archive_sha256() {
    case "${1:-}" in
      http://gitvers-a.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$current_sha"
        ;;
      http://gitvers-b.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "7878787878787878787878787878787878787878787878787878787878787878"
        ;;
      *)
        return 2
        ;;
    esac
  }

  set +e
  output="$(auto_update_if_needed -restart 2>&1)"
  rc=$?
  set -e
  [[ "$rc" -ne 0 ]] || fail "same-tier gitvers divergence should block after first current result"
  [[ "$output" == *"reason=trusted_version_divergence"* ]] || fail "missing same-tier divergence block"
  [[ "$(cat "$calls")" == $'https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh\nhttp://gitvers-a.invalid/chimera.sh\nhttp://gitvers-b.invalid/chimera.sh' ]] || fail "second gitvers mirror was not checked after first current result"
  rm -rf "$tmp_dir"
)

case_peer_same_tier_divergence_blocks_after_first_current_result() (
  local tmp_dir calls output rc current_sha
  tmp_dir="$(mktemp -d)"
  calls="$tmp_dir/calls"
  current_sha="6767676767676767676767676767676767676767676767676767676767676767"
  read_local_runtime_version() { printf '%s\n' 0.1.141; }
  read_local_runtime_bundle_sha() { printf '%s\n' "$current_sha"; }
  read_release_metadata_from_source() {
    printf '%s:%s\n' "${1:-}" "${2:-}" >>"$calls"
    case "${1:-}:${2:-}" in
      github:https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh)
        return 2
        ;;
      peer:http://peer-a.invalid/chimera.sh)
        printf '%s\n%s\n%s\n' \
          0.1.141 \
          http://peer-a.invalid/chimera-pq-release.tar.gz \
          http://peer-a.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      peer:http://peer-b.invalid/chimera.sh)
        printf '%s\n%s\n%s\n' \
          0.1.142 \
          http://peer-b.invalid/chimera-pq-release.tar.gz \
          http://peer-b.invalid/chimera-pq-release.tar.gz.sha256
        ;;
      *)
        return 2
        ;;
    esac
  }
  load_update_gitvers_bootstrap_urls() { return 0; }
  load_update_peer_bootstrap_urls_for_args() {
    printf '%s\n' http://peer-a.invalid/chimera.sh
    printf '%s\n' http://peer-b.invalid/chimera.sh
  }
  remote_archive_sha256() {
    case "${1:-}" in
      http://peer-a.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "$current_sha"
        ;;
      http://peer-b.invalid/chimera-pq-release.tar.gz)
        printf '%s\n' "8686868686868686868686868686868686868686868686868686868686868686"
        ;;
      *)
        return 2
        ;;
    esac
  }

  set +e
  output="$(auto_update_if_needed -restart 2>&1)"
  rc=$?
  set -e
  [[ "$rc" -ne 0 ]] || fail "same-tier peer divergence should block after first current result"
  [[ "$output" == *"reason=trusted_version_divergence"* ]] || fail "missing same-tier peer divergence block"
  [[ "$(cat "$calls")" == $'github:https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh\npeer:http://peer-a.invalid/chimera.sh\npeer:http://peer-b.invalid/chimera.sh' ]] || fail "second peer mirror was not checked after first current result"
  rm -rf "$tmp_dir"
)

case_connect_peer_update_url_does_not_use_general_peer_list() (
  selected_connect_peer_update_bootstrap_url() { return 1; }
  load_update_peer_bootstrap_urls() {
    printf '%s\n' http://general-peer.invalid/chimera.sh
  }

  local output
  output="$(load_update_peer_bootstrap_urls_for_args -connect node-a)"
  [[ -z "$output" ]] || fail "connect without selected peer update URL used general peer list"
)

case_connect_peer_update_url_precedes_general_peer_list() (
  selected_connect_peer_update_bootstrap_url() {
    printf '%s\n' http://selected-peer.invalid/chimera.sh
  }
  load_update_peer_bootstrap_urls() {
    printf '%s\n' http://general-peer.invalid/chimera.sh
  }

  local output
  output="$(load_update_peer_bootstrap_urls_for_args -connect node-a)"
  [[ "$output" == "http://selected-peer.invalid/chimera.sh" ]] || fail "connect did not use selected peer update URL only"
)

case_update_bootstrap_url_rejects_userinfo() (
  if validate_update_bootstrap_url "http://user@peer.invalid/chimera.sh"; then
    fail "userinfo update bootstrap URL accepted"
  fi
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
case_gitverse_repo_root_normalizes_to_raw_bootstrap
case_gitverse_content_viewer_normalizes_to_raw_bootstrap
case_gitverse_archive_viewer_normalizes_to_raw_bootstrap
case_gitverse_default_source_loads_without_operator_config
case_update_download_uses_bounded_bootstrap_timeouts
case_update_download_uses_bounded_curl_args
case_update_download_timeout_bounds_slow_helper
case_newer_release_with_unreachable_checksum_continues
case_newer_release_with_unreachable_checksum_falls_back_to_peer
case_update_required_install_failure_blocks
case_confirmed_update_missing_local_installer_blocks
case_peer_confirmed_update_failure_blocks_start
case_github_install_failure_is_not_masked_by_peer_noop
case_github_invalid_does_not_try_peer_fallback
case_github_invalid_bootstrap_parse_does_not_try_peer_fallback
case_github_unavailable_gitvers_newer_updates_before_peer
case_github_unavailable_default_gitvers_newer_updates_without_operator_config
case_github_unavailable_gitvers_unavailable_falls_back_to_peer
case_github_unavailable_gitvers_current_stops_before_peer_newer_update
case_github_unavailable_gitvers_invalid_does_not_try_peer
case_github_stale_gitvers_newer_updates_before_peer
case_github_stale_gitvers_current_reports_no_newer_release
case_github_current_gitvers_current_peer_newer_updates_from_peer
case_github_stale_gitvers_stale_peer_newer_updates_from_peer
case_github_unavailable_gitvers_same_version_checksum_mismatch_blocks_before_peer
case_missing_local_version_blocks_when_update_unavailable
case_missing_local_version_uses_update_repair
case_semver_update_order
case_auto_update_preserves_bound_transit_env
case_auto_update_heals_legacy_bound_transit_default
case_peer_egress_env_shell_quotes_lane_bindings_path
case_auto_update_preserves_quoted_lane_bindings_env
case_peer_token_stays_in_private_peer_env
case_failed_install_restores_previous_release
case_failed_launcher_link_restores_previous_release
case_peer_update_metadata_does_not_execute_peer_bootstrap
case_peer_metadata_sha_mismatch_blocks_install
case_same_version_checksum_mismatch_blocks
case_same_version_missing_local_checksum_blocks
case_missing_root_checksum_self_heals_from_released_bundle
case_invalid_local_version_self_heals_from_released_bundle
case_invalid_local_version_uses_repaired_version_before_update_choice
case_chimera_sh_sources_update_runtime_state_helper
case_github_unavailable_peer_newer_updates_and_reruns
case_github_current_stops_before_peer_newer_update
case_github_stale_stops_before_peer_newer_update
case_github_install_source_unavailable_falls_back_to_peer
case_github_install_source_unavailable_blocks_peer_newer_release
case_github_install_source_unavailable_blocks_peer_checksum_divergence
case_gitvers_same_tier_divergence_blocks_after_first_current_result
case_peer_same_tier_divergence_blocks_after_first_current_result
case_connect_peer_update_url_does_not_use_general_peer_list
case_connect_peer_update_url_precedes_general_peer_list
case_update_bootstrap_url_rejects_userinfo
case_control_requires_update_first_marker
case_real_control_delegates_direct_start_and_mesh

echo "chimera_update_contract_smoke=pass"
