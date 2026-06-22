# probe

`probe` is an agent preflight and drift scanner for project workspaces. It
answers the first question an agent has after opening a repo:

```text
What am I standing in, what changed, and what commands are likely safe?
```

It detects project stacks, git state, tool availability, lockfile freshness,
and inferred build/test/run commands. It can also save snapshots and compare
later sessions against them.

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
- git branch, HEAD, dirty/untracked counts, ahead/behind, recent commits
- relevant tool availability and versions
- lockfile hashes and stale flags
- inferred commands with confidence

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

## Exit Codes

| Code | Meaning |
| ---- | ------- |
| `0` | Success |
| `1` | Validation or JSON error |
| `2` | IO error |

`probe doctor` returns exit code `0` even when the doctor status is `blocked`;
machine consumers should inspect `doctor.status` in JSON output.

## Design

The implementation contract is in [docs/SPEC.md](docs/SPEC.md).
