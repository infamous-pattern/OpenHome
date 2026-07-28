use std::sync::Arc;

use anyhow::{Result, bail};
use openaction::{Action, ActionUuid, Instance, OpenActionResult};
use serde_json::Value;

use crate::actions::common::{
    action_error, apply_global_settings, display_title, parse_request, send_catalog,
};
use crate::models::{SelectedCharacteristic, SetStateSettings};
use crate::state::PluginState;

const ACTION_UUID: ActionUuid = "com.infamous-pattern.openhomeb.set";

pub struct SetStateAction {
    state: Arc<PluginState>,
}

impl SetStateAction {
    pub fn new(state: Arc<PluginState>) -> Self {
        Self { state }
    }

    async fn apply(
        &self,
        instance: &Instance,
        settings: &SetStateSettings,
    ) -> OpenActionResult<()> {
        if !settings.is_configured() || settings.target_value.is_null() {
            return action_error(
                instance,
                "Select\nstate",
                "set-state action is not configured",
                true,
            )
            .await;
        }

        let global = self.state.global_settings().await;
        let result: Result<String> = async {
            let service = self
                .state
                .client
                .get_accessory(&global, None, &settings.accessory_id)
                .await?;
            let characteristic = service
                .characteristic_by_identity(
                    settings.characteristic_uuid(),
                    settings.characteristic_type(),
                )
                .ok_or_else(|| anyhow::anyhow!(
                    "selected characteristic is no longer available; service_id='{}', characteristic_uuid='{}', characteristic_type='{}', available=[{}]",
                    settings.accessory_id(),
                    settings.characteristic_uuid(),
                    settings.characteristic_type(),
                    service.available_characteristic_types(),
                ))?;
            if !characteristic.can_write {
                bail!("selected characteristic is read-only")
            }
            let characteristic_type = characteristic.characteristic_type.clone();

            self.state
                .client
                .set_characteristic(
                    &global,
                    None,
                    &settings.accessory_id,
                    &characteristic_type,
                    settings.target_value.clone(),
                )
                .await?;

            Ok(display_title(&settings.display_name, &service.service_name))
        }
        .await;

        match result {
            Ok(title) => {
                instance.set_title(Some(title), None).await?;
                instance.show_ok().await
            }
            Err(error) => action_error(instance, "Homebridge\nerror", error, true).await,
        }
    }

    async fn update_title(
        &self,
        instance: &Instance,
        settings: &SetStateSettings,
    ) -> OpenActionResult<()> {
        if settings.is_configured() {
            instance
                .set_title(
                    Some(display_title(&settings.display_name, "Set state")),
                    None,
                )
                .await
        } else {
            instance.set_title(Some("Select\nstate"), None).await
        }
    }
}

#[openaction::async_trait]
impl Action for SetStateAction {
    type Settings = SetStateSettings;
    const UUID: ActionUuid = ACTION_UUID;

    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.update_title(instance, settings).await
    }

    async fn key_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        self.apply(instance, settings).await
    }

    async fn did_receive_settings(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.update_title(instance, settings).await
    }

    async fn property_inspector_did_appear(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        let global = self.state.global_settings().await;
        send_catalog(instance, &self.state, &global, None, "set", false).await
    }

    async fn send_to_plugin(
        &self,
        instance: &Instance,
        current: &Self::Settings,
        payload: &Value,
    ) -> OpenActionResult<()> {
        let request = parse_request(payload)?;
        if request.event != "refreshCatalog" && request.event != "testConnection" {
            return Ok(());
        }

        let global = apply_global_settings(&self.state, request.global_settings).await?;
        let settings = if request.action_settings.is_null() {
            current.clone()
        } else {
            serde_json::from_value::<SetStateSettings>(request.action_settings)?
        };
        instance.set_settings(&settings).await?;
        send_catalog(
            instance,
            &self.state,
            &global,
            request.otp.as_deref(),
            "set",
            request.force_refresh,
        )
        .await?;
        self.update_title(instance, &settings).await
    }
}
