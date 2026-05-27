#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONTROL_SCRIPT="${ROOT_DIR}/scripts/chimera-control.sh"

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

split_csv() {
  local csv="${1:-}"
  IFS=',' read -r -a out <<<"$csv"
  for i in "${!out[@]}"; do
    out[$i]="$(trim "${out[$i]}")"
  done
  printf '%s\n' "${out[@]}"
}

detect_runtime_proxy_url() {
  local rt=""
  if [[ -n "${CHIMERA_PROXY_URL:-}" ]]; then
    printf '%s' "$CHIMERA_PROXY_URL"
    return 0
  fi
  if [[ -x "$CONTROL_SCRIPT" ]]; then
    rt="$(bash "$CONTROL_SCRIPT" route-status 2>/dev/null | awk -F= '$1=="chimera_proxy_url"{print substr($0, index($0,$2)); exit}')"
    rt="$(trim "$rt")"
    if [[ -n "$rt" ]]; then
      printf '%s' "$rt"
      return 0
    fi
  fi
  return 1
}

default_proxy_candidates_csv() {
  local socks_port="${CHIMERA_SOCKS_PORT:-}"
  if [[ -z "$socks_port" ]] && [[ -f "${XDG_CONFIG_HOME:-$HOME/.config}/chimera/upstream_proxy.env" ]]; then
    socks_port="$(awk -F= '$1=="CHIMERA_SOCKS_PORT"{print $2; exit}' "${XDG_CONFIG_HOME:-$HOME/.config}/chimera/upstream_proxy.env" 2>/dev/null || true)"
  fi
  if [[ ! "$socks_port" =~ ^[0-9]+$ ]]; then
    socks_port="12080"
  fi
  printf 'socks5h://127.0.0.1:%s,http://127.0.0.1:18080' "$socks_port"
}

has_listener_for() {
  local hostport="$1"
  local host="${hostport%:*}"
  local port="${hostport##*:}"
  local bind_all="*:${port}"
  local bind_local="127.0.0.1:${port}"
  local bind_local_v6="[::1]:${port}"
  local bind_host="${host}:${port}"
  ss -ltnH 2>/dev/null | awk '{print $4}' | grep -Fxq "$bind_all" && return 0
  ss -ltnH 2>/dev/null | awk '{print $4}' | grep -Fxq "$bind_local" && return 0
  ss -ltnH 2>/dev/null | awk '{print $4}' | grep -Fxq "$bind_local_v6" && return 0
  ss -ltnH 2>/dev/null | awk '{print $4}' | grep -Fxq "$bind_host" && return 0
  return 1
}

http_probe() {
  local url="$1"
  local timeout_sec="$2"
  local mode="default"
  if [[ "${3:-}" == "direct" || "${3:-}" == "default" ]]; then
    mode="$3"
    shift 3
  else
    shift 2
  fi

  local tmp_body
  tmp_body="$(mktemp)"
  local tmp_meta
  tmp_meta="$(mktemp)"

  local curl_exit=0
  if [[ "$mode" == "direct" ]]; then
    if ! env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
      curl -sS -L \
      --noproxy '*' \
      --connect-timeout "$timeout_sec" \
      --max-time "$timeout_sec" \
      -o "$tmp_body" \
      -w '%{http_code} %{remote_ip}' \
      "$@" \
      "$url" >"$tmp_meta" 2>/dev/null; then
      curl_exit=$?
    fi
  else
    if ! curl -sS -L \
      --connect-timeout "$timeout_sec" \
      --max-time "$timeout_sec" \
      -o "$tmp_body" \
      -w '%{http_code} %{remote_ip}' \
      "$@" \
      "$url" >"$tmp_meta" 2>/dev/null; then
      curl_exit=$?
    fi
  fi

  local http_code="000"
  local remote_ip=""
  if [[ -s "$tmp_meta" ]]; then
    http_code="$(awk '{print $1}' "$tmp_meta" 2>/dev/null || true)"
    remote_ip="$(awk '{print $2}' "$tmp_meta" 2>/dev/null || true)"
  fi

  local body=""
  if [[ -s "$tmp_body" ]]; then
    body="$(cat "$tmp_body")"
  fi

  rm -f "$tmp_body" "$tmp_meta"

  printf '%s\t%s\t%s\t%s\n' "$curl_exit" "$http_code" "$remote_ip" "$body"
}

