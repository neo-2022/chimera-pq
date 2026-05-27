#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

policy_file="/tmp/chimera_policy_precedence_smoke.conf"
out_main="/tmp/chimera_policy_precedence_main.json"
out_dns="/tmp/chimera_policy_precedence_dns.json"

cat > "$policy_file" <<'POLICY'
exact-direct = exact:video.example.org => direct
suffix-gateway = suffix:example.org => gateway
default-direct = default => direct
POLICY

cargo run -q -p chimera-cli -- route explain \
  --domain video.example.org \
  --policy "$policy_file" \
  --show-all-matches \
  --json --out "$out_main"

cargo run -q -p chimera-cli -- route explain \
  --ip 203.0.113.77 \
  --dns-bind-domain blocked.example.org \
  --dns-bind-ip 203.0.113.77 \
  --policy "$policy_file" \
  --json --out "$out_dns"

precedence_ok=false
all_matches_ok=false
dns_binding_ok=false
network_state_ok=false

if rg -q '"rule_used":"exact-direct"' "$out_main" && rg -q '"outbound":"direct"' "$out_main"; then
  precedence_ok=true
fi
if rg -q '"matched_rules":\["exact-direct","suffix-gateway","default-direct"\]' "$out_main"; then
  all_matches_ok=true
fi
if rg -q '"domain_source_dns":true' "$out_dns" && rg -q '"rule_used":"suffix-gateway"' "$out_dns" && rg -q '"outbound":"gateway"' "$out_dns"; then
  dns_binding_ok=true
fi
if rg -q '"network_state":"not_modified"' "$out_main" && rg -q '"network_state":"not_modified"' "$out_dns"; then
  network_state_ok=true
fi

json="{\"status\":\"ok\",\"kind\":\"runtime_policy_precedence_smoke\",\"message_en\":\"Runtime policy precedence smoke executed.\",\"message_ru\":\"Smoke-проверка приоритета policy выполнена.\",\"network_state\":\"not_modified\",\"precedence_ok\":${precedence_ok},\"all_matches_ok\":${all_matches_ok},\"dns_binding_ok\":${dns_binding_ok},\"network_state_ok\":${network_state_ok}}"
printf "%s\n" "$json" > docs/RUNTIME_POLICY_PRECEDENCE_SMOKE.json

if [[ "$precedence_ok" != "true" || "$all_matches_ok" != "true" || "$dns_binding_ok" != "true" || "$network_state_ok" != "true" ]]; then
  echo "runtime policy precedence smoke: FAIL" >&2
  exit 1
fi

echo "runtime policy precedence smoke: PASS"
