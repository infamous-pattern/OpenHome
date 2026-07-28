use std::sync::Arc;

use openaction::{Action, ActionUuid, Instance, OpenActionResult, open_url};
use serde_json::Value;

use crate::actions::common::{action_error, apply_global_settings, parse_request, send_catalog};
use crate::homebridge::normalise_base_url;
use crate::models::EmptySettings;
use crate::state::PluginState;

const ACTION_UUID: ActionUuid = "com.infamous-pattern.openhomeb.config-ui";

pub struct LaunchHomebridgeUiAction {
    state: Arc<PluginState>,
}

impl LaunchHomebridgeUiAction {
    pub fn new(state: Arc<PluginState>) -> Self {
        Self { state }
    }
}

#[openaction::async_trait]
impl Action for LaunchHomebridgeUiAction {
    type Settings = EmptySettings;
    const UUID: ActionUuid = ACTION_UUID;

    async fn will_appear(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        instance.set_title(Some("Homebridge\nUI"), None).await
    }

    async fn key_up(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        let global = self.state.global_settings().await;
        match normalise_base_url(&global.homebridge_url) {
            Ok(url) => open_url(url).await,
            Err(error) => action_error(instance, "Invalid\nURL", error, true).await,
        }
    }

    async fn property_inspector_did_appear(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        let global = self.state.global_settings().await;
        send_catalog(instance, &self.state, &global, None, "openUi", false).await
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
            "openUi",
            request.force_refresh,
        )
        .await
    }
}
