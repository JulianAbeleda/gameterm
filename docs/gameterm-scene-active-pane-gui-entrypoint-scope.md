# GameTerm Scene Active Pane GUI Entrypoint Scope

This document fully scopes the deferred GUI action for opening Scene Mode from
the active GameTerm pane without requiring the user to run a shell helper.

It builds on
[GameTerm Scene Active Pane Workflow Scope](gameterm-scene-active-pane-workflow-scope.md),
which defines the current shell-based preview and install workflow.

## Goal

Add a native GameTerm action that generates a Scene Mode workspace from the
active pane and opens it as a transient Scene Mode overlay.

Recommended first-pass action:

```text
ShowGameTermSceneForActivePane
```

The final action name should be checked against existing action naming in
`gameterm-gui/src/commands.rs` before implementation. If the existing action
system prefers shorter names, `ShowGameTermSceneFromActivePane` is an acceptable
alternative, but only one name should ship.

## Product End State

This lane is complete when:

1. A user can bind a key to open Scene Mode generated from the active pane.
2. The action opens a transient generated scene by default and does not modify
   `~/.config/gameterm/scenes/default.json`.
3. The generated scene uses the same pane/workspace metadata contract as
   `ci/gameterm-scene-mux-context.sh discover`.
4. The generated scene validates before it is shown.
5. Missing or invalid active-pane context produces a visible recoverable error.
6. The existing `ShowGameTermScene` action continues to load the normal default
   configured scene unchanged.
7. Deterministic tests cover the collection/generation/opening boundary without
   requiring a live GUI.
8. Live smoke proves the real keybinding/action path.

## User Experience

The user binds a key, for example:

```lua
{ key = 'g', mods = 'CTRL|SHIFT', action = gameterm.action.ShowGameTermScene },
{ key = 'g', mods = 'CTRL|ALT|SHIFT', action = gameterm.action.ShowGameTermSceneForActivePane },
```

The normal Scene key continues to open the configured default scene. The new
active-pane key generates a fresh transient scene from the current active pane
and opens that generated scene immediately.

Expected visible result:

- selected workspace/project/process/pane entities render in Scene Mode
- `pane_context=provided` when active pane metadata is available
- `discovery_source=pane_cwd` when active pane cwd is used
- active pane/window ids are visible in debug state
- no default scene file is overwritten

## First-Pass Product Decision

Use **transient preview**, not install.

Rationale:

- install has overwrite risk and already exists as an explicit shell workflow
- transient preview matches user intent for a keybinding: "show me this pane"
- preview can fail visibly without modifying user config
- install can be added later behind explicit UI confirmation

The shell workflow remains the install path:

```sh
ci/gameterm-scene-mux-context.sh discover --install --force
```

## Non-Goals

- No silent install of generated scenes.
- No overwrite of `default.json`.
- No background watcher.
- No command execution from terminal content.
- No terminal scrollback parsing.
- No process polling loop.
- No multi-pane dashboard.
- No replacement of `ci/gameterm-scene-mux-context.sh`.
- No generated-scene persistence unless the user explicitly installs later.

## Existing Code Boundaries

Relevant current surfaces:

- `gameterm-gui/src/commands.rs`
  - registers `ShowGameTermScene`
  - describes user-bindable commands
- `gameterm-gui/src/overlay/mod.rs`
  - starts overlay panes through `start_overlay` and `start_overlay_pane`
  - already passes mux window identity into visual overlays
- `gameterm-gui/src/overlay/visual.rs`
  - loads and renders Scene Mode runtime
  - receives mux patch notifications
  - already has scene source/status/debug visibility
- `gameterm-gui/src/termwindow/mod.rs`
  - exposes `PaneInformation`
  - can resolve active pane metadata
- `mux/src/pane.rs`
  - exposes pane cwd, foreground process name/info, and progress
- `ci/gameterm-scene-mux-context.sh`
  - defines the shell metadata contract and install workflow
- `ci/gameterm-scene-workspace.sh`
  - currently owns generated workspace scene construction

The first GUI implementation should not fork scene schema logic blindly. It
should either reuse a Rust generation module or create one before wiring the
action.

## Architecture Recommendation

### 1. Extract Scene Workspace Generation Into Rust

Current workspace scene generation is shell/JQ driven. A GUI action cannot rely
on shelling out as the primary path.

