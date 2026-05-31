# GameTerm Scene Mode Product Smoke Checklist

Use this checklist for the first-pass product smoke after runtime, overlay,
patching, input, rendering, or persistence changes. Record the result in
`docs/gameterm-scene-smoke-report.md` when a live pass is run.

## Setup

```sh
cargo check -p gameterm-gui
ci/gameterm-scene-verify.sh --all
ci/gameterm-scene-author.sh install-fixture vertical-slice --force
```

Launch GameTerm and open Scene Mode with the configured keybinding:

```lua
{ key = 'g', mods = 'CTRL|SHIFT', action = gameterm.action.ShowGameTermScene },
```

## App Launch And Close

- Scene Mode opens from the app without extra environment variables.
- The bundled scene opens if `~/.config/gameterm/scenes/default.json` is absent.
- The configured scene opens when `default.json` exists.
- `Esc` or `q` closes the overlay without closing the terminal pane.
- Reopening Scene Mode after close shows a valid scene and accepts input.

## Input And State

- Arrow or tab navigation changes the selected entity/choice.
- Enter activates the selected choice.
- Guarded unavailable choices stay visible and report the blocked guard.
- Layer input maps and mode input maps both handle their configured keys.
- Tile Debugger toggles and shows active mode, layers, variables, RPG state,
  selected entity metadata, process/agent state, patch source, and history.

## Persistence

- Export story state from the GUI path and confirm the status names the path.
- Mutate scene state with a deterministic choice.
- Import story state and confirm variables, RPG state, layers, selected entity,
  dialogue, and transition history restore as expected.
- Close and reopen Scene Mode, then import the same story state again.
- Failed import reports an error and does not corrupt the active runtime state.

## Patch And Recovery

- Submit a local patch through the Scene Mode inbox while Scene Mode is open.
- Submit a mux patch to the active overlay.
- Submit a mux patch to an explicit pane target.
- Missing active overlay returns a transport error.
- Missing target pane returns a transport error.
- Invalid patch reports the source and keeps the previous scene state.

## Agent And Workspace Slice

- Install or launch the `workspace-agent` fixture.
- Confirm workspace, project, task, agent, process, and file entities are
  visible and selectable.
- Run the deterministic choices through planning, running, blocked, and review
  states.
- Confirm blocked state exposes a visible blocker and recoverable next action.
- Confirm review-ready state enables the fixture open-file action and explicit
  verification command.
- Run `ci/gameterm-scene-smoke.sh --describe-scenario workspace-agent` and
  confirm the expected final status is documented.
- When live smoke is available, run
  `ci/gameterm-scene-smoke.sh --launch --scenario workspace-agent`.

## Workspace Discovery

- Run `ci/gameterm-scene-workspace.sh inspect --cwd .` and confirm cwd, root,
  git status, language, file count, and verification argv are reported.
- Generate a scene with
  `ci/gameterm-scene-workspace.sh discover --cwd . --scene-output /tmp/gameterm-workspace.json`.
- Validate the generated scene with
  `ci/gameterm-scene-author.sh validate /tmp/gameterm-workspace.json`.
- Confirm the generated scene includes workspace, project, task, process, and
  file entities.
- Confirm generated command choices are explicit and are not run during
  discovery.
- Run `ci/gameterm-scene-smoke.sh --describe-scenario workspace-discovery`.
- When live smoke is available, run
  `ci/gameterm-scene-smoke.sh --launch --scenario workspace-discovery`.

## Asset And Scene Failure Recovery

- Invalid scene JSON on launch shows an error frame instead of crashing.
- Invalid scene JSON on reload keeps the previous valid scene visible.
- Unknown or missing sprite id renders a placeholder and reports the issue.
- Missing sprite manifest falls back to bundled sprites/placeholders.
- Missing `OpenFile` target reports status without closing Scene Mode.
- Invalid `RunCommand` cwd/target is reported by doctor before live smoke.

## Cleanup

```sh
ci/gameterm-scene-author.sh install-fixture basic --force
ci/gameterm-scene-doctor.sh --strict
```
