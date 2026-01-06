use crate::storage::{self, CountryPlatformAuth, StaticProxy};
use crate::{AppState, SUPABASE_URL};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Response from the proxy-config API (new static proxy format)
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfigResponse {
    /// Static proxies (flat list)
    pub proxies: Vec<ApiStaticProxy>,
    /// Proxies grouped by country (for easier access)
    #[serde(default)]
    pub proxies_by_country: HashMap<String, Vec<ApiStaticProxy>>,
    /// All available countries
    pub countries: Vec<CountryInfo>,
    /// Countries that have proxies configured
    pub configured_countries: Vec<String>,
    /// Type of proxy system ('static')
    pub proxy_type: String,
}

/// Static proxy info from API
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApiStaticProxy {
    /// Unique ID from database
    pub id: String,
    pub country_code: String,
    pub country_name: String,
    pub flag_emoji: Option<String>,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub proxy_type: String,
    /// Priority for selection (higher = preferred)
    #[serde(default)]
    pub priority: i32,
    /// Weight for load balancing
    #[serde(default = "default_weight")]
    pub weight: i32,
}

fn default_weight() -> i32 {
    1
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CountryInfo {
    pub code: String,
    pub name: String,
    pub flag_emoji: Option<String>,
    pub region: Option<String>,
}

#[derive(Serialize)]
pub struct CountryAuthStatus {
    pub country_code: String,
    pub country_name: String,
    pub flag_emoji: Option<String>,
    pub platforms: HashMap<String, bool>, // platform -> is_authenticated
}

/// Fetch proxy configuration from the API (only works for paid users)
/// Returns list of countries that have proxies configured
#[tauri::command]
pub async fn fetch_proxy_config(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<CountryInfo>, String> {
    println!("[Proxy] fetch_proxy_config called");

    let access_token = {
        let auth = state.auth.lock();
        auth.access_token.clone()
    };

    let token = access_token.ok_or_else(|| {
        println!("[Proxy] Not authenticated");
        "Not authenticated".to_string()
    })?;

    println!("[Proxy] Fetching from API...");

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/functions/v1/proxy-config", SUPABASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status() == 403 {
        // User doesn't have a paid plan
        return Err("Geo-targeting requires a paid plan. Please upgrade to access this feature.".to_string());
    }

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, text));
    }

    let config_response: ProxyConfigResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    // Group proxies by country and convert to storage format
    let mut proxies_by_country: HashMap<String, Vec<StaticProxy>> = HashMap::new();

    for api_proxy in &config_response.proxies {
        let static_proxy = StaticProxy {
            id: Some(api_proxy.id.clone()),
            country_code: api_proxy.country_code.to_lowercase(),
            host: api_proxy.host.clone(),
            port: api_proxy.port,
            username: api_proxy.username.clone(),
            password: api_proxy.password.clone(),
            proxy_type: api_proxy.proxy_type.clone(),
            country_name: Some(api_proxy.country_name.clone()),
            added_at: chrono::Utc::now().timestamp(),
            priority: api_proxy.priority,
            weight: api_proxy.weight,
            local_usage_count: 0,
        };

        proxies_by_country
            .entry(api_proxy.country_code.to_lowercase())
            .or_insert_with(Vec::new)
            .push(static_proxy);
    }

    // Only replace proxies if we got some from the API
    // This prevents clearing existing proxies on API failures or empty responses
    if !proxies_by_country.is_empty() {
        if let Err(e) = storage::replace_all_static_proxies(proxies_by_country.clone()) {
            eprintln!("[Proxy] Failed to store proxies: {}", e);
        }
    } else {
        println!("[Proxy] API returned no proxies, keeping existing local proxies");
    }

    println!(
        "[Proxy] Fetched and stored {} static proxies across {} countries, {} total countries available",
        config_response.proxies.len(),
        proxies_by_country.len(),
        config_response.countries.len()
    );

    // Return countries that have proxies configured
    // Filter countries list to only those with configured proxies
    let configured_set: std::collections::HashSet<_> = config_response.configured_countries.iter().collect();
    let available_countries: Vec<CountryInfo> = config_response.countries
        .into_iter()
        .filter(|c| configured_set.contains(&c.code))
        .collect();

    Ok(available_countries)
}

/// Check if proxy config is available (user has paid plan and config is cached)
#[tauri::command]
pub async fn has_proxy_config() -> Result<bool, String> {
    Ok(storage::get_proxy_config().is_some())
}

/// Clear cached proxy config
#[tauri::command]
pub async fn clear_proxy_config() -> Result<(), String> {
    storage::clear_proxy_config()
}

