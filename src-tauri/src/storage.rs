use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::AuthState;

/// Write to a debug log file for troubleshooting
fn debug_log(msg: &str) {
    let log_path = get_config_dir().join("debug.log");
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(file, "[{}] {}", timestamp, msg);
    }
}

/// Get the config directory path
fn get_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("columbus")
}

/// Persistent app data stored locally
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct PersistedState {
    // ============== SHARED ACROSS ALL INSTANCES ==============
    /// Columbus account auth (Supabase) - shared across all instances
    pub auth: Option<PersistedAuth>,
    /// Last selected product
    pub last_product_id: Option<String>,
    /// Per-product configurations keyed by product_id
    #[serde(default)]
    pub product_configs: HashMap<String, ProductConfig>,
    /// Cached proxy configuration (fetched from API for paid users) - DEPRECATED
    #[serde(default)]
    pub proxy_config: Option<ProxyConfig>,
    /// Static proxies per country code (e.g., "us" -> Vec<StaticProxy>)
    /// Supports multiple proxies per country for load balancing
    #[serde(default)]
    pub static_proxies: HashMap<String, Vec<StaticProxy>>,

    // ============== MULTI-INSTANCE SUPPORT ==============
    /// All instances (instance_id -> Instance)
    #[serde(default)]
    pub instances: HashMap<String, Instance>,
    /// Per-instance data (instance_id -> InstanceData)
    #[serde(default)]
    pub instance_data: HashMap<String, InstanceData>,
    /// Currently active instance ID
    #[serde(default)]
    pub active_instance_id: Option<String>,

    // ============== LEGACY FIELDS (for migration) ==============
    // These fields are deprecated and will be migrated to instance_data
    // on first launch. Kept for backward compatibility during migration.
    /// Country/platform authentication status (LEGACY - use instance_data)
    /// Key format: "{country_code}:{platform}" e.g., "us:chatgpt"
    #[serde(default)]
    pub country_platform_auth: HashMap<String, CountryPlatformAuth>,
    /// Platform credentials stored locally (LEGACY - use instance_data)
    /// Key format: platform name (e.g., "chatgpt", "perplexity")
    #[serde(default)]
    pub platform_credentials: HashMap<String, PlatformCredentials>,
    /// Timestamp of last successful auth (LEGACY - use instance_data)
    #[serde(default)]
    pub platforms_last_authenticated_on: Option<i64>,
    /// Hash of prompt regions config (LEGACY - use instance_data)
    #[serde(default)]
    pub platforms_last_authenticated_hash: Option<String>,
    /// Whether onboarding completed (LEGACY - use instance_data)
    #[serde(default)]
    pub onboarding_completed: bool,
}

/// Proxy configuration from the API - DEPRECATED (use StaticProxy instead)
#[derive(Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub provider: String,
    pub hostname: String,
    pub port_http: u16,
    pub port_socks5: u16,
    pub username: String,
    pub password: String,
    /// When the config was last fetched
    pub fetched_at: i64,
}

/// Static proxy configuration for a specific country
/// Supports various proxy formats: host:port, host:port:user:pass, etc.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct StaticProxy {
    /// Unique identifier (from API)
    #[serde(default)]
    pub id: Option<String>,
    /// Country code (lowercase, e.g., "us", "de", "gb")
    pub country_code: String,
    /// Proxy host/IP
    pub host: String,
    /// Proxy port
    pub port: u16,
    /// Optional username for auth
    pub username: Option<String>,
    /// Optional password for auth
    pub password: Option<String>,
    /// Proxy type: "http", "https", "socks5"
    #[serde(default = "default_proxy_type")]
    pub proxy_type: String,
    /// Display name for this country
    pub country_name: Option<String>,
    /// When this proxy was added
    pub added_at: i64,
    /// Priority for selection (higher = preferred)
    #[serde(default)]
    pub priority: i32,
    /// Weight for load balancing
    #[serde(default = "default_weight")]
    pub weight: i32,
    /// Local usage count for client-side load balancing
    #[serde(default)]
    pub local_usage_count: u32,
}

fn default_weight() -> i32 {
    1
}

fn default_proxy_type() -> String {
    "http".to_string()
}

/// Country information for geo-targeting
#[derive(Clone, Serialize, Deserialize)]
pub struct ProxyCountry {
    pub code: String,
    pub name: String,
    pub flag_emoji: Option<String>,
    pub region: Option<String>,
}

/// Authentication status for a country/platform combination
#[derive(Clone, Serialize, Deserialize)]
pub struct CountryPlatformAuth {
    pub country_code: String,
    pub platform: String,
    pub is_authenticated: bool,
    /// Timestamp of last verification
    pub last_verified: Option<i64>,
    /// Timestamp of last successful login
    pub last_login: Option<i64>,
}

/// Platform login credentials stored locally (plain text)
#[derive(Clone, Serialize, Deserialize)]
pub struct PlatformCredentials {
    pub platform: String,
    pub email: String,
    pub password: String,
    /// When credentials were last updated
    pub updated_at: Option<i64>,
}

// ============== Multi-Instance Support ==============

/// Instance metadata for multi-instance support
/// Each instance has separate platform credentials and browser sessions
#[derive(Clone, Serialize, Deserialize)]
pub struct Instance {
    /// Unique identifier (UUID)
    pub id: String,
    /// User-friendly name ("Default", "Instance 2", etc.)
    pub name: String,
    /// Creation timestamp
    pub created_at: i64,
    /// Whether this is the default instance (first one, cannot be deleted)
    pub is_default: bool,
}

/// Per-instance data containing credentials and auth status
/// This data is isolated per instance to allow different platform logins
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct InstanceData {
    /// Platform credentials for this instance
    #[serde(default)]
    pub platform_credentials: HashMap<String, PlatformCredentials>,
    /// Country/platform authentication status for this instance
    #[serde(default)]
    pub country_platform_auth: HashMap<String, CountryPlatformAuth>,
    /// Timestamp of last successful "Authenticate All Platforms" run
    #[serde(default)]
    pub platforms_last_authenticated_on: Option<i64>,
    /// Hash of prompt regions config at time of last authentication
    #[serde(default)]
    pub platforms_last_authenticated_hash: Option<String>,
    /// Whether onboarding (initial credential setup) has been completed
    #[serde(default)]
    pub onboarding_completed: bool,
}

/// Auth credentials to persist (tokens only, not sensitive user data)
#[derive(Clone, Serialize, Deserialize)]
pub struct PersistedAuth {
    pub access_token: String,
    pub refresh_token: String,
    pub user_id: String,
    pub user_email: String,
    pub expires_at: i64,
}

/// Per-product configuration
#[derive(Clone, Serialize, Deserialize)]
pub struct ProductConfig {
    /// Which platforms are enabled for this product
    pub ready_platforms: Vec<String>,
    /// Number of samples per prompt
    pub samples_per_prompt: u32,
    /// Whether auto-run is enabled for this product
    pub auto_run_enabled: bool,
    /// Target scans per day
    pub scans_per_day: u32,
    /// Start hour of the scan time window (0-23), default 9 (9 AM)
    #[serde(default = "default_start_hour")]
    pub time_window_start: u32,
    /// End hour of the scan time window (0-23), default 17 (5 PM)
    #[serde(default = "default_end_hour")]
    pub time_window_end: u32,
    /// Date of last auto scan (ISO format: YYYY-MM-DD)
    pub last_auto_scan_date: Option<String>,
    /// Number of scans completed today
    pub scans_today: u32,
    /// Scheduled scan times for today (hours in 24h format)
    #[serde(default)]
    pub scheduled_times: Vec<u32>,
    /// Countries to scan this product in (empty = user's actual location, no proxy)
    #[serde(default)]
    pub scan_countries: Vec<String>,
}

