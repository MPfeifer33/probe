# probe spec

Status: MVP implementation contract

`probe` is an agent preflight and drift scanner for project workspaces. It
answers the question every agent session asks first: what am I standing in,
what changed, and what commands are likely safe?

## Goals

- Give agents a quick, structured read of a repo before touching files.
- Detect project stacks, git state, tools, lockfiles, and likely commands.
- Save repo-local snapshots that survive compaction and session boundaries.
- Compare current state against a snapshot to surface drift.
- Produce an actionable doctor summary for human and agent triage.

## Non-Goals

- Deep dependency solving or package manager replacement.
- Running arbitrary build/test commands automatically.
- Persisting chat or coordination messages. That belongs in `latch`.
- Global machine inventory. `probe` is repo-scoped.

## Storage

Snapshots live under the scanned repo:

```text
.agent-probe/
  .gitignore
  snapshots/
    20260622-033900.json
```

`.agent-probe/.gitignore` contains `*` by default. Snapshot data is local
coordination state, not a product artifact.

## Commands

### scan

```sh
probe scan
probe scan --repo /path/to/project
probe scan --format json
```

Scans the repo and reports:

- detected project stacks
- git state
- tool availability and versions
- lockfile hashes and stale signals
- inferred commands

Text is the default output. JSON is available with `--format json`.

### snapshot

```sh
probe snapshot
probe snapshot --format json
```

Runs `scan` and writes the scan result as JSON under
`.agent-probe/snapshots/`.

JSON output:

```json
{
  "ok": true,
  "message": "Snapshot saved",
  "snapshot": "20260622-033900.json"
}
```

### diff

```sh
probe diff
probe diff latest
probe diff .agent-probe/snapshots/20260622-033900.json
probe diff --format json
```

Compares the current scan to a baseline snapshot. `latest` is the default.

### doctor

```sh
probe doctor
probe doctor --format json
```

Runs a scan and produces an actionable preflight summary:

- blockers: likely stop-work issues
- warnings: things to understand before editing
- next commands: likely validation commands

## Scan Schema

`probe scan --format json` returns:

```json
{
  "ok": true,
  "scan": {
    "timestamp": "2026-06-22T03:39:00Z",
    "repo_path": "/path/to/repo",
    "projects": [
      {
        "kind": "rust",
        "root": ".",
        "manifest": "Cargo.toml",
        "metadata": {
          "name": "probe"
        }
      }
    ],
    "git": {
      "branch": "master",
      "head_sha": "83a8641",
      "dirty_count": 0,
      "untracked_count": 0,
      "ahead": 0,
      "behind": 0,
      "recent_commits": [
        {
          "sha": "83a8641",
          "message": "Initial skeleton",
          "author": "Nix",
          "date": "2026-06-22T03:42:00Z"
        }
      ]
    },
    "tools": [
      {
        "name": "cargo",
        "version": "1.90.0",
        "available": true
      }
    ],
    "lockfiles": [
      {
        "path": "Cargo.lock",
        "hash": "0123456789abcdef",
        "stale": false
      }
    ],
    "suggested_commands": [
      {
        "action": "test",
        "command": "cargo test",
        "confidence": "high"
      }
    ]
  }
}
```

All arrays are allowed to be empty. `git` is `null` outside a git repository.
`ahead` and `behind` are `null` when no upstream is configured.

## Diff Schema

`probe diff --format json` returns:

```json
{
  "ok": true,
  "diff": {
    "repo_path": "/path/to/repo",
    "baseline_path": ".agent-probe/snapshots/20260622-033900.json",
    "baseline_timestamp": "2026-06-22T03:39:00Z",
    "current_timestamp": "2026-06-22T04:10:00Z",
    "summary": {
      "changes": 3,
      "blockers": 0,
      "warnings": 2,
      "info": 1
    },
    "changes": [
      {
        "kind": "tool",
        "severity": "warning",
        "field": "cargo.version",
        "before": "1.89.0",
        "after": "1.90.0",
        "message": "Tool version changed: cargo"
      }
    ]
  }
}
```

Change severities:

- `blocker`: likely invalidates the next action
- `warning`: important drift, but work can continue with care
- `info`: useful context

MVP diff categories:

- project added or removed
- git branch or HEAD changed
- dirty/untracked counts changed
- tool availability changed
- tool version changed
- lockfile hash changed
- lockfile stale flag changed
- suggested command added or removed

## Doctor Schema

`probe doctor --format json` returns:

```json
{
  "ok": true,
  "doctor": {
    "status": "ready",
    "repo_path": "/path/to/repo",
    "blockers": [],
    "warnings": [
      {
        "code": "git_dirty",
        "message": "Repository has modified files",
        "detail": "2 dirty, 1 untracked"
      }
    ],
    "next_commands": [
      {
        "action": "test",
        "command": "cargo test",
        "confidence": "high"
      }
    ]
  }
}
```

Doctor statuses:

- `ready`: no blockers or warnings
- `caution`: warnings exist, but no blockers
- `blocked`: one or more blockers exist

MVP doctor rules:

- Missing required tool for a detected stack is a blocker.
- Stale lockfiles are warnings.
- Dirty or untracked git state is a warning.
- Git behind upstream is a warning.
- No detected projects is a warning.
- No suggested commands is a warning.

## Exit Codes

| Code | Meaning |
| ---- | ------- |
| `0` | Success |
| `1` | Validation or JSON error |
| `2` | IO error |

Doctor does not fail the process for warnings or blockers in the MVP. Consumers
should inspect the JSON `doctor.status`.

## Relationship To latch

Use `probe` before work starts:

```sh
probe doctor
probe snapshot
```

Use `latch` while coordinating:

```sh
latch claim acquire src/ --intent "implementation"
latch decision add --title "..."
latch note add --kind hazard --body "..."
```

The two tools deliberately do not share storage or responsibility. `probe`
describes repo readiness and drift. `latch` records coordination decisions and
claims.
