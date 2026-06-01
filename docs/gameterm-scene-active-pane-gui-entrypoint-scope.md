# GameTerm Scene Active Pane GUI Entrypoint Scope

This document scopes the deferred GUI action for opening Scene Mode from the
active GameTerm pane without requiring a shell helper command.

It builds on
[GameTerm Scene Active Pane Workflow Scope](gameterm-scene-active-pane-workflow-scope.md),
which defines the current shell-based preview and install workflow.

## Goal

Add a native GameTerm action that lets users generate and open a Scene Mode
workspace from the active pane.

Candidate action name:

```text
ShowGameTermSceneForActivePane
```

Final naming should follow existing action naming conventions before
implementation.

## Product End State

This lane is complete when:

1. A user can bind a key to generate a Scene workspace from the active pane.
2. The generated scene uses the same metadata contract as
   `ci/gameterm-scene-mux-context.sh`.
3. The action validates the generated scene before showing it.
4. The action does not silently overwrite `default.json`.
5. The action has a clear choice between transient preview and explicit install.
6. Errors are shown in Scene Mode or the GUI status path, not only in logs.
7. Deterministic tests cover the action request boundary without requiring a
   live GUI.
8. Live smoke captures the GUI action path.

## Non-Goals

- No background watcher.
- No automatic command execution.
- No terminal scrollback parsing.
- No replacement of the shell helper.
- No silent install of generated scenes.

## Open Design Decision

The main product decision is whether the GUI action should:

- open a transient generated scene without touching config
- install the generated scene as `default.json`
- offer both through separate actions

Recommended first pass: transient preview. Keep install explicit through the
documented shell workflow until there is UI affordance for overwrite consent.

## Implementation Lanes

1. Scope action naming and transient/install behavior.
2. Add action request type and keybinding documentation.
3. Generate active-pane scene from in-process `TermWindow`/mux metadata.
4. Validate generated scene before opening.
5. Add visible error reporting for missing/invalid pane context.
6. Add focused tests and live smoke.

## Acceptance Checklist

- GUI action uses active pane context without shelling out when possible.
- Generated scene matches the existing workspace discovery schema.
- Existing default scene config is not overwritten by default.
- Missing context is recoverable and visible.
- Live smoke proves the action path.
