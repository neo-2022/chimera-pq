# CHIMERA-PQ forced-stop rollback cleanup helpers.
# Sourced by scripts/chimera-control.sh.  All functions are idempotent and
# do not fail closed when the OS artifacts are already absent.

# Build the default list of CHIMERA-managed TUN devices.
chimera_managed_tun_devices() {
  local -a devs=()
  local dev state_dev
  for dev in "${CHIMERA_TUN_NAME:-chimera0}" "${SPLIT_TRANSPARENT_TUN_NAME:-chimera-tun}"; do
    [[ -n "$dev" ]] && devs+=("$dev")
  done
  if [[ -f "$STATE_FILE" ]]; then
    state_dev="$(sed -n 's/.*"tun_device":"\([^"]*\)".*/\1/p' "$STATE_FILE" 2>/dev/null | head -n1 || true)"
    [[ -n "$state_dev" ]] && devs+=("$state_dev")
  fi
  printf '%s\n' "${devs[@]}" | sort -u
}

chimera_cleanup_tun_devices() {
  local rc=0 dev
  while IFS= read -r dev; do
    [[ -n "$dev" ]] || continue
    if run_ip_privileged link show dev "$dev" >/dev/null 2>&1; then
      if run_ip_privileged link delete dev "$dev" >/dev/null 2>&1; then
        echo "cleanup_tun=ok device=$dev"
      else
        echo "cleanup_tun=warn device=$dev reason=delete_failed" >&2
        rc=1
      fi
    else
      echo "cleanup_tun=ok device=$dev reason=absent"
    fi
  done < <(chimera_managed_tun_devices)
  return "$rc"
}

chimera_cleanup_nft_tables() {
  local rc=0 nft_cmd table
  nft_cmd="$(resolve_nft_command 2>/dev/null)" || {
    echo "cleanup_nft=skipped reason=nft_missing"
    return 0
  }
  while IFS= read -r table; do
    [[ -n "$table" ]] || continue
    if run_nft_command delete table inet "$table" >/dev/null 2>&1; then
      echo "cleanup_nft=ok table=$table"
    elif run_nft_command list table inet "$table" >/dev/null 2>&1; then
      echo "cleanup_nft=warn table=$table reason=delete_failed" >&2
      rc=1
    else
      echo "cleanup_nft=ok table=$table reason=absent"
    fi
  done < <(run_nft_command list tables 2>/dev/null | awk '/^table inet chimera_redirect(_[^ ]*)? \{$/ { gsub(/[{}]/,"",$3); print $3 }' || true)
  return "$rc"
}

chimera_cleanup_ip_rules() {
  local rc=0 cidr route_table="$CHIMERA_ROUTE_TABLE" fwmark="$CHIMERA_ROUTE_FWMARK"

  run_ip_privileged route flush table "$route_table" >/dev/null 2>&1 || {
    echo "cleanup_ip_rules=warn reason=flush_table_failed table=$route_table" >&2
    rc=1
  }

  run_ip_privileged rule del fwmark "$fwmark" table main >/dev/null 2>&1 || true

  while IFS= read -r cidr; do
    [[ -n "$cidr" ]] || continue
    run_ip_privileged rule del to "$cidr" pref "$CHIMERA_ROUTE_RULE_PRIORITY" table "$route_table" >/dev/null 2>&1 || true
  done < <(split_csv_lines "$CHIMERA_ROUTE_CIDR")

  while IFS= read -r pref; do
    [[ -n "$pref" ]] || continue
    run_ip_privileged rule del pref "$pref" table "$route_table" >/dev/null 2>&1 || true
  done < <(run_ip_privileged -4 rule show 2>/dev/null | awk -v t="$route_table" '/^([0-9]+):/ && $0 ~ "lookup " t "($|[[:space:]])" { sub(/:/,"",$1); print $1 }' || true)

  if [[ "$rc" -eq 0 ]]; then
    echo "cleanup_ip_rules=ok"
  else
    echo "cleanup_ip_rules=partial"
  fi
  return 0
}

chimera_cleanup_dns() {
  if [[ "${CHIMERA_CLEANUP_ALLOW_DNS:-0}" != "1" ]]; then
    echo "cleanup_dns=skipped reason=not_authorized"
    return 0
  fi

  local resolv_path="" backup_path=""
  if [[ -f "$STATE_FILE" ]]; then
    resolv_path="$(awk -F'"' '/"resolv_conf_path":/ { for(i=1;i<NF;i++) if($i=="resolv_conf_path") { print $(i+2); exit } }' "$STATE_FILE" 2>/dev/null || true)"
    backup_path="$(awk -F'"' '/"dns_backup_path":/ { for(i=1;i<NF;i++) if($i=="dns_backup_path") { print $(i+2); exit } }' "$STATE_FILE" 2>/dev/null || true)"
  fi
  resolv_path="${resolv_path:-/etc/resolv.conf}"
  backup_path="${backup_path:-${resolv_path}.chimera.bak}"

  if [[ ! -f "$backup_path" ]]; then
    echo "cleanup_dns=ok reason=no_backup"
    return 0
  fi

  if [[ -f "$STATE_FILE" ]]; then
    if run_chimera_cli down --state-file "$STATE_FILE" >/dev/null 2>&1; then
      echo "cleanup_dns=ok"
      return 0
    fi
  fi

  if [[ "$EUID" -eq 0 || -w "$resolv_path" ]]; then
    if cat "$backup_path" >"$resolv_path" 2>/dev/null; then
      echo "cleanup_dns=ok"
    else
      echo "cleanup_dns=warn reason=restore_failed" >&2
    fi
  else
    echo "cleanup_dns=warn reason=no_write_access path=$resolv_path" >&2
  fi
  return 0
}

chimera_rollback_cleanup_core() {
  echo "rollback_cleanup=started"
  local rc=0 route_fwmark

  if [[ -f "$STATE_FILE" ]]; then
    route_fwmark="$(route_fwmark_env_value)"
    run_chimera_cli down \
      --config "$(node_config_path)" \
      --state-file "$STATE_FILE" \
      --apply-tun true \
      --apply-route true \
      --route-policy true \
      --route-table "$CHIMERA_ROUTE_TABLE" \
      --route-rule-priority "$CHIMERA_ROUTE_RULE_PRIORITY" \
      --route-fwmark "$route_fwmark" \
      --route-cidr "$CHIMERA_ROUTE_CIDR" \
      --apply-dns "${CHIMERA_APPLY_DNS:-false}" >/dev/null 2>&1 || true
  fi

  chimera_cleanup_tun_devices || { echo "rollback_cleanup=warn reason=tun_partial" >&2; rc=1; }
  chimera_cleanup_ip_rules || { echo "rollback_cleanup=warn reason=ip_rules_partial" >&2; rc=1; }
  chimera_cleanup_nft_tables || { echo "rollback_cleanup=warn reason=nft_partial" >&2; rc=1; }
  chimera_cleanup_dns || { echo "rollback_cleanup=warn reason=dns_partial" >&2; rc=1; }
  clear_runtime_generated_state >/dev/null 2>&1 || true

  if [[ "$rc" -eq 0 ]]; then
    echo "rollback_cleanup=ok"
  else
    echo "rollback_cleanup=partial"
  fi
  return 0
}

chimera_execstoppost_cleanup() {
  CHIMERA_CLEANUP_ALLOW_DNS=1 chimera_rollback_cleanup_core
}

chimera_verify_rollback_state() {
  local dirty=0 dev nft_cmd table route_table="$CHIMERA_ROUTE_TABLE" fwmark="$CHIMERA_ROUTE_FWMARK"

  while IFS= read -r dev; do
    [[ -n "$dev" ]] || continue
    if run_ip_privileged link show dev "$dev" >/dev/null 2>&1; then
      echo "verify=tun_leftover device=$dev"
      dirty=1
    fi
  done < <(chimera_managed_tun_devices)

  nft_cmd="$(resolve_nft_command 2>/dev/null)" || true
  if [[ -n "$nft_cmd" ]]; then
    while IFS= read -r table; do
      [[ -n "$table" ]] || continue
      echo "verify=nft_table_leftover table=$table"
      dirty=1
    done < <(run_nft_command list tables 2>/dev/null | awk '/^table inet chimera_redirect(_[^ ]*)? \{$/ { gsub(/[{}]/,"",$3); print $3 }' || true)
  fi

  if run_ip_privileged -4 rule show 2>/dev/null | grep -Eq "lookup $route_table|fwmark $fwmark"; then
    echo "verify=ip_rule_leftover route_table=$route_table fwmark=$fwmark"
    dirty=1
  fi

  if run_ip_privileged route show table "$route_table" 2>/dev/null | grep -qE '[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+'; then
    echo "verify=route_leftover route_table=$route_table"
    dirty=1
  fi

  if [[ -f "/etc/resolv.conf.chimera.bak" ]]; then
    echo "verify=dns_backup_present path=/etc/resolv.conf.chimera.bak"
    dirty=1
  fi

  if [[ -n "${CHIMERA_VERIFY_REMOTE_HOSTS:-}" ]]; then
    local host
    for host in $CHIMERA_VERIFY_REMOTE_HOSTS; do
      if ping -c1 -W2 "$host" >/dev/null 2>&1; then
        echo "verify=remote_host_reachable host=$host"
      else
        echo "verify=remote_host_unreachable host=$host"
      fi
    done
  fi

  if [[ "$dirty" -eq 0 ]]; then
    echo "verify=rollback_clean"
    return 0
  fi
  echo "verify=rollback_dirty"
  return 1
}
