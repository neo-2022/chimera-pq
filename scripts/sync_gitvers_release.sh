#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="${ROOT_DIR}/target"
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

require_file "$SOURCE_BOOTSTRAP"
require_file "$SOURCE_ARCHIVE"
require_file "$SOURCE_CHECKSUM"
[[ -n "$GITVERS_TOKEN" ]] || fail "missing_token"

version="$(extract_bootstrap_version "$SOURCE_BOOTSTRAP")"
[[ "$version" =~ ^[0-9]+[.][0-9]+[.][0-9]+$ ]] || fail "bad_bootstrap_version"

expected_sha="$(extract_checksum_value "$SOURCE_CHECKSUM")"
[[ -n "$expected_sha" ]] || fail "empty_checksum"
actual_sha="$(sha256_file "$SOURCE_ARCHIVE")"
[[ "$expected_sha" == "$actual_sha" ]] || fail "checksum_mismatch"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT
repo_dir="$tmp_dir/repo"
rendered_bootstrap="$tmp_dir/${GITVERS_BOOTSTRAP_NAME}"
authenticated_url="https://${GITVERS_TOKEN}@gitverse.ru/${GITVERS_OWNER}/${GITVERS_REPO}"

if ! git clone --depth 1 --branch "$GITVERS_BRANCH" "$authenticated_url" "$repo_dir" >/dev/null 2>&1; then
  fail "clone_failed"
fi

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

echo "gitvers_sync=ok sync_changed=true version=${version} checksum_ok=true"
