#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

: "${CHIMERA_LAPTOP_HOST:?CHIMERA_LAPTOP_HOST is required}"
: "${CHIMERA_LAPTOP_USER:?CHIMERA_LAPTOP_USER is required}"
: "${CHIMERA_LAPTOP_PASS:?CHIMERA_LAPTOP_PASS is required}"
: "${CHIMERA_LAPTOP_REPO:?CHIMERA_LAPTOP_REPO is required}"
: "${CHIMERA_LOAD_TARGETS_FILE:?CHIMERA_LOAD_TARGETS_FILE is required}"

REMOTE_HOST="$CHIMERA_LAPTOP_HOST"
REMOTE_USER="$CHIMERA_LAPTOP_USER"
REMOTE_PASS="$CHIMERA_LAPTOP_PASS"
REMOTE_REPO="$CHIMERA_LAPTOP_REPO"
REMOTE_DURATION_SEC="${CHIMERA_LOAD_DURATION_SEC:-300}"
REMOTE_TIMEOUT_SEC="${CHIMERA_LOAD_TIMEOUT_SEC:-12}"
REMOTE_CONNECT_TIMEOUT_SEC="${CHIMERA_LOAD_CONNECT_TIMEOUT_SEC:-5}"
REMOTE_PROXY_FALLBACK="${CHIMERA_PROXY_FALLBACK:-}"
OUT_DIR="${1:-$ROOT_DIR/docs/load}"
TS="$(date +%Y%m%d_%H%M%S)"
LOCAL_OUT="$OUT_DIR/CHIMERA_LOAD_${REMOTE_DURATION_SEC}S_LAPTOP_${TS}.json"

mkdir -p "$OUT_DIR"

if ! command -v sshpass >/dev/null 2>&1; then
  echo "chimera-load-5m-laptop: sshpass is required" >&2
  exit 1
fi

if [[ ! -f "$CHIMERA_LOAD_TARGETS_FILE" ]]; then
  echo "chimera-load-5m-laptop: CHIMERA_LOAD_TARGETS_FILE not found" >&2
  exit 1
fi

TARGETS_PAYLOAD="$(sed '/^[[:space:]]*$/d;/^[[:space:]]*#/d' "$CHIMERA_LOAD_TARGETS_FILE")"
if [[ -z "$TARGETS_PAYLOAD" ]]; then
  echo "chimera-load-5m-laptop: target list is empty" >&2
  exit 1
fi

