use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ScanMode {
    Quick,
    Full,
    Custom,
    Realtime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRequest {
    pub mode: ScanMode,
    #[serde(default)]
    pub targets: Vec<String>,
    #[serde(default)]
    pub use_clamd: bool,
    #[serde(default)]
    pub quarantine_detected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreatMatch {
    pub path: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuarantineEntry {
    pub id: String,
    pub original_path: String,
    pub quarantined_path: String,
    pub signature: String,
    pub detected_at: DateTime<Utc>,
    pub file_name: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub id: String,
    pub mode: ScanMode,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub targets: Vec<String>,
    pub engine: String,
    pub scanned_files: usize,
    pub infected_files: usize,
    pub clean_files: usize,
    pub error_count: usize,
    pub access_denied_count: usize,
    pub access_denied_paths: Vec<String>,
    pub duration_ms: i64,
    pub status: String,
    pub threats: Vec<ThreatMatch>,
    pub quarantined: Vec<QuarantineEntry>,
    pub raw_output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateReport {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub success: bool,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub total_scans: usize,
    pub total_threats: usize,
    pub quarantined_items: usize,
    pub last_scan: Option<ScanReport>,
    pub realtime_enabled: bool,
    pub engine_ready: bool,
    pub freshclam_ready: bool,
    pub clamd_ready: bool,
    pub platform: String,
    pub quick_targets: Vec<String>,
    pub auto_scan_downloads: bool,
    pub engine_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgressEvent {
    pub scan_id: String,
    pub current: usize,
    pub total: usize,
    pub percent: f64,
    pub status: String,
    pub path: Option<String>,
    pub infected_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealtimeStatus {
    pub enabled: bool,
    pub watched_paths: Vec<String>,
    pub auto_quarantine: bool,
    pub using_clamd: bool,
    pub downloads_protected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStatus {
    pub platform: String,
    pub clamscan_path: Option<String>,
    pub clamdscan_path: Option<String>,
    pub freshclam_path: Option<String>,
    pub clamd_path: Option<String>,
    pub signatures_found: bool,
    pub recommended_paths: Vec<String>,
    pub downloads_path: Option<String>,
    pub elevated_session: bool,
    pub permission_level: String,
    pub permission_hint: String,
    pub can_scan_system_paths: bool,
    pub engine_source: String,
    pub bundled_runtime_available: bool,
    pub database_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreatEventPayload {
    pub scan_id: Option<String>,
    pub threat: ThreatMatch,
    pub quarantined: Vec<QuarantineEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub watched_paths: Vec<String>,
    pub auto_quarantine: bool,
    pub use_clamd_by_default: bool,
    pub auto_scan_downloads: bool,
    pub auto_update_signatures: bool,
    pub auto_update_interval_hours: u64,
    pub auto_start_realtime: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            watched_paths: paths::default_watch_paths()
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
            auto_quarantine: true,
            use_clamd_by_default: false,
            auto_scan_downloads: true,
            auto_update_signatures: true,
            auto_update_interval_hours: 24,
            auto_start_realtime: true,
        }
    }
}
