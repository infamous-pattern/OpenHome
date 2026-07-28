use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Map, Value, json};
use tokio::sync::RwLock;

use crate::models::{
    AccessoryService, AuthenticationStatus, Catalog, CatalogService, DeviceMetadata,
    GlobalSettings, RoomLayout,
};

#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    token_type: String,
    expires_at: Instant,
    refresh_at: Instant,
    authentication: String,
    authenticated_at_epoch_ms: u64,
    expires_at_epoch_ms: u64,
    refresh_at_epoch_ms: u64,
}

impl CachedToken {
    fn status(&self) -> AuthenticationStatus {
        AuthenticationStatus {
            method: self.authentication.clone(),
            authenticated_at_epoch_ms: self.authenticated_at_epoch_ms,
            expires_at_epoch_ms: self.expires_at_epoch_ms,
            refresh_at_epoch_ms: self.refresh_at_epoch_ms,
            remaining_seconds: self
                .expires_at
                .checked_duration_since(Instant::now())
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

#[derive(Debug, Clone)]
struct CachedCatalog {
    catalog: Catalog,
    fetched_at: Instant,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default = "default_token_type")]
    token_type: String,
    #[serde(default = "default_expires_in")]
    expires_in: u64,
}

fn default_token_type() -> String {
    "Bearer".to_string()
}

fn default_expires_in() -> u64 {
    3_600
}

#[derive(Debug, Serialize)]
struct LoginRequest<'a> {
    username: &'a str,
    password: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    otp: Option<&'a str>,
}

pub struct HomebridgeClient {
    client: Client,
    tokens: RwLock<HashMap<u64, CachedToken>>,
    catalogs: RwLock<HashMap<u64, CachedCatalog>>,
}

