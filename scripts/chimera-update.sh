#!/usr/bin/env bash

UPDATE_BOOTSTRAP_URL="${CHIMERA_UPDATE_BOOTSTRAP_URL:-https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh}"
UPDATE_GITVERS_BOOTSTRAP_URLS_DEFAULT="${CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URLS_DEFAULT:-https://gitverse.ru/api/repos/ArtReg/chimera/raw/branch/main/chimera.sh}"
UPDATE_GITVERS_BOOTSTRAP_URLS_FILE="${CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URLS_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/update_gitvers_bootstrap_urls.list}"
UPDATE_PEER_BOOTSTRAP_URLS_FILE="${CHIMERA_UPDATE_PEER_BOOTSTRAP_URLS_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/update_peer_bootstrap_urls.list}"
UPDATE_DOWNLOAD_CONNECT_TIMEOUT_SEC="${CHIMERA_UPDATE_DOWNLOAD_CONNECT_TIMEOUT_SEC:-10}"
UPDATE_DOWNLOAD_MAX_TIME_SEC="${CHIMERA_UPDATE_DOWNLOAD_MAX_TIME_SEC:-60}"
UPDATE_DOWNLOAD_RETRIES="${CHIMERA_UPDATE_DOWNLOAD_RETRIES:-2}"
source "$ROOT_DIR/scripts/chimera-update-runtime-state.sh"
source "$ROOT_DIR/scripts/chimera-update-rerun.sh"

CHIMERA_UPDATE_SOURCE_NOT_NEWER_RC=4

UPDATE_AUTHORITY_SOURCE=""
UPDATE_AUTHORITY_VERSION=""
UPDATE_AUTHORITY_SHA=""
UPDATE_AUTHORITY_SHA_KNOWN=0

reset_update_source_authority() {
  UPDATE_AUTHORITY_SOURCE=""
  UPDATE_AUTHORITY_VERSION=""
  UPDATE_AUTHORITY_SHA=""
  UPDATE_AUTHORITY_SHA_KNOWN=0
}

update_source_trust_rank() {
  case "${1:-}" in
    github) printf '%s\n' 1 ;;
    gitvers) printf '%s\n' 2 ;;
    peer) printf '%s\n' 3 ;;
    *) printf '%s\n' 99 ;;
  esac
}

register_update_source_version() {
  local source_name="${1:?source_name_required}"
  local remote_version="${2:?remote_version_required}"
  local source_rank authority_rank
  source_rank="$(update_source_trust_rank "$source_name")"
  if [[ -z "$UPDATE_AUTHORITY_SOURCE" ]]; then
    UPDATE_AUTHORITY_SOURCE="$source_name"
    UPDATE_AUTHORITY_VERSION="$remote_version"
    UPDATE_AUTHORITY_SHA=""
    UPDATE_AUTHORITY_SHA_KNOWN=0
    return 0
  fi
  authority_rank="$(update_source_trust_rank "$UPDATE_AUTHORITY_SOURCE")"
  if [[ "$source_rank" -lt "$authority_rank" ]]; then
    UPDATE_AUTHORITY_SOURCE="$source_name"
    UPDATE_AUTHORITY_VERSION="$remote_version"
    UPDATE_AUTHORITY_SHA=""
    UPDATE_AUTHORITY_SHA_KNOWN=0
    return 0
  fi
  if [[ "$remote_version" != "$UPDATE_AUTHORITY_VERSION" ]]; then
    echo "chimera_update=source_divergent source=$source_name authority_source=$UPDATE_AUTHORITY_SOURCE authority_version=$UPDATE_AUTHORITY_VERSION latest_version=$remote_version action=block reason=trusted_version_divergence" >&2
    return 3
  fi
  return 0
}

