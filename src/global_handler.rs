use std::sync::Arc;

use openaction::OpenActionResult;
use openaction::get_global_settings;
use openaction::global_events::{
    DidReceiveGlobalSettingsEvent, GlobalEventHandler, SystemDidWakeUpEvent,
};

use crate::models::GlobalSettings;
use crate::state::PluginState;

pub struct HomebridgeGlobalEventHandler {
    state: Arc<PluginState>,
}

impl HomebridgeGlobalEventHandler {
    pub fn new(state: Arc<PluginState>) -> Self {
        Self { state }
    }
}

#[openaction::async_trait]
impl GlobalEventHandler for HomebridgeGlobalEventHandler {
    async fn plugin_ready(&self) -> OpenActionResult<()> {
        get_global_settings().await
    }

    async fn did_receive_global_settings(
        &self,
        event: DidReceiveGlobalSettingsEvent,
    ) -> OpenActionResult<()> {
        match serde_json::from_value::<GlobalSettings>(event.payload.settings) {
            Ok(settings) => self.state.update_global_settings(settings).await,
            Err(error) => log::warn!("Could not parse Homebridge global settings: {error}"),
        }
        Ok(())
    }

    async fn system_did_wake_up(&self, _event: SystemDidWakeUpEvent) -> OpenActionResult<()> {
        self.state.client.clear_all_caches().await;
        Ok(())
    }
}
