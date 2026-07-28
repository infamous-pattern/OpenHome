use std::sync::Arc;
use std::time::Duration;

use openaction::visible_instances;

use crate::actions::adjust::{ADJUST_UUID, refresh_adjust_instance};
use crate::actions::brightness::{BRIGHTNESS_UUID, refresh_brightness_instance};
use crate::actions::switch::{SWITCH_UUID, refresh_switch_instance};
use crate::models::SelectedCharacteristic;
use crate::state::PluginState;

pub fn spawn_state_poller(state: Arc<PluginState>) {
    tokio::spawn(async move {
        loop {
            let interval = state.global_settings().await.polling_interval();
            tokio::time::sleep(Duration::from_secs(interval)).await;

            for instance in visible_instances(SWITCH_UUID).await {
                let Some(settings) = state.switch_settings(&instance.instance_id).await else {
                    continue;
                };
                if !settings.is_configured()
                    || !state.switch_polling_enabled(&instance.instance_id).await
                {
                    continue;
                }
                if let Err(error) =
                    refresh_switch_instance(&state, &instance, &settings, false).await
                {
                    log::warn!("Could not refresh switch {}: {error}", instance.instance_id);
                }
            }

            for instance in visible_instances(ADJUST_UUID).await {
                let Some(settings) = state.adjust_settings(&instance.instance_id).await else {
                    continue;
                };
                if !settings.is_configured() {
                    continue;
                }
                if let Err(error) =
                    refresh_adjust_instance(&state, &instance, &settings, false).await
                {
                    log::warn!("Could not refresh encoder {}: {error}", instance.instance_id);
                }
            }

            for instance in visible_instances(BRIGHTNESS_UUID).await {
                let Some(settings) = state.brightness_settings(&instance.instance_id).await else {
                    continue;
                };
                if !settings.is_configured()
                    || !state
                        .brightness_polling_enabled(&instance.instance_id)
                        .await
                {
                    continue;
                }
                if let Err(error) =
                    refresh_brightness_instance(&state, &instance, &settings, false).await
                {
                    log::warn!(
                        "Could not refresh brightness {}: {error}",
                        instance.instance_id
                    );
                }
            }
        }
    });
}
