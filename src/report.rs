use crate::scan::ScanResult;
use crate::ProbeError;
use crate::{diff::DiffReport, doctor::DoctorReport};

pub fn print_scan(result: &ScanResult, is_json: bool) -> Result<(), ProbeError> {
    if is_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "scan": result,
            }))?
        );
    } else {
        print_text(result);
    }
    Ok(())
}

pub fn print_diff(result: &DiffReport, is_json: bool) -> Result<(), ProbeError> {
    if is_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "diff": result,
            }))?
        );
    } else {
        print_diff_text(result);
    }
    Ok(())
}

pub fn print_doctor(result: &DoctorReport, is_json: bool) -> Result<(), ProbeError> {
    if is_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "doctor": result,
            }))?
        );
    } else {
        print_doctor_text(result);
    }
    Ok(())
}

fn print_text(result: &ScanResult) {
    println!("probe scan: {}", result.repo_path);
    println!();

    // Projects
    if result.projects.is_empty() {
        println!("  Projects: none detected");
    } else {
        println!("  Projects:");
        for p in &result.projects {
            let name = p
                .metadata
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if name.is_empty() {
                println!("    {} ({})", p.kind, p.manifest);
            } else {
                println!("    {} \"{}\" ({})", p.kind, name, p.manifest);
            }
        }
    }
    println!();

    // Git
    if let Some(ref git) = result.git {
        println!("  Git: {} @ {}", git.branch, git.head_sha);
        if git.dirty_count > 0 || git.untracked_count > 0 {
            println!(
                "    {} dirty, {} untracked",
                git.dirty_count, git.untracked_count
            );
        }
        if let (Some(ahead), Some(behind)) = (git.ahead, git.behind) {
            if ahead > 0 || behind > 0 {
                println!("    ahead: {ahead}, behind: {behind}");
            }
        }
        if !git.recent_commits.is_empty() {
            println!("    Recent:");
            for c in git.recent_commits.iter().take(3) {
                println!("      {} {}", c.sha, c.message);
            }
        }
    } else {
        println!("  Git: not a git repository");
    }
    println!();

    // Tools
    println!("  Tools:");
    for t in &result.tools {
        if t.available {
            println!("    ✓ {} {}", t.name, t.version.as_deref().unwrap_or(""));
        } else {
            println!("    ✗ {} (not found)", t.name);
        }
    }
    println!();

    // Lockfiles
    if !result.lockfiles.is_empty() {
        println!("  Lockfiles:");
        for l in &result.lockfiles {
            let status = if l.stale { " [STALE]" } else { "" };
            println!(
                "    {} ({}){}",
                l.path,
                &l.hash[..8.min(l.hash.len())],
                status
            );
        }
        println!();
    }

    // Suggested commands
    if !result.suggested_commands.is_empty() {
        println!("  Commands:");
        for cmd in &result.suggested_commands {
            println!(
                "    {} → `{}` [{}]",
                cmd.action, cmd.command, cmd.confidence
            );
        }
    }
}

fn print_diff_text(result: &DiffReport) {
    println!("probe diff: {}", result.repo_path);
    println!("  Baseline: {}", result.baseline_path);
    println!("  Baseline timestamp: {}", result.baseline_timestamp);
    println!("  Current timestamp: {}", result.current_timestamp);
    println!();

    if result.changes.is_empty() {
        println!("  No drift detected.");
        return;
    }

    println!(
        "  Summary: {} changes, {} blockers, {} warnings, {} info",
        result.summary.changes,
        result.summary.blockers,
        result.summary.warnings,
        result.summary.info
    );
    println!();

    for change in &result.changes {
        println!(
            "  [{}] {} {}: {}",
            change.severity, change.kind, change.field, change.message
        );
    }
}

fn print_doctor_text(result: &DoctorReport) {
    println!("probe doctor: {} ({})", result.status, result.repo_path);
    println!();

    print_issues("Blockers", &result.blockers);
    print_issues("Warnings", &result.warnings);

    if !result.next_commands.is_empty() {
        println!("  Next commands:");
        for command in &result.next_commands {
            println!(
                "    {} -> `{}` [{}]",
                command.action, command.command, command.confidence
            );
        }
    }
}

fn print_issues(label: &str, issues: &[crate::doctor::DoctorIssue]) {
    println!("  {label}:");
    if issues.is_empty() {
        println!("    none");
    } else {
        for issue in issues {
            println!("    [{}] {} ({})", issue.code, issue.message, issue.detail);
        }
    }
    println!();
}
