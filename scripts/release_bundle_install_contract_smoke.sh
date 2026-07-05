#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARCHIVE="${1:-${ROOT_DIR}/target/chimera-pq-release.tar.gz}"
CHECKSUM="${2:-${ROOT_DIR}/target/chimera-pq-release.tar.gz.sha256}"

fail() {
  echo "release_bundle_install_contract_smoke=fail reason=$1" >&2
  exit 1
}

require_file() {
  local path="${1:?path_required}"
  [[ -s "$path" ]] || fail "missing_file:${path}"
}

write_fake_systemctl() {
  local dest="${1:?dest_required}"
  cat >"$dest" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  --user)
    shift
    ;;
esac
case "${1:-}" in
  show-environment|daemon-reload)
    exit 0
    ;;
  start|stop|restart|enable|disable|is-active|list-units|list-unit-files)
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
EOF
  chmod +x "$dest"
}

write_fake_sudo() {
  local dest="${1:?dest_required}"
  cat >"$dest" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "-n" ]]; then
  shift
fi
case "${1:-}" in
  ip)
    shift
    case "${1:-}" in
      -Version|rule|tuntap|link)
        exit 0
        ;;
    esac
    ;;
  nft)
    exit 0
    ;;
  apt-get|dnf|yum|pacman|mkdir|install|modprobe|visudo)
    exit 0
    ;;
  rm)
    CHIMERA_FAKE_SUDO_CALL=1 exec "$@"
    ;;
esac
exec "$@"
EOF
  chmod +x "$dest"
}

write_fake_ip() {
  local dest="${1:?dest_required}"
  cat >"$dest" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  -Version|rule|tuntap|link)
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
EOF
  chmod +x "$dest"
}

write_fake_nft() {
  local dest="${1:?dest_required}"
  cat >"$dest" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  --version|list|delete)
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
EOF
  chmod +x "$dest"
}

write_forbidden_cargo() {
  local dest="${1:?dest_required}"
  local marker="${2:?marker_required}"
  cat >"$dest" <<EOF
#!/usr/bin/env bash
printf '%s\n' cargo_forbidden >>"${marker}"
exit 99
EOF
  chmod +x "$dest"
}

write_fake_rm() {
  local dest="${1:?dest_required}"
  cat >"$dest" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${CHIMERA_FAKE_SUDO_CALL:-0}" == "1" ]]; then
  exec /bin/rm "$@"
fi
for arg in "$@"; do
  if [[ "$arg" == *".chimera-previous."* ]]; then
    echo "rm: cannot remove '$arg': Permission denied" >&2
    exit 1
  fi
done
exec /bin/rm "$@"
EOF
  chmod +x "$dest"
}

version_from_archive() {
  local archive="${1:?archive_required}"
  tar -xOf "$archive" chimera-release/.chimera_release_version | tr -d '[:space:]'
}

run_bundle_scan() {
  local patterns_file="${1:?patterns_file_required}"
  local reason="${2:?reason_required}"
  [[ -s "$patterns_file" ]] || return 0
  if rg -n --hidden --glob '!*.sha256' -f "$patterns_file" "$scan_dir/chimera-release" >/dev/null; then
    fail "$reason"
  fi
}

require_file "$ARCHIVE"
require_file "$CHECKSUM"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

fake_bin="$tmp_dir/bin"
home="$tmp_dir/home"
cache="$tmp_dir/cache"
config="$tmp_dir/config"
data="$tmp_dir/data"
runtime="$tmp_dir/runtime"
cargo_marker="$tmp_dir/cargo_called"
install_log="$tmp_dir/install.log"
doctor_log="$tmp_dir/doctor.log"
mkdir -p "$fake_bin" "$home" "$cache" "$config" "$data" "$runtime"
: >"$cargo_marker"
mkdir -p "$config/chimera"
cat >"$config/chimera/mesh_bootstrap.env" <<'EOF'
CHIMERA_PEER_EGRESS_TOKEN=stale-token-from-previous-release
EOF
chmod 600 "$config/chimera/mesh_bootstrap.env"

write_fake_systemctl "$fake_bin/systemctl"
write_fake_sudo "$fake_bin/sudo"
write_fake_ip "$fake_bin/ip"
write_fake_nft "$fake_bin/nft"
write_fake_rm "$fake_bin/rm"
write_forbidden_cargo "$fake_bin/cargo" "$cargo_marker"

