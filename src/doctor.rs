use serde::Serialize;

use crate::scan::ScanResult;
use crate::tools::SuiteToolInfo;

const DOCTOR_SCHEMA_VERSION: &str = "probe.doctor.v1";

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub schema_version: String,
    pub status: String,
    pub action_level: String,
    pub repo_path: String,
    pub gates: Vec<DoctorGate>,
    pub blockers: Vec<DoctorIssue>,
    pub warnings: Vec<DoctorIssue>,
    pub next_commands: Vec<DoctorCommand>,
    pub recommended_commands: Vec<DoctorCommand>,
    pub suite_tools: Vec<SuiteToolInfo>,
}

#[derive(Debug, Serialize)]
pub struct DoctorGate {
    pub name: String,
    pub status: String,
    pub summary: String,
    pub issue_codes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DoctorIssue {
    pub code: String,
    pub message: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorCommand {
    pub action: String,
    pub command: String,
    pub argv: Vec<String>,
    pub cwd: String,
    pub confidence: String,
    pub reason: String,
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

    if scan.git.is_none() {
        warnings.push(issue(
            "git_unavailable",
            "Git state is unavailable",
            "This path is not a git repository, or git could not inspect it",
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

    let action_level = if !blockers.is_empty() {
        "stop"
    } else if !warnings.is_empty() {
        "review"
    } else if !scan.suggested_commands.is_empty() {
        "validate"
    } else {
        "none"
    };

    let recommended_commands: Vec<DoctorCommand> = scan
        .suggested_commands
        .iter()
        .map(|command| DoctorCommand {
            action: command.action.clone(),
            command: command.command.clone(),
            argv: command.argv.clone(),
            cwd: command.cwd.clone(),
            confidence: command.confidence.clone(),
            reason: command.reason.clone(),
        })
        .collect();

    let gates = build_gates(scan, &blockers, &warnings);

    DoctorReport {
        schema_version: DOCTOR_SCHEMA_VERSION.to_string(),
        status: status.to_string(),
        action_level: action_level.to_string(),
        repo_path: scan.repo_path.clone(),
        gates,
        blockers,
        warnings,
        next_commands: recommended_commands.clone(),
        recommended_commands,
        suite_tools: scan.suite_tools.clone(),
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

fn build_gates(
    scan: &ScanResult,
    blockers: &[DoctorIssue],
    warnings: &[DoctorIssue],
) -> Vec<DoctorGate> {
    vec![
        gate(
            "project",
            &["no_projects_detected"],
            blockers,
            warnings,
            if scan.projects.is_empty() {
                "No supported project manifest detected".to_string()
            } else {
                format!(
                    "{} supported project stack(s) detected",
                    scan.projects.len()
                )
            },
        ),
        gate(
            "git_state",
            &["git_unavailable", "git_dirty", "git_behind"],
            blockers,
            warnings,
            if let Some(git) = &scan.git {
                format!(
                    "{} @ {}; {} dirty, {} untracked",
                    git.branch, git.head_sha, git.dirty_count, git.untracked_count
                )
            } else {
                "Git state unavailable".to_string()
            },
        ),
        gate(
            "tools",
            &["tool_missing", "tool_unavailable"],
            blockers,
            warnings,
            summarize_tools(scan),
        ),
        gate(
            "lockfiles",
            &["lockfile_stale"],
            blockers,
            warnings,
            if scan.lockfiles.is_empty() {
                "No lockfiles detected".to_string()
            } else {
                format!(
                    "{} lockfile(s), {} stale",
                    scan.lockfiles.len(),
                    scan.lockfiles
                        .iter()
                        .filter(|lockfile| lockfile.stale)
                        .count()
                )
            },
        ),
        gate(
            "commands",
            &["no_commands_inferred"],
            blockers,
            warnings,
            if scan.suggested_commands.is_empty() {
                "No validation commands inferred".to_string()
            } else {
                format!("{} recommended command(s)", scan.suggested_commands.len())
            },
        ),
        DoctorGate {
            name: "suite_tools".to_string(),
            status: "ok".to_string(),
            summary: summarize_suite_tools(scan),
            issue_codes: Vec::new(),
        },
    ]
}

fn gate(
    name: &str,
    relevant_codes: &[&str],
    blockers: &[DoctorIssue],
    warnings: &[DoctorIssue],
    ok_summary: String,
) -> DoctorGate {
    let blocker_codes = matching_codes(relevant_codes, blockers);
    if !blocker_codes.is_empty() {
        return DoctorGate {
            name: name.to_string(),
            status: "stop".to_string(),
            summary: summarize_issue_codes(&blocker_codes),
            issue_codes: blocker_codes,
        };
    }

    let warning_codes = matching_codes(relevant_codes, warnings);
    if !warning_codes.is_empty() {
        return DoctorGate {
            name: name.to_string(),
            status: "review".to_string(),
            summary: summarize_issue_codes(&warning_codes),
            issue_codes: warning_codes,
        };
    }

    DoctorGate {
        name: name.to_string(),
        status: "ok".to_string(),
        summary: ok_summary,
        issue_codes: Vec::new(),
    }
}

fn matching_codes(relevant_codes: &[&str], issues: &[DoctorIssue]) -> Vec<String> {
    issues
        .iter()
        .filter(|issue| relevant_codes.iter().any(|code| *code == issue.code))
        .map(|issue| issue.code.clone())
        .collect()
}

fn summarize_issue_codes(codes: &[String]) -> String {
    format!("Needs attention: {}", codes.join(", "))
}

fn summarize_tools(scan: &ScanResult) -> String {
    let available = scan.tools.iter().filter(|tool| tool.available).count();
    format!("{available}/{} stack tool(s) available", scan.tools.len())
}

fn summarize_suite_tools(scan: &ScanResult) -> String {
    let installed = scan
        .suite_tools
        .iter()
        .filter(|tool| tool.installed)
        .count();
    let linked = scan
        .suite_tools
        .iter()
        .filter(|tool| tool.initialized)
        .count();
    format!(
        "{installed}/{} suite tool(s) installed; {linked} linked/initialized for this repo",
        scan.suite_tools.len()
    )
}
