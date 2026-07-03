#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "installer_gate=fail reason=$1" >&2
  exit 1
}

write_gitvers_bootstrap_template() {
  local dest="${1:?dest_required}"
  cat >"$dest" <<'EOF'
https://gitverse.ru/api/repos/ArtReg/chimera/raw/branch/main/chimera.sh
EOF
}

run_installer_env_contract_smoke() {
  local tmp_dir install_root fake_bin env_file output rc
  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  fake_bin="$tmp_dir/bin"
  mkdir -p \
    "$fake_bin" \
    "$install_root/bin" \
    "$install_root/configs" \
    "$install_root/deploy/desktop" \
    "$install_root/deploy/systemd-user" \
    "$install_root/scripts" \
    "$tmp_dir/cache" \
    "$tmp_dir/config" \
    "$tmp_dir/data" \
    "$tmp_dir/home" \
    "$tmp_dir/runtime"

  cp "$ROOT_DIR/scripts/install_desktop_control.sh" "$install_root/scripts/install_desktop_control.sh"
  chmod +x "$install_root/scripts/install_desktop_control.sh"

  cat >"$install_root/scripts/chimera-control.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  preflight-perms)
    echo "preflight_status=ok"
    ;;
  grant-perms)
    ;;
  *)
    ;;
esac
EOF
  chmod +x "$install_root/scripts/chimera-control.sh"

  for script in \
    chimera-control-launcher.sh \
    chimera-control-tray.sh \
    chimera-sh \
    chimera-update.sh \
    chimera.sh
  do
    cat >"$install_root/scripts/$script" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
    chmod +x "$install_root/scripts/$script"
  done

  cat >"$install_root/bin/chimera-cli" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
  chmod +x "$install_root/bin/chimera-cli"

  cat >"$fake_bin/nft" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  chmod +x "$fake_bin/nft"

  cat >"$fake_bin/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "--user" && "${2:-}" == "show-environment" ]]; then
  exit 1
fi
exit 0
EOF
  chmod +x "$fake_bin/systemctl"

  cat >"$install_root/configs/mesh-node.example.conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = 127.0.0.1:1
carrier.server_name = node.local
EOF
  cat >"$install_root/configs/mesh_bootstrap.env.example" <<'EOF'
CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS=4000
EOF
  write_gitvers_bootstrap_template "$install_root/configs/update_gitvers_bootstrap_urls.example.list"
  cat >"$install_root/deploy/desktop/chimera-control-gui.desktop" <<'EOF'
[Desktop Entry]
Name=CHIMERA
Exec=__CHIMERA_ROOT__/scripts/chimera-control-launcher.sh
Type=Application
EOF
  touch \
    "$install_root/deploy/systemd-user/chimera-node.service" \
    "$install_root/deploy/systemd-user/chimera-datapath.service"

  set +e
  output="$(
    PATH="$fake_bin:$PATH" \
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$tmp_dir/cache" \
    XDG_CONFIG_HOME="$tmp_dir/config" \
    XDG_DATA_HOME="$tmp_dir/data" \
    XDG_RUNTIME_DIR="$tmp_dir/runtime" \
    CHIMERA_INSTALL_NODE_ROLE=node \
    CHIMERA_MESH_REMOTE_ENDPOINT=198.51.100.10:18142 \
    CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:18135 \
    CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:18142 \
      timeout 15s bash "$install_root/scripts/install_desktop_control.sh" 2>&1
  )"
  rc=$?
  set -e
  [[ "$rc" -eq 0 ]] || {
    echo "$output" >&2
    fail "installer_env_contract_smoke_failed"
  }

  env_file="$tmp_dir/config/chimera/peer-egress.env"
  rg -q '^CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127\.0\.0\.1:18135$' "$env_file" || {
    echo "$output" >&2
    fail "installer_local_listen_env_value_not_written"
  }
  rg -q '^CHIMERA_PEER_EGRESS_PEER_LISTEN=0\.0\.0\.0:18142$' "$env_file" || {
    echo "$output" >&2
    fail "installer_peer_listen_env_value_not_written"
  }
  rm -rf "$tmp_dir"
}

run_installer_configured_node_materialization_smoke() {
  local tmp_dir install_root fake_bin output rc node_conf endpoint_file
  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  fake_bin="$tmp_dir/bin"
  mkdir -p \
    "$fake_bin" \
    "$install_root/bin" \
    "$install_root/configs" \
    "$install_root/deploy/desktop" \
    "$install_root/deploy/systemd-user" \
    "$install_root/scripts" \
    "$tmp_dir/cache" \
    "$tmp_dir/config" \
    "$tmp_dir/data" \
    "$tmp_dir/home" \
    "$tmp_dir/runtime"

  cp "$ROOT_DIR/scripts/install_desktop_control.sh" "$install_root/scripts/install_desktop_control.sh"
  chmod +x "$install_root/scripts/install_desktop_control.sh"

  cat >"$install_root/scripts/chimera-control.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  preflight-perms)
    echo "preflight_status=ok"
    ;;
  grant-perms)
    ;;
  *)
    ;;
esac
EOF
  chmod +x "$install_root/scripts/chimera-control.sh"

  for script in \
    chimera-control-launcher.sh \
    chimera-control-tray.sh \
    chimera-sh \
    chimera-update.sh \
    chimera.sh
  do
    cat >"$install_root/scripts/$script" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
    chmod +x "$install_root/scripts/$script"
  done

  cat >"$install_root/bin/chimera-cli" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
  chmod +x "$install_root/bin/chimera-cli"

  cat >"$fake_bin/nft" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  chmod +x "$fake_bin/nft"

  cat >"$fake_bin/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "--user" && "${2:-}" == "show-environment" ]]; then
  exit 1
fi
exit 0
EOF
  chmod +x "$fake_bin/systemctl"

  cat >"$install_root/configs/mesh-node.example.conf" <<'EOF'