expected_version="$(version_from_archive "$ARCHIVE")"
[[ "$expected_version" =~ ^[0-9]+[.][0-9]+[.][0-9]+$ ]] || fail "bad_release_version"

mkdir -p "$tmp_dir/chimera-home/docs"
printf '%s\n' '{"status":"old"}' >"$tmp_dir/chimera-home/docs/runtime_state_latest.json"

(cd "$(dirname "$ARCHIVE")" && sha256sum -c "$(basename "$CHECKSUM")" >/dev/null)

tar -tzf "$ARCHIVE" >"$tmp_dir/contents.txt"
for required in \
  chimera-release/.chimera_release_version \
  chimera-release/bin/chimera-cli \
  chimera-release/bin/chimera-node \
  chimera-release/bin/chimera-peer-egress \
  chimera-release/bin/chimera-transparent-runtime \
  chimera-release/bin/chimera-bootstrap \
  chimera-release/scripts/install_release.sh \
  chimera-release/scripts/chimera.sh \
  chimera-release/scripts/chimera-sh \
  chimera-release/scripts/chimera-control.sh \
  chimera-release/scripts/chimera-runner.sh \
  chimera-release/scripts/chimera-update.sh \
  chimera-release/scripts/chimera-update-runtime-state.sh \
  chimera-release/configs/mesh-node.example.conf \
  chimera-release/configs/mesh_bootstrap.env.example \
  chimera-release/configs/update_gitvers_bootstrap_urls.example.list \
  chimera-release/deploy/systemd-user/chimera-runtime.service \
  chimera-release/deploy/systemd-user/chimera-node.service \
  chimera-release/deploy/systemd-user/chimera-datapath.service \
  chimera-release/deploy/systemd-user/chimera-site-watch.service
do
  rg -qx "$required" "$tmp_dir/contents.txt" || fail "archive_missing:${required}"
done
for forbidden in \
  'chimera-release/bin/chimera-gateway' \
  'chimera-release/configs/upstream_proxy\.env\.example' \
  'chimera-release/configs/client\.example\.conf' \
  'chimera-release/configs/gateway\.example\.conf' \
  'chimera-release/deploy/systemd-user/chimera-client\.service' \
  'chimera-release/deploy/systemd-user/chimera-gateway\.service' \
  'chimera-release/configs/chimera-app-routes\.conf' \
  'chimera-release/configs/chimera-app-routes\.example\.conf' \
  'chimera-release/configs/policy\.runtime\.conf' \
  'chimera-release/configs/mesh_launch_preflight\.side_[ab]\.env\.example' \
  'chimera-release/scripts/chimera_runtime_bootstrap\.sh'
do
  if rg -qx "$forbidden" "$tmp_dir/contents.txt"; then
    fail "archive_forbidden_config_present:${forbidden}"
  fi
done

scan_dir="$tmp_dir/scan"
mkdir -p "$scan_dir"
tar -xzf "$ARCHIVE" -C "$scan_dir"

default_patterns="$tmp_dir/default_forbidden_patterns.txt"
cat >"$default_patterns" <<'EOF'
BEGIN (RSA |OPENSSH |EC |DSA |PRIVATE )?PRIVATE KEY
PrivateKey[[:space:]]*=
EOF
run_bundle_scan "$default_patterns" "release_bundle_secret_literal_found"

if [[ -n "${CHIMERA_RELEASE_BUNDLE_FORBIDDEN_PATTERNS:-}" ]]; then
  env_patterns="$tmp_dir/env_forbidden_patterns.txt"
  printf '%s\n' "$CHIMERA_RELEASE_BUNDLE_FORBIDDEN_PATTERNS" >"$env_patterns"
  run_bundle_scan "$env_patterns" "release_bundle_secret_or_stand_literal_found"
fi

if [[ -n "${CHIMERA_RELEASE_BUNDLE_FORBIDDEN_PATTERNS_FILE:-}" ]]; then
  [[ -r "$CHIMERA_RELEASE_BUNDLE_FORBIDDEN_PATTERNS_FILE" ]] || fail "forbidden_patterns_file_unreadable"
  run_bundle_scan "$CHIMERA_RELEASE_BUNDLE_FORBIDDEN_PATTERNS_FILE" "release_bundle_secret_or_stand_literal_found"
fi