register_update_source_checksum() {
  local source_name="${1:?source_name_required}"
  local remote_sha="${2:?remote_sha_required}"
  [[ -n "$UPDATE_AUTHORITY_SOURCE" ]] || return 0
  if [[ "$UPDATE_AUTHORITY_SHA_KNOWN" -eq 0 ]]; then
    UPDATE_AUTHORITY_SHA="$remote_sha"
    UPDATE_AUTHORITY_SHA_KNOWN=1
    return 0
  fi
  if [[ "$remote_sha" != "$UPDATE_AUTHORITY_SHA" ]]; then
    echo "chimera_update=source_divergent source=$source_name authority_source=$UPDATE_AUTHORITY_SOURCE authority_version=$UPDATE_AUTHORITY_VERSION authority_sha=$UPDATE_AUTHORITY_SHA latest_sha=$remote_sha action=block reason=trusted_checksum_divergence" >&2
    return 3
  fi
  return 0
}

release_version_to_sortable() {
  local v="${1:-0.0.0}"
  v="${v#v}"
  [[ "$v" =~ ^[0-9]+[.][0-9]+[.][0-9]+$ ]] || return 1
  awk -F. '
    {
      major=$1+0; minor=$2+0; patch=$3+0;
      printf "%06d%06d%06d\n", major, minor, patch;
    }' <<<"$v"
}

is_remote_newer() {
  local local_v="${1:-0.0.0}"
  local remote_v="${2:-0.0.0}"
  local local_key remote_key
  remote_key="$(release_version_to_sortable "$remote_v")" || return 1
  local_key="$(release_version_to_sortable "$local_v")" || local_key="000000000000000000"
  [[ "$remote_key" > "$local_key" ]]
}

read_local_runtime_version() {
  local version_file version
  version_file="$(runtime_version_file)"
  if [[ -f "$version_file" ]]; then
    version="$(tr -d '[:space:]' < "$version_file")"
    if [[ -n "$version" ]]; then
      printf '%s\n' "$version"
      return 0
    fi
  fi
  if version="$(read_local_runtime_version_from_release_bundle 2>/dev/null || true)" && [[ -n "$version" ]]; then
    printf '%s\n' "$version"
    return 0
  fi
  echo "0.0.0"
}

runtime_version_needs_repair() {
  local version="${1:-}"
  [[ -n "$version" && "$version" != "0.0.0" ]] || return 0
  release_version_to_sortable "$version" >/dev/null 2>&1 || return 0
  return 1
}

emit_no_newer_release_status() {
  local local_version="${1:-unknown}"
  local repair_required="${2:-0}"
  if [[ "$repair_required" -ne 0 ]]; then
    echo "chimera_update=no_newer_release current_version=unknown action=continue reason=highest_available_not_newer" >&2
  else
    echo "chimera_update=no_newer_release current_version=$local_version action=continue reason=highest_available_not_newer" >&2
  fi
}

read_local_install_role() {
  local install_role_file
  install_role_file="$(install_node_role_file)"
  if [[ -f "$install_role_file" ]]; then
    normalize_install_role "$(tr -d '[:space:]' < "$install_role_file")"
    return 0
  fi
  local env_file="${XDG_CONFIG_HOME:-$HOME/.config}/chimera/peer-egress.env"
  if [[ -f "$env_file" ]]; then
    local env_mode=""
    env_mode="$(awk -F= '/^CHIMERA_PEER_EGRESS_MODE=/{print $2; exit}' "$env_file" 2>/dev/null | tr -d '[:space:]')"
    case "$env_mode" in
      node|weave-node) echo "node"; return 0 ;;
      client|server|gateway) echo "node"; return 0 ;;
    esac
  fi
  normalize_install_role "${CHIMERA_INSTALL_NODE_ROLE:-node}"
}

normalize_install_role() {
  case "${1:-node}" in
    node|weave-node|client|server|gateway) printf '%s\n' "node" ;;
    *) printf '%s\n' "node" ;;
  esac
}

