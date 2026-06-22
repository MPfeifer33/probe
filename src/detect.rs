use std::path::Path;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct DetectedProject {
    pub kind: String,
    pub root: String,
    pub manifest: String,
    pub metadata: serde_json::Map<String, Value>,
}

pub fn detect_projects(repo: &Path) -> Vec<DetectedProject> {
    let mut projects = Vec::new();

    // Rust/Cargo
    if repo.join("Cargo.toml").exists() {
        let mut metadata = serde_json::Map::new();
        if let Ok(content) = std::fs::read_to_string(repo.join("Cargo.toml")) {
            if let Some(name) = extract_toml_value(&content, "name") {
                metadata.insert("name".into(), Value::String(name));
            }
        }
        projects.push(DetectedProject {
            kind: "rust".into(),
            root: ".".into(),
            manifest: "Cargo.toml".into(),
            metadata,
        });
    }

    // Node/npm/pnpm/yarn
    if repo.join("package.json").exists() {
        let mut metadata = serde_json::Map::new();
        if let Ok(content) = std::fs::read_to_string(repo.join("package.json")) {
            if let Ok(parsed) = serde_json::from_str::<Value>(&content) {
                if let Some(name) = parsed["name"].as_str() {
                    metadata.insert("name".into(), Value::String(name.into()));
                }
                // Detect package manager
                if repo.join("pnpm-lock.yaml").exists() {
                    metadata.insert("package_manager".into(), Value::String("pnpm".into()));
                } else if repo.join("yarn.lock").exists() {
                    metadata.insert("package_manager".into(), Value::String("yarn".into()));
                } else {
                    metadata.insert("package_manager".into(), Value::String("npm".into()));
                }
            }
        }
        projects.push(DetectedProject {
            kind: "node".into(),
            root: ".".into(),
            manifest: "package.json".into(),
            metadata,
        });
    }

    // Python
    if repo.join("pyproject.toml").exists() {
        projects.push(DetectedProject {
            kind: "python".into(),
            root: ".".into(),
            manifest: "pyproject.toml".into(),
            metadata: serde_json::Map::new(),
        });
    } else if repo.join("setup.py").exists() {
        projects.push(DetectedProject {
            kind: "python".into(),
            root: ".".into(),
            manifest: "setup.py".into(),
            metadata: serde_json::Map::new(),
        });
    } else if repo.join("requirements.txt").exists() {
        projects.push(DetectedProject {
            kind: "python".into(),
            root: ".".into(),
            manifest: "requirements.txt".into(),
            metadata: serde_json::Map::new(),
        });
    }

    // Go
    if repo.join("go.mod").exists() {
        let mut metadata = serde_json::Map::new();
        if let Ok(content) = std::fs::read_to_string(repo.join("go.mod")) {
            if let Some(module) = content.lines().find(|l| l.starts_with("module ")) {
                metadata.insert("module".into(), Value::String(module.trim_start_matches("module ").trim().into()));
            }
        }
        projects.push(DetectedProject {
            kind: "go".into(),
            root: ".".into(),
            manifest: "go.mod".into(),
            metadata,
        });
    }

    // Tauri (detected by src-tauri/ directory)
    if repo.join("src-tauri").exists() && repo.join("src-tauri/Cargo.toml").exists() {
        let mut metadata = serde_json::Map::new();
        if let Ok(content) = std::fs::read_to_string(repo.join("src-tauri/Cargo.toml")) {
            if let Some(name) = extract_toml_value(&content, "name") {
                metadata.insert("name".into(), Value::String(name));
            }
        }
        projects.push(DetectedProject {
            kind: "tauri".into(),
            root: "src-tauri".into(),
            manifest: "src-tauri/Cargo.toml".into(),
            metadata,
        });
    }

    projects
}

/// Simple TOML value extractor (no full parser needed for MVP)
fn extract_toml_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(&format!("{key}")) {
            if let Some((_k, v)) = trimmed.split_once('=') {
                let v = v.trim().trim_matches('"');
                return Some(v.to_string());
            }
        }
    }
    None
}
