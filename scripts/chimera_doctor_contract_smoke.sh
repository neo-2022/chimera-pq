#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  echo "chimera_doctor_contract_smoke: $1" >&2
  exit 1
}

make_install_root() {
  local install_root="${1:?install_root_required}"
  mkdir -p "$install_root/scripts" "$install_root/bin" "$install_root/configs" "$install_root/docs"
  cp "$ROOT_DIR/scripts/chimera-control.sh" "$install_root/scripts/chimera-control.sh"
  cat >"$install_root/bin/chimera-cli" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  doctor)
    out=""
    rc="${CHIMERA_FAKE_DOCTOR_RC:-0}"
    shift || true
    while (($# > 0)); do
      case "$1" in
        --out)
          out="${2:-}"
          shift 2
          ;;
        --config)
          shift 2
          ;;
        --json)
          shift
          ;;
        *)
          shift
          ;;
      esac
    done
    if [[ -n "$out" ]]; then
      cat >"$out" <<'JSON'
{"status":"ok","kind":"doctor","network_state":"not_modified"}
JSON
    fi
    exit "$rc"
    ;;
  *)
    exit 0
    ;;
esac
EOF
  chmod +x "$install_root/bin/chimera-cli"
  cat >"$install_root/configs/mesh-node.example.conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = 203.0.113.10:443
carrier.server_name = node.local
capture.mode = auto
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
}

run_doctor_case() {
  local case_name="${1:?case_name_required}"
  local node_conf_mode="${2:?node_conf_mode_required}"
  local fake_rc="${3:?fake_rc_required}"
  local expect_rc="${4:?expect_rc_required}"
  local expect_phrase="${5:?expect_phrase_required}"
  local tmp_dir install_root node_conf output rc doctor_json

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  node_conf="$tmp_dir/mesh-node.conf"
  mkdir -p "$tmp_dir/home" "$tmp_dir/cache" "$tmp_dir/config" "$tmp_dir/runtime"
  make_install_root "$install_root"

  case "$node_conf_mode" in
    missing)
      rm -f "$node_conf"
      ;;
    placeholder)
      cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = 203.0.113.10:443
carrier.server_name = node.local
capture.mode = auto
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
      ;;
    ready)
      cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = carrier.mesh:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
      ;;
    doc_placeholder)
      cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = 198.51.100.10:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
      ;;
    tcp_doc_placeholder)
      cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = tcp://198.51.100.10:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
      ;;
    *)
      fail "$case_name: unknown node_conf_mode=$node_conf_mode"
      ;;
  esac

  set +e
  output="$(
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$tmp_dir/cache" \
    XDG_CONFIG_HOME="$tmp_dir/config" \
    XDG_RUNTIME_DIR="$tmp_dir/runtime" \
    NODE_CONFIG_FILE="$node_conf" \
    CHIMERA_FAKE_DOCTOR_RC="$fake_rc" \
      timeout 10s bash "$install_root/scripts/chimera-control.sh" doctor 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -ne 124 ]] || fail "$case_name: timed out"
  if [[ "$expect_rc" == "0" ]]; then
    [[ "$rc" -eq 0 ]] || fail "$case_name: expected rc=0, got $rc output=$output"
  else
    [[ "$rc" -eq "$expect_rc" ]] || fail "$case_name: expected rc=$expect_rc, got $rc output=$output"
  fi
  [[ "$output" == *"$expect_phrase"* ]] || fail "$case_name: missing phrase '$expect_phrase' output=$output"
  doctor_json="$install_root/docs/doctor_latest.json"
  [[ -f "$doctor_json" ]] || fail "$case_name: doctor report missing"
  [[ -s "$doctor_json" ]] || fail "$case_name: doctor report empty"
  if [[ "$node_conf_mode" == "placeholder" || "$node_conf_mode" == "missing" || "$node_conf_mode" == "doc_placeholder" || "$node_conf_mode" == "tcp_doc_placeholder" ]]; then
    rg -q '"status":"fail"' "$doctor_json" || fail "$case_name: expected fail artifact"
    rg -q '"node_config_ready":false' "$doctor_json" || fail "$case_name: expected unconfigured marker"
  else
    rg -q '"status":"ok"' "$doctor_json" || fail "$case_name: expected ok artifact"
  fi

  rm -rf "$tmp_dir"
}

