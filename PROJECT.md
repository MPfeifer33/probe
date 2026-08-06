# PROJECT.md — probe

**What:** Agent preflight and drift scanner. Detects project type, git state, tool availability, agent-suite linkage, lockfile freshness, and suggests commands — all in one scan.

**Status:** Suite-hardening pass in progress. Scan, snapshot, diff, doctor, docs, and integration tests are complete; JSON contracts now expose schema versions, structured command args, doctor gates/action levels, and suite-tool visibility.

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
- `doctor` status remains data, not process failure; agents inspect JSON
  `doctor.status` and `doctor.action_level`
- Agent-suite detection is observational only; `probe` does not initialize
  latch/atlas/sentinel/witness/switchboard state

## Last Updated

2026-08-06 — Suite-hardening pass: structured scan/doctor contract, agent-suite detection, legacy snapshot compatibility, and expanded integration tests.
