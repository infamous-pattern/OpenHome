# OpenHome 2.0.0

OpenHome 2.0 is a Linux-native OpenAction plugin for discovering and controlling accessories exposed by Homebridge Config UI.

## Highlights

- Dedicated Brightness action for keys and encoders
- Reliable Switch toggling with confirmed post-write state
- Shared Homebridge connection and device catalog cache
- Proactive token renewal and automatic retry after HTTP 401
- Room and device-type filtering
- Manufacturer, model, serial number, and firmware metadata
- Configurable button labels and command confirmation
- Native x86_64 and aarch64 Linux release assets

## Actions

- **OpenHome Devices:** Discover devices, services, characteristics, rooms, and metadata.
- **Switch:** Toggle a writable Boolean characteristic and display live On/Off state.
- **Brightness:** Increase, decrease, set, or cycle brightness from a key or encoder.
- **Set State:** Write a fixed Boolean, numeric, or string value.
- **Adjust State:** Adjust a writable numeric characteristic with an encoder.
- **Open Homebridge UI:** Open the configured Homebridge UI in the default Linux browser.

## Installation

Download `openhome-2.0.0-linux-universal.streamDeckPlugin` from the release assets and install it from OpenDeck's Plugins screen. Restart OpenDeck after replacing an older build.

Homebridge Config UI must be reachable. Homebridge must run in insecure mode (`-I`) for accessory reads and writes. Keep the Config UI port restricted to a trusted network.

## Upgrade notes

The project is now named **OpenHome** and uses the new plugin/action namespace `com.jamessenecal.openhome`. Because OpenDeck identifies actions by UUID, actions created with the former namespace are not migrated automatically. Install OpenHome, recreate each action, select the Homebridge service again, verify control, and then remove the former package.

## Release assets

- Universal OpenDeck package with x86_64 and aarch64 binaries
- Standalone binaries for both supported architectures
- Source archives in ZIP and tar.gz formats
- `SHA256SUMS` for integrity verification

## Known limitations

- Homebridge credentials are stored through OpenDeck's global plugin settings API. Protect the Linux account and OpenDeck configuration directory.
- Homebridge's accessory API requires insecure mode (`-I`), despite still requiring Config UI authentication when enabled.
