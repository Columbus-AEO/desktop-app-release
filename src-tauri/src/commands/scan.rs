use crate::{
    commands::api::get_platform_url,
    storage,
    update_tray_status, webview::WebviewManager, AppState, PlatformState, Prompt, ScanComplete,
    ScanProgress, ScanResult,
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::RwLock as TokioRwLock;
use uuid::Uuid;

#[derive(Clone, Serialize)]
pub struct ScanProgressEvent {
    pub phase: String,
    pub current: usize,
    pub total: usize,
    pub platforms: HashMap<String, PlatformState>,
    #[serde(rename = "countdownSeconds")]
    pub countdown_seconds: Option<usize>,
}

#[tauri::command]
pub async fn start_scan(
    product_id: String,
    samples_per_prompt: Option<usize>,
    platforms: Option<Vec<String>>,
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    start_scan_internal(product_id, samples_per_prompt, platforms, app, state.inner().clone()).await
}

/// Internal scan function that can be called without Tauri State wrapper
pub async fn start_scan_internal(
    product_id: String,
    samples_per_prompt: Option<usize>,
    platforms: Option<Vec<String>>,
    app: AppHandle,
    state: Arc<AppState>,
) -> Result<(), String> {
    // Default to common platforms if none specified
    let selected_platforms: Vec<String> = platforms.unwrap_or_else(|| {
        vec!["chatgpt".to_string(), "claude".to_string(), "gemini".to_string(), "perplexity".to_string(), "google_aio".to_string()]
    });
    // Check if scan is already running
    {
        let scan = state.scan.lock();
        if scan.is_running {
            return Err("Scan already in progress".to_string());
        }
    }

    // Ensure we have a valid auth token (refresh if expired)
    let token = crate::commands::auth::ensure_valid_token(&state).await?;

    // Get prompts from API
    let prompts_response: crate::commands::api::PromptsResponse = {

        let client = reqwest::Client::new();
        let url = format!(
            "{}/functions/v1/extension-prompts?productId={}",
            crate::SUPABASE_URL,
            product_id
        );

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("apikey", crate::SUPABASE_ANON_KEY)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch prompts: {}", e))?;

        if !response.status().is_success() {
            return Err("Failed to fetch prompts".to_string());
        }

        response.json().await.map_err(|e| format!("Parse error: {}", e))?
    };

    if prompts_response.prompts.is_empty() {
        return Err("No prompts found for this product".to_string());
    }

    // Debug: Log received prompts and their target_regions
    eprintln!("[Scan] Received {} prompts from API:", prompts_response.prompts.len());
    for (i, prompt) in prompts_response.prompts.iter().enumerate() {
        eprintln!("[Scan]   Prompt {}: id={}, target_regions={:?}", i, prompt.id, prompt.target_regions);
    }

    let samples = samples_per_prompt.unwrap_or(1);
    let scan_session_id = Uuid::new_v4().to_string();
    let platform_count = selected_platforms.len();

    // Collect all unique regions from prompts
    // Each prompt can have target_regions array specifying where it should be tested
    let mut all_regions: std::collections::HashSet<String> = std::collections::HashSet::new();
    for prompt in &prompts_response.prompts {
        if prompt.target_regions.is_empty() {
            // No regions specified for this prompt - use "local"
            all_regions.insert("local".to_string());
        } else {
            for region in &prompt.target_regions {
                all_regions.insert(region.to_lowercase());
            }
        }
    }

    // Convert to Vec for iteration
    let scan_countries: Vec<String> = if all_regions.is_empty() {
        vec!["local".to_string()]
    } else {
        all_regions.into_iter().collect()
    };

    eprintln!("[Scan] Scan countries (from prompt target_regions): {:?}", scan_countries);

    // Calculate total prompt executions accounting for regional targeting
    // Each prompt runs once per target region (or once for "local" if no regions specified)
    let mut total_prompt_executions: usize = 0;
    for prompt in &prompts_response.prompts {
        if prompt.target_regions.is_empty() {
            // No regions specified - runs once in "local"
            total_prompt_executions += 1;
        } else {
            // Runs once per target region
            total_prompt_executions += prompt.target_regions.len();
        }
    }
    eprintln!("[Scan] Total prompt executions (with regions): {} (base prompts: {})", total_prompt_executions, prompts_response.prompts.len());

    // Initialize scan state
    {
        let mut scan = state.scan.lock();
        scan.is_running = true;
        scan.phase = "initializing".to_string();
        scan.scan_session_id = Some(scan_session_id.clone());
        scan.product_id = Some(product_id.clone());
        // Total = prompt executions × samples × platforms
        scan.total_prompts = total_prompt_executions * samples * platform_count;
        scan.completed_prompts = 0;

        // Initialize platform states for selected platforms only
        // Each platform will process all prompt executions
        scan.platforms.clear();
        for platform in &selected_platforms {
            scan.platforms.insert(
                platform.clone(),
                PlatformState {
                    status: "pending".to_string(),
                    total: total_prompt_executions * samples,
                    submitted: 0,
                    collected: 0,
                    failed: 0,
                },
            );
        }
    }

    // Update tray to show scanning
    update_tray_status(&app, true);

    // Emit scan started event so UI can switch to scanning view
    let _ = app.emit("scan:started", serde_json::json!({
        "productId": product_id,
        "totalPrompts": total_prompt_executions * samples * platform_count,
        "platforms": selected_platforms,
    }));

    // Emit initial progress
    emit_progress_with_state(&app, &state);

    // Clone necessary data for async task
    let state_clone = state.clone();
    let app_clone = app.clone();
    let prompts = prompts_response.prompts.clone();
    let brand = prompts_response.product.brand.clone();
    let brand_domain = prompts_response.product.domain.clone();
    let domain_aliases = prompts_response.product.domain_aliases.clone();
    let competitors = prompts_response.competitors.clone();
    let platforms_for_scan = selected_platforms.clone();
    let countries_for_scan = scan_countries.clone();

    // Spawn scan task
    tokio::spawn(async move {
        let result = run_scan(
            app_clone.clone(),
            state_clone.clone(),
            prompts,
            samples,
            scan_session_id.clone(),
            product_id.clone(),
            brand,
            brand_domain,
            domain_aliases,
            competitors,
            platforms_for_scan,
            countries_for_scan,
        )
        .await;

        // Handle completion or error
        match result {
            Ok(stats) => {
                let _ = app_clone.emit("scan:complete", stats);
            }
            Err(e) => {
                let _ = app_clone.emit("scan:error", e.clone());
                eprintln!("Scan error: {}", e);
            }
        }

        // Reset tray to normal
        update_tray_status(&app_clone, false);

        // Reset scan state
        let mut scan = state_clone.scan.lock();
        scan.is_running = false;
        scan.phase = "complete".to_string();
    });

    Ok(())
}

/// Information about a webview that needs to be processed
#[derive(Clone)]
struct WebviewTask {
    label: String,
    country_code: String,
    platform: String,
    #[allow(dead_code)]
    prompt_idx: usize,
    prompt: Prompt,
    #[allow(dead_code)]
    sample: usize,
    is_local: bool,
}

/// Group tasks by platform for concurrent per-platform execution
fn group_tasks_by_platform(tasks: &[WebviewTask]) -> HashMap<String, Vec<WebviewTask>> {
    let mut platform_tasks: HashMap<String, Vec<WebviewTask>> = HashMap::new();

    for task in tasks {
        platform_tasks
            .entry(task.platform.clone())
            .or_insert_with(Vec::new)
            .push(task.clone());
    }

    platform_tasks
}

/// Get the wait time in seconds for a specific platform
/// Some platforms respond faster than others
fn get_platform_wait_time(platform: &str) -> u64 {
    match platform {
        "google_aio" => 20,      // Google AIO is typically fast
        "google_ai_mode" => 25,  // Google AI Mode needs a bit more time
        "perplexity" => 35,      // Perplexity searches take time
        "gemini" => 40,          // Gemini is moderate
        "chatgpt" => 45,         // ChatGPT can be slow
        "claude" => 45,          // Claude can also be slow
        _ => 45,                 // Default to conservative wait
    }
}

/// Submit a prompt to a webview without needing the WebviewManager lock
/// This is safe to call concurrently from multiple platform tasks
async fn submit_prompt_standalone(
    app: &AppHandle,
    label: &str,
    platform: &str,
    prompt: &str,
) -> Result<(), String> {
    let window = app
        .get_webview_window(label)
        .ok_or("Webview not found")?;

    eprintln!("[Scan] Submitting prompt to {} in webview {}", platform, label);

    let script = crate::webview::get_submit_script(platform, prompt);
    window
        .eval(&script)
        .map_err(|e| format!("Script error: {}", e))?;

    Ok(())
}

/// Collect a response from a webview without needing the WebviewManager lock
/// This is safe to call concurrently from multiple platform tasks
async fn collect_response_standalone(
    app: &AppHandle,
    label: &str,
    platform: &str,
    brand: &str,
    brand_domain: Option<&str>,
    domain_aliases: Option<&[String]>,
    competitors: &[String],
) -> Result<crate::webview::CollectResponse, String> {
    let window = app
        .get_webview_window(label)
        .ok_or("Webview not found")?;

    eprintln!("[Scan] Collecting response from {} in webview {}", platform, label);

    // For Gemini, click the Sources button first to open the sidebar
    if platform == "gemini" {
        let open_sources_script = r#"
            (function() {
                const sourcesButton = document.querySelector('button.legacy-sources-sidebar-button, button[class*="sources-sidebar"], button mat-icon[fonticon="link"]');
                if (sourcesButton) {
                    const btn = sourcesButton.closest('button') || sourcesButton;
                    btn.click();
                }
            })();
        "#;
        window.eval(open_sources_script).ok();
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    }

    // For Perplexity, click the sources button to expand the sources panel
    if platform == "perplexity" {
        let open_sources_script = r#"
            (function() {
                const buttons = document.querySelectorAll('button');
                for (const btn of buttons) {
                    const text = btn.textContent?.toLowerCase() || '';
                    if (/\d+\s*(quellen|sources|source)/i.test(text)) {
                        btn.click();
                        return;
                    }
                    const favicons = btn.querySelectorAll('img[alt*="favicon"]');
                    if (favicons.length >= 2) {
                        btn.click();
                        return;
                    }
                }
            })();
        "#;
        window.eval(open_sources_script).ok();
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    }

    // For Google AI Overview, click "Show more" and "Show all" buttons
    if platform == "google_aio" {
        let expand_aio_script = r#"
            (async function() {
                let showMoreBtn = document.querySelector('div.Jzkafd[aria-label="Show more AI Overview"]');
                if (!showMoreBtn) showMoreBtn = document.querySelector('div[aria-label="Show more AI Overview"]');
                if (!showMoreBtn) showMoreBtn = document.querySelector('[aria-label*="Show more"][aria-label*="AI Overview"]');
                if (showMoreBtn) {
                    const clickable = showMoreBtn.querySelector('div.in7vHe') || showMoreBtn;
                    clickable.click();
                    await new Promise(r => setTimeout(r, 1500));
                }
                let showAllBtn = document.querySelector('div.BjvG9b[aria-label="Show all related links"]');
                if (!showAllBtn) showAllBtn = document.querySelector('div[aria-label="Show all related links"]');
                if (!showAllBtn) showAllBtn = document.querySelector('[aria-label*="Show all"]');
                if (showAllBtn) {
                    showAllBtn.click();
                    await new Promise(r => setTimeout(r, 1000));
                }
            })();
        "#;
        window.eval(expand_aio_script).ok();
        tokio::time::sleep(tokio::time::Duration::from_millis(2500)).await;
    }

    // Inject script that collects response
    let script = crate::webview::get_collect_script(platform, brand, brand_domain, domain_aliases, competitors);
    window
        .eval(&script)
        .map_err(|e| format!("Script error: {}", e))?;

    // Wait for script to execute
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Read the URL which contains our result in the hash
    let url = window.url().map_err(|e| format!("Failed to get URL: {}", e))?;
    let url_str = url.as_str();

    // Parse the result from URL hash
    if let Some(hash_pos) = url_str.find("#COLUMBUS_RESULT:") {
        let data = &url_str[hash_pos + 17..];
        match crate::webview::decode_base64_and_parse(data) {
            Ok(response) => {
                eprintln!("[Scan] Parsed response: brand_mentioned={}, citation_present={}",
                         response.brand_mentioned, response.citation_present);
                return Ok(response);
            }
            Err(e) => {
                eprintln!("[Scan] Failed to parse result: {}", e);
            }
        }
    }

    // Fallback: return empty response
    Ok(crate::webview::CollectResponse::default())
}

/// Result from processing all prompts for a single platform
struct PlatformScanResult {
    platform: String,
    collected: usize,
    mentioned: usize,
    cited: usize,
    failed: usize,
}

/// Process all prompts for a single platform sequentially
/// This runs as an independent task, one per platform
async fn run_platform_scan(
    app: AppHandle,
    state: Arc<AppState>,
    manager: Arc<TokioRwLock<WebviewManager>>,
    platform: String,
    tasks: Vec<WebviewTask>,
    scan_session_id: String,
    product_id: String,
    brand: String,
    brand_domain: Option<String>,
    domain_aliases: Option<Vec<String>>,
    competitors: Vec<String>,
    token: Option<String>,
) -> PlatformScanResult {
    let client = reqwest::Client::new();
    let is_visible = cfg!(debug_assertions);
    let wait_seconds = get_platform_wait_time(&platform);

    let mut collected = 0;
    let mut mentioned = 0;
    let mut cited = 0;
    let mut failed = 0;

    eprintln!("[Scan/{}] Starting platform scan with {} tasks", platform, tasks.len());

    for (task_idx, task) in tasks.iter().enumerate() {
        // Check cancellation
        if !state.scan.lock().is_running {
            eprintln!("[Scan/{}] Cancelled at task {}/{}", platform, task_idx + 1, tasks.len());
            break;
        }

        eprintln!("[Scan/{}] Processing task {}/{}: {}", platform, task_idx + 1, tasks.len(), task.label);

        // === STEP 1: Create webview ===
        let url = match get_platform_url(&task.platform) {
            Some(u) => u,
            None => {
                eprintln!("[Scan/{}] Unknown platform URL, skipping", platform);
                failed += 1;
                continue;
            }
        };

        let create_result = if task.is_local {
            manager.write().await.create_webview_local(&app, &task.label, &url, is_visible, &task.platform)
        } else {
            manager.write().await.create_webview_for_country(&app, &task.label, &url, is_visible, &task.country_code, &task.platform).await
        };

        if create_result.is_err() {
            eprintln!("[Scan/{}] Failed to create webview: {:?}", platform, create_result.err());
            failed += 1;
            {
                let mut scan = state.scan.lock();
                if let Some(ps) = scan.platforms.get_mut(&platform) {
                    ps.failed += 1;
                }
            }
            emit_progress_with_state(&app, &state);
            continue;
        }

        // Track webview label
        state.scan_webview_labels.lock().push(task.label.clone());

        // === STEP 2: Wait for page to load ===
        for _ in 0..10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            if !state.scan.lock().is_running {
                // Cleanup and exit
                manager.write().await.close_webview(&app, &task.label);
                state.scan_webview_labels.lock().retain(|l| l != &task.label);
                eprintln!("[Scan/{}] Cancelled during page load", platform);
                return PlatformScanResult { platform, collected, mentioned, cited, failed };
            }
        }

        // === STEP 3: Submit prompt (no lock needed - uses standalone function) ===
        let submit_result = submit_prompt_standalone(&app, &task.label, &task.platform, &task.prompt.text).await;

        // For google_ai_mode, wait and submit again (handles navigation)
        if task.platform == "google_ai_mode" {
            for _ in 0..8 {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                if !state.scan.lock().is_running {
                    break;
                }
            }
            if state.scan.lock().is_running {
                let _ = submit_prompt_standalone(&app, &task.label, &task.platform, &task.prompt.text).await;
            }
        }

        if submit_result.is_err() {
            eprintln!("[Scan/{}] Failed to submit prompt: {:?}", platform, submit_result.err());
            manager.write().await.close_webview(&app, &task.label);
            state.scan_webview_labels.lock().retain(|l| l != &task.label);
            failed += 1;
            {
                let mut scan = state.scan.lock();
                if let Some(ps) = scan.platforms.get_mut(&platform) {
                    ps.failed += 1;
                }
            }
            emit_progress_with_state(&app, &state);
            continue;
        }

        // Update submitted count
        {
            let mut scan = state.scan.lock();
            if let Some(ps) = scan.platforms.get_mut(&platform) {
                ps.submitted += 1;
                ps.status = "submitting".to_string();
            }
        }
        emit_progress_with_state(&app, &state);

        // === STEP 4: Wait for AI response ===
        for _ in 0..wait_seconds {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            if !state.scan.lock().is_running {
                manager.write().await.close_webview(&app, &task.label);
                state.scan_webview_labels.lock().retain(|l| l != &task.label);
                eprintln!("[Scan/{}] Cancelled during response wait", platform);
                return PlatformScanResult { platform, collected, mentioned, cited, failed };
            }
        }

        // === STEP 5: Collect response (no lock needed - uses standalone function) ===
        let collect_result = collect_response_standalone(
            &app,
            &task.label,
            &task.platform,
            &brand,
            brand_domain.as_deref(),
            domain_aliases.as_deref(),
            &competitors,
        ).await;

        // Close webview after collecting
        manager.write().await.close_webview(&app, &task.label);
        state.scan_webview_labels.lock().retain(|l| l != &task.label);

        match collect_result {
            Ok(response) => {
                collected += 1;
                if response.brand_mentioned {
                    mentioned += 1;
                }
                if response.citation_present {
                    cited += 1;
                }

                // Update platform stats
                {
                    let mut scan = state.scan.lock();
                    if let Some(ps) = scan.platforms.get_mut(&platform) {
                        ps.collected += 1;
                    }
                    scan.completed_prompts += 1;
                }
                emit_progress_with_state(&app, &state);

                // Submit to API
                if let Some(ref token) = token {
                    let api_result = ScanResult {
                        product_id: product_id.clone(),
                        scan_session_id: scan_session_id.clone(),
                        platform: platform.clone(),
                        prompt_id: task.prompt.id.clone(),
                        prompt_text: task.prompt.text.clone(),
                        response_text: response.response_text,
                        brand_mentioned: response.brand_mentioned,
                        citation_present: response.citation_present,
                        position: response.position,
                        sentiment: response.sentiment.clone(),
                        competitor_mentions: response.competitor_mentions,
                        competitor_details: response.competitor_details.iter().map(|cd| {
                            crate::CompetitorDetailResult {
                                name: cd.name.clone(),
                                position: cd.position,
                                sentiment: cd.sentiment.clone(),
                            }
                        }).collect(),
                        citations: response.citations,
                        credits_exhausted: response.credits_exhausted,
                        chat_url: response.chat_url,
                        request_country: Some(task.country_code.clone()),
                    };

                    match client
                        .post(format!("{}/functions/v1/extension-scan-results", crate::SUPABASE_URL))
                        .header("Authorization", format!("Bearer {}", token))
                        .header("apikey", crate::SUPABASE_ANON_KEY)
                        .header("Content-Type", "application/json")
                        .json(&api_result)
                        .send()
                        .await
                    {
                        Ok(resp) => {
                            if !resp.status().is_success() {
                                eprintln!("[Scan/{}] API submission failed: {}", platform, resp.status());
                            }
                        }
                        Err(e) => eprintln!("[Scan/{}] API request error: {}", platform, e),
                    }
                }
            }
            Err(e) => {
                eprintln!("[Scan/{}] Collection failed: {}", platform, e);
                failed += 1;
                {
                    let mut scan = state.scan.lock();
                    if let Some(ps) = scan.platforms.get_mut(&platform) {
                        ps.failed += 1;
                    }
                }
                emit_progress_with_state(&app, &state);
            }
        }
    }

    eprintln!("[Scan/{}] Platform scan complete: collected={}, mentioned={}, cited={}, failed={}",
              platform, collected, mentioned, cited, failed);

    PlatformScanResult { platform, collected, mentioned, cited, failed }
}

