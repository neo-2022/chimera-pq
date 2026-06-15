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
  "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_update_url_not_latest_chimera_pq"
rg -n 'auto_update_if_needed "\$cmd" "\$\{@:2\}"' "$ROOT_DIR/scripts/chimera-sh" >/dev/null || fail "launcher_missing_update_first_for_mesh_or_connect"
rg -n 'source "\$UPDATE_MODULE"' "$ROOT_DIR/scripts/chimera-sh" >/dev/null || fail "launcher_missing_update_module_source"
rg -n 'reason=missing_update_module' "$ROOT_DIR/scripts/chimera-sh" >/dev/null || fail "launcher_missing_update_module_fail_closed"
rg -n 'CHIMERA_UPDATE_FIRST_CHECKED=1 exec "\$CONTROL" start' "$ROOT_DIR/scripts/chimera-sh" >/dev/null || fail "launcher_start_missing_update_first_marker"
rg -n 'CHIMERA_UPDATE_FIRST_CHECKED=1 exec "\$CONTROL" mesh' "$ROOT_DIR/scripts/chimera-sh" >/dev/null || fail "launcher_mesh_missing_update_first_marker"
rg -n 'CHIMERA_UPDATE_PEER_BOOTSTRAP_URLS' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_peer_update_bootstrap_urls"
rg -n 'UPDATE_PEER_BOOTSTRAP_URLS_FILE' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_peer_update_bootstrap_url_file"
rg -n 'CHIMERA_UPDATE_DOWNLOAD_CONNECT_TIMEOUT_SEC' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_update_connect_timeout"
rg -n 'CHIMERA_UPDATE_DOWNLOAD_MAX_TIME_SEC' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_update_max_time"
rg -n 'CHIMERA_UPDATE_DOWNLOAD_RETRIES' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_update_retry_bound"
rg -n 'run_update_download_command' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_bounded_update_download_wrapper"
rg -n 'CHIMERA_BOOTSTRAP_CONNECT_TIMEOUT_SEC' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_bootstrap_connect_timeout_env"
rg -n 'CHIMERA_BOOTSTRAP_DOWNLOAD_TIMEOUT_SEC' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_bootstrap_download_timeout_env"
rg -n 'wget .*--tries=1 .*--timeout="\$connect_timeout_sec" .*--read-timeout="\$max_time_sec"' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_wget_download_not_bounded"
rg -n 'curl .*--retry "\$retries" .*--connect-timeout "\$connect_timeout_sec" .*--max-time "\$max_time_sec"' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_curl_download_not_bounded"
rg -n 'try_update_from_bootstrap_source "peer"' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_peer_update_fallback"
rg -n 'parse-peer-metadata' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_peer_metadata_not_rust_parsed"
rg -n 'chimera_peer_update_metadata' "$ROOT_DIR/crates/chimera-bootstrap/src/peer_update/metadata.rs" >/dev/null || fail "launcher_missing_peer_metadata_kind_check"
rg -n 'metadata_checksum_mismatch' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_peer_metadata_sha_binding"
rg -n 'load_update_peer_bootstrap_urls_for_args "\$\{original_args\[@\]\}"' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_connect_specific_peer_update_sources"
rg -n 'case_github_invalid_bootstrap_parse_does_not_try_peer_fallback' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_github_invalid_fail_closed_contract"
rg -n 'remote_archive_sha256 "\$remote_archive_url" "\$remote_checksum_url"' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_peer_update_checksum_not_bound"
rg -n 'reason=same_version_checksum_mismatch' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_same_version_checksum_mismatch_not_fail_closed"
rg -n 'case_same_version_checksum_mismatch_blocks' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_same_version_checksum_mismatch_contract"
rg -n 'reason=local_checksum_missing' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_same_version_missing_checksum_not_fail_closed"
rg -n 'case_same_version_missing_local_checksum_blocks' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_same_version_local_checksum_contract"
rg -n 'reason=update_sources_unreachable' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_update_source_unavailable_diagnostic"
rg -n 'reason=checksum_unreachable' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_confirmed_update_checksum_unreachable_block"
rg -n 'case_update_download_timeout_bounds_slow_helper' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_bounded_slow_helper_contract"
rg -n 'case_newer_release_with_unreachable_checksum_blocks' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_confirmed_newer_checksum_contract"
rg -n 'case_confirmed_update_missing_local_installer_blocks' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_local_installer_block_contract"
rg -n 'case_peer_confirmed_update_failure_blocks_start' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_peer_install_failure_block_contract"
rg -n 'systemd_stop_deletes_default_redirect_table' "$ROOT_DIR/scripts/chimera_stop_contract_smoke.sh" >/dev/null || fail "stop_contract_missing_systemd_cleanup"
rg -n 'systemd_stop_missing_redirect_table_is_idempotent' "$ROOT_DIR/scripts/chimera_stop_contract_smoke.sh" >/dev/null || fail "stop_contract_missing_idempotent_cleanup"
rg -n 'direct_stop_deletes_default_redirect_table' "$ROOT_DIR/scripts/chimera_stop_contract_smoke.sh" >/dev/null || fail "stop_contract_missing_direct_cleanup"
rg -n 'env_file_chimera_owned_redirect_table_is_allowed' "$ROOT_DIR/scripts/chimera_stop_contract_smoke.sh" >/dev/null || fail "stop_contract_missing_chimera_owned_custom_table"
rg -n 'invalid_redirect_table_fails_closed' "$ROOT_DIR/scripts/chimera_stop_contract_smoke.sh" >/dev/null || fail "stop_contract_missing_invalid_table_guard"
rg -n 'foreign_valid_table_fails_closed' "$ROOT_DIR/scripts/chimera_stop_contract_smoke.sh" >/dev/null || fail "stop_contract_missing_foreign_table_guard"
rg -n 'sudo_execution_failure_fails_stop' "$ROOT_DIR/scripts/chimera_stop_contract_smoke.sh" >/dev/null || fail "stop_contract_missing_sudo_failure_guard"
rg -n 'run_rejects_non_nft_override_case' "$ROOT_DIR/scripts/chimera_stop_contract_smoke.sh" >/dev/null || fail "stop_contract_missing_nft_override_guard"
rg -n 'restart_does_not_hide_cleanup_failure' "$ROOT_DIR/scripts/chimera_stop_contract_smoke.sh" >/dev/null || fail "stop_contract_missing_restart_failure_guard"
rg -n 'uninstall_does_not_hide_cleanup_failure' "$ROOT_DIR/scripts/chimera_stop_contract_smoke.sh" >/dev/null || fail "stop_contract_missing_uninstall_failure_guard"
rg -n 'valid_chimera_redirect_table' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_missing_chimera_redirect_table_ownership_guard"
resolve_nft_block="$(
  awk '
    /^resolve_nft_command\(\) \{/ { in_fn = 1 }
    in_fn { print }
    in_fn && /^}/ { exit }
  ' "$ROOT_DIR/scripts/chimera-control.sh"
)"
printf '%s\n' "$resolve_nft_block" | rg -n 'CHIMERA_ALLOW_TEST_NFT_BIN' >/dev/null || fail "control_missing_test_only_nft_override_guard"
printf '%s\n' "$resolve_nft_block" | rg -n '/usr/sbin/nft|/usr/bin/nft' >/dev/null || fail "control_missing_system_nft_allowlist"
if printf '%s\n' "$resolve_nft_block" | rg -n 'command -v nft' >/dev/null; then
  fail "control_uses_path_nft_resolution"