set +e
PATH="$fake_bin:/usr/sbin:/usr/bin:/sbin:/bin" \
HOME="$home" \
XDG_CACHE_HOME="$cache" \
XDG_CONFIG_HOME="$config" \
XDG_DATA_HOME="$data" \
XDG_RUNTIME_DIR="$runtime" \
CHIMERA_HOME="$tmp_dir/chimera-home" \
CHIMERA_LOCAL_BIN="$tmp_dir/local-bin" \
CHIMERA_ALLOW_LOCAL_RELEASE_SOURCE=1 \
CHIMERA_MESH_REMOTE_ENDPOINT="mesh.example.invalid:443" \
  timeout 60s bash "$ROOT_DIR/scripts/install_release.sh" "$ARCHIVE" "$CHECKSUM" >"$install_log" 2>&1
install_rc=$?
set -e

[[ "$install_rc" -eq 0 ]] || {
  cat "$install_log" >&2
  fail "install_failed"
}

if [[ -s "$cargo_marker" ]]; then
  cat "$install_log" >&2
  fail "cargo_invoked"
fi

installed_home="$tmp_dir/chimera-home"
[[ -d "$installed_home" ]] || fail "installed_home_missing"
[[ "$(tr -d '[:space:]' < "$installed_home/.chimera_release_version")" == "$expected_version" ]] || fail "installed_version_mismatch"
[[ -s "$installed_home/.chimera_release_bundle.sha256" ]] || fail "installed_bundle_sha_missing"
(cd "$installed_home/releases" && sha256sum -c chimera-pq-release.tar.gz.sha256 >/dev/null)
if find "$tmp_dir" -maxdepth 2 -type d -name '.chimera-previous.*' | grep -q .; then
  fail "previous_release_backup_not_cleaned"
fi

for executable in \
  "$installed_home/bin/chimera-cli" \
  "$installed_home/bin/chimera-node" \
  "$installed_home/bin/chimera-peer-egress" \
  "$installed_home/bin/chimera-transparent-runtime" \
  "$installed_home/bin/chimera-bootstrap" \
  "$installed_home/scripts/chimera-sh" \
  "$installed_home/scripts/chimera.sh" \
  "$installed_home/scripts/chimera-update.sh"
do
  [[ -x "$executable" ]] || fail "not_executable:${executable##*/}"
done

bootstrap_install_version_tracks_bundle_not_script_version() {
  local bootstrap_home="$tmp_dir/bootstrap-version-home"
  local bootstrap_local_bin="$tmp_dir/bootstrap-version-bin"
  local bootstrap_cache="$tmp_dir/bootstrap-version-cache"
  local bootstrap_config="$tmp_dir/bootstrap-version-config"
  local bootstrap_data="$tmp_dir/bootstrap-version-data"
  local bootstrap_runtime="$tmp_dir/bootstrap-version-runtime"
  local bootstrap_script="$tmp_dir/bootstrap-version-bootstrap.sh"
  local bootstrap_log="$tmp_dir/bootstrap-version-install.log"
  local installed_version=""

  cp "$installed_home/scripts/chimera.sh" "$bootstrap_script"
  sed -i 's/^VERSION=\"[^\"]*\"$/VERSION=\"9.9.9\"/' "$bootstrap_script"
  chmod +x "$bootstrap_script"
  mkdir -p "$bootstrap_home" "$bootstrap_local_bin" "$bootstrap_cache" "$bootstrap_config" "$bootstrap_data" "$bootstrap_runtime"

  set +e
  PATH="$fake_bin:/usr/sbin:/usr/bin:/sbin:/bin" \
  HOME="$home" \
  XDG_CACHE_HOME="$bootstrap_cache" \
  XDG_CONFIG_HOME="$bootstrap_config" \
  XDG_DATA_HOME="$bootstrap_data" \
  XDG_RUNTIME_DIR="$bootstrap_runtime" \
  CHIMERA_HOME="$bootstrap_home" \
  CHIMERA_LOCAL_BIN="$bootstrap_local_bin" \
  CHIMERA_RELEASE_ARCHIVE_URL="file://$ARCHIVE" \
  CHIMERA_RELEASE_CHECKSUM_URL="file://$CHECKSUM" \
    timeout 60s bash "$bootstrap_script" -install >"$bootstrap_log" 2>&1
  bootstrap_rc=$?
  set -e

  [[ "$bootstrap_rc" -eq 0 ]] || {
    cat "$bootstrap_log" >&2
    fail "bootstrap_install_version_tracks_bundle_not_script_version_failed"
  }

  installed_version="$(tr -d '[:space:]' < "$bootstrap_home/.chimera_release_version")"
  [[ "$installed_version" == "$expected_version" ]] || fail "bootstrap_install_version_tracks_script_not_bundle"
  rg -q "^chimera_install=ok version=${expected_version} " "$bootstrap_log" \
    || fail "bootstrap_install_output_version_not_from_bundle"
}

