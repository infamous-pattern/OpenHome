use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, bail};
use openaction::{Action, ActionUuid, Instance, OpenActionResult, send_arbitrary_json};
use serde_json::{Number, Value};

use crate::actions::common::{
    apply_global_settings, display_title, parse_request, send_catalog, send_status,
};
use crate::models::{
    AccessoryService, BrightnessSettings, Characteristic, SelectedCharacteristic,
};
use crate::state::PluginState;

pub const BRIGHTNESS_UUID: ActionUuid = "com.infamous-pattern.openhomeb.brightness";

pub struct BrightnessAction {
    state: Arc<PluginState>,
}

impl BrightnessAction {
    pub fn new(state: Arc<PluginState>) -> Self {
        Self { state }
    }

    async fn apply_key_mode(
        &self,
        instance: &Instance,
        settings: &BrightnessSettings,
    ) -> OpenActionResult<bool> {
        let result = self.calculate_and_write(settings, None).await;
        self.handle_result(instance, settings, result, true).await
    }

    async fn adjust_by_ticks(
        &self,
        instance: &Instance,
        settings: &BrightnessSettings,
        ticks: i16,
    ) -> OpenActionResult<bool> {
        if ticks == 0 {
            return Ok(false);
        }
        let result = self.calculate_and_write(settings, Some(ticks)).await;
        self.handle_result(instance, settings, result, true).await
    }

    #[allow(clippy::collapsible_if)]
    async fn calculate_and_write(
        &self,
        settings: &BrightnessSettings,
        ticks: Option<i16>,
    ) -> Result<BrightnessDisplayState> {
        if !settings.is_configured() {
            bail!("brightness action is not configured")
        }

        let global = self.state.global_settings().await;
        let service = self
            .state
            .client
            .get_accessory(&global, None, settings.accessory_id())
            .await?;
        let characteristic = selected_brightness_characteristic(&service, settings)?;
        let current = characteristic
            .value
            .as_f64()
            .ok_or_else(|| anyhow::anyhow!("current brightness value is not numeric"))?;

        let minimum = characteristic.min_value.unwrap_or(0.0);
        let maximum = characteristic.max_value.unwrap_or(100.0);
        let homebridge_step = characteristic
            .min_step
            .unwrap_or(1.0)
            .abs()
            .max(f64::EPSILON);
        let increment = sanitise_increment(settings.increment, homebridge_step);

        let requested = if let Some(ticks) = ticks {
            current + f64::from(ticks) * increment
        } else {
            key_target(settings, current, minimum, maximum, increment)
        };
        let requested = align_and_clamp(requested, minimum, maximum, homebridge_step);
        let value = numeric_value(requested, characteristic)?;
        let characteristic_type = characteristic.characteristic_type.clone();

        log::info!(
            "Brightness: service_id='{}', current={}, requested={}, mode='{}'",
            settings.accessory_id(),
            current,
            requested,
            settings.mode,
        );

        self.state
            .client
            .set_characteristic(
                &global,
                None,
                settings.accessory_id(),
                &characteristic_type,
                value,
            )
            .await?;

        if settings.turn_on_when_adjusting && requested > minimum {
            if let Some(power) = service
                .service_characteristics
                .iter()
                .find(|item| item.characteristic_type.eq_ignore_ascii_case("On") && item.is_switch_compatible())
            {
                if !read_boolean(&power.value).unwrap_or(false) {
                    self.state
                        .client
                        .set_characteristic(
                            &global,
                            None,
                            settings.accessory_id(),
                            &power.characteristic_type,
                            Value::Bool(true),
                        )
                        .await?;
                }
            }
        }

        let mut confirmed = query_brightness_state(&self.state, settings).await?;
        for delay_ms in [200_u64, 500_u64, 1_000_u64] {
            if (confirmed.value - requested).abs() <= homebridge_step.max(0.5) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            confirmed = query_brightness_state(&self.state, settings).await?;
        }

        if (confirmed.value - requested).abs() > homebridge_step.max(0.5) {
            bail!(
                "Homebridge accepted the brightness write, but the confirmed value remained {} instead of {}",
                confirmed.value,
                requested
            )
        }

        Ok(confirmed)
    }

