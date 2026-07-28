use std::sync::Arc;

use openaction::{Instance, OpenActionResult, set_global_settings};
use serde::Serialize;
use serde_json::{Value, json};

use crate::models::{GlobalSettings, PropertyInspectorRequest};
use crate::state::PluginState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusMessage<'a> {
    event: &'a str,
    status: &'a str,
    message: &'a str,
}

pub async fn send_status(
    instance: &Instance,
    status: &str,
    message: &str,
) -> OpenActionResult<()> {
    instance
        .send_to_property_inspector(StatusMessage {
            event: "status",
            status,
            message,
        })
        .await
}

pub async fn apply_global_settings(
    state: &Arc<PluginState>,
    settings: Option<GlobalSettings>,
) -> OpenActionResult<GlobalSettings> {
    if let Some(settings) = settings {
        state.update_global_settings(settings.clone()).await;
        set_global_settings(&settings).await?;
        Ok(settings)
    } else {
        Ok(state.global_settings().await)
    }
}

pub async fn send_catalog(
    instance: &Instance,
    state: &Arc<PluginState>,
    global: &GlobalSettings,
    otp: Option<&str>,
    action_kind: &str,
    force_refresh: bool,
) -> OpenActionResult<()> {
    send_status(
        instance,
        "connecting",
        if force_refresh {
            "Refreshing devices from Homebridge…"
        } else {
            "Loading Homebridge devices…"
        },
    )
    .await?;

    match state.client.catalog(global, otp, force_refresh).await {
        Ok(catalog) => {
            let status_message = if catalog.stale {
                catalog
                    .warning
                    .clone()
                    .unwrap_or_else(|| "Showing stale cached device data".to_string())
            } else if catalog.cached {
                format!(
                    "Connected to Homebridge · cached catalogue ({} seconds old)",
                    catalog.cache_age_seconds
                )
            } else {
                "Connected to Homebridge · live catalogue refreshed".to_string()
            };
            instance
                .send_to_property_inspector(json!({
                    "event": "catalog",
                    "actionKind": action_kind,
                    "catalog": catalog,
                }))
                .await?;
            send_status(
                instance,
                if status_message.contains("stale") || status_message.contains("failed") {
                    "warning"
                } else {
                    "connected"
                },
                &status_message,
            )
            .await?;
        }
        Err(error) => {
            send_status(instance, "error", &error.to_string()).await?;
        }
    }

    Ok(())
}

pub fn parse_request(payload: &Value) -> Result<PropertyInspectorRequest, serde_json::Error> {
    serde_json::from_value(payload.clone())
}

pub async fn action_error(
    instance: &Instance,
    title: &str,
    error: impl std::fmt::Display,
    show_alert: bool,
) -> OpenActionResult<()> {
    log::error!("{title}: {error}");
    instance.set_title(Some(title), None).await?;
    if show_alert {
        instance.show_alert().await?;
    }
    Ok(())
}

pub fn display_title(configured_name: &str, fallback: &str) -> String {
    let value = if configured_name.trim().is_empty() {
        fallback.trim()
    } else {
        configured_name.trim()
    };

    if value.is_empty() {
        "Homebridge".to_string()
    } else {
        value.to_string()
    }
}