trim_ascii() {
  local value="${1:-}"
  value="${value#"${value%%[![:space:]]*}"}"
  value="${value%"${value##*[![:space:]]}"}"
  printf '%s' "$value"
}

split_update_candidates() {
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

validate_update_bootstrap_url() {
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

normalize_update_bootstrap_url() {
  local candidate="${1:-}"
  candidate="$(trim_ascii "$candidate")"
  candidate="${candidate%/}"
  candidate="$(normalize_gitverse_update_bootstrap_candidate "$candidate")"
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
  validate_update_bootstrap_url "$candidate" || return 1
  printf '%s\n' "$candidate"
}

normalize_gitverse_update_bootstrap_candidate() {
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

cache_buster_url() {
  local url="${1:?url_required}"
  local stamp="ts=$(date +%s)"
  if [[ "$url" == *"?"* ]]; then
    printf '%s&%s\n' "$url" "$stamp"
  else
    printf '%s?%s\n' "$url" "$stamp"
  fi
}

positive_integer_or_default() {
  local value="${1:-}"
  local default_value="${2:?default_required}"
  if [[ "$value" =~ ^[1-9][0-9]*$ ]]; then
    printf '%s\n' "$value"
  else
    printf '%s\n' "$default_value"
  fi
}

non_negative_integer_or_default() {
  local value="${1:-}"
  local default_value="${2:?default_required}"
  if [[ "$value" =~ ^[0-9]+$ ]]; then
    printf '%s\n' "$value"
  else
    printf '%s\n' "$default_value"
  fi
}

update_connect_timeout_sec() {
  positive_integer_or_default "$UPDATE_DOWNLOAD_CONNECT_TIMEOUT_SEC" 3
}

update_max_time_sec() {
  positive_integer_or_default "$UPDATE_DOWNLOAD_MAX_TIME_SEC" 8
}

update_retries() {
  non_negative_integer_or_default "$UPDATE_DOWNLOAD_RETRIES" 0
}

run_update_download_command() {
  local max_time_sec
  max_time_sec="$(update_max_time_sec)"
  if command -v timeout >/dev/null 2>&1; then
    timeout "${max_time_sec}s" "$@"
  else
    "$@"
  fi
}

download_url_to_file() {
  local url="${1:?url_required}"
  local dest="${2:?dest_required}"
  local bootstrap_bin="${CHIMERA_BOOTSTRAP_BIN:-${ROOT_DIR}/bin/chimera-bootstrap}"
  local connect_timeout_sec max_time_sec retries
  connect_timeout_sec="$(update_connect_timeout_sec)"
  max_time_sec="$(update_max_time_sec)"
  retries="$(update_retries)"
  if [[ -x "$bootstrap_bin" ]]; then
    if run_update_download_command \
      env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
        CHIMERA_BOOTSTRAP_CONNECT_TIMEOUT_SEC="$connect_timeout_sec" \
        CHIMERA_BOOTSTRAP_DOWNLOAD_TIMEOUT_SEC="$max_time_sec" \
        "$bootstrap_bin" download --url "$url" --output "$dest"
    then
      return 0
    fi
  fi
  if command -v curl >/dev/null 2>&1; then
    if env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
      curl --disable -fsSL --retry "$retries" --connect-timeout "$connect_timeout_sec" --max-time "$max_time_sec" "$url" -o "$dest"
    then
      return 0
    fi
  fi
  if command -v wget >/dev/null 2>&1; then
    if env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
      wget --no-config -qO "$dest" --tries=1 --timeout="$connect_timeout_sec" --read-timeout="$max_time_sec" "$url"
    then
      return 0
    fi
  fi
  echo "error: missing downloader: Rust bootstrap helper, curl, or wget" >&2
  return 1
}

load_update_peer_bootstrap_urls() {
  if [[ -n "${CHIMERA_UPDATE_PEER_BOOTSTRAP_URLS:-}" ]]; then
    split_update_candidates "$CHIMERA_UPDATE_PEER_BOOTSTRAP_URLS"
  fi
  if [[ -f "$UPDATE_PEER_BOOTSTRAP_URLS_FILE" ]]; then
    while IFS= read -r line; do
      line="${line%%#*}"
      split_update_candidates "$line"
    done < "$UPDATE_PEER_BOOTSTRAP_URLS_FILE"
  fi
}

load_update_gitvers_bootstrap_urls() {
  {
    if [[ -n "${CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URL:-}" ]]; then
      split_update_candidates "$CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URL"
    fi
    if [[ -n "${CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URLS:-}" ]]; then
      split_update_candidates "$CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URLS"
    fi
    if [[ -f "$UPDATE_GITVERS_BOOTSTRAP_URLS_FILE" ]]; then
      while IFS= read -r line; do
        line="${line%%#*}"
        split_update_candidates "$line"
      done < "$UPDATE_GITVERS_BOOTSTRAP_URLS_FILE"
    fi
    split_update_candidates "$UPDATE_GITVERS_BOOTSTRAP_URLS_DEFAULT"
  } | awk 'NF && !seen[$0]++'
}

mesh_connect_args_from_launcher_args() {
  local cmd="${1:-}"
  shift || true
  case "$cmd" in
    -connect|connect)
      printf '%s\n' "$@"
      ;;
  esac
}

selected_connect_peer_update_bootstrap_url() {
  local -a mesh_args=("$@")
  [[ -x "$ROOT_DIR/bin/chimera-cli" || -x "$ROOT_DIR/scripts/chimera-runner.sh" ]] || return 1
  local output
  if [[ -x "$ROOT_DIR/scripts/chimera-runner.sh" ]]; then
    output="$("$ROOT_DIR/scripts/chimera-runner.sh" cli mesh nodes selected-update-bootstrap-url "${mesh_args[@]}" 2>/dev/null || true)"
  else
    output="$("$ROOT_DIR/bin/chimera-cli" mesh nodes selected-update-bootstrap-url "${mesh_args[@]}" 2>/dev/null || true)"
  fi
  output="$(trim_ascii "$(printf '%s\n' "$output" | head -n 1)")"
  [[ -n "$output" ]] || return 1
  validate_update_bootstrap_url "$output" || return 1
  printf '%s\n' "$output"
}

load_update_peer_bootstrap_urls_for_args() {
  local -a original_args=("$@")
  local -a connect_args=()
  local peer_update_url=""
  mapfile -t connect_args < <(mesh_connect_args_from_launcher_args "${original_args[@]}")
  if [[ "${#connect_args[@]}" -gt 0 ]]; then
    peer_update_url="$(selected_connect_peer_update_bootstrap_url "${connect_args[@]}" || true)"
    if [[ -n "$peer_update_url" ]]; then
      printf '%s\n' "$peer_update_url"
    fi
    return 0
  fi
  load_update_peer_bootstrap_urls
}

parse_release_metadata() {
  local script_file="${1:?script_file_required}"
  local release_version archive_url checksum_url
  release_version="$(grep -m1 '^VERSION="' "$script_file" | cut -d'"' -f2 | tr -d '[:space:]')"
  archive_url="$(grep -m1 '^ARCHIVE_URL_DEFAULT="' "$script_file" | cut -d'"' -f2 | tr -d '[:space:]')"
  checksum_url="$(grep -m1 '^CHECKSUM_URL_DEFAULT="' "$script_file" | cut -d'"' -f2 | tr -d '[:space:]')"
  [[ -n "$release_version" && -n "$archive_url" && -n "$checksum_url" ]] || return 1
  validate_update_bootstrap_url "$archive_url" || return 1
  validate_update_bootstrap_url "$checksum_url" || return 1
  printf '%s\n%s\n%s\n\n' "$release_version" "$archive_url" "$checksum_url"
}

peer_metadata_url_from_bootstrap_url() {
  local url="${1:?url_required}"
  url="${url%%\?*}"
  case "$url" in
    */chimera.sh)
      printf '%s\n' "${url%/chimera.sh}/metadata.json"
      ;;
    */metadata.json)
      printf '%s\n' "$url"
      ;;
    *)
      return 1
      ;;
  esac
}

