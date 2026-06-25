use std::sync::MutexGuard;

use tauri::State;

use crate::auto_update::AutoUpdate;
use crate::clamav;
use crate::models::{
    AppConfig, DashboardStats, QuarantineEntry, RealtimeStatus, ScanReport, ScanRequest,
    SystemStatus, UpdateReport,
};
use crate::quarantine;
use crate::storage::{read_scan_history, write_app_config};
use crate::watcher::RealtimeProtection;
use crate::AppState;

#[tauri::command]
pub fn get_dashboard_data(state: State<'_, AppState>) -> Result<DashboardStats, String> {
    let config = config_snapshot(&state)?;
    clamav::build_dashboard(&state.app, &state.storage, realtime_enabled(&state), &config)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_scan_history(state: State<'_, AppState>) -> Result<Vec<ScanReport>, String> {
    read_scan_history(&state.storage).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_quarantine_items(state: State<'_, AppState>) -> Result<Vec<QuarantineEntry>, String> {
    quarantine::list_items(&state.storage).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn restore_quarantine_item(
    id: String,
    state: State<'_, AppState>,
) -> Result<QuarantineEntry, String> {
    quarantine::restore_item(&id, &state.storage).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_quarantine_item(id: String, state: State<'_, AppState>) -> Result<(), String> {
    quarantine::delete_item(&id, &state.storage).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn scan_target(
    request: ScanRequest,
    state: State<'_, AppState>,
) -> Result<ScanReport, String> {
    let config = config_snapshot(&state)?;
    clamav::execute_scan(&state.app, &state.storage, &config, request)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_signatures(state: State<'_, AppState>) -> Result<UpdateReport, String> {
    clamav::run_signature_update(&state.app, &state.storage).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_system_status(state: State<'_, AppState>) -> Result<SystemStatus, String> {
    Ok(clamav::get_system_status(&state.app, &state.storage))
}

#[tauri::command]
pub fn get_app_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    config_snapshot(&state)
}

#[tauri::command]
pub fn save_app_config(config: AppConfig, state: State<'_, AppState>) -> Result<AppConfig, String> {
    let mut sanitized = config;
    sanitized.watched_paths = crate::paths::normalize_targets(&sanitized.watched_paths)
        .iter()
        .map(|path| path.display().to_string())
        .collect();

    write_app_config(&state.storage, &sanitized).map_err(|error| error.to_string())?;

    {
        let mut guard = state.config.lock().map_err(|error| error.to_string())?;
        *guard = sanitized.clone();
    }

    restart_realtime_if_needed(&state)?;
    Ok(sanitized)
}

#[tauri::command]
pub fn start_realtime_protection(
    auto_quarantine: Option<bool>,
    use_clamd: Option<bool>,
    state: State<'_, AppState>,
) -> Result<RealtimeStatus, String> {
    let mut realtime = realtime_lock(&state).map_err(|error| error.to_string())?;

    if let Some(current) = realtime.as_ref() {
        return Ok(current.into_status());
    }

    let mut config = config_snapshot(&state)?;
    if let Some(value) = auto_quarantine {
        config.auto_quarantine = value;
    }
    if let Some(value) = use_clamd {
        config.use_clamd_by_default = value;
    }

    let guard = RealtimeProtection::start(state.app.clone(), state.storage.clone(), config)
        .map_err(|error| error.to_string())?;
    let status = guard.into_status();
    *realtime = Some(guard);
    Ok(status)
}

#[tauri::command]
pub fn stop_realtime_protection(state: State<'_, AppState>) -> Result<RealtimeStatus, String> {
    let mut realtime = realtime_lock(&state).map_err(|error| error.to_string())?;

    if let Some(current) = realtime.take() {
        let current_status = current.into_status();
        let status = RealtimeStatus {
            enabled: false,
            watched_paths: current_status.watched_paths,
            auto_quarantine: current_status.auto_quarantine,
            using_clamd: current_status.using_clamd,
            downloads_protected: current_status.downloads_protected,
        };
        current.stop();
        Ok(status)
    } else {
        let config = config_snapshot(&state)?;
        Ok(RealtimeStatus {
            enabled: false,
            watched_paths: crate::paths::watch_paths_from_config(
                &config.watched_paths,
                config.auto_scan_downloads,
            )
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
            auto_quarantine: config.auto_quarantine,
            using_clamd: config.use_clamd_by_default,
            downloads_protected: config.auto_scan_downloads,
        })
    }
}

#[tauri::command]
pub fn get_realtime_status(state: State<'_, AppState>) -> Result<RealtimeStatus, String> {
    let realtime = realtime_lock(&state).map_err(|error| error.to_string())?;

    if let Some(current) = realtime.as_ref() {
        Ok(current.into_status())
    } else {
        Ok(RealtimeStatus {
            enabled: false,
            watched_paths: crate::paths::watch_paths_from_config(
                &config_snapshot(&state)?.watched_paths,
                downloads_protected_default(&state)?,
            )
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
            auto_quarantine: auto_quarantine_default(&state)?,
            using_clamd: use_clamd_default(&state)?,
            downloads_protected: downloads_protected_default(&state)?,
        })
    }
}

#[tauri::command]
pub fn get_auto_update_status(state: State<'_, AppState>) -> Result<bool, String> {
    let auto_update = state.auto_update.lock().map_err(|error| error.to_string())?;
    Ok(auto_update.as_ref().map(|au| au.is_running()).unwrap_or(false))
}

#[tauri::command]
pub fn start_auto_update(state: State<'_, AppState>) -> Result<bool, String> {
    let config = config_snapshot(&state)?;
    let mut auto_update = state.auto_update.lock().map_err(|error| error.to_string())?;
    
    if auto_update.is_some() {
        return Ok(true);
    }
    
    let mut updater = AutoUpdate::new(config.auto_update_interval_hours);
    updater.start(state.app.clone(), state.storage.clone()).map_err(|error| error.to_string())?;
    *auto_update = Some(updater);
    
    Ok(true)
}

#[tauri::command]
pub fn stop_auto_update(state: State<'_, AppState>) -> Result<bool, String> {
    let mut auto_update = state.auto_update.lock().map_err(|error| error.to_string())?;
    
    if let Some(mut updater) = auto_update.take() {
        updater.stop();
    }
    
    Ok(true)
}

fn realtime_enabled(state: &State<'_, AppState>) -> bool {
    state
        .realtime
        .lock()
        .map(|guard| guard.is_some())
        .unwrap_or(false)
}

fn realtime_lock<'a>(
    state: &'a State<'_, AppState>,
) -> Result<MutexGuard<'a, Option<RealtimeProtection>>, std::sync::PoisonError<MutexGuard<'a, Option<RealtimeProtection>>>> {
    state.realtime.lock()
}

fn config_snapshot(state: &State<'_, AppState>) -> Result<AppConfig, String> {
    state
        .config
        .lock()
        .map(|guard| guard.clone())
        .map_err(|error| error.to_string())
}

fn auto_quarantine_default(state: &State<'_, AppState>) -> Result<bool, String> {
    Ok(config_snapshot(state)?.auto_quarantine)
}

fn use_clamd_default(state: &State<'_, AppState>) -> Result<bool, String> {
    Ok(config_snapshot(state)?.use_clamd_by_default)
}

fn downloads_protected_default(state: &State<'_, AppState>) -> Result<bool, String> {
    Ok(config_snapshot(state)?.auto_scan_downloads)
}

fn restart_realtime_if_needed(state: &State<'_, AppState>) -> Result<(), String> {
    let was_enabled = realtime_enabled(state);
    if !was_enabled {
        return Ok(());
    }

    {
        let mut realtime = realtime_lock(state).map_err(|error| error.to_string())?;
        if let Some(current) = realtime.take() {
            current.stop();
        }
    }

    let config = config_snapshot(state)?;
    let guard = RealtimeProtection::start(state.app.clone(), state.storage.clone(), config)
        .map_err(|error| error.to_string())?;
    let mut realtime = realtime_lock(state).map_err(|error| error.to_string())?;
    *realtime = Some(guard);
    Ok(())
}
