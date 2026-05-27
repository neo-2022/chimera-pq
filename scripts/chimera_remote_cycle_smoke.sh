#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  chimera_remote_cycle_smoke.sh --host <ip_or_host> --user <user> --pass <password> [--cycles N]

Example:
  chimera_remote_cycle_smoke.sh --host <remote_host> --user <remote_user> --pass '<remote_password>' --cycles 3
EOF
}

HOST=""
USER_NAME=""
PASSWD=""
CYCLES=3

while [[ $# -gt 0 ]]; do
  case "$1" in
    --host) HOST="${2:-}"; shift 2 ;;
    --user) USER_NAME="${2:-}"; shift 2 ;;
    --pass) PASSWD="${2:-}"; shift 2 ;;
    --cycles) CYCLES="${2:-3}"; shift 2 ;;
    -h|--help|help) usage; exit 0 ;;
    *)
      echo "error: unknown arg: $1" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ -z "$HOST" || -z "$USER_NAME" || -z "$PASSWD" ]]; then
  echo "error: --host/--user/--pass are required" >&2
  usage
  exit 2
fi
if ! [[ "$CYCLES" =~ ^[0-9]+$ ]] || [[ "$CYCLES" -lt 1 ]]; then
  echo "error: --cycles must be >= 1" >&2
  exit 2
fi

if ! command -v sshpass >/dev/null 2>&1; then
  echo "error: sshpass is required" >&2
  exit 2
fi

REMOTE_CMD=$(cat <<'EOF'
set -euo pipefail
latest_sha="$(env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
  curl -fsSL https://api.github.com/repos/neo-2022/chimera/commits/main \
  | sed -n 's/^[[:space:]]*"sha":[[:space:]]*"\([0-9a-f]\{40\}\)".*/\1/p' | head -n1)"
if [[ -z "$latest_sha" ]]; then
  echo "error: failed to resolve latest main sha"
  exit 1
fi
bootstrap_url="https://raw.githubusercontent.com/neo-2022/chimera/${latest_sha}/chimera.sh"
for i in $(seq 1 "__CYCLES__"); do
  echo "cycle=$i step=install"
  env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
    bash -lc 'curl -fsSL "'"$bootstrap_url"'" | bash -s -- -install' >/tmp/chimera_cycle_install.log 2>&1

  echo "cycle=$i step=start"
  "$HOME/.local/bin/chimera.sh" -start >/tmp/chimera_cycle_start.log 2>&1
  rc_start=$?

  echo "cycle=$i step=status"
  "$HOME/.local/bin/chimera.sh" -status >/tmp/chimera_cycle_status.log 2>&1
  rc_status=$?

  echo "cycle=$i step=stop"
  "$HOME/.local/bin/chimera.sh" -stop >/tmp/chimera_cycle_stop.log 2>&1
  rc_stop=$?

  echo "cycle=$i step=uninstall"
  "$HOME/.local/bin/chimera.sh" -uninstall >/tmp/chimera_cycle_uninstall.log 2>&1
  rc_uninstall=$?

  echo "cycle=$i rc_start=$rc_start rc_status=$rc_status rc_stop=$rc_stop rc_uninstall=$rc_uninstall"
  if [[ "$rc_start" -ne 0 || "$rc_status" -ne 0 || "$rc_stop" -ne 0 || "$rc_uninstall" -ne 0 ]]; then
    echo "cycle=$i result=fail"
    exit 1
  fi
  echo "cycle=$i result=ok"
done
echo "smoke_result=pass cycles=__CYCLES__"
EOF
)
REMOTE_CMD="${REMOTE_CMD//__CYCLES__/$CYCLES}"

sshpass -p "$PASSWD" ssh \
  -o StrictHostKeyChecking=no \
  -o ConnectTimeout=12 \
  "$USER_NAME@$HOST" \
  "$REMOTE_CMD"
