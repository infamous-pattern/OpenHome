use std::fmt::{Display, Formatter};
use std::sync::Arc;

use anyhow::{Result, bail};
use openaction::{Action, ActionUuid, Instance, OpenActionResult};
use serde_json::Value;

use crate::actions::common::{
    apply_global_settings, display_title, looks_like_connection_error, parse_request, send_catalog,
};
use crate::models::{AccessoryService, Characteristic, SelectedCharacteristic, SwitchSettings};
use crate::state::PluginState;

pub const SWITCH_UUID: ActionUuid = "com.infamous-pattern.openhomeb.switch";

pub struct SwitchAction {
    state: Arc<PluginState>,
}

impl SwitchAction {
    pub fn new(state: Arc<PluginState>) -> Self {
        Self { state }
    }

    #[allow(clippy::collapsible_if)]
    async fn toggle(
        &self,
        instance: &Instance,
        settings: &SwitchSettings,
    ) -> OpenActionResult<bool> {
        if !settings.is_configured() {
            set_not_configured(&self.state, instance).await?;
            instance.show_alert().await?;
            return Ok(false);
        }

        let global = self.state.global_settings().await;
        log::info!(
            "Switch {}: toggle requested for service_id='{}', characteristic_uuid='{}', characteristic_type='{}'",
            instance.instance_id,
            settings.accessory_id(),
            settings.characteristic_uuid(),
            settings.characteristic_type(),
        );

        let result: Result<SwitchDisplayState> = async {
            // Always query immediately before writing so the inverse is calculated from
            // Homebridge's current value rather than an old OpenDeck button state.
            let service = self
                .state
                .client
                .get_accessory(&global, None, &settings.accessory_id)
                .await?;
            let characteristic = selected_switch_characteristic(&service, settings)?;
            let characteristic_type = characteristic.characteristic_type.clone();
            let current_on = boolean_state(&characteristic.value)?;
            let requested_on = !current_on;
            log::info!(
                "Switch {}: current state={}, requesting state={}",
                instance.instance_id,
                if current_on { "On" } else { "Off" },
                if requested_on { "On" } else { "Off" },
            );

            let write_response = self.state
                .client
                .set_characteristic(
                    &global,
                    None,
                    &settings.accessory_id,
                    &characteristic_type,
                    Value::Bool(requested_on),
                )
                .await?;

            if let Ok(written_characteristic) = selected_switch_characteristic(&write_response, settings) {
                if let Ok(written_on) = boolean_state(&written_characteristic.value) {
                    log::info!(
                        "Switch {}: Homebridge PUT response state={}",
                        instance.instance_id,
                        if written_on { "On" } else { "Off" },
                    );
                }
            }

            // Some physical accessories report the old value briefly after a successful
            // write. Re-read up to three times before declaring the command unsuccessful.
            let mut confirmed = query_display_state(&self.state, settings).await?;
            for delay_ms in [250_u64, 750_u64] {
                if confirmed.is_on == requested_on {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                confirmed = query_display_state(&self.state, settings).await?;
            }

            if confirmed.is_on != requested_on {
                bail!(
                    "Homebridge accepted the write but the confirmed state remained {} instead of {}",
                    if confirmed.is_on { "On" } else { "Off" },
                    if requested_on { "On" } else { "Off" },
                );
            }

            log::info!(
                "Switch {}: confirmed state={}",
                instance.instance_id,
                if confirmed.is_on { "On" } else { "Off" },
            );
            Ok(confirmed)
        }
        .await;

        match result {
            Ok(display) => {
                clear_switch_error(&self.state, instance).await;
                apply_display_state(instance, &display).await?;
                if settings.show_confirmation {
                    instance.show_ok().await?;
                }
                Ok(true)
            }
            Err(error) => {
                report_switch_error(&self.state, instance, &error, true).await?;
                Ok(false)
            }
        }
    }
}

#[derive(Debug)]
enum SwitchSelectionError {
    CharacteristicUnavailable(String),
    CannotRead,
    ReadOnly,
    NotBoolean,
}

impl Display for SwitchSelectionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CharacteristicUnavailable(detail) => {
                write!(
                    formatter,
                    "selected characteristic is no longer available; {detail}"
                )
            }
            Self::CannotRead => write!(formatter, "selected characteristic cannot be read"),
            Self::ReadOnly => write!(formatter, "selected characteristic is read-only"),
            Self::NotBoolean => write!(formatter, "selected characteristic is not Boolean"),
        }
    }
}

impl std::error::Error for SwitchSelectionError {}

