# GameTerm Scene Mode Agent Task Bootstrap Scope

This document scopes Product Layer 8 from the broader Scene Mode product pass:
Agent Task Bootstrap.

## Goal

Scene Mode should turn discovered workspace context into an inspectable task
brief that a user can explicitly hand off.

Workspace Discovery answers "what workspace is this?" Task bootstrap answers
"what work could be started from this context?" without starting agents
automatically.

## End Goal

A user can:

1. Generate workspace context.
2. Inspect a task brief generated from that context.
3. Open/export/copy the task brief.
4. Explicitly start or hand off work through a visible action later.

No hidden network call, agent process, or command execution is allowed in the
first pass.

## First-Pass Product Contract

Task brief data should include:

- workspace root
- project label
- repo branch/status
- important files
- active pane/process context when available
- suggested objective
- verification command
- known blockers or missing context
- generated-at metadata

The brief should be a file or entity metadata record that can be inspected
before any action is taken.

## Data Contract

First-pass brief shape:

```json
{
  "brief_version": 1,
  "workspace_root": "/path/to/repo",
  "objective": "Review workspace state",
  "context_files": ["README.md"],
  "verification": ["ci/gameterm-scene-verify.sh", "--all"],
  "constraints": ["do not run commands automatically"]
}
```

Rules:

- argv values remain arrays
- file paths are local and relative to workspace root when possible
- generated brief includes no secrets
- generated brief is not submitted anywhere automatically

## Helper Contract

Workspace Discovery can add:

- a task entity with brief metadata
- an `OpenFile` choice for the exported brief
- optionally a `RunCommand` choice only for explicit verification, not agent
  start

Candidate helper command:

```sh
ci/gameterm-scene-workspace.sh brief --cwd . --output /tmp/gameterm-task-brief.json
```

This can be added only after the brief shape is stable enough to verify.

## Runtime Impact

No new runtime behavior is required for the first pass if the brief is exposed
as:

- file entity metadata
- `OpenFile` choice
- generated scene dialogue/status

Runtime work is deferred until explicit agent handoff has a stable product
contract.

## Verification

Deterministic verification should cover:

- brief JSON shape
- generated brief contains workspace root and important files
- generated scene links to the brief
- generated brief does not start commands
- invalid output/overwrite protection
- `ci/gameterm-scene-verify.sh --all`

## Commit Lanes

1. `[docs] scope Scene agent task bootstrap layer`
2. `[visual] add Scene task brief generation`
3. `[test] verify Scene task bootstrap`
4. `[docs] document Scene task bootstrap workflow`
5. `[tools] record Scene task bootstrap smoke` when live smoke is run

## Deferred Work

- launching agents
- network model calls
- prompt templating beyond local JSON/Markdown
- task approval UI
- agent identity negotiation
- background task monitors

## Done Definition

The layer is first-pass complete when workspace discovery can produce a local,
inspectable task brief and Scene Mode can show/open it without starting work
automatically.
