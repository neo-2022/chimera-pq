#!/usr/bin/env bash
set -euo pipefail
SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHIMERA_HOME="${HOME}/.local/share/chimera"
LOCAL_BIN="${HOME}/.local/bin"
BUNDLE_SOURCE="${1:-${SELF_DIR}/../target/chimera-release}"

echo "CHIMERA self-contained install"
echo "  source: ${BUNDLE_SOURCE}"

if [[ -d "${BUNDLE_SOURCE}" ]]; then
  echo "install: copying release directory"
  rm -rf "${CHIMERA_HOME}"
  mkdir -p "$(dirname "${CHIMERA_HOME}")"
  cp -a "${BUNDLE_SOURCE}" "${CHIMERA_HOME}"
elif [[ -f "${BUNDLE_SOURCE}" && "${BUNDLE_SOURCE}" == *.tar.gz ]]; then
  echo "install: extracting tarball"
  rm -rf "${CHIMERA_HOME}"
  mkdir -p "$(dirname "${CHIMERA_HOME}")"
  tar -xzf "${BUNDLE_SOURCE}" -C "$(dirname "${CHIMERA_HOME}")"
  mv "$(dirname "${CHIMERA_HOME}")/chimera-release" "${CHIMERA_HOME}"
elif [[ "${BUNDLE_SOURCE}" == https://* || "${BUNDLE_SOURCE}" == http://* ]]; then
  echo "install: downloading from ${BUNDLE_SOURCE}"
  rm -rf "${CHIMERA_HOME}"
  mkdir -p "$(dirname "${CHIMERA_HOME}")"
  TMP_ARCHIVE="$(mktemp /tmp/chimera-release-XXXXXX.tar.gz)"
  if command -v curl >/dev/null 2>&1; then
    curl -fL --retry 3 -o "${TMP_ARCHIVE}" "${BUNDLE_SOURCE}"
  elif command -v wget >/dev/null 2>&1; then
    wget -O "${TMP_ARCHIVE}" "${BUNDLE_SOURCE}"
  else
    echo "error: need curl or wget to download release" >&2
    exit 1
  fi
  tar -xzf "${TMP_ARCHIVE}" -C "$(dirname "${CHIMERA_HOME}")"
  mv "$(dirname "${CHIMERA_HOME}")/chimera-release" "${CHIMERA_HOME}"
  rm -f "${TMP_ARCHIVE}"
else
  echo "error: cannot find release at ${BUNDLE_SOURCE}" >&2
  echo "usage: ${0} [<path-to-tarball> | <path-to-release-dir> | <url>]" >&2
  exit 1
fi

chmod +x "${CHIMERA_HOME}/bin/"*
chmod +x "${CHIMERA_HOME}/scripts/"*.sh

echo "install: running desktop control setup"
CHIMERA_RELEASE_VERSION="$(cat "${CHIMERA_HOME}/.chimera_release_version" 2>/dev/null || true)"
export CHIMERA_RELEASE_VERSION
bash "${CHIMERA_HOME}/scripts/install_desktop_control.sh"

mkdir -p "${LOCAL_BIN}"
ln -sfn "${CHIMERA_HOME}/scripts/chimera.sh" "${LOCAL_BIN}/chimera"
ln -sfn "${CHIMERA_HOME}/scripts/chimera-sh" "${LOCAL_BIN}/chimera-sh"

echo
echo "CHIMERA self-contained install complete."
echo "  version: ${CHIMERA_RELEASE_VERSION:-unknown}"
echo "  home:    ${CHIMERA_HOME}"
echo "  bin:     ${LOCAL_BIN}/chimera"
echo
echo "Quick start:"
echo "  chimera -start"
echo "  chimera -status"
echo "  chimera -stop"
