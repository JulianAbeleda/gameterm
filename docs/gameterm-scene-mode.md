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
ci/gameterm-scene-author.sh format ~/.config/gameterm/scenes/experiment.json
ci/gameterm-scene-doctor.sh
```

`validate` runs the same Rust scene parser used by Scene Mode and prints a
short summary of the scene dimensions, entity count, choices, and initial
selection.

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

`cwd` is optional. `target` is optional and defaults to `tab`; supported values
are `tab`, `split_right`, and `split_down`. Scene Mode opens the command in the
same window, then reports the spawned pane id or spawn failure in the status
line and Tile Debugger. Command output belongs to the spawned pane; it does not
mutate the scene JSON.

`Navigate` choices load another scene JSON file. Relative navigation targets
are resolved against the directory of the currently active scene file. After a
successful navigation, `r` reloads the active scene rather than returning to
`default.json`. If navigation fails, Scene Mode keeps the current scene visible
and reports the error in the scene status.

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
`MuxNotification::GameTermScenePatch { patch_json, source_pane_id }`. Active
Scene Mode overlays subscribe to that notification and apply the same patch
schema used by the file inbox. This notification is intentionally local-only;
the mux server/client protocol ignores it until the remote IPC boundary is
designed explicitly.

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

To live-audit RunCommand pane targets, use:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-smoke.sh --launch --fixture run-command-targets
```

After opening Scene Mode, trigger the three choices in order to verify `tab`,
`split_right`, and `split_down` behavior in the real mux/window path.

To live-audit the patch inbox, launch with an inbox path:

```sh
ci/gameterm-scene-smoke.sh --launch --fixture basic --patch-inbox auto
```

After Scene Mode opens, write a patch with `ci/gameterm-scene-patch.sh
write-inbox` using the inbox path printed by the smoke script.

Use `--min-bytes N` to make capture output checks stricter for local visual
regression runs.

After GameTerm opens, press `Ctrl+Shift+G` before the capture timer expires.

macOS requires Screen Recording permission for the terminal or host app that
runs the script. Enable it in System Settings -> Privacy & Security -> Screen
Recording, then fully quit and reopen that app before rerunning the smoke test.

For the next native bitmap rendering step, see [GameTerm Renderer Path](gameterm-renderer-path.md).
