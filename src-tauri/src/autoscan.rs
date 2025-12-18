use crate::{storage, storage::ProductConfig, AppState};
use chrono::Timelike;
use std::sync::Arc;
use tauri::{AppHandle, Manager, async_runtime};
use tokio::time::{Duration, interval};

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
fn calculate_scheduled_times(config: &ProductConfig) -> Vec<u32> {
    let start = config.time_window_start;
    let end = config.time_window_end;
    let scans = config.scans_per_day;

    // Handle edge case: if end <= start, assume it wraps around midnight (not supported yet)
    // For now, require end > start
    if end <= start || scans == 0 {
        return Vec::new();
    }

    let window_hours = end - start;

    if scans == 1 {
        // Single scan: run at the middle of the window
        return vec![start + window_hours / 2];
    }

    // Multiple scans: distribute evenly across the window
    // For N scans, we divide the window into N-1 intervals
    let mut times = Vec::with_capacity(scans as usize);
    let interval = window_hours as f64 / (scans - 1) as f64;

    for i in 0..scans {
        let time = start as f64 + (i as f64 * interval);
        times.push(time.round() as u32);
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

    // Get current date and hour
    let now = chrono::Local::now();
    let today = now.format("%Y-%m-%d").to_string();
    let current_hour = now.hour();

    // Get all product configs
    let product_configs = storage::get_all_product_configs();

    if product_configs.is_empty() {
        println!("[AutoScan] No product configurations found");
        return;
    }

    println!("[AutoScan] Checking {} products for auto-scans (current hour: {})",
        product_configs.len(), current_hour);

    // Iterate over all products
    for (product_id, mut config) in product_configs {
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
        if is_new_day {
            println!("[AutoScan] New day for product {}, resetting scan counter and schedule", product_id);
            config.last_auto_scan_date = Some(today.clone());
            config.scans_today = 0;
            config.scheduled_times = calculate_scheduled_times(&config);
            let _ = storage::update_product_config(&product_id, &config);
            println!("[AutoScan] Scheduled times for product {}: {:?}", product_id, config.scheduled_times);
        }

        // Recalculate schedule if empty (config might have changed)
        if config.scheduled_times.is_empty() {
            config.scheduled_times = calculate_scheduled_times(&config);
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