run_doctor_case "missing_config_fails_closed" "missing" "0" "2" "doctor_status=fail reason=node_endpoint_unconfigured"
run_doctor_case "placeholder_config_fails_closed" "placeholder" "0" "2" "doctor_status=fail reason=node_endpoint_unconfigured"
run_doctor_case "doc_placeholder_config_fails_closed" "doc_placeholder" "0" "2" "doctor_status=fail reason=node_endpoint_unconfigured"
run_doctor_case "tcp_doc_placeholder_config_fails_closed" "tcp_doc_placeholder" "0" "2" "doctor_status=fail reason=node_endpoint_unconfigured"
run_doctor_case "ready_config_success_preserves_zero_exit" "ready" "0" "0" "doctor_status=ok"
run_doctor_case "ready_config_nonzero_exit_is_preserved" "ready" "7" "7" "doctor_status=fail exit=7"

run_doctor_bound_transit_missing_authority_case() {
  local tmp_dir install_root node_conf output rc doctor_json

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  node_conf="$tmp_dir/mesh-node.conf"
  mkdir -p "$tmp_dir/home" "$tmp_dir/cache" "$tmp_dir/config/chimera" "$tmp_dir/runtime"
  make_install_root "$install_root"

  cat >"$node_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = carrier.mesh:443
carrier.server_name = node.local
capture.mode = tun
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF

  cat >"$tmp_dir/config/chimera/peer-egress.env" <<'EOF'
CHIMERA_PEER_EGRESS_MODE=node
CHIMERA_PEER_EGRESS_LOCAL_LISTEN=127.0.0.1:0
CHIMERA_PEER_EGRESS_PEER_LISTEN=0.0.0.0:0
CHIMERA_PEER_EGRESS_STATE_FILE=/tmp/peer-egress.state
CHIMERA_MESH_PEER_EGRESS_STATE_PATH=/tmp/peer-egress.state
CHIMERA_PEER_EGRESS_TOKEN=test-token
CHIMERA_PEER_EGRESS_ALLOW_BOUND_TRANSIT=true
EOF

  set +e
  output="$(
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$tmp_dir/cache" \
    XDG_CONFIG_HOME="$tmp_dir/config" \
    XDG_RUNTIME_DIR="$tmp_dir/runtime" \
    NODE_CONFIG_FILE="$node_conf" \
    CHIMERA_FAKE_DOCTOR_RC="0" \
      timeout 10s bash "$install_root/scripts/chimera-control.sh" doctor 2>&1
  )"
  rc=$?
  set -e

  [[ "$rc" -eq 0 ]] || fail "bound_transit_missing_authority_doctor: expected rc=0 got $rc output=$output"
  [[ "$output" == *"doctor_status=ok"* ]] || fail "bound_transit_missing_authority_doctor: missing ok output=$output"
  [[ "$output" != *"bound_transit_unready"* ]] || fail "bound_transit_missing_authority_doctor: unexpected bound transit preflight failure output=$output"
  doctor_json="$install_root/docs/doctor_latest.json"
  [[ -f "$doctor_json" ]] || fail "bound_transit_missing_authority_doctor: doctor report missing"
  rg -q '"status":"ok"' "$doctor_json" || fail "bound_transit_missing_authority_doctor: expected ok artifact"

  rm -rf "$tmp_dir"
}

run_doctor_bound_transit_missing_authority_case

