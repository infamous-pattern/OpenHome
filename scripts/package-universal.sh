#!/usr/bin/env bash
set -euo pipefail

PLUGIN_ID="com.infamous-pattern.openhomeb"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INPUT_DIR="${1:?Usage: package-universal.sh DIRECTORY_CONTAINING_TARGET_FOLDERS}"
DIST_DIR="${ROOT_DIR}/dist"
PLUGIN_DIR="${DIST_DIR}/${PLUGIN_ID}.sdPlugin"
BINARY="openhomeb"

rm -rf "${PLUGIN_DIR}"
mkdir -p "${PLUGIN_DIR}"
cp -a "${ROOT_DIR}/assets/." "${PLUGIN_DIR}/"

for target in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
  source_binary="${INPUT_DIR}/${target}/${BINARY}"
  if [[ ! -f "${source_binary}" ]]; then
    echo "Missing release binary: ${source_binary}" >&2
    exit 1
  fi
  mkdir -p "${PLUGIN_DIR}/${target}/bin"
  install -m 0755 "${source_binary}" "${PLUGIN_DIR}/${target}/bin/${BINARY}"
done

ARCHIVE="${DIST_DIR}/${PLUGIN_ID}.streamDeckPlugin"
rm -f "${ARCHIVE}"
(
  cd "${DIST_DIR}"
  zip -qr "${ARCHIVE}" "${PLUGIN_ID}.sdPlugin"
)
printf 'Universal Linux package: %s\n' "${ARCHIVE}"
