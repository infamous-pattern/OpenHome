use std::sync::Arc;

use anyhow::{Result, bail};
use openaction::{Action, ActionUuid, Instance, OpenActionResult, send_arbitrary_json};
use serde_json::{Number, Value};

use crate::actions::common::{
    action_error, apply_global_settings, display_title, looks_like_connection_error, parse_request,
    send_catalog,
};
use crate::models::{AdjustStateSettings, Characteristic, SelectedCharacteristic};
use crate::state::PluginState;

pub const ADJUST_UUID: ActionUuid = "com.infamous-pattern.openhomeb.adjust";

pub struct AdjustStateAction {
    state: Arc<PluginState>,
}

impl AdjustStateAction {
    pub fn new(state: Arc<PluginState>) -> Self {
        Self { state }
    }

    async fn adjust(
        &self,
        instance: &Instance,
        settings: &AdjustStateSettings,
        ticks: i16,
    ) -> OpenActionResult<()> {
        if !settings.is_configured() {
            return action_error(
                instance,
                "Select\nstate",
                "adjust action is not configured",
                true,
            )
            .await;
        }

        let global = self.state.global_settings().await;
        let result: Result<(Value, String, Option<f64>, Option<f64>)> = async {
            let service = self
                .state
                .client
                .get_accessory(&global, None, &settings.accessory_id)
                .await?;
            self.state.mark_connection_online();
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
            if !characteristic.is_numeric() {
                bail!("selected characteristic is not numeric")
            }

            let current = characteristic
                .value
                .as_f64()
                .ok_or_else(|| anyhow::anyhow!("current characteristic value is not numeric"))?;
            let step = characteristic.min_step.unwrap_or(1.0).abs().max(f64::EPSILON);
            let speed = if settings.speed.is_finite() && settings.speed > 0.0 {
                settings.speed
            } else {
                1.0
            };
            let mut next = current + f64::from(ticks) * step * speed;
            if let Some(minimum) = characteristic.min_value {
                next = next.max(minimum);
            }
            if let Some(maximum) = characteristic.max_value {
                next = next.min(maximum);
            }

            let value = numeric_value(next, characteristic)?;
            let characteristic_type = characteristic.characteristic_type.clone();
            self.state
                .client
                .set_characteristic(
                    &global,
                    None,
                    &settings.accessory_id,
                    &characteristic_type,
                    value.clone(),
                )
                .await?;

            Ok((
                value,
                display_title(&settings.display_name, &service.service_name),
                characteristic.min_value,
                characteristic.max_value,
            ))
        }
        .await;

        match result {
            Ok((value, title, minimum, maximum)) => {
                set_encoder_feedback(instance, &title, &value, minimum, maximum).await
            }
            Err(error) => action_error(instance, "Homebridge\nerror", error, true).await,
        }
    }
}

pub async fn refresh_adjust_instance(
    state: &Arc<PluginState>,
    instance: &Instance,
    settings: &AdjustStateSettings,
    show_alert: bool,
) -> OpenActionResult<()> {
    if !settings.is_configured() {
        instance.set_title(Some("Select\nstate"), None).await?;
        return Ok(());
    }

    let global = state.global_settings().await;
    match state
        .client
        .get_accessory(&global, None, &settings.accessory_id)
        .await
    {
        Ok(service) => {
            state.mark_connection_online();
            let Some(characteristic) = service.characteristic_by_identity(
                settings.characteristic_uuid(),
                settings.characteristic_type(),
            ) else {
                return action_error(
                    instance,
                    "Missing\nstate",
                    format!(
                        "selected characteristic is no longer available; service_id='{}', characteristic_uuid='{}', characteristic_type='{}', available=[{}]",
                        settings.accessory_id(),
                        settings.characteristic_uuid(),
                        settings.characteristic_type(),
                        service.available_characteristic_types(),
                    ),
                    show_alert,
                )
                .await;
            };
            let title = display_title(&settings.display_name, &service.service_name);
            set_encoder_feedback(
                instance,
                &title,
                &characteristic.value,
                characteristic.min_value,
                characteristic.max_value,
            )
            .await
        }
        Err(error) => {
            let message = error.to_string();
            if looks_like_connection_error(&message) {
                state.mark_connection_offline();
                state.request_reconnect();
                if show_alert {
                    action_error(instance, "Offline", error, true).await
                } else {
                    if !state.has_connected_once() {
                        instance.set_title(Some("Connecting…"), None).await?;
                    }
                    if state
                        .should_log_error(&format!("adjust:{}", instance.instance_id), &message)
                        .await
                    {
                        log::warn!("Adjust {}: {message}", instance.instance_id);
                    }
                    Ok(())
                }
            } else {
                action_error(instance, "Homebridge\nerror", error, show_alert).await
            }
        }
    }
}

