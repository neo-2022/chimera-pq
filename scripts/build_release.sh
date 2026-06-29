#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RELEASE_DIR="${ROOT_DIR}/target/chimera-release"
release_version_from_exact_tag() {
  local tag=""
  tag="$(git -C "$ROOT_DIR" describe --tags --exact-match 2>/dev/null || true)"
  [[ "$tag" == v* ]] || return 1
  printf '%s\n' "${tag#v}"
}

RELEASE_VERSION="${CHIMERA_RELEASE_VERSION:-$(release_version_from_exact_tag || true)}"
ARCHIVE_NAME="chimera-pq-${RELEASE_VERSION}.tar.gz"
LATEST_ARCHIVE_NAME="chimera-pq-release.tar.gz"
LATEST_CHECKSUM_NAME="${LATEST_ARCHIVE_NAME}.sha256"

if [[ ! "$RELEASE_VERSION" =~ ^[0-9]+[.][0-9]+[.][0-9]+$ ]]; then
  echo "error: CHIMERA_RELEASE_VERSION must be semver X.Y.Z, or build from an exact vX.Y.Z tag" >&2
  exit 2
fi

echo "build_release: version=${RELEASE_VERSION}"

build_bin() {
  local package="${1:?package_required}"
  local binary="${2:?binary_required}"
  cargo build --release -p "$package" --bin "$binary"
}

echo "build_release: building release binaries"
build_bin chimera-cli chimera-cli
build_bin chimera-gateway chimera-gateway
build_bin chimera-carrier chimera-peer-egress
build_bin chimera-capture chimera-transparent-runtime
build_bin chimera-capture chimera-transparent-tcp
build_bin chimera-bootstrap chimera-bootstrap

rm -rf "${RELEASE_DIR}"
rm -rf "${ROOT_DIR}/bin"
mkdir -p "${ROOT_DIR}/bin"
mkdir -p "${RELEASE_DIR}/bin"
mkdir -p "${RELEASE_DIR}/configs"
mkdir -p "${RELEASE_DIR}/deploy/systemd-user"
mkdir -p "${RELEASE_DIR}/deploy/desktop"
mkdir -p "${RELEASE_DIR}/scripts"

echo "build_release: copying binaries"
cp -p "${ROOT_DIR}/target/release/chimera-cli" "${ROOT_DIR}/bin/"
cp -p "${ROOT_DIR}/target/release/chimera-gateway" "${ROOT_DIR}/bin/"
cp -p "${ROOT_DIR}/target/release/chimera-peer-egress" "${ROOT_DIR}/bin/"
cp -p "${ROOT_DIR}/target/release/chimera-transparent-runtime" "${ROOT_DIR}/bin/"
cp -p "${ROOT_DIR}/target/release/chimera-transparent-tcp" "${ROOT_DIR}/bin/"
cp -p "${ROOT_DIR}/target/release/chimera-bootstrap" "${ROOT_DIR}/bin/"
cp -p "${ROOT_DIR}/bin/chimera-cli" "${RELEASE_DIR}/bin/"
cp -p "${ROOT_DIR}/bin/chimera-gateway" "${RELEASE_DIR}/bin/"
cp -p "${ROOT_DIR}/bin/chimera-peer-egress" "${RELEASE_DIR}/bin/"
cp -p "${ROOT_DIR}/bin/chimera-transparent-runtime" "${RELEASE_DIR}/bin/"
cp -p "${ROOT_DIR}/bin/chimera-transparent-tcp" "${RELEASE_DIR}/bin/"
cp -p "${ROOT_DIR}/bin/chimera-bootstrap" "${RELEASE_DIR}/bin/"