impl HomebridgeClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(20))
            .user_agent("OpenHomeB/2.0.1")
            .build()
            .context("could not create the Homebridge HTTP client")?;

        Ok(Self {
            client,
            tokens: RwLock::new(HashMap::new()),
            catalogs: RwLock::new(HashMap::new()),
        })
    }

    pub async fn clear_tokens(&self) {
        self.tokens.write().await.clear();
    }

    pub async fn clear_catalogs(&self) {
        self.catalogs.write().await.clear();
    }

    pub async fn clear_all_caches(&self) {
        self.clear_tokens().await;
        self.clear_catalogs().await;
    }

    #[allow(clippy::collapsible_if)]
    pub async fn catalog(
        &self,
        settings: &GlobalSettings,
        otp: Option<&str>,
        force_refresh: bool,
    ) -> Result<Catalog> {
        let base_url = normalise_base_url(&settings.homebridge_url)?;
        let key = credential_key(&base_url, settings);
        let cache_ttl = Duration::from_secs(settings.catalog_cache_ttl());

        if !force_refresh {
            if let Some(cached) = self.catalogs.read().await.get(&key).cloned() {
                let age = cached.fetched_at.elapsed();
                if age <= cache_ttl {
                    return Ok(decorate_cached_catalog(cached, false, None));
                }
            }
        }

        match self.fetch_catalog(settings, otp).await {
            Ok(catalog) => {
                self.catalogs.write().await.insert(
                    key,
                    CachedCatalog {
                        catalog: catalog.clone(),
                        fetched_at: Instant::now(),
                    },
                );
                Ok(catalog)
            }
            Err(error) => {
                if let Some(cached) = self.catalogs.read().await.get(&key).cloned() {
                    log::warn!(
                        "Homebridge catalog refresh failed; returning stale cached catalog: {error}"
                    );
                    Ok(decorate_cached_catalog(
                        cached,
                        true,
                        Some(format!("Live refresh failed: {error}")),
                    ))
                } else {
                    Err(error)
                }
            }
        }
    }

    async fn fetch_catalog(&self, settings: &GlobalSettings, otp: Option<&str>) -> Result<Catalog> {
        let (mut services, authentication, authentication_status) =
            self.list_accessories(settings, otp).await?;
        let layout = self.list_layout(settings, otp).await.unwrap_or_default();

        let mut room_by_service: HashMap<String, (String, Option<String>)> = HashMap::new();
        for room in &layout {
            for service in &room.services {
                room_by_service.insert(
                    service.unique_id.clone(),
                    (room.name.clone(), service.custom_name.clone()),
                );
            }
        }

        services.sort_by(|left, right| {
            left.accessory_name
                .to_ascii_lowercase()
                .cmp(&right.accessory_name.to_ascii_lowercase())
                .then_with(|| {
                    left.service_name
                        .to_ascii_lowercase()
                        .cmp(&right.service_name.to_ascii_lowercase())
                })
        });

        let mut device_keys = HashSet::new();
        let mut catalog_services = Vec::with_capacity(services.len());
        for service in services {
            let bridge = if service.instance.username.trim().is_empty() {
                &service.instance.name
            } else {
                &service.instance.username
            };
            device_keys.insert(format!("{bridge}\u{1f}{}", service.accessory_name));

            let (room_name, custom_name) = room_by_service
                .get(&service.unique_id)
                .cloned()
                .unwrap_or_else(|| ("Unassigned".to_string(), None));
            let device_metadata =
                DeviceMetadata::from_accessory_information(&service.accessory_information);

            catalog_services.push(CatalogService {
                room_name,
                custom_name,
                device_metadata,
                service,
            });
        }

        let mut rooms = catalog_services
            .iter()
            .map(|service| service.room_name.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        rooms.sort_by_key(|room| room.to_ascii_lowercase());

        Ok(Catalog {
            authentication,
            authentication_status,
            device_count: device_keys.len(),
            service_count: catalog_services.len(),
            rooms,
            services: catalog_services,
            refreshed_at_epoch_ms: now_epoch_ms(),
            cache_age_seconds: 0,
            cached: false,
            stale: false,
            warning: None,
        })
    }

    pub async fn list_accessories(
        &self,
        settings: &GlobalSettings,
        otp: Option<&str>,
    ) -> Result<(Vec<AccessoryService>, String, AuthenticationStatus)> {
        let (value, token): (Value, CachedToken) = self
            .authorized_get(settings, otp, "/api/accessories")
            .await?;
        let services = parse_accessory_services(value)?;
        Ok((services, token.authentication.clone(), token.status()))
    }

    pub async fn list_layout(
        &self,
        settings: &GlobalSettings,
        otp: Option<&str>,
    ) -> Result<Vec<RoomLayout>> {
        let (value, _): (Vec<RoomLayout>, CachedToken) = self
            .authorized_get(settings, otp, "/api/accessories/layout")
            .await?;
        Ok(value)
    }

    pub async fn get_accessory(
        &self,
        settings: &GlobalSettings,
        otp: Option<&str>,
        unique_id: &str,
    ) -> Result<AccessoryService> {
        let path = accessory_path(unique_id);
        let (value, _): (Value, CachedToken) = self.authorized_get(settings, otp, &path).await?;
        let service = parse_accessory_service(value)?;
        self.update_cached_service(settings, &service).await;
        Ok(service)
    }

    pub async fn set_characteristic(
        &self,
        settings: &GlobalSettings,
        otp: Option<&str>,
        unique_id: &str,
        characteristic_type: &str,
        value: Value,
    ) -> Result<AccessoryService> {
        let path = accessory_path(unique_id);
        let body = json!({
            "characteristicType": characteristic_type,
            "value": value,
        });
        log::info!(
            "Homebridge write: service_id='{}', characteristic_type='{}', value={}",
            unique_id,
            characteristic_type,
            body["value"],
        );
        let result: Value = self.authorized_put(settings, otp, &path, &body).await?;
        let service = parse_accessory_service(result)?;
        self.update_cached_service(settings, &service).await;
        Ok(service)
    }

    async fn update_cached_service(&self, settings: &GlobalSettings, service: &AccessoryService) {
        let Ok(base_url) = normalise_base_url(&settings.homebridge_url) else {
            return;
        };
        let key = credential_key(&base_url, settings);
        let mut catalogs = self.catalogs.write().await;
        let Some(cached) = catalogs.get_mut(&key) else {
            return;
        };
        if let Some(item) = cached
            .catalog
            .services
            .iter_mut()
            .find(|item| item.service.unique_id == service.unique_id)
        {
            item.service = service.clone();
        }
    }

    async fn authorized_get<T: DeserializeOwned>(
        &self,
        settings: &GlobalSettings,
        otp: Option<&str>,
        path: &str,
    ) -> Result<(T, CachedToken)> {
        let base_url = normalise_base_url(&settings.homebridge_url)?;
        let key = credential_key(&base_url, settings);

        for attempt in 0..2 {
            let token = self.token(&base_url, settings, otp, attempt > 0).await?;
            let endpoint = format!("{base_url}{path}");
            let response = self
                .client
                .get(&endpoint)
                .header(
                    "Authorization",
                    format!("{} {}", token.token_type, token.access_token),
                )
                .send()
                .await
                .with_context(|| format!("could not connect to {endpoint}"))?;

            if response.status() == StatusCode::UNAUTHORIZED && attempt == 0 {
                log::warn!(
                    "Homebridge GET {endpoint}: token rejected; authenticating and retrying once"
                );
                self.tokens.write().await.remove(&key);
                continue;
            }

            let value = decode_response(response, &endpoint).await?;
            return Ok((value, token));
        }

        bail!("Homebridge authentication failed")
    }

    async fn authorized_put<T: DeserializeOwned>(
        &self,
        settings: &GlobalSettings,
        otp: Option<&str>,
        path: &str,
        body: &Value,
    ) -> Result<T> {
        let base_url = normalise_base_url(&settings.homebridge_url)?;
        let key = credential_key(&base_url, settings);

        for attempt in 0..2 {
            let token = self.token(&base_url, settings, otp, attempt > 0).await?;
            let endpoint = format!("{base_url}{path}");
            log::info!("Homebridge PUT {endpoint}: request body={body}");
            let response = self
                .client
                .put(&endpoint)
                .header(
                    "Authorization",
                    format!("{} {}", token.token_type, token.access_token),
                )
                .json(body)
                .send()
                .await
                .with_context(|| format!("could not connect to {endpoint}"))?;

            let status = response.status();
            log::info!("Homebridge PUT {endpoint}: HTTP {status}");
            if status == StatusCode::UNAUTHORIZED && attempt == 0 {
                log::warn!(
                    "Homebridge PUT {endpoint}: token rejected; authenticating and retrying once"
                );
                self.tokens.write().await.remove(&key);
                continue;
            }

            return decode_response(response, &endpoint).await;
        }

        bail!("Homebridge authentication failed")
    }

    #[allow(clippy::collapsible_if)]
    async fn token(
        &self,
        base_url: &str,
        settings: &GlobalSettings,
        otp: Option<&str>,
        force_refresh: bool,
    ) -> Result<CachedToken> {
        let key = credential_key(base_url, settings);
        if !force_refresh {
            if let Some(token) = self.tokens.read().await.get(&key).cloned() {
                if token.refresh_at > Instant::now() {
                    return Ok(token);
                }
                log::info!(
                    "Homebridge authentication token is approaching expiry; refreshing proactively"
                );
            }
        }

        let token = self.authenticate(base_url, settings, otp).await?;
        self.tokens.write().await.insert(key, token.clone());
        Ok(token)
    }

    async fn authenticate(
        &self,
        base_url: &str,
        settings: &GlobalSettings,
        otp: Option<&str>,
    ) -> Result<CachedToken> {
        let no_auth_endpoint = format!("{base_url}/api/auth/noauth");
        let no_auth_response = self
            .client
            .post(&no_auth_endpoint)
            .send()
            .await
            .with_context(|| format!("could not connect to {no_auth_endpoint}"))?;

        if no_auth_response.status().is_success() {
            let response: TokenResponse =
                decode_response(no_auth_response, &no_auth_endpoint).await?;
            return Ok(cached_token(response, "Homebridge authentication disabled"));
        }

        if settings.username.trim().is_empty() || settings.password.is_empty() {
            bail!("Homebridge requires authentication. Enter its UI username and password")
        }

        let login_endpoint = format!("{base_url}/api/auth/login");
        let response = self
            .client
            .post(&login_endpoint)
            .json(&LoginRequest {
                username: settings.username.trim(),
                password: &settings.password,
                otp: otp.filter(|value| !value.trim().is_empty()),
            })
            .send()
            .await
            .with_context(|| format!("could not connect to {login_endpoint}"))?;

        if response.status() == StatusCode::PRECONDITION_FAILED {
            let body = response.text().await.unwrap_or_default();
            bail!(
                "Homebridge requires a valid two-factor authentication code: {}",
                error_detail(&body)
            )
        }

        let token: TokenResponse = decode_response(response, &login_endpoint).await?;
        Ok(cached_token(token, "Homebridge username/password"))
    }
}

