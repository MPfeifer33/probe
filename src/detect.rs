use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct DetectedProject {
    pub kind: String,
    pub root: String,
    pub manifest: String,
    pub metadata: serde_json::Map<String, Value>,
}

pub fn detect_projects(repo: &Path) -> Vec<DetectedProject> {
    let mut projects = Vec::new();
    let mut roots = vec![PathBuf::new()];
    roots.extend(discover_nested_project_roots(repo));

    for root in roots {
        detect_projects_at(repo, &root, &mut projects);
    }

    let mut seen = BTreeSet::new();
    projects.retain(|project| {
        seen.insert((
            project.kind.clone(),
            project.root.clone(),
            project.manifest.clone(),
        ))
    });
    projects.sort_by(|a, b| {
        a.root
            .cmp(&b.root)
            .then_with(|| a.kind.cmp(&b.kind))
            .then_with(|| a.manifest.cmp(&b.manifest))
    });

    projects
}

fn detect_projects_at(repo: &Path, rel_root: &Path, projects: &mut Vec<DetectedProject>) {
    let base = repo.join(rel_root);
    if !base.is_dir() {
        return;
    }
    let root = display_root(rel_root);

    // Rust/Cargo
    if base.join("Cargo.toml").exists() {
        let mut metadata = serde_json::Map::new();
        if let Ok(content) = std::fs::read_to_string(base.join("Cargo.toml")) {
            if let Some(name) = extract_toml_value(&content, "name") {
                metadata.insert("name".into(), Value::String(name));
            }
        }
        projects.push(DetectedProject {
            kind: "rust".into(),
            root: root.clone(),
            manifest: rel_manifest(rel_root, "Cargo.toml"),
            metadata,
        });
    }

    // Node/npm/pnpm/yarn
    if base.join("package.json").exists() {
        let mut metadata = serde_json::Map::new();
        if let Ok(content) = std::fs::read_to_string(base.join("package.json")) {
            if let Ok(parsed) = serde_json::from_str::<Value>(&content) {
                if let Some(name) = parsed["name"].as_str() {
                    metadata.insert("name".into(), Value::String(name.into()));
                }
                // Detect package manager
                if base.join("pnpm-lock.yaml").exists() {
                    metadata.insert("package_manager".into(), Value::String("pnpm".into()));
                } else if base.join("yarn.lock").exists() {
                    metadata.insert("package_manager".into(), Value::String("yarn".into()));
                } else {
                    metadata.insert("package_manager".into(), Value::String("npm".into()));
                }
            }
        }
        projects.push(DetectedProject {
            kind: "node".into(),
            root: root.clone(),
            manifest: rel_manifest(rel_root, "package.json"),
            metadata,
        });
    }

    // Python
    if base.join("pyproject.toml").exists() {
        projects.push(DetectedProject {
            kind: "python".into(),
            root: root.clone(),
            manifest: rel_manifest(rel_root, "pyproject.toml"),
            metadata: serde_json::Map::new(),
        });
    } else if base.join("setup.py").exists() {
        projects.push(DetectedProject {
            kind: "python".into(),
            root: root.clone(),
            manifest: rel_manifest(rel_root, "setup.py"),
            metadata: serde_json::Map::new(),
        });
    } else if base.join("requirements.txt").exists() {
        projects.push(DetectedProject {
            kind: "python".into(),
            root: root.clone(),
            manifest: rel_manifest(rel_root, "requirements.txt"),
            metadata: serde_json::Map::new(),
        });
    }

    // Go
    if base.join("go.mod").exists() {
        let mut metadata = serde_json::Map::new();
        if let Ok(content) = std::fs::read_to_string(base.join("go.mod")) {
            if let Some(module) = content.lines().find(|l| l.starts_with("module ")) {
                metadata.insert(
                    "module".into(),
                    Value::String(module.trim_start_matches("module ").trim().into()),
                );
            }
        }
        projects.push(DetectedProject {
            kind: "go".into(),
            root: root.clone(),
            manifest: rel_manifest(rel_root, "go.mod"),
            metadata,
        });
    }

    // Tauri (detected by src-tauri/ directory)
    if base.join("src-tauri").exists() && base.join("src-tauri/Cargo.toml").exists() {
        let mut metadata = serde_json::Map::new();
        if let Ok(content) = std::fs::read_to_string(base.join("src-tauri/Cargo.toml")) {
            if let Some(name) = extract_toml_value(&content, "name") {
                metadata.insert("name".into(), Value::String(name));
            }
        }
        let tauri_root = rel_root.join("src-tauri");
        projects.push(DetectedProject {
            kind: "tauri".into(),
            root: display_root(&tauri_root),
            manifest: rel_manifest(rel_root, "src-tauri/Cargo.toml"),
            metadata,
        });
    }
}

/// Simple TOML value extractor (no full parser needed for MVP)
fn extract_toml_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some((candidate_key, v)) = trimmed.split_once('=') {
            if candidate_key.trim() == key {
                return Some(v.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

fn discover_nested_project_roots(repo: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    discover_nested_project_roots_inner(repo, Path::new(""), 0, &mut roots);
    roots.sort();
    roots.dedup();
    roots
}

fn discover_nested_project_roots_inner(
    repo: &Path,
    rel_root: &Path,
    depth: usize,
    roots: &mut Vec<PathBuf>,
) {
    const MAX_DEPTH: usize = 3;
    const MAX_ROOTS: usize = 64;

    if depth >= MAX_DEPTH || roots.len() >= MAX_ROOTS {
        return;
    }

    let current = repo.join(rel_root);
    let Ok(entries) = std::fs::read_dir(&current) else {
        return;
    };

    for entry in entries.flatten() {
        if roots.len() >= MAX_ROOTS {
            return;
        }
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if should_skip_dir(&name) {
            continue;
        }

        let child_rel = rel_root.join(name.as_ref());
        if has_project_manifest(&path) && !is_tauri_runtime_dir(&child_rel) {
            roots.push(child_rel.clone());
        }
        discover_nested_project_roots_inner(repo, &child_rel, depth + 1, roots);
    }
}

fn has_project_manifest(path: &Path) -> bool {
    [
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "setup.py",
        "requirements.txt",
        "go.mod",
    ]
    .iter()
    .any(|manifest| path.join(manifest).exists())
}

fn should_skip_dir(name: &str) -> bool {
    name.starts_with('.')
        || matches!(
            name,
            "node_modules"
                | "target"
                | "build"
                | "dist"
                | "coverage"
                | "vendor"
                | "gen"
                | "__pycache__"
        )
}

fn is_tauri_runtime_dir(rel_root: &Path) -> bool {
    rel_root.file_name().and_then(|name| name.to_str()) == Some("src-tauri")
}

fn display_root(rel_root: &Path) -> String {
    if rel_root.as_os_str().is_empty() {
        ".".to_string()
    } else {
        rel_root.to_string_lossy().replace('\\', "/")
    }
}

fn rel_manifest(rel_root: &Path, manifest: &str) -> String {
    if rel_root.as_os_str().is_empty() {
        manifest.to_string()
    } else {
        rel_root.join(manifest).to_string_lossy().replace('\\', "/")
    }
}
