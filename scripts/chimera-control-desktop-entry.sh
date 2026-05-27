#!/usr/bin/env bash
set -euo pipefail

resolve_self() {
  local src="${BASH_SOURCE[0]}"
  while [[ -L "$src" ]]; do
    local dir
    dir="$(cd "$(dirname "$src")" && pwd)"
    src="$(readlink "$src")"
    [[ "$src" != /* ]] && src="$dir/$src"
  done
  cd "$(dirname "$src")" && pwd
}

SCRIPT_DIR="$(resolve_self)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
CONTROL="$ROOT_DIR/scripts/chimera-control.sh"
TRAY="$ROOT_DIR/scripts/chimera-control-tray.sh"
LOG_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/chimera"
ENTRY_LOG="$LOG_DIR/desktop-entry.log"
mkdir -p "$LOG_DIR"

{
  echo "--- $(date '+%F %T') desktop-entry start ---"
  echo "session=${XDG_SESSION_TYPE:-unknown} display=${DISPLAY:-none} wayland=${WAYLAND_DISPLAY:-none}"
} >>"$ENTRY_LOG"

choose_action() {
  if command -v zenity >/dev/null 2>&1; then
    zenity --list \
      --title="CHIMERA Control" \
      --text="Choose action" \
      --column="Action" \
      "Start CHIMERA" "Stop CHIMERA" "Restart CHIMERA" "Status" "Doctor" "Logs" "Nodes" "Start Tray Menu" "Quit" \
      --height=380 --width=380 2>/dev/null || true
    return
  fi

  if command -v kdialog >/dev/null 2>&1; then
    kdialog --menu "CHIMERA Control" \
      start "Start CHIMERA" \
      stop "Stop CHIMERA" \
      restart "Restart CHIMERA" \
      status "Status" \
      doctor "Doctor" \
      logs "Logs" \
      nodes "Nodes" \
      tray "Start Tray Menu" \
      quit "Quit" 2>/dev/null || true
    return
  fi

  echo "Status"
}

show_text() {
  local title="$1"
  if command -v zenity >/dev/null 2>&1; then
    zenity --text-info --title="$title" --width=980 --height=720 --filename=/dev/stdin 2>/dev/null || true
    return
  fi
  if command -v kdialog >/dev/null 2>&1; then
    kdialog --textbox /dev/stdin 920 680 2>/dev/null || true
    return
  fi
  cat
}

chimera_cli() {
  if [[ -x "$ROOT_DIR/bin/chimera-cli" ]]; then
    "$ROOT_DIR/bin/chimera-cli" "$@"
    return
  fi
  echo "error: chimera-cli binary is missing" >&2
  return 1
}

collect_node_rows() {
  awk '
    /^[[:space:]]+[[:alnum:]_:-]+[[:space:]]+[[:alnum:]_.-]+[[:space:]]+/ {
      id=$2
      status=$3
      printf "%s|%s\n", id, status
    }'
}

run_nodes_terminal() {
  local cmd='cd "'"$ROOT_DIR"'" && if [ -x ./bin/chimera-cli ]; then ./bin/chimera-cli --lang ru nodes; else echo "error: chimera-cli binary is missing" >&2; fi; echo; echo "Для выхода из терминала: exit"; echo; exec bash -i'
  if command -v x-terminal-emulator >/dev/null 2>&1; then
    x-terminal-emulator -e bash -lc "$cmd" >/dev/null 2>&1 &
    return 0
  fi
  if command -v gnome-terminal >/dev/null 2>&1; then
    gnome-terminal -- bash -lc "$cmd" >/dev/null 2>&1 &
    return 0
  fi
  if command -v konsole >/dev/null 2>&1; then
    konsole -e bash -lc "$cmd" >/dev/null 2>&1 &
    return 0
  fi
  if command -v xfce4-terminal >/dev/null 2>&1; then
    xfce4-terminal -e "bash -lc \"$cmd\"" >/dev/null 2>&1 &
    return 0
  fi
  if command -v xterm >/dev/null 2>&1; then
    xterm -e bash -lc "$cmd" >/dev/null 2>&1 &
    return 0
  fi
  chimera_cli --lang ru nodes | show_text "CHIMERA Nodes"
}

start_tray() {
  if pgrep -f "chimera-control-tray.sh" >/dev/null 2>&1; then
    return 0
  fi
  nohup "$TRAY" >/dev/null 2>&1 &
}

action="$(choose_action)"
echo "action=$action" >>"$ENTRY_LOG"

case "$action" in
  "Start CHIMERA"|start)
    nohup bash -lc "CHIMERA_SYSTEM_INTEGRATION=1 SPLIT_TRANSPARENT_ENABLED=1 CHIMERA_AUTOFIX_MAX_TIME=20 CHIMERA_AUTOFIX_CURL_MAX_TIME=3 '$CONTROL' start" >>"$ENTRY_LOG" 2>&1 &
    if command -v notify-send >/dev/null 2>&1; then
      notify-send "CHIMERA Control" "CHIMERA запущена с системной интеграцией (split по умолчанию)." || true
    fi
    "$CONTROL" proxy-status 2>&1 | show_text "CHIMERA Start (Background)"
    ;;
  "Stop CHIMERA"|stop)
    "$CONTROL" stop 2>&1 | show_text "CHIMERA Stop"
    ;;
  "Restart CHIMERA"|restart)
    "$CONTROL" restart 2>&1 | show_text "CHIMERA Restart"
    ;;
  "Status"|status)
    "$CONTROL" status 2>&1 | show_text "CHIMERA Status"
    ;;
  "Doctor"|doctor)
    "$CONTROL" doctor 2>&1 | show_text "CHIMERA Doctor"
    ;;
  "Logs"|logs)
    "$CONTROL" logs 2>&1 | show_text "CHIMERA Logs"
    ;;
  "Nodes"|nodes)
    run_nodes_terminal
    ;;
  "Start Tray Menu"|tray)
    start_tray
    if command -v notify-send >/dev/null 2>&1; then
      notify-send "CHIMERA Control" "Tray menu started (if supported by your desktop)." || true
    fi
    ;;
  "Quit"|quit|"")
    exit 0
    ;;
  *)
    "$CONTROL" status 2>&1 | show_text "CHIMERA Status"
    ;;
esac