fn decorate_cached_catalog(cached: CachedCatalog, stale: bool, warning: Option<String>) -> Catalog {
    let mut catalog = cached.catalog;
    catalog.cache_age_seconds = cached.fetched_at.elapsed().as_secs();
    catalog.cached = true;
    catalog.stale = stale;
    catalog.warning = warning;
    let now = now_epoch_ms();
    catalog.authentication_status.remaining_seconds = catalog
        .authentication_status
        .expires_at_epoch_ms
        .saturating_sub(now)
        / 1_000;
    catalog
}

fn cached_token(response: TokenResponse, authentication: &str) -> CachedToken {
    let expires_in = response.expires_in.max(30);
    let mut refresh_buffer = (expires_in / 10).clamp(30, 600);
    refresh_buffer = refresh_buffer.min(expires_in.saturating_sub(1));
    let refresh_in = expires_in.saturating_sub(refresh_buffer).max(1);
    let now = Instant::now();
    let now_epoch = now_epoch_ms();

    CachedToken {
        access_token: response.access_token,
        token_type: response.token_type,
        expires_at: now + Duration::from_secs(expires_in),
        refresh_at: now + Duration::from_secs(refresh_in),
        authentication: authentication.to_string(),
        authenticated_at_epoch_ms: now_epoch,
        expires_at_epoch_ms: now_epoch.saturating_add(expires_in.saturating_mul(1_000)),
        refresh_at_epoch_ms: now_epoch.saturating_add(refresh_in.saturating_mul(1_000)),
    }
}

