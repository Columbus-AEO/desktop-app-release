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
use tokio::sync::Mutex as TokioMutex;
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
    prompt_idx: usize,
    prompt: Prompt,
    sample: usize,
    is_local: bool,
}

/// Result of a scan task
struct ScanTaskResult {
    webview_label: String,
    platform: String,
    country_code: String,
    prompt: Prompt,
    response: Option<crate::webview::CollectResponse>,
    error: Option<String>,
}

/// Build batches that respect platform-specific concurrent limits
/// Claude has a limit of 3 concurrent windows, other platforms use the general max
fn build_smart_batches(
    tasks: &[WebviewTask],
    max_concurrent: usize,
    claude_max: usize,
) -> Vec<Vec<WebviewTask>> {
    let mut batches: Vec<Vec<WebviewTask>> = Vec::new();
    let mut remaining: Vec<WebviewTask> = tasks.to_vec();

    while !remaining.is_empty() {
        let mut batch: Vec<WebviewTask> = Vec::new();
        let mut claude_count = 0;
        let mut indices_to_remove: Vec<usize> = Vec::new();

        for (i, task) in remaining.iter().enumerate() {
            // Check if batch is full
            if batch.len() >= max_concurrent {
                break;
            }

            // Check Claude-specific limit
            if task.platform == "claude" {
                if claude_count >= claude_max {
                    continue; // Skip this Claude task, try to add other platforms
                }
                claude_count += 1;
            }

            batch.push(task.clone());
            indices_to_remove.push(i);
        }

        // Remove added tasks from remaining (in reverse order to preserve indices)
        for i in indices_to_remove.into_iter().rev() {
            remaining.remove(i);
        }

        if !batch.is_empty() {
            batches.push(batch);
        }
    }

    batches
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
    // Use a thread-safe manager wrapped in Arc<TokioMutex>
    let manager = Arc::new(TokioMutex::new(WebviewManager::new()));

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

    // Helper to check if scan is cancelled
    fn is_scan_running(state: &Arc<AppState>) -> bool {
        state.scan.lock().is_running
    }

    let is_visible = cfg!(debug_assertions);

    // Get max concurrent webviews based on system RAM
    let max_concurrent = state.max_concurrent_webviews;

    // Platform-specific concurrent limits (Claude blocks more than 3 open windows)
    const CLAUDE_MAX_CONCURRENT: usize = 3;

    // Build smart batches that respect platform-specific limits
    let batches = build_smart_batches(&webview_tasks, max_concurrent, CLAUDE_MAX_CONCURRENT);
    let num_batches = batches.len();
    let total_tasks = webview_tasks.len();

    eprintln!("[Scan] RAM limit: {} concurrent webviews, Claude limit: {}, processing in {} batches", max_concurrent, CLAUDE_MAX_CONCURRENT, num_batches);

    // Track aggregated results across all batches
    let mut total_collected = 0;
    let mut total_mentioned = 0;
    let mut total_cited = 0;
    let mut batch_number = 0;

    // Get token once for all API submissions
    let token = crate::commands::auth::ensure_valid_token(&state).await.ok();
    let client = reqwest::Client::new();

    // Process webviews in batches - each batch completes its full lifecycle before next
    for batch in &batches {
        batch_number += 1;

        // Check cancellation before starting batch
        if !is_scan_running(&state) {
            eprintln!("[Scan] Cancelled before batch {}", batch_number);
            break;
        }

        eprintln!("[Scan] === Processing batch {}/{} ({} webviews) ===", batch_number, num_batches, batch.len());

        // =============================================================
        // BATCH PHASE 1: Create webviews in this batch
        // =============================================================
        {
            let mut scan = state.scan.lock();
            scan.phase = "submitting".to_string();
        }
        emit_progress_with_state(&app, &state);

        let mut batch_created_labels: Vec<String> = Vec::new();
        let mut batch_submitted_labels: Vec<String> = Vec::new();

        for task in batch {
            if !is_scan_running(&state) {
                eprintln!("[Scan] Cancelled during webview creation in batch {}", batch_number);
                break;
            }

            let url = match get_platform_url(&task.platform) {
                Some(u) => u,
                None => {
                    eprintln!("[Scan] Unknown platform: {}", task.platform);
                    continue;
                }
            };

            eprintln!("[Scan] Creating webview: {}", task.label);

            let create_result = {
                let mut mgr = manager.lock().await;
                if task.is_local {
                    mgr.create_webview_local(&app, &task.label, &url, is_visible, &task.platform)
                } else {
                    mgr.create_webview_for_country(&app, &task.label, &url, is_visible, &task.country_code, &task.platform).await
                }
            };

            match create_result {
                Ok(_) => {
                    batch_created_labels.push(task.label.clone());
                    state.scan_webview_labels.lock().push(task.label.clone());
                }
                Err(e) => {
                    eprintln!("[Scan] Failed to create webview {}: {}", task.label, e);
                }
            }
        }

        eprintln!("[Scan] Batch {}: Created {} webviews", batch_number, batch_created_labels.len());

        if batch_created_labels.is_empty() {
            eprintln!("[Scan] Batch {}: No webviews created, skipping to next batch", batch_number);
            continue;
        }

        // =============================================================
        // BATCH PHASE 2: Wait for pages to load (~5 seconds)
        // =============================================================
        eprintln!("[Scan] Batch {}: Waiting for pages to load...", batch_number);
        for _ in 0..10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            if !is_scan_running(&state) {
                eprintln!("[Scan] Cancelled during page load wait in batch {}", batch_number);
                // Close batch webviews before returning
                let mut mgr = manager.lock().await;
                for label in &batch_created_labels {
                    mgr.close_webview(&app, label);
                }
                return Err("Scan cancelled".to_string());
            }
        }

        // =============================================================
        // BATCH PHASE 3: Submit prompts to webviews in this batch
        // =============================================================
        for task in batch {
            if !batch_created_labels.contains(&task.label) {
                continue;
            }

            if !is_scan_running(&state) {
                eprintln!("[Scan] Cancelled during prompt submission in batch {}", batch_number);
                break;
            }

            eprintln!("[Scan] Submitting prompt to: {}", task.label);

            let submit_result = {
                let mgr = manager.lock().await;
                mgr.submit_prompt(&app, &task.label, &task.platform, &task.prompt.text).await
            };

            // For google_ai_mode, wait and submit again (handles navigation)
            if task.platform == "google_ai_mode" {
                for _ in 0..8 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    if !is_scan_running(&state) {
                        break;
                    }
                }
                if is_scan_running(&state) {
                    let mgr = manager.lock().await;
                    let _ = mgr.submit_prompt(&app, &task.label, &task.platform, &task.prompt.text).await;
                }
            }

            if submit_result.is_ok() {
                batch_submitted_labels.push(task.label.clone());
                {
                    let mut scan = state.scan.lock();
                    if let Some(ps) = scan.platforms.get_mut(&task.platform) {
                        ps.submitted += 1;
                        ps.status = "submitting".to_string();
                    }
                }
                emit_progress_with_state(&app, &state);
            }
        }

        eprintln!("[Scan] Batch {}: Submitted {} prompts", batch_number, batch_submitted_labels.len());

        if batch_submitted_labels.is_empty() {
            eprintln!("[Scan] Batch {}: No prompts submitted, closing webviews and skipping", batch_number);
            let mut mgr = manager.lock().await;
            for label in &batch_created_labels {
                mgr.close_webview(&app, label);
            }
            {
                let mut labels = state.scan_webview_labels.lock();
                labels.retain(|l| !batch_created_labels.contains(l));
            }
            continue;
        }

        // =============================================================
        // BATCH PHASE 4: Wait for AI responses (~45 seconds)
        // =============================================================
        {
            let mut scan = state.scan.lock();
            scan.phase = "waiting".to_string();
            for platform_str in &selected_platforms {
                if let Some(ps) = scan.platforms.get_mut(platform_str) {
                    if ps.status != "skipped" {
                        ps.status = "waiting".to_string();
                    }
                }
            }
        }
        emit_progress_with_state(&app, &state);

        eprintln!("[Scan] Batch {}: Waiting for AI responses...", batch_number);

        const WAIT_SECONDS: usize = 45;
        for remaining in (0..=WAIT_SECONDS).rev() {
            let is_cancelled = {
                let scan = state.scan.lock();
                !scan.is_running
            };

            if is_cancelled {
                let mut mgr = manager.lock().await;
                for label in &batch_created_labels {
                    mgr.close_webview(&app, label);
                }
                return Err("Scan cancelled".to_string());
            }

            emit_progress_with_countdown(&app, &state, remaining);
            if remaining > 0 {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }

        // =============================================================
        // BATCH PHASE 5: Collect responses from this batch
        // =============================================================
        {
            let mut scan = state.scan.lock();
            scan.phase = "collecting".to_string();
        }
        emit_progress_with_state(&app, &state);

        eprintln!("[Scan] Batch {}: Collecting responses...", batch_number);

        // Spawn collection tasks for this batch only
        let mut collection_handles = Vec::new();

        for task in batch {
            if !batch_submitted_labels.contains(&task.label) {
                continue;
            }

            let app_clone = app.clone();
            let state_clone = state.clone();
            let manager_clone = manager.clone();
            let task_clone = task.clone();
            let brand_clone = brand.clone();
            let brand_domain_clone = brand_domain.clone();
            let domain_aliases_clone = domain_aliases.clone();
            let competitors_clone = competitors.clone();

            let handle = tokio::spawn(async move {
                eprintln!("[Scan] Collecting from webview: {}", task_clone.label);

                let collect_result = {
                    let mgr = manager_clone.lock().await;
                    mgr.collect_response(
                        &app_clone,
                        &task_clone.label,
                        &task_clone.platform,
                        &brand_clone,
                        brand_domain_clone.as_deref(),
                        domain_aliases_clone.as_deref(),
                        &competitors_clone,
                    ).await
                };

                // Close webview after collecting
                {
                    let mut mgr = manager_clone.lock().await;
                    mgr.close_webview(&app_clone, &task_clone.label);
                }
                {
                    let mut labels = state_clone.scan_webview_labels.lock();
                    labels.retain(|l| l != &task_clone.label);
                }

                match collect_result {
                    Ok(response) => ScanTaskResult {
                        webview_label: task_clone.label,
                        platform: task_clone.platform,
                        country_code: task_clone.country_code,
                        prompt: task_clone.prompt,
                        response: Some(response),
                        error: None,
                    },
                    Err(e) => ScanTaskResult {
                        webview_label: task_clone.label,
                        platform: task_clone.platform,
                        country_code: task_clone.country_code,
                        prompt: task_clone.prompt,
                        response: None,
                        error: Some(e),
                    },
                }
            });

            collection_handles.push(handle);
        }

        // Wait for batch collections to complete
        let collection_results = futures::future::join_all(collection_handles).await;

        // Process batch results and submit to API
        for result in collection_results {
            match result {
                Ok(scan_result) => {
                    if let Some(response) = scan_result.response {
                        total_collected += 1;
                        if response.brand_mentioned {
                            total_mentioned += 1;
                        }
                        if response.citation_present {
                            total_cited += 1;
                        }

                        // Update platform stats
                        {
                            let mut scan = state.scan.lock();
                            if let Some(ps) = scan.platforms.get_mut(&scan_result.platform) {
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
                                platform: scan_result.platform.clone(),
                                prompt_id: scan_result.prompt.id.clone(),
                                prompt_text: scan_result.prompt.text.clone(),
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
                                request_country: Some(scan_result.country_code.clone()),
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
                                    if resp.status().is_success() {
                                        eprintln!("[Scan] API submission successful for {}", scan_result.webview_label);
                                    } else {
                                        eprintln!("[Scan] API submission failed: {}", resp.status());
                                    }
                                }
                                Err(e) => eprintln!("[Scan] API request error: {}", e),
                            }
                        }
                    } else if let Some(error) = scan_result.error {
                        eprintln!("[Scan] Collection failed for {}: {}", scan_result.webview_label, error);
                        let mut scan = state.scan.lock();
                        if let Some(ps) = scan.platforms.get_mut(&scan_result.platform) {
                            ps.failed += 1;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[Scan] Collection task panicked: {}", e);
                }
            }
        }

        eprintln!("[Scan] Batch {}: Complete. Collected {} responses so far", batch_number, total_collected);
    }

    eprintln!("[Scan] All {} batches processed. Total collected: {} responses", num_batches, total_collected);

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
    {
        let mut mgr = manager.lock().await;
        mgr.close_all(&app);
    }
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