bootstrap_install_version_tracks_bundle_not_script_version

node_help="$tmp_dir/chimera-node-help.txt"
"$installed_home/bin/chimera-node" --help >"$node_help" 2>&1 || fail "node_help_failed"
rg -q '^Команды chimera-node:' "$node_help" || fail "node_help_missing_node_commands"
if rg -q 'chimera-gateway|Gateway doctor|gateway_config_file' "$node_help"; then
  fail "node_help_legacy_gateway_wording"
fi

[[ -f "$installed_home/configs/mesh_bootstrap.env.example" ]] || fail "installed_mesh_bootstrap_example_missing"
[[ -f "$installed_home/configs/mesh-node.example.conf" ]] || fail "installed_mesh_node_example_missing"
[[ -f "$installed_home/deploy/systemd-user/chimera-node.service" ]] || fail "installed_node_unit_missing"
[[ -f "$installed_home/deploy/systemd-user/chimera-datapath.service" ]] || fail "installed_datapath_unit_missing"
[[ -f "$installed_home/deploy/systemd-user/chimera-runtime.service" ]] || fail "installed_runtime_unit_missing"
[[ -f "$installed_home/deploy/systemd-user/chimera-site-watch.service" ]] || fail "installed_site_watch_unit_missing"
[[ ! -f "$installed_home/configs/upstream_proxy.env.example" ]] || fail "installed_legacy_upstream_proxy_example_present"
[[ ! -f "$installed_home/configs/client.example.conf" ]] || fail "installed_legacy_client_example_present"
[[ ! -f "$installed_home/configs/gateway.example.conf" ]] || fail "installed_legacy_gateway_example_present"
[[ ! -f "$installed_home/deploy/systemd-user/chimera-client.service" ]] || fail "installed_legacy_client_unit_present"
[[ ! -f "$installed_home/deploy/systemd-user/chimera-gateway.service" ]] || fail "installed_legacy_gateway_unit_present"
[[ ! -f "$installed_home/configs/chimera-app-routes.example.conf" ]] || fail "installed_legacy_app_routes_example_present"
[[ ! -f "$installed_home/scripts/chimera_runtime_bootstrap.sh" ]] || fail "installed_legacy_third_party_runtime_bootstrap_present"
gitvers_bootstrap_sources_file="$config/chimera/update_gitvers_bootstrap_urls.list"
[[ -f "$gitvers_bootstrap_sources_file" ]] || fail "installed_gitvers_bootstrap_sources_missing"
rg -q '^https://gitverse.ru/api/repos/ArtReg/chimera/raw/branch/main/chimera.sh$' "$gitvers_bootstrap_sources_file" || fail "installed_gitvers_bootstrap_sources_default_missing"
rg -q '^# CHIMERA_MESH_NODES_DISCOVERY_URL=' "$installed_home/configs/mesh_bootstrap.env.example" || fail "discovery_url_template_missing"
rg -q '^# CHIMERA_MESH_NODES_DISCOVERY_PUBKEY=' "$installed_home/configs/mesh_bootstrap.env.example" || fail "discovery_pubkey_template_missing"
rg -q '^# CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS=' "$installed_home/configs/mesh_bootstrap.env.example" || fail "discovery_timeout_template_missing"
rg -q '^# CHIMERA_MESH_NAMESPACE=cef-public$' "$installed_home/configs/mesh_bootstrap.env.example" || fail "mesh_namespace_template_missing"
rg -q '^# CHIMERA_MESH_TRAFFIC_PROFILE=high_speed_anonymous$' "$installed_home/configs/mesh_bootstrap.env.example" || fail "mesh_traffic_profile_template_missing"
rg -q '^# CHIMERA_MESH_REMOTE_PEER_SPEC=$' "$installed_home/configs/mesh_bootstrap.env.example" || fail "mesh_remote_peer_spec_template_missing"
if rg -q '^CHIMERA_UPSTREAM_(USER|HOST|PASS)=' "$config/chimera/mesh_bootstrap.env"; then
  fail "installed_mesh_bootstrap_env_upstream_credentials"
