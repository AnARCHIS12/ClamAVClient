use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use uuid::Uuid;

use crate::models::{QuarantineEntry, ThreatMatch};
use crate::storage::{read_quarantine_index, write_quarantine_index, StoragePaths};

pub fn list_items(paths: &StoragePaths) -> Result<Vec<QuarantineEntry>> {
    read_quarantine_index(paths)
}

pub fn quarantine_threats(
    threats: &[ThreatMatch],
    paths: &StoragePaths,
) -> Result<Vec<QuarantineEntry>> {
    let mut index = read_quarantine_index(paths)?;
    let mut quarantined = Vec::new();

    for threat in threats {
        let source = PathBuf::from(&threat.path);
        if !source.exists() || !source.is_file() {
            continue;
        }

        let file_name = source
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("infected.bin")
            .to_string();

        let id = Uuid::new_v4().to_string();
        let destination = paths
            .quarantine_dir
            .join(format!("{}-{}", id, sanitize_name(&file_name)));
        let metadata = fs::metadata(&source)?;

        move_file(&source, &destination)?;

        let entry = QuarantineEntry {
            id,
            original_path: source.display().to_string(),
            quarantined_path: destination.display().to_string(),
            signature: threat.signature.clone(),
            detected_at: Utc::now(),
            file_name,
            size: metadata.len(),
        };

        quarantined.push(entry.clone());
        index.insert(0, entry);
    }

    write_quarantine_index(paths, &index)?;
    Ok(quarantined)
}

pub fn restore_item(id: &str, paths: &StoragePaths) -> Result<QuarantineEntry> {
    let mut index = read_quarantine_index(paths)?;
    let position = index
        .iter()
        .position(|item| item.id == id)
        .ok_or_else(|| anyhow!("Élément de quarantaine introuvable"))?;

    let entry = index.remove(position);
    let quarantined = PathBuf::from(&entry.quarantined_path);
    let original = PathBuf::from(&entry.original_path);

    if let Some(parent) = original.parent() {
        fs::create_dir_all(parent)?;
    }

    move_file(&quarantined, &original)?;
    write_quarantine_index(paths, &index)?;
    Ok(entry)
}

pub fn delete_item(id: &str, paths: &StoragePaths) -> Result<()> {
    let mut index = read_quarantine_index(paths)?;
    let position = index
        .iter()
        .position(|item| item.id == id)
        .ok_or_else(|| anyhow!("Élément de quarantaine introuvable"))?;

    let entry = index.remove(position);
    let quarantined = PathBuf::from(&entry.quarantined_path);

    if quarantined.exists() {
        fs::remove_file(&quarantined)
            .with_context(|| format!("Suppression impossible: {}", quarantined.display()))?;
    }

    write_quarantine_index(paths, &index)?;
    Ok(())
}

fn move_file(source: &Path, destination: &Path) -> Result<()> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(source, destination)?;
            fs::remove_file(source)?;
            Ok(())
        }
    }
}

fn sanitize_name(file_name: &str) -> String {
    file_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || ['.', '-', '_'].contains(&character) {
                character
            } else {
                '_'
            }
        })
        .collect()
}

