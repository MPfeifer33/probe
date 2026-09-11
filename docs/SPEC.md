# probe spec

Status: MVP implementation contract

`probe` is an agent preflight and drift scanner for project workspaces. It
answers the question every agent session asks first: what am I standing in,
what changed, and what commands are likely safe?

## Goals

- Give agents a quick, structured read of a repo before touching files.
- Answer the cold-start question in one command (`brief`): what is this repo,
  what state is it in, what should I run. This absorbs the former
  `stitch brief` so one tool covers orientation.
- Detect project stacks, git state, tools, lockfiles, and likely commands.
- Save repo-local snapshots that survive compaction and session boundaries.
- Compare current state against a snapshot to surface drift.
- Produce an actionable doctor summary for human and agent triage.
- Surface nearby agent-suite tooling without making `probe` responsible for
  those tools' storage or truth.

## Non-Goals

- Deep dependency solving or package manager replacement.
- Running arbitrary build/test commands automatically.
- Persisting chat or coordination messages. That belongs in `latch`.
- Global machine inventory. `probe` is repo-scoped.
- Mutating or initializing other tools' state. Suite-tool detection is
  observational only.

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

### brief

```sh
probe brief
probe brief --repo /path/to/project
probe brief --format json
```

Runs `scan` and `doctor` and composes them with bounded extra sources into a
compact orientation brief:

- `PROJECT.md` front matter and headings, or `README.md` title/first paragraph
  and headings when no `PROJECT.md` exists
- other root orientation docs present (`CLAUDE.md`, `AGENTS.md`,
  `docs/SPEC.md`, `docs/`, `CHANGELOG.md`, `CONTRIBUTING.md`,
  `.agent-contract.toml`)
- root markers outside the stack model (Unity, Makefile, justfile, Dockerfile,
  docker-compose, CMake, Nix flake, Gemfile, GitHub workflows)
- git state plus changed files (max 10) and total commit count
- doctor status/action level with blockers and warnings, missing tools, stale
  lockfiles
- TODO/FIXME/HACK/XXX marker counts (bounded: max 3000 source-like files,
  depth 6, files up to 256 KiB, generated dirs skipped)
- suite tools grouped by state, sentinel summary when present
- recommended commands

Text output targets well under 60 lines for a typical repo. `scan` remains the
detailed view; `brief` never repeats hashes, tool versions, or gates.

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
- public agent-suite tool availability and repo-local linkage
- lockfile hashes and stale signals
- inferred commands with `argv` and reasons

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
- gates: named health checks with stable issue-code references
- recommended commands: likely validation commands with shell text, `argv`,
  confidence, and reason
- suite tools: installed/linked status for related agent-first tools

## Brief Schema

`probe brief --format json` returns:

```json
{
  "ok": true,
  "brief": {
    "schema_version": "probe.brief.v1",
    "timestamp": "2026-09-11T20:00:00Z",
    "repo_path": "/path/to/repo",
    "name": "repo",
    "docs": {
      "project_md": {
        "path": "PROJECT.md",
        "title": "PROJECT.md — repo",
        "what": "One-line purpose from **What:** or **Purpose:**",
        "status": "From **Status:**",
        "tech": "From **Tech:** or **Stack:**",
        "last_updated": "First line under ## Last Updated",
        "headings": ["Layout", "Build", "Last Updated"]
      },
      "readme": null,
      "other": ["docs/SPEC.md", "docs/"]
    },
    "markers": ["Makefile (Makefile)"],
    "projects": [
      { "kind": "rust", "root": ".", "manifest": "Cargo.toml", "name": "repo" }
    ],
    "git": {
      "branch": "master",
      "head_sha": "83a8641",
      "dirty_count": 1,
      "untracked_count": 0,
      "ahead": null,
      "behind": null,
      "commit_count": 42,
      "changed_files": [{ "status": "M", "path": "src/main.rs" }],
      "changed_files_truncated": false,
      "recent_commits": [
        { "sha": "83a8641", "message": "Initial skeleton", "date": "2026-06-22T03:42:00Z" }
      ]
    },
    "health": {
      "status": "caution",
      "action_level": "review",
      "blockers": [],
      "warnings": [
        { "code": "git_dirty", "message": "Repository has modified files", "detail": "1 dirty, 0 untracked" }
      ]
    },
    "tools": { "available": ["git", "rustc", "cargo"], "missing": [] },
    "lockfiles": { "tracked": 1, "stale": [] },
    "suite_tools": { "linked": ["probe"], "available": ["latch"], "missing": ["atlas"] },
    "todos": {
      "total": 3,
      "files_scanned": 19,
      "truncated": false,
      "top_files": [{ "path": "src/main.rs", "count": 3 }]
    },
    "sentinel": null,
    "commands": [
      {
        "action": "test",
        "command": "cargo test",
        "cwd": ".",
        "argv": ["cargo", "test"],
        "confidence": "high",
        "reason": "Rust manifest detected"
      }
    ]
  }
}
```

Brief rules:

- `docs.project_md` and `docs.readme` are `null` when the file is absent; text
  values are trimmed to 240 characters with a trailing ellipsis.