fi
if rg -q '^CHIMERA_PEER_EGRESS_TOKEN=' "$config/chimera/mesh_bootstrap.env"; then
  fail "installed_mesh_bootstrap_env_peer_token_leak"
fi
bootstrap_env_mode="$(stat -c '%a' "$config/chimera/mesh_bootstrap.env")"
[[ "$bootstrap_env_mode" == "600" ]] || fail "installed_mesh_bootstrap_env_mode:${bootstrap_env_mode}"
gitvers_bootstrap_sources_mode="$(stat -c '%a' "$gitvers_bootstrap_sources_file")"
[[ "$gitvers_bootstrap_sources_mode" == "600" ]] || fail "installed_gitvers_bootstrap_sources_mode:${gitvers_bootstrap_sources_mode}"

state_proof_log="$tmp_dir/state-proof.log"
route_status_log="$tmp_dir/route-status.log"
INSTALLED_STATE_PROOF_CASE_MARKERS=(
  installed_state_proof_missing_state
  installed_state_proof_invalid_not_rejected
  installed_state_proof_duplicate_field
  installed_state_proof_network_not_modified
  installed_state_proof_tun_not_applied
  installed_state_proof_route_not_applied
  installed_state_proof_dns_not_applied
  installed_state_proof_valid_not_accepted
)
INSTALLED_ROUTE_STATUS_CASE_MARKERS=(
  installed_route_status_without_proof
  installed_route_status_duplicate_field
  installed_route_status_network_not_modified
  installed_route_status_tun_not_applied
  installed_route_status_route_not_applied
  installed_route_status_dns_not_applied
  installed_route_status_valid_apply_without_flow_proof
  installed_route_status_stale_flow_proof
  installed_route_status_valid_flow_proof
)
INSTALLED_ROUTE_STATUS_OUTPUT_MARKERS=(
  datapath_mode=unknown
  datapath_apply=unverified
  datapath_proof=missing_state
  datapath_flow_proof=skipped_apply_unverified
  datapath_flow_proof=missing_flow_proof
  datapath_flow_proof=flow_stale
  datapath_mode=transparent
  datapath_apply=ok
  datapath_proof=ok
  datapath_flow_proof=ok
)

write_installed_state_case() {
  local case_name="${1:?case_name_required}"
  local payload="${2:-}"
  installed_state_file="$tmp_dir/installed-${case_name}.json"
  rm -f "$installed_state_file"
  if [[ "$payload" != "__missing__" ]]; then
    printf '%s\n' "$payload" >"$installed_state_file"
  fi
}

assert_installed_state_proof_case() {
  local case_name="${1:?case_name_required}"
  local payload="${2:-}"
  local expected_proof="${3:?expected_proof_required}"
  local rc

  write_installed_state_case "$case_name" "$payload"
  set +e
  "$installed_home/bin/chimera-cli" state proof --state-file "$installed_state_file" >"$state_proof_log" 2>&1
  rc=$?
  set -e
  if [[ "$expected_proof" == "ok" ]]; then
    [[ "$rc" -eq 0 ]] || fail "installed_state_proof_${case_name}_not_accepted"
  else
    [[ "$rc" -ne 0 ]] || fail "installed_state_proof_${case_name}_not_rejected"
  fi
  rg -qx "datapath_proof=$expected_proof" "$state_proof_log" || fail "installed_state_proof_${case_name}_${expected_proof}_missing"
}

write_installed_flow_case() {
  local state_file="${1:?state_file_required}"
  local payload="${2:-}"
  local touch_mode="${3:-}"
  local flow_file="${state_file}.flow.json"
  rm -f "$flow_file"
  if [[ -n "$payload" ]]; then
    printf '%s\n' "$payload" >"$flow_file"
    if [[ "$touch_mode" == "stale" ]]; then
      touch -d '10 minutes ago' "$flow_file"
    fi
  fi
}

