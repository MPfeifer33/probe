use crate::brief::BriefReport;
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

pub fn print_brief(result: &BriefReport, is_json: bool) -> Result<(), ProbeError> {
    if is_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "brief": result,
            }))?
        );
    } else {
        print_brief_text(result);
    }
    Ok(())
}

fn print_text(result: &ScanResult) {
    println!("probe scan: {}", result.repo_path);
    println!("  Schema: {}", result.schema_version);
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

    // Agent suite tools
    if !result.suite_tools.is_empty() {
        println!("  Agent suite:");
        for tool in &result.suite_tools {
            let marker = if tool.installed { "✓" } else { "✗" };
            let state_path = tool
                .state_path
                .as_ref()
                .map(|path| format!(" ({path})"))
                .unwrap_or_default();
            println!(
                "    {marker} {} — {} [{}]{}",
                tool.name, tool.role, tool.state, state_path
            );
        }
        println!();
    }

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
    println!("  Action: {}", result.action_level);
    println!();

    if !result.gates.is_empty() {
        println!("  Gates:");
        for gate in &result.gates {
            println!("    {}: {} — {}", gate.name, gate.status, gate.summary);
        }
        println!();
    }

    print_issues("Blockers", &result.blockers);
    print_issues("Warnings", &result.warnings);

    if !result.recommended_commands.is_empty() {
        println!("  Recommended commands:");
        for command in &result.recommended_commands {
            println!(
                "    {} -> `{}` [{}] — {}",
                command.action, command.command, command.confidence, command.reason
            );
        }
    }

    if !result.suite_tools.is_empty() {
        println!();
        println!("  Suite tools:");
        for tool in &result.suite_tools {
            let marker = if tool.installed { "✓" } else { "✗" };
            println!("    {marker} {} [{}]", tool.name, tool.state);
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

fn print_brief_text(result: &BriefReport) {
    println!("probe brief: {} ({})", result.name, result.repo_path);
    println!();

    // What / state, from the docs.
    let mut described = false;
    if let Some(doc) = &result.docs.project_md {
        if let Some(what) = &doc.what {
            println!("  What: {what}");
            described = true;
        }
        if let Some(status) = &doc.status {
            println!("  Status: {status}");
        }
        if let Some(tech) = &doc.tech {
            println!("  Tech: {tech}");
        }
        if let Some(updated) = &doc.last_updated {
            println!("  Last updated: {updated}");
        }
    }
    if !described {
        if let Some(readme) = &result.docs.readme {
            if let Some(summary) = &readme.summary {
                println!("  What: {summary}");
            }
        }
    }
    let mut docs = Vec::new();
    if let Some(doc) = &result.docs.project_md {
        if doc.headings.is_empty() {
            docs.push(doc.path.clone());
        } else {
            docs.push(format!("{} [{}]", doc.path, doc.headings.join(", ")));
        }
    }
    if let Some(readme) = &result.docs.readme {
        if readme.headings.is_empty() {
            docs.push(readme.path.clone());
        } else {
            docs.push(format!("{} [{}]", readme.path, readme.headings.join(", ")));
        }
    }
    docs.extend(result.docs.other.iter().cloned());
    if docs.is_empty() {
        println!("  Docs: none found (no PROJECT.md or README.md)");
    } else {
        println!("  Docs: {}", docs.join(" · "));
    }
    println!();

    // Stack.
    if result.projects.is_empty() {
        println!("  Stack: none detected (supported: Rust, Node, Python, Go, Tauri)");
    } else {
        let stacks: Vec<String> = result
            .projects
            .iter()
            .map(|p| {
                let name = p
                    .name
                    .as_ref()
                    .map(|n| format!(" \"{n}\""))
                    .unwrap_or_default();
                if p.root == "." {
                    format!("{}{} ({})", p.kind, name, p.manifest)
                } else {
                    format!("{}{} @ {}", p.kind, name, p.root)
                }
            })
            .collect();
        println!("  Stack: {}", stacks.join(" · "));
    }
    if !result.markers.is_empty() {
        println!("  Markers: {}", result.markers.join(" · "));
    }

    // Git.
    match &result.git {
        Some(git) => {
            let mut state = if git.dirty_count == 0 && git.untracked_count == 0 {
                "clean".to_string()
            } else {
                format!(
                    "{} dirty, {} untracked",
                    git.dirty_count, git.untracked_count
                )
            };
            if let (Some(ahead), Some(behind)) = (git.ahead, git.behind) {
                if ahead > 0 || behind > 0 {
                    state.push_str(&format!(", ahead {ahead}/behind {behind}"));
                }
            }
            if let Some(count) = git.commit_count {
                state.push_str(&format!(", {count} commits"));
            }
            println!("  Git: {} @ {} — {}", git.branch, git.head_sha, state);
            if !git.changed_files.is_empty() {
                let mut changed: Vec<String> = git
                    .changed_files
                    .iter()
                    .map(|c| format!("{} {}", c.status, c.path))
                    .collect();
                if git.changed_files_truncated {
                    changed.push("…".to_string());
                }
                println!("    Changed: {}", changed.join(", "));
            }
            if !git.recent_commits.is_empty() {
                println!("    Recent:");
                for commit in &git.recent_commits {
                    println!("      {} {}", commit.sha, commit.message);
                }
            }
        }
        None => println!("  Git: not a git repository"),
    }
    println!();

    // Health, one line plus issue codes.
    let mut issues: Vec<String> = result
        .health
        .blockers
        .iter()
        .map(|i| format!("BLOCKER {}: {}", i.code, i.detail))
        .collect();
    issues.extend(
        result
            .health
            .warnings
            .iter()
            .map(|i| format!("{}: {}", i.code, i.detail)),
    );
    println!(
        "  Health: {} ({})",
        result.health.status, result.health.action_level
    );
    for issue in issues.iter().take(6) {
        println!("    {issue}");
    }
    if issues.len() > 6 {
        println!("    … {} more (see probe doctor)", issues.len() - 6);
    }
    if !result.tools.missing.is_empty() {
        println!("  Missing tools: {}", result.tools.missing.join(", "));
    }
    if !result.lockfiles.stale.is_empty() {
        println!("  Stale lockfiles: {}", result.lockfiles.stale.join(", "));
    }

    // Markers in source.
    if result.todos.total == 0 {
        println!(
            "  TODO markers: none in {} files scanned{}",
            result.todos.files_scanned,
            if result.todos.truncated {
                " (bounded)"
            } else {
                ""
            }
        );
    } else {
        let top: Vec<String> = result
            .todos
            .top_files
            .iter()
            .map(|f| format!("{} ×{}", f.path, f.count))
            .collect();
        println!(
            "  TODO markers: {} across {} files scanned{} — {}",
            result.todos.total,
            result.todos.files_scanned,
            if result.todos.truncated {
                " (bounded)"
            } else {
                ""
            },
            top.join(", ")
        );
    }

    // Suite + sentinel.
    let mut suite = Vec::new();
    if !result.suite_tools.linked.is_empty() {
        suite.push(format!("linked {}", result.suite_tools.linked.join(", ")));
    }
    if !result.suite_tools.available.is_empty() {
        suite.push(format!(
            "available {}",
            result.suite_tools.available.join(", ")
        ));
    }
    if !result.suite_tools.missing.is_empty() {
        suite.push(format!("missing {}", result.suite_tools.missing.join(", ")));
    }
    if !suite.is_empty() {
        println!("  Suite: {}", suite.join(" · "));
    }
    if let Some(sentinel) = &result.sentinel {
        println!(
            "  Sentinel: {} tracked, {} high-risk, {} medium-risk",
            sentinel.tracked_files, sentinel.high_risk, sentinel.medium_risk
        );
    }
    println!();

    // What to run.
    if result.commands.is_empty() {
        println!("  Run: no commands inferred; read the docs above for build/run steps");
    } else {
        println!("  Run:");
        for command in &result.commands {
            println!(
                "    {:<8} {}  [{}]",
                command.action, command.command, command.confidence
            );
        }
    }
    println!();
    println!("  Detail: probe scan · probe doctor");
}
