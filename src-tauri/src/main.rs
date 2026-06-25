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
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WebviewWindow};
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

/// Affiche la fenêtre principale et la met au premier plan.
fn show_window(window: &WebviewWindow) {
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

fn main() {
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let state = AppState::bootstrap(handle.clone())?;

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

            // ── System Tray ──────────────────────────────────────────────────
            let open_item = MenuItem::with_id(app, "open", "Ouvrir ClamAVClient", true, None::<&str>)?;
            let toggle_item = MenuItem::with_id(app, "toggle_rt", "Protection temps réel : ON/OFF", true, None::<&str>)?;
            let quit_item  = MenuItem::with_id(app, "quit", "Quitter", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&open_item, &toggle_item, &quit_item])?;

            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("ClamAVClient — Protection active")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event({
                    let handle = handle.clone();
                    move |_tray, event| {
                        let window = handle.get_webview_window("main");
                        match event.id().as_ref() {
                            "open" => {
                                if let Some(w) = window {
                                    show_window(&w);
                                }
                            }
                            "toggle_rt" => {
                                // Déléguer à la commande Tauri existante via état partagé
                                if let Some(state) = handle.try_state::<AppState>() {
                                    let mut guard = state.realtime.lock().unwrap();
                                    if guard.is_some() {
                                        *guard = None; // stoppe la protection
                                    } else {
                                        let cfg = state.config.lock().unwrap().clone();
                                        if let Ok(rt) = RealtimeProtection::start(
                                            handle.clone(),
                                            state.storage.clone(),
                                            cfg,
                                        ) {
                                            *guard = Some(rt);
                                        }
                                    }
                                }
                            }
                            "quit" => {
                                handle.exit(0);
                            }
                            _ => {}
                        }
                    }
                })
                .on_tray_icon_event({
                    let handle = handle.clone();
                    move |_tray, event| {
                        // Clic gauche → afficher/masquer la fenêtre
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            if let Some(window) = handle.get_webview_window("main") {
                                if window.is_visible().unwrap_or(false) {
                                    let _ = window.hide();
                                } else {
                                    show_window(&window);
                                }
                            }
                        }
                    }
                })
                .build(app)?;

            // ── Fenêtre principale ───────────────────────────────────────────
            if let Some(window) = app.get_webview_window("main") {
                // Intercepter la fermeture : masquer au lieu de quitter
                let win_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win_clone.hide();
                    }
                });

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
