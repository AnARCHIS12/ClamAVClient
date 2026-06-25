use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::models::{
    AppConfig, DashboardStats, ScanMode, ScanProgressEvent, ScanReport, ScanRequest, SystemStatus,
    ThreatEventPayload, ThreatMatch, UpdateReport,
};
use crate::paths;
use crate::quarantine;
use crate::storage::{append_scan_history, append_update_history, read_scan_history, StoragePaths};

#[derive(Debug, Clone)]
pub struct ToolPaths {
    pub clamscan: Option<PathBuf>,
    pub clamdscan: Option<PathBuf>,
    pub freshclam: Option<PathBuf>,
    pub clamd: Option<PathBuf>,
    pub database_dir: Option<PathBuf>,
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanEngineKind {
    Clamscan,
    Clamdscan,
}

impl ToolPaths {
    pub fn discover(app: Option<&AppHandle>, storage: Option<&StoragePaths>) -> Self {
        if let Some(embedded) = discover_embedded_runtime(app, storage) {
            return embedded;
        }

        let database_dir = storage
            .map(|paths| paths.runtime_db_dir.clone())
            .filter(|path| signature_dir_ready(path));

        Self {
            clamscan: find_binary(&["clamscan"], &[]),
            clamdscan: find_binary(&["clamdscan"], &[]),
            freshclam: find_binary(&["freshclam"], &[]),
            clamd: find_binary(&["clamd"], &[]),
            database_dir,
            source: "system".to_string(),
        }
    }
}

pub fn get_system_status(app: &AppHandle, storage: &StoragePaths) -> SystemStatus {
    let tools = ToolPaths::discover(Some(app), Some(storage));
    let signatures_found = paths::signature_locations().iter().any(signature_dir_ready);
    let elevated_session = is_elevated_session();
    let can_scan_system_paths = elevated_session;
    let permission_level = if elevated_session {
        "elevated".to_string()
    } else {
        "standard".to_string()
    };

    SystemStatus {
        platform: paths::platform_name(),
        clamscan_path: tools.clamscan.map(display_path),
        clamdscan_path: tools.clamdscan.map(display_path),
        freshclam_path: tools.freshclam.map(display_path),
        clamd_path: tools.clamd.map(display_path),
        signatures_found: signatures_found || tools.database_dir.as_ref().is_some_and(signature_dir_ready),
        recommended_paths: paths::default_quick_paths()
            .iter()
            .map(display_path)
            .collect(),
        downloads_path: paths::downloads_dir().map(display_path),
        elevated_session,
        permission_level,
        permission_hint: permission_hint(),
        can_scan_system_paths,
        engine_source: tools.source.clone(),
        bundled_runtime_available: tools.source == "bundled",
        database_path: tools.database_dir.map(display_path),
    }
}

pub fn build_dashboard(
    app: &AppHandle,
    storage: &StoragePaths,
    realtime_enabled: bool,
    config: &AppConfig,
) -> Result<DashboardStats> {
    let tools = ToolPaths::discover(Some(app), Some(storage));
    let logs = read_scan_history(storage)?;
    let total_threats = logs.iter().map(|log| log.infected_files).sum();
    let quarantined_items = quarantine::list_items(storage)?.len();

    Ok(DashboardStats {
        total_scans: logs.len(),
        total_threats,
        quarantined_items,
        last_scan: logs.first().cloned(),
        realtime_enabled,
        engine_ready: tools.clamscan.is_some() || tools.clamdscan.is_some(),
        freshclam_ready: tools.freshclam.is_some(),
        clamd_ready: tools.clamdscan.is_some() || tools.clamd.is_some(),
        platform: paths::platform_name(),
        quick_targets: paths::watch_paths_from_config(&config.watched_paths, config.auto_scan_downloads)
            .iter()
            .map(display_path)
            .collect(),
        auto_scan_downloads: config.auto_scan_downloads,
        engine_source: tools.source,
    })
}

pub fn run_signature_update(app: &AppHandle, storage: &StoragePaths) -> Result<UpdateReport> {
    let tools = ToolPaths::discover(Some(app), Some(storage));
    let freshclam = tools
        .freshclam
        .ok_or_else(|| anyhow!("freshclam introuvable sur cette machine"))?;
    let started_at = Utc::now();
    let mut command = Command::new(&freshclam);
    command.arg("--stdout");

    if let Some(database_dir) = &tools.database_dir {
        command.arg(format!("--datadir={}", database_dir.display()));
    }

    if storage.freshclam_conf_file.exists() {
        command.arg(format!(
            "--config-file={}",
            storage.freshclam_conf_file.display()
        ));
    }

    let output = command
        .output()
        .with_context(|| format!("Impossible d'exécuter {}", freshclam.display()))?;
    let finished_at = Utc::now();

    let mut payload = String::new();
    payload.push_str(&String::from_utf8_lossy(&output.stdout));

    if !output.stderr.is_empty() {
        if !payload.is_empty() {
            payload.push('\n');
        }
        payload.push_str(&String::from_utf8_lossy(&output.stderr));
    }

    let report = UpdateReport {
        id: Uuid::new_v4().to_string(),
        started_at,
        finished_at,
        success: output.status.success(),
        output: payload.trim().to_string(),
    };

    append_update_history(storage, &report)?;
    Ok(report)
}

pub fn execute_scan(
    app: &AppHandle,
    storage: &StoragePaths,
    config: &AppConfig,
    request: ScanRequest,
) -> Result<ScanReport> {
    let tools = ToolPaths::discover(Some(app), Some(storage));
    let (engine_path, engine_kind) = select_scan_engine(&tools, request.use_clamd, &request.mode)?;
    let started_at = Utc::now();
    let scan_id = Uuid::new_v4().to_string();
    let targets = resolve_targets(&request, config)?;
    // For full-system scans (target is root "/" or a drive), walking the entire
    // filesystem to count files is extremely slow and error-prone.  Use a large
    // sentinel so the progress bar moves smoothly; the real count comes from the
    // clamscan summary line at the end.
    let is_system_wide = matches!(request.mode, ScanMode::Full);
    let total_files = if is_system_wide {
        // Sentinel: progress will be driven by scanned_line_count until the
        // real summary overwrites it.
        usize::MAX / 2
    } else {
        count_files(&targets).max(1)
    };
    let engine_name = engine_path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("clamscan")
        .to_string();

    let mut command = Command::new(&engine_path);
    for argument in build_scan_args(engine_kind, tools.database_dir.as_ref()) {
        command.arg(argument);
    }

    for target in &targets {
        #[cfg(target_os = "windows")]
        {
            let path_str = target.to_string_lossy().replace('\\', "/");
            command.arg(path_str);
        }
        #[cfg(not(target_os = "windows"))]
        {
            command.arg(target);
        }
    }

    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .with_context(|| format!("Impossible d'exécuter {}", engine_path.display()))?;

    let stdout = child.stdout.take().context("Sortie standard indisponible")?;
    let stderr = child.stderr.take().context("Sortie d'erreur indisponible")?;

    let stderr_handle = thread::spawn(move || -> String {
        let mut buffer = String::new();
        let _ = BufReader::new(stderr).read_to_string(&mut buffer);
        buffer
    });

    let mut reader = BufReader::new(stdout);
    let mut raw_output = Vec::new();
    let mut scanned_line_count = 0usize;
    let mut threats = Vec::new();
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }

        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }

        raw_output.push(trimmed.clone());

        if let Some(threat) = parse_threat_line(&trimmed) {
            scanned_line_count += 1;
            threats.push(threat.clone());

            app.emit(
                "threat-detected",
                ThreatEventPayload {
                    scan_id: Some(scan_id.clone()),
                    threat,
                    quarantined: Vec::new(),
                },
            )?;
        } else if is_scan_line(&trimmed) {
            scanned_line_count += 1;
        }

        // For system-wide scans the total is unknown ahead of time.
        // Show the live file counter as current/total; hold percent at 50%
        // so the progress bar indicates activity without lying about completion.
        let (progress_current, progress_total, progress_percent) = if is_system_wide {
            (scanned_line_count, scanned_line_count.max(1), 50.0_f64)
        } else {
            let capped = scanned_line_count.min(total_files);
            (capped, total_files, (capped as f64 / total_files as f64) * 100.0)
        };
        app.emit(
            "scan-progress",
            ScanProgressEvent {
                scan_id: scan_id.clone(),
                current: progress_current,
                total: progress_total,
                percent: progress_percent,
                status: if threats.is_empty() {
                    "Analyse en cours".to_string()
                } else {
                    "Menace détectée".to_string()
                },
                path: parse_scanned_path(&trimmed),
                infected_count: threats.len(),
            },
        )?;
    }

    let status = child.wait()?;
    let stderr_output = stderr_handle
        .join()
        .map_err(|_| anyhow!("Impossible de joindre la lecture stderr"))?;

    if !stderr_output.trim().is_empty() {
        raw_output.push(stderr_output.trim().to_string());
    }

    let parsed = parse_scan_output(&raw_output);
    let exit_code = status.code();
    let is_error = exit_code.unwrap_or(0) == 2;
    let mut error_count = parsed.error_count;
    if is_error && error_count == 0 {
        error_count = 1;
    }

    // Prefer the authoritative summary value that clamscan/clamdscan prints
    // ("Scanned files: N"). Fall back to the line-counter only when the summary
    // is absent (e.g. early exit).  Never expose the sentinel value.
    let scanned_files = parsed.scanned_files.unwrap_or_else(|| {
        if is_system_wide {
            // No summary parsed and it was a system-wide scan — we genuinely
            // don't know; report what we counted from output lines.
            scanned_line_count
        } else if engine_kind == ScanEngineKind::Clamdscan && scanned_line_count == 0 {
            total_files
        } else {
            scanned_line_count
        }
    });
    let infected_files = parsed.infected_files.unwrap_or(threats.len());
    let clean_files = scanned_files.saturating_sub(infected_files);

    let quarantined = if request.quarantine_detected && !threats.is_empty() {
        quarantine::quarantine_threats(&threats, storage)?
    } else {
        Vec::new()
    };

    let mut raw_output_str = raw_output.join("\n");
    if is_error && raw_output_str.trim().is_empty() {
        raw_output_str = "Une erreur système ClamAV est survenue (code de sortie 2). Vérifiez vos signatures de virus.".to_string();
    }

    let finished_at = Utc::now();
    let report = ScanReport {
        id: scan_id.clone(),
        mode: request.mode,
        started_at,
        finished_at,
        targets: targets.iter().map(display_path).collect(),
        engine: engine_name,
        scanned_files,
        infected_files,
        clean_files,
        error_count,
        access_denied_count: parsed.access_denied_paths.len(),
        access_denied_paths: parsed.access_denied_paths.clone(),
        duration_ms: (finished_at - started_at).num_milliseconds(),
        status: normalize_scan_status(exit_code, infected_files),
        threats: threats.clone(),
        quarantined: quarantined.clone(),
        raw_output: raw_output_str,
    };

    append_scan_history(storage, &report)?;

    // Emit a clean 100% completion event using the real scanned count.
    app.emit(
        "scan-progress",
        ScanProgressEvent {
            scan_id,
            current: scanned_files.max(1),
            total: scanned_files.max(1),
            percent: 100.0,
            status: if threats.is_empty() {
                "Scan terminé".to_string()
            } else {
                "Scan terminé avec menace".to_string()
            },
            path: None,
            infected_count: infected_files,
        },
    )?;

    Ok(report)
}

