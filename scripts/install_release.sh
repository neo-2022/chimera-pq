#!/usr/bin/env bash
set -euo pipefail

SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHIMERA_HOME="${CHIMERA_HOME:-$HOME/.local/share/chimera}"
LOCAL_BIN="${CHIMERA_LOCAL_BIN:-$HOME/.local/bin}"
DEFAULT_RELEASE_URL="https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera-pq-release.tar.gz"
BUNDLE_SOURCE="${1:-${CHIMERA_RELEASE_ARCHIVE_URL:-$DEFAULT_RELEASE_URL}}"
CHECKSUM_SOURCE="${2:-${CHIMERA_RELEASE_CHECKSUM_FILE:-}}"
ALLOW_LOCAL_SOURCE="${CHIMERA_ALLOW_LOCAL_RELEASE_SOURCE:-0}"

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
  echo "error: missing sha256 tool: sha256sum or shasum" >&2
  return 1
}

download_url_to_file() {
  local url="${1:?url_required}"
  local dest="${2:?dest_required}"
  if command -v curl >/dev/null 2>&1; then
    if env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
      curl -fsSL --retry 3 --connect-timeout 10 --max-time 60 -o "$dest" "$url"
    then
      return 0
    fi
  fi
  if command -v wget >/dev/null 2>&1; then
    if env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
      wget -qO "$dest" "$url"
    then
      return 0
    fi
  fi
  echo "error: need curl or wget to download release" >&2
  return 1
}

resolve_checksum_source() {
  local archive="${1:?archive_required}"
  local checksum="${2:-}"
  if [[ -n "$checksum" && -f "$checksum" ]]; then
    printf '%s\n' "$checksum"
    return 0
  fi
  if [[ -f "${archive}.sha256" ]]; then
    printf '%s\n' "${archive}.sha256"
    return 0
  fi
  echo "error: release checksum is required before archive extraction: $archive" >&2
  return 1
}

verify_checksum_required() {
  local archive="${1:?archive_required}"
  local checksum="${2:?checksum_required}"
  local expected actual
  expected="$(awk '{print $1}' "$checksum" | tr -d '[:space:]')"
  [[ -n "$expected" ]] || {
    echo "error: empty checksum file: $checksum" >&2
    return 1
  }
  actual="$(sha256_file "$archive")"
  if [[ "$actual" != "$expected" ]]; then
    echo "error: release checksum mismatch expected=$expected actual=$actual" >&2
    return 1
  fi
  CHIMERA_RELEASE_BUNDLE_SHA256="$actual"
  export CHIMERA_RELEASE_BUNDLE_SHA256
}

install_from_archive() {
  local archive="${1:?archive_required}"
  local checksum
  local extract_tmp
  checksum="$(resolve_checksum_source "$archive" "${2:-}")"
  verify_checksum_required "$archive" "$checksum"
  extract_tmp="$(mktemp -d)"
  trap 'rm -rf "$extract_tmp"' RETURN
  tar -xzf "$archive" -C "$extract_tmp"
  if [[ ! -d "$extract_tmp/chimera-release" ]]; then
    echo "error: release archive did not contain chimera-release/" >&2
    return 1
  fi
  rm -rf "$CHIMERA_HOME"
  mkdir -p "$(dirname "$CHIMERA_HOME")"
  mv "$extract_tmp/chimera-release" "$CHIMERA_HOME"
}

require_local_source_opt_in() {
  if [[ "$ALLOW_LOCAL_SOURCE" != "1" ]]; then
    echo "error: local release sources are development-only." >&2
    echo "hint: use GitHub Latest one-command install, or set CHIMERA_ALLOW_LOCAL_RELEASE_SOURCE=1 for local packaging debug." >&2
    exit 1
  fi
}

echo "CHIMERA self-contained install"
echo "  source: ${BUNDLE_SOURCE}"

if [[ -d "${BUNDLE_SOURCE}" ]]; then
  require_local_source_opt_in
  echo "install: copying release directory"
  rm -rf "${CHIMERA_HOME}"
  mkdir -p "$(dirname "${CHIMERA_HOME}")"
  cp -a "${BUNDLE_SOURCE}" "${CHIMERA_HOME}"
elif [[ -f "${BUNDLE_SOURCE}" && "${BUNDLE_SOURCE}" == *.tar.gz ]]; then
  require_local_source_opt_in
  echo "install: extracting tarball"
  install_from_archive "$BUNDLE_SOURCE" "$CHECKSUM_SOURCE"
elif [[ "${BUNDLE_SOURCE}" == https://* || "${BUNDLE_SOURCE}" == http://* ]]; then
  echo "install: downloading from ${BUNDLE_SOURCE}"
  TMP_DIR="$(mktemp -d)"
  trap 'rm -rf "$TMP_DIR"' EXIT
  TMP_ARCHIVE="${TMP_DIR}/chimera-release.tar.gz"
  TMP_CHECKSUM="${TMP_DIR}/chimera-release.tar.gz.sha256"
  download_url_to_file "${BUNDLE_SOURCE}" "${TMP_ARCHIVE}"
  if [[ -n "${CHIMERA_RELEASE_CHECKSUM_URL:-}" ]]; then
    download_url_to_file "${CHIMERA_RELEASE_CHECKSUM_URL}" "${TMP_CHECKSUM}"
    CHECKSUM_SOURCE="$TMP_CHECKSUM"
  elif download_url_to_file "${BUNDLE_SOURCE}.sha256" "${TMP_CHECKSUM}" 2>/dev/null; then
    CHECKSUM_SOURCE="$TMP_CHECKSUM"
  else
    echo "error: remote release checksum is required for URL install" >&2
    exit 1
  fi
  install_from_archive "$TMP_ARCHIVE" "$CHECKSUM_SOURCE"
else
  echo "error: cannot find release at ${BUNDLE_SOURCE}" >&2
  echo "usage: ${0} [<path-to-tarball> | <path-to-release-dir> | <url>]" >&2
  exit 1
fi

chmod +x "${CHIMERA_HOME}/bin/"*
chmod +x "${CHIMERA_HOME}/scripts/"*.sh

echo "install: running desktop control setup"
CHIMERA_RELEASE_VERSION="$(cat "${CHIMERA_HOME}/.chimera_release_version" 2>/dev/null || true)"
export CHIMERA_RELEASE_VERSION
bash "${CHIMERA_HOME}/scripts/install_desktop_control.sh"

mkdir -p "${LOCAL_BIN}"
ln -sfn "${CHIMERA_HOME}/scripts/chimera.sh" "${LOCAL_BIN}/chimera"
ln -sfn "${CHIMERA_HOME}/scripts/chimera.sh" "${LOCAL_BIN}/chimera.sh"
ln -sfn "${CHIMERA_HOME}/scripts/chimera-sh" "${LOCAL_BIN}/chimera-sh"

echo
echo "CHIMERA self-contained install complete."
echo "  version: ${CHIMERA_RELEASE_VERSION:-unknown}"
echo "  home:    ${CHIMERA_HOME}"
echo "  bin:     ${LOCAL_BIN}/chimera"
echo
echo "Quick start:"
echo "  chimera -start"
echo "  chimera -status"
echo "  chimera -stop"
