use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::homebridge::HomebridgeClient;
use crate::models::{
    AdjustStateSettings, BrightnessSettings, GlobalSettings, SwitchSettings,
};

pub struct PluginState {
    pub client: HomebridgeClient,
    global_settings: RwLock<GlobalSettings>,
    switch_settings: RwLock<HashMap<String, SwitchSettings>>,
    adjust_settings: RwLock<HashMap<String, AdjustStateSettings>>,
    brightness_settings: RwLock<HashMap<String, BrightnessSettings>>,
    last_errors: RwLock<HashMap<String, String>>,
    invalid_switches: RwLock<HashSet<String>>,
    invalid_brightness: RwLock<HashSet<String>>,
}

impl PluginState {
    pub fn new() -> anyhow::Result<Arc<Self>> {
        Ok(Arc::new(Self {
            client: HomebridgeClient::new()?,
            global_settings: RwLock::new(GlobalSettings::default()),
            switch_settings: RwLock::new(HashMap::new()),
            adjust_settings: RwLock::new(HashMap::new()),
            brightness_settings: RwLock::new(HashMap::new()),
            last_errors: RwLock::new(HashMap::new()),
            invalid_switches: RwLock::new(HashSet::new()),
            invalid_brightness: RwLock::new(HashSet::new()),
        }))
    }

    pub async fn global_settings(&self) -> GlobalSettings {
        self.global_settings.read().await.clone()
    }

    pub async fn update_global_settings(&self, settings: GlobalSettings) {
        let previous = self.global_settings().await;
        if previous.homebridge_url != settings.homebridge_url
            || previous.username != settings.username
            || previous.password != settings.password
        {
            self.client.clear_all_caches().await;
        } else if previous.catalog_cache_seconds != settings.catalog_cache_seconds {
            self.client.clear_catalogs().await;
        }
        *self.global_settings.write().await = settings;
    }

    pub async fn remember_switch(&self, instance_id: String, settings: SwitchSettings) {
        self.invalid_switches.write().await.remove(&instance_id);
        self.switch_settings.write().await.insert(instance_id, settings);
    }

    pub async fn forget_switch(&self, instance_id: &str) {
        self.switch_settings.write().await.remove(instance_id);
        self.invalid_switches.write().await.remove(instance_id);
    }

    pub async fn switch_settings(&self, instance_id: &str) -> Option<SwitchSettings> {
        self.switch_settings.read().await.get(instance_id).cloned()
    }

    pub async fn mark_switch_invalid(&self, instance_id: &str) {
        self.invalid_switches
            .write()
            .await
            .insert(instance_id.to_string());
    }

    pub async fn clear_switch_invalid(&self, instance_id: &str) {
        self.invalid_switches.write().await.remove(instance_id);
    }

    pub async fn switch_polling_enabled(&self, instance_id: &str) -> bool {
        !self.invalid_switches.read().await.contains(instance_id)
    }

    pub async fn remember_brightness(
        &self,
        instance_id: String,
        settings: BrightnessSettings,
    ) {
        self.invalid_brightness.write().await.remove(&instance_id);
        self.brightness_settings
            .write()
            .await
            .insert(instance_id, settings);
    }

    pub async fn forget_brightness(&self, instance_id: &str) {
        self.brightness_settings.write().await.remove(instance_id);
        self.invalid_brightness.write().await.remove(instance_id);
    }

    pub async fn brightness_settings(&self, instance_id: &str) -> Option<BrightnessSettings> {
        self.brightness_settings
            .read()
            .await
            .get(instance_id)
            .cloned()
    }

    pub async fn mark_brightness_invalid(&self, instance_id: &str) {
        self.invalid_brightness
            .write()
            .await
            .insert(instance_id.to_string());
    }

    pub async fn clear_brightness_invalid(&self, instance_id: &str) {
        self.invalid_brightness.write().await.remove(instance_id);
    }

    pub async fn brightness_polling_enabled(&self, instance_id: &str) -> bool {
        !self.invalid_brightness.read().await.contains(instance_id)
    }

    pub async fn should_log_error(&self, key: &str, message: &str) -> bool {
        let mut errors = self.last_errors.write().await;
        if errors.get(key).is_some_and(|previous| previous == message) {
            return false;
        }
        errors.insert(key.to_string(), message.to_string());
        true
    }

    pub async fn clear_error(&self, key: &str) {
        self.last_errors.write().await.remove(key);
    }

    pub async fn remember_adjust(&self, instance_id: String, settings: AdjustStateSettings) {
        self.adjust_settings
            .write()
            .await
            .insert(instance_id, settings);
    }

    pub async fn forget_adjust(&self, instance_id: &str) {
        self.adjust_settings.write().await.remove(instance_id);
    }

    pub async fn adjust_settings(&self, instance_id: &str) -> Option<AdjustStateSettings> {
        self.adjust_settings.read().await.get(instance_id).cloned()
    }
}
