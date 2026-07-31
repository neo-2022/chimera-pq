#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

fail() {
  echo "public_artifact_redaction_guard=fail reason=$1" >&2
  exit 1
}

REQUIRED_FILES=(
  docs/RUNTIME_REAL_WORLD_PROBE_SMOKE.json
  docs/probe_access_latest.json
  docs/SHIP_READINESS_REPORT.json
  docs/SHIP_READINESS_REPORT.md
  docs/RELEASE_READINESS_REPORT.json
  docs/RELEASE_READINESS_REPORT.md
  docs/RELEASE_READINESS_REPORT_RU.md
  docs/REPORT_PACK.json
  docs/REPORT_PACK.md
  docs/REALITY_AUDIT_LATEST.json
  docs/SECOND_MACHINE_REPORT.md
)

OPTIONAL_FILES=(
  docs/CHIMERA_PATH_PROOF.json
)

HISTORY_PATTERNS=(
  'docs/*PROOF*'
  'docs/MESH_SESSION_HANDOFF_*.md'
  'docs/mesh_evidence_*/*.json'
  'docs/side_b_sync/*/*.json'
  'configs/runtime_real_world_probe.env'
  'scripts/restore_laptop_ssh.sh'
)

STRICT_REDACTION_RE='https?://|socks[[:alnum:]+.-]*://|"remote_ip":"|"target":"[^"]|"url":"https?://|"direct_url":"https?://|domain_exact=[^"[:space:]]*\.[^"[:space:]]+|/home/|/root/|/Users/|/tmp/chimera|^Workspace under test: (?!<redacted>)|^Host kernel: (?!<redacted>)|BEGIN (RSA |OPENSSH |EC |DSA |PRIVATE )?PRIVATE KEY|OPENSSH PRIVATE KEY|Bearer[[:space:]]+[A-Za-z0-9._~+/=-]+|[A-Za-z0-9._%+-]+@([A-Za-z0-9-]+\.)+[A-Za-z]{2,}|([A-Za-z0-9-]+\.)+[A-Za-z]{2,}:[0-9]{2,5}|([0-9]{1,3}\.){3}[0-9]{1,3}|([0-9A-Fa-f]{1,4}:){3,}[0-9A-Fa-f]{0,4}|(?=[0-9A-Fa-f:]*[0-9])[0-9A-Fa-f:]*::[0-9A-Fa-f:]*|(token|password|passwd|auth|secret)[=:][^[:space:]]|authorization:|payload[=:][^[:space:]]|body[=:][^[:space:]]|hexdump'
HISTORY_REDACTION_RE='(ssh|scp)[[:space:]]+[^[:space:]]+@|ssh-(rsa|ed25519)[[:space:]]+[A-Za-z0-9+/=]{40,}|BEGIN (RSA |OPENSSH |EC |DSA |PRIVATE )?PRIVATE KEY|OPENSSH PRIVATE KEY|Bearer[[:space:]]+[A-Za-z0-9._~+/=-]+|[A-Za-z0-9._%+-]+@([0-9]{1,3}\.){3}[0-9]{1,3}|([0-9]{1,3}\.){3}[0-9]{1,3}|([0-9A-Fa-f]{1,4}:){3,}[0-9A-Fa-f]{0,4}|(?=[0-9A-Fa-f:]*[0-9])[0-9A-Fa-f:]*::[0-9A-Fa-f:]*|/home/|/root/|/Users/|/tmp/chimera|^Workspace under test: (?!<redacted>)|^Host kernel: (?!<redacted>)|(token|password|passwd|auth|secret)[=:][^[:space:]]|authorization:|payload[=:][^[:space:]]|body[=:][^[:space:]]|hexdump|"remote_ip":"(?!<redacted|")|"url":"https?://|"direct_url":"https?://'

add_existing_git_files() {
  if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git ls-files "$@" 2>/dev/null || true
  fi
}

if [[ "$#" -gt 0 ]]; then
  CANDIDATES=("$@")
else
  CANDIDATES=("${REQUIRED_FILES[@]}" "${OPTIONAL_FILES[@]}")
  while IFS= read -r path; do
    [[ -n "$path" ]] && CANDIDATES+=("$path")
  done < <(add_existing_git_files "${HISTORY_PATTERNS[@]}")
  if [[ -n "${CHIMERA_PUBLIC_ARTIFACT_REDACTION_FILES:-}" ]]; then
    read -r -a EXTRA_CANDIDATES <<<"$CHIMERA_PUBLIC_ARTIFACT_REDACTION_FILES"
    CANDIDATES+=("${EXTRA_CANDIDATES[@]}")
  fi
fi

existing=()
for path in "${CANDIDATES[@]}"; do
  if [[ -f "$path" ]]; then
    existing+=("$path")
    continue
  fi
  if [[ "$#" -gt 0 ]]; then
    fail "missing_artifact:${path}"
  fi
  for required in "${REQUIRED_FILES[@]}"; do
    if [[ "$path" == "$required" ]]; then
      fail "missing_required_artifact:${path}"
    fi
  done
  continue
done

if [[ "${#existing[@]}" -eq 0 ]]; then
  fail "no_artifacts_to_scan"
fi

strict_existing=()
for artifact in "${existing[@]}"; do
  case " ${REQUIRED_FILES[*]} ${OPTIONAL_FILES[*]} " in
    *" ${artifact} "*) strict_existing+=("$artifact") ;;
  esac
done

for artifact in "${existing[@]}"; do
  pattern="$HISTORY_REDACTION_RE"
  for strict in "${strict_existing[@]}"; do
    if [[ "$artifact" == "$strict" ]]; then
      pattern="$STRICT_REDACTION_RE"
      break
    fi
  done
  if rg -P -q "$pattern" "$artifact"; then
    fail "unredacted_public_artifact:${artifact}"
  fi
done

echo "public_artifact_redaction_guard=pass files_scanned=${#existing[@]}"
