# Contributing

Contributions should preserve Linux support, saved-settings compatibility, and the confirmed-write behaviour used for Homebridge commands.

## Development checks

```bash
cargo fmt --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
node --check assets/propertyInspector/openhome.js
node test/test_property_inspector.js
python3 -m unittest discover -v
```

Use the included mock Homebridge server for API work:

```bash
./scripts/test-mock.sh
```

## Pull requests

Describe the affected action and characteristic. Include tests for parsing, settings migration, authentication retry, and write confirmation when relevant. Never include live credentials, access tokens, private hostnames, or unredacted logs.
