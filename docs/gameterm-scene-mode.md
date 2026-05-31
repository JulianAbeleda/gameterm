# GameTerm Scene Mode

GameTerm Scene Mode is a native visual state surface inspired by visual novel scene composition and emulator tile/sprite debugging.

The first implementation is intentionally small:

- a bundled default scene
- optional JSON scene loading from `~/.config/gameterm/scenes/default.json`
- optional sprite manifest loading from `~/.config/gameterm/scenes/sprites.json`
- a bundled sprite fallback for first-run rendering
- optional auto-reload with `GAMETERM_SCENE_AUTO_RELOAD=1`
- symbolic project/task/agent entities
- keyboard selection
- manual reload while Scene Mode is open
- a dialogue/inspection panel
- local action status for inspection and document targets
- a Tile Debugger view for scene source, load status, action state, selected
  choice, pending action, layers, entities, sprite ids, positions, flags, and
  metadata

This does not vendor or emulate Ren'Py, Ink, Yarn, mGBA, SameBoy, or ares. Those projects are references for architecture and workflow only. GameTerm keeps its scene model native so it can remain terminal-first and integrated with panes, commands, scripts, and future structured state.

Open the scene with:

```lua
{ key = 'g', mods = 'CTRL|SHIFT', action = gameterm.action.ShowGameTermScene },
```

## Authoring quickstart

Create an editable copy of the bundled scene with:

```sh
ci/gameterm-scene-init.sh
```

This writes:

```text
~/.config/gameterm/scenes/default.json
```

Edit that file, return to Scene Mode, and press `r` to reload it without
restarting GameTerm. If the edited JSON is invalid, Scene Mode keeps the
previous valid scene visible when possible and shows the error in the scene
status and Tile Debugger.

The helper does not overwrite existing files unless `--force` is passed. To
also copy the starter sprite manifest, run:

```sh
ci/gameterm-scene-init.sh --with-sprites
```

The sprite manifest is optional because Scene Mode can use bundled sprite
defaults while custom sprite files are being created.

For fixture-driven authoring, use:

```sh
ci/gameterm-scene-author.sh list-fixtures
ci/gameterm-scene-author.sh install-fixture navigate --force
ci/gameterm-scene-author.sh validate ~/.config/gameterm/scenes/default.json
ci/gameterm-scene-author.sh new-scene ~/.config/gameterm/scenes/experiment.json
ci/gameterm-scene-author.sh add-entity ~/.config/gameterm/scenes/experiment.json \
  --id task-demo --kind Task --label "Demo Task" --x 2 --y 2 --sprite task_tile
ci/gameterm-scene-author.sh add-choice ~/.config/gameterm/scenes/experiment.json \
  --label "Run visual check" --run-argv '["cargo","check","-p","gameterm-visual"]' \
  --target split_right
ci/gameterm-scene-author.sh update-choice ~/.config/gameterm/scenes/experiment.json \
  --choice-index 1 --label "Open docs" --open-file docs/gameterm-scene-mode.md
ci/gameterm-scene-author.sh remove-choice ~/.config/gameterm/scenes/experiment.json \
  --choice-index 1
ci/gameterm-scene-author.sh move-entity ~/.config/gameterm/scenes/experiment.json \
  --id task-demo --x 3 --y 2
ci/gameterm-scene-author.sh set-dialogue ~/.config/gameterm/scenes/experiment.json \
  --speaker "Author" --text "Updated locally."
ci/gameterm-scene-author.sh set-variable ~/.config/gameterm/scenes/experiment.json \
  --key intro_complete --value-bool false
ci/gameterm-scene-author.sh add-layer ~/.config/gameterm/scenes/experiment.json \
  --layer-id story --state dialogue --label Story
ci/gameterm-scene-author.sh add-layer-transition ~/.config/gameterm/scenes/experiment.json \
  --layer-id story --input activate --target-state exploration \
  --condition-variable intro_complete --condition-bool true
ci/gameterm-scene-author.sh add-mode-input ~/.config/gameterm/scenes/experiment.json \
  --input other --action run_update_hooks \
  --condition-source inventory_count --condition-variable field-map --condition-number 1
ci/gameterm-scene-author.sh set-lifecycle ~/.config/gameterm/scenes/experiment.json \
  --enter-status "Scene entered" --update-status "Scene updated" \
  --exit-status "Scene exited"
ci/gameterm-scene-author.sh add-inventory ~/.config/gameterm/scenes/experiment.json \
  --item-id field-map --label "Field Map" --count 1
ci/gameterm-scene-author.sh set-stat ~/.config/gameterm/scenes/experiment.json \
  --owner-id player --key focus --value-number 1
ci/gameterm-scene-author.sh add-quest ~/.config/gameterm/scenes/experiment.json \
  --quest-id first-route --label "First Route" --stage 1 \
  --journal "Find the first route."
ci/gameterm-scene-author.sh format ~/.config/gameterm/scenes/experiment.json
ci/gameterm-scene-doctor.sh
```