fn default_start_hour() -> u32 { 9 }
fn default_end_hour() -> u32 { 17 }

impl Default for ProductConfig {
    fn default() -> Self {
        Self {
            ready_platforms: Vec::new(),
            samples_per_prompt: 1,
            auto_run_enabled: true,
            scans_per_day: 1,
            time_window_start: 9,
            time_window_end: 17,
            last_auto_scan_date: None,
            scans_today: 0,
            scheduled_times: Vec::new(),
            scan_countries: Vec::new(),
        }
    }
}

/// Get the path to the config file
fn get_config_path() -> PathBuf {
    let config_dir = get_config_dir();

    // Ensure directory exists
    if !config_dir.exists() {
        debug_log(&format!("Creating config directory: {:?}", config_dir));
        match fs::create_dir_all(&config_dir) {
            Ok(_) => debug_log("Config directory created successfully"),
            Err(e) => debug_log(&format!("ERROR creating config directory: {}", e)),
        }
    }

    config_dir.join("state.json")
}

/// Load persisted state from disk
pub fn load_state() -> PersistedState {
    let path = get_config_path();
    debug_log(&format!("load_state: path = {:?}", path));

    if !path.exists() {
        debug_log("load_state: file does not exist, returning default");
        return PersistedState::default();
    }

    match fs::read_to_string(&path) {
        Ok(content) => {
            debug_log(&format!("load_state: read {} bytes", content.len()));
            serde_json::from_str(&content).unwrap_or_else(|e| {
                debug_log(&format!("load_state: parse error: {}", e));
                PersistedState::default()
            })
        }
        Err(e) => {
            debug_log(&format!("load_state: read error: {}", e));
            PersistedState::default()
        }
    }
}

/// Save persisted state to disk
pub fn save_state(state: &PersistedState) -> Result<(), String> {
    let path = get_config_path();
    debug_log(&format!("save_state: path = {:?}", path));

    let content = serde_json::to_string_pretty(state)
        .map_err(|e| format!("Failed to serialize state: {}", e))?;

    // Use explicit file operations with sync to ensure data is flushed to disk
    let mut file = fs::File::create(&path)
        .map_err(|e| {
            debug_log(&format!("save_state: create error: {}", e));
            format!("Failed to create state file: {}", e)
        })?;

    file.write_all(content.as_bytes())
        .map_err(|e| {
            debug_log(&format!("save_state: write_all error: {}", e));
            format!("Failed to write state file: {}", e)
        })?;

    // Explicitly sync to disk to ensure the write is complete
    file.sync_all()
        .map_err(|e| {
            debug_log(&format!("save_state: sync_all error: {}", e));
            format!("Failed to sync state file: {}", e)
        })?;

    debug_log(&format!("save_state: saved and synced {} bytes", content.len()));
    Ok(())
}

/// Convert AuthState to PersistedAuth for storage
impl From<&AuthState> for Option<PersistedAuth> {
    fn from(auth: &AuthState) -> Self {
        match (&auth.access_token, &auth.refresh_token, &auth.user) {
            (Some(access), Some(refresh), Some(user)) => Some(PersistedAuth {
                access_token: access.clone(),
                refresh_token: refresh.clone(),
                user_id: user.id.clone(),
                user_email: user.email.clone(),
                expires_at: auth.expires_at.unwrap_or(0),
            }),
            _ => None,
        }
    }
}

/// Convert PersistedAuth back to AuthState
impl From<&PersistedAuth> for AuthState {
    fn from(persisted: &PersistedAuth) -> Self {
        AuthState {
            access_token: Some(persisted.access_token.clone()),
            refresh_token: Some(persisted.refresh_token.clone()),
            user: Some(crate::User {
                id: persisted.user_id.clone(),
                email: persisted.user_email.clone(),
            }),
            expires_at: Some(persisted.expires_at),
        }
    }
}

// Helper functions to update specific parts of the state

/// Update auth in persisted state
pub fn update_auth(auth: Option<PersistedAuth>) -> Result<(), String> {
    debug_log(&format!("update_auth: auth present = {}", auth.is_some()));
    let mut state = load_state();
    state.auth = auth;
    save_state(&state)
}

/// Update last selected product
pub fn update_last_product(product_id: Option<String>) -> Result<(), String> {
    debug_log(&format!("update_last_product: {:?}", product_id));
    let mut state = load_state();
    state.last_product_id = product_id;
    save_state(&state)
}

/// Get config for a specific product
pub fn get_product_config(product_id: &str) -> ProductConfig {
    let state = load_state();
    state.product_configs.get(product_id).cloned().unwrap_or_default()
}

/// Update config for a specific product
pub fn update_product_config(product_id: &str, config: &ProductConfig) -> Result<(), String> {
    debug_log(&format!("update_product_config: product={}, platforms={:?}, auto_run={}, scans_per_day={}",
        product_id, config.ready_platforms, config.auto_run_enabled, config.scans_per_day));
    let mut state = load_state();
    state.product_configs.insert(product_id.to_string(), config.clone());
    save_state(&state)
}

/// Get all product configs (for auto-scan to iterate over)
pub fn get_all_product_configs() -> HashMap<String, ProductConfig> {
    load_state().product_configs
}

/// Clear auth (for logout)
pub fn clear_auth() -> Result<(), String> {
    let mut state = load_state();
    state.auth = None;
    save_state(&state)
}

/// Get current access token (for API calls)
pub fn get_access_token() -> Option<String> {
    let state = load_state();
    state.auth.map(|a| a.access_token)
}

// ============== Proxy Configuration ==============

/// Get cached proxy config
pub fn get_proxy_config() -> Option<ProxyConfig> {
    load_state().proxy_config
}

/// Save proxy config (fetched from API)
pub fn update_proxy_config(config: ProxyConfig) -> Result<(), String> {
    debug_log("update_proxy_config: saving proxy configuration");
    let mut state = load_state();
    state.proxy_config = Some(config);
    save_state(&state)
}

/// Clear proxy config (e.g., when user downgrades from paid plan)
pub fn clear_proxy_config() -> Result<(), String> {
    debug_log("clear_proxy_config: removing proxy configuration");
    let mut state = load_state();
    state.proxy_config = None;
    save_state(&state)
}

// ============== Static Proxy Management ==============

/// Get all configured static proxies (grouped by country)
pub fn get_static_proxies() -> HashMap<String, Vec<StaticProxy>> {
    load_state().static_proxies
}

/// Get all static proxies for a specific country
pub fn get_static_proxies_for_country(country_code: &str) -> Vec<StaticProxy> {
    load_state().static_proxies
        .get(&country_code.to_lowercase())
        .cloned()
        .unwrap_or_default()
}

