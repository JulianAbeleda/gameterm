# `ShowGameTermActivePaneScene`

Generates a transient GameTerm Scene Mode workspace from the active pane and
opens it immediately.

This action does not modify `~/.config/gameterm/scenes/default.json`. It uses
the active pane id, mux window id, pane cwd, foreground process name, and pane
progress as generation context, then validates the generated scene before
rendering it.

Default key assignment:

```lua
{ key = 'g', mods = 'CTRL|ALT|SHIFT', action = gameterm.action.ShowGameTermActivePaneScene },
```

Use [`ShowGameTermScene`](ShowGameTermScene.md) when you want to open the
configured default scene instead.
