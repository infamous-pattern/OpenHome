import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class PackageContractTests(unittest.TestCase):
    def test_manifest_exposes_version_2_action_set_and_linux_targets(self):
        manifest = json.loads((ROOT / "assets" / "manifest.json").read_text())
        action_ids = {action["UUID"] for action in manifest["Actions"]}
        self.assertEqual(
            action_ids,
            {
                "com.jamessenecal.openhome.devices",
                "com.jamessenecal.openhome.switch",
                "com.jamessenecal.openhome.brightness",
                "com.jamessenecal.openhome.set",
                "com.jamessenecal.openhome.adjust",
                "com.jamessenecal.openhome.config-ui",
            },
        )
        self.assertEqual(manifest["Name"], "OpenHome")
        self.assertEqual(manifest["Category"], "OpenHome")
        self.assertEqual(manifest["Version"], "2.0.0")
        self.assertEqual(manifest["OS"], [{"Platform": "linux"}])
        self.assertIn("x86_64-unknown-linux-gnu", manifest["CodePaths"])
        self.assertIn("aarch64-unknown-linux-gnu", manifest["CodePaths"])

        brightness = next(
            action for action in manifest["Actions"]
            if action["UUID"] == "com.jamessenecal.openhome.brightness"
        )
        self.assertEqual(set(brightness["Controllers"]), {"Keypad", "Encoder"})
        self.assertEqual(brightness["Encoder"]["layout"], "$B1")

    def test_manifest_assets_exist(self):
        manifest = json.loads((ROOT / "assets" / "manifest.json").read_text())
        for action in manifest["Actions"]:
            self.assertTrue((ROOT / "assets" / action["PropertyInspectorPath"]).is_file())
            for state in action.get("States", []):
                image = state.get("Image")
                if image:
                    self.assertTrue((ROOT / "assets" / f"{image}.png").is_file())
        self.assertTrue((ROOT / "assets" / "icons" / "brightness.png").is_file())
        self.assertTrue((ROOT / "assets" / "propertyInspector" / "openhome.js").is_file())
        self.assertTrue((ROOT / "assets" / "propertyInspector" / "openhome.css").is_file())

    def test_property_inspector_supports_all_control_modes(self):
        source = (ROOT / "assets" / "propertyInspector" / "openhome.js").read_text()
        html = (ROOT / "assets" / "propertyInspector" / "openhome.html").read_text()
        for action_kind in ("devices", "switch", "brightness", "set", "adjust", "openUi"):
            self.assertIn(action_kind, source)
        for function in (
            "requestCatalog",
            "requestBrightnessTest",
            "renderDiagnostics",
            "renderDeviceCards",
            "renderServiceOptions",
            "renderCharacteristicOptions",
            "parseCycleValues",
            "readTargetValue",
        ):
            self.assertIn(f"function {function}", source)
        self.assertIn("Refresh devices now", html)
        self.assertIn("Device cache (seconds)", html)
        self.assertIn("Brightness behaviour", html)

    def test_connection_fields_are_saved_only_by_explicit_connect(self):
        source = (ROOT / "assets" / "propertyInspector" / "openhome.js").read_text()
        html = (ROOT / "assets" / "propertyInspector" / "openhome.html").read_text()

        self.assertIn("Save and connect", html)
        self.assertIn("function markConnectionEdited", source)
        self.assertIn("function normaliseHomebridgeUrl", source)
        self.assertIn("function saveActionSettings", source)
        self.assertNotIn("function saveAllSettings", source)
        self.assertIn("elements[id].addEventListener('input', markConnectionEdited)", source)
        self.assertNotIn("'homebridgeUrl', 'username', 'password', 'updateInterval', 'displayName'", source)

    def test_catalog_refresh_reconciles_stale_selection(self):
        source = (ROOT / "assets" / "propertyInspector" / "openhome.js").read_text()
        self.assertIn("function reconcileActionSelection", source)
        self.assertIn("The previous service selection is no longer available and was cleared.", source)
        self.assertIn("The previous characteristic selection is no longer available and was cleared.", source)
        self.assertIn("actionSettings.characteristicUuid = ''", source)
        self.assertIn("if (reconciliation.changed) saveActionSettings();", source)

    def test_native_backend_contains_current_homebridge_operations(self):
        source = (ROOT / "src" / "homebridge.rs").read_text()
        for endpoint in (
            "/api/auth/noauth",
            "/api/auth/login",
            "/api/accessories",
            "/api/accessories/layout",
        ):
            self.assertIn(endpoint, source)
        self.assertIn(".put(&endpoint)", source)
        self.assertIn("StatusCode::UNAUTHORIZED", source)
        self.assertIn("utf8_percent_encode", source)
        self.assertIn("Homebridge URL must use http:// or https://", source)
        self.assertIn("query string or fragment", source)

    def test_token_refresh_is_proactive_and_401_safe(self):
        source = (ROOT / "src" / "homebridge.rs").read_text()
        self.assertIn("refresh_at", source)
        self.assertIn("approaching expiry; refreshing proactively", source)
        self.assertIn("refresh_buffer", source)
        self.assertGreaterEqual(source.count("for attempt in 0..2"), 2)
        self.assertGreaterEqual(source.count("StatusCode::UNAUTHORIZED && attempt == 0"), 2)
        self.assertGreaterEqual(source.count("self.tokens.write().await.remove(&key)"), 2)
        self.assertIn("authentication_status", source)

    def test_catalog_is_shared_cached_and_can_fall_back_stale(self):
        source = (ROOT / "src" / "homebridge.rs").read_text()
        models = (ROOT / "src" / "models.rs").read_text()
        inspector = (ROOT / "assets" / "propertyInspector" / "openhome.js").read_text()
        self.assertIn("catalogs: RwLock<HashMap<u64, CachedCatalog>>", source)
        self.assertIn("catalog_cache_ttl", models)
        self.assertIn("force_refresh", models)
        self.assertIn("returning stale cached catalog", source)
        self.assertIn("cache_age_seconds", models)
        self.assertIn("forceRefresh", inspector)
        self.assertIn("Shared cache", inspector)

    def test_compatibility_parser_is_conservative(self):
        source = (ROOT / "src" / "homebridge.rs").read_text()
        self.assertIn("parse_accessory_services", source)
        self.assertIn("normalise_service_value", source)
        self.assertIn("normalise_characteristic_value", source)
        self.assertIn("writableCharacteristics", source)
        self.assertIn("legacy_values_without_write_metadata_remain_read_only", source)

    def test_device_metadata_is_exposed_to_selector_and_cards(self):
        models = (ROOT / "src" / "models.rs").read_text()
        inspector = (ROOT / "assets" / "propertyInspector" / "openhome.js").read_text()
        self.assertIn("pub struct DeviceMetadata", models)
        self.assertIn("manufacturer", models)
        self.assertIn("serial_number", models)
        self.assertIn("item.deviceMetadata", inspector)
        self.assertIn("metadata.manufacturer", inspector)
        self.assertIn("metadata.model", inspector)

    def test_switch_action_keeps_strict_boolean_write_and_configurable_labels(self):
        inspector = (ROOT / "assets" / "propertyInspector" / "openhome.js").read_text()
        backend = (ROOT / "src" / "actions" / "switch.rs").read_text()
        models = (ROOT / "src" / "models.rs").read_text()

        self.assertIn("function isWritableBooleanCharacteristic", inspector)
        self.assertIn("characteristic.canRead && characteristic.canWrite", inspector)
        self.assertIn("pub fn is_switch_compatible", models)
        self.assertIn("query_display_state(&self.state, settings).await", backend)
        self.assertIn('"stateOnly"', backend)
        self.assertIn('"nameOnly"', backend)
        self.assertIn("show_confirmation", backend)

    def test_brightness_action_supports_key_encoder_and_confirmation(self):
        source = (ROOT / "src" / "actions" / "brightness.rs").read_text()
        models = (ROOT / "src" / "models.rs").read_text()
        state = (ROOT / "src" / "state.rs").read_text()
        poller = (ROOT / "src" / "poller.rs").read_text()

        self.assertIn("async fn key_down", source)
        self.assertIn("async fn dial_rotate", source)
        self.assertIn("async fn dial_down", source)
        self.assertIn("key_target", source)
        self.assertIn("next_cycle_value", source)
        self.assertIn("align_and_clamp", source)
        self.assertIn("Homebridge accepted the brightness write", source)
        self.assertIn("turn_on_when_adjusting", models)
        self.assertIn("brightness_settings", state)
        self.assertIn("BRIGHTNESS_UUID", poller)

    def test_characteristics_use_uuid_first_with_type_fallback(self):
        inspector = (ROOT / "assets" / "propertyInspector" / "openhome.js").read_text()
        models = (ROOT / "src" / "models.rs").read_text()
        self.assertIn("pub characteristic_uuid: String", models)
        self.assertIn("pub fn characteristic_by_identity", models)
        self.assertIn("item.uuid.eq_ignore_ascii_case", models)
        self.assertIn("self.characteristic(characteristic_type)", models)
        self.assertIn("function characteristicKey", inspector)
        self.assertIn("characteristicUuid", inspector)

    def test_openhome_branding_replaces_former_project_namespace(self):
        cargo = (ROOT / "Cargo.toml").read_text()
        self.assertIn('name = "openhome"', cargo)

        forbidden = (
            "OpenDeck " + "Homebridge",
            "OpenDeck-" + "Homebridge",
            "opendeck-" + "homebridge",
            "com.jamessenecal." + "homebridge",
        )
        text_suffixes = {".md", ".toml", ".json", ".rs", ".js", ".html", ".css", ".sh", ".yml", ".yaml", ".py"}
        for path in ROOT.rglob("*"):
            if not path.is_file() or path.suffix.lower() not in text_suffixes:
                continue
            content = path.read_text(errors="ignore")
            for value in forbidden:
                self.assertNotIn(value, content, f"Legacy project name remains in {path}")



if __name__ == "__main__":
    unittest.main()
