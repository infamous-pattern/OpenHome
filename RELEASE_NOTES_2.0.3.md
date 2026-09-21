# OpenHomeB 2.0.3

OpenHomeB 2.0.3 is a security maintenance release.

## What changed

- Updated rustls from 0.23.42 to 0.23.45 to fix [RUSTSEC-2026-0285](https://rustsec.org/advisories/RUSTSEC-2026-0285.html), concerning TLS 1.3 handshake encryption-level validation.
- Updated rustls-webpki from 0.103.13 to 0.103.15 as required by the patched rustls dependency.
- Rebuilt x86_64 and aarch64 Linux binaries and the universal plugin package with the fix.

## Installation

Download `openhomeb-2.0.3-linux-universal.streamDeckPlugin` from this release and install it through OpenDeck's Plugins screen. Restart OpenDeck after replacing the older build.

Action identifiers and saved-settings formats are unchanged from 2.0.2.

## Audit note

The dependency audit reports no vulnerabilities. It retains an allowed warning for yanked `chacha20 0.10.1`, which belongs to an optional QUIC dependency chain that is inactive with this project's enabled features.