pub fn scan_single_path(
    app: &AppHandle,
    storage: &StoragePaths,
    file_path: &Path,
    use_clamd: bool,
) -> Result<(String, Vec<ThreatMatch>, String)> {
    // Vérifier si freshclam est en cours d'exécution
    if is_freshclam_running() {
        return Err(anyhow!("freshclam est en cours d'exécution, mise à jour des signatures en cours"));
    }

    let tools = ToolPaths::discover(Some(app), Some(storage));
    let (engine, engine_kind) = select_scan_engine(&tools, use_clamd, &ScanMode::Realtime)?;
    let mut command = Command::new(&engine);

    for argument in build_single_file_args(engine_kind, tools.database_dir.as_ref()) {
        command.arg(argument);
    }

    #[cfg(target_os = "windows")]
    {
        let path_str = file_path.to_string_lossy().replace('\\', "/");
        command.arg(path_str);
    }
    #[cfg(not(target_os = "windows"))]
    {
        command.arg(file_path);
    }
    let output = command.output()?;
    let mut payload = String::from_utf8_lossy(&output.stdout).to_string();

    if !output.stderr.is_empty() {
        if !payload.is_empty() {
            payload.push('\n');
        }
        payload.push_str(&String::from_utf8_lossy(&output.stderr));
    }

    let threats = payload
        .lines()
        .filter_map(parse_threat_line)
        .collect::<Vec<_>>();

    Ok((display_path(engine), threats, payload))
}

