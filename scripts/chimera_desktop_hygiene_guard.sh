#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# Policy:
# CHIMERA must not create/overwrite desktop entries for third-party apps.
# Allowed desktop writes are CHIMERA-owned entries only.

fail=0

# Any explicit write/install/copy targeting Chromium desktop files is forbidden.
if rg -n --glob 'scripts/*.sh' \
  '^[[:space:]]*(install|cp|cat|tee)\b.*(chromium-browser\.desktop|chromium\.desktop|org\.chromium\.Chromium\.desktop)' \
  scripts >/dev/null 2>&1; then
  echo "desktop-hygiene guard: FAIL third-party Chromium desktop file write detected"
  rg -n --glob 'scripts/*.sh' \
    '^[[:space:]]*(install|cp|cat|tee)\b.*(chromium-browser\.desktop|chromium\.desktop|org\.chromium\.Chromium\.desktop)' \
    scripts || true
  fail=1
fi

# Any explicit write path for custom launcher script for chromium routing is forbidden.
if rg -n --glob 'scripts/*.sh' '^[[:space:]]*(install|cp|cat|tee)\b.*(chimera-chromium-launch\.sh)' scripts >/dev/null 2>&1; then
  echo "desktop-hygiene guard: FAIL legacy chromium launcher hook detected"
  rg -n --glob 'scripts/*.sh' '^[[:space:]]*(install|cp|cat|tee)\b.*(chimera-chromium-launch\.sh)' scripts || true
  fail=1
fi

if [[ "$fail" -ne 0 ]]; then
  exit 1
fi

echo "desktop-hygiene guard: PASS"