async fn set_encoder_feedback(
    instance: &Instance,
    title: &str,
    value: &Value,
    minimum: Option<f64>,
    maximum: Option<f64>,
) -> OpenActionResult<()> {
    let indicator_value = indicator_percentage(value, minimum, maximum);
    if let Err(error) = send_arbitrary_json(serde_json::json!({
        "event": "setFeedback",
        "context": instance.instance_id.clone(),
        "payload": {
            "title": title,
            "value": format_value(value),
            "indicator": {
                "value": indicator_value
            }
        }
    }))
    .await
    {
        log::warn!("Could not send encoder feedback: {error}");
    }

    // Keep a title fallback for OpenDeck surfaces that do not render encoder feedback.
    instance
        .set_title(Some(format!("{title}\n{}", format_value(value))), None)
        .await
}

fn indicator_percentage(value: &Value, minimum: Option<f64>, maximum: Option<f64>) -> f64 {
    let current = value.as_f64().unwrap_or(0.0);
    let minimum = minimum.unwrap_or(0.0);
    let maximum = maximum.unwrap_or(100.0);
    if (maximum - minimum).abs() < f64::EPSILON {
        return 0.0;
    }
    ((current - minimum) * 100.0 / (maximum - minimum)).clamp(0.0, 100.0)
}

fn numeric_value(value: f64, characteristic: &Characteristic) -> Result<Value> {
    if !value.is_finite() {
        bail!("calculated value is not finite")
    }

    if characteristic.is_integer() {
        return Ok(Value::Number(Number::from(value.round() as i64)));
    }

    Number::from_f64(value)
        .map(Value::Number)
        .ok_or_else(|| anyhow::anyhow!("calculated value cannot be represented as JSON"))
}

fn format_value(value: &Value) -> String {
    if let Some(number) = value.as_f64() {
        if number.fract().abs() < 0.000_001 {
            return format!("{number:.0}");
        }
        return format!("{number:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string();
    }
    if let Some(text) = value.as_str() {
        return text.to_string();
    }
    value.to_string()
}

#[openaction::async_trait]
impl Action for AdjustStateAction {
    type Settings = AdjustStateSettings;
    const UUID: ActionUuid = ADJUST_UUID;

    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.state
            .remember_adjust(instance.instance_id.clone(), settings.clone())
            .await;
        if !settings.is_configured() {
            return refresh_adjust_instance(&self.state, instance, settings, false).await;
        }
        if !self.state.global_settings_loaded() || !self.state.connection_online() {
            if !self.state.has_connected_once() {
                instance.set_title(Some("Connecting…"), None).await?;
            }
            return Ok(());
        }
        refresh_adjust_instance(&self.state, instance, settings, false).await
    }

    async fn will_disappear(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.state.forget_adjust(&instance.instance_id).await;
        Ok(())
    }

    async fn dial_rotate(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
        ticks: i16,
        _pressed: bool,
    ) -> OpenActionResult<()> {
        self.adjust(instance, settings, ticks).await
    }

    async fn dial_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        refresh_adjust_instance(&self.state, instance, settings, true).await
    }

    async fn did_receive_settings(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.state
            .remember_adjust(instance.instance_id.clone(), settings.clone())
            .await;
        if !settings.is_configured() {
            return refresh_adjust_instance(&self.state, instance, settings, false).await;
        }
        if !self.state.global_settings_loaded() || !self.state.connection_online() {
            if !self.state.has_connected_once() {
                instance.set_title(Some("Connecting…"), None).await?;
            }
            self.state.request_reconnect();
            return Ok(());
        }
        refresh_adjust_instance(&self.state, instance, settings, false).await
    }

    async fn property_inspector_did_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        let global = self.state.global_settings().await;
        send_catalog(instance, &self.state, &global, None, "adjust", false).await?;
        refresh_adjust_instance(&self.state, instance, settings, false).await
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
            serde_json::from_value::<AdjustStateSettings>(request.action_settings)?
        };
        instance.set_settings(&settings).await?;
        self.state
            .remember_adjust(instance.instance_id.clone(), settings.clone())
            .await;
        send_catalog(
            instance,
            &self.state,
            &global,
            request.otp.as_deref(),
            "adjust",
            request.force_refresh,
        )
        .await?;
        refresh_adjust_instance(&self.state, instance, &settings, false).await
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{format_value, indicator_percentage};

    #[test]
    fn formats_numbers_for_encoder_titles() {
        assert_eq!(format_value(&json!(50)), "50");
        assert_eq!(format_value(&json!(20.5)), "20.5");
    }

    #[test]
    fn calculates_encoder_indicator_percentage() {
        assert_eq!(
            indicator_percentage(&json!(50), Some(0.0), Some(100.0)),
            50.0
        );
        assert_eq!(
            indicator_percentage(&json!(125), Some(0.0), Some(100.0)),
            100.0
        );
        assert_eq!(
            indicator_percentage(&json!(-10), Some(0.0), Some(100.0)),
            0.0
        );
    }
}
