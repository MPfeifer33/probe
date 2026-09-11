//! `probe brief`: the compact cold-start read of a repo.
//!
//! Composes the existing scan and doctor modules with a few cheap, bounded
//! extra sources (PROJECT.md/README front matter, changed files, TODO markers,
//! sentinel summary) into one thing an agent reads first. `probe scan` stays
//! the detailed view; `brief` deliberately drops hashes, versions and gates.
//!
//! This command supersedes `stitch brief`.

use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::detect;
use crate::doctor::{self, DoctorCommand, DoctorIssue};
use crate::git::{self, ChangedFile};
use crate::scan::ScanResult;

const BRIEF_SCHEMA_VERSION: &str = "probe.brief.v1";

const CHANGED_FILES_LIMIT: usize = 10;
const RECENT_COMMITS_LIMIT: usize = 5;
const HEADINGS_LIMIT: usize = 12;
const SUMMARY_CHARS: usize = 240;
const TODO_TOP_FILES: usize = 5;
const TODO_MAX_FILES: usize = 3000;
const TODO_MAX_DEPTH: usize = 6;
const TODO_MAX_FILE_BYTES: u64 = 256 * 1024;

#[derive(Debug, Serialize)]
pub struct BriefReport {
    pub schema_version: String,
    pub timestamp: String,
    pub repo_path: String,
    pub name: String,
    pub docs: DocsBrief,
    pub markers: Vec<String>,
    pub projects: Vec<ProjectBrief>,
    pub git: Option<GitBrief>,
    pub health: HealthBrief,
    pub tools: ToolsBrief,
    pub lockfiles: LockfilesBrief,
    pub suite_tools: SuiteToolsBrief,
    pub todos: TodoBrief,
    pub sentinel: Option<SentinelBrief>,
    pub commands: Vec<DoctorCommand>,
}

