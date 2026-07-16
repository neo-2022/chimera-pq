#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="${ROOT_DIR}/target"
VERIFY_ONLY=0
if [[ "${1:-}" == "--verify-only" || "${1:-}" == "-verify" ]]; then
  VERIFY_ONLY=1
  shift || true
fi
SOURCE_BOOTSTRAP="${1:-${TARGET_DIR}/chimera.sh}"
SOURCE_ARCHIVE="${2:-${TARGET_DIR}/chimera-pq-release.tar.gz}"
SOURCE_CHECKSUM="${3:-${TARGET_DIR}/chimera-pq-release.tar.gz.sha256}"
GITVERS_OWNER="${CHIMERA_GITVERS_OWNER:-ArtReg}"
GITVERS_REPO="${CHIMERA_GITVERS_REPO:-chimera}"
GITVERS_BRANCH="${CHIMERA_GITVERS_BRANCH:-main}"
GITVERS_TOKEN="${CHIMERA_GITVERS_TOKEN:-}"
GITVERS_ARCHIVE_NAME="${CHIMERA_GITVERS_ARCHIVE_NAME:-chimera-pq-release.tar.gz}"
GITVERS_CHECKSUM_NAME="${CHIMERA_GITVERS_CHECKSUM_NAME:-${GITVERS_ARCHIVE_NAME}.sha256}"
GITVERS_BOOTSTRAP_NAME="${CHIMERA_GITVERS_BOOTSTRAP_NAME:-chimera.sh}"
GITVERS_GIT_NAME="${CHIMERA_GITVERS_GIT_NAME:-CHIMERA Release Sync}"
GITVERS_GIT_EMAIL="${CHIMERA_GITVERS_GIT_EMAIL:-chimera-release-sync@local}"

fail() {
  echo "gitvers_sync=fail reason=$1" >&2
  exit 1
}

require_file() {
  local path="${1:?path_required}"
  [[ -s "$path" ]] || fail "missing_file:${path}"
}

sha256_file() {
  local file="${1:?file_required}"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
    return 0
  fi
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{print $1}'
    return 0
  fi
  fail "missing_sha256_tool"
}

extract_bootstrap_version() {
  local file="${1:?file_required}"
  awk -F'"' '/^VERSION=/{print $2; exit}' "$file"
}

extract_checksum_value() {
  local file="${1:?file_required}"
  awk '{print $1; exit}' "$file" | tr -d '[:space:]'
}

extract_bootstrap_url() {
  local file="${1:?file_required}"
  local var="${2:?var_required}"
  grep -m1 "^${var}=\"" "$file" | cut -d'"' -f2 | tr -d '[:space:]'
}

cache_buster() {
  date +%s%N 2>/dev/null || date +%s
}

gitvers_raw_base() {
  printf 'https://gitverse.ru/api/repos/%s/%s/raw/branch/%s' \
    "$GITVERS_OWNER" "$GITVERS_REPO" "$GITVERS_BRANCH"
}

gitvers_raw_url() {
  local path="${1:?path_required}"
  local cb
  cb="$(cache_buster)"
  printf '%s/%s?cb=%s\n' "$(gitvers_raw_base)" "$path" "$cb"
}

download_gitvers_raw_file() {
  local url="${1:?url_required}"
  local dest="${2:?dest_required}"
  local curl_rc=0 wget_rc=0
  local curl_present="no" wget_present="no"
  command -v curl >/dev/null 2>&1 && curl_present="yes"
  command -v wget >/dev/null 2>&1 && wget_present="yes"

  if [[ "$curl_present" == "yes" ]]; then
    if env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
      curl --disable -fsSL --retry 3 --connect-timeout 10 --max-time 60 "$url" -o "$dest"
    then
      return 0
    fi
    curl_rc=$?
    echo "gitvers_verify=download_fail tool=curl url=$url rc=$curl_rc" >&2
  fi

  if [[ "$wget_present" == "yes" ]]; then
    if env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
      wget --no-config --tries=3 --timeout=10 --dns-timeout=10 --connect-timeout=10 --read-timeout=60 --waitretry=1 -qO "$dest" "$url"
    then
      return 0
    fi
    wget_rc=$?
    echo "gitvers_verify=download_fail tool=wget url=$url rc=$wget_rc" >&2
  fi

  fail "verify_download_failed"
}

