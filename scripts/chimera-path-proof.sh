#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIG_FILE="${CHIMERA_REAL_WORLD_CONFIG:-$ROOT_DIR/configs/runtime_real_world_probe.env}"
DEFAULT_STATE_FILE="$ROOT_DIR/docs/runtime_state_latest.json"

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
  set +e
  env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
    curl -sS -L \
    --noproxy '*' \
    --connect-timeout "$timeout_sec" \
    --max-time "$timeout_sec" \
    -o "$tmp_body" \
    -w '%{http_code} %{remote_ip}' \
    "$url" >"$tmp_meta" 2>/dev/null
  curl_exit=$?
  set -e

  local http_code="000"
  local remote_ip=""
  if [[ -s "$tmp_meta" ]]; then
    http_code="$(awk '{print $1}' "$tmp_meta" 2>/dev/null || true)"
    remote_ip="$(awk '{print $2}' "$tmp_meta" 2>/dev/null || true)"
  fi

  rm -f "$tmp_body" "$tmp_meta"

  printf '%s\t%s\t%s\n' "$curl_exit" "$http_code" "$remote_ip"
}

target_ref() {
  local idx="${1:?idx_required}"
  printf 'target#%s' "$idx"
}

presence_bool() {
  if [[ -n "$(trim "${1:-}")" ]]; then
    printf 'true'
  else
    printf 'false'
  fi
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

resolve_cli_bin() {
  local candidate
  for candidate in \
    "${CHIMERA_PATH_PROOF_CLI:-}" \
    "${CHIMERA_REAL_WORLD_CLI:-}" \
    "${CHIMERA_CLI_BIN:-}" \
    "$ROOT_DIR/bin/chimera-cli" \
    "$ROOT_DIR/target/debug/chimera-cli"
  do
    [[ -n "$candidate" ]] || continue
    [[ -x "$candidate" ]] || continue
    printf '%s' "$candidate"
    return 0
  done
  return 1
}

strict_flow_proof_state() {
  local cli_bin="${1:?cli_bin_required}"
  local state_file="${2:?state_file_required}"
  local max_age_sec="${3:?max_age_sec_required}"
  local output rc
  output="$(
    "$cli_bin" state proof \
      --state-file "$state_file" \
      --require-flow true \
      --max-flow-age-sec "$max_age_sec" \
      2>/dev/null
  )" && rc=0 || rc=$?
  if [[ "$output" =~ (^|[[:space:]])datapath_proof=([A-Za-z0-9_:-]+) ]]; then
    printf '%s' "${BASH_REMATCH[2]}"
    return "$rc"
  fi
  printf '%s' "flow_proof_command_failed"
  return 1
}

