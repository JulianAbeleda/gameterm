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
- Keep a fixture-backed verification harness for every implemented Scene Mode
  behavior before adding higher-risk action execution.

## Scope Checklist

Current status:

- [x] Bundled default scene loads when user config is missing.
- [x] Editable `default.json` config path is documented and initialized by
  `ci/gameterm-scene-init.sh`.
- [x] Manual reload preserves the previous valid scene on reload failure.
- [x] Optional auto-reload supports authoring sessions.
- [x] Scene source status is visible in Scene Mode and the Tile Debugger.
- [x] Tile Debugger exposes action, selected entity, sprite, flag, metadata,
  pending action, and patch source state.
- [x] `Inspect`, `OpenFile`, `Navigate`, and `RunCommand` choices have explicit
  runtime behavior.
- [x] `RunCommand` supports `tab`, `split_right`, and `split_down` targets.
- [x] Sprite manifests support user-provided sprite ids and bundled fallback
  sprites.
- [x] Sprite image loading is cached and pruned on pane removal.
- [x] Scene authoring helpers cover fixture install, validation, edit
  operations, formatting, and doctor diagnostics.
- [x] Noninteractive verification covers fixtures, authoring helpers, doctor,
  state patches, and focused Rust tests.
- [x] Runtime patch schema updates entity flags, metadata, and status without
  rewriting scene JSON.
- [x] Patch inbox transport supports local file-based updates.
- [x] Mux/CLI patch transport supports active-overlay and explicit-pane
  targeting.
- [x] Missing active overlays and missing target panes fail as transport
  errors.
- [x] Patch source transport and source pane are visible in debug state.
- [x] Multiple simultaneous Scene Mode overlays are allowed; newest overlay is
  the default active target.
- [x] macOS smoke helper can launch fixtures and submit a mux patch before
  capture.
- [x] Run and archive a real live mux-submit smoke capture.
- [x] Add renderer helper tests for cache invalidation and image-disabled
  behavior.
- [x] Expand patchable runtime state beyond entity flags/metadata/status.
- [x] Make agent/process integrations emit Scene Mode patches directly.
- [x] Add higher-level authoring UX for templates and guided scene creation.
- [x] Define the Scene Mode narrative/RPG layer.
- [x] Add dialogue and branching-choice runtime state.
- [x] Add explicit story-state save/load or export/import.
- [x] Prototype lightweight RPG state such as inventory, stats, quests, and
  relationships.
- [x] Define Scene Mode computational modes.
- [x] Add mode descriptors for conversation, memory, agent, and workspace
  contexts.
- [x] Add typed runtime variables for mode/story/RPG state.
- [x] Add guarded choice conditions backed by typed runtime variables.
- [x] Add mode lifecycle hooks for enter, update/poll, and exit behavior.
- [x] Add per-mode input/action maps and guarded transitions.
- [x] Scope the next Scene Mode product-loop roadmap across layered modes,
  deterministic actions, authoring, persistence, agent/process integration, and
  live smoke discipline.
- [x] Add layered state machine model, guarded layer transitions, layer input
  ownership, debug visibility, and a layered fixture.
- [x] Add deterministic state action resolution for variables, inventory,
  stats, quests, and relationships.
- [x] Add visual novel, layered mode, and RPG quest authoring templates with
  verification coverage.
- [x] Add explicit story-state export/import/validate/inspect helper coverage.
- [x] Add typed process-state patches and debug visibility for agent/process
  helpers.
- [x] Add named smoke scenario registry, documentation, and process-state smoke
  wiring.
- [x] Add next-layer smoke report and artifact convention; first live attempt
  captured the workspace instead of Scene Mode, so foreground/open-scene
  automation remains a follow-up.
- [x] Add macOS smoke foreground/open-scene automation and frontmost-process
  diagnostics before capture.
- [x] Add macOS smoke key-sequence automation for guarded input, RunCommand
  targets, and overlay cleanup.
- [x] Add a vertical-slice template and fixture that combines dialogue,
  guarded choices, deterministic RPG/story state, layers, process state, and
  story export coverage.
- [x] Add a live macOS smoke scenario for the playable vertical slice core
  keyboard loop.
- [x] Add GUI-dispatched story-state export/import actions, input-map
  shortcuts, visible status/debug reporting, and overlay dispatch tests.
- [x] Add an authoring-loop fixture and smoke scenario for in-app story-state
  save, mutate, and reload workflow.
- [x] Add deterministic `SetLayerState` resolve operations with validation and
  rollback coverage for variables, RPG state, and layers.
- [x] Add an agent lifecycle patch helper for idle, planning, running, waiting,
  blocked, completed, failed, and cancelled phases with verifier coverage and
  patch variables for scene guards.
- [x] Add a live macOS smoke scenario for agent lifecycle patches through the
  Scene Mode inbox transport.

## Scene Mode Product Loop Roadmap

The foundation layer proves that Scene Mode can represent terminal-native game
state: scenes, choices, dialogue, variables, lightweight RPG records, patches,
mux transport, mode descriptors, lifecycle hooks, and guarded input maps. The
next roadmap should make that state useful as a repeatable product loop.

