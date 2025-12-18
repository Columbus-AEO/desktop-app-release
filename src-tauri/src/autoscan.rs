use crate::{storage, storage::ProductConfig, AppState, SUPABASE_URL, SUPABASE_ANON_KEY};
use chrono::Timelike;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;
use tauri::{AppHandle, Manager, async_runtime};
use tokio::time::{Duration, interval};

#[derive(Deserialize)]
struct StatusProduct {
    id: String,
}

#[derive(Deserialize)]
struct StatusResponse {
    products: Vec<StatusProduct>,
}

/// Fetch the list of product IDs the current user has access to
async fn get_user_product_ids(state: &Arc<AppState>) -> Result<HashSet<String>, String> {
    let token = {
        let auth = state.auth.lock();
        auth.access_token.clone().ok_or("Not authenticated")?
    };

    let client = reqwest::Client::new();
    let url = format!("{}/functions/v1/extension-status", SUPABASE_URL);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, error_text));
    }

    let status: StatusResponse = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    Ok(status.products.into_iter().map(|p| p.id).collect())
}

/// Start the auto-scan background scheduler
pub fn start_scheduler(app: AppHandle) {
    async_runtime::spawn(async move {
        // Wait a bit after startup before first check
        tokio::time::sleep(Duration::from_secs(10)).await;

        // Run initial check
        check_and_run_auto_scans(&app).await;

        // Then check every minute (to catch scheduled times accurately)
        let mut interval = interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            check_and_run_auto_scans(&app).await;
        }
    });
}

/// Calculate scheduled scan times for a product based on its config
/// The `product_index` and `total_products` parameters allow distributing scans across
/// multiple products to avoid all scans happening at the same time.
fn calculate_scheduled_times(config: &ProductConfig, product_index: usize, total_products: usize) -> Vec<u32> {
    let start = config.time_window_start;
    let end = config.time_window_end;
    let scans = config.scans_per_day;

    // Handle edge case: if end <= start, assume it wraps around midnight (not supported yet)
    // For now, require end > start
    if end <= start || scans == 0 {
        return Vec::new();
    }

    let window_hours = end - start;

    // Calculate total scans across all products to distribute evenly
    let total_scans = scans as usize * total_products.max(1);
    let product_offset = product_index as f64 / total_products.max(1) as f64;

    if scans == 1 {
        // Single scan: distribute across products by offset
        // Instead of all products at middle, spread them out
        if total_products > 1 {
            let offset_hours = (window_hours as f64 * product_offset).round() as u32;
            return vec![start + offset_hours];
        } else {
            return vec![start + window_hours / 2];
        }
    }

    // Multiple scans: distribute evenly across the window
    // For N scans, we divide the window into N intervals (not N-1, to leave room at edges)
    let mut times = Vec::with_capacity(scans as usize);
    let interval = window_hours as f64 / scans as f64;

    // Add a small offset based on product index to avoid all products scanning at once
    let product_offset_hours = if total_products > 1 {
        (interval / total_products as f64) * product_index as f64
    } else {
        0.0
    };

    for i in 0..scans {
        // Start from interval/2 to center scans in the window
        let time = start as f64 + (interval / 2.0) + (i as f64 * interval) + product_offset_hours;
        // Clamp to window bounds
        let clamped = time.round() as u32;
        if clamped >= start && clamped < end {
            times.push(clamped);
        } else if clamped >= end {
            times.push(end - 1); // Don't exceed end
        }
    }

    times
}