`validate` runs the same Rust scene parser used by Scene Mode and prints a
short summary of the scene dimensions, entity count, choices, and initial
selection.

Guided templates are available through the authoring helper:

```sh
ci/gameterm-scene-author.sh new-template \
  --template vertical-slice \
  ~/.config/gameterm/scenes/default.json
```

The `vertical-slice` template combines dialogue, guarded choices, deterministic
story/RPG actions, layered state, and a process-state task entity into one
playable Scene Mode loop.

The `workspace-agent` template is the first Agent/Workspace product slice. It
represents workspace, project, task, agent, process, and file entities, then
uses deterministic choices and external patches to move agent work through
planning, running, blocked, and review-ready states:

```sh
ci/gameterm-scene-author.sh new-template \
  --template workspace-agent \
  ~/.config/gameterm/scenes/default.json
```

To generate a Scene Mode workspace view from the current repo, use the
workspace discovery helper:

```sh
ci/gameterm-scene-workspace.sh inspect --cwd .
ci/gameterm-scene-workspace.sh discover \
  --cwd . \
  --scene-output /tmp/gameterm-workspace.json
ci/gameterm-scene-author.sh validate /tmp/gameterm-workspace.json
```

To install the generated scene after validation:

```sh
ci/gameterm-scene-workspace.sh discover --cwd . --install --force
```

Discovery reads local cwd, git metadata, important files, and explicit command
hints. It does not run verification commands; generated commands appear as
explicit Scene Mode choices.

When caller code has active pane metadata, it can pass that context into the
same helper:

```sh
ci/gameterm-scene-workspace.sh discover \
  --pane-cwd . \
  --pane-id 231 \
  --mux-window-id 7 \
  --foreground-process-name zsh \
  --scene-output /tmp/gameterm-workspace.json
```

If `--cwd` is omitted, `--pane-cwd` becomes the discovery cwd. Pane and process
metadata is rendered as scene variables, entity metadata, and patch process
state when available.

For final app-level verification, use the product smoke checklist in
[`docs/gameterm-scene-product-smoke.md`](gameterm-scene-product-smoke.md).

`doctor` checks the configured scene and sprite manifest together. It validates
the scene, reports missing navigation and document targets, checks sprite
manifest shape, checks sprite asset paths, verifies RunCommand targets and
optional cwd directories, and warns when scene sprite ids have no manifest
entry. Use `--strict` when warnings should fail a local or CI run.

## Scene file

Scene Mode looks for a default scene at:

```text
~/.config/gameterm/scenes/default.json
```

If `XDG_CONFIG_HOME` is set, the path is:

```text
$XDG_CONFIG_HOME/gameterm/scenes/default.json
```

If the file is missing, Scene Mode uses the bundled default scene. If the file is present but invalid on initial load, Scene Mode shows an error frame and stays open so the problem is visible.

The scene file uses the same JSON shape as `VisualScene`: title, background, width, height, entities, dialogue speaker/text, and choices. See [the example scene](examples/gameterm-scene-default.json).

Press `r` while Scene Mode is open to reload the scene file and sprite manifest.
If reload fails after a valid scene is already active, Scene Mode keeps the
previous scene visible and reports the reload error in the scene status and Tile
Debugger. Selection is preserved across successful reloads when the selected
entity id still exists.

For authoring sessions that need live refresh, start GameTerm with:

```sh
GAMETERM_SCENE_AUTO_RELOAD=1 target/debug/gameterm-gui start
```

Auto-reload watches the active scene file, the sprite manifest, and the scene
directory for modification-time changes. It uses the same reload path as `r`, so
failed reloads preserve the previous valid scene when one exists. Manual `r`
reload remains the deterministic fallback.

`OpenFile` choices resolve their configured path against GameTerm's current
working directory. If the target is a file, Scene Mode asks the platform to open
it with the default application. Missing paths and directory targets are
reported in Scene Mode without closing the scene. `OpenFile` does not execute
shell commands.

