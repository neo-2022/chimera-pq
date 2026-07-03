#!/usr/bin/env bash
set -euo pipefail

VERSION="0.0.0-dev"
ARCHIVE_URL_DEFAULT="https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera-pq-release.tar.gz"
CHECKSUM_URL_DEFAULT="https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera-pq-release.tar.gz.sha256"
GITVERS_BOOTSTRAP_URLS_DEFAULT="${CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URLS_DEFAULT:-https://gitverse.ru/api/repos/ArtReg/chimera/raw/branch/main/chimera.sh}"
GITVERS_BOOTSTRAP_URLS_FILE="${CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URLS_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/update_gitvers_bootstrap_urls.list}"

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

trim_ascii() {
  local value="${1:-}"
  value="${value#"${value%%[![:space:]]*}"}"
  value="${value%"${value##*[![:space:]]}"}"
  printf '%s' "$value"
}

split_bootstrap_candidates() {
  local raw="${1:-}"
  raw="${raw//$'\r'/ }"
  raw="${raw//$'\n'/ }"
  raw="${raw//,/ }"
  local candidate
  for candidate in $raw; do
    candidate="$(trim_ascii "$candidate")"
    [[ -n "$candidate" ]] || continue
    printf '%s\n' "$candidate"
  done
}

validate_bootstrap_url() {
  local url="${1:-}"
  if [[ -z "$url" || ( "$url" != http://* && "$url" != https://* ) ]]; then
    return 1
  fi
  case "$url" in
    *\"*|*"'"*|*\`*|*\$*|*\\*|*\?*|*"#"*|*@*|*$'\r'*|*$'\n'*|*$'\t'*)
      return 1
      ;;
  esac
  [[ "$url" =~ [[:space:]] ]] && return 1
  return 0
}

normalize_bootstrap_url() {
  local candidate="${1:-}"
  candidate="$(trim_ascii "$candidate")"
  candidate="${candidate%/}"
  candidate="$(normalize_gitverse_bootstrap_candidate "$candidate")"
  case "$candidate" in
    */chimera.sh)
      ;;
    */metadata.json)
      candidate="${candidate%/metadata.json}/chimera.sh"
      ;;
    */chimera-pq-release.tar.gz)
      candidate="${candidate%/chimera-pq-release.tar.gz}/chimera.sh"
      ;;
    */chimera-pq-release.tar.gz.sha256)
      candidate="${candidate%/chimera-pq-release.tar.gz.sha256}/chimera.sh"
      ;;
    *)
      candidate="${candidate}/chimera.sh"
      ;;
  esac
  validate_bootstrap_url "$candidate" || return 1
  printf '%s\n' "$candidate"
}