/// Check if auto-scans should run and execute them for all products
async fn check_and_run_auto_scans(app: &AppHandle) {
    let state = match app.try_state::<Arc<AppState>>() {
        Some(s) => s,
        None => {
            eprintln!("[AutoScan] AppState not available");
            return;
        }
    };

    // Ensure we have a valid (non-expired) auth token, refreshing if needed
    match crate::commands::auth::ensure_valid_token(&state).await {
        Ok(_) => {
            println!("[AutoScan] Auth token is valid");
        }
        Err(e) => {
            println!("[AutoScan] Not authenticated or token refresh failed: {}", e);
            return;
        }
    }

    // Fetch the list of product IDs the current user has access to
    let user_product_ids = match get_user_product_ids(&state).await {
        Ok(ids) => {
            println!("[AutoScan] User has access to {} products", ids.len());
            ids
        }
        Err(e) => {
            eprintln!("[AutoScan] Failed to fetch user products: {}", e);
            return;
        }
    };

    if user_product_ids.is_empty() {
        println!("[AutoScan] User has no products, skipping auto-scan check");
        return;
    }

    // Get current date and hour
    let now = chrono::Local::now();
    let today = now.format("%Y-%m-%d").to_string();
    let current_hour = now.hour();

    // Get all product configs (local storage)
    let product_configs = storage::get_all_product_configs();

    if product_configs.is_empty() {
        println!("[AutoScan] No product configurations found");
        return;
    }

    // Filter to only products the current user has access to and sort by ID for consistent ordering
    let mut user_product_configs: Vec<_> = product_configs
        .into_iter()
        .filter(|(id, _)| user_product_ids.contains(id))
        .collect();

    // Sort by product ID to ensure consistent ordering across runs
    user_product_configs.sort_by(|(a, _), (b, _)| a.cmp(b));

    let total_products = user_product_configs.len();
    println!("[AutoScan] Checking {} products for auto-scans (current hour: {})",
        total_products, current_hour);

    // Iterate over user's products only
    for (product_index, (product_id, mut config)) in user_product_configs.into_iter().enumerate() {
        // Skip if auto-run is disabled for this product
        if !config.auto_run_enabled {
            println!("[AutoScan] Product {} has auto-run disabled, skipping", product_id);
            continue;
        }

        // Skip if no platforms configured
        if config.ready_platforms.is_empty() {
            println!("[AutoScan] Product {} has no platforms configured, skipping", product_id);
            continue;
        }

        // Check if it's a new day - reset counter and recalculate schedule
        let is_new_day = config.last_auto_scan_date.as_ref() != Some(&today);

        // Calculate expected schedule for this product (to check if redistribution is needed)
        let expected_schedule = calculate_scheduled_times(&config, product_index, total_products);

        // Check if schedule needs redistribution (product count changed, or schedule is stale)
        let needs_redistribution = !is_new_day
            && !config.scheduled_times.is_empty()
            && config.scheduled_times != expected_schedule
            && config.scans_today == 0; // Only redistribute if no scans have run yet today

        if is_new_day {
            println!("[AutoScan] New day for product {}, resetting scan counter and schedule", product_id);
            config.last_auto_scan_date = Some(today.clone());
            config.scans_today = 0;
            config.scheduled_times = expected_schedule;
            let _ = storage::update_product_config(&product_id, &config);
            println!("[AutoScan] Scheduled times for product {} (index {}): {:?}", product_id, product_index, config.scheduled_times);
        } else if needs_redistribution {
            println!("[AutoScan] Redistributing schedule for product {} (index {}/{})", product_id, product_index, total_products);
            config.scheduled_times = expected_schedule;
            let _ = storage::update_product_config(&product_id, &config);
            println!("[AutoScan] New scheduled times for product {}: {:?}", product_id, config.scheduled_times);
        }

        // Recalculate schedule if empty (config might have changed)
        if config.scheduled_times.is_empty() {
            config.scheduled_times = calculate_scheduled_times(&config, product_index, total_products);
            let _ = storage::update_product_config(&product_id, &config);
            println!("[AutoScan] Recalculated schedule for product {}: {:?}", product_id, config.scheduled_times);
        }

        // Check if current hour matches any scheduled time that hasn't been run yet
        let scans_completed = config.scans_today as usize;
        let scheduled_times = &config.scheduled_times;

        // Find the next scheduled time we should run
        let next_scheduled_index = scans_completed;
        if next_scheduled_index >= scheduled_times.len() {
            println!("[AutoScan] Product {} has completed all {} scheduled scans today",
                product_id, scheduled_times.len());
            continue;
        }

        let next_scheduled_hour = scheduled_times[next_scheduled_index];

        // Check if it's time for the next scan (current hour >= scheduled hour)
        if current_hour < next_scheduled_hour {
            println!("[AutoScan] Product {}: next scan at {}:00, current hour is {} - waiting",
                product_id, next_scheduled_hour, current_hour);
            continue;
        }

        println!("[AutoScan] Product {}: time to run scan {} (scheduled for {}:00, current hour: {})",
            product_id, next_scheduled_index + 1, next_scheduled_hour, current_hour);

        // Check if a scan is already running
        {
            let scan = state.scan.lock();
            if scan.is_running {
                println!("[AutoScan] Scan already in progress, will retry next check");
                continue;
            }
        }

        // Run the scan
        println!("[AutoScan] Starting scheduled scan {}/{} for product {}",
            next_scheduled_index + 1, scheduled_times.len(), product_id);

        match run_auto_scan(
            app,
            &state,
            &product_id,
            config.samples_per_prompt as usize,
            &config.ready_platforms,
        ).await {
            Ok(_) => {
                println!("[AutoScan] Scheduled scan {} for product {} completed successfully",
                    next_scheduled_index + 1, product_id);

                // Reload config and increment the counter
                let mut updated_config = storage::get_product_config(&product_id);
                updated_config.scans_today += 1;
                updated_config.last_auto_scan_date = Some(today.clone());
                let _ = storage::update_product_config(&product_id, &updated_config);
            }
            Err(e) => {
                eprintln!("[AutoScan] Scheduled scan {} for product {} failed: {}",
                    next_scheduled_index + 1, product_id, e);
                // Still increment to avoid retrying failed scans indefinitely
                let mut updated_config = storage::get_product_config(&product_id);
                updated_config.scans_today += 1;
                updated_config.last_auto_scan_date = Some(today.clone());
                let _ = storage::update_product_config(&product_id, &updated_config);
            }
        }
    }

    println!("[AutoScan] Auto-scan check complete");
}

/// Run a single auto-scan by invoking the command through Tauri
async fn run_auto_scan(
    app: &AppHandle,
    state: &Arc<AppState>,
    product_id: &str,
    samples_per_prompt: usize,
    platforms: &[String],
) -> Result<(), String> {
    use crate::commands::scan::start_scan_internal;

    start_scan_internal(
        product_id.to_string(),
        Some(samples_per_prompt),
        Some(platforms.to_vec()),
        app.clone(),
        state.clone(),
    ).await?;

    // Wait for the scan to complete
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;

        let is_running = {
            let scan = state.scan.lock();
            scan.is_running
        };

        if !is_running {
            break;
        }
    }

    Ok(())
}
