use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent, TrayIcon},
    Manager, WindowEvent,
};

mod autoscan;
mod commands;
mod proxy_server;
mod storage;
mod webview;

pub use commands::*;
pub use storage::*;
pub use webview::*;

// Configuration
pub const SUPABASE_URL: &str = "https://yvhzxuoqodutmllfhcsa.supabase.co";
pub const SUPABASE_ANON_KEY: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6Inl2aHp4dW9xb2R1dG1sbGZoY3NhIiwicm9sZSI6ImFub24iLCJpYXQiOjE3NjM5MDMwOTIsImV4cCI6MjA3OTQ3OTA5Mn0.UxDcyOAGSGKBW26ElQXAyiVC6GRicphIVcrMs8tdkRI";

// Platform URLs (fallback - primary source is ai_platforms table in database)
pub const PLATFORM_URLS: &[(&str, &str)] = &[
    ("chatgpt", "https://chatgpt.com/"),
    ("claude", "https://claude.ai/new"),
    ("gemini", "https://gemini.google.com/app"),
    ("perplexity", "https://www.perplexity.ai/"),
    ("google_aio", "https://www.google.com/"),
];

// Application state
pub struct AppState {
    pub auth: Mutex<AuthState>,
    pub scan: Mutex<ScanState>,
    pub last_product_id: Mutex<Option<String>>,
    /// Currently active instance ID for multi-instance support
    pub active_instance_id: Mutex<String>,
    /// Track webview labels created during scan for cleanup on cancel
    pub scan_webview_labels: Mutex<Vec<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        // Load persisted state on startup
        let persisted = storage::load_state();

        // Restore auth state from persisted data
        let auth_state = match &persisted.auth {
            Some(a) => {
                println!("[AppState] Loaded persisted auth for user: {}", a.user_email);
                println!("[AppState] Token expires at: {}, now: {}", a.expires_at, chrono::Utc::now().timestamp());
                AuthState::from(a)
            }
            None => {
                println!("[AppState] No persisted auth found");
                AuthState::default()
            }
        };

        // Get active instance ID (will be set after migration runs)
        let active_instance = storage::get_active_instance_id();
        println!("[AppState] Active instance ID: {}", if active_instance.is_empty() { "<none>" } else { &active_instance });