verify_gitvers_remote_release() {
  local expected_version="${1:?expected_version_required}"
  local expected_sha="${2:?expected_sha_required}"
  local verify_tmp remote_bootstrap remote_archive remote_checksum
  local remote_version remote_archive_url remote_checksum_url remote_checksum_value

  verify_tmp="$(mktemp -d)"
  trap 'rm -rf "$verify_tmp"' RETURN

  remote_bootstrap="$verify_tmp/remote-chimera.sh"
  remote_checksum="$verify_tmp/remote-checksum.sha256"
  remote_archive="$verify_tmp/remote-archive.tar.gz"

  download_gitvers_raw_file "$(gitvers_raw_url "$GITVERS_BOOTSTRAP_NAME")" "$remote_bootstrap" \
    || fail "verify_bootstrap_download_failed"

  remote_version="$(extract_bootstrap_version "$remote_bootstrap")"
  [[ "$remote_version" == "$expected_version" ]] \
    || fail "verify_version_mismatch expected=${expected_version} actual=${remote_version}"

  remote_archive_url="$(extract_bootstrap_url "$remote_bootstrap" "ARCHIVE_URL_DEFAULT")"
  remote_checksum_url="$(extract_bootstrap_url "$remote_bootstrap" "CHECKSUM_URL_DEFAULT")"
  [[ -n "$remote_archive_url" ]] || fail "verify_bootstrap_missing_archive_url"
  [[ -n "$remote_checksum_url" ]] || fail "verify_bootstrap_missing_checksum_url"

  download_gitvers_raw_file "$(gitvers_raw_url "$GITVERS_CHECKSUM_NAME")" "$remote_checksum" \
    || fail "verify_checksum_download_failed"
  remote_checksum_value="$(extract_checksum_value "$remote_checksum")"
  [[ "$remote_checksum_value" == "$expected_sha" ]] \
    || fail "verify_checksum_mismatch expected=${expected_sha} actual=${remote_checksum_value}"

  download_gitvers_raw_file "$(gitvers_raw_url "$GITVERS_ARCHIVE_NAME")" "$remote_archive" \
    || fail "verify_archive_download_failed"
  [[ "$(sha256_file "$remote_archive")" == "$expected_sha" ]] \
    || fail "verify_archive_checksum_mismatch"

  echo "gitvers_verify=ok version=${expected_version} checksum_ok=true"
}

require_file "$SOURCE_BOOTSTRAP"
require_file "$SOURCE_ARCHIVE"
require_file "$SOURCE_CHECKSUM"

version="$(extract_bootstrap_version "$SOURCE_BOOTSTRAP")"
[[ "$version" =~ ^[0-9]+[.][0-9]+[.][0-9]+$ ]] || fail "bad_bootstrap_version"

expected_sha="$(extract_checksum_value "$SOURCE_CHECKSUM")"
[[ -n "$expected_sha" ]] || fail "empty_checksum"
actual_sha="$(sha256_file "$SOURCE_ARCHIVE")"
[[ "$expected_sha" == "$actual_sha" ]] || fail "checksum_mismatch"

if [[ "$VERIFY_ONLY" -eq 1 ]]; then
  verify_gitvers_remote_release "$version" "$expected_sha"
  exit 0
fi

[[ -n "$GITVERS_TOKEN" ]] || fail "missing_token"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT
repo_dir="$tmp_dir/repo"
rendered_bootstrap="$tmp_dir/${GITVERS_BOOTSTRAP_NAME}"
authenticated_url="https://${GITVERS_TOKEN}@gitverse.ru/${GITVERS_OWNER}/${GITVERS_REPO}"

if ! git clone --depth 1 --branch "$GITVERS_BRANCH" "$authenticated_url" "$repo_dir" >/dev/null 2>&1; then
  fail "clone_failed"
fi

render_gitvers_bootstrap() {
  local src="${1:?src_required}"
  local dest="${2:?dest_required}"
  local base="https://gitverse.ru/api/repos/${GITVERS_OWNER}/${GITVERS_REPO}/raw/branch/${GITVERS_BRANCH}"
  cp "$src" "$dest"
  sed -i \
    -e "s#^ARCHIVE_URL_DEFAULT=.*#ARCHIVE_URL_DEFAULT=\"${base}/${GITVERS_ARCHIVE_NAME}\"#" \
    -e "s#^CHECKSUM_URL_DEFAULT=.*#CHECKSUM_URL_DEFAULT=\"${base}/${GITVERS_CHECKSUM_NAME}\"#" \
    "$dest"
}

render_gitvers_bootstrap "$SOURCE_BOOTSTRAP" "$rendered_bootstrap"

cp "$rendered_bootstrap" "$repo_dir/${GITVERS_BOOTSTRAP_NAME}"
cp "$SOURCE_ARCHIVE" "$repo_dir/${GITVERS_ARCHIVE_NAME}"
cp "$SOURCE_CHECKSUM" "$repo_dir/${GITVERS_CHECKSUM_NAME}"

(
  cd "$repo_dir"
  git config user.name "$GITVERS_GIT_NAME"
  git config user.email "$GITVERS_GIT_EMAIL"
  git add "$GITVERS_BOOTSTRAP_NAME" "$GITVERS_ARCHIVE_NAME" "$GITVERS_CHECKSUM_NAME"
  if git diff --cached --quiet; then
    echo "gitvers_sync=ok sync_changed=false version=${version} checksum_ok=true"
    exit 0
  fi
  git commit -m "CHIMERA release ${version}" >/dev/null
  git push origin "$GITVERS_BRANCH" >/dev/null 2>&1 || fail "push_failed"
)

if [[ "${CHIMERA_GITVERS_VERIFY_DISABLE:-0}" != "1" ]]; then
  verify_gitvers_remote_release "$version" "$expected_sha"
fi

echo "gitvers_sync=ok sync_changed=true version=${version} checksum_ok=true"