async fn decode_response<T: DeserializeOwned>(
    response: reqwest::Response,
    endpoint: &str,
) -> Result<T> {
    let status = response.status();
    let body = response
        .text()
        .await
        .context("could not read the Homebridge response")?;

    if !status.is_success() {
        let detail = error_detail(&body);
        if status == StatusCode::BAD_REQUEST
            && detail.to_ascii_lowercase().contains("insecure mode")
        {
            bail!("Homebridge must run in insecure mode (-I) before accessories can be controlled")
        }
        bail!("Homebridge returned HTTP {status} from {endpoint}: {detail}")
    }

    serde_json::from_str(&body).with_context(|| {
        format!(
            "Homebridge returned invalid JSON from {endpoint}: {}",
            error_detail(&body)
        )
    })
}

fn parse_accessory_services(value: Value) -> Result<Vec<AccessoryService>> {
    let values = match value {
        Value::Array(values) => values,
        Value::Object(mut object) => {
            let nested = object
                .remove("accessories")
                .or_else(|| object.remove("services"))
                .ok_or_else(|| anyhow!("Homebridge accessory response is not an array"))?;
            nested
                .as_array()
                .cloned()
                .ok_or_else(|| anyhow!("Homebridge accessory collection is not an array"))?
        }
        _ => bail!("Homebridge accessory response is not an array"),
    };

    values
        .into_iter()
        .map(parse_accessory_service)
        .collect::<Result<Vec<_>>>()
}

fn parse_accessory_service(value: Value) -> Result<AccessoryService> {
    let normalised = normalise_service_value(value)?;
    serde_json::from_value::<AccessoryService>(normalised)
        .map(AccessoryService::normalise)
        .context("Homebridge returned an unsupported accessory service structure")
}

#[allow(clippy::collapsible_if)]
fn normalise_service_value(value: Value) -> Result<Value> {
    let mut object = value
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("Homebridge accessory service is not an object"))?;

    copy_alias(&mut object, "uniqueId", &["uniqueID", "id"]);
    copy_alias(&mut object, "serviceName", &["displayName", "name"]);
    copy_alias(&mut object, "accessoryName", &["accessoryDisplayName"]);
    copy_alias(&mut object, "humanType", &["serviceTypeName"]);
    copy_alias(&mut object, "serviceType", &["humanType"]);

    if !object.contains_key("uniqueId") {
        if let Some(uuid) = object.get("uuid").cloned() {
            object.insert("uniqueId".to_string(), uuid);
        }
    }

    let mut accessory_information = object
        .get("accessoryInformation")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for (target, aliases) in [
        ("Name", &["accessoryName", "name"][..]),
        ("Manufacturer", &["manufacturer"][..]),
        ("Model", &["model"][..]),
        ("Serial Number", &["serialNumber", "serial_number"][..]),
        ("Firmware Revision", &["firmwareRevision"][..]),
    ] {
        if !accessory_information.contains_key(target) {
            if let Some(value) = aliases.iter().find_map(|key| object.get(*key)).cloned() {
                accessory_information.insert(target.to_string(), value);
            }
        }
    }
    object.insert(
        "accessoryInformation".to_string(),
        Value::Object(accessory_information),
    );

    let characteristics = if let Some(value) = object.get("serviceCharacteristics") {
        value.as_array().cloned().unwrap_or_default()
    } else if let Some(value) = object.get("characteristics") {
        value.as_array().cloned().unwrap_or_default()
    } else if let Some(values) = object.get("values").and_then(Value::as_object) {
        characteristics_from_values(values, &object)
    } else {
        Vec::new()
    };

    let characteristics = characteristics
        .into_iter()
        .map(normalise_characteristic_value)
        .collect::<Result<Vec<_>>>()?;
    object.insert(
        "serviceCharacteristics".to_string(),
        Value::Array(characteristics),
    );

    Ok(Value::Object(object))
}