        Self {
            auth: Mutex::new(auth_state),
            scan: Mutex::new(ScanState::default()),
            last_product_id: Mutex::new(persisted.last_product_id),
            active_instance_id: Mutex::new(active_instance),
            scan_webview_labels: Mutex::new(Vec::new()),
        }
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct AuthState {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub user: Option<User>,
    pub expires_at: Option<i64>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct ScanState {
    pub is_running: bool,
    pub phase: String,
    pub scan_session_id: Option<String>,
    pub product_id: Option<String>,
    pub platforms: HashMap<String, PlatformState>,
    pub total_prompts: usize,
    pub completed_prompts: usize,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct PlatformState {
    pub status: String,  // pending, submitting, waiting, collecting, complete, skipped
    pub total: usize,
    pub submitted: usize,
    pub collected: usize,
    pub failed: usize,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub brand: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    pub id: String,
    pub text: String,
    pub category: Option<String>,
    #[serde(default)]
    pub target_regions: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub phase: String,
    pub current: usize,
    pub total: usize,
    pub platforms: HashMap<String, PlatformState>,
}

// CompetitorDetail for tracking competitor positions
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CompetitorDetailResult {
    pub name: String,
    pub position: Option<i32>,
    pub sentiment: String,
}

// ScanResult uses camelCase to match the Supabase API expected format
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub product_id: String,
    pub scan_session_id: String,
    pub platform: String,
    pub prompt_id: String,
    pub prompt_text: String,
    pub response_text: String,
    pub brand_mentioned: bool,
    pub citation_present: bool,
    pub position: Option<i32>,
    pub sentiment: String,
    pub competitor_mentions: Vec<String>,
    pub competitor_details: Vec<CompetitorDetailResult>,
    pub citations: Vec<Citation>,
    pub credits_exhausted: bool,
    pub chat_url: Option<String>,
    pub request_country: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Citation {
    pub url: String,
    pub title: String,
    pub position: i32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ScanComplete {
    pub total_prompts: usize,
    pub successful_prompts: usize,
    pub mention_rate: f64,
    pub citation_rate: f64,
}

/// Update tray icon and tooltip based on scan state
pub fn update_tray_status(app: &tauri::AppHandle, is_scanning: bool) {
    println!("[Tray] update_tray_status called, is_scanning: {}", is_scanning);

    if let Some(tray) = app.try_state::<TrayIcon>() {
        let tooltip = if is_scanning {
            "Columbus - Scanning..."
        } else {
            "Columbus - AI Brand Monitor"
        };
        let _ = tray.set_tooltip(Some(tooltip));

        // Update tray icon
        if is_scanning {
            println!("[Tray] Creating scanning icon...");
            match create_scanning_icon() {
                Some(scanning_icon) => {
                    println!("[Tray] Scanning icon created, setting...");
                    match tray.set_icon(Some(scanning_icon)) {
                        Ok(_) => println!("[Tray] Scanning icon set successfully"),
                        Err(e) => println!("[Tray] Failed to set scanning icon: {:?}", e),
                    }
                }
                None => println!("[Tray] Failed to create scanning icon"),
            }
        } else {
            // Restore default icon
            println!("[Tray] Restoring default icon...");
            if let Some(default_icon) = app.default_window_icon().cloned() {
                match tray.set_icon(Some(default_icon)) {
                    Ok(_) => println!("[Tray] Default icon restored"),
                    Err(e) => println!("[Tray] Failed to restore default icon: {:?}", e),
                }
            }
        }
    } else {
        println!("[Tray] No tray icon state found");
    }
}

/// Create a scanning indicator icon (original icon with green dot)
fn create_scanning_icon() -> Option<tauri::image::Image<'static>> {
    // Load the 32x32 icon
    let icon_bytes = include_bytes!("../icons/32x32.png");

    let img = image::load_from_memory(icon_bytes).ok()?;
    let mut rgba = img.to_rgba8();

    let (width, height) = rgba.dimensions();

    // Draw a green dot in bottom-right corner (indicator)
    let dot_radius = 6u32;
    let dot_center_x = width - dot_radius - 2;
    let dot_center_y = height - dot_radius - 2;

    for y in 0..height {
        for x in 0..width {
            let dx = x as i32 - dot_center_x as i32;
            let dy = y as i32 - dot_center_y as i32;
            let dist_sq = dx * dx + dy * dy;

            if dist_sq <= (dot_radius as i32 * dot_radius as i32) {
                // Green dot
                rgba.put_pixel(x, y, image::Rgba([0, 200, 0, 255]));
            } else if dist_sq <= ((dot_radius + 1) as i32 * (dot_radius + 1) as i32) {
                // Dark border
                rgba.put_pixel(x, y, image::Rgba([0, 100, 0, 255]));
            }
        }
    }

    let raw = rgba.into_raw();
    Some(tauri::image::Image::new_owned(raw, width, height))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    println!("[Columbus] Starting application...");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // When a second instance tries to run, show the existing window
            println!("[Columbus] Second instance detected, focusing existing window");
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(Arc::new(AppState::default()))
        .invoke_handler(tauri::generate_handler![
            commands::auth::login,
            commands::auth::logout,
            commands::auth::get_auth_status,
            commands::auth::login_with_google,
            commands::api::get_status,
            commands::api::get_prompts,
            commands::api::submit_scan_result,
            commands::api::finalize_scan,
            commands::api::get_ai_platforms,
            commands::api::get_prompt_target_regions,
            commands::api::get_prompt_regions,
            commands::api::fetch_extension_prompts,
            commands::api::check_daily_usage,
            commands::scan::start_scan,
            commands::scan::cancel_scan,
            commands::scan::get_scan_progress,
            commands::scan::is_scan_running,
            commands::platform::open_platform_login,
            commands::platform::close_platform_login,
            commands::platform::open_url_in_browser,
            commands::platform::open_country_login,
            commands::platform::close_country_login,
            commands::platform::open_magic_link,
            commands::settings::get_product_config,
            commands::settings::set_product_config,
            commands::settings::get_schedule_info,
            commands::settings::get_last_product_id,
            commands::settings::set_last_product_id,
            commands::settings::get_autostart_enabled,
            commands::settings::set_autostart_enabled,
            commands::proxy::fetch_proxy_config,
            commands::proxy::has_proxy_config,
            commands::proxy::clear_proxy_config,
            commands::proxy::get_country_auth_statuses,
            commands::proxy::get_country_platform_auth,
            commands::proxy::set_country_platform_auth,
            commands::proxy::set_platform_auth_status,
            commands::proxy::get_authenticated_platforms,
            commands::proxy::get_authenticated_countries,
            commands::proxy::set_product_scan_countries,
            commands::proxy::get_product_scan_countries,
            commands::proxy::get_static_proxies,
            commands::proxy::get_static_proxies_by_country,
            commands::proxy::get_static_proxies_for_country,
            commands::proxy::add_static_proxy,
            commands::proxy::remove_static_proxy,
            commands::proxy::test_static_proxy,
            commands::proxy::has_static_proxy,
            commands::proxy::get_configured_proxy_countries,
            // Instance management
            commands::instance::list_instances,
            commands::instance::get_active_instance,
            commands::instance::get_active_instance_id,
            commands::instance::create_instance,
            commands::instance::delete_instance,
            commands::instance::rename_instance,
            commands::instance::switch_instance,
            commands::instance::get_instance_summary,
        ])
        .setup(|app| {
            println!("[Columbus] Setup starting...");

            // Run multi-instance migration (safe to call multiple times)
            if let Err(e) = storage::migrate_to_multi_instance() {
                eprintln!("[Columbus] Migration error (non-fatal): {}", e);
            }

            // Build system tray menu
            let show_item = MenuItem::with_id(app, "show", "Show Columbus", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            // Create system tray if we have an icon
            if let Some(icon) = app.default_window_icon().cloned() {
                println!("[Columbus] Creating system tray...");
                let tray_result = TrayIconBuilder::new()
                    .icon(icon)
                    .menu(&menu)
                    .tooltip("Columbus - AI Brand Monitor")
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    })
                    .build(app);

                match &tray_result {
                    Ok(_) => println!("[Columbus] System tray created successfully"),
                    Err(e) => {
                        eprintln!("[Columbus] ERROR creating system tray: {}", e);
                    }
                }

                // Store tray handle to keep it alive (important!)
                if let Ok(tray) = tray_result {
                    app.manage(tray);
                }
            } else {
                eprintln!("[Columbus] No window icon available, skipping system tray");
            }

            // Check if started minimized (via --minimized arg)
            let args: Vec<String> = std::env::args().collect();
            if args.contains(&"--minimized".to_string()) {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            // Start the auto-scan scheduler
            println!("[Columbus] Starting auto-scan scheduler...");
            autoscan::start_scheduler(app.handle().clone());

            println!("[Columbus] Setup complete");
            Ok(())
        })
        .on_window_event(|window, event| {
            // Hide window instead of closing when user clicks X
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