    async fn handle_result(
        &self,
        instance: &Instance,
        settings: &BrightnessSettings,
        result: Result<BrightnessDisplayState>,
        show_alert: bool,
    ) -> OpenActionResult<bool> {
        match result {
            Ok(display) => {
                self.state
                    .clear_brightness_invalid(&instance.instance_id)
                    .await;
                self.state
                    .clear_error(&brightness_error_key(instance))
                    .await;
                apply_brightness_display(instance, settings, &display).await?;
                if settings.show_confirmation {
                    instance.show_ok().await?;
                }
                Ok(true)
            }
            Err(error) => {
                report_brightness_error(&self.state, instance, &error, show_alert).await?;
                Ok(false)
            }
        }
    }
}

#[derive(Debug)]
pub struct BrightnessDisplayState {
    value: f64,
    minimum: f64,
    maximum: f64,
    title: String,
}

async fn query_brightness_state(
    state: &Arc<PluginState>,
    settings: &BrightnessSettings,
) -> Result<BrightnessDisplayState> {
    let global = state.global_settings().await;
    let service = state
        .client
        .get_accessory(&global, None, settings.accessory_id())
        .await?;
    let characteristic = selected_brightness_characteristic(&service, settings)?;
    let value = characteristic
        .value
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("current brightness value is not numeric"))?;
    let title = display_title(&settings.display_name, &service.service_name);

    Ok(BrightnessDisplayState {
        value,
        minimum: characteristic.min_value.unwrap_or(0.0),
        maximum: characteristic.max_value.unwrap_or(100.0),
        title,
    })
}

fn selected_brightness_characteristic<'a>(
    service: &'a AccessoryService,
    settings: &BrightnessSettings,
) -> Result<&'a Characteristic> {
    let characteristic = service
        .characteristic_by_identity(
            settings.characteristic_uuid(),
            settings.characteristic_type(),
        )
        .or_else(|| service.characteristic("Brightness"))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "selected brightness characteristic is no longer available; service_id='{}', characteristic_uuid='{}', characteristic_type='{}', available=[{}]",
                settings.accessory_id(),
                settings.characteristic_uuid(),
                settings.characteristic_type(),
                service.available_characteristic_types(),
            )
        })?;

    if !characteristic.can_read {
        bail!("selected brightness characteristic cannot be read")
    }
    if !characteristic.can_write {
        bail!("selected brightness characteristic is read-only")
    }
    if !characteristic.is_brightness_compatible() {
        bail!("selected characteristic is not a writable Brightness value")
    }

    Ok(characteristic)
}

pub async fn refresh_brightness_instance(
    state: &Arc<PluginState>,
    instance: &Instance,
    settings: &BrightnessSettings,
    show_alert: bool,
) -> OpenActionResult<()> {
    if !settings.is_configured() {
        state.mark_brightness_invalid(&instance.instance_id).await;
        instance.set_title(Some("Not\nConfigured"), None).await?;
        return Ok(());
    }

    match query_brightness_state(state, settings).await {
        Ok(display) => {
            state
                .clear_brightness_invalid(&instance.instance_id)
                .await;
            state.clear_error(&brightness_error_key(instance)).await;
            apply_brightness_display(instance, settings, &display).await
        }
        Err(error) => report_brightness_error(state, instance, &error, show_alert).await,
    }
}

