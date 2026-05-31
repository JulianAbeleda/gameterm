# GameTerm Scene Mode Pane And Process Discovery Scope

This document scopes Product Layer 1 from the broader Scene Mode product pass:
Pane And Process Discovery.

## Goal

Scene Mode should connect generated workspace state to the active GameTerm
session.

Workspace Discovery already knows the repository, important files, and default
verification path. Pane/process discovery adds the terminal context around that
workspace:

- active pane id
- mux window id
- active pane cwd
- foreground process name
- foreground process executable path
- pane progress state

This layer does not parse arbitrary terminal output and does not run commands.
It records explicit pane/process metadata when that metadata is supplied by a
trusted caller or future mux integration path.

## Product End State

The first complete layer is done when:

1. Workspace Discovery works with no pane/process metadata.
2. Workspace Discovery can accept explicit pane/process metadata.
3. A pane cwd can become the discovery cwd when `--cwd` is not supplied.
4. Generated scenes expose pane context as variables and metadata.
5. Generated scenes include a pane entity and an active process entity.
6. Generated patches expose pane context as variables and entity metadata.
7. Generated patches update typed process state when a foreground process is
   known.
8. Deterministic verification covers missing and supplied metadata.
9. Live mux auto-discovery remains deferred until the API path is stable.

## Available Context

GameTerm already exposes pane/process concepts in the runtime and user-facing
Lua APIs:

- pane id
- mux window id
- pane current working directory
- foreground process name
- foreground process info
- pane progress

The stable first-pass helper path is explicit metadata input through
`ci/gameterm-scene-workspace.sh`. Direct live mux discovery is a later
integration step because it needs a stable caller context and should not depend
on scraping terminal text.

## Helper Contract

Workspace Discovery accepts these optional inputs:

```sh
ci/gameterm-scene-workspace.sh discover \
  --pane-id 231 \
  --mux-window-id 7 \
  --pane-cwd /path/to/workspace \
  --foreground-process-name zsh \
  --foreground-process-path /bin/zsh \
  --pane-progress None
```

The same inputs are accepted by `inspect`, `discover`, and `patch`.

Rules:

- `--pane-id` and `--mux-window-id` must be non-negative integers.
- `--pane-cwd` must be a local directory when it is used as the workspace cwd.
- If `--cwd` is absent and `--pane-cwd` is present, discovery uses
  `--pane-cwd`.
- If `--cwd` is present, `--pane-cwd` remains metadata only.
- Missing pane/process metadata is represented as absent optional fields or
  `pane_context=absent`.
- Supplied pane/process metadata is represented as `pane_context=provided`.

## Scene Mapping

Generated scenes use variables for small guard-driving values:

- `discovery_source`
- `pane_context`
- `active_pane_id`
- `active_mux_window_id`
- `process_phase`

Generated scenes use metadata for inspectable context:

- workspace metadata: active pane id, mux window id, pane cwd
- pane entity metadata: pane id, mux window id, cwd, progress
- process entity metadata: foreground process name/path, progress, phase,
  message

Generated scenes include:

- `discovered-pane`: a `Task` entity representing the active pane context
- `discovered-process`: a `Task` entity representing the known foreground
  process context

The process entity is marked `running` only when foreground process or progress
metadata is supplied. Otherwise it remains explicit as `none`.

## Patch Mapping

Generated patches update the existing Agent/Workspace fixture entities:

- `workspace-gameterm` receives workspace and pane metadata.
- `scene-verify-process` receives process metadata and flags.

When foreground process or progress metadata is present, the patch also writes
typed `process_state`:

```json
{
  "entity_id": "scene-verify-process",
  "phase": "running",
  "command": "zsh",
  "message": "Foreground process detected"
}
```

When process metadata is absent, `process_state` remains `null`.

## Deferred Work

Deferred items:

- live pane metadata discovery from the active mux session
- automatic foreground process lookup from a pane id
- process tree rendering
- multiple pane discovery
- live progress polling
- policy around agent-proposed pane/process actions

These are intentionally out of the first pass because the current product need
is to represent pane/process context safely when it is available.

## Verification

Deterministic verification covers:

- no metadata path
- explicit pane/process metadata path
- generated scene validation
- generated patch validation
- `pane_context`, active pane id, and process phase variables
- pane/process metadata on generated entities
- typed patch `process_state` when process metadata is available

Live smoke should be added when a stable caller can pass real active pane
metadata from the GUI or mux CLI path into Workspace Discovery.

## Done Status

Implemented:

- explicit pane/process metadata options in `ci/gameterm-scene-workspace.sh`
- pane cwd as default discovery cwd when no `--cwd` is supplied
- generated pane/process variables, entities, metadata, and patch state
- deterministic verifier coverage

Deferred:

- live mux auto-discovery
- multi-pane process map
- progress polling
