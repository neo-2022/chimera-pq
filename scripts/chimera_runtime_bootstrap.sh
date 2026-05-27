#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUNTIME_DIR="${CHIMERA_RUNTIME_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/chimera-pq/runtime}"
SINGBOX_RUNTIME_DIR="${CHIMERA_SINGBOX_RUNTIME_DIR:-$RUNTIME_DIR/singbox}"
SINGBOX_BIN_TARGET="$SINGBOX_RUNTIME_DIR/sing-box"
SINGBOX_VERSION="${CHIMERA_SINGBOX_VERSION:-1.11.8}"
SINGBOX_URL="${CHIMERA_SINGBOX_URL:-}"
SINGBOX_SHA256="${CHIMERA_SINGBOX_SHA256:-}"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "bootstrap_error=missing_cmd cmd=$1" >&2
    exit 1
  }
}

detect_arch() {
  local arch
  arch="$(uname -m)"
  case "$arch" in
    x86_64|amd64) echo "amd64" ;;
    aarch64|arm64) echo "arm64" ;;
    *)
      echo "bootstrap_error=unsupported_arch arch=$arch" >&2
      return 1
      ;;
  esac
}

system_singbox_path() {
  command -v sing-box 2>/dev/null || true
}

runtime_singbox_ok() {
  [[ -x "$SINGBOX_BIN_TARGET" ]] || return 1
  "$SINGBOX_BIN_TARGET" version >/dev/null 2>&1
}

resolve_default_url() {
  local arch="$1"
  echo "https://github.com/SagerNet/sing-box/releases/download/v${SINGBOX_VERSION}/sing-box-${SINGBOX_VERSION}-linux-${arch}.tar.gz"
}

install_runtime_singbox() {
  need_cmd curl
  need_cmd tar
  local arch url tmp tgz extracted
  arch="$(detect_arch)"
  url="${SINGBOX_URL:-$(resolve_default_url "$arch")}"
  tmp="$(mktemp -d)"
  tgz="$tmp/sing-box.tar.gz"

  mkdir -p "$SINGBOX_RUNTIME_DIR"
  curl -fL "$url" -o "$tgz"

  if [[ -n "$SINGBOX_SHA256" ]]; then
    need_cmd sha256sum
    local got
    got="$(sha256sum "$tgz" | awk '{print $1}')"
    if [[ "$got" != "$SINGBOX_SHA256" ]]; then
      echo "bootstrap_error=sha256_mismatch expected=$SINGBOX_SHA256 got=$got" >&2
      rm -rf "$tmp"
      exit 1
    fi
  fi

  tar -xzf "$tgz" -C "$tmp"
  extracted="$(find "$tmp" -type f -name 'sing-box' | head -n 1)"
  if [[ -z "$extracted" ]]; then
    echo "bootstrap_error=singbox_not_found_in_archive" >&2
    rm -rf "$tmp"
    exit 1
  fi
  install -m 0755 "$extracted" "$SINGBOX_BIN_TARGET"
  rm -rf "$tmp"
}

ensure_singbox() {
  local sys_bin
  if runtime_singbox_ok; then
    echo "singbox_path=$SINGBOX_BIN_TARGET"
    return 0
  fi

  sys_bin="$(system_singbox_path)"
  if [[ -n "$sys_bin" ]]; then
    mkdir -p "$SINGBOX_RUNTIME_DIR"
    install -m 0755 "$sys_bin" "$SINGBOX_BIN_TARGET"
    if runtime_singbox_ok; then
      echo "singbox_path=$SINGBOX_BIN_TARGET"
      return 0
    fi
  fi

  install_runtime_singbox
  if runtime_singbox_ok; then
    echo "singbox_path=$SINGBOX_BIN_TARGET"
    return 0
  fi
  echo "bootstrap_error=singbox_runtime_unavailable" >&2
  return 1
}

cmd="${1:-ensure-singbox}"
case "$cmd" in
  ensure-singbox)
    ensure_singbox
    ;;
  *)
    echo "usage: $0 [ensure-singbox]" >&2
    exit 2
    ;;
esac

