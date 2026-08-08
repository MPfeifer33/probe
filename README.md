# probe

`probe` is an agent preflight and drift scanner for project workspaces. It
answers the first question an agent has after opening a repo:

```text
What am I standing in, what changed, and what commands are likely safe?
```

It detects project stacks, git state, tool availability, lockfile freshness,
inferred build/test/run commands, and nearby agent-suite tooling. It can also
save snapshots and compare later sessions against them.

## Suite Context

Probe is part of a local-first agent tool suite centered on
[Switchboard](https://github.com/MPfeifer33/switchboard):

- [Probe](https://github.com/MPfeifer33/probe): project preflight and drift
  scanner
- [Latch](https://github.com/MPfeifer33/latch): repo-local coordination ledger
- [Atlas](https://github.com/MPfeifer33/atlas): codebase graph and impact map
- [Sentinel](https://github.com/MPfeifer33/sentinel): regression risk watcher
- [Witness](https://github.com/MPfeifer33/witness): reproducible command
  evidence recorder

## Quickstart

```sh
cargo build

# Scan the current repo.
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
probe scan
probe doctor
```

Use JSON when another tool or prompt pipeline needs stable structure:

```sh
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

### scan

```sh
probe scan
probe scan --repo /path/to/repo
probe scan --format json
```

Reports:

- project stacks: Rust, Node, Python, Go, Tauri
- shallow nested project manifests, such as `app/package.json`,
  `app/src-tauri/Cargo.toml`, or `crates/*/Cargo.toml`
- git branch, HEAD, dirty/untracked counts, ahead/behind, recent commits
- relevant tool availability and versions
- agent-suite tools such as latch, atlas, sentinel, witness, and switchboard
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
# 1. Understand the repo.
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
- `atlas`: `.agent-atlas/graph.json`
- `sentinel`: `.agent-sentinel/matrix.json`
- `witness`: `.agent-witness/`
- `switchboard`, `sieve`, and `rivet`: installed binary check only

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