async fn run_scan(
    app: AppHandle,
    state: Arc<AppState>,
    prompts: Vec<Prompt>,
    samples: usize,
    scan_session_id: String,
    product_id: String,
    brand: String,
    brand_domain: Option<String>,
    domain_aliases: Option<Vec<String>>,
    competitors: Vec<String>,
    selected_platforms: Vec<String>,
    scan_countries: Vec<String>,
) -> Result<ScanComplete, String> {
    // Use a thread-safe manager wrapped in Arc<TokioRwLock>
    // RwLock allows concurrent reads (submit_prompt, collect_response)
    // while write operations (create_webview, close_webview) get exclusive access
    let manager = Arc::new(TokioRwLock::new(WebviewManager::new()));

    // Clear any previous scan webview labels
    {
        let mut labels = state.scan_webview_labels.lock();
        labels.clear();
    }

    // Update phase
    {
        let mut scan = state.scan.lock();
        scan.phase = "initializing".to_string();
    }
    emit_progress_with_state(&app, &state);

    // ============== PHASE 1: Build Valid Combinations ==============
    // Build list of country/platform combos based on stored auth status
    eprintln!("[Scan] Phase 1: Building valid platform combinations...");

    let mut valid_combinations: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();

    for country_code in &scan_countries {
        let is_local = country_code == "local";

        for platform_str in &selected_platforms {
            // For geo-targeted scans, check if this country/platform combo is marked as authenticated
            if !is_local {
                let is_authenticated = storage::is_country_platform_authenticated(country_code, platform_str);
                if !is_authenticated {
                    eprintln!("[Scan] Country {} / Platform {} not authenticated, skipping", country_code, platform_str);
                    continue;
                }
            }

            // Add to valid combinations
            valid_combinations.insert((country_code.clone(), platform_str.clone()));

            // Mark platform as ready
            {
                let mut scan = state.scan.lock();
                if let Some(ps) = scan.platforms.get_mut(platform_str) {
                    ps.status = "ready".to_string();
                }
            }
        }
    }
    emit_progress_with_state(&app, &state);

    eprintln!("[Scan] Valid combinations: {:?}", valid_combinations);

    if valid_combinations.is_empty() {
        return Err("No platforms available - please authenticate at least one platform".to_string());
    }

    // ============== PHASE 2: Parallel Webview Creation & Prompt Submission ==============
    {
        let mut scan = state.scan.lock();
        scan.phase = "submitting".to_string();
    }
    emit_progress_with_state(&app, &state);

    eprintln!("[Scan] Phase 2: Creating webviews and submitting prompts in parallel...");

    // Build list of all webview tasks
    let mut webview_tasks: Vec<WebviewTask> = Vec::new();

    for country_code in &scan_countries {
        let is_local = country_code == "local";

        for platform_str in &selected_platforms {
            // Skip if not a valid combination
            if !valid_combinations.contains(&(country_code.clone(), platform_str.clone())) {
                continue;
            }

            // Only process prompts that target this specific country
            let prompts_for_country: Vec<_> = prompts.iter().enumerate().filter(|(_, p)| {
                if p.target_regions.is_empty() {
                    is_local
                } else {
                    p.target_regions.iter().any(|r| r.to_lowercase() == country_code.to_lowercase())
                }
            }).collect();

            for (prompt_idx, prompt) in prompts_for_country {
                for sample in 0..samples {
                    webview_tasks.push(WebviewTask {
                        label: format!("scan-{}-{}-{}-{}-{}", &scan_session_id[..8], country_code, platform_str, prompt_idx, sample),
                        country_code: country_code.clone(),
                        platform: platform_str.clone(),
                        prompt_idx,
                        prompt: prompt.clone(),
                        sample,
                        is_local,
                    });
                }
            }
        }
    }

    eprintln!("[Scan] Total webview tasks to process: {}", webview_tasks.len());

    // Get token once for all API submissions
    let token = crate::commands::auth::ensure_valid_token(&state).await.ok();

    // Group tasks by platform for concurrent execution
    let platform_tasks = group_tasks_by_platform(&webview_tasks);
    let num_platforms = platform_tasks.len();

    eprintln!("[Scan] Running {} platforms concurrently (one prompt at a time per platform)", num_platforms);
    for (platform, tasks) in &platform_tasks {
        eprintln!("[Scan]   {} - {} tasks", platform, tasks.len());
    }

    // Update phase to submitting - all platforms will run concurrently
    {
        let mut scan = state.scan.lock();
        scan.phase = "submitting".to_string();
        for platform_str in &selected_platforms {
            if let Some(ps) = scan.platforms.get_mut(platform_str) {
                ps.status = "running".to_string();
            }
        }
    }
    emit_progress_with_state(&app, &state);

    // Spawn one task per platform - all run concurrently
    let mut platform_handles = Vec::new();

    for (platform, tasks) in platform_tasks {
        let app_clone = app.clone();
        let state_clone = state.clone();
        let manager_clone = manager.clone();
        let scan_session_id_clone = scan_session_id.clone();
        let product_id_clone = product_id.clone();
        let brand_clone = brand.clone();
        let brand_domain_clone = brand_domain.clone();
        let domain_aliases_clone = domain_aliases.clone();
        let competitors_clone = competitors.clone();
        let token_clone = token.clone();

        let handle = tokio::spawn(async move {
            run_platform_scan(
                app_clone,
                state_clone,
                manager_clone,
                platform,
                tasks,
                scan_session_id_clone,
                product_id_clone,
                brand_clone,
                brand_domain_clone,
                domain_aliases_clone,
                competitors_clone,
                token_clone,
            ).await
        });

        platform_handles.push(handle);
    }

    // Wait for all platforms to complete
    let platform_results = futures::future::join_all(platform_handles).await;

    // Aggregate results from all platforms
    let mut total_collected = 0;
    let mut total_mentioned = 0;
    let mut total_cited = 0;

    for result in platform_results {
        match result {
            Ok(platform_result) => {
                eprintln!("[Scan] Platform {} finished: collected={}, mentioned={}, cited={}, failed={}",
                          platform_result.platform, platform_result.collected, platform_result.mentioned,
                          platform_result.cited, platform_result.failed);
                total_collected += platform_result.collected;
                total_mentioned += platform_result.mentioned;
                total_cited += platform_result.cited;
            }
            Err(e) => {
                eprintln!("[Scan] Platform task panicked: {}", e);
            }
        }
    }

    eprintln!("[Scan] All {} platforms complete. Total collected: {} responses", num_platforms, total_collected);

    // Mark all platforms as complete
    {
        let mut scan = state.scan.lock();
        for platform_str in &selected_platforms {
            if let Some(ps) = scan.platforms.get_mut(platform_str) {
                if ps.status != "skipped" {
                    ps.status = "complete".to_string();
                }
            }
        }
    }
    emit_progress_with_state(&app, &state);

    // ============== PHASE 5: Finalize ==============
    let client = reqwest::Client::new();
    if let Some(token) = token {
        eprintln!("[Scan] Finalizing scan session {}...", scan_session_id);

        match client
            .post(format!("{}/functions/v1/extension-finalize-scan", crate::SUPABASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .header("apikey", crate::SUPABASE_ANON_KEY)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "scanSessionId": scan_session_id,
                "productId": product_id
            }))
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    eprintln!("[Scan] Finalize successful");
                } else {
                    eprintln!("[Scan] Finalize failed: {}", resp.status());
                }
            }
            Err(e) => eprintln!("[Scan] Finalize request error: {}", e),
        }
    }

    // Final cleanup
    eprintln!("[Columbus] Scan complete - performing final webview cleanup");
    manager.write().await.close_all(&app);
    eprintln!("[Columbus] Final webview cleanup complete");

    let mention_rate = if total_collected > 0 {
        (total_mentioned as f64 / total_collected as f64) * 100.0
    } else {
        0.0
    };

    let citation_rate = if total_collected > 0 {
        (total_cited as f64 / total_collected as f64) * 100.0
    } else {
        0.0
    };

    // Calculate total prompt executions for the completion stats
    let mut completion_total: usize = 0;
    for prompt in &prompts {
        if prompt.target_regions.is_empty() {
            completion_total += 1;
        } else {
            completion_total += prompt.target_regions.len();
        }
    }

    Ok(ScanComplete {
        total_prompts: completion_total * samples * selected_platforms.len(),
        successful_prompts: total_collected,
        mention_rate,
        citation_rate,
    })
}

