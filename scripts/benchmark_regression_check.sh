#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

max_attempts=2
attempt=1
max_regression_pct="${CHIMERA_BENCHMARK_MAX_REGRESSION_PCT:-20}"
baseline_profile="${CHIMERA_BENCHMARK_BASELINE_PROFILE:-local}"

if [[ -n "${CHIMERA_BENCHMARK_BASELINE_FILE:-}" ]]; then
  baseline_file="$CHIMERA_BENCHMARK_BASELINE_FILE"
  baseline_profile="${CHIMERA_BENCHMARK_BASELINE_PROFILE:-custom}"
elif [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
  baseline_file="docs/benchmark_ci_baseline.json"
  baseline_profile="github_actions"
else
  baseline_file="docs/benchmark_baseline.json"
fi

if [[ ! "$baseline_file" =~ ^docs/benchmark[_A-Za-z0-9.-]*[.]json$ ]]; then
  echo "benchmark-regression-check: invalid benchmark baseline path: $baseline_file" >&2
  exit 2
fi
if [[ ! -f "$baseline_file" ]]; then
  echo "benchmark-regression-check: missing baseline file: $baseline_file" >&2
  exit 1
fi
if [[ ! "$max_regression_pct" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
  echo "benchmark-regression-check: invalid CHIMERA_BENCHMARK_MAX_REGRESSION_PCT: $max_regression_pct" >&2
  exit 2
fi
if ! awk -v value="$max_regression_pct" 'BEGIN { exit(value <= 20 ? 0 : 1) }'; then
  echo "benchmark-regression-check: CHIMERA_BENCHMARK_MAX_REGRESSION_PCT exceeds release gate maximum 20: $max_regression_pct" >&2
  exit 2
fi
if [[ ! "$baseline_profile" =~ ^[A-Za-z0-9_.-]+$ ]]; then
  echo "benchmark-regression-check: invalid CHIMERA_BENCHMARK_BASELINE_PROFILE: $baseline_profile" >&2
  exit 2
fi

report_file="docs/BENCHMARK_REGRESSION_GATE.json"

while [[ "$attempt" -le "$max_attempts" ]]; do
  tmp_out="$(mktemp)"
  if cargo run -p chimera-lab --bin chimera-lab -- benchmark-report --baseline "$baseline_file" --max-regression-pct "$max_regression_pct" --out "$tmp_out"; then
    mv "$tmp_out" docs/benchmark_latest.json
    cat > "$report_file" <<JSON
{"status":"ok","kind":"benchmark_regression_gate","message_en":"Benchmark regression gate passed.","message_ru":"Гейт регрессии производительности пройден.","attempt":${attempt},"max_attempts":${max_attempts},"max_regression_pct":${max_regression_pct},"baseline_profile":"${baseline_profile}","baseline_file":"${baseline_file}","output_file":"docs/benchmark_latest.json"}
JSON
    exit 0
  fi
  rm -f "$tmp_out"

  if [[ "$attempt" -lt "$max_attempts" ]]; then
    echo "benchmark-regression-check: transient fail on attempt ${attempt}, retrying once..." >&2
    sleep 1
  fi

  attempt=$((attempt + 1))
done

cat > "$report_file" <<JSON
{"status":"fail","kind":"benchmark_regression_gate","message_en":"Benchmark regression gate failed after retries.","message_ru":"Гейт регрессии производительности не пройден после повторов.","attempt":${max_attempts},"max_attempts":${max_attempts},"max_regression_pct":${max_regression_pct},"baseline_profile":"${baseline_profile}","baseline_file":"${baseline_file}","output_file":"docs/benchmark_latest.json"}
JSON

echo "benchmark-regression-check: failed after ${max_attempts} attempts" >&2
exit 1