`RunCommand` choices execute an explicit argv array without invoking a shell:

```json
{
  "label": "Run visual check",
  "kind": {
    "RunCommand": {
      "argv": ["cargo", "check", "-p", "gameterm-visual"],
      "cwd": "/path/to/workspace",
      "target": "split_right"
    }
  }
}
```

`cwd` is optional. For generated or reusable scenes, prefer setting `cwd`
explicitly so the command is tied to an auditable workspace root. `doctor` warns
when a `RunCommand` choice has no cwd because the command then depends on launch
context. `target` is optional and defaults to `tab`; supported values are
`tab`, `split_right`, and `split_down`. Scene Mode opens the command in the same
window, then reports the spawned pane id or spawn failure in the status line and
Tile Debugger. Command output belongs to the spawned pane; it does not mutate
the scene JSON.

`Navigate` choices load another scene JSON file. Relative navigation targets
are resolved against the directory of the currently active scene file. After a
successful navigation, `r` reloads the active scene rather than returning to
`default.json`. If navigation fails, Scene Mode keeps the current scene visible
and reports the error in the scene status.

`ExportStoryState` and `ImportStoryState` choices save and load runtime story
state without rewriting the source scene JSON:

```json
{
  "label": "Save story",
  "kind": { "ExportStoryState": { "path": "saves/default.story.json" } }
}
```

Relative paths resolve against GameTerm's current working directory. Input maps
can also use `export_story_state` or `import_story_state`; those use a default
path next to the active scene such as `default.story.json`.

For workspace-oriented persistence, use the workspace session helper. It wraps
the same validated runtime state with workspace metadata and never rewrites the
source scene JSON:

```sh
ci/gameterm-scene-session.sh save \
  --scene ~/.config/gameterm/scenes/default.json \
  --workspace-root /path/to/workspace \
  --output /tmp/gameterm-workspace.session.json

ci/gameterm-scene-session.sh inspect \
  --session /tmp/gameterm-workspace.session.json

ci/gameterm-scene-session.sh restore \
  --scene ~/.config/gameterm/scenes/default.json \
  --session /tmp/gameterm-workspace.session.json \
  --output /tmp/gameterm-workspace-restored.story.json
```

Relationships are explicit RPG state records that connect scene entities:

```json
{
  "source_id": "discovered-task",
  "target_id": "file-0",
  "kind": "references",
  "value": 1,
  "metadata": [
    ["source", "workspace-discovery"],
    ["reason", "task related_files metadata"]
  ]
}
```

`source_id` and `target_id` must reference entities in the same scene. The
normal Scene Mode view shows incoming/outgoing relationship counts and a compact
summary for the selected entity. The Tile Debugger shows full relationship rows
with labels, ids, kind, value, and metadata. Workspace Discovery generates
local deterministic relationships for workspace/project/file/task/process
entities; it does not perform background indexing or semantic recall.

`Resolve` choices can update layer state in the same deterministic transaction
as story/RPG state:

```json
{
  "label": "Complete scene",
  "kind": {
    "Resolve": {
      "operations": [
        { "SetLayerState": { "layer_id": "story", "state": "complete" } }
      ]
    }
  }
}
```

Invalid layer ids or empty target states fail validation. Runtime failures keep
variables, RPG state, and layers unchanged.

Choice, input-map, and layer-transition guards read typed variables by default.
Set `source` on a condition to guard against lightweight RPG state instead:

- `inventory_count`: inventory count by `item_id`.
- `inventory_has`: whether an inventory item is present.
- `quest_stage`: quest stage by `quest_id`.
- `quest_completed`: quest completion by `quest_id`.
- `stat`: stat value by `key` or `owner_id:key`.

`ci/gameterm-scene-agent.sh` emits Scene Mode patches for agent lifecycle
states without running a process:

```sh
ci/gameterm-scene-agent.sh status \
  --entity-id project-harness \
  --phase planning \
  --message "Planning visual slice" \
  --patch /tmp/gameterm-agent.json
```

Supported phases are `idle`, `planning`, `running`, `waiting`, `blocked`,
`complete`/`completed`, `failed`, and `cancelled`. The helper maps those phases
onto typed process state so Scene Mode can render agent work as queued,
running, blocked, succeeded, or failed. It also writes `agent_phase` and
`agent_process_phase` variables into the patch, so scenes can drive guards or
layer transitions from the agent lifecycle without parsing entity metadata.

## State patches

