#!/usr/bin/env bash
set -euo pipefail

artifacts=(
  "docs/MVP_VERIFY.json"
  "docs/RELEASE_READINESS_REPORT.json"
  "docs/benchmark_latest.json"
  "docs/benchmark_baseline.json"
  "docs/BENCHMARK_REGRESSION_GATE.json"
  "docs/MVP_SNAPSHOT.json"
  "docs/REPORT_PACK.json"
  "docs/ARTIFACT_AUDIT.json"
)

cp docs/benchmark_latest.json docs/benchmark_baseline.json

sha256sum "${artifacts[@]}" > docs/V1_MVP_BASELINE.sha256

today="$(date +%F)"
manifest_tmp="$(mktemp)"

{
  echo "{"
  echo "  \"kind\": \"v1_mvp_baseline_manifest\"," 
  echo "  \"created_at\": \"${today}\"," 
  echo "  \"scope\": \"CHIMERA-PQ MVP M5/M6 baseline artifacts\"," 
  echo "  \"network_state\": \"not_modified\"," 
  echo "  \"artifacts\": ["
  first=1
  while read -r sum path; do
    size="$(wc -c < "$path" | tr -d ' ')"
    if [[ $first -eq 0 ]]; then
      echo "    ,{"
    else
      echo "    {"
    fi
    echo "      \"path\": \"$path\"," 
    echo "      \"size_bytes\": $size," 
    echo "      \"sha256\": \"$sum\""
    echo "    }"
    first=0
  done < docs/V1_MVP_BASELINE.sha256
  echo "  ]"
  echo "}"
} > "$manifest_tmp"

mv "$manifest_tmp" docs/V1_MVP_BASELINE_MANIFEST.json