fn resolve_targets(request: &ScanRequest, config: &AppConfig) -> Result<Vec<PathBuf>> {
    let targets = match request.mode {
        ScanMode::Quick => {
            paths::quick_paths_from_config(&config.watched_paths, config.auto_scan_downloads)
        }
        ScanMode::Full => paths::default_full_targets(),
        ScanMode::Custom | ScanMode::Realtime => paths::normalize_targets(&request.targets),
    };

    if targets.is_empty() {
        Err(anyhow!("Aucune cible valide à analyser"))
    } else {
        Ok(targets)
    }
}

fn build_scan_args(engine_kind: ScanEngineKind, database_dir: Option<&PathBuf>) -> Vec<OsString> {
    if engine_kind == ScanEngineKind::Clamdscan {
        vec![
            OsString::from("--fdpass"),
            OsString::from("--infected"),
            OsString::from("--multiscan"),
            OsString::from("--verbose"),
        ]
    } else {
        let mut args = vec![
            OsString::from("--recursive"),
            OsString::from("--infected"),
            OsString::from("--stdout"),
            OsString::from("--verbose"),
        ];

        if let Some(database_dir) = database_dir {
            args.push(OsString::from(format!("--database={}", database_dir.display())));
        }

        args
    }
}

fn build_single_file_args(engine_kind: ScanEngineKind, database_dir: Option<&PathBuf>) -> Vec<OsString> {
    if engine_kind == ScanEngineKind::Clamdscan {
        vec![OsString::from("--fdpass"), OsString::from("--infected")]
    } else {
        let mut args = vec![OsString::from("--infected"), OsString::from("--stdout")];

        if let Some(database_dir) = database_dir {
            args.push(OsString::from(format!("--database={}", database_dir.display())));
        }

        args
    }
}