#[tauri::command]
pub async fn cancel_scan(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    eprintln!("[Scan] Cancel requested");

    // Mark scan as cancelled
    {
        let mut scan = state.scan.lock();
        scan.is_running = false;
        scan.phase = "cancelled".to_string();
    }

    // Close all scan webviews
    let labels_to_close: Vec<String> = {
        let mut labels = state.scan_webview_labels.lock();
        let to_close = labels.clone();
        labels.clear();
        to_close
    };

    eprintln!("[Scan] Closing {} webviews on cancel", labels_to_close.len());
    for label in labels_to_close {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.destroy();
        }
    }

    // Reset tray to normal
    update_tray_status(&app, false);

    Ok(())
}

#[tauri::command]
pub async fn get_scan_progress(state: State<'_, Arc<AppState>>) -> Result<ScanProgress, String> {
    let scan = state.scan.lock();
    Ok(ScanProgress {
        phase: scan.phase.clone(),
        current: scan.completed_prompts,
        total: scan.total_prompts,
        platforms: scan.platforms.clone(),
    })
}

#[tauri::command]
pub async fn is_scan_running(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    let scan = state.scan.lock();
    Ok(scan.is_running)
}

fn emit_progress_with_state(app: &AppHandle, state: &Arc<AppState>) {
    let scan = state.scan.lock();
    let _ = app.emit(
        "scan:progress",
        ScanProgressEvent {
            phase: scan.phase.clone(),
            current: scan.completed_prompts,
            total: scan.total_prompts,
            platforms: scan.platforms.clone(),
            countdown_seconds: None,
        },
    );
}

fn emit_progress_with_countdown(app: &AppHandle, state: &Arc<AppState>, countdown: usize) {
    let scan = state.scan.lock();
    let _ = app.emit(
        "scan:progress",
        ScanProgressEvent {
            phase: scan.phase.clone(),
            current: scan.completed_prompts,
            total: scan.total_prompts,
            platforms: scan.platforms.clone(),
            countdown_seconds: Some(countdown),
        },
    );
}