normalize_gitverse_bootstrap_candidate() {
  local candidate="${1:-}" scheme path path_no_query trimmed owner repo ref remainder
  local -a parts=()
  case "$candidate" in
    http://gitverse.ru/*|https://gitverse.ru/*)
      ;;
    *)
      printf '%s\n' "$candidate"
      return 0
      ;;
  esac

  scheme="${candidate%%://*}"
  path="${candidate#${scheme}://gitverse.ru}"
  path_no_query="${path%%\?*}"
  path_no_query="${path_no_query%/}"

  case "$path_no_query" in
    /api/repos/*/raw/branch/*)
      printf '%s://gitverse.ru%s\n' "$scheme" "$path_no_query"
      return 0
      ;;
  esac

  trimmed="${path_no_query#/}"
  IFS='/' read -r -a parts <<<"$trimmed"
  if [[ "${#parts[@]}" -eq 2 ]]; then
    owner="${parts[0]}"
    repo="${parts[1]}"
    printf '%s://gitverse.ru/api/repos/%s/%s/raw/branch/main\n' "$scheme" "$owner" "$repo"
    return 0
  fi
  if [[ "${#parts[@]}" -ge 4 && "${parts[2]}" == "content" ]]; then
    owner="${parts[0]}"
    repo="${parts[1]}"
    ref="${parts[3]}"
    remainder="${path_no_query#/${owner}/${repo}/content/${ref}}"
    if [[ -z "$remainder" ]]; then
      printf '%s://gitverse.ru/api/repos/%s/%s/raw/branch/%s\n' "$scheme" "$owner" "$repo" "$ref"
    else
      printf '%s://gitverse.ru/api/repos/%s/%s/raw/branch/%s%s\n' "$scheme" "$owner" "$repo" "$ref" "$remainder"
    fi
    return 0
  fi

  printf '%s\n' "$candidate"
}

load_gitvers_bootstrap_urls() {
  {
    if [[ -n "${CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URL:-}" ]]; then
      split_bootstrap_candidates "$CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URL"
    fi
    if [[ -n "${CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URLS:-}" ]]; then
      split_bootstrap_candidates "$CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URLS"
    fi
    if [[ -f "$GITVERS_BOOTSTRAP_URLS_FILE" ]]; then
      while IFS= read -r line; do
        line="${line%%#*}"
        split_bootstrap_candidates "$line"
      done < "$GITVERS_BOOTSTRAP_URLS_FILE"
    fi
    split_bootstrap_candidates "$GITVERS_BOOTSTRAP_URLS_DEFAULT"
  } | awk 'NF && !seen[$0]++'
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

bootstrap_metadata_from_script() {
  local script_file="${1:?script_file_required}"
  local release_version archive_url checksum_url
  release_version="$(grep -m1 '^VERSION="' "$script_file" | cut -d'"' -f2 | tr -d '[:space:]')"
  archive_url="$(grep -m1 '^ARCHIVE_URL_DEFAULT="' "$script_file" | cut -d'"' -f2 | tr -d '[:space:]')"
  checksum_url="$(grep -m1 '^CHECKSUM_URL_DEFAULT="' "$script_file" | cut -d'"' -f2 | tr -d '[:space:]')"
  [[ -n "$release_version" && -n "$archive_url" && -n "$checksum_url" ]] || return 1
  validate_bootstrap_url "$archive_url" || return 1
  validate_bootstrap_url "$checksum_url" || return 1
  printf '%s\n%s\n%s\n' "$release_version" "$archive_url" "$checksum_url"
}

bootstrap_install_from_archive_urls() {
  local source_name="${1:?source_name_required}"
  local archive_url="${2:?archive_url_required}"
  local checksum_url="${3:?checksum_url_required}"
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

  echo "chimera_bootstrap=download source=$source_name archive=$archive_url"
  if ! download_url_to_file "$archive_url" "$archive"; then
    echo "error: release archive download unavailable from $source_name" >&2
    return 2
  fi
  if ! download_url_to_file "$checksum_url" "$checksum"; then
    echo "error: release checksum download unavailable from $source_name" >&2
    return 2
  fi
  verify_archive_checksum "$archive" "$checksum" || return 3
  install_release_archive "$archive" "$checksum" || return 3
}

bootstrap_install_from_bootstrap_source() {
  local source_name="${1:?source_name_required}"
  local bootstrap_url="${2:?bootstrap_url_required}"
  local normalized_bootstrap_url tmp_dir bootstrap_file cleanup_tmp_dir metadata=()
  local remote_version archive_url checksum_url

  normalized_bootstrap_url="$(normalize_bootstrap_url "$bootstrap_url")" || {
    echo "error: invalid bootstrap source url for $source_name" >&2
    return 3
  }

  tmp_dir="$(mktemp -d)"
  cleanup_tmp_dir="$(printf '%q' "$tmp_dir")"
  bootstrap_file="$tmp_dir/chimera.sh"
  trap "rm -rf -- ${cleanup_tmp_dir}" RETURN

  echo "chimera_bootstrap=metadata source=$source_name bootstrap=$normalized_bootstrap_url"
  if ! download_url_to_file "$normalized_bootstrap_url" "$bootstrap_file"; then
    echo "error: bootstrap metadata download unavailable from $source_name" >&2
    return 2
  fi
  if ! mapfile -t metadata < <(bootstrap_metadata_from_script "$bootstrap_file"); then
    echo "error: invalid bootstrap metadata from $source_name" >&2
    return 3
  fi
  remote_version="${metadata[0]:-}"
  archive_url="${metadata[1]:-}"
  checksum_url="${metadata[2]:-}"
  [[ -n "$remote_version" && -n "$archive_url" && -n "$checksum_url" ]] || {
    echo "error: incomplete bootstrap metadata from $source_name" >&2
    return 3
  }
  bootstrap_install_from_archive_urls "$source_name" "$archive_url" "$checksum_url"
}

bootstrap_install_from_configured_sources() {
  local archive_url="${CHIMERA_RELEASE_ARCHIVE_URL:-$ARCHIVE_URL_DEFAULT}"
  local checksum_url="${CHIMERA_RELEASE_CHECKSUM_URL:-$CHECKSUM_URL_DEFAULT}"
  local explicit_archive_override="${CHIMERA_RELEASE_ARCHIVE_URL:-}"
  local explicit_checksum_override="${CHIMERA_RELEASE_CHECKSUM_URL:-}"
  local gitvers_bootstrap_url normalized_gitvers_url rc

  if [[ -n "$explicit_archive_override" || -n "$explicit_checksum_override" ]]; then
    bootstrap_install_from_archive_urls "explicit" "$archive_url" "$checksum_url"
    return $?
  fi

  set +e
  bootstrap_install_from_archive_urls "github" "$archive_url" "$checksum_url"
  rc=$?
  set -e
  case "$rc" in
    0)
      return 0
      ;;
    2)
      ;;
    *)
      return "$rc"
      ;;
  esac

  while IFS= read -r gitvers_bootstrap_url; do
    normalized_gitvers_url="$(normalize_bootstrap_url "$gitvers_bootstrap_url" || true)"
    [[ -n "$normalized_gitvers_url" ]] || continue
    set +e
    bootstrap_install_from_bootstrap_source "gitvers" "$normalized_gitvers_url"
    rc=$?
    set -e
    case "$rc" in
      0)
        return 0
        ;;
      2)
        ;;
      *)
        return "$rc"
        ;;
    esac
  done < <(load_gitvers_bootstrap_urls)

  echo "error: release sources unavailable (github and gitvers)" >&2
  return 2
}

SCRIPT_DIR="$(resolve_self)"
LOCAL_SH="$SCRIPT_DIR/chimera-sh"

case "${1:-}" in
  -install|install)
    bootstrap_install_from_configured_sources
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
    echo "error: CHIMERA is not installed. Run the GitHub one-command install or a configured Gitvers bootstrap mirror first." >&2
    echo "usage: bash -o pipefail -c 'curl --disable -fsSL --retry 3 --connect-timeout 10 --max-time 60 https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh | bash -s -- -install'" >&2
    exit 2
    ;;
esac
