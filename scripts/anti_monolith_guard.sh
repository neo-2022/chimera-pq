#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# Hard limits: if any file grows beyond limit, guard fails.
# Format: <path>:<max_lines>
LIMITS=(
  "crates/chimera-mesh/src/runtime.rs:450"
  "crates/chimera-mesh/src/policy.rs:800"
  "crates/chimera-mesh/src/policy_parse.rs:220"
  "crates/chimera-mesh/src/model.rs:300"
  "crates/chimera-mesh/src/preemptive.rs:200"
)

FAILED=0

for entry in "${LIMITS[@]}"; do
  path="${entry%%:*}"
  max_lines="${entry##*:}"

  if [[ ! -f "$path" ]]; then
    echo "anti-monolith guard: missing file: $path" >&2
    FAILED=1
    continue
  fi

  lines="$(wc -l < "$path" | tr -d ' ')"
  if (( lines > max_lines )); then
    echo "anti-monolith guard: FAIL $path lines=$lines limit=$max_lines" >&2
    FAILED=1
  else
    echo "anti-monolith guard: OK   $path lines=$lines limit=$max_lines"
  fi
done

# Runtime domain files must stay split; block oversized runtime leaf modules.
RUNTIME_FILE_LIMIT=400
while IFS= read -r runtime_file; do
  lines="$(wc -l < "$runtime_file" | tr -d ' ')"
  if (( lines > RUNTIME_FILE_LIMIT )); then
    echo "anti-monolith guard: FAIL $runtime_file lines=$lines limit=$RUNTIME_FILE_LIMIT" >&2
    FAILED=1
  fi
done < <(find "crates/chimera-mesh/src/runtime" -maxdepth 1 -type f -name '*.rs' | sort)

# CLI mesh files must stay split by command/contract/parser/test responsibility.
CLI_MESH_FILE_LIMIT=400
if [[ -f "crates/chimera-cli/src/mesh_cli.rs" ]]; then
  echo "anti-monolith guard: FAIL root mesh_cli.rs detected; split CLI mesh by domain modules" >&2
  FAILED=1
else
  echo "anti-monolith guard: OK   no root mesh_cli.rs monolith file"
fi

while IFS= read -r cli_mesh_file; do
  lines="$(wc -l < "$cli_mesh_file" | tr -d ' ')"
  if (( lines > CLI_MESH_FILE_LIMIT )); then
    echo "anti-monolith guard: FAIL $cli_mesh_file lines=$lines limit=$CLI_MESH_FILE_LIMIT" >&2
    FAILED=1
  fi
done < <(find "crates/chimera-cli/src/mesh_cli" -maxdepth 1 -type f -name '*.rs' | sort)

# Mesh test leaf files must stay readable without becoming another runtime.
MESH_TEST_FILE_LIMIT=450
while IFS= read -r mesh_test_file; do
  lines="$(wc -l < "$mesh_test_file" | tr -d ' ')"
  if (( lines > MESH_TEST_FILE_LIMIT )); then
    echo "anti-monolith guard: FAIL $mesh_test_file lines=$lines limit=$MESH_TEST_FILE_LIMIT" >&2
    FAILED=1
  fi
done < <(find "crates/chimera-mesh/src" -mindepth 2 -type f -path '*/tests*/*.rs' | sort)

# No new test monoliths in src root. Tests must live in split test modules.
ROOT_TEST_MONOLITHS=()
while IFS= read -r file; do
  ROOT_TEST_MONOLITHS+=("$file")
done < <(find "crates/chimera-mesh/src" -maxdepth 1 -type f -name 'tests_*.rs' | sort)

if (( ${#ROOT_TEST_MONOLITHS[@]} > 0 )); then
  echo "anti-monolith guard: FAIL root test monolith files detected:" >&2
  for file in "${ROOT_TEST_MONOLITHS[@]}"; do
    echo "  - $file" >&2
  done
  FAILED=1
else
  echo "anti-monolith guard: OK   no root tests_*.rs monolith files"
fi

if [[ -f "crates/chimera-mesh/src/tests.rs" ]]; then
  echo "anti-monolith guard: FAIL root tests.rs detected; split tests by domain modules" >&2
  FAILED=1
else
  echo "anti-monolith guard: OK   no root tests.rs monolith file"
fi

if (( FAILED != 0 )); then
  echo "anti-monolith guard: blocked. Split by domain modules before merge." >&2
  exit 1
fi

echo "anti-monolith guard: PASS"
