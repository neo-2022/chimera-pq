#!/usr/bin/env bash

runtime_version_file() {
  printf '%s\n' "${ROOT_DIR}/.chimera_release_version"
}

runtime_bundle_sha_file() {
  printf '%s\n' "${ROOT_DIR}/.chimera_release_bundle.sha256"
}

runtime_release_archive_file() {
  printf '%s\n' "${ROOT_DIR}/releases/chimera-pq-release.tar.gz"
}

runtime_release_bundle_sha_file() {
  printf '%s\n' "${ROOT_DIR}/releases/chimera-pq-release.tar.gz.sha256"
}

read_local_runtime_version_from_release_bundle() {
  local version_file archive_file archive_bundle_sha_file expected actual version
  version_file="$(runtime_version_file)"
  archive_file="$(runtime_release_archive_file)"
  archive_bundle_sha_file="$(runtime_release_bundle_sha_file)"
  [[ -f "$archive_file" && -f "$archive_bundle_sha_file" ]] || return 1
  command -v tar >/dev/null 2>&1 || return 1
  expected="$(awk '{print $1}' "$archive_bundle_sha_file" | tr -d '[:space:]')"
  [[ -n "$expected" ]] || return 1
  actual="$(sha256_file "$archive_file")" || return 1
  [[ "$actual" == "$expected" ]] || return 1
  version="$(tar -xOf "$archive_file" chimera-release/.chimera_release_version 2>/dev/null | head -n 1 | tr -d '[:space:]')"
  [[ -n "$version" ]] || return 1
  if ! printf '%s\n' "$version" > "$version_file" 2>/dev/null; then
    :
  fi
  printf '%s\n' "$version"
}

install_node_role_file() {
  printf '%s\n' "${ROOT_DIR}/.chimera_install_role"
}

sha256_file() {
  local file="${1:?file_required}"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
    return 0
  fi
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{print $1}'
    return 0
  fi
  echo "error: missing sha256 tool: sha256sum or shasum" >&2
  return 1
}

read_local_runtime_bundle_sha() {
  local bundle_sha_file archive_file archive_bundle_sha_file expected actual
  bundle_sha_file="$(runtime_bundle_sha_file)"
  if [[ -f "$bundle_sha_file" ]]; then
    tr -d '[:space:]' < "$bundle_sha_file"
    return 0
  fi
  archive_file="$(runtime_release_archive_file)"
  archive_bundle_sha_file="$(runtime_release_bundle_sha_file)"
  if [[ -f "$archive_file" && -f "$archive_bundle_sha_file" ]]; then
    expected="$(awk '{print $1}' "$archive_bundle_sha_file" | tr -d '[:space:]')"
    [[ -n "$expected" ]] || return 0
    actual="$(sha256_file "$archive_file")" || return 0
    if [[ "$actual" != "$expected" ]]; then
      return 0
    fi
    if ! printf '%s\n' "$actual" > "$bundle_sha_file" 2>/dev/null; then
      :
    fi
    printf '%s\n' "$actual"
    return 0
  fi
  echo ""
}