The remaining first-pass completion scope is tracked in
[GameTerm Scene Mode First-Pass Scope](gameterm-scene-first-pass-scope.md).
That document defines the 100% first-pass target by runtime, authoring,
state-model, persistence, integration, presentation, observability, and smoke
layers.

The next product layer is scoped in
[GameTerm Scene Mode Agent And Workspace Scope](gameterm-scene-agent-workspace-scope.md).
That document defines the purpose, end goal, state contracts, fixture plan,
workflow targets, acceptance criteria, and commit lanes for making Scene Mode
represent active GameTerm workspace and agent state.

The follow-up discovery layer is scoped in
[GameTerm Scene Mode Workspace Discovery Scope](gameterm-scene-workspace-discovery-scope.md).
That document defines how authored Agent/Workspace state becomes a generated
view of cwd, git, project files, explicit commands, and future pane/process
metadata.

Priority order:

1. Layered state machines.
2. Deterministic action resolution.
3. Authoring UX.
4. Persistence UX.
5. Agent/process integration.
6. Live smoke discipline.

The priority order is architectural, not strictly serial. Documentation,
fixtures, and focused tests can move in parallel, but runtime behavior should
not depend on later lanes before earlier state contracts exist.

### Lane 1: Layered State Machines

Purpose: allow Scene Mode to run multiple active behavioral contexts at once
instead of forcing every behavior through one global mode.

Initial layers:

1. Global mode: startup, workspace, paused, error.
2. UI mode: scene, dialogue, tile debugger, command palette.
3. Agent/process mode: idle, planning, running, blocked, complete.
4. Selected entity mode: idle, focused, actionable, hidden.
5. Story mode: dialogue, choice, transition, save/load.

Deliverables:

1. Add a structured layered-state model to `gameterm-visual`.
2. Keep the existing single `VisualModeDescriptor` as the active top-level mode
   until layered behavior is proven.
3. Expose every active layer in `VisualSceneDebugReport`.
4. Render active layers in the Tile Debugger.
5. Define deterministic input ownership when multiple layers bind the same
   input.
6. Add transition status fields: last layer, attempted transition, guard result,
   and target.

Acceptance criteria:

1. Existing scenes continue to load without layered-state JSON.
2. A fixture can activate at least two layers at once.
3. The Tile Debugger shows all active layers and the layer that handled the last
   input.
4. Guarded layer transitions can fail visibly without mutating state.
5. Focused `gameterm-visual` tests cover load defaults, transition success,
   transition failure, input ownership, and debug output.

Commit-sized tasks:

1. Add default layered-state data structures and validation.
2. Add debug report and Tile Debugger visibility.
3. Add transition attempt/result tracking.
4. Add input ownership rules.
5. Add fixtures and `ci/gameterm-scene-verify.sh --all` coverage.

### Lane 2: Deterministic Action Resolution

Purpose: let choices and input bindings update story/RPG state directly through
auditable, typed operations instead of requiring ad hoc external patches for
every state change.

Initial action operations:

1. Set, increment, decrement, and clear typed variables.
2. Add, remove, and count inventory items.
3. Set stat values or adjust numeric stats.
4. Advance quest stage, complete quest, and append quest journal entries.
5. Adjust relationship values and metadata.
6. Advance dialogue and navigate scene, reusing existing action behavior.

Deliverables:

1. Add a deterministic `SceneActionKind` variant for state operations or a
   separate action-resolution object referenced by choices/input maps.
2. Apply operations in memory only; source scene files remain immutable.
3. Reuse the same validation rules as story/RPG state import.
4. Report action results in Scene Mode status and the Tile Debugger.
5. Keep shell execution explicit through existing `RunCommand`; deterministic
   actions must not spawn processes.

Acceptance criteria:

1. A choice can grant an item, advance a quest, and unlock a guarded branch in
   one deterministic action path.
2. Invalid operations fail without partial mutation.
3. Action resolution bumps visual generation exactly when state changes.
4. Story export includes the resolved state.
5. Focused tests cover success, failure, rollback/no-mutation, and debug output.

Commit-sized tasks:

1. Define the action-operation schema and validation.
2. Implement variable operations.
3. Implement inventory/stat operations.
4. Implement quest/relationship operations.
5. Add debug reporting and fixtures.

### Lane 3: Authoring UX

Purpose: make scenes practical to create without hand-writing large JSON files.

Initial templates:

1. `visual-novel`: dialogue lines, branching choices, portraits/sprites, and
   guarded branches.
2. `agent-workflow`: task/project entities, process state, command choices, and
   patch targets.
3. `rpg-quest`: inventory, stats, quests, relationships, and deterministic
   state actions.
4. `layered-mode`: sample scene with global, UI, story, and selected-entity
   layers.

Deliverables:

1. Extend `ci/gameterm-scene-author.sh new-template` with the templates above.
2. Add a doctor mode that explains missing sprite ids, invalid guards, invalid
   action operations, and unreachable transition targets.
