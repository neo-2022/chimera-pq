#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "installer_gate=fail reason=$1" >&2
  exit 1
}

rg -n "installer_gate_prepare_upstream_env|transparent runtime|transparent runtime" \
  "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_missing_transparent_bootstrap"

rg -n "datapath-status|transparent_runtime|split-transparent" \
  "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_missing_transparent_runtime"
rg -n '^GATEWAY_LOG="\$\{GATEWAY_LOG:-\$\{XDG_CACHE_HOME:-\$HOME/\.cache\}/chimera/chimera_gateway\.service\.log\}"' \
  "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_gateway_log_not_user_cache"
rg -n '^CLIENT_LOG="\$\{CLIENT_LOG:-\$\{XDG_CACHE_HOME:-\$HOME/\.cache\}/chimera/chimera_client\.service\.log\}"' \
  "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_client_log_not_user_cache"
rg -n '^ensure_runtime_log_paths\(\) \{$' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_missing_runtime_log_preparation_helper"
rg -n '^  ensure_runtime_log_paths$' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_missing_runtime_log_preparation_call"
if rg -n '^Standard(Output|Error)=append:__CHIMERA_ROOT__' "$ROOT_DIR/deploy/systemd-user"/*.service >/dev/null; then
  fail "systemd_unit_logs_under_release_root"
fi
rg -n '^StandardOutput=append:%h/\.cache/chimera/chimera_gateway\.service\.log$' \
  "$ROOT_DIR/deploy/systemd-user/chimera-gateway.service" >/dev/null || fail "gateway_unit_stdout_not_user_cache"
rg -n '^StandardError=append:%h/\.cache/chimera/chimera_gateway\.service\.log$' \
  "$ROOT_DIR/deploy/systemd-user/chimera-gateway.service" >/dev/null || fail "gateway_unit_stderr_not_user_cache"
rg -n '^StandardOutput=append:%h/\.cache/chimera/chimera_client\.service\.log$' \
  "$ROOT_DIR/deploy/systemd-user/chimera-client.service" >/dev/null || fail "client_unit_stdout_not_user_cache"
rg -n '^StandardError=append:%h/\.cache/chimera/chimera_client\.service\.log$' \
  "$ROOT_DIR/deploy/systemd-user/chimera-client.service" >/dev/null || fail "client_unit_stderr_not_user_cache"
rg -n 'reason=node_service_failed' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_start_does_not_fail_failed_node"
rg -n 'reason=transparent_service_failed' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_start_does_not_fail_failed_transparent"
rg -n 'ensure_runtime_log_paths' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_missing_runtime_log_preparation"
rg -n 'CHIMERA_UPDATE_FIRST_CHECKED' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_missing_update_first_guard"
rg -n 'update_first_gate -start' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_start_can_bypass_update_first"
rg -n 'update_first_gate -mesh' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_mesh_can_bypass_update_first"

rg -n '^VERSION="' "$ROOT_DIR/scripts/chimera.sh" >/dev/null || fail "bootstrap_missing_version_metadata"
rg -n '^ARCHIVE_URL_DEFAULT="https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera-pq-release\.tar\.gz"' \
  "$ROOT_DIR/scripts/chimera.sh" >/dev/null || fail "bootstrap_missing_latest_release_archive_url"
rg -n '^CHECKSUM_URL_DEFAULT="https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera-pq-release\.tar\.gz\.sha256"' \
  "$ROOT_DIR/scripts/chimera.sh" >/dev/null || fail "bootstrap_missing_latest_release_checksum_url"
rg -n 'verify_archive_checksum "\$archive" "\$checksum"' "$ROOT_DIR/scripts/chimera.sh" >/dev/null || fail "bootstrap_missing_archive_checksum_verify"
rg -n 'CHIMERA_RELEASE_BUNDLE_SHA256=' "$ROOT_DIR/scripts/chimera.sh" >/dev/null || fail "bootstrap_missing_installed_bundle_sha_export"
rg -n 'https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera.sh' \
  "$ROOT_DIR/scripts/chimera-sh" >/dev/null || fail "launcher_update_url_not_latest_chimera_pq"
rg -n 'auto_update_if_needed "\$cmd" "\$\{@:2\}"' "$ROOT_DIR/scripts/chimera-sh" >/dev/null || fail "launcher_missing_update_first_for_mesh_or_connect"
rg -n 'CHIMERA_UPDATE_FIRST_CHECKED=1 exec "\$CONTROL" start' "$ROOT_DIR/scripts/chimera-sh" >/dev/null || fail "launcher_start_missing_update_first_marker"
rg -n 'CHIMERA_UPDATE_FIRST_CHECKED=1 exec "\$CONTROL" mesh' "$ROOT_DIR/scripts/chimera-sh" >/dev/null || fail "launcher_mesh_missing_update_first_marker"
rg -n 'CHIMERA_UPDATE_PEER_BOOTSTRAP_URLS' "$ROOT_DIR/scripts/chimera-sh" >/dev/null || fail "launcher_missing_peer_update_bootstrap_urls"
rg -n 'UPDATE_PEER_BOOTSTRAP_URLS_FILE' "$ROOT_DIR/scripts/chimera-sh" >/dev/null || fail "launcher_missing_peer_update_bootstrap_url_file"
rg -n 'try_update_from_bootstrap_source "peer"' "$ROOT_DIR/scripts/chimera-sh" >/dev/null || fail "launcher_missing_peer_update_fallback"
rg -n 'remote_archive_sha256 "\$remote_archive_url" "\$remote_checksum_url"' "$ROOT_DIR/scripts/chimera-sh" >/dev/null || fail "launcher_peer_update_checksum_not_bound"
rg -n 'reason=update_sources_unreachable' "$ROOT_DIR/scripts/chimera-sh" >/dev/null || fail "launcher_missing_update_source_unavailable_diagnostic"
rg -n 'PEER_UPDATE_IO_TIMEOUT' "$ROOT_DIR/crates/chimera-bootstrap/src/main.rs" >/dev/null || fail "peer_release_missing_io_timeout"
rg -n 'PEER_UPDATE_HEADER_TIMEOUT' "$ROOT_DIR/crates/chimera-bootstrap/src/main.rs" >/dev/null || fail "peer_release_missing_header_timeout"
rg -n 'PEER_UPDATE_MAX_ACTIVE_CONNECTIONS' "$ROOT_DIR/crates/chimera-bootstrap/src/main.rs" >/dev/null || fail "peer_release_missing_connection_cap"
rg -n 'thread::spawn' "$ROOT_DIR/crates/chimera-bootstrap/src/main.rs" >/dev/null || fail "peer_release_missing_worker_thread"
rg -n 'set_read_timeout' "$ROOT_DIR/crates/chimera-bootstrap/src/main.rs" >/dev/null || fail "peer_release_missing_read_timeout"
rg -n 'set_write_timeout' "$ROOT_DIR/crates/chimera-bootstrap/src/main.rs" >/dev/null || fail "peer_release_missing_write_timeout"
rg -n 'HTTP request header timed out' "$ROOT_DIR/crates/chimera-bootstrap/src/main.rs" >/dev/null || fail "peer_release_missing_header_deadline"
rg -n 'too_many_active_connections' "$ROOT_DIR/crates/chimera-bootstrap/src/main.rs" >/dev/null || fail "peer_release_missing_active_connection_reject"
rg -n 'LATEST_ARCHIVE_NAME="chimera-pq-release\.tar\.gz"' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_stable_latest_archive"
rg -n 'CHIMERA_RELEASE_VERSION must be semver' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_semver_guard"
if rg -n 'date \+%Y%m%d' "$ROOT_DIR/scripts/build_release.sh" >/dev/null; then
  fail "release_build_uses_timestamp_version_fallback"
fi
rg -n 'cargo build --release' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_binary_build_step"
rg -n 'target/release/chimera-cli' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_ready_binary_copy"
rg -n 'target/chimera\.sh' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_public_bootstrap_asset"
rg -n '\.sha256' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_checksum_output"
rg -n 'sha256sum -c "\$\{LATEST_CHECKSUM_NAME\}"' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_checksum_self_verify"
[[ -f "$ROOT_DIR/.github/workflows/release.yml" ]] || fail "github_release_workflow_missing"
rg -n 'gh release create "\$RELEASE_TAG"' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_create_missing"
rg -n 'target/chimera\.sh' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_bootstrap_asset"
rg -n 'target/chimera-pq-release\.tar\.gz' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_archive_asset"
rg -n 'target/chimera-pq-release\.tar\.gz\.sha256' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_checksum_asset"
rg -n 'gh release view --json tagName' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_latest_verification_missing"
rg -n 'release assets do not match required set' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_asset_set_guard_missing"
rg -n 'configure_peer_egress_env "node"' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_not_weave_node_first"
rg -n '^INSTALL_NODE_ROLE="\$\{CHIMERA_INSTALL_NODE_ROLE:-node\}"' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_default_role_not_node"
if rg -n 'configure_peer_egress_env "(vps|laptop)"' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null; then
  fail "installer_writes_legacy_peer_egress_role"
fi
rg -n '"node" \| "weave-node" => Mode::Node' "$ROOT_DIR/crates/chimera-carrier/src/peer_egress/options.rs" >/dev/null || fail "peer_egress_missing_node_mode"
rg -n 'Mode::Node => node::run_node' "$ROOT_DIR/crates/chimera-carrier/src/bin/chimera-peer-egress.rs" >/dev/null || fail "peer_egress_binary_not_dispatching_node_mode"
rg -n 'remote release checksum is required for URL install' "$ROOT_DIR/scripts/install_release.sh" >/dev/null || fail "install_release_url_checksum_not_required"
rg -n 'verify_checksum_required' "$ROOT_DIR/scripts/install_release.sh" >/dev/null || fail "install_release_missing_checksum_verification"
rg -n 'restore_previous_release' "$ROOT_DIR/scripts/install_release.sh" >/dev/null || fail "install_release_missing_failed_update_restore"
rg -n 'release checksum is required before archive extraction' "$ROOT_DIR/scripts/install_release.sh" >/dev/null || fail "install_release_local_archive_checksum_not_required"
rg -n 'DEFAULT_RELEASE_URL="https://github.com/neo-2022/chimera-pq/releases/latest/download/chimera-pq-release\.tar\.gz"' "$ROOT_DIR/scripts/install_release.sh" >/dev/null || fail "install_release_default_not_github_latest"
if rg -n 'cargo run|CHIMERA_ALLOW_CARGO_FALLBACK|CHIMERA_ALLOW_BUILD_FALLBACK|ALLOW_BUILD_FALLBACK' \
  "$ROOT_DIR/scripts/chimera-runner.sh" "$ROOT_DIR/scripts/chimera-control.sh" "$ROOT_DIR/scripts/chimera.sh" "$ROOT_DIR/scripts/chimera-sh" >/dev/null; then
  fail "runtime_contains_cargo_fallback"
fi
if rg -n 'neo-2022/chimera/main/chimera\.sh|raw\.githubusercontent\.com/neo-2022/chimera/' \
  "$ROOT_DIR/scripts/chimera.sh" "$ROOT_DIR/scripts/chimera-sh" "$ROOT_DIR/scripts/chimera_remote_cycle_smoke.sh" "$ROOT_DIR/scripts/install_release.sh" >/dev/null; then
  fail "legacy_wrong_repo_bootstrap_reference"
fi

echo "installer_gate=pass"
