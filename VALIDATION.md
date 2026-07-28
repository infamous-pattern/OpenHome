# Validation status — version 2.0.1

Validated in the artifact-generation environment on 28 July 2026.

## Automated validation completed

- OpenDeck manifest JSON syntax and version `2.0.1`.
- Cargo package version and HTTP user agent set to `2.0.1`.
- Six action definitions, including the Keypad/Encoder **Brightness** action.
- Linux code paths for x86_64 and aarch64.
- Existence of every manifest icon and property-inspector asset.
- Property-inspector JavaScript syntax and behaviour tests.
- Python mock Homebridge server syntax and API/package-contract tests.
- Shell syntax for all build, mock-test, and universal-package scripts.
- GitHub Actions workflow YAML syntax and Node.js 24-compatible checkout actions.
- Universal Linux package staging for x86_64 and aarch64 using non-release placeholder executables.
- Source archive layout and checksum generation.

## Automated API and package-contract tests

Twenty tests passed, covering authentication, discovery, reads, writes, brightness control, token renewal, HTTP 401 retry, catalogue caching, stale-cache fallback, UUID-first characteristic matching, compatibility parsing, metadata, action-selection reconciliation, label options, package structure, and OpenHomeB branding and publisher-neutral identifiers.

## Rust release-validation corrections

Version 2.0.1 carries forward the release-build fixes for:

- unused action UUID re-exports;
- the unused `Characteristic` import;
- Clippy `collapsible_if` findings that were promoted to errors by `-D warnings`;
- GitHub Actions checkout runtime compatibility;
- separate validation steps for clearer failure reporting.

## Native compile status

The Rust executable was not compiled in this artifact environment because a Rust toolchain and Cargo registry access were unavailable. Native compile-time checks are configured in `.github/workflows/ci.yml` and `.github/workflows/release.yml`:

```text
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

The Fedora build script performs a native release build, and the tagged GitHub Actions workflow builds x86_64 and aarch64 binaries before creating the installable universal `.streamDeckPlugin` asset.