3. Add formatting support that preserves stable key ordering for new schema
   sections.
4. Add small fixture scenes for each template.
5. Document authoring commands in this roadmap and in the helper output.

Acceptance criteria:

1. Each template validates immediately after generation.
2. Each template is covered by `ci/gameterm-scene-verify.sh --all`.
3. Doctor output identifies at least one intentional broken fixture per major
   schema family.
4. Generated templates avoid shell execution by default.
5. Templates demonstrate the product loop: state, action, visible result,
   exportable state.

Commit-sized tasks:

1. Add visual-novel and layered-mode templates.
2. Add RPG quest template.
3. Add agent workflow template updates for layered/process state.
4. Expand doctor diagnostics.
5. Add template fixtures and verification coverage.

### Lane 4: Persistence UX

Purpose: make runtime story/RPG state durable without silently rewriting source
scene files.

Persistence surfaces:

1. Library API: already has `VisualStoryState` export/import.
2. CLI helper: save, load, inspect, and validate story-state files.
3. Scene action: explicit save/export and load/import requests.
4. GUI overlay: visible save/load status in Scene Mode and Tile Debugger.

Deliverables:

1. Define the save-file path convention, likely
   `~/.local/state/gameterm/scenes/<scene-id>.story.json` unless overridden.
2. Add a story-state helper script or CLI subcommand.
3. Add explicit Scene Mode action requests for save/load/export/import.
4. Include state version, source scene identity, timestamp, variables,
   dialogue, and RPG state.
5. Reject incompatible state with visible errors and no mutation.

Acceptance criteria:

1. Exported state can be imported into a fresh runtime and produce the same
   variables, dialogue position/history, and RPG records.
2. Import rejects incompatible versions and out-of-bounds dialogue positions.
3. Save/load status appears in debug report and Tile Debugger.
4. Source scene JSON is unchanged unless the user runs an explicit export-scene
   authoring helper.
5. CLI/helper tests cover success, validation failure, and path override.

Commit-sized tasks:

1. Define save-file path and metadata schema.
2. Add CLI/helper save/load/inspect commands.
3. Add Scene Mode action requests for persistence.
4. Wire GUI dispatch and status reporting.
5. Add fixtures and verification coverage.

### Lane 5: Agent/Process Integration

Purpose: make Scene Mode the visible state surface for real terminal work,
where commands, agents, and scripts update structured runtime state.

Initial agent/process states:

1. `idle`: no active task.
2. `planning`: plan or checklist is being produced.
3. `running`: command or agent task is executing.
4. `blocked`: task needs user input or external state.
5. `complete`: task finished and produced a result.
6. `failed`: command or agent failed with recoverable status.

Deliverables:

1. Extend process helper patches to include agent/process mode fields.
2. Add structured status operations for start, progress, blocked, complete, and
   failed.
3. Map process results to scene variables, entity metadata, quests/tasks, and
   selected-entity state.
4. Keep raw command output in panes; only structured summaries enter Scene Mode
   through patches/actions.
5. Show source pane, target overlay, process state, and last update in the Tile
   Debugger.

Acceptance criteria:

1. A process helper can mark an entity running, then complete or failed, through
   mux patch transport.
2. Agent/process mode transitions are visible as layered state once Lane 1 is
   implemented.
3. Failed process patches do not close Scene Mode or corrupt state.
4. Multiple overlays still route patches by active overlay or explicit pane id.
5. Focused GUI/overlay tests and smoke helpers cover process-driven patches.

Commit-sized tasks:

1. Add process/agent state patch fields.
2. Add helper commands for start/progress/blocked/complete/failed.
3. Add debug report and Tile Debugger visibility.
4. Add mux transport tests for process-state patches.
5. Add live smoke scenario for process-driven state changes.

### Lane 6: Live Smoke Discipline

Purpose: keep real GUI behavior honest as Scene Mode becomes more stateful.

Smoke scenarios:

1. Launch Scene Mode with bundled default and config default.
2. Submit mux patch to active overlay.
3. Submit mux patch to explicit pane.
4. Navigate between scenes and reload.
5. Exercise guarded choice and guarded input map.
6. Exercise story export/import once GUI persistence exists.
7. Exercise process helper start/complete/fail once agent/process integration
   exists.

Deliverables:

1. Maintain noninteractive tests as the first gate.
2. Maintain macOS live smoke as the real render/overlay/mux gate.
3. Archive screenshots with timestamped names on the Desktop or configured
   artifact path.
4. Add a short smoke result checklist to this roadmap whenever a new scenario
   is introduced.
5. Keep expected pre-existing warnings documented so smoke output is readable.

Acceptance criteria:

1. Every product-loop lane adds or updates at least one deterministic test.
2. Every GUI/transport lane adds or updates one live smoke scenario.
3. Smoke failures produce actionable output: launch failure, missing permission,
   patch failure, capture failure, or visual mismatch.
