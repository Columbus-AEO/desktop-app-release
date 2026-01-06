use crate::{AppState, Product, Prompt, ScanResult, SUPABASE_ANON_KEY, SUPABASE_URL};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

// Cached AI platforms
lazy_static::lazy_static! {
    static ref CACHED_PLATFORMS: Mutex<Option<Vec<AIPlatform>>> = Mutex::new(None);
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AIPlatform {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub logo_url: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub website_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct StatusResponse {
    pub user: UserInfo,
    pub products: Vec<Product>,
    #[serde(rename = "activeProduct")]
    pub active_product: Option<Product>,
}

#[derive(Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
}

#[derive(Serialize, Deserialize)]
pub struct PromptsResponse {
    pub product: ProductInfo,
    pub prompts: Vec<Prompt>,
    pub competitors: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QuotaInfo {
    #[serde(rename = "promptsUsedToday")]
    pub prompts_used_today: i32,
    #[serde(rename = "promptsPerDay")]
    pub prompts_per_day: i32,
    #[serde(rename = "promptsRemaining")]
    pub prompts_remaining: i32,
    #[serde(rename = "resetAt")]
    pub reset_at: Option<String>,
    pub plan: String,
    #[serde(rename = "isUnlimited")]
    pub is_unlimited: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExtensionPromptsResponse {
    pub product: ProductInfo,
    pub prompts: Vec<serde_json::Value>, // Using Value to include all fields
    pub competitors: Vec<String>,
    #[serde(rename = "totalPrompts")]
    pub total_prompts: i32,
    pub platforms: Vec<String>,
    pub quota: QuotaInfo,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DailyUsageResponse {
    pub current: i32,
    pub limit: i32,
    pub remaining: i32,
    #[serde(rename = "effectiveRemaining", default)]
    pub effective_remaining: Option<i32>,
    #[serde(rename = "pendingEvaluations", default)]
    pub pending_evaluations: Option<i32>,
    #[serde(rename = "resetAt")]
    pub reset_at: Option<String>,
    pub plan: String,
    #[serde(rename = "isUnlimited")]
    pub is_unlimited: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProductInfo {
    pub id: String,
    pub name: String,
    pub brand: String,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub domain_aliases: Option<Vec<String>>,
}

async fn api_request<T: serde::de::DeserializeOwned>(
    endpoint: &str,
    method: &str,
    body: Option<serde_json::Value>,
    state: &State<'_, Arc<AppState>>,
) -> Result<T, String> {
    let token = {
        let auth = state.auth.lock();
        auth.access_token.clone().ok_or("Not authenticated")?
    };

    let client = reqwest::Client::new();
    let url = format!("{}{}", SUPABASE_URL, endpoint);

    let mut request = match method {
        "POST" => client.post(&url),
        _ => client.get(&url),
    };

    request = request
        .header("Authorization", format!("Bearer {}", token))
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Content-Type", "application/json");

    if let Some(b) = body {
        request = request.json(&b);
    }

    let response = request.send().await.map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, error_text));
    }

    response.json().await.map_err(|e| format!("Parse error: {}", e))
}

#[tauri::command]
pub async fn get_status(state: State<'_, Arc<AppState>>) -> Result<StatusResponse, String> {
    api_request("/functions/v1/extension-status", "GET", None, &state).await
}

#[tauri::command]
pub async fn get_prompts(
    product_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<PromptsResponse, String> {
    let endpoint = format!("/functions/v1/extension-prompts?productId={}", product_id);
    api_request(&endpoint, "GET", None, &state).await
}

#[tauri::command]
pub async fn submit_scan_result(
    result: ScanResult,
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    api_request(
        "/functions/v1/extension-scan-results",
        "POST",
        Some(serde_json::to_value(&result).map_err(|e| e.to_string())?),
        &state,
    )
    .await
}

#[tauri::command]
pub async fn finalize_scan(
    scan_session_id: String,
    product_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    api_request(
        "/functions/v1/extension-finalize-scan",
        "POST",
        Some(serde_json::json!({
            "scanSessionId": scan_session_id,
            "productId": product_id
        })),
        &state,
    )
    .await
}

/// Fetch AI platforms from the database (public, no auth required)
#[tauri::command]
pub async fn get_ai_platforms(force_refresh: Option<bool>) -> Result<Vec<AIPlatform>, String> {
    // Check cache first
    if force_refresh != Some(true) {
        let cache = CACHED_PLATFORMS.lock();
        if let Some(platforms) = cache.as_ref() {
            return Ok(platforms.clone());
        }
    }

    // Fetch from Supabase REST API (public table with RLS allowing SELECT)
    let client = reqwest::Client::new();
    let url = format!("{}/rest/v1/ai_platforms?select=*&order=name", SUPABASE_URL);

    let response = client
        .get(&url)
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch platforms: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, error_text));
    }

    let platforms: Vec<AIPlatform> = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    // Cache the platforms
    {
        let mut cache = CACHED_PLATFORMS.lock();
        *cache = Some(platforms.clone());
    }

    Ok(platforms)
}

/// Get prompt target regions for a product
/// Returns a map of prompt_id -> target_regions
#[tauri::command]
pub async fn get_prompt_target_regions(
    product_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<std::collections::HashMap<String, Vec<String>>, String> {
    let prompts_response: PromptsResponse =
        get_prompts(product_id, state).await?;

    let mut result = std::collections::HashMap::new();
    for prompt in prompts_response.prompts {
        result.insert(prompt.id, prompt.target_regions);
    }

    Ok(result)
}

#[derive(Deserialize)]
struct PromptRegionsResponse {
    regions: Vec<String>,
}

/// Get unique prompt regions for a product
/// Returns a list of unique region codes that prompts target
#[tauri::command]
pub async fn get_prompt_regions(
    product_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<String>, String> {
    let endpoint = format!("/functions/v1/extension-prompt-regions?productId={}", product_id);
    let response: PromptRegionsResponse = api_request(&endpoint, "GET", None, &state).await?;
    Ok(response.regions)
}

/// Get platform URL for opening login. Uses cached platforms or fallback.
pub fn get_platform_url(platform_id: &str) -> Option<String> {
    // Try to get from cache
    let cache = CACHED_PLATFORMS.lock();
    if let Some(platforms) = cache.as_ref() {
        if let Some(p) = platforms.iter().find(|p| p.id == platform_id) {
            return p.website_url.clone();
        }
    }

    // Fallback to hardcoded URLs for common platforms
    match platform_id {
        "chatgpt" => Some("https://chatgpt.com/".to_string()),
        "claude" => Some("https://claude.ai/new".to_string()),
        "gemini" => Some("https://gemini.google.com/app".to_string()),
        "perplexity" => Some("https://www.perplexity.ai/".to_string()),
        "google_aio" => Some("https://www.google.com/".to_string()),
        "google_ai_mode" => Some("https://www.google.com/".to_string()),
        _ => None,
    }
}

/// Fetch full extension prompts response including quota info
#[tauri::command]
pub async fn fetch_extension_prompts(
    product_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<ExtensionPromptsResponse, String> {
    let endpoint = format!("/functions/v1/extension-prompts?productId={}", product_id);
    api_request(&endpoint, "GET", None, &state).await
}

/// Check daily usage for prompt tests
#[tauri::command]
pub async fn check_daily_usage(
    state: State<'_, Arc<AppState>>,
) -> Result<DailyUsageResponse, String> {
    api_request("/functions/v1/check-daily-usage", "GET", None, &state).await
}