fi
rg -n 'CHIMERA_BOOTSTRAP_CONNECT_TIMEOUT_SEC' "$ROOT_DIR/crates/chimera-bootstrap/src/main.rs" >/dev/null || fail "bootstrap_missing_connect_timeout_env"
rg -n 'CHIMERA_BOOTSTRAP_DOWNLOAD_TIMEOUT_SEC' "$ROOT_DIR/crates/chimera-bootstrap/src/main.rs" >/dev/null || fail "bootstrap_missing_download_timeout_env"
rg -n 'url = "2"' "$ROOT_DIR/crates/chimera-bootstrap/Cargo.toml" >/dev/null || fail "peer_metadata_missing_url_parser_dependency"
rg -n 'serde_json::from_str' "$ROOT_DIR/crates/chimera-bootstrap/src/peer_update/metadata.rs" >/dev/null || fail "peer_metadata_missing_json_parser"
rg -n 'require_same_origin' "$ROOT_DIR/crates/chimera-bootstrap/src/peer_update/metadata.rs" >/dev/null || fail "peer_metadata_missing_same_origin_check"
rg -n 'url.contains.*@|contains.*@' "$ROOT_DIR/crates/chimera-mesh/src/nodes_model.rs" >/dev/null || fail "mesh_update_url_allows_userinfo"
if rg -n 'json_string_field|sed -n "s/.*\\".*metadata|awk .*metadata' "$ROOT_DIR/scripts/chimera-update.sh" "$ROOT_DIR/scripts/chimera-sh" >/dev/null; then
  fail "peer_metadata_shell_json_parser_detected"