4. The latest smoke capture path is reported after each manual run.
5. Manual smoke remains optional for pure library changes and required for
   overlay/mux/render changes.

Commit-sized tasks:

1. Add a smoke scenario registry to the helper script.
2. Add guarded input/choice smoke fixture.
3. Add persistence smoke once GUI save/load exists.
4. Add agent/process smoke once structured process state exists.
5. Add documentation for when manual smoke is required.

## Scene Mode Next Product Layer

The product-loop roadmap established the state/runtime foundation. The next
layer should prove that foundation through a real, repeatable GameTerm
experience: smoke-verified GUI behavior, a playable scene, in-app authoring
surfaces, richer state transitions, and live agent/process updates.

Priority order:

1. Live manual smoke pass.
2. First playable vertical slice.
3. GUI authoring/save UX.
4. State transition polish.
5. Agent integration.

These priorities should remain separate commits where possible. The smoke pass
should happen first because it validates the real overlay/mux/render path before
new product behavior builds on it.

### Next 1: Live Manual Smoke Pass

Purpose: verify the real GUI path after the Scene Mode state roadmap: launch,
overlay activation, keyboard handling, mux patches, process-state patches, and
screen capture.

Deliverables:

1. Run every named smoke scenario from `ci/gameterm-scene-smoke.sh
   --list-scenarios` that can run on the current machine.
2. Capture output screenshots to timestamped Desktop paths or a configured
   artifact directory.
3. Record pass/fail status, command used, capture path, and manual observations
   in this roadmap or a dedicated smoke report.
4. Confirm `process-state` shows typed process state in the Tile Debugger.
5. Confirm `guarded-input` keeps Scene Mode open when guards block input.

Acceptance criteria:

1. `renderer-rows`, `guarded-input`, `run-command-targets`, `patch-inbox`,
   `mux-patch`, and `process-state` have recorded results.
2. Any failed scenario has a clear failure class: build, launch, permission,
   capture, patch transport, input handling, or visual/render issue.
3. Successful scenarios include the capture path and the exact command used.
4. Manual smoke does not leave stray GameTerm processes running.
5. Follow-up defects are filed as separate roadmap items instead of mixed into
   the smoke-report commit.

Commit-sized tasks:

1. Add smoke report template and artifact path convention.
2. Run `renderer-rows`, `guarded-input`, and `run-command-targets`.
3. Run `patch-inbox`, `mux-patch`, and `process-state`.
4. Commit the smoke report.
5. Open follow-up fixes for any failures.

### Next 2: First Playable Vertical Slice

Purpose: build one coherent Scene Mode experience that proves visual novel,
RPG, layered state, deterministic actions, persistence, and process state can
compose into something usable.

Target slice:

1. A workspace/project scene with a clear objective.
2. Dialogue introduction and branching choices.
3. One RPG quest with inventory/stat/relationship updates.
4. Layered UI/story/process states visible in the Tile Debugger.
5. A process-driven task that updates an entity from running to complete or
   failed.
6. Exportable story state after the slice is completed.

Deliverables:

1. Add a `vertical-slice` authoring template or fixture.
2. Include deterministic choices that update quest, inventory, stats, and
   relationship state.
3. Include at least one guarded branch unlocked by previous state.
4. Include one process-state path using the existing helper/patch schema.
5. Add verification coverage that loads the scene, validates actions, applies
   process patches, and exports story state.

Acceptance criteria:

1. The scene validates immediately and is included in
   `ci/gameterm-scene-verify.sh --all`.
2. A user can complete the core loop with keyboard input only.
3. The final story export contains the expected quest, inventory/stat, dialogue,
   and relationship state.
4. The process task visibly updates an entity and debug report state.
5. No default action in the slice runs arbitrary shell commands without an
   explicit user choice.

Commit-sized tasks:

1. Add the scene/template skeleton.
2. Add deterministic story/RPG action paths.
3. Add process-state fixture path.
4. Add export/verification coverage.
5. Add docs explaining how to run the slice.

### Next 3: GUI Authoring/Save UX

Purpose: make Scene Mode state useful from inside GameTerm instead of relying
only on shell helpers.

Surfaces:

1. Save/export current story state.
2. Load/import story state.
3. Export current runtime scene after applying patches.
4. Show save/load status and errors in Scene Mode and the Tile Debugger.
5. Optional authoring commands for adding/updating entities and choices later.

Deliverables:

1. Add `VisualActionRequest` variants for story-state export/import or reuse a
   small persistence command request type.
2. Wire GUI overlay dispatch for persistence requests.
3. Define default state paths and user-visible status messages.
4. Add key/input-map actions for save, load, export, and inspect where
   appropriate.
5. Add focused tests for request generation, dispatch status, invalid state,
   and no-mutation failure.

Acceptance criteria:

1. A scene can trigger save/export without editing source scene JSON.
2. Loading invalid or incompatible state reports a visible error and preserves
   runtime state.
3. Save/load actions are available through input maps or choices, not hardcoded
   only to one fixture.
