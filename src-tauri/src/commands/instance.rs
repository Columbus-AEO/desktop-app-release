//! Instance management commands for multi-instance support
//!
//! Each instance has separate platform credentials and browser sessions,
//! allowing users to authenticate with different accounts on AI platforms.

use crate::storage::{self, Instance};
use crate::AppState;
use std::sync::Arc;
use tauri::State;

/// List all instances
#[tauri::command]
pub fn list_instances() -> Vec<Instance> {
    storage::get_all_instances()
}

/// Get the currently active instance
#[tauri::command]
pub fn get_active_instance() -> Option<Instance> {
    storage::get_active_instance()
}

/// Get the active instance ID
#[tauri::command]
pub fn get_active_instance_id(state: State<'_, Arc<AppState>>) -> String {
    state.active_instance_id.lock().clone()
}

/// Create a new instance
#[tauri::command]
pub fn create_instance(name: Option<String>) -> Result<Instance, String> {
    storage::create_instance(name)
}

/// Delete an instance (cannot delete the default instance)
#[tauri::command]
pub fn delete_instance(instance_id: String) -> Result<(), String> {
    storage::delete_instance(&instance_id)
}

/// Rename an instance
#[tauri::command]
pub fn rename_instance(instance_id: String, new_name: String) -> Result<(), String> {
    storage::rename_instance(&instance_id, &new_name)
}

/// Switch to a different instance
#[tauri::command]
pub fn switch_instance(
    instance_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    // Update persisted state
    storage::set_active_instance_id(&instance_id)?;

    // Update in-memory state
    let mut active_id = state.active_instance_id.lock();
    *active_id = instance_id.clone();

    println!("[Instance] Switched to instance: {}", instance_id);
    Ok(())
}

/// Get instance data summary (for UI display)
#[tauri::command]
pub fn get_instance_summary(instance_id: String) -> InstanceSummary {
    let platforms_with_creds = storage::get_instance_platforms_with_credentials(&instance_id);
    let auth_statuses = storage::get_instance_all_country_platform_auth(&instance_id);
    let onboarding_completed = storage::is_instance_onboarding_completed(&instance_id);

    let authenticated_count = auth_statuses
        .values()
        .filter(|a| a.is_authenticated)
        .count();

    InstanceSummary {
        credentials_count: platforms_with_creds.len(),
        platforms_with_credentials: platforms_with_creds,
        authenticated_count,
        onboarding_completed,
    }
}

#[derive(serde::Serialize)]
pub struct InstanceSummary {
    pub credentials_count: usize,
    pub platforms_with_credentials: Vec<String>,
    pub authenticated_count: usize,
    pub onboarding_completed: bool,
}

/// Check if onboarding has been completed for the active instance
#[tauri::command]
pub fn is_onboarding_completed() -> bool {
    let instance_id = storage::get_active_instance_id();
    if !instance_id.is_empty() {
        storage::is_instance_onboarding_completed(&instance_id)
    } else {
        storage::is_onboarding_completed()
    }
}

/// Mark onboarding as completed for the active instance
#[tauri::command]
pub fn set_onboarding_completed(completed: bool) -> Result<(), String> {
    let instance_id = storage::get_active_instance_id();
    if !instance_id.is_empty() {
        storage::set_instance_onboarding_completed(&instance_id, completed)
    } else {
        storage::set_onboarding_completed(completed)
    }
}