/// Get static proxy for a specific country (best one based on priority and usage)
/// Uses weighted round-robin: selects highest priority, then lowest (usage_count/weight)
pub fn get_static_proxy(country_code: &str) -> Option<StaticProxy> {
    let state = load_state();
    let proxies = state.static_proxies.get(&country_code.to_lowercase())?;

    if proxies.is_empty() {
        return None;
    }

    // Sort by priority (desc), then by usage/weight ratio (asc)
    let mut sorted = proxies.clone();
    sorted.sort_by(|a, b| {
        // First by priority (higher is better)
        match b.priority.cmp(&a.priority) {
            std::cmp::Ordering::Equal => {
                // Then by usage/weight ratio (lower is better = less used)
                let ratio_a = a.local_usage_count as f64 / a.weight.max(1) as f64;
                let ratio_b = b.local_usage_count as f64 / b.weight.max(1) as f64;
                ratio_a.partial_cmp(&ratio_b).unwrap_or(std::cmp::Ordering::Equal)
            }
            other => other
        }
    });

    sorted.into_iter().next()
}

/// Add a static proxy for a country (appends to list)
pub fn add_static_proxy(proxy: StaticProxy) -> Result<(), String> {
    debug_log(&format!("add_static_proxy: adding proxy for {}", proxy.country_code));
    let mut state = load_state();
    let country_code = proxy.country_code.to_lowercase();

    state.static_proxies
        .entry(country_code)
        .or_insert_with(Vec::new)
        .push(proxy);

    save_state(&state)
}

/// Set all static proxies for a country (replaces existing)
pub fn set_static_proxies_for_country(country_code: &str, proxies: Vec<StaticProxy>) -> Result<(), String> {
    debug_log(&format!("set_static_proxies_for_country: setting {} proxies for {}", proxies.len(), country_code));
    let mut state = load_state();
    state.static_proxies.insert(country_code.to_lowercase(), proxies);
    save_state(&state)
}

/// Clear all static proxies and set new ones (for API refresh)
pub fn replace_all_static_proxies(proxies_by_country: HashMap<String, Vec<StaticProxy>>) -> Result<(), String> {
    debug_log(&format!("replace_all_static_proxies: replacing with {} countries", proxies_by_country.len()));
    let mut state = load_state();
    state.static_proxies = proxies_by_country;
    save_state(&state)
}

/// Increment usage count for a proxy (for local load balancing)
pub fn increment_proxy_usage(country_code: &str, proxy_id: Option<&str>) -> Result<(), String> {
    let mut state = load_state();

    if let Some(proxies) = state.static_proxies.get_mut(&country_code.to_lowercase()) {
        // Find the proxy by ID or host:port
        for proxy in proxies.iter_mut() {
            let matches = match proxy_id {
                Some(id) => proxy.id.as_deref() == Some(id),
                None => false, // If no ID, increment first proxy
            };

            if matches || proxy_id.is_none() {
                proxy.local_usage_count += 1;
                break;
            }
        }
    }

    save_state(&state)
}

/// Remove all static proxies for a country
pub fn remove_static_proxies_for_country(country_code: &str) -> Result<(), String> {
    debug_log(&format!("remove_static_proxies_for_country: removing all proxies for {}", country_code));
    let mut state = load_state();
    state.static_proxies.remove(&country_code.to_lowercase());
    save_state(&state)
}

/// Legacy: Add or update a single static proxy (backward compatibility)
pub fn set_static_proxy(proxy: StaticProxy) -> Result<(), String> {
    add_static_proxy(proxy)
}

/// Legacy: Remove proxies for a country
pub fn remove_static_proxy(country_code: &str) -> Result<(), String> {
    remove_static_proxies_for_country(country_code)
}

/// Parse a proxy string in various formats and create a StaticProxy
/// Supported formats:
/// - host:port
/// - host:port:username:password
/// - username:password@host:port
/// - http://host:port
/// - http://username:password@host:port
pub fn parse_proxy_string(country_code: &str, proxy_str: &str, country_name: Option<String>) -> Result<StaticProxy, String> {
    let proxy_str = proxy_str.trim();

    // Remove protocol prefix if present
    let (proxy_type, rest) = if proxy_str.starts_with("http://") {
        ("http".to_string(), &proxy_str[7..])
    } else if proxy_str.starts_with("https://") {
        ("https".to_string(), &proxy_str[8..])
    } else if proxy_str.starts_with("socks5://") {
        ("socks5".to_string(), &proxy_str[9..])
    } else {
        ("http".to_string(), proxy_str)
    };

    // Check for username:password@host:port format
    if let Some(at_pos) = rest.find('@') {
        let auth_part = &rest[..at_pos];
        let host_part = &rest[at_pos + 1..];

        let (username, password) = auth_part.split_once(':')
            .ok_or("Invalid auth format - expected username:password")?;

        let (host, port_str) = host_part.rsplit_once(':')
            .ok_or("Invalid host:port format")?;

        let port: u16 = port_str.parse()
            .map_err(|_| format!("Invalid port: {}", port_str))?;

        return Ok(StaticProxy {
            id: None,
            country_code: country_code.to_lowercase(),
            host: host.to_string(),
            port,
            username: Some(username.to_string()),
            password: Some(password.to_string()),
            proxy_type,
            country_name,
            added_at: chrono::Utc::now().timestamp(),
            priority: 0,
            weight: 1,
            local_usage_count: 0,
        });
    }

    // Parse host:port or host:port:username:password format
    let parts: Vec<&str> = rest.split(':').collect();

    match parts.len() {
        2 => {
            // host:port
            let port: u16 = parts[1].parse()
                .map_err(|_| format!("Invalid port: {}", parts[1]))?;
            Ok(StaticProxy {
                id: None,
                country_code: country_code.to_lowercase(),
                host: parts[0].to_string(),
                port,
                username: None,
                password: None,
                proxy_type,
                country_name,
                added_at: chrono::Utc::now().timestamp(),
                priority: 0,
                weight: 1,
                local_usage_count: 0,
            })
        }
        4 => {
            // host:port:username:password
            let port: u16 = parts[1].parse()
                .map_err(|_| format!("Invalid port: {}", parts[1]))?;
            Ok(StaticProxy {
                id: None,
                country_code: country_code.to_lowercase(),
                host: parts[0].to_string(),
                port,
                username: Some(parts[2].to_string()),
                password: Some(parts[3].to_string()),
                proxy_type,
                country_name,
                added_at: chrono::Utc::now().timestamp(),
                priority: 0,
                weight: 1,
                local_usage_count: 0,
            })
        }
        _ => Err(format!("Invalid proxy format: {}. Expected host:port or host:port:username:password", proxy_str))
    }
}

/// Build a proxy URL for a specific country (now uses static proxies)
pub fn build_proxy_url(_country_code: &str) -> Option<String> {
    // We no longer use this synchronously - use build_proxy_url_async instead
    None
}

