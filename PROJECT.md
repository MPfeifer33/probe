# PROJECT.md — probe

**What:** Agent preflight and drift scanner. Detects project type, git state, tool availability, agent-suite linkage, lockfile freshness, and suggests commands — all in one scan. `probe brief` is the one-page cold-start read (what is this repo, what state is it in, what should I run).

**Status:** Scan, snapshot, diff, doctor, brief, docs, and integration tests are complete (33 tests, clippy clean). `brief` (2026-09-11) absorbed `stitch brief`; stitch is archived. JSON contracts expose schema versions (`probe.scan.v1`, `probe.doctor.v1`, `probe.brief.v1`), structured command args, command working directories, doctor gates/action levels, and suite-tool visibility.

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
| brief.rs | Nix | Done |
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
probe brief                        # cold-start brief: docs, stack, git, health, TODOs, commands
probe brief --format json          # same, structured (probe.brief.v1)
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
- Project detection is heuristic — checks root and shallow nested manifest
  files, not deep parsing
- Lockfile staleness: compares manifest mtime vs lockfile mtime
- Tool detection: runs version commands, extracts version numbers
- Snapshots are timestamped JSON, one file per snapshot
- `doctor` status remains data, not process failure; agents inspect JSON
  `doctor.status` and `doctor.action_level`
- Agent-suite detection is observational only; `probe` does not initialize
  latch/atlas/sentinel/witness/switchboard state
- `brief` composes `scan` + `doctor` (no new detection traits); it adds only
  cheap, bounded sources: PROJECT.md/README front matter, changed files,
  commit count, TODO markers (max 3000 files, depth 6), root markers such as
  Unity, and the sentinel summary. It never repeats hashes/versions/gates —
  `scan` and `doctor` stay the detailed views

## Last Updated

2026-09-11 — Added `probe brief` (supersedes `stitch brief`): one-page cold-start orientation composed from scan + doctor + PROJECT.md/README front matter, changed files, TODO markers, root markers, sentinel summary. Text + JSON (`probe.brief.v1`), 6 new integration tests + 4 unit tests. Degrades gracefully on non-stack repos (e.g. Unity).