Scene Mode now has a versioned in-memory patch schema in `gameterm-visual`.
Patches update the active runtime only; they do not rewrite the source scene
JSON.

```json
{
  "scene_patch_version": 1,
  "updates": [
    {
      "entity_id": "task-render",
      "state_flags": ["running", "verified"],
      "metadata": [["status", "tests passed"]]
    }
  ],
  "status": "Verification passed"
}
```

The first supported update fields are `state_flags` and `metadata`. Unknown
entity ids are rejected and accepted patches bump the visual generation so
renderer caches refresh. The current command-line verification path is:

```sh
cargo run -q -p gameterm-visual --example scene_patch_apply -- \
  ci/fixtures/gameterm-scene/default.json \
  ci/fixtures/gameterm-scene/patch-status.json
```

The helper script wraps common patch workflows:

```sh
ci/gameterm-scene-patch.sh set-entity-status \
  --output /tmp/gameterm-scene-patch.json \
  --entity-id project-harness \
  --status "Verification passed" \
  --flag loaded --flag verified \
  --metadata status=patched

ci/gameterm-scene-patch.sh validate \
  --scene ci/fixtures/gameterm-scene/default.json \
  --patch /tmp/gameterm-scene-patch.json
```

The portable GUI transport from panes or agents into the active Scene Mode
overlay is an explicit local patch file. Start GameTerm with:

```sh
GAMETERM_SCENE_PATCH_FILE=/tmp/gameterm-scene-patch.json target/debug/gameterm-gui start
```

While Scene Mode is open, a pane, script, or agent can atomically write a patch
to that path. The overlay polls for modification-time changes between input
events, applies valid patches to the active runtime, and reports malformed
patches or unknown entity ids in the Scene Mode status and Tile Debugger. The
patch file is a transport inbox, not persistent scene storage.

In-process GameTerm callers can also publish a local mux notification:
`MuxNotification::GameTermScenePatch { patch_json, target_pane_id,
source_pane_id }`. Active Scene Mode overlays register their pane id with the
mux, and patch notifications are routed to the requested target pane or the
currently active Scene Mode overlay. Multiple Scene Mode overlays may exist;
the most recently opened overlay is the active default target, and explicit
`--target-pane-id` submission is the stable way to address an older overlay.

The command-line submit path uses the mux protocol and returns the target Scene
Mode pane id on success:

```sh
gameterm cli scene-patch --patch ci/fixtures/gameterm-scene/patch-status.json

ci/gameterm-scene-patch.sh submit-mux \
  --patch ci/fixtures/gameterm-scene/patch-status.json
```

For automated smoke launches, prefer explicit targeting through an isolated GUI
class. The active overlay fallback is process-local, while the CLI submit path
can be handled by a separate mux process or by the caller's inherited
`GAMETERM_UNIX_SOCKET`. The smoke harness launches a uniquely classed GUI,
unsets `GAMETERM_UNIX_SOCKET` for class-targeted CLI calls, discovers the open
Scene Mode target from `gameterm cli --class CLASS list --format json`,
preferring a listed `GameTerm Scene` pane title when one is exposed and
otherwise using the active pane id that owns the overlay, then submits with
`--target-pane-id`.

If no Scene Mode overlay is active, or if an explicit target pane no longer
exists, submission fails as a transport error. Malformed JSON or unknown entity
ids are runtime patch errors and are shown in Scene Mode status without
mutating the active scene. The Tile Debugger reports the last patch transport
and source pane when that information is available.

Use the helper to write the inbox atomically:

```sh
ci/gameterm-scene-patch.sh write-inbox \
  --inbox /tmp/gameterm-scene-patch.json \
  --patch ci/fixtures/gameterm-scene/patch-status.json
```

To persist a patched runtime state intentionally, export a new scene file:

```sh
ci/gameterm-scene-patch.sh export-scene \
  --scene ci/fixtures/gameterm-scene/default.json \
  --patch /tmp/gameterm-scene-patch.json \
  --output /tmp/gameterm-scene-export.json
```

This is an explicit authoring step. Scene Mode never silently rewrites the
active scene file when a patch is applied.

## Sprite manifest

Scene Mode can render scene sprite ids through image files listed in:

```text
~/.config/gameterm/scenes/sprites.json
```

The manifest maps sprite ids used by the scene `background` and entity `sprite`
fields to image paths. Relative paths are resolved against the directory that
contains `sprites.json`.

