# GameTerm Scene Mode Workspace Sessions Scope

This document scopes Product Layer 5 from the broader Scene Mode product pass:
Persisted Workspace Sessions.

## Goal

Scene Mode should preserve useful workspace state across launches without
turning generated scene JSON into the only durable storage.

Story-state import/export already proves that runtime state can be persisted.
Workspace sessions need a product-level wrapper around daily work state:
workspace identity, selected entity, recent process/agent state, generated
context, and recovery status.

## End Goal

A user can:

1. Generate or open a workspace scene.
2. Save the current workspace session to an inspectable file.
3. Restore that session later.
4. See save/load status in normal view and the Tile Debugger.
5. Recover cleanly from invalid or incompatible session files.

Saving a session must not silently mutate the source scene file.

## First-Pass Product Contract

The first pass should reuse the existing story-state machinery where possible,
but it should present a workspace-session workflow rather than asking the user
to think in visual-novel terms.

Session data should include:

- source scene identity
- workspace root
- selected entity id
- selected choice index when useful
- variables
- RPG/story records when present
- active layer states
- dialogue index/history when present
- last known process state
- last known patch source metadata when available

Session data should not include:

- terminal scrollback
- command stdout/stderr
- secrets from process environments
- arbitrary background memory indexes
- generated scene source as an opaque blob

## Storage Contract

Default storage should be explicit and local:

```text
~/.local/state/gameterm/scene-workspaces/<workspace-id>.json
```

The first pass may use helper-provided paths before standardizing the exact
default. Every saved file must be JSON and inspectable with `jq`.

Workspace id should be deterministic from:

- workspace root when available
- scene title/path as fallback
- sanitized file-system-safe encoding

## Helper Contract

Add a helper path only if runtime dispatch does not cover the full first pass.
Candidate commands:

```sh
ci/gameterm-scene-session.sh save --scene <scene> --output <session>
ci/gameterm-scene-session.sh restore --scene <scene> --session <session> --output <scene-or-patch>
ci/gameterm-scene-session.sh inspect --session <session>
```

Rules:

- validate before writing
- write atomically through a temp file and rename
- never overwrite without `--force`
- failed restore must leave source scene unchanged
- restore should report a status or patch rather than silently changing state

## Runtime Impact

Likely runtime work:

- expose workspace-session action names or reuse existing export/import
- add normal-view status for save/load result
- ensure debug report shows last session path/action
- preserve existing story-state behavior

Avoid adding a broad persistence framework. Keep the first pass focused on one
workspace-session file shape.

## Verification

Deterministic verification should cover:

- save writes valid JSON
- restore applies expected selected entity and variables
- restore rejects invalid JSON
- restore rejects incompatible scene/session identity when applicable
- failed restore leaves original scene unchanged
- helper overwrite protection
- `ci/gameterm-scene-verify.sh --all`

Live smoke is useful only after GUI save/load dispatch is wired.

## Commit Lanes

1. `[docs] scope Scene workspace sessions layer`
2. `[visual] add Scene workspace session runtime support` if runtime changes are needed
3. `[visual] add Scene workspace session helper path` if helper belongs to Scene Mode authoring
4. `[test] verify Scene workspace sessions`
5. `[docs] document Scene workspace session workflow`
6. `[tools] record Scene workspace session smoke` when live smoke is run

## Deferred Work

- multiple named sessions per workspace
- session browser UI
- auto-save
- encrypted or private session storage
- remote workspace session sync
- cross-device restore
- background process replay

## Done Definition

The layer is first-pass complete when a user can save and restore workspace
state through documented commands, bad session files fail visibly, and source
scene JSON is not mutated by failed persistence operations.