/// Build a proxy URL asynchronously (starts local proxy server if needed)
pub async fn build_proxy_url_async(country_code: &str) -> Option<String> {
    // First check for static proxy
    if let Some(_static_proxy) = get_static_proxy(country_code) {
        // Use local proxy server to handle auth
        match crate::proxy_server::get_local_proxy_for_country(country_code).await {
            Ok(url) => {
                eprintln!("[Proxy] Using static proxy for country {}: {}", country_code, url);
                return Some(url);
            }
            Err(e) => {
                eprintln!("[Proxy] Failed to start local proxy for {}: {}", country_code, e);
                return None;
            }
        }
    }

    // Fallback to old IPRoyal config (deprecated)
    if get_proxy_config().is_some() {
        match crate::proxy_server::get_local_proxy_for_country(country_code).await {
            Ok(url) => {
                eprintln!("[Proxy] Using IPRoyal proxy for country {}: {}", country_code, url);
                return Some(url);
            }
            Err(e) => {
                eprintln!("[Proxy] Failed to get IPRoyal proxy for {}: {}", country_code, e);
            }
        }
    }

    None
}

/// Get proxy credentials for manual entry or display
pub fn get_proxy_credentials(country_code: &str) -> Option<(String, String)> {
    // First check static proxy
    if let Some(proxy) = get_static_proxy(country_code) {
        if let (Some(user), Some(pass)) = (proxy.username, proxy.password) {
            return Some((user, pass));
        }
    }

    // Fallback to old config
    let config = get_proxy_config()?;
    let password_with_country = format!("{}_country-{}", config.password, country_code.to_lowercase());
    Some((config.username.clone(), password_with_country))
}

// ============== Country/Platform Authentication ==============

/// Generate key for country/platform auth map
fn country_platform_key(country_code: &str, platform: &str) -> String {
    format!("{}:{}", country_code.to_lowercase(), platform.to_lowercase())
}

/// Get authentication status for a country/platform combination
pub fn get_country_platform_auth(country_code: &str, platform: &str) -> Option<CountryPlatformAuth> {
    let state = load_state();
    let key = country_platform_key(country_code, platform);
    state.country_platform_auth.get(&key).cloned()
}

/// Check if a country/platform combination is authenticated
pub fn is_country_platform_authenticated(country_code: &str, platform: &str) -> bool {
    get_country_platform_auth(country_code, platform)
        .map(|auth| auth.is_authenticated)
        .unwrap_or(false)
}

/// Update authentication status for a country/platform combination
pub fn update_country_platform_auth(
    country_code: &str,
    platform: &str,
    is_authenticated: bool,
) -> Result<(), String> {
    debug_log(&format!(
        "update_country_platform_auth: {}:{} = {}",
        country_code, platform, is_authenticated
    ));
    let mut state = load_state();
    let key = country_platform_key(country_code, platform);
    let now = chrono::Utc::now().timestamp();

    let auth = CountryPlatformAuth {
        country_code: country_code.to_lowercase(),
        platform: platform.to_lowercase(),
        is_authenticated,
        last_verified: Some(now),
        last_login: if is_authenticated { Some(now) } else {
            // Preserve last_login if just marking as not authenticated
            state.country_platform_auth.get(&key).and_then(|a| a.last_login)
        },
    };
    state.country_platform_auth.insert(key, auth);
    save_state(&state)
}

/// Get all country/platform auth statuses
pub fn get_all_country_platform_auth() -> HashMap<String, CountryPlatformAuth> {
    load_state().country_platform_auth
}

/// Get all authenticated platforms for a specific country
pub fn get_authenticated_platforms_for_country(country_code: &str) -> Vec<String> {
    let state = load_state();
    let prefix = format!("{}:", country_code.to_lowercase());
    state.country_platform_auth
        .iter()
        .filter(|(key, auth)| key.starts_with(&prefix) && auth.is_authenticated)
        .map(|(_, auth)| auth.platform.clone())
        .collect()
}

/// Get all authenticated countries for a specific platform
pub fn get_authenticated_countries_for_platform(platform: &str) -> Vec<String> {
    let state = load_state();
    let suffix = format!(":{}", platform.to_lowercase());
    state.country_platform_auth
        .iter()
        .filter(|(key, auth)| key.ends_with(&suffix) && auth.is_authenticated)
        .map(|(_, auth)| auth.country_code.clone())
        .collect()
}

/// Clear all country/platform auth (e.g., on logout)
pub fn clear_all_country_platform_auth() -> Result<(), String> {
    debug_log("clear_all_country_platform_auth: clearing all auth statuses");
    let mut state = load_state();
    state.country_platform_auth.clear();
    save_state(&state)
}

// ============== Webview Data Directory ==============

/// Get the data directory for a specific country/platform webview session
/// This provides cookie isolation per country/platform combination
pub fn get_webview_data_dir(country_code: &str, platform: &str) -> PathBuf {
    get_config_dir()
        .join("webview-data")
        .join(country_code.to_lowercase())
        .join(platform.to_lowercase())
}

/// Get the data directory for user's actual location (no proxy)
pub fn get_webview_data_dir_local(platform: &str) -> PathBuf {
    get_config_dir()
        .join("webview-data")
        .join("local")
        .join(platform.to_lowercase())
}

/// Ensure webview data directory exists
pub fn ensure_webview_data_dir(country_code: &str, platform: &str) -> Result<PathBuf, String> {
    let dir = get_webview_data_dir(country_code, platform);
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create webview data dir: {}", e))?;
    }
    Ok(dir)
}

/// Ensure webview data directory exists for local (no proxy)
pub fn ensure_webview_data_dir_local(platform: &str) -> Result<PathBuf, String> {
    let dir = get_webview_data_dir_local(platform);
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create webview data dir: {}", e))?;
    }
    Ok(dir)
}

// ============== Platform Credentials ==============

/// Get credentials for a specific platform
pub fn get_platform_credentials(platform: &str) -> Option<PlatformCredentials> {
    let state = load_state();
    state.platform_credentials.get(&platform.to_lowercase()).cloned()
}

/// Get all platform credentials
pub fn get_all_platform_credentials() -> HashMap<String, PlatformCredentials> {
    load_state().platform_credentials
}

/// Save or update credentials for a platform
pub fn update_platform_credentials(
    platform: &str,
    email: &str,
    password: &str,
) -> Result<(), String> {
    debug_log(&format!(
        "update_platform_credentials: platform={}, email={}",
        platform, email
    ));
    let mut state = load_state();
    let now = chrono::Utc::now().timestamp();

    let creds = PlatformCredentials {
        platform: platform.to_lowercase(),
        email: email.to_string(),
        password: password.to_string(),
        updated_at: Some(now),
    };
    state.platform_credentials.insert(platform.to_lowercase(), creds);
    save_state(&state)
}

/// Remove credentials for a platform
pub fn remove_platform_credentials(platform: &str) -> Result<(), String> {
    debug_log(&format!("remove_platform_credentials: {}", platform));
    let mut state = load_state();
    state.platform_credentials.remove(&platform.to_lowercase());
    save_state(&state)
}

/// Check if any platform credentials are stored
pub fn has_platform_credentials() -> bool {
    !load_state().platform_credentials.is_empty()
}

/// Get list of platforms that have credentials stored
pub fn get_platforms_with_credentials() -> Vec<String> {
    load_state()
        .platform_credentials
        .keys()
        .cloned()
        .collect()
}

// ============== Onboarding & Authentication Tracking ==============

/// Check if onboarding has been completed
pub fn is_onboarding_completed() -> bool {
    load_state().onboarding_completed
}

