#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONTROL="$ROOT_DIR/scripts/chimera-control.sh"

if ! command -v yad >/dev/null 2>&1; then
  echo "yad is not installed. Running text control instead."
  exec "$CONTROL" status
fi

export ROOT_DIR CONTROL

yad --notification \
  --image=network-vpn \
  --text="CHIMERA Control" \
  --command="bash -lc '
    choice=\$(yad --list --title=\"CHIMERA Control\" --column=\"Action\" \
      \"Start CHIMERA\" \"Stop CHIMERA\" \"Restart CHIMERA\" \"Status\" \"Doctor\" \"Logs\" \
      \"Mode: Full\" \"Mode: Split\" \"Mode: Off\" \
      \"Split List: Allow\" \"Split List: Deny\" \
      \"Sites: List\" \"Quit\" \
      --height=260 --width=300 --separator=\"\" --single-click --no-headers 2>/dev/null || true);
    case \"\$choice\" in
      \"Start CHIMERA\") \"\$CONTROL\" start ;;
      \"Stop CHIMERA\") \"\$CONTROL\" stop ;;
      \"Restart CHIMERA\") \"\$CONTROL\" restart ;;
      \"Status\") \"\$CONTROL\" status | yad --text-info --title=\"CHIMERA Status\" --width=900 --height=600 ;;
      \"Doctor\") cd \"\$ROOT_DIR\" && cargo run -p chimera-cli -- doctor --config configs/client.example.conf --json --out docs/doctor_latest.json ; yad --info --text=\"Doctor completed: docs/doctor_latest.json\" ;;
      \"Logs\") \"\$CONTROL\" logs | yad --text-info --title=\"CHIMERA Logs\" --width=900 --height=600 ;;
      \"Mode: Full\") \"\$CONTROL\" route-mode full | yad --text-info --title=\"CHIMERA Mode\" --width=700 --height=300 ;;
      \"Mode: Split\") \"\$CONTROL\" route-mode split | yad --text-info --title=\"CHIMERA Mode\" --width=700 --height=300 ;;
      \"Mode: Off\") \"\$CONTROL\" route-mode off | yad --text-info --title=\"CHIMERA Mode\" --width=700 --height=300 ;;
      \"Split List: Allow\") \"\$CONTROL\" split-list-mode allow | yad --text-info --title=\"CHIMERA Split List Mode\" --width=700 --height=300 ;;
      \"Split List: Deny\") \"\$CONTROL\" split-list-mode deny | yad --text-info --title=\"CHIMERA Split List Mode\" --width=700 --height=300 ;;
      \"Sites: List\") \"\$CONTROL\" site-list | yad --text-info --title=\"CHIMERA Sites\" --width=700 --height=400 ;;
      *) exit 0 ;;
    esac
  '"