run_logs_redaction_case() {
  local tmp_dir install_root output
  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  mkdir -p "$tmp_dir/home" "$tmp_dir/cache" "$tmp_dir/config" "$tmp_dir/runtime"
  make_install_root "$install_root"
  mkdir -p "$tmp_dir/cache/chimera"
  cat >"$tmp_dir/cache/chimera/chimera_node.service.log" <<'EOF'
carrier_addr=203.0.113.10:443 endpoint=198.51.100.10:9443 server=node.local host=node.example.org
carrier_server_name=stand.internal.example listen_addr=127.0.0.1:9443 server_name=control.private.example
{"carrier_addr":"203.0.113.10:443","carrier_server_name":"node.example.org","listen_addr":"0.0.0.0:443","token":"raw-token","private_key":"raw-key"}
CHIMERA_PEER_EGRESS_TOKEN="quoted-token" SECRET='quoted-secret' password=plain-password url=https://node.example.org:443/path home=/home/rawuser/chimera
Authorization: Bearer raw-bearer-token
token: colon-token
token: "quoted-colon-token"
secret = spaced-secret
secret: 'quoted-colon-secret'
peer connected from 203.0.113.77:5555
peer connected from [2001:db8::1]:9443
raw fallback host control.private.example
EOF
  set +e
  output="$(
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$tmp_dir/cache" \
    XDG_CONFIG_HOME="$tmp_dir/config" \
    XDG_RUNTIME_DIR="$tmp_dir/runtime" \
      timeout 10s bash "$install_root/scripts/chimera-control.sh" logs 20 2>&1
  )"
  rc=$?
  set -e
  [[ "$rc" -eq 0 ]] || fail "logs_redaction: expected rc=0 got $rc output=$output"
  [[ "$output" == *"<redacted>"* ]] || fail "logs_redaction: missing redacted marker"
  [[ "$output" == *"<redacted-ip>"* ]] || fail "logs_redaction: missing redacted ip marker"
  [[ "$output" == *"/home/<redacted-user>"* ]] || fail "logs_redaction: missing redacted home marker"
  [[ "$output" != *"203.0.113.10"* ]] || fail "logs_redaction: leaked documentation IP"
  [[ "$output" != *"203.0.113.77"* ]] || fail "logs_redaction: leaked standalone IP"
  [[ "$output" != *"198.51.100.10"* ]] || fail "logs_redaction: leaked endpoint IP"
  [[ "$output" != *"2001:db8::1"* ]] || fail "logs_redaction: leaked ipv6 address"
  [[ "$output" != *"0.0.0.0:443"* ]] || fail "logs_redaction: leaked listen addr"
  [[ "$output" != *"127.0.0.1:9443"* ]] || fail "logs_redaction: leaked key listen addr"
  [[ "$output" != *"node.local"* ]] || fail "logs_redaction: leaked local hostname"
  [[ "$output" != *"node.example.org"* ]] || fail "logs_redaction: leaked example hostname"
  [[ "$output" != *"stand.internal.example"* ]] || fail "logs_redaction: leaked carrier server name"
  [[ "$output" != *"control.private.example"* ]] || fail "logs_redaction: leaked public-style hostname"
  [[ "$output" != *"raw-bearer-token"* ]] || fail "logs_redaction: leaked authorization token"
  [[ "$output" != *"colon-token"* ]] || fail "logs_redaction: leaked colon token"
  [[ "$output" != *"quoted-colon-token"* ]] || fail "logs_redaction: leaked quoted colon token"
  [[ "$output" != *"spaced-secret"* ]] || fail "logs_redaction: leaked spaced secret"
  [[ "$output" != *"quoted-colon-secret"* ]] || fail "logs_redaction: leaked quoted colon secret"
  [[ "$output" != *"quoted-token"* ]] || fail "logs_redaction: leaked quoted token"
  [[ "$output" != *"quoted-secret"* ]] || fail "logs_redaction: leaked quoted secret"
  [[ "$output" != *"plain-password"* ]] || fail "logs_redaction: leaked plain password"
  [[ "$output" != *"raw-key"* ]] || fail "logs_redaction: leaked private key"
  rm -rf "$tmp_dir"
}