carrier.profile = tls
carrier.addr = ${CHIMERA_NODE_PEER_ENDPOINT}
carrier.server_name = ${CHIMERA_NODE_SERVER_NAME}
capture.mode = auto
capture.tun_supported = true
peer.listen_addr = ${CHIMERA_NODE_LISTEN_ADDR}
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
  cat >"$install_root/configs/mesh_bootstrap.env.example" <<'EOF'
CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS=4000
EOF
  write_gitvers_bootstrap_template "$install_root/configs/update_gitvers_bootstrap_urls.example.list"
  cat >"$install_root/deploy/desktop/chimera-control-gui.desktop" <<'EOF'
[Desktop Entry]
Name=CHIMERA
Exec=__CHIMERA_ROOT__/scripts/chimera-control-launcher.sh
Type=Application
EOF
  touch \
    "$install_root/deploy/systemd-user/chimera-node.service" \
    "$install_root/deploy/systemd-user/chimera-datapath.service"

  set +e
  output="$(
    PATH="$fake_bin:$PATH" \
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$tmp_dir/cache" \
    XDG_CONFIG_HOME="$tmp_dir/config" \
    XDG_DATA_HOME="$tmp_dir/data" \
    XDG_RUNTIME_DIR="$tmp_dir/runtime" \
    CHIMERA_INSTALL_NODE_ROLE=node \
    CHIMERA_MESH_REMOTE_ENDPOINT=node.mesh.invalid:18142 \
      timeout 15s bash "$install_root/scripts/install_desktop_control.sh" 2>&1
  )"
  rc=$?
  set -e
  [[ "$rc" -eq 0 ]] || {
    echo "$output" >&2
    fail "installer_configured_node_materialization_smoke_failed"
  }

  node_conf="$install_root/configs/mesh-node.conf"
  endpoint_file="$install_root/configs/chimera_runtime_endpoint.txt"
  [[ -f "$node_conf" ]] || fail "installer_missing_configured_node_conf"
  rg -q '^node\.mode = mesh-node$' "$node_conf" || fail "installer_configured_node_mode_missing"
  rg -q '^carrier\.addr = tcp://node\.mesh\.invalid:18142$' "$node_conf" || fail "installer_configured_node_carrier_addr_not_tcp"
  rg -q '^carrier\.server_name = node\.mesh\.invalid$' "$node_conf" || fail "installer_configured_node_server_name_not_materialized"
  rg -q '^peer\.listen_addr = auto$' "$node_conf" || fail "installer_configured_node_listen_addr_not_materialized"
  ! rg -q '\$\{CHIMERA_NODE_SERVER_NAME\}|\$\{CHIMERA_NODE_LISTEN_ADDR\}' "$node_conf" || fail "installer_configured_node_conf_kept_template_placeholders"
  [[ -f "$endpoint_file" ]] || fail "installer_missing_runtime_endpoint_file"
  grep -qx 'node.mesh.invalid:18142' "$endpoint_file" || fail "installer_runtime_endpoint_file_not_raw_host_port"
  rm -rf "$tmp_dir"
}

run_installer_unconfigured_node_template_smoke() {
  local tmp_dir install_root fake_bin output rc node_conf
  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  fake_bin="$tmp_dir/bin"
  mkdir -p \
    "$fake_bin" \
    "$install_root/bin" \
    "$install_root/configs" \
    "$install_root/deploy/desktop" \
    "$install_root/deploy/systemd-user" \
    "$install_root/scripts" \
    "$tmp_dir/cache" \
    "$tmp_dir/config" \
    "$tmp_dir/data" \
    "$tmp_dir/home" \
    "$tmp_dir/runtime"

  cp "$ROOT_DIR/scripts/install_desktop_control.sh" "$install_root/scripts/install_desktop_control.sh"
  chmod +x "$install_root/scripts/install_desktop_control.sh"

  cat >"$install_root/scripts/chimera-control.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  preflight-perms)
    echo "preflight_status=ok"
    ;;
  grant-perms)
    ;;
  *)
    ;;
esac
EOF
  chmod +x "$install_root/scripts/chimera-control.sh"

  for script in \
    chimera-control-launcher.sh \
    chimera-control-tray.sh \
    chimera-sh \
    chimera-update.sh \
    chimera.sh
  do
    cat >"$install_root/scripts/$script" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
    chmod +x "$install_root/scripts/$script"
  done

  cat >"$install_root/bin/chimera-cli" <<'EOF'
#!/usr/bin/env bash
case "${1:-}" in
  mesh)
    exit 1
    ;;
  *)
    exit 0
    ;;
esac
EOF
  chmod +x "$install_root/bin/chimera-cli"

  cat >"$fake_bin/nft" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  chmod +x "$fake_bin/nft"

  cat >"$fake_bin/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "--user" && "${2:-}" == "show-environment" ]]; then
  exit 1
fi
exit 0
EOF
  chmod +x "$fake_bin/systemctl"

  cat >"$install_root/configs/mesh-node.example.conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = 203.0.113.10:443
carrier.server_name = node.local
capture.mode = auto
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
  cat >"$install_root/configs/mesh_bootstrap.env.example" <<'EOF'
CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS=4000
EOF
  write_gitvers_bootstrap_template "$install_root/configs/update_gitvers_bootstrap_urls.example.list"
  cat >"$install_root/deploy/desktop/chimera-control-gui.desktop" <<'EOF'
[Desktop Entry]
Name=CHIMERA
Exec=__CHIMERA_ROOT__/scripts/chimera-control-launcher.sh
Type=Application
EOF
  touch \
    "$install_root/deploy/systemd-user/chimera-node.service" \
    "$install_root/deploy/systemd-user/chimera-datapath.service"

  set +e
  output="$(
    PATH="$fake_bin:$PATH" \
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$tmp_dir/cache" \
    XDG_CONFIG_HOME="$tmp_dir/config" \
    XDG_DATA_HOME="$tmp_dir/data" \
    XDG_RUNTIME_DIR="$tmp_dir/runtime" \
    CHIMERA_INSTALL_NODE_ROLE=node \
      timeout 15s bash "$install_root/scripts/install_desktop_control.sh" 2>&1
  )"
  rc=$?
  set -e
  [[ "$rc" -eq 0 ]] || {
    echo "$output" >&2
    fail "installer_unconfigured_node_template_smoke_failed"
  }
  [[ "$output" == *"peer_config_node_endpoint=none"* ]] || fail "installer_missing_unconfigured_endpoint_diagnostic"
  node_conf="$install_root/configs/mesh-node.conf"
  [[ -f "$node_conf" ]] || fail "installer_missing_unconfigured_node_conf"
  rg -q '^carrier\.addr = 203\.0\.113\.10:443$' "$node_conf" || fail "installer_node_conf_not_inert_placeholder"
  rm -rf "$tmp_dir"
}

