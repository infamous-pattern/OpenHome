#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-2.0.1}"
PLUGIN_ID="com.infamous-pattern.openhomeb"
BINARY="openhomeb"
ASSET_DIR="${ROOT_DIR}/release-assets"

rm -rf "${ASSET_DIR}"
mkdir -p "${ASSET_DIR}"

package="${ROOT_DIR}/dist/${PLUGIN_ID}.streamDeckPlugin"
[[ -f "${package}" ]] || {
  echo "Missing ${package}. Build or package the plugin first." >&2
  exit 1
}

cp "${package}" "${ASSET_DIR}/openhomeb-${VERSION}-linux-universal.streamDeckPlugin"

for target in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
  binary="${ROOT_DIR}/release/${target}/${BINARY}"
  if [[ -f "${binary}" ]]; then
    cp "${binary}" "${ASSET_DIR}/${BINARY}-${VERSION}-${target}"
  fi
done

(
  cd "${ROOT_DIR}"
  zip -qr "${ASSET_DIR}/openhomeb-${VERSION}-source.zip" \
    . -x '.git/*' 'target/*' 'dist/*' 'release/*' 'release-assets/*'
)

notes_file="${ROOT_DIR}/RELEASE_NOTES_${VERSION}.md"
[[ -f "${notes_file}" ]] || {
  echo "Missing release notes: ${notes_file}" >&2
  exit 1
}
cp "${notes_file}" "${ASSET_DIR}/"
(
  cd "${ASSET_DIR}"
  sha256sum * > SHA256SUMS
)

echo "Release assets prepared in ${ASSET_DIR}"