async fn apply_brightness_display(
    instance: &Instance,
    settings: &BrightnessSettings,
    display: &BrightnessDisplayState,
) -> OpenActionResult<()> {
    let percentage = percentage(display.value, display.minimum, display.maximum);
    let value_label = format!("{percentage:.0}%");
    let title = match settings.label_mode.as_str() {
        "valueOnly" => value_label.clone(),
        "nameOnly" => display.title.clone(),
        "hidden" => String::new(),
        _ => format!("{}\n{}", display.title, value_label),
    };

    if let Err(error) = send_arbitrary_json(serde_json::json!({
        "event": "setFeedback",
        "context": instance.instance_id.clone(),
        "payload": {
            "title": display.title.clone(),
            "value": value_label.clone(),
            "indicator": { "value": percentage }
        }
    }))
    .await
    {
        log::warn!("Could not send brightness encoder feedback: {error}");
    }

    instance.set_title(Some(title), None).await
}

async fn report_brightness_error(
    state: &Arc<PluginState>,
    instance: &Instance,
    error: &anyhow::Error,
    show_alert: bool,
) -> OpenActionResult<()> {
    let key = brightness_error_key(instance);
    let message = error.to_string();
    if state.should_log_error(&key, &message).await || show_alert {
        log::error!("Brightness {}: {message}", instance.instance_id);
    }
    if message.contains("not configured") || message.contains("no longer available") {
        state.mark_brightness_invalid(&instance.instance_id).await;
        instance.set_title(Some("Not\nConfigured"), None).await?;
    } else {
        instance.set_title(Some("Offline"), None).await?;
    }
    if show_alert {
        instance.show_alert().await?;
    }
    Ok(())
}

fn brightness_error_key(instance: &Instance) -> String {
    format!("brightness:{}", instance.instance_id)
}

fn sanitise_increment(configured: f64, homebridge_step: f64) -> f64 {
    if configured.is_finite() && configured > 0.0 {
        configured.max(homebridge_step)
    } else {
        homebridge_step.max(1.0)
    }
}

fn key_target(
    settings: &BrightnessSettings,
    current: f64,
    minimum: f64,
    maximum: f64,
    increment: f64,
) -> f64 {
    match settings.mode.as_str() {
        "decrease" => current - increment,
        "set" => settings.target_value,
        "cycle" => next_cycle_value(
            current,
            &settings.cycle_values,
            minimum,
            maximum,
            settings.wrap,
        ),
        _ => current + increment,
    }
}

fn next_cycle_value(
    current: f64,
    configured: &[f64],
    minimum: f64,
    maximum: f64,
    wrap: bool,
) -> f64 {
    let mut values = configured
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(minimum, maximum))
        .collect::<Vec<_>>();
    values.sort_by(f64::total_cmp);
    values.dedup_by(|left, right| (*left - *right).abs() < 0.000_001);
    if values.is_empty() {
        return maximum;
    }
    if let Some(next) = values.iter().find(|value| **value > current + 0.000_001) {
        return *next;
    }
    if wrap {
        values[0]
    } else {
        *values.last().unwrap_or(&maximum)
    }
}

fn align_and_clamp(value: f64, minimum: f64, maximum: f64, step: f64) -> f64 {
    let value = value.clamp(minimum, maximum);
    let aligned = minimum + ((value - minimum) / step).round() * step;
    aligned.clamp(minimum, maximum)
}

fn numeric_value(value: f64, characteristic: &Characteristic) -> Result<Value> {
    if !value.is_finite() {
        bail!("calculated brightness value is not finite")
    }
    if characteristic.is_integer() {
        return Ok(Value::Number(Number::from(value.round() as i64)));
    }
    Number::from_f64(value)
        .map(Value::Number)
        .ok_or_else(|| anyhow::anyhow!("brightness value cannot be represented as JSON"))
}

fn percentage(value: f64, minimum: f64, maximum: f64) -> f64 {
    if (maximum - minimum).abs() < f64::EPSILON {
        return 0.0;
    }
    ((value - minimum) * 100.0 / (maximum - minimum)).clamp(0.0, 100.0)
}

