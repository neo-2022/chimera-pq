#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIG_FILE="${CHIMERA_REAL_WORLD_CONFIG:-$ROOT_DIR/configs/runtime_real_world_probe.env}"

now_utc() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

json_escape() {
  local s="${1:-}"
  s=${s//\\/\\\\}
  s=${s//\"/\\\"}
  s=${s//$'\n'/ }
  s=${s//$'\r'/ }
  s=${s//$'\t'/ }
  printf '%s' "$s"
}

trim() {
  local v="${1:-}"
  v="${v#${v%%[![:space:]]*}}"
  v="${v%${v##*[![:space:]]}}"
  printf '%s' "$v"
}

load_config_file() {
  [[ -f "$CONFIG_FILE" ]] || return 0
  set -a
  # shellcheck disable=SC1090
  source "$CONFIG_FILE"
  set +a
}

split_csv() {
  local csv="${1:-}"
  IFS=',' read -r -a out <<<"$csv"
  for i in "${!out[@]}"; do
    out[$i]="$(trim "${out[$i]}")"
  done
  printf '%s\n' "${out[@]}"
}

http_probe() {
  local url="$1"
  local timeout_sec="$2"
  local tmp_body
  tmp_body="$(mktemp)"
  local tmp_meta
  tmp_meta="$(mktemp)"

  local curl_exit=0
  if ! env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
    curl -sS -L \
    --noproxy '*' \
    --connect-timeout "$timeout_sec" \
    --max-time "$timeout_sec" \
    -o "$tmp_body" \
    -w '%{http_code} %{remote_ip}' \
    "$url" >"$tmp_meta" 2>/dev/null; then
    curl_exit=$?
  fi

  local http_code="000"
  local remote_ip=""
  if [[ -s "$tmp_meta" ]]; then
    http_code="$(awk '{print $1}' "$tmp_meta" 2>/dev/null || true)"
    remote_ip="$(awk '{print $2}' "$tmp_meta" 2>/dev/null || true)"
  fi

  rm -f "$tmp_body" "$tmp_meta"

  printf '%s\t%s\t%s\n' "$curl_exit" "$http_code" "$remote_ip"
}

reason_for_probe() {
  local curl_exit="$1"
  local http_code="$2"
  if [[ "$curl_exit" != "0" ]]; then
    printf 'curl_exit_%s' "$curl_exit"
    return
  fi
  if [[ "$http_code" =~ ^2|^3 ]]; then
    printf 'ok'
    return
  fi
  if [[ "$http_code" == "000" ]]; then
    printf 'no_http_response'
    return
  fi
  printf 'http_%s' "$http_code"
}

main() {
  load_config_file

  local direct_url="${CHIMERA_PATH_PROOF_DIRECT_URL:-${CHIMERA_REAL_WORLD_DIRECT_URL:-}}"
  local targets_csv="${CHIMERA_PATH_PROOF_TARGETS_CSV:-${CHIMERA_REAL_WORLD_DATAPATH_TARGETS:-}}"
  local timeout_sec="${CHIMERA_PATH_PROOF_TIMEOUT_SEC:-${CHIMERA_REAL_WORLD_DATAPATH_TIMEOUT_SEC:-12}}"
  local json_out="${1:-${CHIMERA_PATH_PROOF_JSON_OUT:-}}"

  if [[ -z "$(trim "$direct_url")" || -z "$(trim "$targets_csv")" ]]; then
    echo "chimera path proof: CHIMERA_PATH_PROOF_DIRECT_URL/CHIMERA_PATH_PROOF_TARGETS_CSV or runtime_real_world_probe.env values are required" >&2
    exit 2
  fi
  if ! [[ "$timeout_sec" =~ ^[0-9]+$ ]] || (( timeout_sec < 1 )); then
    echo "chimera path proof: timeout must be a positive integer" >&2
    exit 2
  fi

  mapfile -t targets < <(split_csv "$targets_csv")

  local started_at
  started_at="$(now_utc)"

  local direct_probe direct_exit direct_http direct_remote direct_reason direct_ok
  direct_probe="$(http_probe "$direct_url" "$timeout_sec")"
  IFS=$'\t' read -r direct_exit direct_http direct_remote <<<"$direct_probe"
  direct_reason="$(reason_for_probe "$direct_exit" "$direct_http")"
  direct_ok="false"
  [[ "$direct_reason" == "ok" ]] && direct_ok="true"

  local results_json=""
  local total=0
  local passed=0
  local failed=0
  printf 'target\tdatapath\treason\n'
  local t
  for t in "${targets[@]}"; do
    [[ -z "$t" ]] && continue
    total=$((total + 1))

    local probe exit_code http_code remote_ip reason ok row
    probe="$(http_probe "$t" "$timeout_sec")"
    IFS=$'\t' read -r exit_code http_code remote_ip <<<"$probe"
    reason="$(reason_for_probe "$exit_code" "$http_code")"
    ok="false"
    if [[ "$reason" == "ok" ]]; then
      ok="true"
      passed=$((passed + 1))
    else
      failed=$((failed + 1))
    fi

    printf '%s\t%s\t%s\n' "$t" "$ok" "$reason"
    row="{\"target\":\"$(json_escape "$t")\",\"datapath\":{\"ok\":$ok,\"http_code\":\"$http_code\",\"remote_ip\":\"$(json_escape "$remote_ip")\",\"reason\":\"$reason\"},\"row_pass\":$ok,\"row_reason\":\"$reason\"}"
    if [[ -n "$results_json" ]]; then
      results_json+=",$row"
    else
      results_json="$row"
    fi
  done

  local path_proof="fail"
  local path_reason="datapath_targets_failed"
  if [[ "$direct_ok" != "true" ]]; then
    path_reason="direct_baseline_failed:${direct_reason}"
  elif [[ "$total" -eq 0 ]]; then
    path_reason="no_datapath_targets"
  elif [[ "$failed" -eq 0 ]]; then
    path_proof="pass"
    path_reason="transparent_datapath_targets_ok"
  fi

  local finished_at
  finished_at="$(now_utc)"
  local summary="{\"kind\":\"chimera_path_proof\",\"status\":\"$path_proof\",\"reason\":\"$path_reason\",\"mode\":\"transparent_datapath\",\"started_at\":\"$started_at\",\"finished_at\":\"$finished_at\",\"direct_baseline\":{\"url\":\"$(json_escape "$direct_url")\",\"ok\":$direct_ok,\"http_code\":\"$direct_http\",\"remote_ip\":\"$(json_escape "$direct_remote")\",\"reason\":\"$direct_reason\"},\"datapath\":{\"attempted\":true,\"ok\":$([[ "$path_proof" == "pass" ]] && echo true || echo false),\"targets_total\":$total,\"targets_passed\":$passed,\"targets_failed\":$failed},\"results\":[${results_json}],\"network_state\":\"not_modified\"}"

  if [[ -n "$json_out" ]]; then
    mkdir -p "$(dirname "$json_out")"
    printf '%s\n' "$summary" >"$json_out"
  fi

  printf '%s\n' "$summary"

  [[ "$path_proof" == "pass" ]]
}

main "$@"
