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
  apt-get|dnf|yum|pacman|mkdir|install|rm|modprobe|visudo)
    exit 0
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
cat >"$config/chimera/upstream_proxy.env" <<'EOF'
CHIMERA_PEER_EGRESS_TOKEN=stale-token-from-previous-release
EOF
chmod 600 "$config/chimera/upstream_proxy.env"

write_fake_systemctl "$fake_bin/systemctl"
write_fake_sudo "$fake_bin/sudo"
write_fake_ip "$fake_bin/ip"
write_fake_nft "$fake_bin/nft"
write_forbidden_cargo "$fake_bin/cargo" "$cargo_marker"

expected_version="$(version_from_archive "$ARCHIVE")"
[[ "$expected_version" =~ ^[0-9]+[.][0-9]+[.][0-9]+$ ]] || fail "bad_release_version"

(cd "$(dirname "$ARCHIVE")" && sha256sum -c "$(basename "$CHECKSUM")" >/dev/null)

tar -tzf "$ARCHIVE" >"$tmp_dir/contents.txt"
for required in \
  chimera-release/.chimera_release_version \
  chimera-release/bin/chimera-cli \
  chimera-release/bin/chimera-gateway \
  chimera-release/bin/chimera-peer-egress \
  chimera-release/bin/chimera-transparent-runtime \
  chimera-release/bin/chimera-bootstrap \
  chimera-release/scripts/install_release.sh \
  chimera-release/scripts/chimera.sh \
  chimera-release/scripts/chimera-sh \
  chimera-release/scripts/chimera-update.sh \
  chimera-release/scripts/chimera_runtime_bootstrap.sh \
  chimera-release/configs/upstream_proxy.env.example
do
  rg -qx "$required" "$tmp_dir/contents.txt" || fail "archive_missing:${required}"
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

for executable in \
  "$installed_home/bin/chimera-cli" \
  "$installed_home/bin/chimera-gateway" \
  "$installed_home/bin/chimera-peer-egress" \
  "$installed_home/bin/chimera-transparent-runtime" \
  "$installed_home/bin/chimera-bootstrap" \
  "$installed_home/scripts/chimera-sh" \
  "$installed_home/scripts/chimera.sh" \
  "$installed_home/scripts/chimera-update.sh"
do
  [[ -x "$executable" ]] || fail "not_executable:${executable##*/}"
done

[[ -f "$installed_home/configs/upstream_proxy.env.example" ]] || fail "installed_upstream_proxy_example_missing"
rg -q '^CHIMERA_MESH_NODES_DISCOVERY_URL=' "$installed_home/configs/upstream_proxy.env.example" || fail "discovery_url_missing"
rg -q '^CHIMERA_MESH_NODES_DISCOVERY_PUBKEY=' "$installed_home/configs/upstream_proxy.env.example" || fail "discovery_pubkey_missing"
rg -q '^CHIMERA_MESH_NODES_PROBE_TIMEOUT_MS=' "$installed_home/configs/upstream_proxy.env.example" || fail "discovery_timeout_missing"
rg -q '^CHIMERA_MESH_NODES_DISCOVERY_URL=' "$config/chimera/upstream_proxy.env" || fail "installed_upstream_env_discovery_missing"
if rg -q '^CHIMERA_UPSTREAM_(USER|HOST|PASS)=' "$config/chimera/upstream_proxy.env"; then
  fail "installed_upstream_env_placeholder_credentials"
fi
if rg -q '^CHIMERA_PEER_EGRESS_TOKEN=' "$config/chimera/upstream_proxy.env"; then
  fail "installed_upstream_env_peer_token_leak"
fi
upstream_env_mode="$(stat -c '%a' "$config/chimera/upstream_proxy.env")"
[[ "$upstream_env_mode" == "600" ]] || fail "installed_upstream_env_mode:${upstream_env_mode}"

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

[[ "$doctor_rc" -eq 0 ]] || {
  cat "$doctor_log" >&2
  fail "doctor_failed"
}
rg -q '^doctor_status=ok$' "$doctor_log" || fail "doctor_status_missing"
rg -q '"secrets":"<redacted>"' "$installed_home/docs/doctor_latest.json" || fail "doctor_redaction_missing"
rg -q '"network_state":"not_modified"' "$installed_home/docs/doctor_latest.json" || fail "doctor_network_state_modified"

echo "release_bundle_install_contract_smoke=pass version=${expected_version} install_without_cargo_ok=true artifact_checksum_ok=true config_validate_ok=true diagnostics_redacted_ok=true"
