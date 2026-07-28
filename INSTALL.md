# Installation

## Release package

1. Download the universal `.streamDeckPlugin` release asset.
2. Open OpenDeck and go to **Plugins**.
3. Install the downloaded package.
4. Restart OpenDeck.
5. Add an OpenHomeB action to a key or encoder.
6. Enter the Homebridge Config UI address, usually `http://homebridge.local:8581`.
7. Enter credentials only when Config UI authentication is enabled.
8. Select **Save and connect**.

## Homebridge requirement

Homebridge must run with the `-I` argument so Homebridge Config UI can read and write accessory characteristics. Keep TCP port `8581` available only to trusted devices on the local network.

## Build from source on Fedora Workstation 44

```bash
sudo dnf install -y rust cargo gcc zip git

git clone <repository-url>
cd openhomeb
chmod +x scripts/*.sh
./scripts/build-fedora.sh
```

The package is created at:

```text
dist/com.infamous-pattern.openhomeb.streamDeckPlugin
```

## Verify a downloaded release

```bash
sha256sum -c SHA256SUMS --ignore-missing
```

## Migration from an earlier package

OpenHomeB uses the plugin identifier `com.infamous-pattern.openhomeb`. Existing actions created under another identifier are not automatically reassigned by OpenDeck. Install OpenHomeB, add the replacement actions, select the Homebridge services again, verify operation, and then remove the earlier package.
