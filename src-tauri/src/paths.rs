use std::env;
use std::path::PathBuf;

pub fn platform_name() -> String {
    match env::consts::OS {
        "linux" => "Linux".to_string(),
        "windows" => "Windows".to_string(),
        "macos" => "macOS".to_string(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => "Inconnu".to_string(),
            }
        }
    }
}

pub fn default_quick_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    paths.extend([
        dirs::desktop_dir(),
        dirs::document_dir(),
        downloads_dir(),
    ]
    .into_iter()
    .flatten());

    if let Some(home) = dirs::home_dir() {
        if paths.is_empty() {
            paths.push(home);
        }
    }

    unique_existing(paths)
}

pub fn quick_paths_from_config(raw_paths: &[String], auto_scan_downloads: bool) -> Vec<PathBuf> {
    let mut paths = default_quick_paths();
    paths.extend(watch_paths_from_config(raw_paths, auto_scan_downloads));
    unique_existing(paths)
}

pub fn default_watch_paths() -> Vec<PathBuf> {
    let mut paths = default_quick_paths();

    if let Some(downloads) = downloads_dir() {
        paths.push(downloads);
    }

    unique_existing(paths)
}

pub fn watch_paths_from_config(raw_paths: &[String], auto_scan_downloads: bool) -> Vec<PathBuf> {
    let mut paths = normalize_targets(raw_paths);

    if auto_scan_downloads {
        if let Some(downloads) = downloads_dir() {
            paths.push(downloads);
        }
    }

    unique_existing(paths)
}

pub fn downloads_dir() -> Option<PathBuf> {
    dirs::download_dir().or_else(|| {
        dirs::home_dir().and_then(|home| {
            let candidate = home.join("Downloads");
            candidate.exists().then_some(candidate)
        })
    })
}

pub fn default_full_targets() -> Vec<PathBuf> {
    let candidates = if cfg!(target_os = "windows") {
        let system_drive = env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
        vec![PathBuf::from(format!("{system_drive}\\"))]
    } else {
        vec![PathBuf::from("/")]
    };

    unique_existing(candidates)
}

pub fn normalize_targets(raw_targets: &[String]) -> Vec<PathBuf> {
    let targets = raw_targets.iter().map(PathBuf::from).collect::<Vec<_>>();
    unique_existing(targets)
}

pub fn common_binary_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(path) = env::var_os("PATH") {
        dirs.extend(env::split_paths(&path));
    }

    if cfg!(target_os = "windows") {
        dirs.extend([
            PathBuf::from(r"C:\Program Files\ClamAV"),
            PathBuf::from(r"C:\Program Files (x86)\ClamAV"),
        ]);
    } else if cfg!(target_os = "macos") {
        dirs.extend([
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/opt/local/bin"),
        ]);
    } else {
        dirs.extend([
            PathBuf::from("/usr/bin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/snap/bin"),
        ]);
    }

    dirs.sort();
    dirs.dedup();
    dirs
}

pub fn signature_locations() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if cfg!(target_os = "windows") {
        paths.extend([
            PathBuf::from(r"C:\ProgramData\.clamwin\db"),
            PathBuf::from(r"C:\Program Files\ClamAV\Database"),
        ]);
    } else if cfg!(target_os = "macos") {
        paths.extend([
            PathBuf::from("/usr/local/var/lib/clamav"),
            PathBuf::from("/opt/homebrew/var/lib/clamav"),
        ]);
    } else {
        paths.extend([
            PathBuf::from("/var/lib/clamav"),
            PathBuf::from("/usr/local/share/clamav"),
        ]);
    }

    paths
}

fn unique_existing(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut unique = Vec::new();

    for path in paths {
        if path.exists() && !unique.iter().any(|existing| existing == &path) {
            unique.push(path);
        }
    }

    unique
}
