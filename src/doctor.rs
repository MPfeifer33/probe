use serde::Serialize;

use crate::scan::ScanResult;

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub status: String,
    pub repo_path: String,
    pub blockers: Vec<DoctorIssue>,
    pub warnings: Vec<DoctorIssue>,
    pub next_commands: Vec<DoctorCommand>,
}

#[derive(Debug, Serialize)]
pub struct DoctorIssue {
    pub code: String,
    pub message: String,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct DoctorCommand {
    pub action: String,
    pub command: String,
    pub confidence: String,
}

pub fn build_report(scan: &ScanResult) -> DoctorReport {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    if scan.projects.is_empty() {
        warnings.push(issue(
            "no_projects_detected",
            "No supported project manifests detected",
            "Supported stacks: Rust, Node, Python, Go, Tauri",
        ));
    }

    for tool in &scan.tools {
        if !tool.available {
            if is_required_tool(&tool.name) {
                blockers.push(issue(
                    "tool_missing",
                    format!("Required tool missing: {}", tool.name),
                    format!("Install {} or make it available on PATH", tool.name),
                ));
            } else {
                warnings.push(issue(
                    "tool_unavailable",
                    format!("Optional tool unavailable: {}", tool.name),
                    "Some validation commands may be unavailable",
                ));
            }
        }
    }

    for lockfile in &scan.lockfiles {
        if lockfile.stale {
            warnings.push(issue(
                "lockfile_stale",
                format!("Lockfile may be stale: {}", lockfile.path),
                "Manifest modification time is newer than the lockfile",
            ));
        }
    }

    if let Some(git) = &scan.git {
        if git.dirty_count > 0 || git.untracked_count > 0 {
            warnings.push(issue(
                "git_dirty",
                "Repository has modified files",
                format!(
                    "{} dirty, {} untracked",
                    git.dirty_count, git.untracked_count
                ),
            ));
        }

        if git.behind.unwrap_or(0) > 0 {
            warnings.push(issue(
                "git_behind",
                "Branch is behind upstream",
                format!("{} commits behind upstream", git.behind.unwrap_or(0)),
            ));
        }
    }

    if scan.suggested_commands.is_empty() {
        warnings.push(issue(
            "no_commands_inferred",
            "No validation commands inferred",
            "Add project manifests or run project-specific commands manually",
        ));
    }

    let status = if !blockers.is_empty() {
        "blocked"
    } else if !warnings.is_empty() {
        "caution"
    } else {
        "ready"
    };

    DoctorReport {
        status: status.to_string(),
        repo_path: scan.repo_path.clone(),
        blockers,
        warnings,
        next_commands: scan
            .suggested_commands
            .iter()
            .map(|command| DoctorCommand {
                action: command.action.clone(),
                command: command.command.clone(),
                confidence: command.confidence.clone(),
            })
            .collect(),
    }
}

fn is_required_tool(name: &str) -> bool {
    matches!(
        name,
        "cargo" | "rustc" | "node" | "npm" | "pnpm" | "yarn" | "python3" | "go"
    )
}

fn issue(
    code: impl Into<String>,
    message: impl Into<String>,
    detail: impl Into<String>,
) -> DoctorIssue {
    DoctorIssue {
        code: code.into(),
        message: message.into(),
        detail: detail.into(),
    }
}