#[derive(Debug)]
struct SwitchDisplayState {
    is_on: bool,
    title: String,
}

async fn query_display_state(
    state: &Arc<PluginState>,
    settings: &SwitchSettings,
) -> Result<SwitchDisplayState> {
    let global = state.global_settings().await;
    let service = state
        .client
        .get_accessory(&global, None, &settings.accessory_id)
        .await?;
    state.mark_connection_online();
    let characteristic = selected_switch_characteristic(&service, settings)?;
    let is_on = boolean_state(&characteristic.value)?;
    let name = display_title(&settings.display_name, &service.service_name);

    Ok(SwitchDisplayState {
        is_on,
        title: state_title(&name, is_on, &settings.label_mode),
    })
}

fn selected_switch_characteristic<'a>(
    service: &'a AccessoryService,
    settings: &SwitchSettings,
) -> Result<&'a Characteristic> {
    let characteristic = service
        .characteristic_by_identity(
            settings.characteristic_uuid(),
            settings.characteristic_type(),
        )
        .ok_or_else(|| {
            SwitchSelectionError::CharacteristicUnavailable(format!(
                "service_id='{}', characteristic_uuid='{}', characteristic_type='{}', available=[{}]",
                settings.accessory_id(),
                settings.characteristic_uuid(),
                settings.characteristic_type(),
                service.available_characteristic_types(),
            ))
        })?;

    if !characteristic.can_read {
        return Err(SwitchSelectionError::CannotRead.into());
    }
    if !characteristic.can_write {
        return Err(SwitchSelectionError::ReadOnly.into());
    }
    if !characteristic.is_boolean() {
        return Err(SwitchSelectionError::NotBoolean.into());
    }

    Ok(characteristic)
}

async fn apply_display_state(
    instance: &Instance,
    display: &SwitchDisplayState,
) -> OpenActionResult<()> {
    instance
        .set_state(if display.is_on { 1 } else { 0 })
        .await?;
    instance.set_title(Some(display.title.clone()), None).await
}

pub async fn refresh_switch_instance(
    state: &Arc<PluginState>,
    instance: &Instance,
    settings: &SwitchSettings,
    show_alert: bool,
) -> OpenActionResult<()> {
    if !settings.is_configured() {
        return set_not_configured(state, instance).await;
    }

    match query_display_state(state, settings).await {
        Ok(display) => {
            clear_switch_error(state, instance).await;
            apply_display_state(instance, &display).await
        }
        Err(error) => report_switch_error(state, instance, &error, show_alert).await,
    }
}

async fn set_not_configured(state: &Arc<PluginState>, instance: &Instance) -> OpenActionResult<()> {
    state.mark_switch_invalid(&instance.instance_id).await;
    state.clear_error(&switch_error_key(instance)).await;
    instance.set_title(Some("Not\nConfigured"), None).await
}

async fn report_switch_error(
    state: &Arc<PluginState>,
    instance: &Instance,
    error: &anyhow::Error,
    show_alert: bool,
) -> OpenActionResult<()> {
    let key = switch_error_key(instance);
    let message = error.to_string();
    let first_occurrence = state.should_log_error(&key, &message).await;
    if first_occurrence || show_alert {
        log::error!("Switch {}: {message}", instance.instance_id);
    }

    let title = if error.downcast_ref::<SwitchSelectionError>().is_some() {
        state.mark_switch_invalid(&instance.instance_id).await;
        Some("Not\nConfigured")
    } else if looks_like_connection_error(&message) {
        state.mark_connection_offline();
        state.request_reconnect();
        if show_alert {
            Some("Offline")
        } else if state.has_connected_once() {
            None
        } else {
            Some("Connecting…")
        }
    } else {
        Some("Homebridge\nerror")
    };
    if let Some(title) = title {
        instance.set_title(Some(title), None).await?;
    }
    if show_alert {
        instance.show_alert().await?;
    }
    Ok(())
}

async fn clear_switch_error(state: &Arc<PluginState>, instance: &Instance) {
    state.clear_switch_invalid(&instance.instance_id).await;
    state.clear_error(&switch_error_key(instance)).await;
}

fn switch_error_key(instance: &Instance) -> String {
    format!("switch:{}", instance.instance_id)
}

fn state_title(name: &str, is_on: bool, label_mode: &str) -> String {
    let state = if is_on { "On" } else { "Off" };
    match label_mode {
        "stateOnly" => state.to_string(),
        "nameOnly" => name.to_string(),
        "hidden" => String::new(),
        _ => format!("{name}\n{state}"),
    }
}

