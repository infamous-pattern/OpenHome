## Summary

Describe the change and the Homebridge/OpenDeck behaviour it affects.

## Validation

- [ ] `cargo fmt --check`
- [ ] `cargo test --all-targets`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `node test/test_property_inspector.js`
- [ ] `python3 -m unittest discover -v`
- [ ] Tested in OpenDeck on Linux, when applicable

## Security and compatibility

- [ ] No credentials or tokens are logged
- [ ] Existing saved action settings remain compatible, or migration is documented
- [ ] Homebridge writes are confirmed by a follow-up read