4. The Tile Debugger shows the last persistence action and path.
5. The helper and GUI paths share validation semantics.

Commit-sized tasks:

1. Add persistence action request schema.
2. Wire runtime request generation.
3. Wire GUI dispatch and status reporting.
4. Add tests and fixture coverage.
5. Document save/load workflows.

### Next 4: State Transition Polish

Purpose: make Scene Mode feel like structured game state, not only isolated
actions.

Polish targets:

1. Layer transitions triggered by deterministic actions.
2. Mode/layer transitions driven by process state.
3. Quest-stage transitions that change dialogue, visible entities, and
   available choices.
4. Transition history in the debug report.
5. Clear rollback behavior when a transition guard or operation fails.

Deliverables:

1. Add transition operations to deterministic action resolution.
2. Add guard helpers for process phase, quest status, inventory, and
   relationship state.
3. Record last successful and failed transition in debug output.
4. Add fixtures that demonstrate blocked, successful, and chained transitions.
5. Keep transition behavior deterministic and in-memory unless explicitly
   exported.

Acceptance criteria:

1. A quest action can move the story layer from dialogue to choice or complete.
2. A process completion can unlock a guarded branch or update the process layer.
3. Failed transitions do not partially mutate variables/RPG state/layers.
4. Debug output explains why a guard blocked a transition.
5. Tests cover success, failure, rollback, and debug reporting.

Commit-sized tasks:

1. Add transition operation schema.
2. Add process/quest/inventory guard helpers.
3. Add runtime application and rollback tests.
4. Add fixtures and verifier coverage.
5. Update Tile Debugger output.

### Next 5: Agent Integration

Purpose: connect real agent/task execution to Scene Mode as a structured,
inspectable state surface.

Initial integration shape:

1. Agent/task runners emit typed Scene Mode patches.
2. Raw logs stay in terminal panes.
3. Scene Mode receives summaries, process phases, entity/task status, and
   optional quest/task progress.
4. Explicit pane ids or active overlay routing determine the target.
5. User approval/blocking states are represented as process state, not hidden
   shell state.

Deliverables:

1. Define a stable agent patch contract on top of `process_state`, variables,
   entity metadata, and deterministic state operations.
2. Add helper commands for `planning`, `running`, `blocked`, `complete`, and
   `failed` agent phases.
3. Add a sample agent workflow scene using those patches.
4. Add mux/inbox transport examples for active-overlay and explicit-pane
   routing.
5. Add smoke and verifier coverage for one full agent task lifecycle.

Acceptance criteria:

1. An agent task can update a Scene Mode entity from planning to running to
   complete.
2. A blocked task visibly asks for user input or external state.
3. Failed tasks report recoverable status without closing Scene Mode.
4. Multiple overlays can still receive the intended patch target.
5. The final state is exportable through story-state persistence.

Commit-sized tasks:

1. Add agent helper commands or extend `ci/gameterm-scene-process.sh`.
2. Add sample agent workflow fixture/template.
3. Add mux/inbox transport verification.
4. Add live smoke scenario for the full lifecycle.
5. Document the agent patch contract.

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
3. Provide `ci/gameterm-scene-init.sh` so authors can create editable config
   from the bundled example without hand-building the config directory.
4. Add a manual reload action from inside Scene Mode so authors can update
   `default.json`, return to GameTerm, and refresh without closing the window.
5. Keep the previous valid scene visible if a reload fails. The error should be
   surfaced in Scene Mode and the Tile Debugger status instead of replacing the
   scene with a blank or partial state.
6. Use the bundled default scene only when no default scene file exists at load
   time. If a previously valid default scene later fails to parse, keep the
   previous scene and report the failed reload.
7. Reset selection only when the reloaded scene no longer contains the selected
   entity id. If the id still exists, preserve selection and update the
   inspection panel from the new entity data.

Reload status should distinguish these cases:

- `bundled`: no default scene file exists, so the bundled default scene is
  active.
- `loaded`: the default scene file loaded successfully.
- `reload_failed`: the default scene file exists but the latest reload failed;
  the previous valid scene remains active.
- `invalid`: the scene file failed during initial load and there is no previous
  valid scene to keep.

Automatic file watching is available as an opt-in authoring helper with:

```sh
GAMETERM_SCENE_AUTO_RELOAD=1
```

The watcher checks the active scene file, sprite manifest, and scene directory
between input polls. It intentionally uses the same reload path as `r` so parse
errors, sprite warnings, and previous-valid-scene preservation behave the same
way. Manual reload remains the default because it is predictable across
platforms and keeps parse errors tied to an explicit action.

## Verification Harness

Implemented verification behavior:

1. Use `ci/fixtures/gameterm-scene` for deterministic Scene Mode fixtures.
2. Use `ci/gameterm-scene-verify.sh --all` for noninteractive checks covering
   fixture setup, init helper behavior, authoring validation, JSON validity,
   and focused Rust tests.
3. Use `ci/gameterm-scene-smoke.sh --launch --fixture <name>` for macOS visual
   smoke runs against the same fixture set.