reason_for_probe() {
  local curl_exit="$1"
  local http_code="$2"
  local body="${3:-}"
  if [[ "$curl_exit" != "0" ]]; then
    printf 'curl_exit_%s' "$curl_exit"
    return
  fi
  if [[ -n "$(trim "$body")" ]]; then
    printf 'ok'
    return
  fi
  if [[ "$http_code" == "000" ]]; then
    printf 'no_http_response'
    return
  fi
  if [[ "$http_code" =~ ^2|^3 ]]; then
    printf 'ok'
    return
  fi
  printf 'http_%s' "$http_code"
}

main() {
  local ip_check_url="${CHIMERA_PATH_PROOF_IP_CHECK_URL:-https://api.ipify.org}"
  local targets_csv="${CHIMERA_PATH_PROOF_TARGETS_CSV:-https://example.org,https://api.ipify.org}"
  local proxy_candidates_csv="${CHIMERA_PATH_PROOF_PROXY_CANDIDATES:-$(default_proxy_candidates_csv)}"
  local timeout_sec="${CHIMERA_PATH_PROOF_TIMEOUT_SEC:-8}"
  local allow_same_ip="${CHIMERA_PATH_PROOF_ALLOW_SAME_IP:-0}"
  local json_out="${1:-${CHIMERA_PATH_PROOF_JSON_OUT:-}}"
  local runtime_proxy_url=""

  if runtime_proxy_url="$(detect_runtime_proxy_url)"; then
    if [[ -n "$proxy_candidates_csv" ]]; then
      proxy_candidates_csv="${runtime_proxy_url},${proxy_candidates_csv}"
    else
      proxy_candidates_csv="${runtime_proxy_url}"
    fi
  fi

  mapfile -t targets < <(split_csv "$targets_csv")
  mapfile -t proxies < <(split_csv "$proxy_candidates_csv")

  local started_at
  started_at="$(now_utc)"

  local selected_proxy=""
  local listener_ok="false"
  local listener_reason="no_proxy_selected"

  for p in "${proxies[@]}"; do
    [[ -z "$p" ]] && continue
    local stripped="${p#*://}"
    stripped="${stripped%%/*}"
    if [[ "$stripped" == *"@"* ]]; then
      stripped="${stripped##*@}"
    fi
    if [[ "$stripped" != *:* ]]; then
      continue
    fi
    if has_listener_for "$stripped"; then
      selected_proxy="$p"
      listener_ok="true"
      listener_reason="ok"
      break
    fi
  done

  if [[ -z "$selected_proxy" ]]; then
    if [[ ${#proxies[@]} -gt 0 && -n "${proxies[0]}" ]]; then
      selected_proxy="${proxies[0]}"
      listener_reason="listener_not_found"
    fi
  fi

  local direct_probe
  direct_probe="$(http_probe "$ip_check_url" "$timeout_sec" direct)"
  local direct_exit direct_http direct_remote direct_body
  IFS=$'\t' read -r direct_exit direct_http direct_remote direct_body <<<"$direct_probe"
  local direct_reason
  direct_reason="$(reason_for_probe "$direct_exit" "$direct_http" "$direct_body")"
  local direct_ok="false"
  [[ "$direct_reason" == "ok" ]] && direct_ok="true"
  local direct_ip="$(trim "$direct_body")"

  local chim_probe_exit=""
  local chim_probe_http=""
  local chim_probe_remote=""
  local chim_probe_body=""
  local chimera_reason="proxy_not_checked"
  local chimera_ok="false"
  local chimera_ip=""

  if [[ "$listener_ok" == "true" && -n "$selected_proxy" ]]; then
    local prox_probe
    prox_probe="$(http_probe "$ip_check_url" "$timeout_sec" --proxy "$selected_proxy")"
    IFS=$'\t' read -r chim_probe_exit chim_probe_http chim_probe_remote chim_probe_body <<<"$prox_probe"
    chimera_reason="$(reason_for_probe "$chim_probe_exit" "$chim_probe_http" "$chim_probe_body")"
    if [[ "$chimera_reason" == "ok" ]]; then
      chimera_ok="true"
      chimera_ip="$(trim "$chim_probe_body")"
    fi
  elif [[ -n "$selected_proxy" ]]; then
    chimera_reason="proxy_listener_not_found"
  fi

  local path_proof="fail"
  local path_reason=""
  if [[ "$direct_ok" != "true" ]]; then
    path_reason="direct_path_failed"
  elif [[ "$chimera_ok" != "true" ]]; then
    path_reason="chimera_path_failed:${chimera_reason}"
  elif [[ -n "$direct_ip" && -n "$chimera_ip" && "$direct_ip" == "$chimera_ip" ]]; then
    if [[ "$allow_same_ip" == "1" ]]; then
      path_proof="pass"
      path_reason="same_public_ip_allowed"
    else
      path_reason="same_public_ip"
    fi
  else
    path_proof="pass"
    path_reason="distinct_path_ip"
  fi

  local results_json=""
  local total=0
  local passed=0
  printf 'target\tdirect\tchimera\treason\n'
  for t in "${targets[@]}"; do
    [[ -z "$t" ]] && continue
    total=$((total + 1))

    local td tc tr
    td="$(http_probe "$t" "$timeout_sec" direct)"
    local td_exit td_http td_remote td_body
    IFS=$'\t' read -r td_exit td_http td_remote td_body <<<"$td"
    local td_reason td_ok
    td_reason="$(reason_for_probe "$td_exit" "$td_http" "$td_body")"
    td_ok="false"; [[ "$td_reason" == "ok" ]] && td_ok="true"

    local tc_exit="" tc_http="" tc_remote="" tc_body=""
    local tc_reason="proxy_listener_not_found"
    local tc_ok="false"
    if [[ "$listener_ok" == "true" && -n "$selected_proxy" ]]; then
      tc="$(http_probe "$t" "$timeout_sec" --proxy "$selected_proxy")"
      IFS=$'\t' read -r tc_exit tc_http tc_remote tc_body <<<"$tc"
      tc_reason="$(reason_for_probe "$tc_exit" "$tc_http" "$tc_body")"
      [[ "$tc_reason" == "ok" ]] && tc_ok="true"
    fi

    local row_reason=""
    local row_pass="false"
    if [[ "$td_ok" != "true" ]]; then
      row_reason="direct_failed:${td_reason}"
    elif [[ "$tc_ok" != "true" ]]; then
      row_reason="chimera_failed:${tc_reason}"
    elif [[ -n "$direct_ip" && -n "$chimera_ip" && "$direct_ip" == "$chimera_ip" ]]; then
      row_reason="same_public_ip"
    else
      row_reason="ok"
      row_pass="true"
      passed=$((passed + 1))
    fi

    printf '%s\t%s\t%s\t%s\n' "$t" "$td_reason" "$tc_reason" "$row_reason"

    local row
    row="{\"target\":\"$(json_escape "$t")\",\"direct\":{\"ok\":$td_ok,\"http_code\":\"$td_http\",\"remote_ip\":\"$(json_escape "$td_remote")\",\"reason\":\"$td_reason\"},\"chimera\":{\"ok\":$tc_ok,\"http_code\":\"$tc_http\",\"remote_ip\":\"$(json_escape "$tc_remote")\",\"reason\":\"$tc_reason\"},\"row_pass\":$row_pass,\"row_reason\":\"$row_reason\"}"
    if [[ -n "$results_json" ]]; then
      results_json+=" ,$row"
    else
      results_json="$row"
    fi
  done

  local finished_at
  finished_at="$(now_utc)"
  local summary="{\"kind\":\"chimera_path_proof\",\"status\":\"$path_proof\",\"reason\":\"$path_reason\",\"started_at\":\"$started_at\",\"finished_at\":\"$finished_at\",\"listener\":{\"ok\":$listener_ok,\"reason\":\"$listener_reason\",\"selected_proxy\":\"$(json_escape "$selected_proxy")\"},\"observed_public_ip\":{\"direct\":{\"ok\":$direct_ok,\"ip\":\"$(json_escape "$direct_ip")\",\"reason\":\"$direct_reason\",\"http_code\":\"$direct_http\",\"remote_ip\":\"$(json_escape "$direct_remote")\"},\"chimera\":{\"ok\":$chimera_ok,\"ip\":\"$(json_escape "$chimera_ip")\",\"reason\":\"$chimera_reason\",\"http_code\":\"$chim_probe_http\",\"remote_ip\":\"$(json_escape "$chim_probe_remote")\"}},\"totals\":{\"targets\":$total,\"passed\":$passed,\"failed\":$((total - passed))},\"results\":[${results_json}] }"

  if [[ -n "$json_out" ]]; then
    mkdir -p "$(dirname "$json_out")"
    printf '%s\n' "$summary" >"$json_out"
  fi

  printf '%s\n' "$summary"

  [[ "$path_proof" == "pass" ]]
}

main "$@"