Add a Rust workspace scene generator in or near `gameterm-visual`:

```text
gameterm-visual/src/workspace_discovery.rs
```

Suggested public API:

```rust
pub struct SceneWorkspaceContext {
    pub cwd: PathBuf,
    pub title: Option<String>,
    pub task: Option<String>,
    pub verify_argv: Option<Vec<String>>,
    pub pane: Option<ScenePaneContext>,
    pub max_files: usize,
}

pub struct ScenePaneContext {
    pub pane_id: Option<u64>,
    pub mux_window_id: Option<u64>,
    pub pane_cwd: Option<PathBuf>,
    pub foreground_process_name: Option<String>,
    pub foreground_process_path: Option<PathBuf>,
    pub pane_progress: Option<String>,
}

pub fn generate_workspace_scene(context: SceneWorkspaceContext) -> anyhow::Result<VisualScene>;
```

The initial Rust generator should preserve the shell helper's observable
contract for active-pane scenes:

- variables
- entity ids
- relationship ids
- process state conventions
- choice policy metadata
- layout positions

The shell helper can remain as-is for this lane, but long-term consolidation
should be a follow-up: use the Rust generator from helper examples/tests or
replace duplicated JQ generation once the Rust path is proven.

### 2. Collect Active Pane Context In-Process

The GUI action should collect:

- active pane id
- mux window id
- pane cwd
- foreground process name
- foreground process executable path when available
- pane progress

Preferred source:

- active `TermWindow`
- `get_active_pane_or_overlay()`
- mux pane methods with stale-cache policy where appropriate

Fallback:

- if pane cwd is missing, use current process cwd or active workspace cwd only
  if product behavior explicitly chooses fallback
- otherwise show a visible error and keep terminal alive

Do not parse terminal text.

### 3. Open Transient Scene

The overlay layer needs a way to open Scene Mode from an in-memory scene or from
a generated temporary scene file.

Preferred first pass:

- generate `VisualScene` in memory
- open overlay with a `SceneSource::GeneratedActivePane`-style source label
- show source/debug status as generated, not config-backed

Acceptable fallback if in-memory source is too invasive:

- write generated JSON to a temp path owned by the process
- open overlay from that temp path
- mark source as generated active-pane temp scene
- clean up temp path when safe

Do not write `default.json`.

## Proposed Runtime Model

Add an internal source variant conceptually equivalent to:

```text
SceneSource::GeneratedActivePane {
  pane_id,
  mux_window_id,
  cwd,
}
```

If the current source model is string/status based rather than enum-based,
preserve that style but ensure Tile Debugger/debug report can distinguish:

- bundled default scene
- config default scene
- file-loaded scene
- generated active-pane scene

## Action Semantics

Action: `ShowGameTermSceneForActivePane`

Behavior:

1. Resolve active terminal pane.
2. Collect pane/process metadata.
3. Resolve discovery cwd.
4. Generate workspace scene.
5. Validate scene with the same Rust validation path used for loaded scenes.
6. Start Scene Mode overlay with generated scene.
7. Show success/source status in Tile Debugger.

Failure behavior:

- no active pane: show error, do not open overlay
- cwd missing/invalid: show error, do not open overlay unless fallback is
  explicitly implemented
- generation failure: show error, do not open overlay
- validation failure: show error, do not open overlay

The action should never run project commands.

## Config And Keybinding Documentation

Document a sample binding after implementation:

```lua
{
  key = 'g',
  mods = 'CTRL|ALT|SHIFT',
  action = gameterm.action.ShowGameTermSceneForActivePane,
}
```

Documentation must clearly distinguish:

- `ShowGameTermScene`: opens configured/default Scene Mode
- `ShowGameTermSceneForActivePane`: generates a transient scene from the active
  pane
- shell install workflow: writes `default.json`

## Data Contract

Generated active-pane scenes should match the existing active-pane workflow:

Variables:

- `workspace_mode`
- `workspace_root`
- `repo_branch`
- `repo_status`
- `active_task_id`
- `process_phase`
- `verification_status`
- `discovery_source`
- `pane_context`
- `discovered_file_count`
- `active_pane_id` when known
- `active_mux_window_id` when known

Entities:

- `discovered-workspace`
- `discovered-project`
- `discovered-pane`
- `discovered-task`
- `discovered-process`
- discovered file entities as available