run_status_redaction_case() {
  local tmp_dir install_root output
  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  mkdir -p "$tmp_dir/home" "$tmp_dir/cache/chimera" "$tmp_dir/config" "$tmp_dir/runtime"
  make_install_root "$install_root"
  cat >"$install_root/docs/runtime_state_latest.json" <<'EOF'
carrier.addr = 203.0.113.10:443
selected_node = peer#1
mesh_node = node#1
autoconnect = true
EOF
  cat >"$tmp_dir/cache/chimera/peer-egress.state" <<'EOF'
resolved_local_listen=127.0.0.1:11111
resolved_peer_listen=198.51.100.44:45678
mode=peer
EOF
  set +e
  output="$(
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$tmp_dir/cache" \
    XDG_CONFIG_HOME="$tmp_dir/config" \
    XDG_RUNTIME_DIR="$tmp_dir/runtime" \
      timeout 10s bash "$install_root/scripts/chimera-control.sh" status 2>&1
  )"
  rc=$?
  set -e
  [[ "$rc" -eq 0 ]] || fail "status_redaction: expected rc=0 got $rc output=$output"
  [[ "$output" == *"state_file=<redacted>"* ]] || fail "status_redaction: state file not redacted"
  [[ "$output" == *"peer_egress_state=<redacted>"* ]] || fail "status_redaction: peer state file not redacted"
  [[ "$output" == *"carrier_addr=<redacted>"* ]] || fail "status_redaction: carrier addr not redacted"
  [[ "$output" == *"peer_egress_resolved_local_listen=<redacted>"* ]] || fail "status_redaction: local listen not redacted"
  [[ "$output" == *"peer_egress_resolved_peer_listen=<redacted>"* ]] || fail "status_redaction: peer listen not redacted"
  [[ "$output" == *"selected_node=<redacted>"* ]] || fail "status_redaction: selected node not redacted"
  [[ "$output" == *"mesh_node=<redacted>"* ]] || fail "status_redaction: mesh node not redacted"
  [[ "$output" != *"203.0.113.10"* ]] || fail "status_redaction: leaked carrier addr"
  [[ "$output" != *"peer#1"* ]] || fail "status_redaction: leaked selected node"
  [[ "$output" != *"node#1"* ]] || fail "status_redaction: leaked mesh node"
  [[ "$output" != *"127.0.0.1:11111"* ]] || fail "status_redaction: leaked local listen"
  [[ "$output" != *"198.51.100.44:45678"* ]] || fail "status_redaction: leaked peer listen"
  [[ "$output" != *"$tmp_dir"* ]] || fail "status_redaction: leaked temp path"
  rm -rf "$tmp_dir"
}

run_legacy_app_workflow_disabled_case() {
  local tmp_dir install_root cmd output rc
  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  mkdir -p "$tmp_dir/home" "$tmp_dir/cache" "$tmp_dir/config" "$tmp_dir/runtime"
  make_install_root "$install_root"
  for cmd in \
    "run-app browser" \
    "verify-app browser" \
    "verify-cmd true" \
    "service-route-enable browser" \
    "service-route-disable browser" \
    "verify-service browser" \
    "service-route-enable-running browser"
  do
    set +e
    output="$(
      HOME="$tmp_dir/home" \
      XDG_CACHE_HOME="$tmp_dir/cache" \
      XDG_CONFIG_HOME="$tmp_dir/config" \
      XDG_RUNTIME_DIR="$tmp_dir/runtime" \
        timeout 10s bash "$install_root/scripts/chimera-control.sh" $cmd 2>&1
    )"
    rc=$?
    set -e
    [[ "$rc" -eq 2 ]] || fail "legacy_app_workflow: expected rc=2 for '$cmd', got $rc output=$output"
    [[ "$output" == *"reason=legacy_lab_only_not_datapath_evidence"* ]] || fail "legacy_app_workflow: missing lab-only reason for '$cmd' output=$output"
    [[ "$output" == *"product_datapath_evidence=false"* ]] || fail "legacy_app_workflow: missing evidence=false for '$cmd' output=$output"
    [[ "$output" != *"_status=pass"* ]] || fail "legacy_app_workflow: false pass leaked for '$cmd' output=$output"
  done
  rm -rf "$tmp_dir"
}

run_logs_redaction_case
run_status_redaction_case
run_legacy_app_workflow_disabled_case

echo "chimera_doctor_contract_smoke=pass"