/// Mark onboarding as completed
pub fn set_onboarding_completed(completed: bool) -> Result<(), String> {
    debug_log(&format!("set_onboarding_completed: {}", completed));
    let mut state = load_state();
    state.onboarding_completed = completed;
    save_state(&state)
}

/// Get the last authentication timestamp
pub fn get_platforms_last_authenticated_on() -> Option<i64> {
    load_state().platforms_last_authenticated_on
}

/// Get the last authentication hash
pub fn get_platforms_last_authenticated_hash() -> Option<String> {
    load_state().platforms_last_authenticated_hash
}

/// Update authentication tracking after successful bulk authentication
pub fn update_platforms_authentication(hash: &str) -> Result<(), String> {
    debug_log(&format!("update_platforms_authentication: hash={}", hash));
    let mut state = load_state();
    let now = chrono::Utc::now().timestamp();
    state.platforms_last_authenticated_on = Some(now);
    state.platforms_last_authenticated_hash = Some(hash.to_string());
    save_state(&state)
}

/// Check if authentication is stale (older than 30 days)
pub fn is_authentication_stale() -> bool {
    const THIRTY_DAYS_SECS: i64 = 30 * 24 * 60 * 60;
    match load_state().platforms_last_authenticated_on {
        Some(timestamp) => {
            let now = chrono::Utc::now().timestamp();
            now - timestamp > THIRTY_DAYS_SECS
        }
        None => true, // No authentication recorded = stale
    }
}

/// Check if authentication hash matches the given hash
pub fn does_authentication_hash_match(current_hash: &str) -> bool {
    match load_state().platforms_last_authenticated_hash {
        Some(stored_hash) => stored_hash == current_hash,
        None => false,
    }
}

/// Compute hash of prompt regions configuration
/// Input: HashMap<prompt_id, Vec<region_code>>
pub fn compute_prompt_regions_hash(prompt_regions: &HashMap<String, Vec<String>>) -> String {
    use std::collections::BTreeMap;

    // Sort for consistent hashing
    let sorted: BTreeMap<&String, &Vec<String>> = prompt_regions.iter().collect();
    let json = serde_json::to_string(&sorted).unwrap_or_default();

    // Simple hash using std (no external crate needed)
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    let mut hasher = DefaultHasher::new();
    json.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Clear all authentication tracking (e.g., on logout)
pub fn clear_authentication_tracking() -> Result<(), String> {
    debug_log("clear_authentication_tracking");
    let mut state = load_state();
    state.platforms_last_authenticated_on = None;
    state.platforms_last_authenticated_hash = None;
    save_state(&state)
}

// ============== OpenAI API Key Storage ==============

const KEYRING_SERVICE: &str = "columbus-desktop";
const OPENAI_KEY_NAME: &str = "openai-api-key";

// Google platforms that share authentication state
const GOOGLE_PLATFORMS: &[&str] = &["gemini", "google_aio", "google_ai_mode"];

/// Check if a platform is a Google platform (shares auth with other Google platforms)
fn is_google_platform(platform: &str) -> bool {
    GOOGLE_PLATFORMS.contains(&platform.to_lowercase().as_str())
}

/// Get OpenAI API key from OS keychain
pub fn get_openai_api_key() -> Option<String> {
    match keyring::Entry::new(KEYRING_SERVICE, OPENAI_KEY_NAME) {
        Ok(entry) => match entry.get_password() {
            Ok(key) => Some(key),
            Err(e) => {
                debug_log(&format!("Failed to get OpenAI API key from keychain: {}", e));
                None
            }
        },
        Err(e) => {
            debug_log(&format!("Failed to create keyring entry: {}", e));
            None
        }
    }
}

/// Save OpenAI API key to OS keychain
pub fn set_openai_api_key(api_key: &str) -> Result<(), String> {
    debug_log("set_openai_api_key: storing key in keychain");
    let entry = keyring::Entry::new(KEYRING_SERVICE, OPENAI_KEY_NAME)
        .map_err(|e| format!("Failed to create keyring entry: {}", e))?;

    entry
        .set_password(api_key)
        .map_err(|e| format!("Failed to store OpenAI API key: {}", e))?;

    Ok(())
}

/// Remove OpenAI API key from OS keychain
pub fn remove_openai_api_key() -> Result<(), String> {
    debug_log("remove_openai_api_key: removing key from keychain");
    let entry = keyring::Entry::new(KEYRING_SERVICE, OPENAI_KEY_NAME)
        .map_err(|e| format!("Failed to create keyring entry: {}", e))?;

    // Ignore error if key doesn't exist
    let _ = entry.delete_credential();
    Ok(())
}

/// Check if OpenAI API key is configured
pub fn has_openai_api_key() -> bool {
    get_openai_api_key().is_some() || std::env::var("OPENAI_API_KEY").is_ok()
}

// ============== Secure Credential Storage (using OS keychain) ==============

/// Save platform credentials with password stored securely in OS keychain
pub fn save_platform_credentials_secure(
    platform: &str,
    email: &str,
    password: &str,
) -> Result<(), String> {
    let platform_lower = platform.to_lowercase();
    debug_log(&format!(
        "save_platform_credentials_secure: START platform={}, email={}",
        platform_lower, email
    ));

    // Store password in OS keychain using credential builder for explicit target
    let keyring_key = format!("{}:{}", platform_lower, email);
    debug_log(&format!("save_platform_credentials_secure: keyring_key={}", keyring_key));

    // Try using the new credential builder API
    let entry = keyring::Entry::new_with_target(&keyring_key, KEYRING_SERVICE, &keyring_key)
        .map_err(|e| {
            debug_log(&format!("save_platform_credentials_secure: keyring entry creation failed: {}", e));
            format!("Failed to create keyring entry: {}", e)
        })?;

    debug_log(&format!("save_platform_credentials_secure: entry created, setting password..."));

    entry
        .set_password(password)
        .map_err(|e| {
            debug_log(&format!("save_platform_credentials_secure: keyring set_password failed: {}", e));
            format!("Failed to store password in keychain: {}", e)
        })?;

    debug_log("save_platform_credentials_secure: password stored in keychain");

    // Verify the password was actually stored
    match entry.get_password() {
        Ok(_) => debug_log("save_platform_credentials_secure: verified password can be read back"),
        Err(e) => debug_log(&format!("save_platform_credentials_secure: WARNING - cannot read back password: {}", e)),
    }

    // Store email and metadata in regular storage (password is in keychain)
    let mut state = load_state();
    let now = chrono::Utc::now().timestamp();

    let creds = PlatformCredentials {
        platform: platform_lower.clone(),
        email: email.to_string(),
        password: String::new(), // Don't store password in file
        updated_at: Some(now),
    };

    debug_log(&format!("save_platform_credentials_secure: inserting into state.platform_credentials with key={}", platform_lower));
    state.platform_credentials.insert(platform_lower.clone(), creds);

    debug_log(&format!("save_platform_credentials_secure: state now has {} platform_credentials: {:?}",
        state.platform_credentials.len(),
        state.platform_credentials.keys().collect::<Vec<_>>()));

    save_state(&state)?;

    debug_log("save_platform_credentials_secure: END - success");

    Ok(())
}

/// Get platform credentials with password from OS keychain
pub fn get_platform_credentials_secure(platform: &str) -> Option<(String, String)> {
    let platform_lower = platform.to_lowercase();
    debug_log(&format!("get_platform_credentials_secure: START looking for platform={}", platform_lower));

    let config_path = get_config_path();
    debug_log(&format!("get_platform_credentials_secure: config_path={:?}, exists={}", config_path, config_path.exists()));

    let state = load_state();
    debug_log(&format!("get_platform_credentials_secure: loaded state, platform_credentials count={}",
        state.platform_credentials.len()));
    debug_log(&format!("get_platform_credentials_secure: stored platforms={:?}",
        state.platform_credentials.keys().collect::<Vec<_>>()));

    let creds = match state.platform_credentials.get(&platform_lower) {
        Some(c) => c,
        None => {
            debug_log(&format!("get_platform_credentials_secure: NO CREDENTIALS FOUND for platform={}", platform_lower));
            return None;
        }
    };

    let email = &creds.email;
    debug_log(&format!("get_platform_credentials_secure: found email={}", email));

    // Get password from OS keychain using same target format as save
    let keyring_key = format!("{}:{}", platform_lower, email);
    debug_log(&format!("get_platform_credentials_secure: keyring_key={}", keyring_key));

    let entry = match keyring::Entry::new_with_target(&keyring_key, KEYRING_SERVICE, &keyring_key) {
        Ok(e) => e,
        Err(e) => {
            debug_log(&format!("get_platform_credentials_secure: keyring entry error: {}", e));
            return None;
        }
    };

    let password = match entry.get_password() {
        Ok(p) => p,
        Err(e) => {
            debug_log(&format!("get_platform_credentials_secure: keyring get_password error: {}", e));
            return None;
        }
    };

    debug_log("get_platform_credentials_secure: END - successfully retrieved credentials");
    Some((email.clone(), password))
}

/// Remove platform credentials including password from keychain
pub fn remove_platform_credentials_secure(platform: &str) -> Result<(), String> {
    let platform_lower = platform.to_lowercase();
    debug_log(&format!("remove_platform_credentials_secure: {}", platform_lower));

    let state = load_state();

    // Remove password from keychain if we have the email
    if let Some(creds) = state.platform_credentials.get(&platform_lower) {
        let keyring_key = format!("{}:{}", platform_lower, creds.email);
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, &keyring_key) {
            let _ = entry.delete_credential();
        }
    }

    // Remove from regular storage
    remove_platform_credentials(&platform_lower)
}

