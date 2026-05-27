#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONTROL="$ROOT_DIR/scripts/chimera-control.sh"
TRAY="$ROOT_DIR/scripts/chimera-control-tray.sh"
UI_MODE_FILE="${UI_MODE_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/chimera/ui_mode}"
LOG_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/chimera"
LOG_FILE="$LOG_DIR/launcher.log"

mkdir -p "$LOG_DIR"
exec >>"$LOG_FILE" 2>&1
echo "--- $(date '+%F %T') launcher start session=${XDG_SESSION_TYPE:-unknown} display=${DISPLAY:-none} wayland=${WAYLAND_DISPLAY:-none} ---"

has_gui=false
if [[ -n "${DISPLAY:-}" || -n "${WAYLAND_DISPLAY:-}" ]]; then
  has_gui=true
fi

if [[ "${CHIMERA_FORCE_CLI:-0}" == "1" ]]; then
  echo "forced cli mode"
  exec "$CONTROL" status
fi

choose_action_gui() {
  if command -v zenity >/dev/null 2>&1; then
    zenity --list \
      --title="CHIMERA Control" \
      --text="Select action" \
      --column="Action" \
      "Start CHIMERA" "Stop CHIMERA" "Restart CHIMERA" "Status" "Doctor" "Logs" \
      "Mode: Full" "Mode: Split" "Mode: Off" \
      "Split List: Allow" "Split List: Deny" \
      "Sites: List" "Sites: Add..." "Sites: Remove..." \
      "Apps: Running" "Services: Running" \
      "Quit" \
      --height=360 --width=360 2>/dev/null || true
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
      mode_full "Mode: Full" \
      mode_split "Mode: Split" \
      mode_off "Mode: Off" \
      split_allow "Split List: Allow" \
      split_deny "Split List: Deny" \
      site_list "Sites: List" \
      site_add "Sites: Add..." \
      site_remove "Sites: Remove..." \
      apps_running "Apps: Running" \
      services_running "Services: Running" \
      quit "Quit" 2>/dev/null || true
    return
  fi

  if command -v yad >/dev/null 2>&1; then
    yad --list --title="CHIMERA Control" --column="Action" \
      "Start CHIMERA" "Stop CHIMERA" "Restart CHIMERA" "Status" "Doctor" "Logs" \
      "Mode: Full" "Mode: Split" "Mode: Off" \
      "Split List: Allow" "Split List: Deny" \
      "Sites: List" "Sites: Add..." "Sites: Remove..." \
      "Apps: Running" "Services: Running" \
      "Quit" \
      --height=320 --width=320 --separator="" --no-headers 2>/dev/null || true
    return
  fi

  echo ""
}

notify_issue() {
  local text="$1"
  if command -v notify-send >/dev/null 2>&1; then
    notify-send "CHIMERA Control" "$text" || true
  fi
}

open_terminal_status() {
  local cmd="$CONTROL status; echo; echo 'Press Enter to close...'; read -r _"
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
  return 1
}

show_text() {
  local title="$1"
  if command -v zenity >/dev/null 2>&1; then
    zenity --text-info --title="$title" --width=960 --height=680 --filename=/dev/stdin 2>/dev/null
    return
  fi
  if command -v kdialog >/dev/null 2>&1; then
    kdialog --textbox /dev/stdin 900 650 2>/dev/null
    return
  fi
  if command -v yad >/dev/null 2>&1; then
    yad --text-info --title="$title" --width=960 --height=680 2>/dev/null
    return
  fi
  cat
}

