use std::path::Path;

use crate::detect::{self, DetectedProject};
use crate::git::{self, GitState};
use crate::tools::{self, SuiteToolInfo, ToolInfo};
use crate::ProbeError;

use chrono::Utc;
use serde::{Deserialize, Serialize};

const SCAN_SCHEMA_VERSION: &str = "probe.scan.v1";

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanResult {
    #[serde(default = "default_scan_schema_version")]
    pub schema_version: String,
    pub timestamp: String,
    pub repo_path: String,
    pub projects: Vec<DetectedProject>,
    pub git: Option<GitState>,
    pub tools: Vec<ToolInfo>,
    #[serde(default)]
    pub suite_tools: Vec<SuiteToolInfo>,
    pub lockfiles: Vec<LockfileInfo>,
    pub suggested_commands: Vec<SuggestedCommand>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LockfileInfo {
    pub path: String,
    pub hash: String,
    pub stale: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuggestedCommand {
    pub action: String,
    pub command: String,
    #[serde(default = "default_command_argv")]
    pub argv: Vec<String>,
    pub confidence: String,
    #[serde(default)]
    pub reason: String,
}

pub fn run_scan(repo: &Path) -> Result<ScanResult, ProbeError> {
    let projects = detect::detect_projects(repo);
    let git_state = git::get_state(repo);
    let tools = tools::detect_tools(&projects);
    let suite_tools = tools::detect_suite_tools(repo);
    let lockfiles = detect_lockfiles(repo, &projects);
    let suggested_commands = suggest_commands(&projects);

    Ok(ScanResult {
        schema_version: SCAN_SCHEMA_VERSION.to_string(),
        timestamp: Utc::now().to_rfc3339(),
        repo_path: repo.display().to_string(),
        projects,
        git: git_state,
        tools,
        suite_tools,
        lockfiles,
        suggested_commands,
    })
}

fn detect_lockfiles(repo: &Path, projects: &[DetectedProject]) -> Vec<LockfileInfo> {
    let mut lockfiles = Vec::new();

    let candidates: Vec<(&str, &str)> = projects
        .iter()
        .flat_map(|p| match p.kind.as_str() {
            "rust" => vec![("Cargo.lock", &*p.root)],
            "node" => vec![
                ("package-lock.json", &*p.root),
                ("pnpm-lock.yaml", &*p.root),
                ("yarn.lock", &*p.root),
            ],
            "python" => vec![("poetry.lock", &*p.root), ("requirements.txt", &*p.root)],
            "go" => vec![("go.sum", &*p.root)],
            _ => vec![],
        })
        .collect();

    for (filename, project_root) in candidates {
        let lockfile_path = if project_root == "." {
            repo.join(filename)
        } else {
            repo.join(project_root).join(filename)
        };

        if lockfile_path.exists() {
            let hash = hash_file(&lockfile_path).unwrap_or_default();
            let rel_path = if project_root == "." {
                filename.to_string()
            } else {
                format!("{}/{}", project_root, filename)
            };

            // Staleness: check if manifest is newer than lockfile
            let stale = is_lockfile_stale(
                &lockfile_path,
                repo,
                project_root,
                &detect_manifest_for(filename),
            );

            lockfiles.push(LockfileInfo {
                path: rel_path,
                hash,
                stale,
            });
        }
    }

    lockfiles
}

fn detect_manifest_for(lockfile: &str) -> String {
    match lockfile {
        "Cargo.lock" => "Cargo.toml".to_string(),
        "package-lock.json" | "pnpm-lock.yaml" | "yarn.lock" => "package.json".to_string(),
        "poetry.lock" => "pyproject.toml".to_string(),
        "go.sum" => "go.mod".to_string(),
        _ => String::new(),
    }
}

fn is_lockfile_stale(
    lockfile: &Path,
    repo: &Path,
    project_root: &str,
    manifest_name: &str,
) -> bool {
    if manifest_name.is_empty() {
        return false;
    }
    let manifest_path = if project_root == "." {
        repo.join(manifest_name)
    } else {
        repo.join(project_root).join(manifest_name)
    };

    if let (Ok(lock_meta), Ok(manifest_meta)) = (lockfile.metadata(), manifest_path.metadata()) {
        if let (Ok(lock_modified), Ok(manifest_modified)) =
            (lock_meta.modified(), manifest_meta.modified())
        {
            return manifest_modified > lock_modified;
        }
    }
    false
}

fn hash_file(path: &Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    let data = std::fs::read(path).ok()?;
    let hash = Sha256::digest(&data);
    Some(format!("{:x}", hash)[..16].to_string())
}

fn suggest_commands(projects: &[DetectedProject]) -> Vec<SuggestedCommand> {
    let mut commands = Vec::new();

    for project in projects {
        match project.kind.as_str() {
            "rust" => {
                commands.push(command(
                    "check",
                    &["cargo", "check"],
                    "high",
                    "Rust manifest detected",
                ));
                commands.push(command(
                    "test",
                    &["cargo", "test"],
                    "high",
                    "Rust manifest detected",
                ));
                commands.push(command(
                    "build",
                    &["cargo", "build"],
                    "high",
                    "Rust manifest detected",
                ));
            }
            "node" => {
                let pm = if project
                    .metadata
                    .get("package_manager")
                    .and_then(|v| v.as_str())
                    == Some("pnpm")
                {
                    "pnpm"
                } else {
                    "npm"
                };
                commands.push(command(
                    "install",
                    &[pm, "install"],
                    "high",
                    "Node package manifest detected",
                ));
                commands.push(command(
                    "build",
                    &[pm, "run", "build"],
                    "medium",
                    "Common Node build script; verify package scripts if this fails",
                ));
                commands.push(command(
                    "test",
                    &[pm, "test"],
                    "medium",
                    "Common Node test script; verify package scripts if this fails",
                ));
            }
            "python" => {
                commands.push(command(
                    "test",
                    &["python", "-m", "pytest"],
                    "medium",
                    "Python manifest or requirements file detected",
                ));
            }
            "go" => {
                commands.push(command(
                    "build",
                    &["go", "build", "./..."],
                    "high",
                    "Go module detected",
                ));
                commands.push(command(
                    "test",
                    &["go", "test", "./..."],
                    "high",
                    "Go module detected",
                ));
            }
            "tauri" => {
                commands.push(command(
                    "dev",
                    &["npm", "run", "tauri", "dev"],
                    "medium",
                    "Tauri project detected; run as a smoke/dev command, not unattended CI",
                ));
                commands.push(command(
                    "build",
                    &["npm", "run", "tauri", "build"],
                    "medium",
                    "Tauri project detected; binary build may require human smoke testing",
                ));
            }
            _ => {}
        }
    }

    commands
}

fn command(action: &str, argv: &[&str], confidence: &str, reason: &str) -> SuggestedCommand {
    SuggestedCommand {
        action: action.to_string(),
        command: argv.join(" "),
        argv: argv.iter().map(|part| part.to_string()).collect(),
        confidence: confidence.to_string(),
        reason: reason.to_string(),
    }
}

fn default_scan_schema_version() -> String {
    "probe.scan.legacy".to_string()
}

fn default_command_argv() -> Vec<String> {
    Vec::new()
}