4. Use `ci/gameterm-scene-author.sh validate <path>` for local scene authoring
   checks backed by the Rust scene parser.
5. Use `ci/gameterm-scene-doctor.sh` for combined scene, action target, sprite
   manifest, sprite asset, and sprite id coverage diagnostics.
6. Use `ci/gameterm-scene-smoke.sh --launch --fixture run-command-targets` for
   live mux/window checks of `tab`, `split_right`, and `split_down` command
   targets.
7. Use `cargo run -q -p gameterm-visual --example scene_patch_apply -- SCENE
   PATCH` for fixture-backed validation of in-memory state patches.

## Tile Debugger Path, Action, And Status

The Tile Debugger should show scene runtime source details alongside layer,
action, entity, sprite, position, flag, and metadata inspection.

Implemented source fields:

- `Scene path`: absolute resolved path for `default.json`, or `bundled default`
  when the fallback scene is active.
- `Load status`: one of the statuses above.
- `Reload counter`: monotonic count of reload attempts, including the initial
  load.
- `Error`: concise parse, schema, or I/O failure text when the current status is
  `reload failed` or `invalid`.
- `Action base dir`: directory used to resolve relative action targets.

Implemented action fields:

- `Status`: the last action, reload, or validation status shown in Scene Mode.
- `Selected choice`: selected choice index.
- `Choice label`: selected choice label from the scene JSON.
- `Choice kind`: `Inspect`, `OpenFile`, `RunCommand`, or `Navigate`.
- `Choice detail`: resolved action authoring detail, such as `path=...`,
  `argv=...`, or `target=...`.
- `Pending action`: the action waiting for GUI dispatch, or `none`.

The path/status line should be visible without selecting an entity. Entity
selection should continue to drive entity-specific debugger rows, but source
status and action status belong to the scene as a whole.

The runtime also exposes a structured `VisualSceneDebugReport` so tests and
authoring tools can assert debugger state without scraping rendered text.

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
4. Open valid file targets through the existing platform opener used by
   GameTerm URL handling.

`OpenFile` should not execute shell commands. It is a document/navigation
action, even when the target file extension is executable.

### RunCommand

Implemented `RunCommand` behavior:

1. Require an explicit `argv` array in the scene JSON. Scene Mode does not infer
   commands from labels, metadata, or file paths.
2. Execute the argv directly without invoking a shell.
3. Open the command in the same GameTerm window. `target` defaults to `tab` and
   can opt into `split_right` or `split_down`.
4. Keep Scene Mode responsive while the command pane is spawned.
5. Show command spawn and failure state in the Tile Debugger action status.
6. Treat command output as pane output, not as scene JSON mutation, until a
   separate structured-state update channel is designed.

`RunCommand` is opt-in and visibly represented in the UI before execution. The
JSON scene file is local configuration, but command execution still needs a
clear user action boundary.

### Navigate

Implemented `Navigate` behavior:

1. Emit a navigation request from the scene runtime instead of treating
   navigation as a local status placeholder.
2. Resolve relative navigation targets against the current scene file's
   directory.
3. Load the target scene and update the active scene path so later `r` reloads
   refresh the navigated scene.
4. Keep the current scene visible and report an action error if navigation
   fails.

## Computational Modes

Scene Mode should treat state as active behavior, not only as data shown on a
different screen. A computational mode is a context that can change input,
simulation, rendering, polling, available actions, and transition rules while
the same terminal and mux runtime stays underneath it.

Candidate GameTerm modes:

- Conversation: chat input, LLM responses, dialogue history, context
  inspection.
- Memory: memory navigation, relationship exploration, recall/search.
- Agent: planning, task execution, process monitoring, handoff, and patch
  intake.
- Workspace: files, projects, panes, processes, and command actions.

The first implementation should be declarative. A mode descriptor should be
data that the runtime can inspect, validate, render, and expose through the
Tile Debugger before modes gain richer lifecycle behavior.

### Phase 1: Mode Descriptors

Add a small mode descriptor model:

1. `mode_id`: stable id such as `conversation`, `memory`, `agent`, or
   `workspace`.
2. `label`: user-facing mode name.
3. `description`: short authoring/debug description.
4. `scene_profile`: optional render/layout profile for the mode.
5. `allowed_actions`: the action kinds available in this mode.
6. `default_transition`: optional fallback transition target.

Scene Mode should show the active mode and selected entity mode in the Tile
Debugger. This gives users and tests a way to distinguish "same scene, different
behavior" from "different scene file."

### Phase 2: Lifecycle Hooks

Add explicit behavior boundaries:

1. Enter actions run when a mode becomes active. Implemented first as
   declarative `enter_status` hooks.
2. Update/poll actions run while the mode is active. Implemented first as
   declarative `update_status` hooks.
3. Exit actions run before leaving the mode. Implemented first as declarative
   `exit_status` hooks.
4. Failure during enter or update should transition to a visible error state or
   report a mode error without closing Scene Mode.