fi
rg -n 'PEER_UPDATE_IO_TIMEOUT' "$ROOT_DIR/crates/chimera-bootstrap/src/peer_update/server.rs" >/dev/null || fail "peer_release_missing_io_timeout"
rg -n 'PEER_UPDATE_HEADER_TIMEOUT' "$ROOT_DIR/crates/chimera-bootstrap/src/peer_update/server.rs" >/dev/null || fail "peer_release_missing_header_timeout"
rg -n 'PEER_UPDATE_MAX_ACTIVE_CONNECTIONS' "$ROOT_DIR/crates/chimera-bootstrap/src/peer_update/server.rs" >/dev/null || fail "peer_release_missing_connection_cap"
rg -n 'thread::spawn' "$ROOT_DIR/crates/chimera-bootstrap/src/peer_update/server.rs" >/dev/null || fail "peer_release_missing_worker_thread"
rg -n 'set_read_timeout' "$ROOT_DIR/crates/chimera-bootstrap/src/peer_update/server.rs" >/dev/null || fail "peer_release_missing_read_timeout"
rg -n 'set_write_timeout' "$ROOT_DIR/crates/chimera-bootstrap/src/peer_update/server.rs" >/dev/null || fail "peer_release_missing_write_timeout"
rg -n 'HTTP request header timed out' "$ROOT_DIR/crates/chimera-bootstrap/src/peer_update/server.rs" >/dev/null || fail "peer_release_missing_header_deadline"
rg -n 'too_many_active_connections' "$ROOT_DIR/crates/chimera-bootstrap/src/peer_update/server.rs" >/dev/null || fail "peer_release_missing_active_connection_reject"
rg -n 'LATEST_ARCHIVE_NAME="chimera-pq-release\.tar\.gz"' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_stable_latest_archive"
rg -n 'CHIMERA_RELEASE_VERSION must be semver' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_semver_guard"
if rg -n 'date \+%Y%m%d' "$ROOT_DIR/scripts/build_release.sh" >/dev/null; then
  fail "release_build_uses_timestamp_version_fallback"
fi
rg -n 'cargo build --release' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_binary_build_step"
rg -n 'target/release/chimera-cli' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_ready_binary_copy"
rg -n 'target/chimera\.sh' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_public_bootstrap_asset"
rg -n 'scripts/install_release\.sh' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_update_installer"
rg -n 'scripts/chimera-update\.sh' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_update_module"
if rg -n 'scripts/chimera-update\.sh.*\|\| true|scripts/chimera-sh.*\|\| true|scripts/chimera\.sh.*\|\| true' "$ROOT_DIR/scripts/build_release.sh" >/dev/null; then
  fail "release_build_optional_required_update_scripts"
fi
rg -n 'chimera-release/bin/chimera-bootstrap' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_bootstrap_binary_content_guard"
rg -n 'chimera-release/scripts/install_release\\.sh' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_update_installer_content_guard"
rg -n 'chimera-release/scripts/chimera-update\\.sh' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_update_module_content_guard"
rg -n '^[[:space:]]*--base-url http://node\.example:18179' "$ROOT_DIR/docs/OPERATIONS.md" >/dev/null || fail "operations_missing_peer_release_base_url_example"
rg -n 'serve_release\(Path::new\(&root\), &listen, base_url\.as_deref\(\)\)' "$ROOT_DIR/crates/chimera-bootstrap/src/main.rs" >/dev/null || fail "bootstrap_missing_peer_release_base_url_wiring"
rg -n '\.sha256' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_checksum_output"
rg -n 'sha256sum -c "\$\{LATEST_CHECKSUM_NAME\}"' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_checksum_self_verify"
[[ -f "$ROOT_DIR/.github/workflows/release.yml" ]] || fail "github_release_workflow_missing"
rg -n 'gh release create "\$RELEASE_TAG"' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_create_missing"
rg -n 'target/chimera\.sh' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_bootstrap_asset"
rg -n 'target/chimera-pq-release\.tar\.gz' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_archive_asset"
rg -n 'target/chimera-pq-release\.tar\.gz\.sha256' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_checksum_asset"
rg -n 'chimera-release/bin/chimera-bootstrap' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_bootstrap_binary_bundle_guard"
rg -n -F 'chimera-release/scripts/install_release\.sh' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_update_installer_bundle_guard"
rg -n 'chimera-release/scripts/chimera-update\.sh' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_update_module_bundle_guard"
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
