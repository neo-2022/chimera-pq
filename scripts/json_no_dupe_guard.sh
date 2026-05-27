#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -lt 1 ]]; then
  echo "usage: $0 <json-file> [<json-file> ...]" >&2
  exit 1
fi

cargo run -q -p chimera-lab --bin json_no_dupe_guard -- "$@"
