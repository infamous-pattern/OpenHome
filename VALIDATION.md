# Validation status — version 2.0.0

Validated in the artifact-generation environment on 27 July 2026.

## Automated validation completed

- OpenDeck manifest JSON syntax and version `2.0.0`.
- Six action definitions, including the new Keypad/Encoder **Brightness** action.
- Linux code paths for x86_64 and aarch64.
- Existence of every manifest icon and property-inspector asset.
- Property-inspector JavaScript syntax.
- Property-inspector URL normalisation, characteristic field mapping, and brightness-preset parsing.
- Python mock Homebridge server syntax.
- Shell syntax for all build, mock-test, and universal-package scripts.
- Universal Linux package staging for x86_64 and aarch64 using non-release placeholder executables.
- Source archive layout and checksum generation.

## Automated API and package-contract tests

Twenty tests passed, covering:

- no-auth token acquisition;
- username/password authentication success and failure;
- accessory and room-layout discovery;
- individual service retrieval;
- Boolean characteristic writes and confirmation;
- Brightness writes and updated-state retrieval;
- read-only characteristic rejection;
- proactive token refresh metadata;
- automatic retry after HTTP 401;
- shared catalogue caching and forced refresh;
- stale-cache fallback;
- UUID-first characteristic matching with type fallback;
- conservative parsing of current and selected legacy response structures;
- manufacturer/model/serial/firmware metadata exposure;
- explicit connection saving rather than saving while typing;
- stale action-selection reconciliation;
- configurable Switch labels and confirmation feedback;
- Brightness key, encoder, preset-cycle, bounds, and confirmation behaviour;
- manifest action parity and Linux target structure.
- complete OpenHome branding and namespace replacement, including package IDs and property-inspector paths.

## Native compile status

The Rust executable was not compiled in this environment because the Rust toolchain and Cargo crate cache were unavailable, and outbound package-registry DNS access is disabled. Native compile-time checks are configured in `.github/workflows/ci.yml` and `.github/workflows/release.yml`:

```text
cargo fmt --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

The Fedora build script performs the native release build and creates the installable `.streamDeckPlugin` package.
