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

1. Run the live smoke flow regularly: open Scene Mode during
   `ci/gameterm-scene-smoke.sh --launch --submit-mux-patch PATCH` and confirm
   the captured scene reflects the submitted patch.
2. Keep multiple simultaneous Scene Mode overlays allowed, with the most
   recently opened overlay as the default active target and explicit pane ids
   for older overlays.
3. Keep `GAMETERM_SCENE_PATCH_FILE` as the portable fallback and smoke-test
   path.
