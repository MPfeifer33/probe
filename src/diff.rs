use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

use crate::detect::DetectedProject;
use crate::scan::{LockfileInfo, ScanResult, SuggestedCommand};
use crate::tools::ToolInfo;

#[derive(Debug, Serialize)]
pub struct DiffReport {
    pub repo_path: String,
    pub baseline_path: String,
    pub baseline_timestamp: String,
    pub current_timestamp: String,
    pub summary: DiffSummary,
    pub changes: Vec<DiffChange>,
}

#[derive(Debug, Serialize)]
pub struct DiffSummary {
    pub changes: usize,
    pub blockers: usize,
    pub warnings: usize,
    pub info: usize,
}

#[derive(Debug, Serialize)]
pub struct DiffChange {
    pub kind: String,
    pub severity: String,
    pub field: String,
    pub before: Value,
    pub after: Value,
    pub message: String,
}

pub fn build_report(
    current: &ScanResult,
    baseline: &ScanResult,
    baseline_path: &Path,
) -> DiffReport {
    let mut changes = Vec::new();

    diff_projects(&mut changes, current, baseline);
    diff_git(&mut changes, current, baseline);
    diff_tools(&mut changes, current, baseline);
    diff_lockfiles(&mut changes, current, baseline);
    diff_commands(&mut changes, current, baseline);

    let summary = summarize(&changes);

    DiffReport {
        repo_path: current.repo_path.clone(),
        baseline_path: baseline_path.display().to_string(),
        baseline_timestamp: baseline.timestamp.clone(),
        current_timestamp: current.timestamp.clone(),
        summary,
        changes,
    }
}

fn diff_projects(changes: &mut Vec<DiffChange>, current: &ScanResult, baseline: &ScanResult) {
    let before = keyed_projects(&baseline.projects);
    let after = keyed_projects(&current.projects);

    for (key, project) in &before {
        if !after.contains_key(key) {
            push_change(
                changes,
                "project",
                "warning",
                "projects",
                json!(project_label(project)),
                Value::Null,
                format!("Project removed: {}", project_label(project)),
            );
        }
    }

    for (key, project) in &after {
        if !before.contains_key(key) {
            push_change(
                changes,
                "project",
                "warning",
                "projects",
                Value::Null,
                json!(project_label(project)),
                format!("Project added: {}", project_label(project)),
            );
        }
    }
}

fn diff_git(changes: &mut Vec<DiffChange>, current: &ScanResult, baseline: &ScanResult) {
    match (&baseline.git, &current.git) {
        (None, None) => {}
        (None, Some(git)) => push_change(
            changes,
            "git",
            "info",
            "git",
            Value::Null,
            json!({ "branch": git.branch, "head_sha": git.head_sha }),
            "Git repository detected".to_string(),
        ),
        (Some(git), None) => push_change(
            changes,
            "git",
            "warning",
            "git",
            json!({ "branch": git.branch, "head_sha": git.head_sha }),
            Value::Null,
            "Git repository no longer detected".to_string(),
        ),
        (Some(before), Some(after)) => {
            if before.branch != after.branch {
                push_change(
                    changes,
                    "git",
                    "warning",
                    "git.branch",
                    json!(before.branch),
                    json!(after.branch),
                    "Git branch changed".to_string(),
                );
            }
            if before.head_sha != after.head_sha {
                push_change(
                    changes,
                    "git",
                    "info",
                    "git.head_sha",
                    json!(before.head_sha),
                    json!(after.head_sha),
                    "Git HEAD changed".to_string(),
                );
            }
            if before.dirty_count != after.dirty_count {
                push_change(
                    changes,
                    "git",
                    "warning",
                    "git.dirty_count",
                    json!(before.dirty_count),
                    json!(after.dirty_count),
                    "Dirty file count changed".to_string(),
                );
            }
            if before.untracked_count != after.untracked_count {
                push_change(
                    changes,
                    "git",
                    "warning",
                    "git.untracked_count",
                    json!(before.untracked_count),
                    json!(after.untracked_count),
                    "Untracked file count changed".to_string(),
                );
            }
            if before.ahead != after.ahead {
                push_change(
                    changes,
                    "git",
                    "info",
                    "git.ahead",
                    json!(before.ahead),
                    json!(after.ahead),
                    "Git ahead count changed".to_string(),
                );
            }
            if before.behind != after.behind {
                push_change(
                    changes,
                    "git",
                    "warning",
                    "git.behind",
                    json!(before.behind),
                    json!(after.behind),
                    "Git behind count changed".to_string(),
                );
            }
        }
    }
}

