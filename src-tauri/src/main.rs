// Empêche l'ouverture d'une fenêtre terminal (console) au démarrage sur Windows.
// Sans cette directive, Windows ouvre une fenêtre cmd noire derrière l'application.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod auto_update;
mod clamav;
mod commands;
mod models;
mod paths;
mod quarantine;
mod storage;
mod watcher;

use std::sync::Mutex;

use anyhow::Result;
use models::AppConfig;
use storage::read_app_config;
use storage::StoragePaths;
use tauri::Manager;
use watcher::RealtimeProtection;
use auto_update::AutoUpdate;

pub struct AppState {
    pub app: tauri::AppHandle,
    pub storage: StoragePaths,
    pub config: Mutex<AppConfig>,
    pub realtime: Mutex<Option<RealtimeProtection>>,
    pub auto_update: Mutex<Option<AutoUpdate>>,
}

impl AppState {
    fn bootstrap(app: tauri::AppHandle) -> Result<Self> {
        let storage = StoragePaths::new(&app)?;
        let config = read_app_config(&storage)?;

        Ok(Self {
            app,
            storage,
            config: Mutex::new(config),
            realtime: Mutex::new(None),
            auto_update: Mutex::new(None),
        })
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let state = AppState::bootstrap(handle)?;

            let config = state.config.lock().unwrap().clone();

            // Démarrer l'auto-update des signatures si activé
            if config.auto_update_signatures {
                let mut auto_update = AutoUpdate::new(config.auto_update_interval_hours);
                let _ = auto_update.start(state.app.clone(), state.storage.clone());
                *state.auto_update.lock().unwrap() = Some(auto_update);
            }

            // Démarrer la protection temps réel si activé
            if config.auto_start_realtime {
                if let Ok(guard) = RealtimeProtection::start(
                    state.app.clone(),
                    state.storage.clone(),
                    config,
                ) {
                    *state.realtime.lock().unwrap() = Some(guard);
                }
            }

            app.manage(state);

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard_data,
            commands::get_scan_history,
            commands::get_quarantine_items,
            commands::restore_quarantine_item,
            commands::delete_quarantine_item,
            commands::scan_target,
            commands::update_signatures,
            commands::get_system_status,
            commands::get_app_config,
            commands::save_app_config,
            commands::start_realtime_protection,
            commands::stop_realtime_protection,
            commands::get_realtime_status,
            commands::get_auto_update_status,
            commands::start_auto_update,
            commands::stop_auto_update
        ])
        .run(tauri::generate_context!())
        .expect("Erreur au démarrage de l'application Tauri");
}