Hooks should start with safe, existing action types: scene patches, navigation,
status updates, and process-wrapper integration. They should not introduce
implicit shell execution.

### Phase 3: Input And Transition Rules

Add mode-specific controls:

1. Per-mode choices and key/action maps.
2. Guarded transitions based on flags, variables, selected entity, process
   status, or story state.
3. Explicit fallback/error transitions.
4. Debug output for the last transition attempt, guard result, and target mode.

This is the point where Conversation, Memory, Agent, and Workspace can feel like
different computational realities rather than different pages.

### Phase 4: Layered State Machines

Large game-like systems often need multiple simultaneous state machines. After
single active mode support is stable, support layered modes:

1. Global mode: startup, workspace, paused/error.
2. UI mode: scene, dialogue, debugger, command palette.
3. Agent/process mode: idle, planning, running, blocked, complete.
4. Selected entity mode: idle, focused, actionable, hidden.
5. Story mode: dialogue, choice, transition, save/load.

Layered state should be added only after the single-mode descriptor, lifecycle,
and transition rules are covered by fixtures and focused tests. The current
single-mode runtime now covers descriptors, lifecycle status hooks, guarded
choices, and guarded input/action maps.

## Narrative/RPG Layer

Scene Mode is already close to a visual-novel-style prototype because it has
scene loading, dialogue text, choices, navigation, sprites, entity inspection,
runtime patches, and mux/script transport. The next product track is to make
that direction explicit without turning GameTerm into a separate game engine.

The narrative/RPG layer should stay terminal-first:

- story and RPG state remains visible through Scene Mode and the Tile Debugger;
- actions are explicit choices, commands, patches, or scene transitions;
- scripts and agents update runtime state through the existing patch transport;
- persistence is explicit export/save behavior, not silent source-file mutation;
- Ren'Py, Ink, Yarn, emulator tile debuggers, and RPG engines remain references,
  not vendored runtimes.

Definition: the narrative/RPG layer is structured scene state plus deterministic
runtime transitions. It owns dialogue position, branch availability, story
variables, and lightweight RPG records such as inventory, stats, quests, and
relationships. It does not own terminal process execution, shell parsing, mux
routing, or renderer-specific sprite loading.

Acceptance boundary: each new narrative/RPG concept must be inspectable through
`VisualSceneDebugReport`, visible in the Tile Debugger when active, serializable
as stable JSON, and covered by focused `gameterm-visual` tests before GUI
transport or authoring helpers depend on it.

### Phase 1: Visual Novel Foundation

Add a first-class dialogue model on top of the existing scene JSON shape:

1. Represent dialogue as ordered lines with speaker, body text, optional
   portrait/sprite id, and optional metadata. Implemented with
   `dialogue_lines`.
2. Add dialogue history so users can inspect previous lines in Scene Mode.
   Implemented in runtime state and the Tile Debugger.
3. Let choices advance dialogue, navigate scenes, or apply runtime flags.
   Implemented for dialogue advancement through `AdvanceDialogue`; navigation
   already uses `Navigate`, and state changes continue to use scene patches.
4. Add authoring templates for simple branching dialogue scenes.
5. Cover templates and dialogue parsing in `ci/gameterm-scene-verify.sh --all`.

This phase should prefer static JSON plus runtime patches. It should not add a
general scripting language until the data model is proven.

### Phase 2: Persistent Story State

Add state that can survive scene navigation and GameTerm restarts:

1. Add variables and flags with explicit types and predictable JSON
   serialization.
2. Add conditional choice visibility and conditional entity visibility.
3. Add explicit save/load or export/import helpers for runtime state.
   Implemented in `gameterm-visual` as `VisualStoryState` export/import APIs.
4. Keep source scene files immutable unless the user runs an explicit authoring
   or export command.
5. Show active flags, variables, and save source in the Tile Debugger.

### Phase 3: Lightweight RPG Systems

Prototype RPG features as structured state before adding rules:

1. Inventory: item ids, labels, counts, ownership, and metadata. Implemented as
   validated `VisualInventoryItem` records.
2. Stats: named numeric or textual values attached to player, entities, or
   parties. Implemented as typed `VisualStat` records with optional owners.
3. Quests: quest ids, stages, completion state, and journal text. Implemented
   as `VisualQuest` records.
4. Relationships: named relationship values between entities or agents.
   Implemented as `VisualRelationship` records.
5. Action resolution: small deterministic operations that update the above
   state through the patch runtime.

Combat, equipment rules, leveling, and procedural maps should wait until the
basic inventory/stat/quest model is useful and testable.

### Phase 4: Engine Boundary

After the narrative and RPG data model settles, decide whether it remains in
`gameterm-visual` or moves into a dedicated GameTerm crate. The boundary should
be based on real complexity: parsing/story-state/rules tests may justify a
separate crate, while rendering and mux transport should stay integrated with
the existing Scene Mode path.

## Open Questions

- Should future scene files be able to specify a base directory for relative
  `OpenFile` paths?
- Should `RunCommand` support additional split sizing or domain selection
  beyond the current `tab`, `split_right`, and `split_down` targets?
