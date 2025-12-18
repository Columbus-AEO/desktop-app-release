use crate::commands::api::get_platform_url;
use crate::storage;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

#[tauri::command]
pub async fn open_platform_login(platform: String, app: AppHandle) -> Result<(), String> {
    let url = get_platform_url(&platform)
        .ok_or_else(|| format!("Unknown platform: {}", platform))?;

    let label = format!("login-{}", platform);

    // Check if window already exists
    if let Some(window) = app.get_webview_window(&label) {
        // Focus existing window
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    // Create a visible webview for the user to log in
    let parsed_url: url::Url = url.parse().map_err(|_| "Invalid platform URL")?;
    WebviewWindowBuilder::new(&app, &label, WebviewUrl::External(parsed_url))
        .title(format!("Login to {} - Columbus", platform_display_name(&platform)))
        .inner_size(1200.0, 800.0)
        .visible(true)
        .center()
        .build()
        .map_err(|e| format!("Failed to open login window: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn open_url_in_browser(url: String, app: AppHandle) -> Result<(), String> {
    // Validate URL
    let parsed_url: url::Url = url.parse().map_err(|_| "Invalid URL")?;

    // Only allow http/https URLs
    if parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
        return Err("Only HTTP/HTTPS URLs are allowed".to_string());
    }

    let label = "columbus-browser";

    // Check if browser window already exists
    if let Some(window) = app.get_webview_window(label) {
        // Navigate to new URL
        let _ = window.navigate(parsed_url);
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    // Create a new browser window
    WebviewWindowBuilder::new(&app, label, WebviewUrl::External(parsed_url))
        .title("Columbus Browser")
        .inner_size(1200.0, 800.0)
        .visible(true)
        .center()
        .build()
        .map_err(|e| format!("Failed to open browser: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn close_platform_login(platform: String, app: AppHandle) -> Result<(), String> {
    let label = format!("login-{}", platform);

    if let Some(window) = app.get_webview_window(&label) {
        window.close().map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Open a country-specific login webview with isolated data directory
/// Uses instance-scoped storage for session isolation
#[tauri::command]
pub async fn open_country_login(
    country_code: String,
    platform: String,
    visible: bool,
    app: AppHandle,
) -> Result<(), String> {
    let url = get_platform_url(&platform)
        .ok_or_else(|| format!("Unknown platform: {}", platform))?;

    let label = format!("login-{}-{}", country_code.to_lowercase(), platform.to_lowercase());

    // Check if window already exists and is still valid
    if let Some(window) = app.get_webview_window(&label) {
        // Check if window is still visible/valid by trying to get its visibility
        match window.is_visible() {
            Ok(true) => {
                // Window exists and is visible, just focus it
                window.set_focus().map_err(|e| e.to_string())?;
                return Ok(());
            }
            Ok(false) => {
                // Window exists but is hidden, show and focus it
                let _ = window.show();
                window.set_focus().map_err(|e| e.to_string())?;
                return Ok(());
            }
            Err(_) => {
                // Window reference is stale, destroy it and create new one
                let _ = window.destroy();
            }
        }
    }

    // Get instance-scoped data directory for this country/platform
    let instance_id = storage::get_active_instance_id();
    let data_dir: PathBuf = if instance_id.is_empty() {
        storage::ensure_webview_data_dir(&country_code, &platform)?
    } else {
        storage::ensure_instance_webview_data_dir(&instance_id, &country_code, &platform)?
    };

    println!(
        "[Platform] Opening country login: country={}, platform={}, instance={}, data_dir={:?}",
        country_code, platform, instance_id, data_dir
    );

    // Create a visible webview for the user to log in with isolated storage
    let parsed_url: url::Url = url.parse().map_err(|_| "Invalid platform URL")?;

    let mut builder = WebviewWindowBuilder::new(&app, &label, WebviewUrl::External(parsed_url))
        .title(format!(
            "Login to {} ({}) - Columbus",
            platform_display_name(&platform),
            country_code.to_uppercase()
        ))
        .inner_size(1200.0, 800.0)
        .visible(visible)
        .center();

    // Add data directory for cookie isolation (Windows only)
    #[cfg(target_os = "windows")]
    {
        builder = builder.data_directory(data_dir);
    }

    let window = builder
        .build()
        .map_err(|e| format!("Failed to open login window: {}", e))?;

    // Handle window close event - destroy the webview when user clicks X
    let app_handle = app.clone();
    let window_label = label.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            // Prevent default close behavior
            api.prevent_close();
            // Destroy the window to fully release WebView2 resources
            if let Some(win) = app_handle.get_webview_window(&window_label) {
                let _ = win.destroy();
            }
        }
    });

    Ok(())
}

/// Close a country-specific login webview
#[tauri::command]
pub async fn close_country_login(
    country_code: String,
    platform: String,
    app: AppHandle,
) -> Result<(), String> {
    let label = format!("login-{}-{}", country_code.to_lowercase(), platform.to_lowercase());

    if let Some(window) = app.get_webview_window(&label) {
        // Use destroy() instead of close() to fully remove the window
        window.destroy().map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn platform_display_name(platform: &str) -> &str {
    match platform {
        "chatgpt" => "ChatGPT",
        "claude" => "Claude",
        "gemini" => "Gemini",
        "perplexity" => "Perplexity",
        _ => platform,
    }
}

/// Open a magic link URL in a country-specific webview with isolated data directory
/// This allows users to paste authentication URLs (e.g., email magic links) that will
/// be opened in the correct session context for that country
#[tauri::command]
pub async fn open_magic_link(
    country_code: String,
    url: String,
    app: AppHandle,
) -> Result<(), String> {
    // Validate URL
    let parsed_url: url::Url = url.parse().map_err(|_| "Invalid URL")?;

    // Only allow http/https URLs
    if parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
        return Err("Only HTTP/HTTPS URLs are allowed".to_string());
    }

    // Use a generic label for magic link webviews (one per country)
    let label = format!("magic-link-{}", country_code.to_lowercase());

    // Check if window already exists
    if let Some(window) = app.get_webview_window(&label) {
        // Navigate to the new URL
        let _ = window.navigate(parsed_url.clone());
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    // Get instance-scoped data directory for this country (use "magic" as platform)
    let instance_id = storage::get_active_instance_id();
    let data_dir: PathBuf = if instance_id.is_empty() {
        storage::ensure_webview_data_dir(&country_code, "magic")?
    } else {
        storage::ensure_instance_webview_data_dir(&instance_id, &country_code, "magic")?
    };

    println!(
        "[Platform] Opening magic link: country={}, instance={}, url={}",
        country_code, instance_id, &url[..url.len().min(50)]
    );

    // Create a visible webview with isolated storage
    let mut builder = WebviewWindowBuilder::new(&app, &label, WebviewUrl::External(parsed_url))
        .title(format!(
            "Magic Link ({}) - Columbus",
            country_code.to_uppercase()
        ))
        .inner_size(1200.0, 800.0)
        .visible(true)
        .center();

    // Add data directory for cookie isolation (Windows only)
    #[cfg(target_os = "windows")]
    {
        builder = builder.data_directory(data_dir);
    }

    let window = builder
        .build()
        .map_err(|e| format!("Failed to open magic link window: {}", e))?;

    // Handle window close event - destroy the webview when user clicks X
    let app_handle = app.clone();
    let window_label = label.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            if let Some(win) = app_handle.get_webview_window(&window_label) {
                let _ = win.destroy();
            }
        }
    });

    Ok(())
}