fn normalize_scan_status(exit_code: Option<i32>, threats: usize) -> String {
    match exit_code {
        Some(0) => "clean".to_string(),
        Some(1) if threats > 0 => "infected".to_string(),
        Some(2) => "error".to_string(),
        Some(code) => format!("completed_{code}"),
        None => "unknown".to_string(),
    }
}

fn count_files(targets: &[PathBuf]) -> usize {
    targets
        .iter()
        .map(|target| {
            if target.is_file() {
                1
            } else {
                WalkDir::new(target)
                    .follow_links(false)
                    .into_iter()
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| entry.file_type().is_file())
                    .count()
            }
        })
        .sum()
}

fn parse_threat_line(line: &str) -> Option<ThreatMatch> {
    if !line.ends_with(" FOUND") {
        return None;
    }

    let separator = line.rfind(": ")?;
    let path = line[..separator].trim().to_string();
    let signature = line[separator + 2..line.len() - " FOUND".len()]
        .trim()
        .to_string();

    if path.is_empty() || signature.is_empty() {
        None
    } else {
        Some(ThreatMatch { path, signature })
    }
}

fn parse_scanned_path(line: &str) -> Option<String> {
    line.rfind(": ").map(|index| line[..index].trim().to_string())
}

fn is_scan_line(line: &str) -> bool {
    [": OK", " FOUND", " ERROR", ": Empty file"].iter().any(|suffix| line.ends_with(suffix))
}

fn select_scan_engine(
    tools: &ToolPaths,
    use_clamd: bool,
    mode: &ScanMode,
) -> Result<(PathBuf, ScanEngineKind)> {
    let prefer_clamd = use_clamd && !matches!(mode, ScanMode::Full);

    if prefer_clamd {
        if let Some(path) = &tools.clamdscan {
            return Ok((path.clone(), ScanEngineKind::Clamdscan));
        }
    }

    if let Some(path) = &tools.clamscan {
        return Ok((path.clone(), ScanEngineKind::Clamscan));
    }

    if let Some(path) = &tools.clamdscan {
        return Ok((path.clone(), ScanEngineKind::Clamdscan));
    }

    Err(anyhow!("Aucun moteur ClamAV exploitable n'a ete trouve sur cette machine"))
}