assert_installed_route_status_case() {
  local case_name="${1:?case_name_required}"
  local state_file="${2:?state_file_required}"
  local expected_mode="${3:?expected_mode_required}"
  local expected_apply="${4:?expected_apply_required}"
  local expected_proof="${5:?expected_proof_required}"
  local expected_flow_proof="${6:?expected_flow_proof_required}"
  local rc

  set +e
  PATH="$fake_bin:/usr/sbin:/usr/bin:/sbin:/bin" \
  HOME="$home" \
  XDG_CACHE_HOME="$cache" \
  XDG_CONFIG_HOME="$config" \
  XDG_DATA_HOME="$data" \
  XDG_RUNTIME_DIR="$runtime" \
  STATE_FILE="$state_file" \
  NODE_CONFIG_FILE="$tmp_dir/missing-node.conf" \
  APP_ROUTES_FILE="$tmp_dir/app-routes.conf" \
  MANUAL_TRANSIT_DOMAINS_FILE="$tmp_dir/manual-transit.txt" \
  ADAPTIVE_DOMAINS_FILE="$tmp_dir/adaptive.txt" \
    timeout 20s "$installed_home/scripts/chimera-control.sh" route-status >"$route_status_log" 2>&1
  rc=$?
  set -e
  [[ "$rc" -eq 0 ]] || fail "installed_route_status_${case_name}_rc:${rc}"
  rg -qx "datapath_mode=$expected_mode" "$route_status_log" || fail "installed_route_status_${case_name}_missing_mode_${expected_mode}"
  rg -qx "datapath_apply=$expected_apply" "$route_status_log" || fail "installed_route_status_${case_name}_missing_apply_${expected_apply}"
  rg -qx "datapath_proof=$expected_proof" "$route_status_log" || fail "installed_route_status_${case_name}_missing_proof_${expected_proof}"
  rg -qx "datapath_flow_proof=$expected_flow_proof" "$route_status_log" || fail "installed_route_status_${case_name}_missing_flow_proof_${expected_flow_proof}"
  if [[ "$expected_mode" != "transparent" ]]; then
    ! rg -qx 'datapath_mode=transparent' "$route_status_log" || fail "installed_route_status_${case_name}_false_transparent"
  fi
  if [[ "$expected_apply" != "ok" ]]; then
    ! rg -qx 'datapath_apply=ok' "$route_status_log" || fail "installed_route_status_${case_name}_false_ok"
  fi
}

valid_state_payload='{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true}'
valid_flow_payload='{"status":"ok","kind":"chimera_datapath_flow_proof","flow_id":"flow#1","path_kind":"local_egress_via_secure_peer","transparent_flow_observed":true,"counter_delta_ok":true,"secure_peer_egress_observed":true,"secure_peer_bytes_delta_ok":true,"network_state":"modified"}'

installed_state_file=""
assert_installed_state_proof_case "missing_state" "__missing__" "missing_state"
missing_state_file="$installed_state_file"
assert_installed_route_status_case "without_proof" "$missing_state_file" "unknown" "unverified" "missing_state" "skipped_apply_unverified"

assert_installed_state_proof_case "invalid" "{not json" "state_invalid_json"
assert_installed_route_status_case "invalid" "$installed_state_file" "unknown" "unverified" "state_invalid_json" "skipped_apply_unverified"

assert_installed_state_proof_case \
  "duplicate_field" \
  '{"status":"down","status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true}' \
  "duplicate_field"
assert_installed_route_status_case "duplicate_field" "$installed_state_file" "unknown" "unverified" "duplicate_field" "skipped_apply_unverified"

assert_installed_state_proof_case \
  "network_not_modified" \
  '{"status":"up","network_state":"not_modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":true}' \
  "network_not_modified"
assert_installed_route_status_case "network_not_modified" "$installed_state_file" "unknown" "unverified" "network_not_modified" "skipped_apply_unverified"

assert_installed_state_proof_case \
  "tun_not_applied" \
  '{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":false,"route_applied":true,"dns_applied":true}' \
  "tun_not_applied"
assert_installed_route_status_case "tun_not_applied" "$installed_state_file" "unknown" "unverified" "tun_not_applied" "skipped_apply_unverified"

assert_installed_state_proof_case \
  "route_not_applied" \
  '{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":false,"dns_applied":true}' \
  "route_not_applied"
assert_installed_route_status_case "route_not_applied" "$installed_state_file" "unknown" "unverified" "route_not_applied" "skipped_apply_unverified"

