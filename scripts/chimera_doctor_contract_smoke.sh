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
  cat >"$install_root/configs/client.example.conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = 203.0.113.10:443
carrier.server_name = gateway.local
capture.mode = auto
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
}

run_doctor_case() {
  local case_name="${1:?case_name_required}"
  local client_conf_mode="${2:?client_conf_mode_required}"
  local fake_rc="${3:?fake_rc_required}"
  local expect_rc="${4:?expect_rc_required}"
  local expect_phrase="${5:?expect_phrase_required}"
  local tmp_dir install_root client_conf output rc doctor_json

  tmp_dir="$(mktemp -d)"
  install_root="$tmp_dir/chimera-release"
  client_conf="$tmp_dir/client.conf"
  mkdir -p "$tmp_dir/home" "$tmp_dir/cache" "$tmp_dir/config" "$tmp_dir/runtime"
  make_install_root "$install_root"

  case "$client_conf_mode" in
    missing)
      rm -f "$client_conf"
      ;;
    placeholder)
      cat >"$client_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = 203.0.113.10:443
carrier.server_name = gateway.local
capture.mode = auto
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
      ;;
    ready)
      cat >"$client_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = carrier.mesh:443
carrier.server_name = gateway.local
capture.mode = tun
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
      ;;
    doc_placeholder)
      cat >"$client_conf" <<'EOF'
carrier.profile = in-memory
carrier.addr = 198.51.100.10:443
carrier.server_name = gateway.local
capture.mode = tun
capture.tun_supported = true
rekey.max_age_seconds = 300
rekey.max_packets_per_key = 10000
EOF
      ;;
    *)
      fail "$case_name: unknown client_conf_mode=$client_conf_mode"
      ;;
  esac

  set +e
  output="$(
    HOME="$tmp_dir/home" \
    XDG_CACHE_HOME="$tmp_dir/cache" \
    XDG_CONFIG_HOME="$tmp_dir/config" \
    XDG_RUNTIME_DIR="$tmp_dir/runtime" \
    CLIENT_CONFIG_FILE="$client_conf" \
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
  if [[ "$client_conf_mode" == "placeholder" || "$client_conf_mode" == "missing" || "$client_conf_mode" == "doc_placeholder" ]]; then
    rg -q '"status":"fail"' "$doctor_json" || fail "$case_name: expected fail artifact"
    rg -q '"client_config_ready":false' "$doctor_json" || fail "$case_name: expected unconfigured marker"
  else
    rg -q '"status":"ok"' "$doctor_json" || fail "$case_name: expected ok artifact"
  fi

  rm -rf "$tmp_dir"
}

run_doctor_case "missing_config_fails_closed" "missing" "0" "2" "doctor_status=fail reason=client_endpoint_unconfigured"
run_doctor_case "placeholder_config_fails_closed" "placeholder" "0" "2" "doctor_status=fail reason=client_endpoint_unconfigured"
run_doctor_case "doc_placeholder_config_fails_closed" "doc_placeholder" "0" "2" "doctor_status=fail reason=client_endpoint_unconfigured"
run_doctor_case "ready_config_success_preserves_zero_exit" "ready" "0" "0" "doctor_status=ok"
run_doctor_case "ready_config_nonzero_exit_is_preserved" "ready" "7" "7" "doctor_status=fail exit=7"

echo "chimera_doctor_contract_smoke=pass"
