use crate::{storage::{self, ProductConfig}, AppState};
use serde::Serialize;
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

#[derive(Serialize)]
pub struct ScheduleInfo {
    pub next_scan_hour: Option<u32>,
    pub scans_completed_today: u32,
    pub scans_total_today: u32,
    pub scheduled_times: Vec<u32>,
}

/// Get config for a specific product
#[tauri::command]
pub async fn get_product_config(product_id: String) -> Result<ProductConfig, String> {
    let config = storage::get_product_config(&product_id);
    println!("[Settings] get_product_config for {}: platforms={:?}, auto_run={}, scans_per_day={}",
        product_id, config.ready_platforms, config.auto_run_enabled, config.scans_per_day);
    Ok(config)
}

/// Calculate scheduled scan times based on config
fn calculate_scheduled_times(scans_per_day: u32, time_window_start: u32, time_window_end: u32) -> Vec<u32> {
    let start = time_window_start;
    let end = time_window_end;
    let scans = scans_per_day;

    // Handle edge case: if end <= start, return empty
    if end <= start || scans == 0 {
        return Vec::new();
    }

    let window_hours = end - start;

    if scans == 1 {
        // Single scan: run at the middle of the window
        return vec![start + window_hours / 2];
    }

    // Multiple scans: distribute evenly across the window
    let mut times = Vec::with_capacity(scans as usize);
    let interval = window_hours as f64 / (scans - 1) as f64;

    for i in 0..scans {
        let time = start as f64 + (i as f64 * interval);
        times.push(time.round() as u32);
    }

    times
}

/// Set config for a specific product
#[tauri::command]
pub async fn set_product_config(
    product_id: String,
    ready_platforms: Vec<String>,
    samples_per_prompt: u32,
    auto_run_enabled: bool,
    scans_per_day: u32,
    time_window_start: u32,
    time_window_end: u32,
    scan_countries: Option<Vec<String>>,
) -> Result<ProductConfig, String> {
    println!("[Settings] set_product_config for {}: platforms={:?}, auto_run={}, scans_per_day={}, window={}-{}, countries={:?}",
        product_id, ready_platforms, auto_run_enabled, scans_per_day, time_window_start, time_window_end, scan_countries);

    let existing = storage::get_product_config(&product_id);

    let new_scans_per_day = scans_per_day.max(1).min(24);
    let new_time_window_start = time_window_start.min(23);
    let new_time_window_end = time_window_end.min(23);

    // Use provided scan_countries or preserve existing
    let new_scan_countries = scan_countries.unwrap_or(existing.scan_countries);

    // Check if schedule-affecting settings changed
    let schedule_changed = existing.scans_per_day != new_scans_per_day
        || existing.time_window_start != new_time_window_start
        || existing.time_window_end != new_time_window_end;

    // Recalculate schedule if settings changed, otherwise keep existing
    let scheduled_times = if schedule_changed {
        let new_schedule = calculate_scheduled_times(new_scans_per_day, new_time_window_start, new_time_window_end);
        println!("[Settings] Schedule changed, recalculated times: {:?}", new_schedule);
        new_schedule
    } else {
        existing.scheduled_times
    };

    // If schedule changed, also reset scans_today to 0 since the schedule is new
    let scans_today = if schedule_changed { 0 } else { existing.scans_today };

    let config = ProductConfig {
        ready_platforms,
        samples_per_prompt: samples_per_prompt.max(1),
        auto_run_enabled,
        scans_per_day: new_scans_per_day,
        time_window_start: new_time_window_start,
        time_window_end: new_time_window_end,
        last_auto_scan_date: existing.last_auto_scan_date,
        scans_today,
        scheduled_times,
        scan_countries: new_scan_countries,
    };

    // Persist config to disk
    if let Err(e) = storage::update_product_config(&product_id, &config) {
        eprintln!("[Settings] Failed to persist product config: {}", e);
        return Err(format!("Failed to save config: {}", e));
    }

    println!("[Settings] Product config persisted successfully");
    Ok(config)
}

/// Get schedule info for a specific product
#[tauri::command]
pub async fn get_schedule_info(product_id: String) -> Result<ScheduleInfo, String> {
    let mut config = storage::get_product_config(&product_id);

    if !config.auto_run_enabled {
        return Ok(ScheduleInfo {
            next_scan_hour: None,
            scans_completed_today: 0,
            scans_total_today: 0,
            scheduled_times: Vec::new(),
        });
    }

    // Calculate schedule if empty (first run or new config)
    if config.scheduled_times.is_empty() && config.scans_per_day > 0 {
        config.scheduled_times = calculate_scheduled_times(
            config.scans_per_day,
            config.time_window_start,
            config.time_window_end,
        );
        // Persist the calculated schedule
        let _ = storage::update_product_config(&product_id, &config);
        println!("[Settings] Calculated initial schedule for {}: {:?}", product_id, config.scheduled_times);
    }

    let scans_completed = config.scans_today as usize;

    // Find the next scheduled scan
    let next_scan_hour = if scans_completed < config.scheduled_times.len() {
        Some(config.scheduled_times[scans_completed])
    } else {
        None // All scans completed for today
    };

    Ok(ScheduleInfo {
        next_scan_hour,
        scans_completed_today: config.scans_today,
        scans_total_today: config.scans_per_day,
        scheduled_times: config.scheduled_times,
    })
}

#[tauri::command]
pub async fn get_last_product_id(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Option<String>, String> {
    let product_id = state.last_product_id.lock().clone();
    Ok(product_id)
}

#[tauri::command]
pub async fn set_last_product_id(
    state: tauri::State<'_, Arc<AppState>>,
    product_id: Option<String>,
) -> Result<(), String> {
    println!("[Settings] set_last_product_id called: {:?}", product_id);

    {
        let mut last_product = state.last_product_id.lock();
        *last_product = product_id.clone();
    }

    // Persist to disk
    if let Err(e) = storage::update_last_product(product_id) {
        eprintln!("[Settings] Failed to persist last product: {}", e);
    } else {
        println!("[Settings] Last product persisted successfully");
    }

    Ok(())
}

#[tauri::command]
pub async fn get_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    let autostart_manager = app.autolaunch();
    autostart_manager
        .is_enabled()
        .map_err(|e| format!("Failed to get autostart status: {}", e))
}

#[tauri::command]
pub async fn set_autostart_enabled(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let autostart_manager = app.autolaunch();

    if enabled {
        autostart_manager
            .enable()
            .map_err(|e| format!("Failed to enable autostart: {}", e))?;
    } else {
        autostart_manager
            .disable()
            .map_err(|e| format!("Failed to disable autostart: {}", e))?;
    }

    // Return the new state
    autostart_manager
        .is_enabled()
        .map_err(|e| format!("Failed to verify autostart status: {}", e))
}
