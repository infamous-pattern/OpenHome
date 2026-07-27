# Migrating to OpenHome

The project, binary, package identifier, action UUIDs, property-inspector assets, release filenames, documentation, and screenshots have been renamed to **OpenHome**.

## New identifiers

- Plugin ID: `com.jamessenecal.openhome`
- Binary: `openhome`
- Release package: `openhome-<version>-linux-universal.streamDeckPlugin`
- Action namespace: `com.jamessenecal.openhome.*`

## Existing OpenDeck buttons

OpenDeck binds configured buttons to action UUIDs. Since the namespace changed, buttons created with the former package do not automatically become OpenHome actions.

1. Install OpenHome.
2. Add the equivalent OpenHome action to each key or encoder.
3. Select the Homebridge service and characteristic again.
4. Verify reads and writes.
5. Remove the former package after migration is complete.

Homebridge itself does not need to be reconfigured.
