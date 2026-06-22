# PROJECT.md — probe

**What:** Agent preflight and drift scanner. Detects project type, git state, tool availability, lockfile freshness, and suggests commands — all in one scan.

**Status:** Scan and snapshot implemented. Diff and doctor are stubs for Bjarn.

**Tech:** Rust 2021, clap 4, serde/serde_json, chrono, sha2, thiserror.

**Storage:** `.agent-probe/snapshots/` under repo root, gitignored by default.

## Module Ownership

| Module | Owner | Status |
|--------|-------|--------|
| cli.rs | Nix | Done |
| main.rs | Nix | Done |
| scan.rs | Nix | Done |
| detect.rs | Nix | Done |
| git.rs | Nix | Done |
| tools.rs | Nix | Done |
| snapshot.rs | Nix | Done |
| report.rs | Nix | Done |
| diff (TBD) | Bjarn | Stub |
| doctor (TBD) | Bjarn | Stub |

## Build

```sh
cargo build
cargo check
cargo test
```

## Usage

```sh
probe scan                         # text report of current project
probe scan --format json           # structured JSON output
probe scan --repo /path/to/project # scan a different project
probe snapshot                     # save current state for later diff
probe diff [latest|path]           # compare against snapshot (stub)
probe doctor                       # actionable preflight summary (stub)
```

## Detected Stacks

- Rust/Cargo
- Node (npm/pnpm/yarn auto-detected)
- Python (pyproject.toml, setup.py, requirements.txt)
- Go
- Tauri (src-tauri/ detection)

## Key Design Choices

- Text output by default (agents read text; use --format json for structured consumption)
- Project detection is heuristic — checks for manifest files, not deep parsing
- Lockfile staleness: compares manifest mtime vs lockfile mtime
- Tool detection: runs version commands, extracts version numbers
- Snapshots are timestamped JSON, one file per snapshot

## Last Updated

2026-06-22 — Initial skeleton with scan/snapshot (Nix)