fn read_boolean(value: &Value) -> Result<bool> {
    if let Some(value) = value.as_bool() {
        return Ok(value);
    }
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
            "true" | "on" | "1" => Ok(true),
            "false" | "off" | "0" => Ok(false),
            _ => bail!("power value is not Boolean"),
        };
    }
    bail!("power value is not Boolean")
}

#[openaction::async_trait]
impl Action for BrightnessAction {
    type Settings = BrightnessSettings;
    const UUID: ActionUuid = BRIGHTNESS_UUID;

    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.state
            .remember_brightness(instance.instance_id.clone(), settings.clone())
            .await;
        refresh_brightness_instance(&self.state, instance, settings, false).await
    }

    async fn will_disappear(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.state.forget_brightness(&instance.instance_id).await;
        self.state
            .clear_error(&brightness_error_key(instance))
            .await;
        Ok(())
    }

    async fn key_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.apply_key_mode(instance, settings).await.map(|_| ())
    }

    async fn dial_rotate(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
        ticks: i16,
        _pressed: bool,
    ) -> OpenActionResult<()> {
        self.adjust_by_ticks(instance, settings, ticks).await.map(|_| ())
    }

    async fn dial_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.apply_key_mode(instance, settings).await.map(|_| ())
    }

    async fn did_receive_settings(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.state
            .remember_brightness(instance.instance_id.clone(), settings.clone())
            .await;
        refresh_brightness_instance(&self.state, instance, settings, false).await
    }

    async fn property_inspector_did_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        let global = self.state.global_settings().await;
        send_catalog(
            instance,
            &self.state,
            &global,
            None,
            "brightness",
            false,
        )
        .await?;
        refresh_brightness_instance(&self.state, instance, settings, false).await
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
            && request.event != "testBrightness"
        {
            return Ok(());
        }

        let global = apply_global_settings(&self.state, request.global_settings).await?;
        let settings = if request.action_settings.is_null() {
            current.clone()
        } else {
            serde_json::from_value::<BrightnessSettings>(request.action_settings)?
        };
        instance.set_settings(&settings).await?;
        self.state
            .remember_brightness(instance.instance_id.clone(), settings.clone())
            .await;

        if request.event == "testBrightness" {
            let changed = self.apply_key_mode(instance, &settings).await?;
            send_status(
                instance,
                if changed { "connected" } else { "error" },
                if changed {
                    "Brightness changed and confirmed by Homebridge"
                } else {
                    "Brightness command failed; check the plugin log"
                },
            )
            .await?;
            return Ok(());
        }

        send_catalog(
            instance,
            &self.state,
            &global,
            request.otp.as_deref(),
            "brightness",
            request.force_refresh,
        )
        .await?;
        refresh_brightness_instance(&self.state, instance, &settings, false).await
    }
}

#[cfg(test)]
mod tests {
    use super::{align_and_clamp, next_cycle_value, percentage};

    #[test]
    fn cycles_and_wraps_brightness_values() {
        assert_eq!(next_cycle_value(50.0, &[25.0, 50.0, 75.0], 0.0, 100.0, true), 75.0);
        assert_eq!(next_cycle_value(75.0, &[25.0, 50.0, 75.0], 0.0, 100.0, true), 25.0);
        assert_eq!(next_cycle_value(75.0, &[25.0, 50.0, 75.0], 0.0, 100.0, false), 75.0);
    }

    #[test]
    fn aligns_to_homebridge_step() {
        assert_eq!(align_and_clamp(53.0, 0.0, 100.0, 5.0), 55.0);
        assert_eq!(align_and_clamp(110.0, 0.0, 100.0, 1.0), 100.0);
    }

    #[test]
    fn calculates_percentage_for_nonstandard_ranges() {
        assert_eq!(percentage(50.0, 0.0, 100.0), 50.0);
        assert_eq!(percentage(15.0, 10.0, 20.0), 50.0);
    }
}