// ============== Multi-Instance Management ==============

/// Get all instances sorted by creation time (default first)
pub fn get_all_instances() -> Vec<Instance> {
    let state = load_state();
    let mut instances: Vec<Instance> = state.instances.values().cloned().collect();
    // Sort by creation time, default first
    instances.sort_by(|a, b| {
        if a.is_default != b.is_default {
            b.is_default.cmp(&a.is_default) // Default first
        } else {
            a.created_at.cmp(&b.created_at)
        }
    });
    instances
}

/// Get instance by ID
pub fn get_instance(instance_id: &str) -> Option<Instance> {
    let state = load_state();
    state.instances.get(instance_id).cloned()
}

/// Get the active instance ID, creating default if needed
pub fn get_active_instance_id() -> String {
    let state = load_state();

    // If we have an active instance, return it
    if let Some(ref id) = state.active_instance_id {
        if state.instances.contains_key(id) {
            return id.clone();
        }
    }

    // Find the default instance
    if let Some(instance) = state.instances.values().find(|i| i.is_default) {
        return instance.id.clone();
    }

    // No instances exist - this shouldn't happen after migration
    // Return empty string as fallback
    String::new()
}

/// Get the active instance
pub fn get_active_instance() -> Option<Instance> {
    let id = get_active_instance_id();
    if id.is_empty() {
        return None;
    }
    get_instance(&id)
}

/// Set the active instance ID
pub fn set_active_instance_id(instance_id: &str) -> Result<(), String> {
    debug_log(&format!("set_active_instance_id: {}", instance_id));
    let mut state = load_state();

    // Verify instance exists
    if !state.instances.contains_key(instance_id) {
        return Err(format!("Instance {} not found", instance_id));
    }

    state.active_instance_id = Some(instance_id.to_string());
    save_state(&state)
}

/// Generate the next instance name ("Instance 2", "Instance 3", etc.)
fn generate_next_instance_name(state: &PersistedState) -> String {
    let mut max_num = 1;

    for instance in state.instances.values() {
        // Try to parse "Instance N" format
        if instance.name.starts_with("Instance ") {
            if let Ok(num) = instance.name[9..].parse::<u32>() {
                if num > max_num {
                    max_num = num;
                }
            }
        }
    }

    format!("Instance {}", max_num + 1)
}

/// Create a new instance
pub fn create_instance(name: Option<String>) -> Result<Instance, String> {
    let mut state = load_state();

    let instance_name = name.unwrap_or_else(|| generate_next_instance_name(&state));
    let instance_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();

    debug_log(&format!("create_instance: id={}, name={}", instance_id, instance_name));

    let instance = Instance {
        id: instance_id.clone(),
        name: instance_name,
        created_at: now,
        is_default: false,
    };

    // Create empty instance data
    let instance_data = InstanceData::default();

    state.instances.insert(instance_id.clone(), instance.clone());
    state.instance_data.insert(instance_id.clone(), instance_data);

    save_state(&state)?;

    Ok(instance)
}

/// Delete an instance (cannot delete the default instance)
pub fn delete_instance(instance_id: &str) -> Result<(), String> {
    debug_log(&format!("delete_instance: {}", instance_id));
    let mut state = load_state();

    // Check if instance exists
    let instance = state.instances.get(instance_id)
        .ok_or_else(|| format!("Instance {} not found", instance_id))?;

    // Cannot delete default instance
    if instance.is_default {
        return Err("Cannot delete the default instance".to_string());
    }

    // Cannot delete if it's the only instance
    if state.instances.len() <= 1 {
        return Err("Cannot delete the last instance".to_string());
    }

    // Get instance data for cleanup
    if let Some(data) = state.instance_data.get(instance_id) {
        // Delete keyring entries for this instance
        for (platform, creds) in &data.platform_credentials {
            let keyring_key = format!("instance:{}:{}:{}", instance_id, platform, creds.email);
            if let Ok(entry) = keyring::Entry::new_with_target(&keyring_key, KEYRING_SERVICE, &keyring_key) {
                let _ = entry.delete_credential();
            }
        }
    }

    // Delete webview data directory for this instance
    let webview_dir = get_config_dir().join("webview-data").join(instance_id);
    if webview_dir.exists() {
        let _ = fs::remove_dir_all(&webview_dir);
    }

    // Remove from state
    state.instances.remove(instance_id);
    state.instance_data.remove(instance_id);

    // If this was the active instance, switch to default
    if state.active_instance_id.as_deref() == Some(instance_id) {
        state.active_instance_id = state.instances.values()
            .find(|i| i.is_default)
            .map(|i| i.id.clone());
    }

    save_state(&state)
}

/// Rename an instance
pub fn rename_instance(instance_id: &str, new_name: &str) -> Result<(), String> {
    debug_log(&format!("rename_instance: {} -> {}", instance_id, new_name));
    let mut state = load_state();

    let instance = state.instances.get_mut(instance_id)
        .ok_or_else(|| format!("Instance {} not found", instance_id))?;

    instance.name = new_name.to_string();

    save_state(&state)
}