/// Get all country/platform authentication statuses
#[tauri::command]
pub async fn get_country_auth_statuses(
    countries: Vec<CountryInfo>,
    platforms: Vec<String>,
) -> Result<Vec<CountryAuthStatus>, String> {
    // Use instance-scoped auth if available
    let instance_id = storage::get_active_instance_id();
    let all_auth = if instance_id.is_empty() {
        storage::get_all_country_platform_auth()
    } else {
        storage::get_instance_all_country_platform_auth(&instance_id)
    };

    let statuses: Vec<CountryAuthStatus> = countries
        .into_iter()
        .map(|country| {
            let mut platform_status = HashMap::new();
            for platform in &platforms {
                let key = format!("{}:{}", country.code.to_lowercase(), platform.to_lowercase());
                let is_auth = all_auth
                    .get(&key)
                    .map(|a| a.is_authenticated)
                    .unwrap_or(false);
                platform_status.insert(platform.clone(), is_auth);
            }
            CountryAuthStatus {
                country_code: country.code,
                country_name: country.name,
                flag_emoji: country.flag_emoji,
                platforms: platform_status,
            }
        })
        .collect();

    Ok(statuses)
}

/// Get authentication status for a specific country/platform (uses active instance)
#[tauri::command]
pub async fn get_country_platform_auth(
    country_code: String,
    platform: String,
) -> Result<Option<CountryPlatformAuth>, String> {
    let instance_id = storage::get_active_instance_id();
    if instance_id.is_empty() {
        // Fallback to legacy if no instance
        return Ok(storage::get_country_platform_auth(&country_code, &platform));
    }
    Ok(storage::get_instance_country_platform_auth(&instance_id, &country_code, &platform))
}

/// Update authentication status for a country/platform (uses active instance)
#[tauri::command]
pub async fn set_country_platform_auth(
    country_code: String,
    platform: String,
    is_authenticated: bool,
) -> Result<(), String> {
    let instance_id = storage::get_active_instance_id();
    if instance_id.is_empty() {
        // Fallback to legacy if no instance
        return storage::update_country_platform_auth(&country_code, &platform, is_authenticated);
    }
    storage::update_instance_country_platform_auth(&instance_id, &country_code, &platform, is_authenticated)
}

/// Update authentication status for a platform in a region (alias for set_country_platform_auth)
/// Uses camelCase parameter names to match JS caller
#[tauri::command(rename_all = "camelCase")]
pub async fn set_platform_auth_status(
    country_code: String,
    platform: String,
    authenticated: bool,
) -> Result<(), String> {
    let instance_id = storage::get_active_instance_id();
    if instance_id.is_empty() {
        return storage::update_country_platform_auth(&country_code, &platform, authenticated);
    }
    storage::update_instance_country_platform_auth(&instance_id, &country_code, &platform, authenticated)
}

/// Get all authenticated platforms for a country (uses active instance)
#[tauri::command]
pub async fn get_authenticated_platforms(country_code: String) -> Result<Vec<String>, String> {
    let instance_id = storage::get_active_instance_id();
    if instance_id.is_empty() {
        return Ok(storage::get_authenticated_platforms_for_country(&country_code));
    }
    Ok(storage::get_instance_authenticated_platforms_for_country(&instance_id, &country_code))
}

/// Get all authenticated countries for a platform (uses active instance)
#[tauri::command]
pub async fn get_authenticated_countries(platform: String) -> Result<Vec<String>, String> {
    let instance_id = storage::get_active_instance_id();
    if instance_id.is_empty() {
        return Ok(storage::get_authenticated_countries_for_platform(&platform));
    }
    Ok(storage::get_instance_authenticated_countries_for_platform(&instance_id, &platform))
}

/// Update product's scan countries
#[tauri::command]
pub async fn set_product_scan_countries(
    product_id: String,
    scan_countries: Vec<String>,
) -> Result<(), String> {
    let mut config = storage::get_product_config(&product_id);
    config.scan_countries = scan_countries;
    storage::update_product_config(&product_id, &config)
}

/// Get product's scan countries
#[tauri::command]
pub async fn get_product_scan_countries(product_id: String) -> Result<Vec<String>, String> {
    let config = storage::get_product_config(&product_id);
    Ok(config.scan_countries)
}

// ============== Static Proxy Management ==============

/// Response type for static proxy list
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaticProxyInfo {
    pub id: Option<String>,
    pub country_code: String,
    pub country_name: Option<String>,
    pub host: String,
    pub port: u16,
    pub has_auth: bool,
    pub proxy_type: String,
    pub added_at: i64,
    pub priority: i32,
    pub weight: i32,
    pub local_usage_count: u32,
}