```json
{
  "sprites": [
    { "id": "workspace-map", "path": "sprites/workspace-map.png" },
    { "id": "project_core", "path": "sprites/project-core.png" },
    { "id": "agent_idle", "path": "sprites/agent-idle.png" }
  ]
}
```

If the manifest is missing, Scene Mode uses bundled sprite defaults. If a
manifest is invalid or references files that cannot be read, Scene Mode still
opens and uses deterministic placeholder blocks for unresolved sprite ids.
Warnings are shown in the scene frame and logged for renderer-side file loading
failures.

See [the example sprite manifest](examples/gameterm-scene-sprites.json).

## Smoke test

The noninteractive verification harness checks scene fixtures, authoring init
behavior, authoring validation, doctor output, JSON validity, debugger state,
scene patch application, and focused Rust tests:

```sh
ci/gameterm-scene-verify.sh --all
```

To check one fixture setup path:

```sh
ci/gameterm-scene-verify.sh --fixture navigate
```

The macOS visual smoke test uses `ffmpeg` screen capture through AVFoundation:

```sh
ci/gameterm-scene-smoke.sh --launch
```

The smoke test can launch with a specific fixture:

```sh
ci/gameterm-scene-smoke.sh --launch --fixture sprites
```

Named smoke scenarios are available for repeatable audits:

```sh
ci/gameterm-scene-smoke.sh --list-scenarios
ci/gameterm-scene-smoke.sh --describe-scenario process-state
ci/gameterm-scene-smoke.sh --launch --scenario process-state
```

To live-audit the playable vertical slice, use:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario vertical-slice
```

The scenario sends `enter,j,enter,j,enter,j,enter` by default to accept the
brief, prepare the launch kit, complete the scene loop, and read the ending.

To live-audit the in-app authoring loop, use:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario authoring-loop
```

The scenario sends `enter,j,enter,j,enter` by default to save story state,
mutate draft state, and reload the saved state from inside Scene Mode.

To live-audit RunCommand pane targets, use:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-smoke.sh --launch --scenario run-command-targets
```

The scenario sends `enter,j,enter,j,enter` by default, which activates the
`tab`, `split_right`, and `split_down` choices in the real mux/window path.
Override it with `--key-sequence LIST` when auditing a narrower interaction.

To live-audit overlay cleanup, use:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario overlay-cleanup
```

The scenario sends `escape` before capture and should show the underlying
terminal instead of Scene Mode.

To live-audit the patch inbox, launch with an inbox path:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario patch-inbox
```

The smoke script writes the fixture patch into the printed inbox path before
capture.

To live-audit agent lifecycle patches, use:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario agent-lifecycle
```

The scenario emits `planning`, `blocked`, and `complete` patches through the
auto-created inbox and captures the final completed agent state. The
deterministic verifier also covers `waiting` and `cancelled` phase patches.

To live-audit the Agent/Workspace product slice, use:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario workspace-agent
```

The scenario launches the `workspace-agent` fixture, runs a real `true` command
through the process helper, emits planning/running/blocked/complete agent
patches, and captures the final workspace state.

To live-audit generated workspace discovery, use:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario workspace-discovery
```

The scenario generates a scene from the current repository, launches it, and
captures the generated workspace view.

To live-audit mux submission, let the smoke script launch GameTerm, open Scene
Mode before the wait timer expires, and have the script submit a patch before
capture:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario mux-patch
```

Use `--min-bytes N` to make capture output checks stricter for local visual
regression runs.

On macOS, `--launch` now foregrounds the launched GameTerm process and sends
`Ctrl+Shift+G` before capture. Use `--no-auto-open-scene` to disable that
automation and open Scene Mode manually. The script prints the frontmost macOS
process before capture and fails if the launched GameTerm process is not
frontmost, so missed-focus captures do not silently pass. Use
`--allow-background-capture` only when intentionally collecting a best-effort
capture.
Use `--key-sequence` with comma-separated keys such as `space,enter` or
`delay:1,escape` to automate post-launch Scene Mode interaction.
Use `--post-action-wait N` to give patch writes or key sequences more time to
render before capture on slower machines.

macOS requires Screen Recording permission for the terminal or host app that
runs the script. Enable it in System Settings -> Privacy & Security -> Screen
Recording, then fully quit and reopen that app before rerunning the smoke test.
The automatic Scene Mode shortcut also requires Accessibility permission for
the same terminal or host app.

For the next native bitmap rendering step, see [GameTerm Renderer Path](gameterm-renderer-path.md).
