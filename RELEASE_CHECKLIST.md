# Release checklist — 2.0.0

## Repository

- [x] Version is `2.0.0` in Cargo and OpenDeck manifest metadata.
- [x] README includes overview, actions, installation, screenshots, and security guidance.
- [x] Installation, contribution, security, release, changelog, licence, and attribution documents are present.
- [x] Bug, feature, and pull-request templates are present.

## Validation

- [x] Property-inspector JavaScript syntax test passes.
- [x] Property-inspector behaviour tests pass.
- [x] Twenty Homebridge API and package-contract tests pass.
- [x] Manifest JSON parses successfully.
- [x] GitHub workflow YAML parses successfully.
- [x] Shell scripts pass `bash -n`.

## GitHub publication

- [ ] Create the GitHub repository and push `main`.
- [ ] Push annotated tag `v2.0.0`.
- [ ] Confirm CI completes.
- [ ] Confirm the release workflow attaches x86_64, aarch64, universal plugin, source, release notes, and `SHA256SUMS` assets.
- [ ] Test the universal package on Fedora Workstation 44 with OpenDeck.
