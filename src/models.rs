use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GlobalSettings {
    pub homebridge_url: String,
    pub username: String,
    pub password: String,
    pub update_interval: u64,
    pub catalog_cache_seconds: u64,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            homebridge_url: "http://homebridge.local:8581".to_string(),
            username: String::new(),
            password: String::new(),
            update_interval: 5,
            catalog_cache_seconds: 60,
        }
    }
}

impl GlobalSettings {
    pub fn polling_interval(&self) -> u64 {
        self.update_interval.clamp(1, 3_600)
    }

    pub fn catalog_cache_ttl(&self) -> u64 {
        self.catalog_cache_seconds.clamp(5, 3_600)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct EmptySettings {}

fn default_switch_label_mode() -> String {
    "nameAndState".to_string()
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SwitchSettings {
    pub accessory_id: String,
    pub characteristic_type: String,
    pub characteristic_uuid: String,
    pub display_name: String,
    pub label_mode: String,
    pub show_confirmation: bool,
}

impl Default for SwitchSettings {
    fn default() -> Self {
        Self {
            accessory_id: String::new(),
            characteristic_type: String::new(),
            characteristic_uuid: String::new(),
            display_name: String::new(),
            label_mode: default_switch_label_mode(),
            show_confirmation: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SetStateSettings {
    pub accessory_id: String,
    pub characteristic_type: String,
    pub characteristic_uuid: String,
    pub target_value: Value,
    pub display_name: String,
}

impl Default for SetStateSettings {
    fn default() -> Self {
        Self {
            accessory_id: String::new(),
            characteristic_type: String::new(),
            characteristic_uuid: String::new(),
            target_value: Value::Null,
            display_name: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AdjustStateSettings {
    pub accessory_id: String,
    pub characteristic_type: String,
    pub characteristic_uuid: String,
    pub speed: f64,
    pub display_name: String,
}

impl Default for AdjustStateSettings {
    fn default() -> Self {
        Self {
            accessory_id: String::new(),
            characteristic_type: String::new(),
            characteristic_uuid: String::new(),
            speed: 1.0,
            display_name: String::new(),
        }
    }
}

fn default_brightness_mode() -> String {
    "increase".to_string()
}

fn default_brightness_increment() -> f64 {
    10.0
}

fn default_brightness_target() -> f64 {
    50.0
}

fn default_brightness_cycle_values() -> Vec<f64> {
    vec![25.0, 50.0, 75.0, 100.0]
}

fn default_brightness_label_mode() -> String {
    "nameAndValue".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrightnessSettings {
    pub accessory_id: String,
    pub characteristic_type: String,
    pub characteristic_uuid: String,
    pub display_name: String,
    pub mode: String,
    pub increment: f64,
    pub target_value: f64,
    pub cycle_values: Vec<f64>,
    pub wrap: bool,
    pub label_mode: String,
    pub turn_on_when_adjusting: bool,
    pub show_confirmation: bool,
}

impl Default for BrightnessSettings {
    fn default() -> Self {
        Self {
            accessory_id: String::new(),
            characteristic_type: "Brightness".to_string(),
            characteristic_uuid: String::new(),
            display_name: String::new(),
            mode: default_brightness_mode(),
            increment: default_brightness_increment(),
            target_value: default_brightness_target(),
            cycle_values: default_brightness_cycle_values(),
            wrap: true,
            label_mode: default_brightness_label_mode(),
            turn_on_when_adjusting: true,
            show_confirmation: true,
        }
    }
}

pub trait SelectedCharacteristic {
    fn accessory_id(&self) -> &str;
    fn characteristic_type(&self) -> &str;
    fn characteristic_uuid(&self) -> &str;
    fn is_configured(&self) -> bool {
        !self.accessory_id().trim().is_empty()
            && (!self.characteristic_type().trim().is_empty()
                || !self.characteristic_uuid().trim().is_empty())
    }
}

macro_rules! impl_selected_characteristic {
    ($type_name:ty) => {
        impl SelectedCharacteristic for $type_name {
            fn accessory_id(&self) -> &str {
                &self.accessory_id
            }

            fn characteristic_type(&self) -> &str {
                &self.characteristic_type
            }

            fn characteristic_uuid(&self) -> &str {
                &self.characteristic_uuid
            }
        }
    };
}

impl_selected_characteristic!(SwitchSettings);
impl_selected_characteristic!(SetStateSettings);
impl_selected_characteristic!(AdjustStateSettings);
impl_selected_characteristic!(BrightnessSettings);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Characteristic {
    #[serde(rename = "type", alias = "characteristicType")]
    pub characteristic_type: String,
    pub uuid: String,
    pub description: String,
    pub value: Value,
    pub format: String,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub min_step: Option<f64>,
    pub can_read: bool,
    pub can_write: bool,
    pub valid_values: Vec<Value>,
}

impl Default for Characteristic {
    fn default() -> Self {
        Self {
            characteristic_type: String::new(),
            uuid: String::new(),
            description: String::new(),
            value: Value::Null,
            format: String::new(),
            min_value: None,
            max_value: None,
            min_step: None,
            can_read: false,
            can_write: false,
            valid_values: Vec::new(),
        }
    }
}

impl Characteristic {
    pub fn is_boolean(&self) -> bool {
        self.format.eq_ignore_ascii_case("bool") || self.value.is_boolean()
    }

    pub fn is_switch_compatible(&self) -> bool {
        self.can_read && self.can_write && self.is_boolean()
    }

    pub fn is_numeric(&self) -> bool {
        matches!(
            self.format.to_ascii_lowercase().as_str(),
            "int" | "float" | "uint8" | "uint16" | "uint32" | "uint64"
        ) || self.value.is_number()
    }

    pub fn is_integer(&self) -> bool {
        matches!(
            self.format.to_ascii_lowercase().as_str(),
            "int" | "uint8" | "uint16" | "uint32" | "uint64"
        )
    }

    pub fn is_brightness_compatible(&self) -> bool {
        self.can_read
            && self.can_write
            && self.is_numeric()
            && (self.characteristic_type.eq_ignore_ascii_case("Brightness")
                || self.description.eq_ignore_ascii_case("Brightness"))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HomebridgeInstance {
    pub name: String,
    pub username: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AccessoryService {
    pub aid: Option<u64>,
    pub iid: Option<u64>,
    pub uuid: String,
    #[serde(rename = "type")]
    pub raw_type: String,
    pub human_type: String,
    pub service_name: String,
    pub service_type: String,
    pub unique_id: String,
    pub accessory_name: String,
    pub accessory_information: Value,
    pub service_characteristics: Vec<Characteristic>,
    pub instance: HomebridgeInstance,
}

impl AccessoryService {
    pub fn normalise(mut self) -> Self {
        if self.service_type.trim().is_empty() {
            self.service_type = if !self.human_type.trim().is_empty() {
                self.human_type.clone()
            } else if !self.raw_type.trim().is_empty() {
                self.raw_type.clone()
            } else {
                "Unknown service".to_string()
            };
        }

        if self.service_name.trim().is_empty() {
            self.service_name = self
                .accessory_information
                .get("Name")
                .and_then(Value::as_str)
                .unwrap_or("Unnamed service")
                .to_string();
        }

        if self.accessory_name.trim().is_empty() {
            self.accessory_name = self
                .accessory_information
                .get("Name")
                .and_then(Value::as_str)
                .unwrap_or(&self.service_name)
                .to_string();
        }

        if self.unique_id.trim().is_empty() {
            self.unique_id = if !self.uuid.trim().is_empty() {
                self.uuid.clone()
            } else if let (Some(aid), Some(iid)) = (self.aid, self.iid) {
                format!("{aid}.{iid}")
            } else {
                format!("{}:{}", self.accessory_name, self.service_name)
            };
        }

        self
    }

    pub fn characteristic(&self, characteristic_type: &str) -> Option<&Characteristic> {
        self.service_characteristics.iter().find(|characteristic| {
            characteristic
                .characteristic_type
                .eq_ignore_ascii_case(characteristic_type)
        })
    }

    pub fn characteristic_by_identity(
        &self,
        characteristic_uuid: &str,
        characteristic_type: &str,
    ) -> Option<&Characteristic> {
        if !characteristic_uuid.trim().is_empty() {
            if let Some(characteristic) = self.service_characteristics.iter().find(|item| {
                item.uuid.eq_ignore_ascii_case(characteristic_uuid.trim())
            }) {
                return Some(characteristic);
            }
        }

        self.characteristic(characteristic_type)
    }

    pub fn available_characteristic_types(&self) -> String {
        let values = self
            .service_characteristics
            .iter()
            .map(|item| {
                if item.uuid.trim().is_empty() {
                    item.characteristic_type.clone()
                } else {
                    format!("{} ({})", item.characteristic_type, item.uuid)
                }
            })
            .collect::<Vec<_>>();

        if values.is_empty() {
            "none".to_string()
        } else {
            values.join(", ")
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RoomLayout {
    pub name: String,
    pub services: Vec<LayoutService>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LayoutService {
    pub unique_id: String,
    pub custom_name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceMetadata {
    pub manufacturer: String,
    pub model: String,
    pub serial_number: String,
    pub firmware_revision: String,
}

impl DeviceMetadata {
    pub fn from_accessory_information(value: &Value) -> Self {
        Self {
            manufacturer: first_string(value, &["Manufacturer", "manufacturer"]),
            model: first_string(value, &["Model", "model"]),
            serial_number: first_string(
                value,
                &["Serial Number", "SerialNumber", "serialNumber", "serial_number"],
            ),
            firmware_revision: first_string(
                value,
                &["Firmware Revision", "FirmwareRevision", "firmwareRevision"],
            ),
        }
    }
}

fn first_string(value: &Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .unwrap_or_default()
        .to_string()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogService {
    pub room_name: String,
    pub custom_name: Option<String>,
    pub device_metadata: DeviceMetadata,
    pub service: AccessoryService,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationStatus {
    pub method: String,
    pub authenticated_at_epoch_ms: u64,
    pub expires_at_epoch_ms: u64,
    pub refresh_at_epoch_ms: u64,
    pub remaining_seconds: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub authentication: String,
    pub authentication_status: AuthenticationStatus,
    pub device_count: usize,
    pub service_count: usize,
    pub rooms: Vec<String>,
    pub services: Vec<CatalogService>,
    pub refreshed_at_epoch_ms: u64,
    pub cache_age_seconds: u64,
    pub cached: bool,
    pub stale: bool,
    pub warning: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PropertyInspectorRequest {
    pub event: String,
    pub global_settings: Option<GlobalSettings>,
    pub action_settings: Value,
    pub otp: Option<String>,
    pub force_refresh: bool,
}

impl Default for PropertyInspectorRequest {
    fn default() -> Self {
        Self {
            event: String::new(),
            global_settings: None,
            action_settings: Value::Null,
            otp: None,
            force_refresh: false,
        }
    }
}