fn boolean_state(value: &Value) -> Result<bool> {
    if let Some(value) = value.as_bool() {
        return Ok(value);
    }

    // Some Homebridge accessory plugins serialise HAP Boolean values as 0/1 or
    // strings even while advertising `format: "bool"`.
    if let Some(value) = value.as_f64() {
        if value == 0.0 {
            return Ok(false);
        }
        if value == 1.0 {
            return Ok(true);
        }
    }
    if let Some(value) = value.as_str() {
        return match value.trim().to_ascii_lowercase().as_str() {
            "true" | "on" | "yes" | "1" => Ok(true),
            "false" | "off" | "no" | "0" => Ok(false),
            _ => bail!("characteristic value is not Boolean"),
        };
    }
    bail!("characteristic value is not Boolean")
}

#[openaction::async_trait]
impl Action for SwitchAction {
    type Settings = SwitchSettings;
    const UUID: ActionUuid = SWITCH_UUID;

    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.state
            .remember_switch(instance.instance_id.clone(), settings.clone())
            .await;
        if !settings.is_configured() {
            return set_not_configured(&self.state, instance).await;
        }
        if !self.state.global_settings_loaded() || !self.state.connection_online() {
            if !self.state.has_connected_once() {
                instance.set_title(Some("Connecting…"), None).await?;
            }
            return Ok(());
        }
        refresh_switch_instance(&self.state, instance, settings, false).await
    }

    async fn will_disappear(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.state.forget_switch(&instance.instance_id).await;
        self.state
            .clear_error(&format!("switch:{}", instance.instance_id))
            .await;
        Ok(())
    }

    async fn key_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        log::info!("Switch {}: keyDown event received", instance.instance_id);
        self.toggle(instance, settings).await.map(|_| ())
    }

    async fn did_receive_settings(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.state
            .remember_switch(instance.instance_id.clone(), settings.clone())
            .await;
        if !settings.is_configured() {
            return set_not_configured(&self.state, instance).await;
        }
        if !self.state.global_settings_loaded() || !self.state.connection_online() {
            if !self.state.has_connected_once() {
                instance.set_title(Some("Connecting…"), None).await?;
            }
            self.state.request_reconnect();
            return Ok(());
        }
        refresh_switch_instance(&self.state, instance, settings, false).await
    }

    async fn property_inspector_did_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        let global = self.state.global_settings().await;
        send_catalog(instance, &self.state, &global, None, "switch", false).await?;
        refresh_switch_instance(&self.state, instance, settings, false).await
    }

    async fn send_to_plugin(
        &self,
        instance: &Instance,
        current: &Self::Settings,
        payload: &Value,
    ) -> OpenActionResult<()> {
        let request = parse_request(payload)?;
        if request.event != "refreshCatalog"
            && request.event != "testConnection"
            && request.event != "toggleSwitch"
        {
            return Ok(());
        }

        let global = apply_global_settings(&self.state, request.global_settings).await?;
        let settings = if request.action_settings.is_null() {
            current.clone()
        } else {
            serde_json::from_value::<SwitchSettings>(request.action_settings)?
        };
        instance.set_settings(&settings).await?;
        self.state
            .remember_switch(instance.instance_id.clone(), settings.clone())
            .await;

        if request.event == "toggleSwitch" {
            log::info!(
                "Switch {}: property-inspector test toggle requested",
                instance.instance_id
            );
            let changed = self.toggle(instance, &settings).await?;
            let message = if changed {
                "Switch state changed and confirmed by Homebridge"
            } else {
                "Switch command failed; check the plugin log for details"
            };
            crate::actions::common::send_status(
                instance,
                if changed { "connected" } else { "error" },
                message,
            )
            .await?;
            return Ok(());
        }

        send_catalog(
            instance,
            &self.state,
            &global,
            request.otp.as_deref(),
            "switch",
            request.force_refresh,
        )
        .await?;
        refresh_switch_instance(&self.state, instance, &settings, false).await
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{boolean_state, state_title};

    #[test]
    fn reads_common_boolean_representations() {
        assert!(boolean_state(&json!(true)).unwrap());
        assert!(!boolean_state(&json!(0)).unwrap());
        assert!(boolean_state(&json!("on")).unwrap());
        assert!(boolean_state(&json!(2)).is_err());
    }

    #[test]
    fn title_contains_current_state() {
        assert_eq!(
            state_title("Desk Lamp", true, "nameAndState"),
            "Desk Lamp\nOn"
        );
        assert_eq!(state_title("Desk Lamp", false, "stateOnly"), "Off");
        assert_eq!(state_title("Desk Lamp", true, "hidden"), "");
    }
}
