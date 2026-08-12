use std::sync::Arc;
use std::time::Duration;

use openaction::visible_instances;
use tokio::sync::watch;

use crate::actions::adjust::{ADJUST_UUID, refresh_adjust_instance};
use crate::actions::brightness::{BRIGHTNESS_UUID, refresh_brightness_instance};
use crate::actions::devices::{DEVICES_UUID, refresh_devices_instance};
use crate::actions::switch::{SWITCH_UUID, refresh_switch_instance};
use crate::models::SelectedCharacteristic;
use crate::state::PluginState;

const STARTUP_GRACE_SECONDS: u64 = 3;
const HEALTH_CHECK_SECONDS: u64 = 60;
const RETRY_DELAYS_SECONDS: [u64; 5] = [2, 5, 10, 30, 60];

pub fn spawn_state_poller(state: Arc<PluginState>) {
    tokio::spawn(async move {
        loop {
            let interval = state.global_settings().await.polling_interval();
            tokio::time::sleep(Duration::from_secs(interval)).await;

            if !state.global_settings_loaded() || !state.connection_online() {
                continue;
            }

            let _ = refresh_visible_actions(&state).await;
        }
    });
}

pub fn spawn_reconnect_monitor(state: Arc<PluginState>) {
    tokio::spawn(async move {
        let mut reconnect_rx = state.reconnect_receiver();

        while !state.global_settings_loaded() {
            if reconnect_rx.changed().await.is_err() {
                return;
            }
        }

        log::info!(
            "OpenHomeB startup: Homebridge settings loaded; waiting {} seconds for network readiness",
            STARTUP_GRACE_SECONDS
        );
        if wait_or_reconnect(
            &mut reconnect_rx,
            Duration::from_secs(STARTUP_GRACE_SECONDS),
        )
        .await
        {
            log::info!(
                "OpenHomeB startup: connection settings changed during startup grace period"
            );
        }

        let mut retry_index = 0_usize;
        let mut last_failure: Option<(usize, String)> = None;

        loop {
            if !state.global_settings_loaded() {
                if reconnect_rx.changed().await.is_err() {
                    return;
                }
                continue;
            }

            let settings = state.global_settings().await;
            match state.client.refresh_catalog_live(&settings, None).await {
                Ok(catalog) => {
                    let reconnected = state.mark_connection_online();
                    if reconnected {
                        log::info!("OpenHomeB reconnect: Homebridge API reachable");
                        log::info!(
                            "OpenHomeB reconnect: authenticated successfully ({})",
                            catalog.authentication
                        );
                        log::info!(
                            "OpenHomeB reconnect: refreshed {} services across {} devices",
                            catalog.service_count,
                            catalog.device_count
                        );
                    }

                    retry_index = 0;
                    last_failure = None;
                    let refreshed = refresh_visible_actions(&state).await;
                    if reconnected && refreshed > 0 {
                        log::info!("OpenHomeB reconnect: refreshed {refreshed} visible actions");
                    }

                    if wait_or_reconnect(
                        &mut reconnect_rx,
                        Duration::from_secs(HEALTH_CHECK_SECONDS),
                    )
                    .await
                    {
                        state.mark_connection_offline();
                        log::info!(
                            "OpenHomeB reconnect: reconnect requested; refreshing Homebridge"
                        );
                    }
                }
                Err(error) => {
                    let was_online = state.mark_connection_offline();
                    let message = error.to_string();
                    let delay = RETRY_DELAYS_SECONDS[retry_index];
                    let failure = (retry_index, message.clone());

                    if was_online || last_failure.as_ref() != Some(&failure) {
                        log::warn!(
                            "OpenHomeB reconnect: Homebridge unavailable ({message}); retrying in {delay}s"
                        );
                        last_failure = Some(failure);
                    }

                    if wait_or_reconnect(&mut reconnect_rx, Duration::from_secs(delay)).await {
                        retry_index = 0;
                        last_failure = None;
                        log::info!(
                            "OpenHomeB reconnect: settings or system state changed; retrying now"
                        );
                    } else {
                        retry_index = (retry_index + 1).min(RETRY_DELAYS_SECONDS.len() - 1);
                    }
                }
            }
        }
    });
}

pub async fn refresh_visible_actions(state: &Arc<PluginState>) -> usize {
    let mut refreshed = 0_usize;

    for instance in visible_instances(DEVICES_UUID).await {
        if let Err(error) = refresh_devices_instance(state, &instance).await {
            log::warn!(
                "Could not refresh Homebridge devices action {}: {error}",
                instance.instance_id
            );
        } else {
            refreshed += 1;
        }
    }

    for instance in visible_instances(SWITCH_UUID).await {
        let Some(settings) = state.switch_settings(&instance.instance_id).await else {
            continue;
        };
        if !settings.is_configured() || !state.switch_polling_enabled(&instance.instance_id).await {
            continue;
        }
        if let Err(error) = refresh_switch_instance(state, &instance, &settings, false).await {
            log::warn!("Could not refresh switch {}: {error}", instance.instance_id);
        } else {
            refreshed += 1;
        }
    }

    for instance in visible_instances(ADJUST_UUID).await {
        let Some(settings) = state.adjust_settings(&instance.instance_id).await else {
            continue;
        };
        if !settings.is_configured() {
            continue;
        }
        if let Err(error) = refresh_adjust_instance(state, &instance, &settings, false).await {
            log::warn!(
                "Could not refresh encoder {}: {error}",
                instance.instance_id
            );
        } else {
            refreshed += 1;
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
        if let Err(error) = refresh_brightness_instance(state, &instance, &settings, false).await {
            log::warn!(
                "Could not refresh brightness {}: {error}",
                instance.instance_id
            );
        } else {
            refreshed += 1;
        }
    }

    refreshed
}

async fn wait_or_reconnect(reconnect_rx: &mut watch::Receiver<u64>, duration: Duration) -> bool {
    tokio::select! {
        _ = tokio::time::sleep(duration) => false,
        changed = reconnect_rx.changed() => changed.is_ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::RETRY_DELAYS_SECONDS;

    #[test]
    fn reconnect_backoff_is_bounded_and_progressive() {
        assert_eq!(RETRY_DELAYS_SECONDS, [2, 5, 10, 30, 60]);
    }
}