parse_peer_release_metadata_json() {
  local metadata_file="${1:?metadata_file_required}"
  local metadata_url="${2:?metadata_url_required}"
  local bootstrap_bin="${CHIMERA_BOOTSTRAP_BIN:-${ROOT_DIR}/bin/chimera-bootstrap}"
  [[ -x "$bootstrap_bin" ]] || return 3
  "$bootstrap_bin" parse-peer-metadata --file "$metadata_file" --metadata-url "$metadata_url"
}

read_release_metadata_from_source() {
  local source_name="${1:?source_name_required}"
  local bootstrap_url="${2:?bootstrap_url_required}"
  local tmp_file metadata_url release_meta_output
  tmp_file="$(mktemp)"
  case "$source_name" in
    peer)
      metadata_url="$(peer_metadata_url_from_bootstrap_url "$bootstrap_url")" || {
        rm -f "$tmp_file"
        return 2
      }
      if ! download_url_to_file "$(cache_buster_url "$metadata_url")" "$tmp_file" 2>/dev/null; then
        rm -f "$tmp_file"
        return 2
      fi
      if ! release_meta_output="$(parse_peer_release_metadata_json "$tmp_file" "$metadata_url")"; then
        rm -f "$tmp_file"
        return 3
      fi
      ;;
    *)
      if ! download_url_to_file "$(cache_buster_url "$bootstrap_url")" "$tmp_file" 2>/dev/null; then
        rm -f "$tmp_file"
        return 2
      fi
      if ! release_meta_output="$(parse_release_metadata "$tmp_file")"; then
        rm -f "$tmp_file"
        return 3
      fi
      ;;
  esac
  rm -f "$tmp_file"
  [[ -n "$release_meta_output" ]] || return 3
  printf '%s\n' "$release_meta_output"
}