REMOTE_CMD=$(
  cat <<'EOF_REMOTE'
set -euo pipefail
cd "$REMOTE_REPO"
proxy_url="$(bash scripts/chimera-control.sh route-status | sed -n 's/^chimera_proxy_url=//p' | head -n 1)"
if [[ -z "$proxy_url" ]]; then
  proxy_url="$REMOTE_PROXY_FALLBACK"
fi
if [[ -z "$proxy_url" ]]; then
  echo "chimera-load-5m-laptop: proxy URL is unavailable" >&2
  exit 1
fi

tmp_out="docs/CHIMERA_LOAD_${REMOTE_DURATION_SEC}S_${TS}.json"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

i=0
while IFS= read -r site; do
  [[ -n "$site" ]] || continue
  i=$((i + 1))
  (
    stop=$(( $(date +%s) + REMOTE_DURATION_SEC ))
    ok=0
    fail=0
    last_error=""
    codes_file="$tmp_dir/codes_$i.txt"
    : > "$codes_file"
    while (( $(date +%s) < stop )); do
      err_file="$tmp_dir/err_${i}_$$.txt"
      code="$(curl -sS -L --proxy "$proxy_url" \
        --connect-timeout "$REMOTE_CONNECT_TIMEOUT_SEC" \
        --max-time "$REMOTE_TIMEOUT_SEC" \
        -o /dev/null -w '%{http_code}' "$site" 2>"$err_file" || true)"
      code="${code:-000}"
      printf '%s\n' "$code" >> "$codes_file"
      if [[ "$code" != "000" ]]; then
        ok=$((ok + 1))
      else
        fail=$((fail + 1))
        last_error="$(tail -n 1 "$err_file" | cut -c1-300)"
      fi
      rm -f "$err_file"
    done
    {
      printf 'site=%s\n' "$site"
      printf 'ok=%s\n' "$ok"
      printf 'fail=%s\n' "$fail"
      printf 'last_error=%s\n' "$last_error"
      sort "$codes_file" | uniq -c | awk '{print "code_" $2 "=" $1}'
    } > "$tmp_dir/site_$i.env"
  ) &
done <<EOF_TARGETS
$TARGETS_PAYLOAD
EOF_TARGETS
wait

json_escape() {
  sed 's/\\/\\\\/g; s/"/\\"/g' <<<"$1" | tr -d '\n'
}

{
  printf '{"kind":"chimera_load_test","duration_sec":%s,"proxy":"%s","generated_at":%s,"sites":[' \
    "$REMOTE_DURATION_SEC" "$(json_escape "$proxy_url")" "$(date +%s)"
  first=1
  for file in "$tmp_dir"/site_*.env; do
    [[ -f "$file" ]] || continue
    site="" ok=0 fail=0 last_error=""
    codes_json=""
    while IFS='=' read -r key value; do
      case "$key" in
        site) site="$value" ;;
        ok) ok="$value" ;;
        fail) fail="$value" ;;
        last_error) last_error="$value" ;;
        code_*)
          code="${key#code_}"
          if [[ -n "$codes_json" ]]; then codes_json=",$codes_json"; fi
          codes_json="\"$(json_escape "$code")\":$value$codes_json"
          ;;
      esac
    done < "$file"
    total=$((ok + fail))
    if (( total > 0 )); then
      success_rate="$(awk -v ok="$ok" -v total="$total" 'BEGIN { printf "%.6f", ok / total }')"
    else
      success_rate="0.000000"
    fi
    if (( first == 0 )); then printf ','; fi
    first=0
    printf '{"site":"%s","ok":%s,"fail":%s,"success_rate":%s,"codes":{%s},"last_error":"%s"}' \
      "$(json_escape "$site")" "$ok" "$fail" "$success_rate" "$codes_json" "$(json_escape "$last_error")"
  done
  printf ']}'
} > "$tmp_out"

sed -n '1,260p' "$tmp_out"
echo
echo "__REMOTE_OUT__=$tmp_out"
EOF_REMOTE
)

REMOTE_OUTPUT="$(sshpass -p "$REMOTE_PASS" ssh -o StrictHostKeyChecking=no \
  "$REMOTE_USER@$REMOTE_HOST" \
  REMOTE_REPO="$REMOTE_REPO" \
  REMOTE_DURATION_SEC="$REMOTE_DURATION_SEC" \
  REMOTE_TIMEOUT_SEC="$REMOTE_TIMEOUT_SEC" \
  REMOTE_CONNECT_TIMEOUT_SEC="$REMOTE_CONNECT_TIMEOUT_SEC" \
  REMOTE_PROXY_FALLBACK="$REMOTE_PROXY_FALLBACK" \
  TS="$TS" \
  TARGETS_PAYLOAD="$TARGETS_PAYLOAD" \
  bash -s <<<"$REMOTE_CMD")"
printf '%s\n' "$REMOTE_OUTPUT"

REMOTE_OUT_PATH="$(printf '%s\n' "$REMOTE_OUTPUT" | awk -F= '/^__REMOTE_OUT__=/{print $2; exit}')"
if [[ -z "$REMOTE_OUT_PATH" ]]; then
  echo "chimera-load-5m-laptop: failed to discover remote output path" >&2
  exit 1
fi

sshpass -p "$REMOTE_PASS" scp -o StrictHostKeyChecking=no \
  "$REMOTE_USER@$REMOTE_HOST:$REMOTE_REPO/$REMOTE_OUT_PATH" "$LOCAL_OUT" >/dev/null

echo "local_artifact=$LOCAL_OUT"