fn diff_tools(changes: &mut Vec<DiffChange>, current: &ScanResult, baseline: &ScanResult) {
    let before = keyed_tools(&baseline.tools);
    let after = keyed_tools(&current.tools);

    for (name, tool) in &before {
        match after.get(name) {
            Some(current_tool) => {
                if tool.available != current_tool.available {
                    let severity = if current_tool.available {
                        "info"
                    } else {
                        "blocker"
                    };
                    push_change(
                        changes,
                        "tool",
                        severity,
                        &format!("{name}.available"),
                        json!(tool.available),
                        json!(current_tool.available),
                        format!("Tool availability changed: {name}"),
                    );
                }
                if tool.version != current_tool.version {
                    push_change(
                        changes,
                        "tool",
                        "warning",
                        &format!("{name}.version"),
                        json!(tool.version),
                        json!(current_tool.version),
                        format!("Tool version changed: {name}"),
                    );
                }
            }
            None => push_change(
                changes,
                "tool",
                "warning",
                name,
                json!(tool_summary(tool)),
                Value::Null,
                format!("Tool removed from scan: {name}"),
            ),
        }
    }

    for (name, tool) in &after {
        if !before.contains_key(name) {
            push_change(
                changes,
                "tool",
                "info",
                name,
                Value::Null,
                json!(tool_summary(tool)),
                format!("Tool added to scan: {name}"),
            );
        }
    }
}

fn diff_lockfiles(changes: &mut Vec<DiffChange>, current: &ScanResult, baseline: &ScanResult) {
    let before = keyed_lockfiles(&baseline.lockfiles);
    let after = keyed_lockfiles(&current.lockfiles);

    for (path, lockfile) in &before {
        match after.get(path) {
            Some(current_lockfile) => {
                if lockfile.hash != current_lockfile.hash {
                    push_change(
                        changes,
                        "lockfile",
                        "warning",
                        &format!("{path}.hash"),
                        json!(lockfile.hash),
                        json!(current_lockfile.hash),
                        format!("Lockfile hash changed: {path}"),
                    );
                }
                if lockfile.stale != current_lockfile.stale {
                    push_change(
                        changes,
                        "lockfile",
                        "warning",
                        &format!("{path}.stale"),
                        json!(lockfile.stale),
                        json!(current_lockfile.stale),
                        format!("Lockfile stale state changed: {path}"),
                    );
                }
            }
            None => push_change(
                changes,
                "lockfile",
                "warning",
                path,
                json!(lockfile_summary(lockfile)),
                Value::Null,
                format!("Lockfile removed: {path}"),
            ),
        }
    }

    for (path, lockfile) in &after {
        if !before.contains_key(path) {
            push_change(
                changes,
                "lockfile",
                "warning",
                path,
                Value::Null,
                json!(lockfile_summary(lockfile)),
                format!("Lockfile added: {path}"),
            );
        }
    }
}

fn diff_commands(changes: &mut Vec<DiffChange>, current: &ScanResult, baseline: &ScanResult) {
    let before = keyed_commands(&baseline.suggested_commands);
    let after = keyed_commands(&current.suggested_commands);

    for (key, command) in &before {
        if !after.contains_key(key) {
            push_change(
                changes,
                "command",
                "info",
                key,
                json!(command_summary(command)),
                Value::Null,
                format!("Suggested command removed: {}", command.command),
            );
        }
    }

    for (key, command) in &after {
        if !before.contains_key(key) {
            push_change(
                changes,
                "command",
                "info",
                key,
                Value::Null,
                json!(command_summary(command)),
                format!("Suggested command added: {}", command.command),
            );
        }
    }
}

fn summarize(changes: &[DiffChange]) -> DiffSummary {
    DiffSummary {
        changes: changes.len(),
        blockers: changes
            .iter()
            .filter(|change| change.severity == "blocker")
            .count(),
        warnings: changes
            .iter()
            .filter(|change| change.severity == "warning")
            .count(),
        info: changes
            .iter()
            .filter(|change| change.severity == "info")
            .count(),
    }
}

fn keyed_projects(projects: &[DetectedProject]) -> BTreeMap<String, &DetectedProject> {
    projects
        .iter()
        .map(|project| {
            (
                format!("{}:{}:{}", project.kind, project.root, project.manifest),
                project,
            )
        })
        .collect()
}

fn keyed_tools(tools: &[ToolInfo]) -> BTreeMap<String, &ToolInfo> {
    tools.iter().map(|tool| (tool.name.clone(), tool)).collect()
}

fn keyed_lockfiles(lockfiles: &[LockfileInfo]) -> BTreeMap<String, &LockfileInfo> {
    lockfiles
        .iter()
        .map(|lockfile| (lockfile.path.clone(), lockfile))
        .collect()
}

fn keyed_commands(commands: &[SuggestedCommand]) -> BTreeMap<String, &SuggestedCommand> {
    commands
        .iter()
        .map(|command| (format!("{}:{}", command.action, command.command), command))
        .collect()
}

fn project_label(project: &DetectedProject) -> String {
    format!(
        "{} at {} ({})",
        project.kind, project.root, project.manifest
    )
}

fn tool_summary(tool: &ToolInfo) -> Value {
    json!({
        "available": tool.available,
        "version": tool.version,
    })
}

fn lockfile_summary(lockfile: &LockfileInfo) -> Value {
    json!({
        "hash": lockfile.hash,
        "stale": lockfile.stale,
    })
}

fn command_summary(command: &SuggestedCommand) -> Value {
    json!({
        "action": command.action,
        "command": command.command,
        "confidence": command.confidence,
    })
}

fn push_change(
    changes: &mut Vec<DiffChange>,
    kind: &str,
    severity: &str,
    field: &str,
    before: Value,
    after: Value,
    message: String,
) {
    changes.push(DiffChange {
        kind: kind.to_string(),
        severity: severity.to_string(),
        field: field.to_string(),
        before,
        after,
        message,
    });
}