- `git` is `null` outside a git repository; `commit_count` is `null` when git
  cannot count.
- `health` mirrors the doctor status, action level, blockers, and warnings but
  omits gates and the compatibility `next_commands` alias.
- `commands` uses the doctor command shape (`argv`, `cwd`, `confidence`,
  `reason`).
- `sentinel` is populated only from an existing `.agent-sentinel/matrix.json`
  `summary` block; `probe` never writes sentinel state.
- `todos.truncated` is `true` when the bounded walk hit its file or depth cap.

## Scan Schema

`probe scan --format json` returns:

```json
{
  "ok": true,
  "scan": {
    "timestamp": "2026-06-22T03:39:00Z",
    "schema_version": "probe.scan.v1",
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
          "author": "Reviewer",
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
    "suite_tools": [
      {
        "name": "latch",
        "binary": "latch",
        "role": "repo-local coordination ledger",
        "installed": true,
        "version": "0.1.0",
        "state_path": ".agent-workspace/workspace.sqlite",
        "initialized": true,
        "state": "linked"
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
        "cwd": ".",
        "argv": ["cargo", "test"],
        "confidence": "high",
        "reason": "Rust manifest detected"
      }
    ]
  }
}
```

All arrays are allowed to be empty. `git` is `null` outside a git repository.
`ahead` and `behind` are `null` when no upstream is configured. Project
detection starts at the scanned repo root and also checks shallow nested
manifest roots, such as `app/package.json`, `app/src-tauri/Cargo.toml`, and
`crates/*/Cargo.toml`. Generated/dependency directories such as
`node_modules`, `target`, `dist`, and hidden dot directories are skipped.
Snapshots written before `probe.scan.v1` remain readable; missing
`schema_version`, `suite_tools`, `cwd`, `argv`, or `reason` fields are treated
as legacy-compatible defaults.

Suggested command rules:

- `argv` is the structured command and arguments to execute.
- `cwd` is the directory where the command should run, relative to `repo_path`.
- `command` is a display/copy string. For nested projects it includes a
  `cd <cwd> && ...` prefix so agents do not have to infer execution context.

Suite-tool states:

- `missing`: the binary was not found on `PATH`
- `available`: the binary exists, but the expected repo-local state path is not
  present
- `linked`: the binary exists and the expected repo-local state path is present

Tools without repo-local state paths, such as `switchboard`, are considered
initialized when their binary is installed. `version` may be `null` for tools
that are installed and answer `--help` but do not expose a `--version` flag.

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
- suite tool installed/version/linkage changed
- suite tool added to or removed from the scan contract
- lockfile hash changed
- lockfile stale flag changed
- suggested command added or removed

## Doctor Schema

`probe doctor --format json` returns:

```json
{
  "ok": true,
  "doctor": {
    "schema_version": "probe.doctor.v1",
    "status": "ready",
    "action_level": "validate",
    "repo_path": "/path/to/repo",
    "gates": [
      {
        "name": "git_state",
        "status": "ok",
        "summary": "master @ 83a8641; 0 dirty, 0 untracked",
        "issue_codes": []
      }
    ],
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
        "cwd": ".",
        "argv": ["cargo", "test"],
        "confidence": "high",
        "reason": "Rust manifest detected"
      }
    ],
    "recommended_commands": [
      {
        "action": "test",
        "command": "cargo test",
        "cwd": ".",
        "argv": ["cargo", "test"],
        "confidence": "high",
        "reason": "Rust manifest detected"
      }
    ],
    "suite_tools": []
  }
}
```

Doctor statuses:

- `ready`: no blockers or warnings
- `caution`: warnings exist, but no blockers
- `blocked`: one or more blockers exist

Doctor action levels:

- `stop`: one or more blockers exist
- `review`: warnings exist, but no blockers
- `validate`: no blockers/warnings and at least one recommended command exists
- `none`: no blocker, warning, or recommended validation command exists

Doctor gates:

- `project`: detected project manifests
- `git_state`: dirty/untracked/behind/unavailable git state
- `tools`: required and optional stack tools
- `lockfiles`: stale or missing lockfile signals
- `commands`: inferred validation commands
- `suite_tools`: related agent-suite binaries and repo-local linkage

MVP doctor rules:

- Missing required tool for a detected stack is a blocker.
- Stale lockfiles are warnings.
- Dirty or untracked git state is a warning.
- Git behind upstream is a warning.
- Unavailable git state is a warning.
- No detected projects is a warning.
- No suggested commands is a warning.
- `recommended_commands` is the canonical command list for new consumers.
  `next_commands` is retained as a compatibility alias.

## Exit Codes

| Code | Meaning |
| ---- | ------- |
| `0` | Success |
| `1` | Validation or JSON error |
| `2` | IO error |

Doctor does not fail the process for warnings or blockers in the MVP. Consumers
should inspect the JSON `doctor.status` and `doctor.action_level`.

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

The tools deliberately do not share storage or responsibility. `probe`
describes repo readiness, drift, recommended local validation, and whether the
rest of the public agent-tool suite is visible. `latch` records coordination
decisions and claims.
