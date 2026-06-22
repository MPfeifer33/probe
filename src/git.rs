use std::path::Path;
use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GitState {
    pub branch: String,
    pub head_sha: String,
    pub dirty_count: usize,
    pub untracked_count: usize,
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
    pub recent_commits: Vec<RecentCommit>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecentCommit {
    pub sha: String,
    pub message: String,
    pub author: String,
    pub date: String,
}

pub fn get_state(repo: &Path) -> Option<GitState> {
    // Check if it's a git repo
    let output = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(repo)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let branch = get_branch(repo).unwrap_or_else(|| "HEAD".into());
    let head_sha = get_head_sha(repo).unwrap_or_default();
    let (dirty_count, untracked_count) = get_dirty_counts(repo);
    let (ahead, behind) = get_ahead_behind(repo);
    let recent_commits = get_recent_commits(repo, 5);

    Some(GitState {
        branch,
        head_sha,
        dirty_count,
        untracked_count,
        ahead,
        behind,
        recent_commits,
    })
}

fn get_branch(repo: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(repo)
        .output()
        .ok()?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch.is_empty() {
            None
        } else {
            Some(branch)
        }
    } else {
        None
    }
}

fn get_head_sha(repo: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(repo)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn get_dirty_counts(repo: &Path) -> (usize, usize) {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            let mut dirty = 0;
            let mut untracked = 0;
            for line in text.lines() {
                if line.starts_with("??") {
                    untracked += 1;
                } else if !line.is_empty() {
                    dirty += 1;
                }
            }
            (dirty, untracked)
        }
        _ => (0, 0),
    }
}

fn get_ahead_behind(repo: &Path) -> (Option<usize>, Option<usize>) {
    let output = Command::new("git")
        .args(["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
        .current_dir(repo)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let parts: Vec<&str> = text.split_whitespace().collect();
            if parts.len() == 2 {
                let ahead = parts[0].parse().ok();
                let behind = parts[1].parse().ok();
                (ahead, behind)
            } else {
                (None, None)
            }
        }
        _ => (None, None),
    }
}

fn get_recent_commits(repo: &Path, count: usize) -> Vec<RecentCommit> {
    let output = Command::new("git")
        .args(["log", &format!("-{count}"), "--format=%h|%s|%an|%aI"])
        .current_dir(repo)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.splitn(4, '|').collect();
                    if parts.len() == 4 {
                        Some(RecentCommit {
                            sha: parts[0].to_string(),
                            message: parts[1].to_string(),
                            author: parts[2].to_string(),
                            date: parts[3].to_string(),
                        })
                    } else {
                        None
                    }
                })
                .collect()
        }
        _ => Vec::new(),
    }
}