run_action() {
  local action="$1"
  case "$action" in
    "Start CHIMERA"|start) "$CONTROL" start ;;
    "Stop CHIMERA"|stop) "$CONTROL" stop ;;
    "Restart CHIMERA"|restart) "$CONTROL" restart ;;
    "Status"|status) "$CONTROL" status | show_text "CHIMERA Status" ;;
    "Doctor"|doctor) "$CONTROL" doctor | show_text "CHIMERA Doctor" ;;
    "Logs"|logs) "$CONTROL" logs | show_text "CHIMERA Logs" ;;
    "Mode: Full"|mode_full) "$CONTROL" route-mode full | show_text "CHIMERA Mode" ;;
    "Mode: Split"|mode_split) "$CONTROL" route-mode split | show_text "CHIMERA Mode" ;;
    "Mode: Off"|mode_off) "$CONTROL" route-mode off | show_text "CHIMERA Mode" ;;
    "Split List: Allow"|split_allow) "$CONTROL" split-list-mode allow | show_text "CHIMERA Split List Mode" ;;
    "Split List: Deny"|split_deny) "$CONTROL" split-list-mode deny | show_text "CHIMERA Split List Mode" ;;
    "Sites: List"|site_list) "$CONTROL" site-list | show_text "CHIMERA Sites" ;;
    "Apps: Running"|apps_running) "$CONTROL" apps-running | show_text "Running Apps" ;;
    "Services: Running"|services_running) "$CONTROL" services-running | show_text "Running Services" ;;
    "Sites: Add..."|site_add)
      if command -v zenity >/dev/null 2>&1; then
        domain="$(zenity --entry --title='Add Site' --text='Domain:' 2>/dev/null || true)"
      elif command -v kdialog >/dev/null 2>&1; then
        domain="$(kdialog --inputbox 'Domain:' 2>/dev/null || true)"
      elif command -v yad >/dev/null 2>&1; then
        domain="$(yad --entry --title='Add Site' --text='Domain:' 2>/dev/null || true)"
      else
        domain=""
      fi
      if [[ -n "${domain:-}" ]]; then
        "$CONTROL" site-add "$domain" | show_text "CHIMERA Sites"
      fi
      ;;
    "Sites: Remove..."|site_remove)
      if command -v zenity >/dev/null 2>&1; then
        domain="$(zenity --entry --title='Remove Site' --text='Domain:' 2>/dev/null || true)"
      elif command -v kdialog >/dev/null 2>&1; then
        domain="$(kdialog --inputbox 'Domain:' 2>/dev/null || true)"
      elif command -v yad >/dev/null 2>&1; then
        domain="$(yad --entry --title='Remove Site' --text='Domain:' 2>/dev/null || true)"
      else
        domain=""
      fi
      if [[ -n "${domain:-}" ]]; then
        "$CONTROL" site-remove "$domain" | show_text "CHIMERA Sites"
      fi
      ;;
    "Quit"|quit|"") exit 0 ;;
    *) exit 0 ;;
  esac
}

if [[ "$has_gui" == true ]]; then
  mode="auto"
  if [[ -n "${CHIMERA_UI_MODE:-}" ]]; then
    mode="${CHIMERA_UI_MODE}"
  fi
  if [[ -f "$UI_MODE_FILE" ]]; then
    mode="$(tr -d '[:space:]' < "$UI_MODE_FILE")"
  fi

  # Auto mode: select best backend by environment.
  if [[ "$mode" == "auto" ]]; then
    echo "mode=auto"
    nohup bash -lc "CHIMERA_SYSTEM_INTEGRATION=1 SPLIT_TRANSPARENT_ENABLED=1 CHIMERA_AUTOFIX_MAX_TIME=20 CHIMERA_AUTOFIX_CURL_MAX_TIME=3 '$CONTROL' start" >>"$LOG_FILE" 2>&1 &
    if command -v zenity >/dev/null 2>&1; then
      "$CONTROL" status | zenity --text-info --title="CHIMERA Started" --width=960 --height=680 --filename=/dev/stdin 2>/dev/null || true
      exit 0
    fi
    if command -v kdialog >/dev/null 2>&1; then
      "$CONTROL" status | kdialog --textbox /dev/stdin 900 650 2>/dev/null || true
      exit 0
    fi
    if command -v yad >/dev/null 2>&1; then
      "$CONTROL" status | yad --text-info --title="CHIMERA Started" --width=960 --height=680 2>/dev/null || true
      exit 0
    fi
    notify_issue "CHIMERA started. No GUI backend found, opening terminal status."
    open_terminal_status || "$CONTROL" status
    exit 0
  fi

  if [[ "$mode" == "tray" ]]; then
    echo "mode=tray"
    if command -v yad >/dev/null 2>&1; then
      exec "$TRAY"
    fi
    action="$(choose_action_gui)"
    if [[ -z "${action:-}" ]]; then
      notify_issue "Tray backend unavailable. Opening terminal fallback."
      "$CONTROL" start || true
      open_terminal_status || "$CONTROL" status
      exit 0
    fi
    run_action "$action"
    exit 0
  fi

  if [[ "$mode" == "dialog" ]]; then
    echo "mode=dialog"
    action="$(choose_action_gui)"
    if [[ -z "${action:-}" ]]; then
      notify_issue "Dialog backend unavailable. Opening terminal fallback."
      "$CONTROL" start || true
      open_terminal_status || "$CONTROL" status
      exit 0
    fi
    run_action "$action"
    exit 0
  fi

  if [[ "$mode" == "cli" ]]; then
    echo "mode=cli"
    "$CONTROL" status
    exit 0
  fi

  # Unknown mode value => safe auto fallback.
  echo "mode=unknown($mode), fallback to auto behavior"
  action="$(choose_action_gui)"
  if [[ -z "${action:-}" ]]; then
    notify_issue "No GUI backend. Opening terminal fallback."
    "$CONTROL" start || true
    open_terminal_status || "$CONTROL" status
    exit 0
  fi
  run_action "$action"
  exit 0
fi

# Headless fallback
"$CONTROL" status