rg -n "installer_gate_prepare_bootstrap_env|transparent runtime|transparent runtime" \
  "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_missing_transparent_bootstrap"

rg -n "datapath-status|transparent_runtime|split-transparent" \
  "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_missing_transparent_runtime"
rg -n '^NODE_LOG="\$\{NODE_LOG:-\$\{GATEWAY_LOG:-\$\{XDG_CACHE_HOME:-\$HOME/\.cache\}/chimera/chimera_node\.service\.log\}\}"' \
  "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_node_log_not_user_cache"
rg -n '^DATAPATH_LOG="\$\{DATAPATH_LOG:-\$\{CLIENT_LOG:-\$\{XDG_CACHE_HOME:-\$HOME/\.cache\}/chimera/chimera_datapath\.service\.log\}\}"' \
  "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_datapath_log_not_user_cache"
rg -n '^ensure_runtime_log_paths\(\) \{$' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_missing_runtime_log_preparation_helper"
rg -n '^  ensure_runtime_log_paths$' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_missing_runtime_log_preparation_call"
if rg -n '^Standard(Output|Error)=append:__CHIMERA_ROOT__' "$ROOT_DIR/deploy/systemd-user"/*.service >/dev/null; then
  fail "systemd_unit_logs_under_release_root"
fi
rg -n '^StandardOutput=append:%h/\.cache/chimera/chimera_node\.service\.log$' \
  "$ROOT_DIR/deploy/systemd-user/chimera-node.service" >/dev/null || fail "node_unit_stdout_not_user_cache"
rg -n '^StandardError=append:%h/\.cache/chimera/chimera_node\.service\.log$' \
  "$ROOT_DIR/deploy/systemd-user/chimera-node.service" >/dev/null || fail "node_unit_stderr_not_user_cache"
rg -n '^StandardOutput=append:%h/\.cache/chimera/chimera_datapath\.service\.log$' \
  "$ROOT_DIR/deploy/systemd-user/chimera-datapath.service" >/dev/null || fail "datapath_unit_stdout_not_user_cache"
rg -n '^StandardError=append:%h/\.cache/chimera/chimera_datapath\.service\.log$' \
  "$ROOT_DIR/deploy/systemd-user/chimera-datapath.service" >/dev/null || fail "datapath_unit_stderr_not_user_cache"
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
rg -n 'CHIMERA_UPDATE_GITVERS_BOOTSTRAP_URLS' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_gitvers_update_bootstrap_urls"
rg -n 'UPDATE_GITVERS_BOOTSTRAP_URLS_DEFAULT=' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_gitvers_update_default_source"
rg -n 'UPDATE_GITVERS_BOOTSTRAP_URLS_FILE' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_gitvers_update_bootstrap_url_file"
rg -n 'UPDATE_PEER_BOOTSTRAP_URLS_FILE' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_peer_update_bootstrap_url_file"
rg -n 'CHIMERA_UPDATE_DOWNLOAD_CONNECT_TIMEOUT_SEC' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_update_connect_timeout"
rg -n 'CHIMERA_UPDATE_DOWNLOAD_MAX_TIME_SEC' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_update_max_time"
rg -n 'CHIMERA_UPDATE_DOWNLOAD_RETRIES' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_update_retry_bound"
rg -n 'run_update_download_command' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_bounded_update_download_wrapper"
rg -n 'CHIMERA_BOOTSTRAP_CONNECT_TIMEOUT_SEC' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_bootstrap_connect_timeout_env"
rg -n 'CHIMERA_BOOTSTRAP_DOWNLOAD_TIMEOUT_SEC' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_bootstrap_download_timeout_env"
rg -n 'wget --no-config .*--tries=1 .*--timeout="\$connect_timeout_sec" .*--read-timeout="\$max_time_sec"' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_wget_download_not_bounded"
rg -n 'curl --disable .*--retry "\$retries" .*--connect-timeout "\$connect_timeout_sec" .*--max-time "\$max_time_sec"' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_curl_download_not_bounded"
rg -n 'curl --disable .*--retry 3 .*--connect-timeout 10 .*--max-time 60' "$ROOT_DIR/scripts/chimera.sh" >/dev/null || fail "bootstrap_curl_rcfile_not_disabled"
rg -n 'GITVERS_BOOTSTRAP_URLS_DEFAULT=' "$ROOT_DIR/scripts/chimera.sh" >/dev/null || fail "bootstrap_missing_gitvers_default_source"
rg -n 'GITVERS_BOOTSTRAP_URLS_FILE' "$ROOT_DIR/scripts/chimera.sh" >/dev/null || fail "bootstrap_missing_gitvers_source_file"
rg -n 'bootstrap_install_from_bootstrap_source "gitvers"' "$ROOT_DIR/scripts/chimera.sh" >/dev/null || fail "bootstrap_missing_gitvers_fallback"
rg -n 'bootstrap_uninstall_current_installation' "$ROOT_DIR/scripts/chimera.sh" >/dev/null || fail "bootstrap_missing_self_uninstall"
rg -n 'INSTALL_LOCAL_BIN_FILE=' "$ROOT_DIR/scripts/chimera.sh" >/dev/null || fail "bootstrap_missing_install_local_bin_contract"
rg -n "bash -o pipefail -c 'curl --disable -fsSL --retry 3 --connect-timeout 10 --max-time 60 .*chimera\\.sh \\| bash -s -- -install'" "$ROOT_DIR/scripts/chimera.sh" >/dev/null || fail "bootstrap_usage_not_pipefail_bounded"
rg -n "bash -o pipefail -lc 'curl --disable -fsSL --retry 3 --connect-timeout 10 --max-time 60 .*\\\$bootstrap_url.* \\| bash -s -- -install'" "$ROOT_DIR/scripts/chimera_remote_cycle_smoke.sh" >/dev/null || fail "remote_cycle_bootstrap_not_pipefail_bounded"
if ! bash -lc 'false | bash -s -- -install'; then
  fail "bootstrap_pipe_without_pipefail_contract_changed"
fi
if bash -o pipefail -lc 'false | bash -s -- -install'; then
  fail "bootstrap_pipefail_did_not_block_download_failure"
fi
rg -n 'wget --no-config .*--tries=3 .*--timeout=10 .*--dns-timeout=10 .*--connect-timeout=10 .*--read-timeout=60 .*--waitretry=1 .*-qO "\$dest" "\$url"' "$ROOT_DIR/scripts/chimera.sh" >/dev/null || fail "bootstrap_wget_download_not_bounded"
rg -n 'curl --disable .*--retry 3 .*--connect-timeout 10 .*--max-time 60' "$ROOT_DIR/scripts/install_release.sh" >/dev/null || fail "install_release_curl_rcfile_not_disabled"
rg -n 'wget --no-config .*--tries=3 .*--timeout=10 .*--dns-timeout=10 .*--connect-timeout=10 .*--read-timeout=60 .*--waitretry=1 .*-qO "\$dest" "\$url"' "$ROOT_DIR/scripts/install_release.sh" >/dev/null || fail "install_release_wget_download_not_bounded"
if rg -n 'RUNTIME_BOOTSTRAP_SCRIPT|ensure-singbox|SINGBOX_BIN|singbox-split\.json|chimera-singbox\.pid' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null; then
  fail "control_start_uses_legacy_third_party_runtime_bootstrap"
fi
if rg -n 'chimera_runtime_bootstrap\.sh' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null; then
  fail "installer_chmods_legacy_third_party_runtime_bootstrap"
fi
rg -n 'curl disable rcfile flag missing' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "update_smoke_missing_curl_disable_contract"
rg -n 'try_update_from_bootstrap_source "peer"' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_peer_update_fallback"
rg -n 'try_update_from_bootstrap_source "gitvers"' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_gitvers_update_fallback"
rg -n 'parse-peer-metadata' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_peer_metadata_not_rust_parsed"
rg -n 'chimera_peer_update_metadata' "$ROOT_DIR/crates/chimera-bootstrap/src/peer_update/metadata.rs" >/dev/null || fail "launcher_missing_peer_metadata_kind_check"
rg -n 'metadata_checksum_mismatch' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_peer_metadata_sha_binding"
rg -n 'load_update_peer_bootstrap_urls_for_args "\$\{original_args\[@\]\}"' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_connect_specific_peer_update_sources"
rg -n 'case_github_invalid_bootstrap_parse_does_not_try_peer_fallback' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_github_invalid_fail_closed_contract"
rg -n 'case_gitverse_default_source_loads_without_operator_config' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_gitvers_default_load_contract"
rg -n 'case_github_unavailable_gitvers_newer_updates_before_peer' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_gitvers_success_contract"
rg -n 'case_github_unavailable_default_gitvers_newer_updates_without_operator_config' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_default_gitvers_update_contract"
rg -n 'case_github_unavailable_gitvers_unavailable_falls_back_to_peer' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_gitvers_outage_peer_contract"
rg -n 'case_github_unavailable_gitvers_invalid_does_not_try_peer' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_gitvers_invalid_fail_closed_contract"
rg -n 'remote_archive_sha256 "\$remote_archive_url" "\$remote_checksum_url"' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_peer_update_checksum_not_bound"
rg -n 'reason=same_version_checksum_mismatch' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_same_version_checksum_mismatch_not_fail_closed"
rg -n 'case_same_version_checksum_mismatch_blocks' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_same_version_checksum_mismatch_contract"
rg -n 'reason=local_checksum_missing' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_same_version_missing_checksum_not_fail_closed"
rg -n 'case_same_version_missing_local_checksum_blocks' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_same_version_local_checksum_contract"
rg -n 'reason=source_not_newer' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_not_newer_source_continue_diagnostic"
rg -n 'case_github_current_stops_before_peer_newer_update' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_verified_github_current_ceiling_contract"
rg -n 'case_github_stale_stops_before_peer_newer_update' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_verified_github_stale_ceiling_contract"
rg -n 'case_github_unavailable_gitvers_current_stops_before_peer_newer_update' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_verified_gitvers_current_ceiling_contract"
rg -n 'case_github_install_source_unavailable_blocks_peer_newer_release' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_divergent_peer_release_block_contract"
rg -n 'case_github_install_source_unavailable_blocks_peer_checksum_divergence' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_divergent_peer_checksum_block_contract"
rg -n 'reason=trusted_version_divergence' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_trusted_version_divergence_block"
rg -n 'reason=trusted_checksum_divergence' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_trusted_checksum_divergence_block"
rg -n 'source "\$ROOT_DIR/scripts/chimera-update-runtime-state\.sh"' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_runtime_state_helper"
rg -n 'read_local_runtime_bundle_sha' "$ROOT_DIR/scripts/chimera-update-runtime-state.sh" >/dev/null || fail "launcher_missing_runtime_state_bundle_reader"
rg -n 'sha256_file' "$ROOT_DIR/scripts/chimera-update-runtime-state.sh" >/dev/null || fail "launcher_missing_runtime_state_sha_helper"
rg -n 'reason=update_sources_unreachable' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_update_source_unavailable_diagnostic"
rg -n 'reason=checksum_unreachable' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "launcher_missing_confirmed_update_checksum_unreachable_block"
rg -n 'case_update_download_timeout_bounds_slow_helper' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_bounded_slow_helper_contract"
rg -n 'case_newer_release_with_unreachable_checksum_continues' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_confirmed_newer_checksum_contract"
rg -n 'case_newer_release_with_unreachable_checksum_falls_back_to_peer' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "launcher_missing_peer_fallback_after_checksum_outage_contract"
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
rg -n 'systemd_datapath_apply_failure' "$ROOT_DIR/scripts/chimera_start_contract_smoke.sh" >/dev/null || fail "start_contract_missing_systemd_datapath_apply_failure"
rg -n 'systemd_apply_rc0_state_missing' "$ROOT_DIR/scripts/chimera_start_contract_smoke.sh" >/dev/null || fail "start_contract_missing_rc0_state_missing"
rg -n 'systemd_apply_rc0_network_not_modified' "$ROOT_DIR/scripts/chimera_start_contract_smoke.sh" >/dev/null || fail "start_contract_missing_rc0_network_not_modified"
rg -n 'systemd_apply_rc0_valid_state_allows_ok' "$ROOT_DIR/scripts/chimera_start_contract_smoke.sh" >/dev/null || fail "start_contract_missing_valid_state_positive_case"
rg -n 'route_status_without_proof' "$ROOT_DIR/scripts/chimera_start_contract_smoke.sh" >/dev/null || fail "start_contract_missing_route_status_without_proof_case"
rg -n 'direct_datapath_apply_failure' "$ROOT_DIR/scripts/chimera_start_contract_smoke.sh" >/dev/null || fail "start_contract_missing_direct_datapath_apply_failure"
rg -n 'reason=datapath_apply_failed' "$ROOT_DIR/scripts/chimera_start_contract_smoke.sh" >/dev/null || fail "start_contract_missing_apply_failure_reason"
rg -n 'reason=datapath_proof_failed' "$ROOT_DIR/scripts/chimera_start_contract_smoke.sh" >/dev/null || fail "start_contract_missing_proof_failure_reason"
rg -n 'datapath_rollback=ok' "$ROOT_DIR/scripts/chimera_start_contract_smoke.sh" >/dev/null || fail "start_contract_missing_apply_failure_rollback_assert"
rg -n 'partial runtime state was not removed by rollback' "$ROOT_DIR/scripts/chimera_start_contract_smoke.sh" >/dev/null || fail "start_contract_missing_partial_state_cleanup_assert"
rg -n 'node/datapath units were not stopped after apply failure' "$ROOT_DIR/scripts/chimera_start_contract_smoke.sh" >/dev/null || fail "start_contract_missing_systemd_apply_failure_stop_assert"
rg -n 'datapath_apply_failed' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_start_missing_apply_failure_contract"
rg -n 'datapath_proof_failed' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_start_missing_proof_failure_contract"
rg -n 'rollback recover' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_start_missing_apply_failure_recover"
rg -n 'datapath_apply_proof_state' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_start_missing_proof_validator_call"
rg -n 'state_proof_command' "$ROOT_DIR/crates/chimera-cli/src/main.rs" >/dev/null || fail "cli_missing_state_proof_command"
rg -n 'validate_datapath_state_proof' "$ROOT_DIR/crates/chimera-cli/src/main.rs" >/dev/null || fail "cli_missing_datapath_state_proof_validator"
rg -n 'duplicate_field' "$ROOT_DIR/crates/chimera-cli/src/main.rs" >/dev/null || fail "cli_missing_duplicate_key_state_proof_guard"
rg -n 'unverified' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_start_missing_unverified_apply_status"
rg -n 'datapath_mode="unknown"' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_route_status_missing_unknown_mode"
rg -n 'installed_state_proof_missing_state' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_state_proof_missing_state_check"
rg -n 'installed_state_proof_invalid_not_rejected' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_state_proof_invalid_check"
rg -n 'installed_state_proof_duplicate_field' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_state_proof_duplicate_check"
rg -n 'installed_state_proof_network_not_modified' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_state_proof_network_check"
rg -n 'installed_state_proof_tun_not_applied' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_state_proof_tun_check"
rg -n 'installed_state_proof_route_not_applied' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_state_proof_route_check"
rg -n 'installed_state_proof_dns_not_applied' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_state_proof_dns_check"
rg -n 'installed_state_proof_valid_not_accepted' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_state_proof_valid_check"
rg -n 'installed_route_status_without_proof' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_route_status_without_proof_check"
rg -n 'installed_route_status_duplicate_field' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_route_status_duplicate_check"
rg -n 'installed_route_status_network_not_modified' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_route_status_network_check"
rg -n 'installed_route_status_tun_not_applied' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_route_status_tun_check"
rg -n 'installed_route_status_route_not_applied' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_route_status_route_check"
rg -n 'installed_route_status_dns_not_applied' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_route_status_dns_check"
rg -n 'installed_route_status_valid_apply_without_flow_proof' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_route_status_valid_apply_without_flow_proof_check"
rg -n 'installed_route_status_stale_flow_proof' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_route_status_stale_flow_proof_check"
rg -n 'installed_route_status_valid_flow_proof' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_route_status_valid_flow_proof_check"
rg -n 'datapath_apply=unverified' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_route_status_unverified_assert"
rg -n 'datapath_flow_proof=missing_flow_proof' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_route_status_missing_flow_proof_assert"
rg -n 'datapath_flow_proof=flow_stale' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_route_status_flow_stale_assert"
rg -n 'datapath_mode=transparent' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_route_status_transparent_assert"
rg -n 'datapath_proof=ok' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_route_status_ok_assert"
rg -n 'datapath_flow_proof=ok' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_installed_route_status_flow_ok_assert"
rg -n 'installed_gitvers_bootstrap_sources_missing' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_gitvers_sources_check"
rg -n 'installed_gitvers_bootstrap_sources_mode' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_gitvers_sources_mode_check"
rg -n '\$installed_home/scripts/chimera\.sh" -uninstall' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_bootstrap_uninstall_path"
rg -n 'legacy runtime bug where the installed control path reports' "$ROOT_DIR/scripts/release_bundle_install_contract_smoke.sh" >/dev/null || fail "release_bundle_missing_legacy_uninstall_regression_case"
rg -n 'datapath_apply=\$systemd_datapath_apply_status' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_start_missing_systemd_apply_status_output"
rg -n 'datapath_apply=\$direct_datapath_apply_status' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_start_missing_apply_status_output"
rg -n 'GITVERS_BOOTSTRAP_URLS_FILE' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_missing_gitvers_sources_file"
rg -n 'installer_gate_prepare_gitvers_bootstrap_sources' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_missing_gitvers_sources_seed"
rg -n 'CHIMERA_NFT_PRIVILEGE_MODE' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_missing_nft_privilege_mode"
rg -n 'local peer_listen="\$\{CHIMERA_PEER_EGRESS_PEER_LISTEN:-\$\{4:-0\.0\.0\.0:0\}\}"' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_peer_listen_env_not_respected"
rg -n 'local local_listen="\$\{CHIMERA_PEER_EGRESS_LOCAL_LISTEN:-\$\{5:-127\.0\.0\.1:0\}\}"' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_local_listen_env_not_respected"
rg -n '^  transparent_uid="\$\(id -u\)"$' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_transparent_uid_not_current_user"
rg -n '^  transparent_gid="\$\(id -g\)"$' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_transparent_gid_not_current_user"
rg -n 'CHIMERA_TRANSPARENT_RUNTIME_UID:-\$transparent_uid' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_transparent_uid_default_not_current_user"
rg -n 'CHIMERA_TRANSPARENT_RUNTIME_GID:-\$transparent_gid' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_transparent_gid_default_not_current_user"
if rg -n 'CHIMERA_TRANSPARENT_RUNTIME_UID.*:-0|CHIMERA_TRANSPARENT_RUNTIME_GID.*:-0' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null; then
  fail "installer_transparent_runtime_uid_gid_must_not_default_to_zero"
fi
rg -n 'prepare_transparent_runtime_env' "$ROOT_DIR/scripts/chimera-runner.sh" >/dev/null || fail "runner_missing_transparent_nft_mode_mapping"
rg -n 'CHIMERA_NFT_PRIVILEGE_MODE="sudo"' "$ROOT_DIR/scripts/chimera-runner.sh" >/dev/null || fail "runner_missing_legacy_sudo_to_nft_mapping"
if rg -n 'exec sudo|sudo -n env|sudo -n bash' "$ROOT_DIR/scripts/chimera-runner.sh" >/dev/null; then
  fail "runner_contains_broad_sudo_reexec"
fi
rg -n 'NftPrivilegeMode::Sudo' "$ROOT_DIR/crates/chimera-capture/src/nft_exec.rs" >/dev/null || fail "capture_missing_nft_sudo_mode"
if rg -n 'Command::new\("nft"\)' "$ROOT_DIR/crates/chimera-capture/src/bin/chimera-transparent-runtime.rs" "$ROOT_DIR/crates/chimera-capture/src/bin/chimera-transparent-rules.rs" >/dev/null; then
  fail "transparent_runtime_uses_path_nft_directly"
fi
rg -n 'case_legacy_runner_sudo_maps_to_nft_sudo_mode' "$ROOT_DIR/scripts/chimera_runner_sudo_contract_smoke.sh" >/dev/null || fail "runner_sudo_smoke_missing_legacy_mapping_case"
rg -n 'case_sudo_flag_does_not_apply_to_other_targets' "$ROOT_DIR/scripts/chimera_runner_sudo_contract_smoke.sh" >/dev/null || fail "runner_sudo_smoke_missing_target_scope_case"
rg -n 'case_runner_contains_no_sudo_reexec' "$ROOT_DIR/scripts/chimera_runner_sudo_contract_smoke.sh" >/dev/null || fail "runner_sudo_smoke_missing_no_reexec_case"
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
rg -n 'chimera-release/bin/chimera-node' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_node_binary_content_guard"
rg -n 'chimera-release/scripts/install_release\\.sh' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_update_installer_content_guard"
rg -n 'chimera-release/scripts/chimera-update\\.sh' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_update_module_content_guard"
rg -n 'mesh_control_plane_env_from_preflight\\.sh' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_mesh_control_plane_env_writer"
rg -n 'mesh_bootstrap\\.env\\.example' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_mesh_bootstrap_example_guard"
rg -n -F 'mesh-node\.example\.conf' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_mesh_node_example_guard"
rg -n -F 'chimera-node.service' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_node_unit_guard"
rg -n -F 'chimera-datapath.service' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_datapath_unit_guard"
rg -n -F 'build_bin chimera-gateway chimera-node' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_node_binary_build_guard"
rg -n -F 'target/release/chimera-node' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_node_binary_source_guard"
! rg -n -F 'target/release/chimera-gateway' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_uses_legacy_gateway_binary_source"
rg -n -F 'chimera-client\.service' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_legacy_client_unit_exclusion_guard"
rg -n -F 'chimera-gateway\.service' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_legacy_gateway_unit_exclusion_guard"
rg -n -F 'chimera-release/bin/chimera-gateway' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_legacy_gateway_binary_exclusion_guard"
rg -n -F 'chimera-release/scripts/chimera_runtime_bootstrap' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_legacy_third_party_bootstrap_exclusion_guard"
if rg -n 'cp -p .*\$\{ROOT_DIR\}/scripts/chimera_runtime_bootstrap\.sh' "$ROOT_DIR/scripts/build_release.sh" >/dev/null; then
  fail "release_build_copies_legacy_third_party_runtime_bootstrap"
fi
rg -n -F 'upstream_proxy.env.example' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_legacy_proxy_exclusion_guard"
rg -n -F 'client.example.conf' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_legacy_client_exclusion_guard"
rg -n -F 'gateway.example.conf' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_legacy_gateway_exclusion_guard"
rg -n -F 'chimera-app-routes.example.conf' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_app_route_exclusion_guard"
rg -n -F 'mesh_launch_preflight.side_a.env.example' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_side_a_fixture_exclusion_guard"
rg -n -F 'mesh_launch_preflight.side_b.env.example' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_side_b_fixture_exclusion_guard"
rg -n '^[[:space:]]*--listen 0\.0\.0\.0:0' "$ROOT_DIR/docs/OPERATIONS.md" >/dev/null || fail "operations_missing_peer_release_auto_port_example"
rg -n '^[[:space:]]*--base-url http://node\.example' "$ROOT_DIR/docs/OPERATIONS.md" >/dev/null || fail "operations_missing_peer_release_base_url_example"
rg -n '^[[:space:]]*--state-file "\$\{XDG_CACHE_HOME:-\$HOME/\.cache\}/chimera/peer-update\.state\.json"' "$ROOT_DIR/docs/OPERATIONS.md" >/dev/null || fail "operations_missing_peer_release_state_file_example"
rg -n 'ServeReleaseOptions::from_args' "$ROOT_DIR/crates/chimera-bootstrap/src/main.rs" >/dev/null || fail "bootstrap_missing_peer_release_arg_parser"
rg -n 'listen: listen\.unwrap_or_else\(\|\| "0\.0\.0\.0:0"\.to_string\(\)\)' "$ROOT_DIR/crates/chimera-bootstrap/src/peer_update/serve_args.rs" >/dev/null || fail "bootstrap_peer_release_auto_port_default_missing"
rg -n 'CHIMERA_PEER_UPDATE_BASE_URL' "$ROOT_DIR/crates/chimera-bootstrap/src/peer_update/serve_args.rs" >/dev/null || fail "bootstrap_peer_release_base_url_env_missing"
rg -n 'CHIMERA_PEER_UPDATE_STATE_FILE' "$ROOT_DIR/crates/chimera-bootstrap/src/peer_update/serve_args.rs" >/dev/null || fail "bootstrap_peer_release_state_file_env_missing"
rg -n 'advertised_base_url\(public_base_url, bound_addr\)' "$ROOT_DIR/crates/chimera-bootstrap/src/peer_update/server.rs" >/dev/null || fail "bootstrap_peer_release_bound_port_base_url_missing"
rg -n 'write_peer_update_state_file' "$ROOT_DIR/crates/chimera-bootstrap/src/peer_update/server.rs" >/dev/null || fail "bootstrap_peer_release_state_file_writer_missing"
rg -n 'update_bootstrap_url.*as_deref' "$ROOT_DIR/crates/chimera-cli/src/mesh_cli/nodes_cmd/advertise.rs" >/dev/null || fail "mesh_advertise_update_bootstrap_url_missing"
rg -n 'CHIMERA_PEER_UPDATE_STATE_FILE' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "mesh_control_missing_peer_update_state_env"
rg -n '\.sha256' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_checksum_output"
rg -n 'sha256sum -c "\$\{LATEST_CHECKSUM_NAME\}"' "$ROOT_DIR/scripts/build_release.sh" >/dev/null || fail "release_build_missing_checksum_self_verify"
[[ -f "$ROOT_DIR/.github/workflows/release.yml" ]] || fail "github_release_workflow_missing"
rg -n 'gh release create "\$RELEASE_TAG"' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_create_missing"
rg -n 'target/chimera\.sh' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_bootstrap_asset"
rg -n 'target/chimera-pq-release\.tar\.gz' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_archive_asset"
rg -n 'target/chimera-pq-release\.tar\.gz\.sha256' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_checksum_asset"
rg -n 'chimera-release/bin/chimera-bootstrap' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_bootstrap_binary_bundle_guard"
rg -n 'chimera-release/bin/chimera-node' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_node_binary_bundle_guard"
rg -n 'chimera-release/scripts/chimera-control\\.sh' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_control_script_bundle_guard"
rg -n 'chimera-release/scripts/chimera-runner\\.sh' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_runner_script_bundle_guard"
rg -n -F 'chimera-release/scripts/install_release\.sh' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_update_installer_bundle_guard"
rg -n 'chimera-release/scripts/chimera-update\.sh' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_update_module_bundle_guard"
rg -n -F 'chimera-release/scripts/mesh_control_plane_env_from_preflight\.sh' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_mesh_control_plane_env_writer_bundle_guard"
rg -n -F 'chimera-release/configs/mesh-node\.example\.conf' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_mesh_node_example_bundle_guard"
rg -n -F 'chimera-release/deploy/systemd-user/chimera-node\.service' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_node_unit_bundle_guard"
rg -n -F 'chimera-release/deploy/systemd-user/chimera-datapath\.service' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_datapath_unit_bundle_guard"
rg -n -F 'chimera-release/configs/client\.example\.conf' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_legacy_client_exclusion_guard"
rg -n -F 'chimera-release/configs/gateway\.example\.conf' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_legacy_gateway_exclusion_guard"
rg -n -F 'chimera-release/deploy/systemd-user/chimera-client\.service' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_legacy_client_unit_exclusion_guard"
rg -n -F 'chimera-release/deploy/systemd-user/chimera-gateway\.service' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_legacy_gateway_unit_exclusion_guard"
rg -n -F 'chimera-release/bin/chimera-gateway' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_legacy_gateway_binary_exclusion_guard"
rg -n -F 'chimera-release/scripts/chimera_runtime_bootstrap\.sh' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_legacy_third_party_bootstrap_exclusion_guard"
rg -n 'bash scripts/product_language_guard\.sh' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_product_language_guard"
rg -n 'git verify-tag --raw -v "\$\{release_tag\}"' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_gpg_tag_verification"
rg -n 'CHIMERA_RELEASE_GPG_PUBLIC_KEY' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_gpg_public_key_import"
rg -n 'CHIMERA_RELEASE_GPG_FINGERPRINT' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_gpg_fingerprint_check"
rg -n 'immutable release policy forbids asset replacement' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_missing_immutable_asset_guard"
if rg -n 'delete-asset|--clobber' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null; then
  fail "github_release_mutable_asset_path_present"
fi
rg -n 'gh release view --json tagName' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_latest_verification_missing"
rg -n 'release assets do not match required set' "$ROOT_DIR/.github/workflows/release.yml" >/dev/null || fail "github_release_asset_set_guard_missing"
rg -n 'configure_peer_egress_env "node"' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_not_weave_node_first"
rg -n 'INSTALL_NODE_ROLE="\$\(normalize_install_node_role "\$\{CHIMERA_INSTALL_NODE_ROLE:-node\}"\)"' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_default_role_not_normalized_node"
rg -n 'normalize_install_role "\$\(tr -d' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "update_install_role_file_not_normalized"
rg -n -F 'client|server|gateway) echo "node"' "$ROOT_DIR/scripts/chimera-update.sh" >/dev/null || fail "update_legacy_roles_not_normalized_to_node"
rg -n 'CHIMERA_PEER_EGRESS_TRANSIT_LANE_BINDINGS_FILE' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_missing_transit_lane_bindings_env"
rg -n 'shell_quote_env_value' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_missing_shell_safe_env_writer"
rg -n 'shell_quote_env_value' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_missing_shell_safe_env_writer"
rg -n 'shell_quote_env_value' "$ROOT_DIR/scripts/mesh_control_plane_env_from_preflight.sh" >/dev/null || fail "mesh_control_plane_env_writer_not_shell_safe"
rg -n 'missing_route_binding_id' "$ROOT_DIR/scripts/mesh_control_plane_env_from_preflight.sh" >/dev/null || fail "mesh_control_plane_env_writer_missing_route_binding_guard"
rg -n 'mesh-control-plane\.env' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_missing_default_mesh_control_plane_env"
rg -n 'mesh-bind-control-plane' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_missing_mesh_bind_control_plane_command"
rg -n 'mesh-bind-control-plane' "$ROOT_DIR/scripts/chimera-sh" >/dev/null || fail "launcher_missing_mesh_bind_control_plane_command"
rg -n 'publish_peer_egress_transit_lane_bindings_from_control_plane.*strict' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_missing_strict_transit_lane_binding_publish"
rg -n 'validate_mesh_control_plane_env_file_for_source' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_missing_control_plane_env_source_guard"
rg -n 'mesh_control_plane_env_smoke' "$ROOT_DIR/justfile" >/dev/null || fail "justfile_missing_mesh_control_plane_env_smoke"
rg -n 'quoted_value="\$\(shell_quote_env_value "\$key" "\$value"\)"' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_upsert_env_not_shell_safe"
rg -n 'quoted_value="\$\(shell_quote_env_value "\$key" "\$value"\)"' "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null || fail "control_upsert_env_not_shell_safe"
rg -n '^normalize_node_connect_addr\(\) \{$' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_missing_node_connect_normalizer"
rg -n '^materialize_node_runtime_config\(\) \{$' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_missing_node_config_materializer"
rg -n 'CHIMERA_NODE_SERVER_NAME' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_missing_node_server_name_override"
rg -n 'CHIMERA_NODE_LISTEN_ADDR' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null || fail "installer_missing_node_listen_addr_override"
rg -n 'tcp://node\\\.mesh\\\.invalid:18142' "$ROOT_DIR/scripts/chimera_installer_gate.sh" >/dev/null || fail "installer_gate_missing_tcp_materialization_contract"
if rg -n 'awk -v .*quoted_value|awk -v .*line=' "$ROOT_DIR/scripts/install_desktop_control.sh" "$ROOT_DIR/scripts/chimera-control.sh" >/dev/null; then
  fail "shell_quoted_env_writer_uses_awk_v"
fi
rg -n 'case_auto_update_preserves_bound_transit_env' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "update_smoke_missing_bound_transit_env_preservation_case"
rg -n 'case_peer_egress_env_shell_quotes_lane_bindings_path' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "update_smoke_missing_peer_env_shell_quote_case"
rg -n 'case_auto_update_preserves_quoted_lane_bindings_env' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "update_smoke_missing_quoted_lane_bindings_preservation_case"
rg -n 'case_peer_token_stays_in_private_peer_env' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "update_smoke_missing_private_peer_token_case"
rg -n 'mesh bootstrap env leaked peer token' "$ROOT_DIR/scripts/chimera_update_contract_smoke.sh" >/dev/null || fail "update_smoke_missing_bootstrap_token_leak_guard"
if rg -n 'configure_peer_egress_env "(side_a|side_b)"' "$ROOT_DIR/scripts/install_desktop_control.sh" >/dev/null; then
  fail "installer_writes_legacy_peer_egress_role"
fi
if rg -n 'CHIMERA_SIDE_[AB]|SIDE_A|SIDE_B|side_a|side_b' \
  "$ROOT_DIR/scripts/install_desktop_control.sh" \
  "$ROOT_DIR/scripts/chimera-update.sh" \
  "$ROOT_DIR/scripts/chimera-control.sh" \
  "$ROOT_DIR/scripts/chimera-sh" \
  "$ROOT_DIR/scripts/chimera.sh" \
  "$ROOT_DIR/scripts/install_release.sh" >/dev/null; then
  fail "shipped_scripts_contain_stand_role_marker"
fi
rg -n '"node" \| "weave-node" => Ok\(Mode::Node\)' "$ROOT_DIR/crates/chimera-carrier/src/peer_egress/options_mode.rs" >/dev/null || fail "peer_egress_missing_node_mode"
rg -n 'Mode::Node => node::run_node' "$ROOT_DIR/crates/chimera-carrier/src/bin/chimera-peer-egress.rs" >/dev/null || fail "peer_egress_binary_not_dispatching_node_mode"
rg -n 'remote release checksum is unavailable' "$ROOT_DIR/scripts/install_release.sh" >/dev/null || fail "install_release_url_checksum_not_required"
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
run_installer_configured_node_materialization_smoke
run_installer_unconfigured_node_template_smoke
run_installer_env_contract_smoke

echo "installer_gate=pass"
