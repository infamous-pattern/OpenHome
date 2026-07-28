#!/usr/bin/env bash
set -euo pipefail

PLUGIN_ID="com.infamous-pattern.openhomeb"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"
PLUGIN_DIR="${DIST_DIR}/${PLUGIN_ID}.sdPlugin"
BINARY="openhomeb"
TARGET="${1:-}"

for command in cargo rustc zip; do
  if ! command -v "${command}" >/dev/null 2>&1; then
    echo "Missing required command: ${command}" >&2
    echo "On Fedora Workstation 44, install prerequisites with:" >&2
    echo "  sudo dnf install -y rust cargo gcc zip" >&2
    exit 1
  fi
done

if [[ -z "${TARGET}" ]]; then
  TARGET="$(rustc -vV | sed -n 's/^host: //p')"
fi

case "${TARGET}" in
  x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu) ;;
  *)
    echo "Unsupported target: ${TARGET}" >&2
    echo "Supported Linux targets: x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu" >&2
    exit 1
    ;;
esac

cd "${ROOT_DIR}"
echo "Building ${BINARY} for ${TARGET}..."
cargo build --locked --release --target "${TARGET}" 2>/dev/null || cargo build --release --target "${TARGET}"

rm -rf "${PLUGIN_DIR}"
mkdir -p "${PLUGIN_DIR}/${TARGET}/bin"
cp -a "${ROOT_DIR}/assets/." "${PLUGIN_DIR}/"
install -m 0755 \
  "${ROOT_DIR}/target/${TARGET}/release/${BINARY}" \
  "${PLUGIN_DIR}/${TARGET}/bin/${BINARY}"

ARCHIVE="${DIST_DIR}/${PLUGIN_ID}.streamDeckPlugin"
rm -f "${ARCHIVE}"
(
  cd "${DIST_DIR}"
  zip -qr "${ARCHIVE}" "${PLUGIN_ID}.sdPlugin"
)

echo
printf 'Target: %s\n' "${TARGET}"
printf 'Plugin folder: %s\n' "${PLUGIN_DIR}"
printf 'Installable archive: %s\n' "${ARCHIVE}"