- Should narrative state live in the existing scene JSON file, in a separate
  runtime-state file, or both?
- What minimum save/load format is enough before RPG inventory and quest state
  are introduced?
- Should dialogue history be patchable through mux, or only produced by local
  Scene Mode choices?
- Should computational modes live inside scene JSON, in a separate mode
  registry, or both?
- Which mode layer should own input when UI mode, agent mode, and story mode all
  have active bindings?

## State Update Channel

The first state update channel is implemented in `gameterm-visual` as an
in-memory runtime patch. The first GUI transport is an explicit local patch
inbox file enabled by `GAMETERM_SCENE_PATCH_FILE`. It is intentionally not
coupled to `RunCommand` stdout: command output remains pane output, and scripts
or agents must write a structured patch file when they want to update Scene
Mode.

Implemented patch contract:

```json
{
  "scene_patch_version": 1,
  "updates": [
    {
      "entity_id": "task-render",
      "label": "Render Verified",
      "position": {"x": 5, "y": 6},
      "sprite": "task_tile_done",
      "visible": true,
      "state_flags": ["running", "verified"],
      "metadata": [["status", "tests passed"]]
    }
  ],
  "selected_entity_id": "task-render",
  "status": "Verification passed"
}
```

Implemented constraints:

- Apply patches only to the active scene runtime, not directly to the source
  JSON file.
- Entity patches may update label, grid position, sprite id, state flags, and
  metadata.
- Entity patches may show or hide entities with `visible`.
- Patches may move focus with `selected_entity_id`.
- Reject unknown entity ids and malformed patches with visible Scene Mode
  status once GUI transport exists. The current library API returns a typed
  error.
- Reject patched entity positions outside the current scene bounds.
- Bump the visual generation after every accepted patch so render caches
  invalidate correctly.
- Keep persistence as a later explicit authoring action rather than silently
  rewriting local config.

Current verification:

```sh
cargo run -q -p gameterm-visual --example scene_patch_apply -- \
  ci/fixtures/gameterm-scene/default.json \
  ci/fixtures/gameterm-scene/patch-status.json
```

Authoring and inbox helpers:

```sh
ci/gameterm-scene-patch.sh set-entity \
  --output /tmp/gameterm-scene-patch.json \
  --entity-id project-harness \
  --status "Verification passed" \
  --label "Harness Verified" \
  --position 5,3 \
  --sprite project_core \
  --select-entity-id project-harness \
  --visible \
  --flag loaded --flag verified \
  --metadata status=patched

ci/gameterm-scene-author.sh new-template \
  --template agent-workflow \
  ~/.config/gameterm/scenes/default.json

ci/gameterm-scene-process.sh \
  --entity-id project-harness \
  --inbox /tmp/gameterm-scene-patch.json \
  --select \
  -- cargo test -p gameterm-visual scene_patch

ci/gameterm-scene-patch.sh write-inbox \
  --inbox /tmp/gameterm-scene-patch.json \
  --patch ci/fixtures/gameterm-scene/patch-status.json

ci/gameterm-scene-patch.sh export-scene \
  --scene ci/fixtures/gameterm-scene/default.json \
  --patch ci/fixtures/gameterm-scene/patch-status.json \
  --output /tmp/gameterm-scene-export.json
```

Implemented transport behavior:

- The overlay records the patch file's current modification time at startup.
- A patch is applied only after the watched file appears or changes.
- Active Scene Mode overlays subscribe to local
  `MuxNotification::GameTermScenePatch { patch_json, target_pane_id,
  source_pane_id }` notifications and apply those patches through the same
  runtime path as the file inbox.
- The mux tracks the active Scene Mode overlay pane. Submitters can target a
  specific overlay pane or default to the active overlay.
- `gameterm cli scene-patch --patch PATCH` submits a patch through the mux
  protocol and prints the target Scene Mode pane id on success.
- Accepted patches update the active runtime and bump visual generation.
- Rejected patches update Scene Mode status without mutating scene state.
- Missing active overlays and missing explicit target panes fail as transport
  errors before runtime patch validation.
- Runtime status and the Tile Debugger record the last patch transport and
  source pane when the submitter provides one.
- The patch file is not copied into the scene JSON and is not treated as
  persistent storage.
- Persistence is available only through the explicit `export-scene` helper,
  which writes a new scene JSON file after applying a patch.

Next transport step:

1. Keep multiple simultaneous Scene Mode overlays allowed, with the most
   recently opened overlay as the default active target and explicit pane ids
   for older overlays.
2. Keep the `mux-patch` smoke path using a uniquely classed, published GUI,
   with `GAMETERM_UNIX_SOCKET` unset for class-targeted CLI calls and explicit
   pane targeting discovered from `gameterm cli --class CLASS list --format
   json`; use the listed Scene pane when exposed, otherwise the active pane
   that owns the overlay.
3. Keep `GAMETERM_SCENE_PATCH_FILE` as the portable fallback and smoke-test
   path.
