#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UPSTREAM_ENV_FILE="${UPSTREAM_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/upstream_proxy.env}"
POOL_FILE_DEFAULT="${XDG_CONFIG_HOME:-$HOME/.config}/chimera/upstream_pool.list"
POOL_FILE_FALLBACK="$ROOT_DIR/configs/upstream_pool.list"
PROBE_TIMEOUT_SEC="${CHIMERA_UPSTREAM_BOOTSTRAP_PROBE_TIMEOUT_SEC:-3}"

trim_ascii() {
  local s="${1:-}"
  s="${s#"${s%%[![:space:]]*}"}"
  s="${s%"${s##*[![:space:]]}"}"
  printf '%s' "$s"
}

split_csv_lines() {
  local csv="${1:-}"
  IFS=',' read -r -a out <<<"$csv"
  local item
  for item in "${out[@]}"; do
    item="$(trim_ascii "$item")"
    [[ -z "$item" ]] && continue
    printf '%s\n' "$item"
  done
}

parse_transport_endpoint() {
  local candidate="${1:-}"
  local transport="ssh"
  local endpoint="$candidate"
  if [[ "$candidate" == *@* ]]; then
    transport="${candidate%%@*}"
    endpoint="${candidate#*@}"
  fi
  transport="$(trim_ascii "${transport,,}")"
  endpoint="$(trim_ascii "$endpoint")"
  if [[ -z "$endpoint" ]]; then
    return 1
  fi
  local host="${endpoint%:*}"
  local port="${endpoint##*:}"
  if [[ "$host" == "$port" ]]; then
    case "$transport" in
      ssh443|ssh-443|tls|tcp443) port="443" ;;
      ssh8443|ssh-8443|tcp8443) port="8443" ;;
      *) port="${CHIMERA_UPSTREAM_PORT:-22}" ;;
    esac
    endpoint="${host}:${port}"
  fi
  printf '%s|%s\n' "$transport" "$endpoint"
}

probe_endpoint_ms() {
  local candidate="${1:-}"
  local parsed endpoint host port
  parsed="$(parse_transport_endpoint "$candidate" || true)"
  endpoint="${parsed#*|}"
  host="${endpoint%:*}"
  port="${endpoint##*:}"
  [[ -z "$host" || -z "$port" ]] && echo 2147483647 && return 0
  local start end
  start="$(date +%s 2>/dev/null || echo 0)"
  if timeout "$PROBE_TIMEOUT_SEC" bash -lc "</dev/tcp/$host/$port" >/dev/null 2>&1; then
    end="$(date +%s 2>/dev/null || echo 0)"
    if [[ "$start" =~ ^[0-9]+$ && "$end" =~ ^[0-9]+$ && "$end" -ge "$start" ]]; then
      echo $(((end - start) * 1000))
    else
      echo 1
    fi
  else
    echo 2147483647
  fi
}

load_pool_candidates() {
  if [[ -n "${CHIMERA_UPSTREAM_ENDPOINTS_CSV:-}" ]]; then
    split_csv_lines "$CHIMERA_UPSTREAM_ENDPOINTS_CSV"
    return 0
  fi
  local pool_file="${CHIMERA_UPSTREAM_POOL_FILE:-$POOL_FILE_DEFAULT}"
  if [[ ! -f "$pool_file" && -f "$POOL_FILE_FALLBACK" ]]; then
    pool_file="$POOL_FILE_FALLBACK"
  fi
  [[ -f "$pool_file" ]] || return 0
  awk '
    { line=$0; sub(/[[:space:]]*#.*/, "", line); gsub(/^[[:space:]]+|[[:space:]]+$/, "", line); if (line != "") print line; }
  ' "$pool_file"
}

main() {
  local user="${CHIMERA_UPSTREAM_POOL_USER:-${CHIMERA_UPSTREAM_USER:-}}"
  local pass="${CHIMERA_UPSTREAM_POOL_PASS:-${CHIMERA_UPSTREAM_PASS:-}}"
  if [[ -z "$user" || -z "$pass" ]]; then
    echo "upstream-autobootstrap: skip missing credentials (CHIMERA_UPSTREAM_POOL_USER/PASS)" >&2
    return 0
  fi

  local candidates=()
  local c
  while IFS= read -r c; do
    [[ -z "$c" ]] && continue
    candidates+=("$c")
  done < <(load_pool_candidates)
  if [[ "${#candidates[@]}" -eq 0 && -n "${CHIMERA_UPSTREAM_HOST:-}" ]]; then
    local host="${CHIMERA_UPSTREAM_HOST}"
    local p22="${CHIMERA_UPSTREAM_PORT:-22}"
    candidates+=("${host}:${p22}" "${host}:443" "${host}:8443")
  fi
  if [[ "${#candidates[@]}" -eq 0 ]]; then
    echo "upstream-autobootstrap: skip no candidate endpoints" >&2
    return 0
  fi

  local best="" best_ms=2147483647
  local cand ms parsed endpoint
  local normalized=()
  for cand in "${candidates[@]}"; do
    parsed="$(parse_transport_endpoint "$cand" || true)"
    endpoint="${parsed#*|}"
    [[ -z "$endpoint" ]] && continue
    normalized+=("$endpoint")
    ms="$(probe_endpoint_ms "$cand")"
    if [[ "$ms" =~ ^[0-9]+$ ]] && [[ "$ms" -lt "$best_ms" ]]; then
      best_ms="$ms"
      best="$endpoint"
    fi
  done
  if [[ -z "$best" ]]; then
    echo "upstream-autobootstrap: skip no reachable endpoint" >&2
    return 0
  fi

  local host="${best%:*}"
  local port="${best##*:}"
  mkdir -p "$(dirname "$UPSTREAM_ENV_FILE")"
  {
    printf 'CHIMERA_UPSTREAM_USER=%s\n' "$user"
    printf 'CHIMERA_UPSTREAM_HOST=%s\n' "$host"
    printf 'CHIMERA_UPSTREAM_PORT=%s\n' "$port"
    printf 'CHIMERA_UPSTREAM_PASS=%s\n' "$pass"
    printf 'CHIMERA_UPSTREAM_ENDPOINTS_CSV=%s\n' "$(IFS=,; echo "${normalized[*]}")"
  } >"$UPSTREAM_ENV_FILE"
  chmod 600 "$UPSTREAM_ENV_FILE"
  echo "upstream-autobootstrap: selected=$best candidates=${#normalized[@]} file=$UPSTREAM_ENV_FILE"
}

main "$@"
