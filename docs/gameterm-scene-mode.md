# GameTerm Scene Mode

GameTerm Scene Mode is a native visual state surface inspired by visual novel scene composition and emulator tile/sprite debugging.

The first implementation is intentionally small:

- a built-in demo scene
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