// ============== Instance-Scoped Credential Functions ==============

/// Save platform credentials for a specific instance
pub fn save_instance_credentials_secure(
    instance_id: &str,
    platform: &str,
    email: &str,
    password: &str,
) -> Result<(), String> {
    let platform_lower = platform.to_lowercase();
    debug_log(&format!(
        "save_instance_credentials_secure: instance={}, platform={}, email={}",
        instance_id, platform_lower, email
    ));

    // Store password in OS keychain with instance prefix
    let keyring_key = format!("instance:{}:{}:{}", instance_id, platform_lower, email);
    debug_log(&format!("save_instance_credentials_secure: keyring_key={}", keyring_key));

    let entry = keyring::Entry::new_with_target(&keyring_key, KEYRING_SERVICE, &keyring_key)
        .map_err(|e| format!("Failed to create keyring entry: {}", e))?;

    entry.set_password(password)
        .map_err(|e| format!("Failed to store password in keychain: {}", e))?;

    debug_log("save_instance_credentials_secure: password stored in keychain");

    // Update instance data
    let mut state = load_state();
    let now = chrono::Utc::now().timestamp();

    let instance_data = state.instance_data
        .entry(instance_id.to_string())
        .or_insert_with(InstanceData::default);

    let creds = PlatformCredentials {
        platform: platform_lower.clone(),
        email: email.to_string(),
        password: String::new(), // Don't store password in file
        updated_at: Some(now),
    };

    instance_data.platform_credentials.insert(platform_lower, creds);
    save_state(&state)?;

    debug_log("save_instance_credentials_secure: END - success");
    Ok(())
}

/// Get platform credentials for a specific instance
pub fn get_instance_credentials_secure(
    instance_id: &str,
    platform: &str,
) -> Option<(String, String)> {
    let platform_lower = platform.to_lowercase();
    debug_log(&format!(
        "get_instance_credentials_secure: instance={}, platform={}",
        instance_id, platform_lower
    ));

    let state = load_state();
    let instance_data = state.instance_data.get(instance_id)?;
    let creds = instance_data.platform_credentials.get(&platform_lower)?;

    let email = &creds.email;
    let keyring_key = format!("instance:{}:{}:{}", instance_id, platform_lower, email);

    let entry = keyring::Entry::new_with_target(&keyring_key, KEYRING_SERVICE, &keyring_key).ok()?;
    let password = entry.get_password().ok()?;

    debug_log("get_instance_credentials_secure: END - success");
    Some((email.clone(), password))
}

/// Remove platform credentials for a specific instance
pub fn remove_instance_credentials_secure(
    instance_id: &str,
    platform: &str,
) -> Result<(), String> {
    let platform_lower = platform.to_lowercase();
    debug_log(&format!(
        "remove_instance_credentials_secure: instance={}, platform={}",
        instance_id, platform_lower
    ));

    let mut state = load_state();

    // Remove password from keychain
    if let Some(instance_data) = state.instance_data.get(instance_id) {
        if let Some(creds) = instance_data.platform_credentials.get(&platform_lower) {
            let keyring_key = format!("instance:{}:{}:{}", instance_id, platform_lower, creds.email);
            if let Ok(entry) = keyring::Entry::new_with_target(&keyring_key, KEYRING_SERVICE, &keyring_key) {
                let _ = entry.delete_credential();
            }
        }
    }

    // Remove from instance data
    if let Some(instance_data) = state.instance_data.get_mut(instance_id) {
        instance_data.platform_credentials.remove(&platform_lower);
    }

    save_state(&state)
}

/// Get all platform credentials for an instance (metadata only, no passwords)
pub fn get_instance_all_credentials(instance_id: &str) -> HashMap<String, PlatformCredentials> {
    let state = load_state();
    state.instance_data
        .get(instance_id)
        .map(|d| d.platform_credentials.clone())
        .unwrap_or_default()
}

/// Get list of platforms with credentials for an instance
pub fn get_instance_platforms_with_credentials(instance_id: &str) -> Vec<String> {
    let state = load_state();
    state.instance_data
        .get(instance_id)
        .map(|d| d.platform_credentials.keys().cloned().collect())
        .unwrap_or_default()
}

// ============== Instance-Scoped Auth Status Functions ==============

/// Get country/platform auth status for a specific instance
pub fn get_instance_country_platform_auth(
    instance_id: &str,
    country_code: &str,
    platform: &str,
) -> Option<CountryPlatformAuth> {
    let state = load_state();
    let instance_data = state.instance_data.get(instance_id)?;
    let key = country_platform_key(country_code, platform);
    instance_data.country_platform_auth.get(&key).cloned()
}

/// Check if country/platform is authenticated for a specific instance
pub fn is_instance_country_platform_authenticated(
    instance_id: &str,
    country_code: &str,
    platform: &str,
) -> bool {
    get_instance_country_platform_auth(instance_id, country_code, platform)
        .map(|auth| auth.is_authenticated)
        .unwrap_or(false)
}

/// Update country/platform auth status for a specific instance
pub fn update_instance_country_platform_auth(
    instance_id: &str,
    country_code: &str,
    platform: &str,
    is_authenticated: bool,
) -> Result<(), String> {
    debug_log(&format!(
        "update_instance_country_platform_auth: instance={}, {}:{} = {}",
        instance_id, country_code, platform, is_authenticated
    ));

    let mut state = load_state();
    let now = chrono::Utc::now().timestamp();

    let instance_data = state.instance_data
        .entry(instance_id.to_string())
        .or_insert_with(InstanceData::default);

    // Determine which platforms to update (sync Google platforms)
    let platforms_to_update: Vec<&str> = if is_google_platform(platform) {
        GOOGLE_PLATFORMS.to_vec()
    } else {
        vec![platform]
    };

    for plat in platforms_to_update {
        let key = country_platform_key(country_code, plat);
        let auth = CountryPlatformAuth {
            country_code: country_code.to_lowercase(),
            platform: plat.to_lowercase(),
            is_authenticated,
            last_verified: Some(now),
            last_login: if is_authenticated { Some(now) } else {
                instance_data.country_platform_auth.get(&key).and_then(|a| a.last_login)
            },
        };
        instance_data.country_platform_auth.insert(key, auth);
    }

    save_state(&state)
}

/// Get all country/platform auth for a specific instance
pub fn get_instance_all_country_platform_auth(
    instance_id: &str,
) -> HashMap<String, CountryPlatformAuth> {
    let state = load_state();
    state.instance_data
        .get(instance_id)
        .map(|d| d.country_platform_auth.clone())
        .unwrap_or_default()
}

