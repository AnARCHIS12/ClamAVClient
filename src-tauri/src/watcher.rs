use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::Utc;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::clamav;
use crate::models::{AppConfig, RealtimeStatus, ScanMode, ScanReport, ThreatEventPayload};
use crate::paths;
use crate::quarantine;
use crate::storage::{append_scan_history, StoragePaths};

pub struct RealtimeProtection {
    watcher: RecommendedWatcher,
    stop_signal: Arc<AtomicBool>,
    worker_handle: JoinHandle<()>,
    watched_paths: Vec<PathBuf>,
    auto_quarantine: bool,
    use_clamd: bool,
}

impl RealtimeProtection {
    pub fn start(
        app: AppHandle,
        storage: StoragePaths,
        config: AppConfig,
    ) -> Result<Self> {
        let runtime_config = config.clone();
        let watched_paths = paths::watch_paths_from_config(
            &runtime_config.watched_paths,
            runtime_config.auto_scan_downloads,
        );
        if watched_paths.is_empty() {
            return Err(anyhow::anyhow!(
                "Aucun dossier surveillé n'est configuré pour la protection temps réel"
            ));
        }
        let (tx, rx) = mpsc::channel();

        let mut watcher = RecommendedWatcher::new(
            move |event| {
                let _ = tx.send(event);
            },
            Config::default(),
        )?;

        for path in &watched_paths {
            watcher.watch(path, RecursiveMode::Recursive)?;
        }

        let stop_signal = Arc::new(AtomicBool::new(false));
        let worker_stop_signal = Arc::clone(&stop_signal);
        let worker_app = app.clone();
        let worker_paths = watched_paths.clone();
        let worker_handle = thread::spawn(move || {
            let mut recently_scanned = HashMap::<PathBuf, Instant>::new();

            while !worker_stop_signal.load(Ordering::Relaxed) {
                match rx.recv_timeout(Duration::from_millis(900)) {
                    Ok(Ok(event)) => {
                        if !matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                            continue;
                        }

                        for path in event.paths {
                            if !path.exists() || !path.is_file() {
                                continue;
                            }

                            if should_skip(&mut recently_scanned, &path) {
                                continue;
                            }

                            match clamav::scan_single_path(
                                &worker_app,
                                &storage,
                                &path,
                                runtime_config.use_clamd_by_default,
                            ) {
                                Ok((engine, threats, raw_output)) => {
                                    if threats.is_empty() {
                                        continue;
                                    }

                                    let quarantined = if runtime_config.auto_quarantine {
                                        quarantine::quarantine_threats(&threats, &storage)
                                            .unwrap_or_default()
                                    } else {
                                        Vec::new()
                                    };

                                    let report = ScanReport {
                                        id: Uuid::new_v4().to_string(),
                                        mode: ScanMode::Realtime,
                                        started_at: Utc::now(),
                                        finished_at: Utc::now(),
                                        targets: vec![path.display().to_string()],
                                        engine,
                                        scanned_files: 1,
                                        infected_files: threats.len(),
                                        clean_files: 0,
                                        error_count: 0,
                                        access_denied_count: 0,
                                        access_denied_paths: Vec::new(),
                                        duration_ms: 0,
                                        status: "infected".to_string(),
                                        threats: threats.clone(),
                                        quarantined: quarantined.clone(),
                                        raw_output,
                                    };

                                    let _ = append_scan_history(&storage, &report);

                                    for threat in threats {
                                        let _ = worker_app.emit(
                                            "realtime-threat",
                                            ThreatEventPayload {
                                                scan_id: Some(report.id.clone()),
                                                threat,
                                                quarantined: quarantined.clone(),
                                            },
                                        );
                                    }
                                }
                                Err(e) => {
                                    // Ignorer les erreurs temporaires comme freshclam en cours
                                    if e.to_string().contains("freshclam") {
                                        continue;
                                    }
                                    // Loguer l'erreur mais continuer
                                    eprintln!("Erreur de scan temps réel: {}", e);
                                }
                            }
                        }
                    }
                    Ok(Err(_)) => {}
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }

            let _ = worker_app.emit(
                "realtime-status",
                RealtimeStatus {
                    enabled: false,
                    watched_paths: worker_paths
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect(),
                    auto_quarantine: runtime_config.auto_quarantine,
                    using_clamd: runtime_config.use_clamd_by_default,
                    downloads_protected: runtime_config.auto_scan_downloads,
                },
            );
        });

        Ok(Self {
            watcher,
            stop_signal,
            worker_handle,
            watched_paths,
            auto_quarantine: config.auto_quarantine,
            use_clamd: config.use_clamd_by_default,
        })
    }

    pub fn into_status(&self) -> RealtimeStatus {
        RealtimeStatus {
            enabled: true,
            watched_paths: self
                .watched_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
            auto_quarantine: self.auto_quarantine,
            using_clamd: self.use_clamd,
            downloads_protected: self
                .watched_paths
                .iter()
                .any(|path| paths::downloads_dir().as_ref().is_some_and(|downloads| downloads == path)),
        }
    }

    pub fn stop(self) {
        self.stop_signal.store(true, Ordering::Relaxed);
        drop(self.watcher);
        let _ = self.worker_handle.join();
    }
}

fn should_skip(cache: &mut HashMap<PathBuf, Instant>, path: &PathBuf) -> bool {
    let now = Instant::now();
    cache.retain(|_, instant| now.duration_since(*instant) < Duration::from_secs(5));

    if cache.contains_key(path) {
        true
    } else {
        cache.insert(path.clone(), now);
        false
    }
}
