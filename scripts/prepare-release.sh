#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-2.0.0}"
PLUGIN_ID="com.jamessenecal.openhome"
BINARY="openhome"
ASSET_DIR="${ROOT_DIR}/release-assets"

rm -rf "${ASSET_DIR}"
mkdir -p "${ASSET_DIR}"

package="${ROOT_DIR}/dist/${PLUGIN_ID}.streamDeckPlugin"
[[ -f "${package}" ]] || {
  echo "Missing ${package}. Build or package the plugin first." >&2
  exit 1
}

cp "${package}" "${ASSET_DIR}/openhome-${VERSION}-linux.streamDeckPlugin"

for target in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
  binary="${ROOT_DIR}/release/${target}/${BINARY}"
  if [[ -f "${binary}" ]]; then
    cp "${binary}" "${ASSET_DIR}/${BINARY}-${VERSION}-${target}"
  fi
done

(
  cd "${ROOT_DIR}"
  zip -qr "${ASSET_DIR}/openhome-${VERSION}-source.zip" \
    . -x '.git/*' 'target/*' 'dist/*' 'release/*' 'release-assets/*'
)

cp "${ROOT_DIR}/RELEASE_NOTES_2.0.0.md" "${ASSET_DIR}/"
(
  cd "${ASSET_DIR}"
  sha256sum * > SHA256SUMS
)

echo "Release assets prepared in ${ASSET_DIR}"
