use std::path::Path;
use chrono::Utc;

use crate::scan::ScanResult;
use crate::ProbeError;

const PROBE_DIR: &str = ".agent-probe";
const SNAPSHOTS_DIR: &str = "snapshots";

pub fn save(repo: &Path, result: &ScanResult) -> Result<String, ProbeError> {
    let snapshots_dir = repo.join(PROBE_DIR).join(SNAPSHOTS_DIR);
    std::fs::create_dir_all(&snapshots_dir)?;

    // Write .gitignore if it doesn't exist
    let gitignore = repo.join(PROBE_DIR).join(".gitignore");
    if !gitignore.exists() {
        std::fs::write(&gitignore, "*\n")?;
    }

    let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let filename = format!("{timestamp}.json");
    let filepath = snapshots_dir.join(&filename);

    let json = serde_json::to_string_pretty(result)?;
    std::fs::write(&filepath, json)?;

    Ok(filename)
}

pub fn latest_snapshot_path(repo: &Path) -> Option<std::path::PathBuf> {
    let snapshots_dir = repo.join(PROBE_DIR).join(SNAPSHOTS_DIR);
    if !snapshots_dir.exists() {
        return None;
    }

    let mut entries: Vec<_> = std::fs::read_dir(&snapshots_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
        .collect();

    entries.sort_by_key(|e| e.file_name());
    entries.last().map(|e| e.path())
}

pub fn load_snapshot(path: &Path) -> Result<ScanResult, ProbeError> {
    let content = std::fs::read_to_string(path)?;
    let result: ScanResult = serde_json::from_str(&content)?;
    Ok(result)
}
