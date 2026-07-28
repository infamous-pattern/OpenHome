use std::sync::Arc;

use openaction::{Action, ActionUuid, Instance, OpenActionResult};
use serde_json::Value;

use crate::actions::common::{apply_global_settings, parse_request, send_catalog};
use crate::models::EmptySettings;
use crate::state::PluginState;

const ACTION_UUID: ActionUuid = "com.infamous-pattern.openhomeb.devices";

pub struct HomebridgeDevicesAction {
    state: Arc<PluginState>,
}

impl HomebridgeDevicesAction {
    pub fn new(state: Arc<PluginState>) -> Self {
        Self { state }
    }

    async fn refresh(
        &self,
        instance: &Instance,
        otp: Option<&str>,
        button_feedback: bool,
    ) -> OpenActionResult<()> {
        let global = self.state.global_settings().await;
        match self.state.client.catalog(&global, otp, true).await {
            Ok(catalog) => {
                let title = if catalog.device_count == 1 {
                    "1\ndevice".to_string()
                } else {
                    format!("{}\ndevices", catalog.device_count)
                };
                instance.set_title(Some(title), None).await?;
                if button_feedback {
                    instance.show_ok().await?;
                }
                instance
                    .send_to_property_inspector(serde_json::json!({
                        "event": "catalog",
                        "actionKind": "devices",
                        "catalog": catalog,
                    }))
                    .await?;
            }
            Err(error) => {
                instance.set_title(Some("Homebridge\nerror"), None).await?;
                if button_feedback {
                    instance.show_alert().await?;
                }
                crate::actions::common::send_status(instance, "error", &error.to_string()).await?;
            }
        }
        Ok(())
    }
}

#[openaction::async_trait]
impl Action for HomebridgeDevicesAction {
    type Settings = EmptySettings;
    const UUID: ActionUuid = ACTION_UUID;

    async fn will_appear(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        instance.set_title(Some("Homebridge"), None).await
    }

    async fn key_up(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.refresh(instance, None, true).await
    }

    async fn property_inspector_did_appear(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        let global = self.state.global_settings().await;
        send_catalog(instance, &self.state, &global, None, "devices", false).await
    }

    async fn send_to_plugin(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
        payload: &Value,
    ) -> OpenActionResult<()> {
        let request = parse_request(payload)?;
        if request.event != "refreshCatalog" && request.event != "testConnection" {
            return Ok(());
        }
        let global = apply_global_settings(&self.state, request.global_settings).await?;
        send_catalog(
            instance,
            &self.state,
            &global,
            request.otp.as_deref(),
            "devices",
            request.force_refresh,
        )
        .await
    }
}
