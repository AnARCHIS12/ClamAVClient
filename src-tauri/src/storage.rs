use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::models::{AppConfig, QuarantineEntry, ScanReport, UpdateReport};

#[derive(Debug, Clone)]
pub struct StoragePaths {
    pub data_dir: PathBuf,
    pub logs_file: PathBuf,
    pub updates_file: PathBuf,
    pub quarantine_dir: PathBuf,
    pub quarantine_index_file: PathBuf,
    pub config_file: PathBuf,
    pub runtime_db_dir: PathBuf,
    pub freshclam_conf_file: PathBuf,
}

impl StoragePaths {
    pub fn new(app: &AppHandle) -> Result<Self> {
        let data_dir = app
            .path()
            .app_data_dir()
            .context("Impossible de déterminer le dossier d'application")?;

        let paths = Self {
            logs_file: data_dir.join("scan-history.json"),
            updates_file: data_dir.join("signature-updates.json"),
            quarantine_dir: data_dir.join("quarantine"),
            quarantine_index_file: data_dir.join("quarantine-index.json"),
            config_file: data_dir.join("config.json"),
            runtime_db_dir: data_dir.join("clamav-db"),
            freshclam_conf_file: data_dir.join("freshclam.conf"),
            data_dir,
        };

        paths.ensure()?;
        Ok(paths)
    }

    pub fn ensure(&self) -> Result<()> {
        fs::create_dir_all(&self.data_dir)?;
        fs::create_dir_all(&self.quarantine_dir)?;
        fs::create_dir_all(&self.runtime_db_dir)?;

        ensure_json_file(&self.logs_file)?;
        ensure_json_file(&self.updates_file)?;
        ensure_json_file(&self.quarantine_index_file)?;
        ensure_json_value(&self.config_file, &AppConfig::default())?;
        ensure_freshclam_conf(self)?;
        Ok(())
    }
}

pub fn read_scan_history(paths: &StoragePaths) -> Result<Vec<ScanReport>> {
    read_json_vec(&paths.logs_file)
}

pub fn append_scan_history(paths: &StoragePaths, report: &ScanReport) -> Result<()> {
    let mut items = read_scan_history(paths)?;
    items.insert(0, report.clone());
    write_json_vec(&paths.logs_file, &items)
}

pub fn read_update_history(paths: &StoragePaths) -> Result<Vec<UpdateReport>> {
    read_json_vec(&paths.updates_file)
}

pub fn append_update_history(paths: &StoragePaths, report: &UpdateReport) -> Result<()> {
    let mut items = read_update_history(paths)?;
    items.insert(0, report.clone());
    write_json_vec(&paths.updates_file, &items)
}

pub fn read_quarantine_index(paths: &StoragePaths) -> Result<Vec<QuarantineEntry>> {
    read_json_vec(&paths.quarantine_index_file)
}

pub fn write_quarantine_index(paths: &StoragePaths, items: &[QuarantineEntry]) -> Result<()> {
    write_json_vec(&paths.quarantine_index_file, items)
}

pub fn read_app_config(paths: &StoragePaths) -> Result<AppConfig> {
    read_json_value_or_default(&paths.config_file)
}

pub fn write_app_config(paths: &StoragePaths, config: &AppConfig) -> Result<()> {
    write_json_value(&paths.config_file, config)
}

fn ensure_json_file(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::write(path, "[]")?;
    }

    Ok(())
}

fn ensure_json_value<T>(path: &Path, value: &T) -> Result<()>
where
    T: Serialize,
{
    if !path.exists() {
        write_json_value(path, value)?;
    }

    Ok(())
}

fn read_json_vec<T>(path: &Path) -> Result<Vec<T>>
where
    T: DeserializeOwned,
{
    let content = fs::read_to_string(path)?;

    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let parsed = serde_json::from_str(&content)?;
    Ok(parsed)
}

fn write_json_vec<T>(path: &Path, items: &[T]) -> Result<()>
where
    T: Serialize,
{
    let payload = serde_json::to_string_pretty(items)?;
    fs::write(path, payload)?;
    Ok(())
}

fn read_json_value_or_default<T>(path: &Path) -> Result<T>
where
    T: DeserializeOwned + Default,
{
    let content = fs::read_to_string(path)?;

    if content.trim().is_empty() {
        return Ok(T::default());
    }

    let parsed = serde_json::from_str(&content).unwrap_or_default();
    Ok(parsed)
}

fn write_json_value<T>(path: &Path, value: &T) -> Result<()>
where
    T: Serialize,
{
    let payload = serde_json::to_string_pretty(value)?;
    fs::write(path, payload)?;
    Ok(())
}

fn ensure_freshclam_conf(paths: &StoragePaths) -> Result<()> {
    let content = format!(
        "DatabaseDirectory {}\nDatabaseMirror database.clamav.net\nChecks 12\nForeground yes\n",
        paths.runtime_db_dir.display()
    );

    fs::write(&paths.freshclam_conf_file, content)?;
    Ok(())
}
