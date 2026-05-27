#!/usr/bin/env bash
set -euo pipefail

ENTRY="${HOME}/chimera-pq/scripts/chimera-control-desktop-entry.sh"
LOG_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/chimera"
CLICK_LOG="$LOG_DIR/desktop-click.log"

mkdir -p "$LOG_DIR"
{
  echo "--- $(date '+%F %T') desktop launcher clicked ---"
  echo "session=${XDG_SESSION_TYPE:-unknown} display=${DISPLAY:-none} wayland=${WAYLAND_DISPLAY:-none}"
  echo "entry=$ENTRY"
} >>"$CLICK_LOG"

if command -v notify-send >/dev/null 2>&1; then
  notify-send "CHIMERA Control" "Launch requested..." || true
fi

if [[ ! -x "$ENTRY" ]]; then
  echo "CHIMERA entry script is missing or not executable: $ENTRY" >&2
  echo "error: entry missing or not executable" >>"$CLICK_LOG"
  exit 1
fi

launch_term() {
  # shellcheck disable=SC2068
  "$@" >/dev/null 2>&1 &
  local pid=$!
  sleep 0.4
  if kill -0 "$pid" >/dev/null 2>&1; then
    echo "terminal launched pid=$pid: $*" >>"$CLICK_LOG"
    return 0
  fi
  echo "terminal failed quickly: $*" >>"$CLICK_LOG"
  return 1
}

if command -v /bin/kgx >/dev/null 2>&1; then
  launch_term /bin/kgx -- /bin/bash -lc "$ENTRY" && exit 0
fi

if command -v /bin/gnome-terminal >/dev/null 2>&1; then
  launch_term /bin/gnome-terminal -- /bin/bash -lc "$ENTRY" && exit 0
fi

if command -v x-terminal-emulator >/dev/null 2>&1; then
  launch_term x-terminal-emulator -e /bin/bash -lc "$ENTRY" && exit 0
fi

if command -v konsole >/dev/null 2>&1; then
  launch_term konsole -e /bin/bash -lc "$ENTRY" && exit 0
fi

if command -v xfce4-terminal >/dev/null 2>&1; then
  launch_term xfce4-terminal -e "/bin/bash -lc \"$ENTRY\"" && exit 0
fi

if command -v xterm >/dev/null 2>&1; then
  launch_term xterm -e /bin/bash -lc "$ENTRY" && exit 0
fi

# Last-resort fallback: run directly and notify.
echo "launch direct fallback" >>"$CLICK_LOG"
"$ENTRY" >>"$CLICK_LOG" 2>&1 || true
if command -v notify-send >/dev/null 2>&1; then
  notify-send "CHIMERA Control" "Started in fallback mode. See ~/.cache/chimera/desktop-click.log" || true
fi
