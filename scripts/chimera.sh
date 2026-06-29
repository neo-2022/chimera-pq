#!/usr/bin/env bash
set -euo pipefail

VERSION="0.0.0-dev"
ARCHIVE_URL_DEFAULT="https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera-pq-release.tar.gz"
CHECKSUM_URL_DEFAULT="https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera-pq-release.tar.gz.sha256"

resolve_self() {
  local src="${BASH_SOURCE[0]}"
  while [[ -n "${src:-}" && -L "$src" ]]; do
    local dir
    dir="$(cd "$(dirname "$src")" && pwd)"
    src="$(readlink "$src")"
    [[ "$src" != /* ]] && src="$dir/$src"
  done
  if [[ -n "${src:-}" && -f "$src" ]]; then
    cd "$(dirname "$src")" && pwd
    return 0
  fi
  pwd
}

download_url_to_file() {
  local url="${1:?url_required}"
  local dest="${2:?dest_required}"
  if command -v curl >/dev/null 2>&1; then
    if env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
      curl --disable -fsSL --retry 3 --connect-timeout 10 --max-time 60 "$url" -o "$dest"
    then
      return 0
    fi
  fi
  if command -v wget >/dev/null 2>&1; then
    if env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
      wget --no-config --tries=3 --timeout=10 --dns-timeout=10 --connect-timeout=10 --read-timeout=60 --waitretry=1 -qO "$dest" "$url"
    then
      return 0
    fi
  fi
  echo "error: missing downloader: curl or wget" >&2
  return 1
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
  echo "error: missing sha256 tool: sha256sum or shasum" >&2
  return 1
}

verify_archive_checksum() {
  local archive="${1:?archive_required}"
  local checksum_file="${2:?checksum_required}"
  local expected actual
  expected="$(awk '{print $1}' "$checksum_file" | tr -d '[:space:]')"
  [[ -n "$expected" ]] || {
    echo "error: empty checksum file" >&2
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

link_launchers() {
  local chimera_home="${1:?chimera_home_required}"
  local local_bin="${2:?local_bin_required}"
  mkdir -p "$local_bin" || return 1
  ln -sfn "$chimera_home/scripts/chimera.sh" "$local_bin/chimera" || return 1
  ln -sfn "$chimera_home/scripts/chimera.sh" "$local_bin/chimera.sh" || return 1
  ln -sfn "$chimera_home/scripts/chimera-sh" "$local_bin/chimera-sh" || return 1
}

restore_previous_release() {
  local chimera_home="${1:?chimera_home_required}"
  local backup_home="${2:-}"
  local had_previous="${3:-0}"
  rm -rf "$chimera_home"
  if [[ "$had_previous" == "1" && -n "$backup_home" && -d "$backup_home" ]]; then
    mv "$backup_home" "$chimera_home"
  fi
}

install_release_archive() {
  local archive="${1:?archive_required}"
  local checksum_file="${2:?checksum_required}"
  local chimera_home="${CHIMERA_HOME:-$HOME/.local/share/chimera}"
  local local_bin="${CHIMERA_LOCAL_BIN:-$HOME/.local/bin}"
  local extract_parent
  local extract_tmp
  local cleanup_extract_tmp
  local prepared_release
  local backup_home=""
  local had_previous=0
  extract_parent="$(dirname "$chimera_home")"

  mkdir -p "$extract_parent"
  extract_tmp="$(mktemp -d)"
  cleanup_extract_tmp="$(printf '%q' "$extract_tmp")"
  trap "rm -rf -- ${cleanup_extract_tmp}" RETURN
  tar -xzf "$archive" -C "$extract_tmp"
  prepared_release="$extract_tmp/chimera-release"
  if [[ ! -d "$prepared_release" ]]; then
    echo "error: release archive did not contain chimera-release/" >&2
    return 1
  fi
  mkdir -p "$prepared_release/releases"
  cp -f "$archive" "$prepared_release/releases/chimera-pq-release.tar.gz"
  cp -f "$checksum_file" "$prepared_release/releases/chimera-pq-release.tar.gz.sha256"

  chmod +x "$prepared_release/bin/"* 2>/dev/null || true
  chmod +x "$prepared_release/scripts/"*.sh 2>/dev/null || true
  chmod +x "$prepared_release/scripts/chimera-sh" 2>/dev/null || true

  if [[ -e "$chimera_home" ]]; then
    backup_home="$(mktemp -d "${extract_parent}/.chimera-previous.XXXXXX")"
    rmdir "$backup_home"
    mv "$chimera_home" "$backup_home"
    had_previous=1
  fi
  if ! mv "$prepared_release" "$chimera_home"; then
    restore_previous_release "$chimera_home" "$backup_home" "$had_previous"
    return 1
  fi

  CHIMERA_RELEASE_VERSION="${VERSION}"
  export CHIMERA_RELEASE_VERSION
  local install_rc=0
  bash "$chimera_home/scripts/install_desktop_control.sh" || install_rc=$?
  if [[ "$install_rc" -ne 0 ]]; then
    restore_previous_release "$chimera_home" "$backup_home" "$had_previous"
    if [[ "$had_previous" == "1" ]]; then
      link_launchers "$chimera_home" "$local_bin"
    fi
    return "$install_rc"
  fi

  if ! link_launchers "$chimera_home" "$local_bin"; then
    restore_previous_release "$chimera_home" "$backup_home" "$had_previous"
    if [[ "$had_previous" == "1" ]]; then
      link_launchers "$chimera_home" "$local_bin" || true
    fi
    return 1
  fi
  if [[ -n "$backup_home" ]]; then
    rm -rf "$backup_home"
  fi

  echo "chimera_install=ok version=$VERSION home=$chimera_home"
}

bootstrap_install_from_github() {
  local archive_url="${CHIMERA_RELEASE_ARCHIVE_URL:-$ARCHIVE_URL_DEFAULT}"
  local checksum_url="${CHIMERA_RELEASE_CHECKSUM_URL:-$CHECKSUM_URL_DEFAULT}"
  local tmp_dir archive checksum
  local cleanup_tmp_dir

  command -v tar >/dev/null 2>&1 || {
    echo "error: missing required command: tar" >&2
    return 1
  }

  tmp_dir="$(mktemp -d)"
  cleanup_tmp_dir="$(printf '%q' "$tmp_dir")"
  archive="$tmp_dir/chimera-pq-release.tar.gz"
  checksum="$tmp_dir/chimera-pq-release.tar.gz.sha256"
  trap "rm -rf -- ${cleanup_tmp_dir}" RETURN

  echo "chimera_bootstrap=download archive=$archive_url"
  download_url_to_file "$archive_url" "$archive"
  download_url_to_file "$checksum_url" "$checksum"
  verify_archive_checksum "$archive" "$checksum"
  install_release_archive "$archive" "$checksum"
}

SCRIPT_DIR="$(resolve_self)"
LOCAL_SH="$SCRIPT_DIR/chimera-sh"

case "${1:-}" in
  -install|install)
    bootstrap_install_from_github
    ;;
  "")
    if [[ -x "$LOCAL_SH" ]]; then
      exec "$LOCAL_SH" -help
    fi
    echo "usage: bash -o pipefail -c 'curl --disable -fsSL --retry 3 --connect-timeout 10 --max-time 60 https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh | bash -s -- -install'" >&2
    exit 2
    ;;
  *)
    if [[ -x "$LOCAL_SH" ]]; then
      exec "$LOCAL_SH" "$@"
    fi
    echo "error: CHIMERA is not installed. Run GitHub one-command install first." >&2
    echo "usage: bash -o pipefail -c 'curl --disable -fsSL --retry 3 --connect-timeout 10 --max-time 60 https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh | bash -s -- -install'" >&2
    exit 2
    ;;
esac
