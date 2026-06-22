# PROJECT.md — probe

**What:** Agent preflight and drift scanner. Detects project type, git state, tool availability, lockfile freshness, and suggests commands — all in one scan.

**Status:** MVP implemented. Scan, snapshot, diff, doctor, docs, and integration tests are complete.

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
| diff.rs | Bjarn | Done |
| doctor.rs | Bjarn | Done |
| docs/SPEC.md | Bjarn | Done |
| README.md | Bjarn | Done |

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
probe diff [latest|path]           # compare against snapshot
probe doctor                       # actionable preflight summary
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

2026-06-22 — MVP complete; `cargo test` passes with 17 integration tests.
