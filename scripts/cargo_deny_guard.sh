#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source_root="${CHIMERA_ADVISORY_SOURCE_ROOT:-$HOME/.cargo/advisory-dbs}"
target_root="${CHIMERA_ADVISORY_TARGET_ROOT:-/tmp/chimera-advisory-dbs}"

if cargo deny check; then
  echo "cargo-deny guard: PASS (live advisory fetch)"
  exit 0
fi

echo "cargo-deny guard: live check failed, trying offline advisory mirror fallback" >&2

if [[ ! -d "$source_root" ]]; then
  echo "cargo-deny guard: FAIL (source root not found: $source_root)" >&2
  exit 1
fi

source_dir="$(find "$source_root" -maxdepth 1 -type d -name 'advisory-db-*' | head -n 1 || true)"
if [[ -z "$source_dir" ]]; then
  echo "cargo-deny guard: FAIL (no local advisory db mirror at $source_root)" >&2
  exit 1
fi

target_dir="$target_root/$(basename "$source_dir")"
mkdir -p "$target_root"
rm -rf "$target_dir"
cp -a "$source_dir" "$target_root/"

cargo deny check --disable-fetch
echo "cargo-deny guard: PASS (offline advisory mirror)"
