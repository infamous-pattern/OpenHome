# Changelog

## 2.0.0 — 2026-07-27

- Added a dedicated Brightness action supporting OpenDeck keys and encoders.
- Added increase, decrease, fixed-value, and preset-cycle key/encoder-press modes.
- Added relative encoder rotation with Homebridge min/max/min-step enforcement.
- Added optional automatic power-on when increasing brightness.
- Added post-write brightness confirmation and live polling.
- Added configurable Switch and Brightness label modes and optional success feedback.
- Added proactive token renewal before reported expiry while retaining one retry after HTTP 401.
- Added authentication and token timing diagnostics in the property inspector.
- Added a shared, configurable device-catalogue cache across all action instances.
- Added manual cache-bypassing refresh and stale-cache fallback.
- Added manufacturer, model, serial number, and firmware metadata.
- Added room and device-category filters.
- Added conservative parsing for current and selected legacy Homebridge accessory response structures.
- Updated the mock server and automated tests for brightness writes and the complete 2.0 feature contract.

## 0.1.4 — 2026-07-27

- Fixed Homebridge characteristic mapping from the serialized `type` field.
- Automatically selected the sole compatible Boolean characteristic for Switch actions.
- Accepted UUID-only legacy selections and resolved the canonical type before writes.

## 0.1.3 — 2026-07-27

- Moved Switch toggling to `keyDown`.
- Added detailed write and confirmation logging.
- Added service-ID URL encoding and a property-inspector Switch test command.

## 0.1.2 — 2026-07-27

- Stopped saving connection fields while typing.
- Added URL validation, UUID-first selection, stale-selection reconciliation, polling safeguards, and deduplicated errors.

## 0.1.1 — 2026-07-27

- Added strict Boolean Switch filtering, current-state display, post-write refresh, and automatic reauthentication after HTTP 401.

## 0.1.0 — 2026-07-27

- Initial Linux-native OpenDeck release with discovery, Switch, Set State, Adjust State, UI launch, authentication, polling, and Fedora packaging.