impl From<StaticProxy> for StaticProxyInfo {
    fn from(p: StaticProxy) -> Self {
        StaticProxyInfo {
            id: p.id,
            country_code: p.country_code,
            country_name: p.country_name,
            host: p.host,
            port: p.port,
            has_auth: p.username.is_some() && p.password.is_some(),
            proxy_type: p.proxy_type,
            added_at: p.added_at,
            priority: p.priority,
            weight: p.weight,
            local_usage_count: p.local_usage_count,
        }
    }
}

/// Response type for proxies grouped by country
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CountryProxiesInfo {
    pub country_code: String,
    pub proxy_count: usize,
    pub proxies: Vec<StaticProxyInfo>,
}

/// Get all configured static proxies (flat list)
#[tauri::command]
pub fn get_static_proxies() -> Result<Vec<StaticProxyInfo>, String> {
    let proxies_by_country = storage::get_static_proxies();
    let mut result: Vec<StaticProxyInfo> = proxies_by_country
        .into_values()
        .flatten()
        .map(|p| p.into())
        .collect();
    result.sort_by(|a, b| a.country_code.cmp(&b.country_code));
    Ok(result)
}

/// Get all configured static proxies grouped by country
#[tauri::command]
pub fn get_static_proxies_by_country() -> Result<Vec<CountryProxiesInfo>, String> {
    let proxies_by_country = storage::get_static_proxies();
    let mut result: Vec<CountryProxiesInfo> = proxies_by_country
        .into_iter()
        .map(|(country_code, proxies)| {
            let proxy_infos: Vec<StaticProxyInfo> = proxies.into_iter().map(|p| p.into()).collect();
            CountryProxiesInfo {
                country_code,
                proxy_count: proxy_infos.len(),
                proxies: proxy_infos,
            }
        })
        .collect();
    result.sort_by(|a, b| a.country_code.cmp(&b.country_code));
    Ok(result)
}

/// Get static proxies for a specific country
#[tauri::command]
pub fn get_static_proxies_for_country(country_code: String) -> Result<Vec<StaticProxyInfo>, String> {
    let proxies = storage::get_static_proxies_for_country(&country_code);
    Ok(proxies.into_iter().map(|p| p.into()).collect())
}

/// Add or update a static proxy for a country
/// proxy_string can be in formats:
/// - host:port
/// - host:port:username:password
/// - username:password@host:port
/// - http://host:port
/// - http://username:password@host:port
#[tauri::command]
pub fn add_static_proxy(
    country_code: String,
    proxy_string: String,
    country_name: Option<String>,
) -> Result<StaticProxyInfo, String> {
    let proxy = storage::parse_proxy_string(&country_code, &proxy_string, country_name)?;
    storage::set_static_proxy(proxy.clone())?;
    println!("[Proxy] Added static proxy for {}: {}:{}",
             proxy.country_code, proxy.host, proxy.port);
    Ok(proxy.into())
}

/// Remove a static proxy for a country
#[tauri::command]
pub fn remove_static_proxy(country_code: String) -> Result<(), String> {
    storage::remove_static_proxy(&country_code)?;
    println!("[Proxy] Removed static proxy for {}", country_code);
    Ok(())
}

/// Test a static proxy connection
#[tauri::command]
pub async fn test_static_proxy(country_code: String) -> Result<String, String> {
    let proxy = storage::get_static_proxy(&country_code)
        .ok_or_else(|| format!("No proxy configured for {}", country_code))?;

    // Try to connect through the proxy to a test URL
    let proxy_url = if let (Some(user), Some(pass)) = (&proxy.username, &proxy.password) {
        format!("http://{}:{}@{}:{}", user, pass, proxy.host, proxy.port)
    } else {
        format!("http://{}:{}", proxy.host, proxy.port)
    };

    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(&proxy_url).map_err(|e| format!("Invalid proxy: {}", e))?)
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    // Use a simple IP check service
    let response = client
        .get("https://api.ipify.org?format=json")
        .send()
        .await
        .map_err(|e| format!("Proxy connection failed: {}", e))?;

    if response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        Ok(format!("Proxy working! Response: {}", text))
    } else {
        Err(format!("Proxy returned error: {}", response.status()))
    }
}

/// Check if a country has a static proxy configured
#[tauri::command]
pub fn has_static_proxy(country_code: String) -> bool {
    storage::get_static_proxy(&country_code).is_some()
}

/// Get list of countries that have static proxies configured
#[tauri::command]
pub fn get_configured_proxy_countries() -> Vec<String> {
    storage::get_static_proxies().keys().cloned().collect()
}