assert_installed_state_proof_case \
  "dns_not_applied" \
  '{"status":"up","network_state":"modified","rollback_ready":true,"tun_applied":true,"route_applied":true,"dns_applied":false}' \
  "dns_not_applied"
assert_installed_route_status_case "dns_not_applied" "$installed_state_file" "unknown" "unverified" "dns_not_applied" "skipped_apply_unverified"

assert_installed_state_proof_case \
  "valid_apply_without_flow_proof" \
  "$valid_state_payload" \
  "ok"
valid_state_file="$installed_state_file"
assert_installed_route_status_case "valid_apply_without_flow_proof" "$valid_state_file" "unknown" "ok" "ok" "missing_flow_proof"

assert_installed_state_proof_case \
  "valid_apply_with_stale_flow_proof" \
  "$valid_state_payload" \
  "ok"
stale_flow_state_file="$installed_state_file"
write_installed_flow_case "$stale_flow_state_file" "$valid_flow_payload" "stale"
assert_installed_route_status_case "stale_flow_proof" "$stale_flow_state_file" "unknown" "ok" "ok" "flow_stale"

assert_installed_state_proof_case \
  "valid_apply_with_fresh_flow_proof" \
  "$valid_state_payload" \
  "ok"
valid_flow_state_file="$installed_state_file"
write_installed_flow_case "$valid_flow_state_file" "$valid_flow_payload" ""
assert_installed_route_status_case "valid_flow_proof" "$valid_flow_state_file" "transparent" "ok" "ok" "ok"

version_out="$("$tmp_dir/local-bin/chimera-sh" -version)"
[[ "$version_out" == "chimera-runtime ${expected_version}" ]] || fail "version_output_mismatch"

set +e
PATH="$fake_bin:/usr/sbin:/usr/bin:/sbin:/bin" \
HOME="$home" \
XDG_CACHE_HOME="$cache" \
XDG_CONFIG_HOME="$config" \
XDG_DATA_HOME="$data" \
XDG_RUNTIME_DIR="$runtime" \
  timeout 20s "$tmp_dir/local-bin/chimera-sh" -doctor >"$doctor_log" 2>&1
doctor_rc=$?
set -e

if [[ "$doctor_rc" -ne 0 && "$doctor_rc" -ne 2 ]]; then
  cat "$doctor_log" >&2
  fail "doctor_failed"
fi
if [[ "$doctor_rc" -eq 0 ]]; then
  rg -q '^doctor_status=ok$' "$doctor_log" || fail "doctor_status_missing"
else
  rg -q '^doctor_status=fail reason=node_endpoint_unconfigured$' "$doctor_log" || fail "doctor_missing_endpoint_diagnostic"
fi
rg -q '"secrets":"<redacted>"' "$installed_home/docs/doctor_latest.json" || fail "doctor_redaction_missing"
rg -q '"network_state":"not_modified"' "$installed_home/docs/doctor_latest.json" || fail "doctor_network_state_modified"

[[ -L "$tmp_dir/local-bin/chimera" ]] || fail "launcher_chimera_missing_before_uninstall"
[[ -L "$tmp_dir/local-bin/chimera.sh" ]] || fail "launcher_chimera_sh_missing_before_uninstall"
[[ -L "$tmp_dir/local-bin/chimera-sh" ]] || fail "launcher_chimera_dash_sh_missing_before_uninstall"
[[ -f "$config/systemd/user/chimera-runtime.service" ]] || fail "installed_user_runtime_unit_missing_before_uninstall"
[[ -f "$config/systemd/user/chimera-node.service" ]] || fail "installed_user_node_unit_missing_before_uninstall"
[[ -f "$config/systemd/user/chimera-datapath.service" ]] || fail "installed_user_datapath_unit_missing_before_uninstall"
[[ -f "$config/systemd/user/chimera-site-watch.service" ]] || fail "installed_user_site_watch_unit_missing_before_uninstall"
[[ -L "$config/systemd/user/default.target.wants/chimera-runtime.service" ]] || fail "installed_user_runtime_wants_missing_before_uninstall"
[[ ! -e "$config/systemd/user/default.target.wants/chimera-node.service" ]] || fail "installed_user_node_wants_should_be_absent"
[[ ! -e "$config/systemd/user/default.target.wants/chimera-datapath.service" ]] || fail "installed_user_datapath_wants_should_be_absent"
[[ ! -e "$config/systemd/user/default.target.wants/chimera-site-watch.service" ]] || fail "installed_user_site_watch_wants_should_be_absent"
[[ -f "$data/applications/chimera-control-gui.desktop" ]] || fail "installed_desktop_entry_missing_before_uninstall"
[[ -d "$config/chimera" ]] || fail "installed_config_dir_missing_before_uninstall"
[[ -d "$cache/chimera" ]] || fail "installed_cache_dir_missing_before_uninstall"

