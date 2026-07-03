#!/usr/bin/env bash
set -euo pipefail

# This fixture must fail product_language_guard: normal app traffic must use
# CHIMERA's own transparent mesh datapath, not a third-party runtime bootstrap.
normal_app_path_uses_third_party_runtime=1
RUNTIME_BOOTSTRAP_SCRIPT="${RUNTIME_BOOTSTRAP_SCRIPT:-scripts/chimera_runtime_bootstrap.sh}"
CHIMERA_SINGBOX_URL="https://example.invalid/sing-box.tar.gz"
"$RUNTIME_BOOTSTRAP_SCRIPT" ensure-singbox
