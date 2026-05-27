#!/usr/bin/env bash
set -euo pipefail

GAP_MAP_PATH="${1:-docs/CEF_GAP_MAP_2026-05-18.md}"

if [[ ! -f "$GAP_MAP_PATH" ]]; then
  echo "cef gap map guard: missing file: $GAP_MAP_PATH" >&2
  exit 1
fi

rg -q '^# CHIMERA Full CEF Gap Map \(2026-05-18\)$' "$GAP_MAP_PATH"
rg -q '^## Scope$' "$GAP_MAP_PATH"
rg -q '^## Truth Boundary$' "$GAP_MAP_PATH"
rg -q '^## Gap Matrix \(High-Level\)$' "$GAP_MAP_PATH"
rg -q '^## What Is Already Strong \(Use as Base\)$' "$GAP_MAP_PATH"
rg -q '^## Execution Recommendation$' "$GAP_MAP_PATH"

for n in 1 2 3 4 5 6 7; do
  rg -q "^$n\\. " "$GAP_MAP_PATH"
done

# Enforce strict 1..7 numbering set without duplicates/gaps.
number_set="$(rg -o '^[0-9]+\.' "$GAP_MAP_PATH" | tr -d '.' | sort -n | uniq | tr '\n' ' ' | sed 's/ $//')"
test "$number_set" = "1 2 3 4 5 6 7"

test "$(rg -c '^1\. Full cooperative mesh runtime$' "$GAP_MAP_PATH")" -eq 1
test "$(rg -c '^2\. DHT discovery \(public/private\) and provider records$' "$GAP_MAP_PATH")" -eq 1
test "$(rg -c '^3\. Distributed Policy Store \(DPS\)$' "$GAP_MAP_PATH")" -eq 1
test "$(rg -c '^4\. Cooperative relay participation/consent model$' "$GAP_MAP_PATH")" -eq 1
test "$(rg -c '^5\. Emergency/OOB carriers$' "$GAP_MAP_PATH")" -eq 1
test "$(rg -c '^6\. Roaming cache / distributed bootstrap continuation$' "$GAP_MAP_PATH")" -eq 1
test "$(rg -c '^7\. Reputation / complaint / relay credit subsystems$' "$GAP_MAP_PATH")" -eq 1

# Each gap block must preserve actionable structure.
count_current_fact="$(rg -c '^- Current fact:' "$GAP_MAP_PATH")"
count_status="$(rg -c '^- Status:' "$GAP_MAP_PATH")"
count_next_step="$(rg -c '^- Next step:' "$GAP_MAP_PATH")"
count_pdf_evidence="$(rg -c '^- `CHIMERA.pdf` evidence:' "$GAP_MAP_PATH")"
test "$count_current_fact" -ge 7
test "$count_status" -ge 7
test "$count_next_step" -ge 7
test "$count_pdf_evidence" -ge 7

# For each numbered block, required lines must be locally present.
for n in 1 2 3 4 5 6 7; do
  start_line="$(rg -n "^$n\\. " "$GAP_MAP_PATH" | head -n1 | cut -d: -f1)"
  test -n "$start_line"
  if [[ "$n" -lt 7 ]]; then
    next_line="$(rg -n "^$((n + 1))\\. " "$GAP_MAP_PATH" | head -n1 | cut -d: -f1)"
    end_line="$((next_line - 1))"
  else
    end_line="$(wc -l < "$GAP_MAP_PATH")"
  fi
  sed -n "${start_line},${end_line}p" "$GAP_MAP_PATH" | rg -q '^- `CHIMERA.pdf` evidence:'
  sed -n "${start_line},${end_line}p" "$GAP_MAP_PATH" | rg -q '^- Current fact:'
  sed -n "${start_line},${end_line}p" "$GAP_MAP_PATH" | rg -q '^- Status:'
  sed -n "${start_line},${end_line}p" "$GAP_MAP_PATH" | rg -q '^- Next step:'

  evidence_line="$(sed -n "${start_line},${end_line}p" "$GAP_MAP_PATH" | rg -n '^- `CHIMERA.pdf` evidence:' | head -n1 | cut -d: -f1)"
  current_line="$(sed -n "${start_line},${end_line}p" "$GAP_MAP_PATH" | rg -n '^- Current fact:' | head -n1 | cut -d: -f1)"
  status_line="$(sed -n "${start_line},${end_line}p" "$GAP_MAP_PATH" | rg -n '^- Status:' | head -n1 | cut -d: -f1)"
  next_step_line="$(sed -n "${start_line},${end_line}p" "$GAP_MAP_PATH" | rg -n '^- Next step:' | head -n1 | cut -d: -f1)"
  test -n "$evidence_line" && test -n "$current_line" && test -n "$status_line" && test -n "$next_step_line"
  test "$evidence_line" -lt "$current_line"
  test "$current_line" -lt "$status_line"
  test "$status_line" -lt "$next_step_line"
done

rg -q 'Full CEF contour from `CHIMERA.pdf`: PARTIAL / NOT CLOSED\.' "$GAP_MAP_PATH"
rg -q 'MVP/Lab contour: PASS' "$GAP_MAP_PATH"

echo "cef gap map guard: PASS"