# Simulate the legacy runtime bug where the installed control path reports
# uninstall success without removing the release tree. The bootstrap script must
# still perform a full cleanup.
cat >"$installed_home/scripts/chimera-control.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  stop)
    exit 0
    ;;
  uninstall)
    echo "uninstall_status=ok"
    ;;
  *)
    exit 0
    ;;
esac
EOF
chmod +x "$installed_home/scripts/chimera-control.sh"

uninstall_log="$tmp_dir/uninstall.log"
set +e
PATH="$fake_bin:/usr/sbin:/usr/bin:/sbin:/bin" \
HOME="$home" \
XDG_CACHE_HOME="$cache" \
XDG_CONFIG_HOME="$config" \
XDG_DATA_HOME="$data" \
XDG_RUNTIME_DIR="$runtime" \
NFT_BIN="$fake_bin/nft" \
CHIMERA_ALLOW_TEST_NFT_BIN=1 \
  timeout 20s "$installed_home/scripts/chimera.sh" -uninstall >"$uninstall_log" 2>&1
uninstall_rc=$?
set -e

[[ "$uninstall_rc" -eq 0 ]] || {
  cat "$uninstall_log" >&2
  fail "uninstall_failed"
}
rg -q '^uninstall_status=ok$' "$uninstall_log" || fail "uninstall_status_missing"
[[ ! -e "$installed_home" && ! -L "$installed_home" ]] || fail "installed_home_present_after_uninstall"
[[ ! -e "$tmp_dir/local-bin/chimera" && ! -L "$tmp_dir/local-bin/chimera" ]] || fail "launcher_chimera_present_after_uninstall"
[[ ! -e "$tmp_dir/local-bin/chimera.sh" && ! -L "$tmp_dir/local-bin/chimera.sh" ]] || fail "launcher_chimera_sh_present_after_uninstall"
[[ ! -e "$tmp_dir/local-bin/chimera-sh" && ! -L "$tmp_dir/local-bin/chimera-sh" ]] || fail "launcher_chimera_dash_sh_present_after_uninstall"
[[ ! -e "$config/systemd/user/chimera-runtime.service" ]] || fail "user_runtime_unit_present_after_uninstall"
[[ ! -e "$config/systemd/user/chimera-node.service" ]] || fail "user_node_unit_present_after_uninstall"
[[ ! -e "$config/systemd/user/chimera-datapath.service" ]] || fail "user_datapath_unit_present_after_uninstall"
[[ ! -e "$config/systemd/user/chimera-site-watch.service" ]] || fail "user_site_watch_unit_present_after_uninstall"
[[ ! -e "$config/systemd/user/default.target.wants/chimera-runtime.service" ]] || fail "user_runtime_wants_present_after_uninstall"
[[ ! -e "$config/systemd/user/chimera-gateway.service" ]] || fail "legacy_user_gateway_unit_present_after_uninstall"
[[ ! -e "$config/systemd/user/chimera-client.service" ]] || fail "legacy_user_client_unit_present_after_uninstall"
[[ ! -e "$data/applications/chimera-control-gui.desktop" ]] || fail "desktop_entry_present_after_uninstall"
[[ ! -e "$data/applications/chimera-control.desktop" ]] || fail "legacy_desktop_entry_present_after_uninstall"
[[ ! -e "$config/chimera" ]] || fail "config_dir_present_after_uninstall"
[[ ! -e "$cache/chimera" ]] || fail "cache_dir_present_after_uninstall"
if find "$tmp_dir" -maxdepth 2 -type d -name '.chimera-previous.*' | grep -q .; then
  fail "previous_release_backup_present_after_uninstall"
fi

echo "release_bundle_install_contract_smoke=pass version=${expected_version} install_without_cargo_ok=true artifact_checksum_ok=true installed_state_proof_ok=true installed_route_status_contract_ok=true diagnostic_contract_ok=true diagnostics_redacted_ok=true uninstall_cleanup_ok=true"
