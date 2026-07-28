# OpenHomeB 2.0.1

OpenHomeB 2.0.1 is a Linux-native OpenAction plugin for discovering and controlling accessories exposed by Homebridge Config UI.

## What changed

- Renamed the project, plugin, binary, property-inspector assets, release files, and visible branding to **OpenHomeB**.
- Changed the plugin identifier and complete action namespace to `com.infamous-pattern.openhomeb`.
- Removed personal-name references from source code, package metadata, documentation, licence notices, workflows, and tests.
- Retained the Clippy and GitHub Actions validation fixes included in the 2.0.1 codebase.
- Retained the complete 2.0 feature set and Homebridge behaviour.

## Included functionality

- **OpenHomeB Devices:** Discover devices, services, characteristics, rooms, and metadata.
- **Switch:** Toggle a writable Boolean characteristic and display live On/Off state.
- **Brightness:** Increase, decrease, set, or cycle brightness from a key or encoder.
- **Set State:** Write a fixed Boolean, numeric, or string value.
- **Adjust State:** Adjust a writable numeric characteristic with an encoder.
- **Open Homebridge UI:** Open the configured Homebridge UI in the default Linux browser.
- Shared catalogue caching, proactive token renewal, HTTP 401 retry, room/device filtering, and confirmed writes.

## Installation

Download `openhomeb-2.0.1-linux-universal.streamDeckPlugin` from the GitHub release assets and install it from OpenDeck's Plugins screen. Restart OpenDeck after replacing an older build.

Homebridge Config UI must be reachable. Homebridge must run in insecure mode (`-I`) for accessory reads and writes. Keep the Config UI port restricted to a trusted network.

## Upgrade notes

The new identifier is `com.infamous-pattern.openhomeb`. OpenDeck treats actions registered under this namespace as new actions. Re-add each action and select its Homebridge service and characteristic again.

## Release assets

- Universal OpenDeck package with x86_64 and aarch64 binaries
- Standalone binaries for both supported architectures
- Source archives in ZIP and tar.gz formats
- `SHA256SUMS` for integrity verification

## Known limitations

- Homebridge credentials are stored through OpenDeck's global plugin settings API. Protect the Linux account and OpenDeck configuration directory.
- Homebridge's accessory API requires insecure mode (`-I`), despite still requiring Config UI authentication when enabled.