Relationships:

- workspace contains project
- task targets project
- process verifies project
- pane observes process when pane context exists
- project includes files
- task references files

Patch behavior is not required for the initial GUI action.

## Error Reporting

Visible errors should include:

- short user-facing status
- debug detail in Tile Debugger/debug report
- no panic/crash

Examples:

```text
Scene active pane unavailable
Scene active pane cwd invalid: /missing/path
Scene active pane generation failed: <reason>
Scene active pane validation failed: <reason>
```

## Verification Plan

Deterministic Rust tests:

- workspace generator produces valid `VisualScene`
- generated scene includes expected variables/entities for pane metadata
- generated scene handles missing optional process metadata
- invalid cwd is rejected before generation/opening
- generated source/debug status identifies active-pane generation
- action dispatch requests generated scene overlay instead of config scene

Shell verifier:

- keep existing active-pane install workflow checks
- add no GUI dependency to deterministic CI

Focused GUI tests:

- command/action enum includes new action
- command docs/description present
- action path routes to generated scene launcher
- failure path returns visible status/error

Live smoke:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-smoke.sh --launch --scenario active-pane-gui-entrypoint
```

The smoke scenario should:

- launch GameTerm
- trigger the new keybinding/action
- capture generated active-pane scene
- record pane id, mux window id, cwd, and capture path

## Implementation Lanes

### Lane 1: Full Scope

Deliverables:

- this expanded scope document

Verification:

- `git diff --check`

Commit:

- `[docs] fully scope Scene active pane GUI entrypoint`

### Lane 2: Rust Workspace Generator

Deliverables:

- Rust workspace generation module
- active-pane metadata structs
- tests matching shell helper output contract for core variables/entities

Verification:

- `cargo test -p gameterm-visual workspace`
- `cargo test -p gameterm-visual scene_fixture`

Commit:

- `[visual] add Scene workspace generator`

### Lane 3: Generated Scene Overlay Source

Deliverables:

- overlay/runtime path for generated active-pane scenes
- generated source status/debug visibility
- no `default.json` writes

Verification:

- `cargo test -p gameterm-gui overlay::visual`
- focused generated-source tests

Commit:

- `[visual] open generated Scene overlays`

### Lane 4: GUI Action

Deliverables:

- action registration and docs in command list
- active pane context collection
- action dispatch to generated overlay
- visible error handling

Verification:

- `cargo test -p gameterm-gui active_pane`
- `cargo test -p gameterm-gui overlay::visual`

Commit:

- `[visual] add active pane Scene action`

### Lane 5: Documentation

Deliverables:

- keybinding docs
- distinction between default scene, transient active-pane scene, and shell
  install workflow
- recovery notes

Verification:

- `git diff --check`

Commit:

- `[docs] document active pane Scene action`

### Lane 6: Smoke Scenario

Deliverables:

- named smoke scenario for GUI action
- key-sequence or direct action trigger
- smoke report entry

Verification:

- `ci/gameterm-scene-smoke.sh --list-scenarios`
- live smoke capture
- `ci/gameterm-scene-verify.sh --all`

Commit:

- `[test] add active pane Scene action smoke`

## Acceptance Checklist

- New GUI action is bindable.
- Action opens a generated active-pane scene.
- Action does not overwrite user scene config.
- Generated scene validates before opening.
- Active pane/window/cwd metadata appears in scene state.
- Missing context produces visible recoverable errors.
- Normal `ShowGameTermScene` behavior is unchanged.
- Deterministic tests cover generator and action boundary.
- Live smoke proves the real GUI path.

## Risks

- Duplicating shell/JQ scene generation in Rust can drift over time.
- Writing temp files for generated scenes can leak stale files if cleanup is
  not handled.
- Opening an overlay from an overlay pane may collect overlay pane metadata
  instead of the underlying terminal pane if the action is triggered from the
  wrong context.
- Foreground process metadata may be unavailable or stale on some platforms.
- GUI action naming may require config/schema compatibility work.

## Recommendation

Implement this in two separable technical steps:

1. Extract/grow a Rust workspace scene generator and prove it against the
   existing shell contract.
2. Wire the GUI action to that generator as a transient overlay.

Do not start by shelling out from the GUI action. Shelling out can remain a
debug fallback, but the product path should be in-process and typed.
