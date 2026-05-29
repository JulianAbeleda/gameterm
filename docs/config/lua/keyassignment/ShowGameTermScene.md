# `ShowGameTermScene`

Activates GameTerm Scene Mode.

Scene Mode is the first GameTerm-native visual state surface. It displays a symbolic workspace scene with selectable entities, a visual-novel-style inspection panel, choices, and a tile debugger view.

By default, it loads `~/.config/gameterm/scenes/default.json` when present. If the file is absent, it uses the built-in demo scene.

Default key assignment:

```lua
{ key = 'g', mods = 'CTRL|SHIFT', action = gameterm.action.ShowGameTermScene },
```

While Scene Mode is active:

- Arrow keys or `h`/`j`/`k`/`l` move selection.
- `Enter` activates the selected choice.
- `Tab` toggles the Tile Debugger.
- `Esc` or `q` closes the scene.