fn find_binary(names: &[&str], search_dirs: &[PathBuf]) -> Option<PathBuf> {
    let executable_names = names
        .iter()
        .flat_map(|name| {
            if cfg!(target_os = "windows") {
                vec![format!("{name}.exe"), (*name).to_string()]
            } else {
                vec![(*name).to_string()]
            }
        })
        .collect::<Vec<_>>();

    for directory in search_dirs {
        for executable in &executable_names {
            let candidate = directory.join(executable);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    for directory in paths::common_binary_dirs() {
        for executable in &executable_names {
            let candidate = directory.join(executable);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

fn signature_dir_ready(path: &PathBuf) -> bool {
    path.exists()
        && ["main.cvd", "main.cld", "daily.cvd", "daily.cld"]
            .iter()
            .any(|file_name| path.join(file_name).exists())
}

fn display_path(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}

fn discover_embedded_runtime(app: Option<&AppHandle>, storage: Option<&StoragePaths>) -> Option<ToolPaths> {
    let app = app?;
    let storage = storage?;
    let runtime_root = candidate_runtime_roots(app)
        .into_iter()
        .find(|root| root.exists())?;
    let bin_dir = runtime_root.join("bin");
    let seed_db_dir = runtime_root.join("db");
    let runtime_db_dir = storage.runtime_db_dir.clone();

    let _ = seed_database_if_needed(&seed_db_dir, &runtime_db_dir);

    let clamscan = find_binary(&["clamscan"], std::slice::from_ref(&bin_dir));
    let clamdscan = find_binary(&["clamdscan"], std::slice::from_ref(&bin_dir));
    let freshclam = find_binary(&["freshclam"], std::slice::from_ref(&bin_dir));
    let clamd = find_binary(&["clamd"], std::slice::from_ref(&bin_dir));

    if clamscan.is_none() && freshclam.is_none() {
        return None;
    }

    Some(ToolPaths {
        clamscan,
        clamdscan,
        freshclam,
        clamd,
        database_dir: Some(runtime_db_dir),
        source: "bundled".to_string(),
    })
}

fn candidate_runtime_roots(app: &AppHandle) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(resource_dir) = app.path().resource_dir() {
        roots.push(resource_dir.join("vendor").join("clamav").join(platform_folder_name()));
        roots.push(resource_dir.join("clamav").join(platform_folder_name()));
    }

    roots.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("vendor")
            .join("clamav")
            .join(platform_folder_name()),
    );

    roots
}

fn platform_folder_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

fn seed_database_if_needed(seed_dir: &Path, runtime_db_dir: &Path) -> Result<()> {
    fs::create_dir_all(runtime_db_dir)?;

    if signature_dir_ready(&runtime_db_dir.to_path_buf()) || !seed_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(seed_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let extension = path.extension().and_then(|value| value.to_str()).unwrap_or_default();
        if !["cvd", "cld", "cdiff", "cfg"].contains(&extension) {
            continue;
        }

        let target = runtime_db_dir.join(entry.file_name());
        if !target.exists() {
            fs::copy(path, target)?;
        }
    }

    Ok(())
}

#[derive(Default)]
struct ParsedScanOutput {
    scanned_files: Option<usize>,
    infected_files: Option<usize>,
    error_count: usize,
    access_denied_paths: Vec<String>,
}

fn parse_scan_output(lines: &[String]) -> ParsedScanOutput {
    let mut parsed = ParsedScanOutput::default();

    for line in lines {
        if parsed.scanned_files.is_none() {
            parsed.scanned_files = parse_summary_value(line, "Scanned files:");
        }

        if parsed.infected_files.is_none() {
            parsed.infected_files = parse_summary_value(line, "Infected files:");
        }

        if let Some(total_errors) = parse_summary_value(line, "Total errors:") {
            parsed.error_count = parsed.error_count.max(total_errors);
        }

        if line.ends_with(" ERROR") || line.contains("ERROR:") {
            parsed.error_count += 1;
        }

        if is_access_denied_line(line) {
            let path = parse_scanned_path(line).unwrap_or_else(|| line.to_string());
            if !parsed.access_denied_paths.iter().any(|existing| existing == &path) {
                parsed.access_denied_paths.push(path);
            }
        }
    }

    parsed
}

fn parse_summary_value(line: &str, label: &str) -> Option<usize> {
    line.strip_prefix(label)
        .and_then(|value| value.split_whitespace().next())
        .and_then(|value| value.parse::<usize>().ok())
}

fn is_access_denied_line(line: &str) -> bool {
    ["Access denied", "Permission denied", "Operation not permitted", "Can't access file"]
        .iter()
        .any(|pattern| line.contains(pattern))
}

fn is_elevated_session() -> bool {
    #[cfg(target_os = "windows")]
    {
        return Command::new("net")
            .arg("session")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        return Command::new("id")
            .arg("-u")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|uid| uid.trim() == "0")
            .unwrap_or(false);
    }

    #[allow(unreachable_code)]
    false
}

fn is_freshclam_running() -> bool {
    #[cfg(target_os = "windows")]
    {
        return Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq freshclam.exe", "/NH"])
            .output()
            .map(|output| {
                let output = String::from_utf8_lossy(&output.stdout);
                output.contains("freshclam.exe")
            })
            .unwrap_or(false);
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        return Command::new("pgrep")
            .arg("freshclam")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
    }

    #[allow(unreachable_code)]
    false
}

fn permission_hint() -> String {
    if cfg!(target_os = "windows") {
        "Un scan complet de zones protégées fonctionne mieux en lançant l'application comme administrateur."
            .to_string()
    } else if cfg!(target_os = "macos") {
        "Sur macOS, accordez l'accès complet au disque pour analyser les zones système et les répertoires d'autres applications."
            .to_string()
    } else {
        "Sur Linux, les scans système complets et la quarantaine peuvent nécessiter des droits élevés selon les dossiers protégés."
            .to_string()
    }
}