remote_archive_sha256() {
  local archive_url="${1:?archive_url_required}"
  local checksum_url="${2:-${archive_url}.sha256}" tmp_checksum expected
  tmp_checksum="$(mktemp)"
  if ! download_url_to_file "$(cache_buster_url "$checksum_url")" "$tmp_checksum" 2>/dev/null; then
    rm -f "$tmp_checksum"
    return 2
  fi
  expected="$(awk '{print $1}' "$tmp_checksum" | tr -d '[:space:]')"
  rm -f "$tmp_checksum"
  [[ -n "$expected" && ${#expected} -eq 64 && "$expected" =~ ^[[:xdigit:]]+$ ]] || return 3
  printf '%s\n' "$expected"
}

install_update_from_release_metadata() {
  local source_name="${1:?source_name_required}"
  local remote_version="${2:?remote_version_required}"
  local remote_archive_url="${3:?remote_archive_url_required}"
  local remote_checksum_url="${4:?remote_checksum_url_required}"
  local remote_metadata_sha="${5:-}"
  local local_version="${6:?local_version_required}"
  local local_sha="${7:-}"
  shift 7 || true
  local -a original_args=("$@")
  local remote_sha remote_sha_rc install_role installer installed_version installed_sha install_rc remote_newer=0

  if is_remote_newer "$local_version" "$remote_version"; then
    remote_newer=1
  fi

  if remote_sha="$(remote_archive_sha256 "$remote_archive_url" "$remote_checksum_url")"; then
    :
  else
    remote_sha_rc=$?
    case "$remote_sha_rc" in
      2)
        echo "chimera_update=unavailable source=$source_name latest_version=$remote_version action=continue reason=checksum_unreachable" >&2
        return 2
        ;;
      *)
        echo "chimera_update=source_invalid source=$source_name latest_version=$remote_version action=block reason=invalid_checksum" >&2
        return 3
        ;;
    esac
  fi
  if [[ -n "$remote_metadata_sha" && "$remote_metadata_sha" != "$remote_sha" ]]; then
    echo "chimera_update=source_invalid source=$source_name latest_version=$remote_version action=block reason=metadata_checksum_mismatch" >&2
    return 3
  fi
  if ! register_update_source_version "$source_name" "$remote_version"; then
    return 3
  fi
  if ! register_update_source_checksum "$source_name" "$remote_sha"; then
    return 3
  fi

  if [[ "$remote_newer" -eq 0 ]]; then
    if [[ "$local_version" == "$remote_version" ]]; then
      if [[ -z "$local_sha" ]]; then
        echo "chimera_update=source_inconsistent source=$source_name current_version=$local_version latest_version=$remote_version current_sha=none latest_sha=$remote_sha action=block reason=local_checksum_missing" >&2
        return 3
      fi
      if [[ "$local_sha" != "$remote_sha" ]]; then
        echo "chimera_update=source_inconsistent source=$source_name current_version=$local_version latest_version=$remote_version current_sha=$local_sha latest_sha=$remote_sha action=block reason=same_version_checksum_mismatch" >&2
        return 3
      fi
      echo "chimera_update=source_current source=$source_name current_version=$local_version latest_version=$remote_version current_sha=$local_sha latest_sha=$remote_sha action=continue reason=source_not_newer" >&2
    else
      echo "chimera_update=source_stale source=$source_name current_version=$local_version latest_version=$remote_version current_sha=${local_sha:-none} latest_sha=$remote_sha action=continue reason=source_not_newer" >&2
    fi
    return "$CHIMERA_UPDATE_SOURCE_NOT_NEWER_RC"
  fi

  installer="$ROOT_DIR/scripts/install_release.sh"
  if [[ ! -f "$installer" ]]; then
    echo "chimera_update=install_failed source=$source_name latest_version=$remote_version action=block reason=missing_local_installer" >&2
    return 3
  fi

  echo "chimera_update=available source=$source_name current_version=$local_version latest_version=$remote_version current_sha=${local_sha:-none} latest_sha=$remote_sha action=install"
  install_role="$(read_local_install_role)"
  install_rc=0
  if CHIMERA_INSTALL_NODE_ROLE="$install_role" CHIMERA_RELEASE_CHECKSUM_URL="$remote_checksum_url" bash "$installer" "$remote_archive_url"; then
    installed_version="$(read_local_runtime_version)"
    installed_sha="$(read_local_runtime_bundle_sha)"
    if [[ "$installed_version" != "$remote_version" || "$installed_sha" != "$remote_sha" ]]; then
      echo "chimera_update=verify_failed source=$source_name expected_version=$remote_version installed_version=${installed_version:-unknown} expected_sha=$remote_sha installed_sha=${installed_sha:-none} action=block" >&2
      return 3
    fi
    local -a update_rerun_args
    mapfile -t update_rerun_args < <(prepare_update_rerun_args "${original_args[@]}")
    if rerun_after_update "${update_rerun_args[@]}"; then
      return 0
    fi
  else
    install_rc=$?
    if [[ "$install_rc" -eq 2 ]]; then
      echo "chimera_update=unavailable source=$source_name latest_version=$remote_version action=continue reason=install_source_unavailable" >&2
      return 2
    fi
  fi

  return 3
}

try_update_from_bootstrap_source() {
  local source_name="${1:?source_name_required}"
  local bootstrap_url="${2:?bootstrap_url_required}"
  local local_version="${3:?local_version_required}"
  local local_sha="${4:-}"
  shift 4 || true
  local -a original_args=("$@")
  local release_meta_output=""
  local -a release_meta=()
  local remote_version remote_archive_url remote_checksum_url remote_metadata_sha metadata_rc

  if release_meta_output="$(read_release_metadata_from_source "$source_name" "$bootstrap_url")"; then
    :
  else
    metadata_rc=$?
    return "$metadata_rc"
  fi

  mapfile -t release_meta <<<"$release_meta_output"
  remote_version="${release_meta[0]:-}"
  remote_archive_url="${release_meta[1]:-}"
  remote_checksum_url="${release_meta[2]:-}"
  remote_metadata_sha="${release_meta[3]:-}"
  if [[ -z "$remote_version" || -z "$remote_archive_url" || -z "$remote_checksum_url" ]]; then
    return 3
  fi
  if ! release_version_to_sortable "$remote_version" >/dev/null; then
    echo "chimera_update=source_invalid source=$source_name latest_version=$remote_version action=hold reason=invalid_release_version" >&2
    return 3
  fi

  install_update_from_release_metadata "$source_name" "$remote_version" "$remote_archive_url" "$remote_checksum_url" "$remote_metadata_sha" "$local_version" "$local_sha" "${original_args[@]}"
}

auto_update_if_needed() {
  local -a original_args=("$@")
  command -v bash >/dev/null 2>&1 || return 0

  local local_version local_sha
  local_version="$(read_local_runtime_version)"
  local_sha="$(read_local_runtime_bundle_sha)"
  local repair_required=0
  if runtime_version_needs_repair "$local_version"; then
    local repaired_version=""
    repaired_version="$(read_local_runtime_version_from_release_bundle 2>/dev/null || true)"
    if runtime_version_needs_repair "$repaired_version"; then
      repair_required=1
      local_version="0.0.0"
    else
      local_version="$repaired_version"
    fi
  fi

  local update_rc gitvers_bootstrap_url normalized_gitvers_url peer_bootstrap_url normalized_peer_url
  local gitvers_verified_not_newer=0 peer_verified_not_newer=0
  local update_failed=0 update_source_unavailable=0
  reset_update_source_authority

  set +e
  try_update_from_bootstrap_source "github" "$UPDATE_BOOTSTRAP_URL" "$local_version" "$local_sha" "${original_args[@]}"
  update_rc=$?
  set -e
  case "$update_rc" in
    0)
      return 0
      ;;
    4)
      emit_no_newer_release_status "$local_version" "$repair_required"
      return 0
      ;;
    2)
      update_source_unavailable=1
      ;;
    3)
      update_failed=1
      ;;

  esac

  while IFS= read -r gitvers_bootstrap_url; do
    normalized_gitvers_url="$(normalize_update_bootstrap_url "$gitvers_bootstrap_url" || true)"
    [[ -n "$normalized_gitvers_url" ]] || continue
    set +e
    try_update_from_bootstrap_source "gitvers" "$normalized_gitvers_url" "$local_version" "$local_sha" "${original_args[@]}"
    update_rc=$?
    set -e
    case "$update_rc" in
      0)
        return 0
        ;;
      4)
        gitvers_verified_not_newer=1
        ;;
      2)
        update_source_unavailable=1
        ;;
      3)
        update_failed=1
        return 1
        ;;
    esac
  done < <(load_update_gitvers_bootstrap_urls)
  if [[ "$gitvers_verified_not_newer" -ne 0 ]]; then
    emit_no_newer_release_status "$local_version" "$repair_required"
    return 0
  fi

  while IFS= read -r peer_bootstrap_url; do
    normalized_peer_url="$(normalize_update_bootstrap_url "$peer_bootstrap_url" || true)"
    [[ -n "$normalized_peer_url" ]] || continue
    set +e
    try_update_from_bootstrap_source "peer" "$normalized_peer_url" "$local_version" "$local_sha" "${original_args[@]}"
    update_rc=$?
    set -e
    case "$update_rc" in
      0)
        return 0
        ;;
      4)
        peer_verified_not_newer=1
        ;;
      2)
        update_source_unavailable=1
        ;;
      3)
        update_failed=1
        return 1
        ;;
    esac
  done < <(load_update_peer_bootstrap_urls_for_args "${original_args[@]}")
  if [[ "$peer_verified_not_newer" -ne 0 ]]; then
    emit_no_newer_release_status "$local_version" "$repair_required"
    return 0
  fi

  if [[ "$update_failed" -ne 0 ]]; then
    return 1
  fi

  if [[ "$update_source_unavailable" -ne 0 ]]; then
    if [[ "$repair_required" -ne 0 ]]; then
      echo "chimera_update=unavailable current_version=unknown action=block reason=local_version_unverified" >&2
      return 1
    fi
    echo "chimera_update=unavailable current_version=$local_version action=continue reason=update_sources_unreachable" >&2
  fi

  return 0
}
