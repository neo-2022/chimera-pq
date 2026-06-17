#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

tmp_dir="$(mktemp -d)"
restore() {
  if [[ -f "$tmp_dir/BENCHMARK_REGRESSION_GATE.json" ]]; then
    cp "$tmp_dir/BENCHMARK_REGRESSION_GATE.json" docs/BENCHMARK_REGRESSION_GATE.json
  fi
  if [[ -f "$tmp_dir/benchmark_latest.json" ]]; then
    cp "$tmp_dir/benchmark_latest.json" docs/benchmark_latest.json
  fi
  if [[ -f "$tmp_dir/benchmark_baseline.original.json" ]]; then
    cp "$tmp_dir/benchmark_baseline.original.json" docs/benchmark_baseline.json
  fi
  if [[ -f "$tmp_dir/benchmark_ci_baseline.json" ]]; then
    cp "$tmp_dir/benchmark_ci_baseline.json" docs/benchmark_ci_baseline.json
  fi
  rm -rf "$tmp_dir"
}
trap restore EXIT

cp docs/BENCHMARK_REGRESSION_GATE.json "$tmp_dir/BENCHMARK_REGRESSION_GATE.json"
cp docs/benchmark_latest.json "$tmp_dir/benchmark_latest.json"
cp docs/benchmark_baseline.json "$tmp_dir/benchmark_baseline.original.json"
cp docs/benchmark_ci_baseline.json "$tmp_dir/benchmark_ci_baseline.json"

# This smoke verifies selection contracts, not local hardware performance.
# Use CI-speed baseline content for the local-profile branch while keeping the
# path as docs/benchmark_baseline.json, so GitHub runners do not compare
# themselves against a faster developer workstation baseline.
cp docs/benchmark_ci_baseline.json docs/benchmark_baseline.json
env -u GITHUB_ACTIONS bash scripts/benchmark_regression_check.sh >/dev/null
jq -e '.status == "ok" and .baseline_profile == "local" and .baseline_file == "docs/benchmark_baseline.json" and .max_regression_pct == 20' docs/BENCHMARK_REGRESSION_GATE.json >/dev/null
cp "$tmp_dir/benchmark_baseline.original.json" docs/benchmark_baseline.json

GITHUB_ACTIONS=true bash scripts/benchmark_regression_check.sh >/dev/null
jq -e '.status == "ok" and .baseline_profile == "github_actions" and .baseline_file == "docs/benchmark_ci_baseline.json" and .max_regression_pct == 20' docs/BENCHMARK_REGRESSION_GATE.json >/dev/null

CHIMERA_BENCHMARK_BASELINE_FILE=docs/benchmark_ci_baseline.json \
  CHIMERA_BENCHMARK_BASELINE_PROFILE=custom \
  bash scripts/benchmark_regression_check.sh >/dev/null
jq -e '.status == "ok" and .baseline_profile == "custom" and .baseline_file == "docs/benchmark_ci_baseline.json" and .max_regression_pct == 20' docs/BENCHMARK_REGRESSION_GATE.json >/dev/null

if CHIMERA_BENCHMARK_BASELINE_FILE=/tmp/benchmark.json bash scripts/benchmark_regression_check.sh >/dev/null 2>&1; then
  echo "benchmark profile contract smoke: invalid external baseline path was accepted" >&2
  exit 1
fi

if CHIMERA_BENCHMARK_MAX_REGRESSION_PCT=21 bash scripts/benchmark_regression_check.sh >/dev/null 2>&1; then
  echo "benchmark profile contract smoke: max regression threshold above 20 was accepted" >&2
  exit 1
fi

mv docs/benchmark_baseline.json "$tmp_dir/benchmark_baseline.missing.json"
if env -u GITHUB_ACTIONS bash scripts/benchmark_regression_check.sh >/dev/null 2>&1; then
  echo "benchmark profile contract smoke: missing local baseline was accepted" >&2
  exit 1
fi
mv "$tmp_dir/benchmark_baseline.missing.json" docs/benchmark_baseline.json

mv docs/benchmark_ci_baseline.json "$tmp_dir/benchmark_ci_baseline.json.missing"
if GITHUB_ACTIONS=true bash scripts/benchmark_regression_check.sh >/dev/null 2>&1; then
  echo "benchmark profile contract smoke: missing GitHub Actions baseline was accepted" >&2
  exit 1
fi
mv "$tmp_dir/benchmark_ci_baseline.json.missing" docs/benchmark_ci_baseline.json

echo "benchmark profile contract smoke: PASS"