#[derive(Debug, Serialize)]
pub struct DocsBrief {
    pub project_md: Option<ProjectDoc>,
    pub readme: Option<ReadmeDoc>,
    pub other: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ProjectDoc {
    pub path: String,
    pub title: Option<String>,
    pub what: Option<String>,
    pub status: Option<String>,
    pub tech: Option<String>,
    pub last_updated: Option<String>,
    pub headings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ReadmeDoc {
    pub path: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub headings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ProjectBrief {
    pub kind: String,
    pub root: String,
    pub manifest: String,
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GitBrief {
    pub branch: String,
    pub head_sha: String,
    pub dirty_count: usize,
    pub untracked_count: usize,
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
    pub commit_count: Option<usize>,
    pub changed_files: Vec<ChangedFile>,
    pub changed_files_truncated: bool,
    pub recent_commits: Vec<CommitBrief>,
}

#[derive(Debug, Serialize)]
pub struct CommitBrief {
    pub sha: String,
    pub message: String,
    pub date: String,
}

#[derive(Debug, Serialize)]
pub struct HealthBrief {
    pub status: String,
    pub action_level: String,
    pub blockers: Vec<DoctorIssue>,
    pub warnings: Vec<DoctorIssue>,
}

#[derive(Debug, Serialize)]
pub struct ToolsBrief {
    pub available: Vec<String>,
    pub missing: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LockfilesBrief {
    pub tracked: usize,
    pub stale: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SuiteToolsBrief {
    pub linked: Vec<String>,
    pub available: Vec<String>,
    pub missing: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct TodoBrief {
    pub total: usize,
    pub files_scanned: usize,
    pub truncated: bool,
    pub top_files: Vec<TodoFile>,
}

#[derive(Debug, Serialize)]
pub struct TodoFile {
    pub path: String,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SentinelBrief {
    pub tracked_files: usize,
    pub high_risk: usize,
    pub medium_risk: usize,
}

/// Build the brief from an already-completed scan.
pub fn build_report(repo: &Path, scan: &ScanResult) -> BriefReport {
    let doctor = doctor::build_report(scan);

    let name = repo
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty())
        .unwrap_or("repo")
        .to_string();

    BriefReport {
        schema_version: BRIEF_SCHEMA_VERSION.to_string(),
        timestamp: Utc::now().to_rfc3339(),
        repo_path: scan.repo_path.clone(),
        name,
        docs: gather_docs(repo),
        markers: detect_markers(repo),
        projects: scan
            .projects
            .iter()
            .map(|p| ProjectBrief {
                kind: p.kind.clone(),
                root: p.root.clone(),
                manifest: p.manifest.clone(),
                name: p
                    .metadata
                    .get("name")
                    .or_else(|| p.metadata.get("module"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            })
            .collect(),
        git: scan.git.as_ref().map(|state| {
            let (changed_files, changed_files_truncated) =
                git::get_changed_files(repo, CHANGED_FILES_LIMIT);
            GitBrief {
                branch: state.branch.clone(),
                head_sha: state.head_sha.clone(),
                dirty_count: state.dirty_count,
                untracked_count: state.untracked_count,
                ahead: state.ahead,
                behind: state.behind,
                commit_count: git::get_commit_count(repo),
                changed_files,
                changed_files_truncated,
                recent_commits: state
                    .recent_commits
                    .iter()
                    .take(RECENT_COMMITS_LIMIT)
                    .map(|c| CommitBrief {
                        sha: c.sha.clone(),
                        message: c.message.clone(),
                        date: c.date.clone(),
                    })
                    .collect(),
            }
        }),
        health: HealthBrief {
            status: doctor.status.clone(),
            action_level: doctor.action_level.clone(),
            blockers: doctor.blockers,
            warnings: doctor.warnings,
        },
        tools: ToolsBrief {
            available: scan
                .tools
                .iter()
                .filter(|t| t.available)
                .map(|t| t.name.clone())
                .collect(),
            missing: scan
                .tools
                .iter()
                .filter(|t| !t.available)
                .map(|t| t.name.clone())
                .collect(),
        },
        lockfiles: LockfilesBrief {
            tracked: scan.lockfiles.len(),
            stale: scan
                .lockfiles
                .iter()
                .filter(|l| l.stale)
                .map(|l| l.path.clone())
                .collect(),
        },
        suite_tools: SuiteToolsBrief {
            linked: suite_names_in_state(scan, "linked"),
            available: suite_names_in_state(scan, "available"),
            missing: suite_names_in_state(scan, "missing"),
        },
        todos: scan_todo_markers(repo),
        sentinel: load_sentinel_summary(repo),
        commands: doctor.recommended_commands,
    }
}

fn suite_names_in_state(scan: &ScanResult, state: &str) -> Vec<String> {
    scan.suite_tools
        .iter()
        .filter(|t| t.state == state)
        .map(|t| t.name.clone())
        .collect()
}

// ---------------------------------------------------------------------------
// Docs: PROJECT.md / README.md front matter
// ---------------------------------------------------------------------------

fn gather_docs(repo: &Path) -> DocsBrief {
    let project_md = read_doc(repo, "PROJECT.md").map(|content| parse_project_md(&content));
    let readme = read_doc(repo, "README.md").map(|content| parse_readme(&content));

    let other = [
        "CLAUDE.md",
        "AGENTS.md",
        "docs/SPEC.md",
        "docs/",
        "CHANGELOG.md",
        "CONTRIBUTING.md",
        ".agent-contract.toml",
    ]
    .iter()
    .filter(|candidate| repo.join(candidate).exists())
    .map(|candidate| candidate.to_string())
    .collect();

    DocsBrief {
        project_md,
        readme,
        other,
    }
}

fn read_doc(repo: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(repo.join(name)).ok()
}

fn parse_project_md(content: &str) -> ProjectDoc {
    let mut what = None;
    let mut status = None;
    let mut tech = None;

    for line in content.lines() {
        if let Some((label, value)) = parse_bold_label(line) {
            match label.to_ascii_lowercase().as_str() {
                "what" | "purpose" => {
                    what.get_or_insert_with(|| truncate(value, SUMMARY_CHARS));
                }
                "status" => {
                    status.get_or_insert_with(|| truncate(value, SUMMARY_CHARS));
                }
                "tech" | "stack" => {
                    tech.get_or_insert_with(|| truncate(value, SUMMARY_CHARS));
                }
                _ => {}
            }
        }
    }

    ProjectDoc {
        path: "PROJECT.md".to_string(),
        title: first_heading(content),
        what,
        status,
        tech,
        last_updated: section_first_line(content, "last updated"),
        headings: section_headings(content),
    }
}

fn parse_readme(content: &str) -> ReadmeDoc {
    ReadmeDoc {
        path: "README.md".to_string(),
        title: first_heading(content),
        summary: first_paragraph(content).map(|p| truncate(&p, SUMMARY_CHARS)),
        headings: section_headings(content),
    }
}

/// Parses `**Label:** value` lines (the PROJECT.md house style).
fn parse_bold_label(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("**")?;
    let (label, rest) = rest.split_once("**")?;
    let label = label.trim();
    // House style is `**Label:** value`; tolerate `**Label**: value` too.
    let has_colon = label.ends_with(':') || rest.trim_start().starts_with(':');
    let label = label.trim_end_matches(':');
    if !has_colon || label.is_empty() || label.len() > 24 {
        return None;
    }
    let value = rest.trim().trim_start_matches(':').trim();
    if value.is_empty() {
        return None;
    }
    Some((label, value))
}

fn first_heading(content: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let trimmed = line.trim();
        trimmed
            .strip_prefix("# ")
            .map(|title| title.trim().to_string())
    })
}

fn section_headings(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| line.trim().strip_prefix("## "))
        .map(|heading| heading.trim().to_string())
        .take(HEADINGS_LIMIT)
        .collect()
}

/// First non-empty line under a `## <name>` heading (case-insensitive).
fn section_first_line(content: &str, name: &str) -> Option<String> {
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("## ") {
            in_section = heading.trim().eq_ignore_ascii_case(name);
            continue;
        }
        if in_section && !trimmed.is_empty() && !trimmed.starts_with('#') {
            return Some(truncate(trimmed, SUMMARY_CHARS));
        }
    }
    None
}

/// First prose paragraph: skips headings, badges, blank lines and fences.
fn first_paragraph(content: &str) -> Option<String> {
    let mut paragraph: Vec<&str> = Vec::new();
    let mut in_fence = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let skip = trimmed.starts_with('#')
            || trimmed.starts_with("[![")
            || trimmed.starts_with("<")
            || trimmed.starts_with("![");
        if trimmed.is_empty() || skip {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        paragraph.push(trimmed);
    }
    if paragraph.is_empty() {
        None
    } else {
        Some(paragraph.join(" "))
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let value = value.trim();
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let cut: String = value.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{}…", cut.trim_end())
}

// ---------------------------------------------------------------------------
// Markers: things scan does not treat as a stack but an agent should know
// ---------------------------------------------------------------------------

fn detect_markers(repo: &Path) -> Vec<String> {
    let candidates: [(&str, &str); 9] = [
        ("ProjectSettings/ProjectVersion.txt", "Unity project"),
        ("Makefile", "Makefile"),
        ("justfile", "justfile"),
        ("Dockerfile", "Dockerfile"),
        ("docker-compose.yml", "docker-compose"),
        ("CMakeLists.txt", "CMake"),
        ("flake.nix", "Nix flake"),
        ("Gemfile", "Ruby bundle"),
        (".github/workflows", "GitHub workflows"),
    ];

    let mut markers: Vec<String> = candidates
        .iter()
        .filter(|(path, _)| repo.join(path).exists())
        .map(|(path, label)| format!("{label} ({path})"))
        .collect();

    if let Some(version) = unity_version(repo) {
        if let Some(first) = markers.first_mut() {
            if first.starts_with("Unity project") {
                *first = format!("Unity {version} (ProjectSettings/ProjectVersion.txt)");
            }
        }
    }

    markers
}

fn unity_version(repo: &Path) -> Option<String> {
    let content = std::fs::read_to_string(repo.join("ProjectSettings/ProjectVersion.txt")).ok()?;
    content
        .lines()
        .find_map(|line| line.strip_prefix("m_EditorVersion:"))
        .map(|v| v.trim().to_string())
}

// ---------------------------------------------------------------------------
// TODO markers: bounded walk, source-ish files only
// ---------------------------------------------------------------------------

fn scan_todo_markers(repo: &Path) -> TodoBrief {
    let mut walk = TodoWalk {
        unity: detect::is_unity_root(repo),
        ..TodoWalk::default()
    };
    walk.visit(repo, Path::new(""), 0);

    walk.files
        .sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.path.cmp(&b.path)));
    let total = walk.files.iter().map(|f| f.count).sum();
    walk.files.truncate(TODO_TOP_FILES);

    TodoBrief {
        total,
        files_scanned: walk.scanned,
        truncated: walk.truncated,
        top_files: walk.files,
    }
}

#[derive(Default)]
struct TodoWalk {
    unity: bool,
    scanned: usize,
    truncated: bool,
    files: Vec<TodoFile>,
}

impl TodoWalk {
    fn visit(&mut self, repo: &Path, rel: &Path, depth: usize) {
        if depth > TODO_MAX_DEPTH {
            self.truncated = true;
            return;
        }
        let Ok(entries) = std::fs::read_dir(repo.join(rel)) else {
            return;
        };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            if self.scanned >= TODO_MAX_FILES {
                self.truncated = true;
                return;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let child = rel.join(name.as_ref());

            if file_type.is_dir() {
                if detect::should_skip_dir_in(self.unity, &name) {
                    continue;
                }
                self.visit(repo, &child, depth + 1);
            } else if file_type.is_file() && is_source_like(&name) {
                let path = entry.path();
                let small = path
                    .metadata()
                    .map(|m| m.len() <= TODO_MAX_FILE_BYTES)
                    .unwrap_or(false);
                if !small {
                    continue;
                }
                self.scanned += 1;
                let count = std::fs::read_to_string(&path)
                    .map(|content| count_markers(&content))
                    .unwrap_or(0);
                if count > 0 {
                    self.files.push(TodoFile {
                        path: child.to_string_lossy().replace('\\', "/"),
                        count,
                    });
                }
            }
        }
    }
}

fn is_source_like(name: &str) -> bool {
    let Some(ext) = Path::new(name).extension().and_then(|e| e.to_str()) else {
        return matches!(name, "Makefile" | "justfile" | "Dockerfile");
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "rs" | "toml"
            | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "mjs"
            | "cjs"
            | "svelte"
            | "vue"
            | "py"
            | "go"
            | "cs"
            | "c"
            | "cc"
            | "cpp"
            | "h"
            | "hpp"
            | "java"
            | "kt"
            | "swift"
            | "rb"
            | "sh"
            | "bash"
            | "zsh"
            | "md"
            | "yaml"
            | "yml"
            | "css"
            | "scss"
            | "html"
            | "sql"
            | "shader"
            | "hlsl"
            | "glsl"
    )
}

fn count_markers(content: &str) -> usize {
    const MARKERS: [&str; 4] = ["TODO", "FIXME", "HACK", "XXX"];
    content
        .lines()
        .filter(|line| {
            MARKERS.iter().any(|marker| {
                line.match_indices(marker).any(|(idx, _)| {
                    let before = line[..idx].chars().next_back();
                    let after = line[idx + marker.len()..].chars().next();
                    !before.is_some_and(|c| c.is_ascii_alphanumeric())
                        && !after.is_some_and(|c| c.is_ascii_alphanumeric())
                })
            })
        })
        .count()
}

// ---------------------------------------------------------------------------
// Sentinel summary (observational; probe never writes sentinel state)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct SentinelMatrix {
    summary: SentinelBrief,
}

fn load_sentinel_summary(repo: &Path) -> Option<SentinelBrief> {
    let content = std::fs::read_to_string(repo.join(".agent-sentinel/matrix.json")).ok()?;
    serde_json::from_str::<SentinelMatrix>(&content)
        .ok()
        .map(|matrix| matrix.summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bold_label_parses_house_style() {
        assert_eq!(
            parse_bold_label("**What:** A thing."),
            Some(("What", "A thing."))
        );
        assert_eq!(
            parse_bold_label("**Purpose:** Sandbox."),
            Some(("Purpose", "Sandbox."))
        );
        assert_eq!(parse_bold_label("**bold** text without a label"), None);
        assert_eq!(parse_bold_label("plain line"), None);
    }

    #[test]
    fn markers_require_word_boundaries() {
        assert_eq!(count_markers("// TODO: x\n// FIXME y\nfoo\n"), 2);
        assert_eq!(count_markers("let TODOS = 1; // not a marker\n"), 0);
        assert_eq!(count_markers("TODO and FIXME on one line\n"), 1);
    }

    #[test]
    fn readme_paragraph_skips_badges_and_headings() {
        let content = "# Title\n\n[![ci](x)](y)\n\nFirst line\nsecond line.\n\nNext para.\n";
        assert_eq!(
            first_paragraph(content).as_deref(),
            Some("First line second line.")
        );
    }

    #[test]
    fn truncate_marks_cut() {
        assert_eq!(truncate("short", 10), "short");
        let long = "a".repeat(20);
        let cut = truncate(&long, 10);
        assert!(cut.ends_with('…'));
        assert_eq!(cut.chars().count(), 10);
    }
}
