#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RELEASE_DIR="${ROOT_DIR}/target/chimera-release"
RELEASE_VERSION="${CHIMERA_RELEASE_VERSION:-$(date +%Y%m%d-%H%M%S)}"
ARCHIVE_NAME="chimera-pq-${RELEASE_VERSION}.tar.gz"

echo "build_release: version=${RELEASE_VERSION}"

rm -rf "${RELEASE_DIR}"
mkdir -p "${RELEASE_DIR}/bin"
mkdir -p "${RELEASE_DIR}/configs"
mkdir -p "${RELEASE_DIR}/deploy/systemd-user"
mkdir -p "${RELEASE_DIR}/deploy/desktop"
mkdir -p "${RELEASE_DIR}/scripts"

echo "build_release: copying binaries"
cp -p "${ROOT_DIR}/bin/chimera-cli" "${RELEASE_DIR}/bin/"
cp -p "${ROOT_DIR}/bin/chimera-gateway" "${RELEASE_DIR}/bin/"
cp -p "${ROOT_DIR}/bin/chimera-peer-egress" "${RELEASE_DIR}/bin/"
cp -p "${ROOT_DIR}/bin/chimera-transparent-runtime" "${RELEASE_DIR}/bin/"
cp -p "${ROOT_DIR}/bin/chimera-transparent-tcp" "${RELEASE_DIR}/bin/"
cp -p "${ROOT_DIR}/bin/chimera-bootstrap" "${RELEASE_DIR}/bin/"

echo "build_release: copying configs"
cp -p "${ROOT_DIR}/configs"/*.example.* "${RELEASE_DIR}/configs/" 2>/dev/null || true
cp -p "${ROOT_DIR}/configs"/*.conf "${RELEASE_DIR}/configs/" 2>/dev/null || true

echo "build_release: copying deploy units"
cp -p "${ROOT_DIR}/deploy/systemd-user/chimera-client.service" "${RELEASE_DIR}/deploy/systemd-user/"
cp -p "${ROOT_DIR}/deploy/systemd-user/chimera-gateway.service" "${RELEASE_DIR}/deploy/systemd-user/"
cp -p "${ROOT_DIR}/deploy/desktop/chimera-control-gui.desktop" "${RELEASE_DIR}/deploy/desktop/"

echo "build_release: copying scripts"
cp -p "${ROOT_DIR}/scripts/install_desktop_control.sh" "${RELEASE_DIR}/scripts/"
cp -p "${ROOT_DIR}/scripts/chimera_installer_gate.sh" "${RELEASE_DIR}/scripts/"
cp -p "${ROOT_DIR}/scripts/chimera-control.sh" "${RELEASE_DIR}/scripts/"
cp -p "${ROOT_DIR}/scripts/chimera-control-tray.sh" "${RELEASE_DIR}/scripts/"
cp -p "${ROOT_DIR}/scripts/chimera-control-launcher.sh" "${RELEASE_DIR}/scripts/"
cp -p "${ROOT_DIR}/scripts/chimera_runtime_bootstrap.sh" "${RELEASE_DIR}/scripts/"
cp -p "${ROOT_DIR}/scripts/chimera-runner.sh" "${RELEASE_DIR}/scripts/"
cp -p "${ROOT_DIR}/scripts/chimera-sh" "${RELEASE_DIR}/scripts/" 2>/dev/null || true
cp -p "${ROOT_DIR}/scripts/chimera.sh" "${RELEASE_DIR}/scripts/" 2>/dev/null || true

printf '%s' "${RELEASE_VERSION}" > "${RELEASE_DIR}/.chimera_release_version"

echo "build_release: creating tarball"
tar -czf "${ROOT_DIR}/target/${ARCHIVE_NAME}" -C "${ROOT_DIR}/target" "chimera-release"

BUNDLE_SHA256="$(sha256sum "${ROOT_DIR}/target/${ARCHIVE_NAME}" | cut -d' ' -f1)"
printf '%s' "${BUNDLE_SHA256}" > "${ROOT_DIR}/target/chimera-release.sha256"

echo "build_release: done"
echo "  archive:   target/${ARCHIVE_NAME}"
echo "  size:      $(du -h "${ROOT_DIR}/target/${ARCHIVE_NAME}" | cut -f1)"
echo "  sha256:    ${BUNDLE_SHA256}"
echo "  contents:  ${RELEASE_DIR}/"
