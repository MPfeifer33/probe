# probe

`probe` is an agent preflight and drift scanner for project workspaces. It
answers the first question an agent has after opening a repo:

```text
What am I standing in, what changed, and what commands are likely safe?
```

It detects project stacks, git state, tool availability, lockfile freshness,
inferred build/test/run commands, and nearby agent-suite tooling. It can also
save snapshots and compare later sessions against them, and it produces a
compact cold-start `brief` meant to be the first thing an agent reads.

## Suite Context

Probe is part of a local-first agent tool suite centered on
[Switchboard](https://github.com/MPfeifer33/switchboard):

- [Probe](https://github.com/MPfeifer33/probe): project preflight and drift
  scanner
- [Latch](https://github.com/MPfeifer33/latch): repo-local coordination ledger
- [Sentinel](https://github.com/MPfeifer33/sentinel): regression risk watcher
- [Witness](https://github.com/MPfeifer33/witness): reproducible command
  evidence recorder

## Quickstart

```sh
cargo build

# Cold-start brief: what is this repo, what state is it in, what should I run.
cargo run -- brief

# Scan the current repo (the detailed view).
cargo run -- scan

# Save a local baseline.
cargo run -- snapshot

# Compare current state to the latest baseline.
cargo run -- diff

# Get an actionable preflight summary.
cargo run -- doctor
```

After installation, replace `cargo run --` with `probe`.

Install the CLI from a local checkout:

```sh
cargo install --path .
probe --help
```

## Output

Text is the default because `probe` is often read directly by agents and
humans:

```sh
probe brief
probe scan
probe doctor
```

Use JSON when another tool or prompt pipeline needs stable structure:

```sh
probe brief --format json
probe scan --format json
probe diff --format json
probe doctor --format json
```

## Storage

`probe snapshot` writes local snapshots under:

```text
.agent-probe/
  .gitignore
  snapshots/
    20260622-033900.json
```

`.agent-probe/` is ignored by default. It is local session state, not a product
artifact.

## Commands

### brief

```sh
probe brief
probe brief --repo /path/to/repo
probe brief --format json
```

One compact page (typically under 40 lines of text) answering the cold-start
question: what is this repo, what state is it in, what should I run. It
composes the scan and doctor results with a few cheap, bounded extra sources:

- `PROJECT.md` front matter (`**What:**`/`**Purpose:**`, `**Status:**`,
  `**Tech:**`), its `## Last Updated` line, and its section headings; falls
  back to the `README.md` title, first paragraph, and headings
- other orientation docs present at the root (`CLAUDE.md`, `AGENTS.md`,
  `docs/SPEC.md`, `docs/`, `CHANGELOG.md`, `.agent-contract.toml`)
- detected stacks (name, root) and root markers scan does not treat as a stack
  (Unity `ProjectSettings/ProjectVersion.txt`, `Makefile`, `justfile`,
  `Dockerfile`, `CMakeLists.txt`, `flake.nix`, ...)
- git branch/HEAD, dirty/untracked counts, total commit count, the changed
  file list (capped at 10), and the last 5 commits
- doctor status and action level with issue codes, missing tools, stale
  lockfiles
- `TODO`/`FIXME`/`HACK`/`XXX` marker counts from a bounded walk of source-like
  files (skips generated directories such as `target`, `node_modules`, and
  Unity `Library`/`Temp`)
- agent-suite tools grouped by linked/available/missing, and the sentinel
  risk summary when `.agent-sentinel/matrix.json` exists
- recommended commands with `cwd`-aware shell text

Hashes, tool versions, gates, and command `argv` stay in `scan`/`doctor`;
`brief` points at them instead of repeating them. Repos with no supported
stack or no git still produce a brief (stack: none, git: not a repository).

`probe brief` supersedes `stitch brief`; the `stitch` repo is archived.

### scan

```sh
probe scan
probe scan --repo /path/to/repo
probe scan --format json
```

Reports:

- project stacks: Rust, Node, Python, Go, Tauri
- shallow nested project manifests, such as `app/package.json`,
  `app/src-tauri/Cargo.toml`, or `crates/*/Cargo.toml` (generated
  directories are skipped; in a Unity project the editor's `Library/` cache is
  skipped too, and UPM `package.json` files that declare a `unity` version are
  not reported as Node projects)
- git branch, HEAD, dirty/untracked counts, ahead/behind, recent commits
- relevant tool availability and versions
- agent-suite tools such as latch, sentinel, witness, switchboard, sieve, rivet,
  acurl, and quarry
- lockfile hashes and stale flags
- inferred commands with confidence, `cwd`, structured `argv`, and a short
  reason

### snapshot

```sh
probe snapshot
probe snapshot --format json
```

Runs a scan and saves the scan JSON for later comparison.

### diff

```sh
probe diff
probe diff latest
probe diff .agent-probe/snapshots/20260622-033900.json
probe diff --format json
```

Compares the current scan to a baseline snapshot and reports drift in project
shape, git state, tools, lockfiles, and suggested commands.

### doctor

```sh
probe doctor
probe doctor --format json
```

Summarizes the repo into:

- `ready`: no blockers or warnings
- `caution`: warnings exist, but no blockers
- `blocked`: one or more blockers exist

JSON doctor reports also expose an `action_level` for agents:

- `stop`: a blocker needs human/agent attention before normal work
- `review`: warnings exist; read them before editing
- `validate`: no warnings/blockers and recommended commands exist
- `none`: no clear action was inferred

The report includes named `gates`, `recommended_commands`, and `suite_tools`.
`next_commands` remains as a compatibility alias for older consumers.
Recommended commands include a `cwd` field. When a command belongs to a nested
project, the human-readable `command` includes the matching `cd ... && ...`
prefix so agents can copy it without guessing the working directory.

Doctor checks are conservative. It does not run build or test commands; it
only tells you what looks safe to run next.

## Typical Agent Flow

```sh
# 1. Orient, then check readiness.
probe brief
probe doctor

# 2. Save a baseline before touching files.
probe snapshot

# 3. Coordinate active work with latch.
latch claim acquire src/ --intent "implementation"

# 4. Work, test, and inspect drift.
probe diff
```

`probe` complements `latch`: `probe` describes repo readiness and drift;
`latch` persists coordination claims, decisions, tasks, and hazards.

For the current public-agent-tool suite, `probe scan`/`probe doctor` also
report whether related tools are installed and whether their repo-local state
appears initialized:

- `probe`: `.agent-probe/`
- `latch`: `.agent-workspace/workspace.sqlite`
- `sentinel`: `.agent-sentinel/matrix.json`
- `witness`: `.agent-witness/`
- `acurl`: `.agent-acurl/`
- `switchboard`, `sieve`, `rivet`, and `quarry`: installed binary check only

Archived tools (atlas, stitch, trail, harbor, loom, mender) are no longer
listed.

## Exit Codes

| Code | Meaning |
| ---- | ------- |
| `0` | Success |
| `1` | Validation or JSON error |
| `2` | IO error |

`probe doctor` returns exit code `0` even when the doctor status is `blocked`;
machine consumers should inspect `doctor.status` and `doctor.action_level` in
JSON output.

## Design

The implementation contract is in [docs/SPEC.md](docs/SPEC.md).

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) and
[NOTICE](NOTICE). Redistributed or derivative works must preserve the NOTICE
attribution required by the license.