main() {
  load_config_file

  local direct_url="${CHIMERA_PATH_PROOF_DIRECT_URL:-${CHIMERA_REAL_WORLD_DIRECT_URL:-}}"
  local targets_csv="${CHIMERA_PATH_PROOF_TARGETS_CSV:-${CHIMERA_REAL_WORLD_DATAPATH_TARGETS:-}}"
  local timeout_sec="${CHIMERA_PATH_PROOF_TIMEOUT_SEC:-${CHIMERA_REAL_WORLD_DATAPATH_TIMEOUT_SEC:-12}}"
  local state_file="${CHIMERA_PATH_PROOF_STATE_FILE:-${CHIMERA_REAL_WORLD_STATE_FILE:-${STATE_FILE:-$DEFAULT_STATE_FILE}}}"
  local max_flow_age_sec="${CHIMERA_PATH_PROOF_MAX_FLOW_AGE_SEC:-${CHIMERA_REAL_WORLD_FLOW_MAX_AGE_SEC:-300}}"
  local json_out="${1:-${CHIMERA_PATH_PROOF_JSON_OUT:-}}"
  local cli_bin=""

  if [[ -z "$(trim "$direct_url")" || -z "$(trim "$targets_csv")" ]]; then
    echo "chimera path proof: CHIMERA_PATH_PROOF_DIRECT_URL/CHIMERA_PATH_PROOF_TARGETS_CSV or runtime_real_world_probe.env values are required" >&2
    exit 2
  fi
  if ! [[ "$timeout_sec" =~ ^[0-9]+$ ]] || (( timeout_sec < 1 )); then
    echo "chimera path proof: timeout must be a positive integer" >&2
    exit 2
  fi
  if ! [[ "$max_flow_age_sec" =~ ^[0-9]+$ ]] || (( max_flow_age_sec < 1 )); then
    echo "chimera path proof: max flow age must be a positive integer" >&2
    exit 2
  fi
  cli_bin="$(resolve_cli_bin || true)"

  mapfile -t targets < <(split_csv "$targets_csv")

  local started_at
  started_at="$(now_utc)"

  local direct_probe direct_exit direct_http direct_remote direct_reason direct_ok
  direct_probe="$(http_probe "$direct_url" "$timeout_sec")"
  IFS=$'\t' read -r direct_exit direct_http direct_remote <<<"$direct_probe"
  direct_reason="$(reason_for_probe "$direct_exit" "$direct_http")"
  direct_ok="false"
  [[ "$direct_reason" == "ok" ]] && direct_ok="true"

  local total=0
  local passed=0
  local failed=0
  local -a result_refs=()
  local -a external_ok_rows=()
  local -a external_http_codes=()
  local -a external_remote_present_rows=()
  local -a external_reasons_rows=()
  printf 'target_ref\texternal_reachability\treason\n'
  local t
  local target_idx=0
  for t in "${targets[@]}"; do
    [[ -z "$t" ]] && continue
    total=$((total + 1))
    target_idx=$((target_idx + 1))

    local probe exit_code http_code remote_ip reason ok row ref remote_ip_present
    ref="$(target_ref "$target_idx")"
    probe="$(http_probe "$t" "$timeout_sec")"
    IFS=$'\t' read -r exit_code http_code remote_ip <<<"$probe"
    reason="$(reason_for_probe "$exit_code" "$http_code")"
    remote_ip_present="$(presence_bool "$remote_ip")"
    ok="false"
    if [[ "$reason" == "ok" ]]; then
      ok="true"
      passed=$((passed + 1))
    else
      failed=$((failed + 1))
    fi

    printf '%s\t%s\t%s\n' "$ref" "$ok" "$reason"
    result_refs+=("$ref")
    external_ok_rows+=("$ok")
    external_http_codes+=("$http_code")
    external_remote_present_rows+=("$remote_ip_present")
    external_reasons_rows+=("$reason")
  done

  local path_mode="external_reachability_without_system_proxy"
  local chimera_datapath_evidence="false"
  local truth_boundary="ordinary curl --noproxy proves external reachability without system proxy only; it is not CHIMERA/WEAVE datapath evidence"
  local datapath_attempted="false"
  local datapath_ok="false"
  local datapath_total=0
  local datapath_passed=0
  local datapath_failed=0
  local flow_proof_state="flow_cli_missing"
  if [[ -n "$cli_bin" ]]; then
    flow_proof_state="$(strict_flow_proof_state "$cli_bin" "$state_file" "$max_flow_age_sec" || true)"
  fi
  if [[ "$flow_proof_state" == "ok" ]]; then
    path_mode="chimera_transparent_datapath"
    chimera_datapath_evidence="true"
    truth_boundary="strict CHIMERA flow-proof plus ordinary target outcomes provide CHIMERA/WEAVE datapath evidence; direct curl remains an external baseline only"
    datapath_attempted="true"
    datapath_total="$total"
    datapath_passed="$passed"
    datapath_failed="$failed"
    if [[ "$datapath_failed" -eq 0 && "$datapath_total" -gt 0 ]]; then
      datapath_ok="true"
    fi
  fi

  local results_json=""
  local row=""
  local idx=0
  for idx in "${!result_refs[@]}"; do
    row="{\"target_ref\":\"$(json_escape "${result_refs[$idx]}")\",\"external_reachability\":{\"ok\":${external_ok_rows[$idx]},\"http_code\":\"${external_http_codes[$idx]}\",\"remote_ip_present\":${external_remote_present_rows[$idx]},\"reason\":\"${external_reasons_rows[$idx]}\"},\"row_pass\":${external_ok_rows[$idx]},\"row_reason\":\"${external_reasons_rows[$idx]}\"}"
    if [[ -n "$results_json" ]]; then
      results_json+=",$row"
    else
      results_json="$row"
    fi
  done

  local path_proof="not_done"
  local path_reason="chimera_datapath_evidence_missing"
  if [[ "$chimera_datapath_evidence" == "true" ]]; then
    if [[ "$datapath_ok" == "true" ]]; then
      path_proof="pass"
      path_reason="ok"
    else
      path_proof="fail"
      path_reason="datapath_target_failed"
    fi
  elif [[ -n "$cli_bin" ]]; then
    path_reason="flow_proof_${flow_proof_state}"
  elif [[ "$direct_ok" != "true" ]]; then
    path_proof="fail"
    path_reason="direct_baseline_failed:${direct_reason}"
  elif [[ "$total" -eq 0 ]]; then
    path_proof="fail"
    path_reason="no_datapath_targets"
  elif [[ "$failed" -ne 0 ]]; then
    path_proof="fail"
    path_reason="external_reachability_targets_failed"
  fi

  local finished_at
  finished_at="$(now_utc)"
  local direct_remote_present
  direct_remote_present="$(presence_bool "$direct_remote")"
  local summary="{\"kind\":\"chimera_path_proof\",\"status\":\"$path_proof\",\"reason\":\"$path_reason\",\"mode\":\"$path_mode\",\"chimera_datapath_evidence\":$chimera_datapath_evidence,\"truth_boundary\":\"$truth_boundary\",\"started_at\":\"$started_at\",\"finished_at\":\"$finished_at\",\"redaction\":\"raw_targets_and_remote_ips_redacted\",\"direct_baseline\":{\"target_ref\":\"direct#1\",\"ok\":$direct_ok,\"http_code\":\"$direct_http\",\"remote_ip_present\":$direct_remote_present,\"reason\":\"$direct_reason\"},\"datapath\":{\"attempted\":$datapath_attempted,\"ok\":$datapath_ok,\"targets_total\":$datapath_total,\"targets_passed\":$datapath_passed,\"targets_failed\":$datapath_failed,\"flow_proof\":\"$flow_proof_state\"},\"external_reachability\":{\"attempted\":true,\"ok\":$([[ "$failed" -eq 0 && "$total" -gt 0 ]] && echo true || echo false),\"targets_total\":$total,\"targets_passed\":$passed,\"targets_failed\":$failed},\"results\":[${results_json}],\"network_state\":\"not_modified\"}"

  if [[ -n "$json_out" ]]; then
    mkdir -p "$(dirname "$json_out")"
    printf '%s\n' "$summary" >"$json_out"
  fi

  printf '%s\n' "$summary"

  [[ "$path_proof" == "pass" ]]
}

main "$@"