echo "build_release: copying configs"
cp -p "${ROOT_DIR}/configs"/*.example.* "${RELEASE_DIR}/configs/" 2>/dev/null || true
cp -p "${ROOT_DIR}/configs"/*.env.example "${RELEASE_DIR}/configs/" 2>/dev/null || true
cp -p "${ROOT_DIR}/configs"/*.conf "${RELEASE_DIR}/configs/" 2>/dev/null || true

echo "build_release: copying deploy units"
cp -p "${ROOT_DIR}/deploy/systemd-user/chimera-client.service" "${RELEASE_DIR}/deploy/systemd-user/"
cp -p "${ROOT_DIR}/deploy/systemd-user/chimera-gateway.service" "${RELEASE_DIR}/deploy/systemd-user/"
cp -p "${ROOT_DIR}/deploy/desktop/chimera-control-gui.desktop" "${RELEASE_DIR}/deploy/desktop/"

echo "build_release: copying scripts"
cp -p "${ROOT_DIR}/scripts/install_desktop_control.sh" "${RELEASE_DIR}/scripts/"
cp -p "${ROOT_DIR}/scripts/install_release.sh" "${RELEASE_DIR}/scripts/"
cp -p "${ROOT_DIR}/scripts/chimera-control.sh" "${RELEASE_DIR}/scripts/"
cp -p "${ROOT_DIR}/scripts/chimera-control-tray.sh" "${RELEASE_DIR}/scripts/"
cp -p "${ROOT_DIR}/scripts/chimera-control-launcher.sh" "${RELEASE_DIR}/scripts/"
cp -p "${ROOT_DIR}/scripts/chimera_runtime_bootstrap.sh" "${RELEASE_DIR}/scripts/"
cp -p "${ROOT_DIR}/scripts/chimera-runner.sh" "${RELEASE_DIR}/scripts/"
cp -p "${ROOT_DIR}/scripts/chimera-sh" "${RELEASE_DIR}/scripts/"
cp -p "${ROOT_DIR}/scripts/chimera-update.sh" "${RELEASE_DIR}/scripts/"
cp -p "${ROOT_DIR}/scripts/chimera-update-runtime-state.sh" "${RELEASE_DIR}/scripts/"
cp -p "${ROOT_DIR}/scripts/chimera-update-rerun.sh" "${RELEASE_DIR}/scripts/"
cp -p "${ROOT_DIR}/scripts/mesh_control_plane_env_from_preflight.sh" "${RELEASE_DIR}/scripts/"
cp -p "${ROOT_DIR}/scripts/chimera.sh" "${RELEASE_DIR}/scripts/"

sed -i \
  -e "s|^VERSION=.*|VERSION=\"${RELEASE_VERSION}\"|" \
  -e "s|^ARCHIVE_URL_DEFAULT=.*|ARCHIVE_URL_DEFAULT=\"https://github.com/neo-2022/chimera-pq/releases/latest/download/${LATEST_ARCHIVE_NAME}\"|" \
  -e "s|^CHECKSUM_URL_DEFAULT=.*|CHECKSUM_URL_DEFAULT=\"https://github.com/neo-2022/chimera-pq/releases/latest/download/${LATEST_CHECKSUM_NAME}\"|" \
  "${RELEASE_DIR}/scripts/chimera.sh"
cp -p "${RELEASE_DIR}/scripts/chimera.sh" "${ROOT_DIR}/target/chimera.sh"

printf '%s' "${RELEASE_VERSION}" > "${RELEASE_DIR}/.chimera_release_version"

echo "build_release: creating tarball"
tar -czf "${ROOT_DIR}/target/${ARCHIVE_NAME}" -C "${ROOT_DIR}/target" "chimera-release"
cp -p "${ROOT_DIR}/target/${ARCHIVE_NAME}" "${ROOT_DIR}/target/${LATEST_ARCHIVE_NAME}"

BUNDLE_SHA256="$(sha256sum "${ROOT_DIR}/target/${LATEST_ARCHIVE_NAME}" | cut -d' ' -f1)"
printf '%s  %s\n' "${BUNDLE_SHA256}" "${LATEST_ARCHIVE_NAME}" > "${ROOT_DIR}/target/${LATEST_CHECKSUM_NAME}"
cp -p "${ROOT_DIR}/target/${LATEST_CHECKSUM_NAME}" "${ROOT_DIR}/target/chimera-release.sha256"
(cd "${ROOT_DIR}/target" && sha256sum -c "${LATEST_CHECKSUM_NAME}")
tar -tzf "${ROOT_DIR}/target/${LATEST_ARCHIVE_NAME}" > "${ROOT_DIR}/target/chimera-release-contents.txt"
grep -q '^chimera-release/bin/chimera-bootstrap$' "${ROOT_DIR}/target/chimera-release-contents.txt"
grep -q '^chimera-release/scripts/install_release\.sh$' "${ROOT_DIR}/target/chimera-release-contents.txt"
grep -q '^chimera-release/scripts/chimera-update\.sh$' "${ROOT_DIR}/target/chimera-release-contents.txt"
grep -q '^chimera-release/scripts/chimera-update-runtime-state\.sh$' "${ROOT_DIR}/target/chimera-release-contents.txt"
grep -q '^chimera-release/scripts/chimera-update-rerun\.sh$' "${ROOT_DIR}/target/chimera-release-contents.txt"
grep -q '^chimera-release/scripts/chimera-sh$' "${ROOT_DIR}/target/chimera-release-contents.txt"
grep -q '^chimera-release/scripts/chimera\.sh$' "${ROOT_DIR}/target/chimera-release-contents.txt"
grep -q '^chimera-release/scripts/mesh_control_plane_env_from_preflight\.sh$' "${ROOT_DIR}/target/chimera-release-contents.txt"
grep -q '^chimera-release/configs/upstream_proxy\.env\.example$' "${ROOT_DIR}/target/chimera-release-contents.txt"

echo "build_release: done"
echo "  archive:   target/${ARCHIVE_NAME}"
echo "  latest:    target/${LATEST_ARCHIVE_NAME}"
echo "  bootstrap: target/chimera.sh"
echo "  size:      $(du -h "${ROOT_DIR}/target/${ARCHIVE_NAME}" | cut -f1)"
echo "  sha256:    ${BUNDLE_SHA256}"
echo "  contents:  ${RELEASE_DIR}/"
