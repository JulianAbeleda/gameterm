# GameTerm Scene Mode

GameTerm Scene Mode is a native visual state surface inspired by visual novel scene composition and emulator tile/sprite debugging.

The first implementation is intentionally small:

- a built-in demo scene
- optional JSON scene loading from `~/.config/gameterm/scenes/default.json`
- symbolic project/task/agent entities
- keyboard selection
- a dialogue/inspection panel
- placeholder command actions
- a Tile Debugger view for scene layers, entities, sprite ids, positions, flags, and metadata

This does not vendor or emulate Ren'Py, Ink, Yarn, mGBA, SameBoy, or ares. Those projects are references for architecture and workflow only. GameTerm keeps its scene model native so it can remain terminal-first and integrated with panes, commands, scripts, and future structured state.

Open the scene with:

```lua
{ key = 'g', mods = 'CTRL|SHIFT', action = gameterm.action.ShowGameTermScene },
```

## Scene file

Scene Mode looks for a default scene at:

```text
~/.config/gameterm/scenes/default.json
```

If `XDG_CONFIG_HOME` is set, the path is:

```text
$XDG_CONFIG_HOME/gameterm/scenes/default.json
```

If the file is missing, Scene Mode uses the built-in demo scene. If the file is present but invalid, Scene Mode shows an error frame and stays open so the problem is visible.

The scene file uses the same JSON shape as `VisualScene`: title, background, width, height, entities, dialogue speaker/text, and choices. See [the example scene](examples/gameterm-scene-default.json).

## Smoke test

The macOS visual smoke test uses `ffmpeg` screen capture through AVFoundation:

```sh
ci/gameterm-scene-smoke.sh --launch
```

After GameTerm opens, press `Ctrl+Shift+G` before the capture timer expires.

macOS requires Screen Recording permission for the terminal or host app that
runs the script. Enable it in System Settings -> Privacy & Security -> Screen
Recording, then fully quit and reopen that app before rerunning the smoke test.

For the next native bitmap rendering step, see [GameTerm Renderer Path](gameterm-renderer-path.md).