/// Get all authenticated platforms for a country in a specific instance
pub fn get_instance_authenticated_platforms_for_country(
    instance_id: &str,
    country_code: &str,
) -> Vec<String> {
    let state = load_state();
    let prefix = format!("{}:", country_code.to_lowercase());

    state.instance_data
        .get(instance_id)
        .map(|d| {
            d.country_platform_auth
                .iter()
                .filter(|(key, auth)| key.starts_with(&prefix) && auth.is_authenticated)
                .map(|(_, auth)| auth.platform.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Get all authenticated countries for a platform in a specific instance
pub fn get_instance_authenticated_countries_for_platform(
    instance_id: &str,
    platform: &str,
) -> Vec<String> {
    let state = load_state();
    let suffix = format!(":{}", platform.to_lowercase());

    state.instance_data
        .get(instance_id)
        .map(|d| {
            d.country_platform_auth
                .iter()
                .filter(|(key, auth)| key.ends_with(&suffix) && auth.is_authenticated)
                .map(|(_, auth)| auth.country_code.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Clear all country/platform auth for a specific instance
pub fn clear_instance_country_platform_auth(instance_id: &str) -> Result<(), String> {
    debug_log(&format!("clear_instance_country_platform_auth: {}", instance_id));
    let mut state = load_state();

    if let Some(instance_data) = state.instance_data.get_mut(instance_id) {
        instance_data.country_platform_auth.clear();
    }

    save_state(&state)
}

// ============== Instance-Scoped Onboarding & Auth Tracking ==============

/// Check if onboarding completed for a specific instance
pub fn is_instance_onboarding_completed(instance_id: &str) -> bool {
    let state = load_state();
    state.instance_data
        .get(instance_id)
        .map(|d| d.onboarding_completed)
        .unwrap_or(false)
}

/// Set onboarding completed for a specific instance
pub fn set_instance_onboarding_completed(instance_id: &str, completed: bool) -> Result<(), String> {
    debug_log(&format!("set_instance_onboarding_completed: instance={}, completed={}", instance_id, completed));
    let mut state = load_state();

    let instance_data = state.instance_data
        .entry(instance_id.to_string())
        .or_insert_with(InstanceData::default);

    instance_data.onboarding_completed = completed;
    save_state(&state)
}

/// Get last authentication timestamp for a specific instance
pub fn get_instance_platforms_last_authenticated_on(instance_id: &str) -> Option<i64> {
    let state = load_state();
    state.instance_data
        .get(instance_id)
        .and_then(|d| d.platforms_last_authenticated_on)
}

/// Update authentication tracking for a specific instance
pub fn update_instance_platforms_authentication(instance_id: &str, hash: &str) -> Result<(), String> {
    debug_log(&format!("update_instance_platforms_authentication: instance={}, hash={}", instance_id, hash));
    let mut state = load_state();
    let now = chrono::Utc::now().timestamp();

    let instance_data = state.instance_data
        .entry(instance_id.to_string())
        .or_insert_with(InstanceData::default);

    instance_data.platforms_last_authenticated_on = Some(now);
    instance_data.platforms_last_authenticated_hash = Some(hash.to_string());

    save_state(&state)
}

// ============== Instance-Scoped Webview Data Paths ==============

/// Get webview data directory for a specific instance/country/platform
pub fn get_instance_webview_data_dir(instance_id: &str, country_code: &str, platform: &str) -> PathBuf {
    get_config_dir()
        .join("webview-data")
        .join(instance_id)
        .join(country_code.to_lowercase())
        .join(platform.to_lowercase())
}

/// Get webview data directory for local (no proxy) for a specific instance
pub fn get_instance_webview_data_dir_local(instance_id: &str, platform: &str) -> PathBuf {
    get_config_dir()
        .join("webview-data")
        .join(instance_id)
        .join("local")
        .join(platform.to_lowercase())
}

/// Ensure webview data directory exists for a specific instance
pub fn ensure_instance_webview_data_dir(
    instance_id: &str,
    country_code: &str,
    platform: &str,
) -> Result<PathBuf, String> {
    let dir = get_instance_webview_data_dir(instance_id, country_code, platform);
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create webview data dir: {}", e))?;
    }
    Ok(dir)
}

/// Ensure local webview data directory exists for a specific instance
pub fn ensure_instance_webview_data_dir_local(
    instance_id: &str,
    platform: &str,
) -> Result<PathBuf, String> {
    let dir = get_instance_webview_data_dir_local(instance_id, platform);
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create webview data dir: {}", e))?;
    }
    Ok(dir)
}

// ============== Migration to Multi-Instance ==============

/// Migrate existing data to multi-instance structure
/// Called on app startup - safe to call multiple times
pub fn migrate_to_multi_instance() -> Result<(), String> {
    let mut state = load_state();

    // Check if already migrated (has instances)
    if !state.instances.is_empty() {
        debug_log("migrate_to_multi_instance: already migrated, skipping");
        return Ok(());
    }

    debug_log("migrate_to_multi_instance: starting migration");

    // Create default instance
    let default_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();

    let default_instance = Instance {
        id: default_id.clone(),
        name: "Default".to_string(),
        created_at: now,
        is_default: true,
    };

    // Migrate existing data to default instance
    let default_data = InstanceData {
        platform_credentials: state.platform_credentials.clone(),
        country_platform_auth: state.country_platform_auth.clone(),
        platforms_last_authenticated_on: state.platforms_last_authenticated_on,
        platforms_last_authenticated_hash: state.platforms_last_authenticated_hash.clone(),
        onboarding_completed: state.onboarding_completed,
    };

    // Migrate keyring entries (add instance prefix)
    for (platform, creds) in &state.platform_credentials {
        let old_keyring_key = format!("{}:{}", platform, creds.email);
        let new_keyring_key = format!("instance:{}:{}:{}", default_id, platform, creds.email);

        // Try to read from old key
        if let Ok(old_entry) = keyring::Entry::new_with_target(&old_keyring_key, KEYRING_SERVICE, &old_keyring_key) {
            if let Ok(password) = old_entry.get_password() {
                // Write to new key
                if let Ok(new_entry) = keyring::Entry::new_with_target(&new_keyring_key, KEYRING_SERVICE, &new_keyring_key) {
                    let _ = new_entry.set_password(&password);
                    debug_log(&format!("migrate_to_multi_instance: migrated keyring entry for {}", platform));
                }
                // Keep old key for now (backward compatibility if rollback needed)
            }
        }
    }

    // Migrate webview data directories
    let webview_base = get_config_dir().join("webview-data");
    if webview_base.exists() {
        // Move existing directories to instance-scoped location
        if let Ok(entries) = fs::read_dir(&webview_base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let dir_name = path.file_name().unwrap().to_str().unwrap_or("");

                    // Skip if it's already a UUID (already migrated)
                    if uuid::Uuid::parse_str(dir_name).is_ok() {
                        continue;
                    }

                    // Move country dir to instance-scoped location
                    let new_path = webview_base.join(&default_id).join(dir_name);
                    if let Some(parent) = new_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }

                    // Copy instead of move for safety during migration
                    if let Err(e) = copy_dir_recursive(&path, &new_path) {
                        debug_log(&format!("migrate_to_multi_instance: failed to copy {}: {}", dir_name, e));
                    } else {
                        debug_log(&format!("migrate_to_multi_instance: migrated webview data for {}", dir_name));
                    }
                }
            }
        }
    }

    // Save new structure
    state.instances.insert(default_id.clone(), default_instance);
    state.instance_data.insert(default_id.clone(), default_data);
    state.active_instance_id = Some(default_id);

    save_state(&state)?;

    debug_log("migrate_to_multi_instance: migration complete");
    Ok(())
}

/// Helper to recursively copy a directory
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)?;
        }
    }

    Ok(())
}
