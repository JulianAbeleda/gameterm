# `ShowGameTermScene`

Activates GameTerm Scene Mode.

Scene Mode is the first GameTerm-native visual state surface. It displays a symbolic workspace scene with selectable entities, a visual-novel-style inspection panel, choices, and a tile debugger view.

By default, it loads `~/.config/gameterm/scenes/default.json` when present. If the file is absent, it uses the bundled default scene.

To create an editable scene config from the bundled example, run:

```sh
ci/gameterm-scene-init.sh
```

Scene Mode also looks for `~/.config/gameterm/scenes/sprites.json`. This JSON
file maps scene sprite ids to image files:

```json
{
  "sprites": [
    { "id": "project_core", "path": "sprites/project-core.png" },
    { "id": "agent_idle", "path": "sprites/agent-idle.png" }
  ]
}
```

Relative sprite paths are resolved against the directory containing
`sprites.json`. If the manifest is absent, Scene Mode uses bundled sprite
defaults. Missing or invalid sprite entries fall back to visible placeholder
blocks so Scene Mode remains usable while the manifest is fixed.

Default key assignment:

```lua
{ key = 'g', mods = 'CTRL|SHIFT', action = gameterm.action.ShowGameTermScene },
```

While Scene Mode is active:

- Arrow keys or `h`/`j`/`k`/`l` move selection.
- `Enter` activates the selected choice.
- `Tab` toggles the Tile Debugger.
- `r` reloads the scene file and sprite manifest.
- `Esc` or `q` closes the scene.
