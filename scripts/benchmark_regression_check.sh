#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

max_attempts=2
attempt=1
baseline_file="docs/benchmark_baseline.json"
if [[ ! -f "$baseline_file" ]]; then
  baseline_file="docs/benchmark_latest.json"
fi

report_file="docs/BENCHMARK_REGRESSION_GATE.json"

while [[ "$attempt" -le "$max_attempts" ]]; do
  tmp_out="$(mktemp)"
  if cargo run -p chimera-lab --bin chimera-lab -- benchmark-report --baseline "$baseline_file" --max-regression-pct 20 --out "$tmp_out"; then
    mv "$tmp_out" docs/benchmark_latest.json
    cat > "$report_file" <<JSON
{"status":"ok","kind":"benchmark_regression_gate","message_en":"Benchmark regression gate passed.","message_ru":"Гейт регрессии производительности пройден.","attempt":${attempt},"max_attempts":${max_attempts},"baseline_file":"${baseline_file}","output_file":"docs/benchmark_latest.json"}
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
{"status":"fail","kind":"benchmark_regression_gate","message_en":"Benchmark regression gate failed after retries.","message_ru":"Гейт регрессии производительности не пройден после повторов.","attempt":${max_attempts},"max_attempts":${max_attempts},"baseline_file":"${baseline_file}","output_file":"docs/benchmark_latest.json"}
JSON

echo "benchmark-regression-check: failed after ${max_attempts} attempts" >&2
exit 1
