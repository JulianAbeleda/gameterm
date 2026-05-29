# GameTerm Scene Runtime Roadmap

This note tracks the scene runtime action and reload design. It is scoped to
runtime behavior for JSON scene files, Tile Debugger visibility, and future
choice actions.

## Goals

- Make `~/.config/gameterm/scenes/default.json` iteration fast enough for
  scene authoring.
- Keep the currently loaded scene path and load status visible in the Tile
  Debugger.
- Define action behavior before wiring `OpenFile` and `RunCommand` into the
  terminal runtime.
- Preserve the bundled default scene as the fallback when no default scene file
  exists.

## Default Scene Reload

Scene Mode currently loads the optional default scene from:

```text
~/.config/gameterm/scenes/default.json
```

When `XDG_CONFIG_HOME` is set, the path is:

```text
$XDG_CONFIG_HOME/gameterm/scenes/default.json
```

Implemented reload behavior:

1. Load the default scene when Scene Mode opens.
2. Use the bundled default scene when no default scene file exists.
3. Add a manual reload action from inside Scene Mode so authors can update
   `default.json`, return to GameTerm, and refresh without closing the window.
4. Keep the previous valid scene visible if a reload fails. The error should be
   surfaced in Scene Mode and the Tile Debugger status instead of replacing the
   scene with a blank or partial state.
5. Use the bundled default scene only when no default scene file exists at load
   time. If a previously valid default scene later fails to parse, keep the
   previous scene and report the failed reload.
6. Reset selection only when the reloaded scene no longer contains the selected
   entity id. If the id still exists, preserve selection and update the
   inspection panel from the new entity data.

Reload status should distinguish these cases:

- `bundled`: no default scene file exists, so the bundled default scene is
  active.
- `loaded`: the default scene file loaded successfully.
- `reload failed`: the default scene file exists but the latest reload failed;
  the previous valid scene remains active.
- `invalid`: the scene file failed during initial load and there is no previous
  valid scene to keep.

Automatic file watching can come later. The first implementation should favor a
predictable manual reload because it avoids platform watcher differences and
keeps parse errors tied to an explicit user action.

## Tile Debugger Path And Status

The Tile Debugger should show scene runtime source details alongside layer,
entity, sprite, position, flag, and metadata inspection.

Minimum fields:

- `Scene path`: absolute resolved path for `default.json`, or `bundled default`
  when the fallback scene is active.
- `Load status`: one of the statuses above.
- `Reload counter`: monotonic count of reload attempts, including the initial
  load.
- `Error`: concise parse, schema, or I/O failure text when the current status is
  `reload failed` or `invalid`.

The path/status line should be visible without selecting an entity. Entity
selection should continue to drive entity-specific debugger rows, but source
status belongs to the scene as a whole.

## Choice Actions

Scene choices already model placeholder actions such as `Inspect` and
`OpenFile`. Runtime action execution should stay narrow at first and expand
behind explicit action variants.

### Inspect

`Inspect` remains local to Scene Mode. It should focus or refresh the selected
entity details without leaving the scene.

### OpenFile

Implemented `OpenFile` behavior:

1. Resolve relative paths against the current workspace or process working
   directory used by GameTerm, not against the JSON file path unless a separate
   `base` field is introduced.
2. Normalize and display the resolved path in the Tile Debugger action status.
3. Report missing files as an action error in Scene Mode without closing the
   scene.

Remaining `OpenFile` behavior:

1. Open the file through the existing terminal/application file-opening path
   once that runtime hook exists.

`OpenFile` should not execute shell commands. It is a document/navigation
action, even when the target file extension is executable.

### RunCommand

Future `RunCommand` behavior:

1. Require an explicit command payload in the scene JSON. Do not infer commands
   from labels, metadata, or file paths.
2. Route execution through the same trusted command pathway used elsewhere in
   GameTerm so panes, working directory, environment, and auditability remain
   consistent.
3. Prefer spawning in an existing or new pane over blocking the scene runtime.
4. Show command start, exit, and failure state in the Tile Debugger action
   status.
5. Treat command output as terminal output, not as scene JSON mutation, until a
   separate structured-state update channel is designed.

`RunCommand` should be considered opt-in and visibly represented in the UI
before execution. The JSON scene file is local configuration, but command
execution still needs a clear user action boundary.

## Open Questions

- Should future scene files be able to specify a base directory for relative
  `OpenFile` paths?
- Should `RunCommand` support a dry-run preview state before the first
  execution implementation?
