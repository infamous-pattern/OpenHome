# OpenHomeB 2.0.2

OpenHomeB 2.0.2 is a reliability release focused on startup and reconnect behaviour on Linux/OpenDeck systems.

## What changed

- Added automatic Homebridge recovery after Fedora login and OpenDeck startup.
- Waits 3 seconds after global settings arrive before the first connection attempt so NetworkManager and local DNS have time to settle.
- Retries unavailable Homebridge servers with bounded backoff: 2, 5, 10, 30, then 60 seconds.
- Performs a live Homebridge catalogue health check every 60 seconds; stale cached data cannot be mistaken for a healthy connection.
- Automatically reauthenticates through the existing token flow when required.
- Refreshes visible Devices, Switch, Adjust State, and Brightness actions immediately after a successful reconnect.
- Requests an immediate reconnect after system wake or a connection-related action failure.
- Preserves the last-known button state during background outages instead of repeatedly replacing it with an Offline title.
- Suppresses duplicate reconnect messages once the retry interval reaches the steady-state 60-second cadence.

## Existing functionality retained

- Homebridge device discovery and room-aware selectors.
- Switch state query, toggle, confirmation, and automatic 401 reauthentication.
- Brightness control from keys and encoders.
- Set State and Adjust State controls.
- Shared catalogue caching and stale-cache fallback for the property inspector.
- Proactive authentication-token renewal.
- x86_64 and aarch64 Linux packaging.

## Installation

Download `openhomeb-2.0.2-linux-universal.streamDeckPlugin` from the GitHub release assets and install it through OpenDeck's Plugins screen. Restart OpenDeck after replacing an older build.

Existing OpenHomeB 2.0.1 action UUIDs and settings are unchanged, so configured buttons should carry forward without being recreated.