fn characteristics_from_values(
    values: &Map<String, Value>,
    service: &Map<String, Value>,
) -> Vec<Value> {
    let explicitly_writable = service
        .get("writableCharacteristics")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|value| value.to_ascii_lowercase())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();

    values
        .iter()
        .map(|(characteristic_type, value)| {
            let can_write = explicitly_writable.contains(&characteristic_type.to_ascii_lowercase())
                || legacy_permission_can_write(service, characteristic_type);
            json!({
                "type": characteristic_type,
                "description": characteristic_type,
                "value": value,
                "format": infer_format(value),
                "canRead": true,
                "canWrite": can_write,
            })
        })
        .collect()
}

fn legacy_permission_can_write(service: &Map<String, Value>, characteristic_type: &str) -> bool {
    let Some(permissions) = service
        .get("characteristicPermissions")
        .and_then(Value::as_object)
        .and_then(|items| items.get(characteristic_type))
    else {
        return false;
    };

    if permissions
        .get("canWrite")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return true;
    }

    permissions
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some("pw")))
}

#[allow(clippy::collapsible_if)]
fn normalise_characteristic_value(value: Value) -> Result<Value> {
    let mut object = value
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("Homebridge characteristic is not an object"))?;

    copy_alias(&mut object, "type", &["characteristicType", "name"]);
    copy_alias(&mut object, "uuid", &["characteristicUuid"]);
    copy_alias(&mut object, "description", &["displayName", "name"]);

    if let Some(props) = object.get("props").and_then(Value::as_object).cloned() {
        for key in ["format", "minValue", "maxValue", "minStep", "validValues"] {
            if !object.contains_key(key) {
                if let Some(value) = props.get(key).cloned() {
                    object.insert(key.to_string(), value);
                }
            }
        }
        if !object.contains_key("perms") {
            if let Some(value) = props.get("perms").cloned() {
                object.insert("perms".to_string(), value);
            }
        }
    }

    let permissions = object
        .get("perms")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !object.contains_key("canRead") {
        let can_read = object
            .get("readable")
            .and_then(Value::as_bool)
            .unwrap_or_else(|| permissions.iter().any(|item| item.as_str() == Some("pr")));
        object.insert("canRead".to_string(), Value::Bool(can_read));
    }
    if !object.contains_key("canWrite") {
        let can_write = object
            .get("writable")
            .and_then(Value::as_bool)
            .unwrap_or_else(|| permissions.iter().any(|item| item.as_str() == Some("pw")));
        object.insert("canWrite".to_string(), Value::Bool(can_write));
    }
    if !object.contains_key("format") {
        object.insert(
            "format".to_string(),
            Value::String(infer_format(object.get("value").unwrap_or(&Value::Null))),
        );
    }
    if !object.contains_key("description") {
        let description = object
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("Characteristic")
            .to_string();
        object.insert("description".to_string(), Value::String(description));
    }

    Ok(Value::Object(object))
}

fn copy_alias(object: &mut Map<String, Value>, target: &str, aliases: &[&str]) {
    if object.contains_key(target) {
        return;
    }
    if let Some(value) = aliases.iter().find_map(|alias| object.get(*alias)).cloned() {
        object.insert(target.to_string(), value);
    }
}

fn infer_format(value: &Value) -> String {
    match value {
        Value::Bool(_) => "bool",
        Value::Number(number) if number.is_i64() || number.is_u64() => "int",
        Value::Number(_) => "float",
        Value::String(_) => "string",
        _ => "unknown",
    }
    .to_string()
}

