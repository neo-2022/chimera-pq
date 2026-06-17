#!/usr/bin/env bash

runtime_route_build_cli() {
  cargo build -q -p chimera-cli
  CHIMERA_CLI_BIN="$ROOT_DIR/target/debug/chimera-cli"
  export CHIMERA_CLI_BIN
  test -x "$CHIMERA_CLI_BIN"
}

runtime_route_sudo_netns_allowed() {
  [[ "${CHIMERA_RUNTIME_ROUTE_ALLOW_SUDO_NETNS:-}" == "1" || "${GITHUB_ACTIONS:-}" == "true" ]]
}

runtime_route_probe_rootless_netns() {
  local probe="${1:?probe_required}"
  unshare -Urn bash -ceu "$probe" >/dev/null 2>&1
}

runtime_route_probe_sudo_netns() {
  local probe="${1:?probe_required}"
  runtime_route_sudo_netns_allowed || return 1
  command -v sudo >/dev/null 2>&1 || return 1
  sudo -n modprobe tun >/dev/null 2>&1 || true
  sudo -n env PATH="/usr/sbin:/usr/bin:/sbin:/bin" unshare -n bash -ceu "$probe" >/dev/null 2>&1
}

runtime_route_select_netns() {
  local probe="${1:?probe_required}"
  if runtime_route_probe_rootless_netns "$probe"; then
    CHIMERA_ROUTE_NETNS_MODE="rootless"
    export CHIMERA_ROUTE_NETNS_MODE
    return 0
  fi
  if runtime_route_probe_sudo_netns "$probe"; then
    CHIMERA_ROUTE_NETNS_MODE="sudo"
    export CHIMERA_ROUTE_NETNS_MODE
    return 0
  fi
  CHIMERA_ROUTE_NETNS_MODE="none"
  export CHIMERA_ROUTE_NETNS_MODE
  return 1
}

runtime_route_run_netns() {
  local script="${1:?script_required}"
  case "${CHIMERA_ROUTE_NETNS_MODE:-none}" in
    rootless)
      env CHIMERA_CLI_BIN="$CHIMERA_CLI_BIN" unshare -Urn bash -ceu "$script"
      ;;
    sudo)
      sudo -n env \
        PATH="/usr/sbin:/usr/bin:/sbin:/bin" \
        CHIMERA_CLI_BIN="$CHIMERA_CLI_BIN" \
        unshare -n bash -ceu "$script"
      ;;
    *)
      return 1
      ;;
  esac
}
