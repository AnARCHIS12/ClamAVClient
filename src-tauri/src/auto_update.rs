use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::Result;
use chrono::Utc;
use tauri::{AppHandle, Emitter};

use crate::clamav;
use crate::models::UpdateReport;
use crate::storage::StoragePaths;

pub struct AutoUpdate {
    stop_signal: Arc<AtomicBool>,
    worker_handle: Option<JoinHandle<()>>,
    interval_hours: u64,
    last_update: Arc<Mutex<Option<chrono::DateTime<Utc>>>>,
}

impl AutoUpdate {
    pub fn new(interval_hours: u64) -> Self {
        Self {
            stop_signal: Arc::new(AtomicBool::new(false)),
            worker_handle: None,
            interval_hours,
            last_update: Arc::new(Mutex::new(None)),
        }
    }

    pub fn start(&mut self, app: AppHandle, storage: StoragePaths) -> Result<()> {
        if self.worker_handle.is_some() {
            return Ok(());
        }

        let stop_signal = Arc::clone(&self.stop_signal);
        let interval = Duration::from_secs(self.interval_hours * 3600);
        let last_update = Arc::clone(&self.last_update);
        let worker_app = app.clone();
        let worker_storage = storage.clone();

        let handle = thread::spawn(move || {
            // Premier check immédiat au démarrage
            if let Ok(report) = Self::run_update(&worker_app, &worker_storage) {
                let _ = Self::update_last_update(&last_update, &report);
                let _ = worker_app.emit("signature-update-complete", report);
            }

            // Boucle périodique
            while !stop_signal.load(Ordering::Relaxed) {
                thread::sleep(interval);

                if stop_signal.load(Ordering::Relaxed) {
                    break;
                }

                if let Ok(report) = Self::run_update(&worker_app, &worker_storage) {
                    let _ = Self::update_last_update(&last_update, &report);
                    let _ = worker_app.emit("signature-update-complete", report);
                }
            }
        });

        self.worker_handle = Some(handle);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.stop_signal.store(true, Ordering::Relaxed);
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn is_running(&self) -> bool {
        self.worker_handle.is_some()
    }

    pub fn get_last_update(&self) -> Option<chrono::DateTime<Utc>> {
        self.last_update.lock().ok().and_then(|opt| *opt)
    }

    pub fn set_interval(&mut self, hours: u64) {
        self.interval_hours = hours;
        // Redémarrer pour appliquer le nouvel intervalle
        if self.is_running() {
            // Note: nécessite de redémarrer avec le nouvel intervalle
        }
    }

    fn run_update(app: &AppHandle, storage: &StoragePaths) -> Result<UpdateReport> {
        clamav::run_signature_update(app, storage)
    }

    fn update_last_update(
        last_update: &Arc<Mutex<Option<chrono::DateTime<Utc>>>>,
        report: &UpdateReport,
    ) -> Result<()> {
        if let Ok(mut guard) = last_update.lock() {
            *guard = Some(report.finished_at);
        }
        Ok(())
    }
}

impl Drop for AutoUpdate {
    fn drop(&mut self) {
        self.stop();
    }
}