fn accessory_path(unique_id: &str) -> String {
    let encoded = utf8_percent_encode(unique_id, NON_ALPHANUMERIC).to_string();
    format!("/api/accessories/{encoded}")
}

pub fn normalise_base_url(input: &str) -> Result<String> {
    let mut value = input.trim().trim_end_matches('/').to_string();
    if value.is_empty() {
        bail!("enter the Homebridge UI address, for example http://homebridge.local:8581")
    }

    if value.contains("://") && !value.starts_with("http://") && !value.starts_with("https://") {
        bail!("Homebridge URL must use http:// or https://")
    }
    if !value.starts_with("http://") && !value.starts_with("https://") {
        value = format!("http://{value}");
    }

    let mut parsed = reqwest::Url::parse(&value)
        .with_context(|| format!("{value} is not a valid Homebridge URL"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        bail!("Homebridge URL must use http:// or https://")
    }
    if parsed.host_str().is_none() {
        return Err(anyhow!("{value} does not contain a host name"));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        bail!(
            "do not include credentials in the Homebridge URL; use the username and password fields"
        )
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        bail!("Homebridge URL must not contain a query string or fragment")
    }

    let mut path = parsed.path().trim_end_matches('/').to_string();
    if path.ends_with("/api") {
        path.truncate(path.len() - 4);
    }
    parsed.set_path(&path);
    parsed.set_query(None);
    parsed.set_fragment(None);

    let normalised = parsed.to_string().trim_end_matches('/').to_string();
    if normalised.is_empty() {
        bail!("Homebridge URL is empty after normalisation")
    }

    Ok(normalised)
}

fn credential_key(base_url: &str, settings: &GlobalSettings) -> u64 {
    let mut hasher = DefaultHasher::new();
    base_url.hash(&mut hasher);
    settings.username.hash(&mut hasher);
    settings.password.hash(&mut hasher);
    hasher.finish()
}

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn error_detail(body: &str) -> String {
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        if let Some(message) = value.get("message") {
            if let Some(text) = message.as_str() {
                return text.to_string();
            }
            if let Some(items) = message.as_array() {
                let joined = items
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join("; ");
                if !joined.is_empty() {
                    return joined;
                }
            }
        }
        if let Some(error) = value.get("error").and_then(Value::as_str) {
            return error.to_string();
        }
    }

    let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty() {
        "no additional error information".to_string()
    } else if compact.chars().count() > 240 {
        format!("{}…", compact.chars().take(240).collect::<String>())
    } else {
        compact
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{normalise_base_url, parse_accessory_services};

    #[test]
    fn adds_http_scheme() {
        assert_eq!(
            normalise_base_url("homebridge.local:8581").unwrap(),
            "http://homebridge.local:8581"
        );
    }

    #[test]
    fn removes_api_suffix() {
        assert_eq!(
            normalise_base_url("http://localhost:8581/api/").unwrap(),
            "http://localhost:8581"
        );
    }

    #[test]
    fn parses_current_homebridge_shape() {
        let services = parse_accessory_services(json!([{
            "uniqueId": "abc",
            "serviceName": "Lamp",
            "accessoryName": "Lamp",
            "serviceCharacteristics": [{
                "type": "On",
                "value": false,
                "format": "bool",
                "canRead": true,
                "canWrite": true
            }]
        }]))
        .unwrap();
        assert_eq!(services.len(), 1);
        assert!(services[0].service_characteristics[0].is_switch_compatible());
    }

    #[test]
    fn parses_legacy_characteristics_and_permissions_conservatively() {
        let services = parse_accessory_services(json!({
            "accessories": [{
                "id": "legacy",
                "name": "Legacy Lamp",
                "values": {"On": false, "Brightness": 40},
                "writableCharacteristics": ["On", "Brightness"]
            }]
        }))
        .unwrap();
        assert_eq!(services[0].service_characteristics.len(), 2);
        assert!(services[0].characteristic("On").unwrap().can_write);
    }

    #[test]
    fn legacy_values_without_write_metadata_remain_read_only() {
        let services = parse_accessory_services(json!([{
            "id": "legacy",
            "values": {"On": false}
        }]))
        .unwrap();
        assert!(!services[0].characteristic("On").unwrap().can_write);
    }
}
