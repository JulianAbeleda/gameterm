# Migrating From WezTerm

GameTerm is a hard rename of a WezTerm fork. It intentionally uses GameTerm-branded binary names, crate names, config names, runtime paths, and Lua module names.

## Config Paths

GameTerm does not automatically read existing WezTerm config paths.

Common path changes:

| Purpose | WezTerm | GameTerm |
| --- | --- | --- |
| XDG config directory | `~/.config/wezterm/` | `~/.config/gameterm/` |
| Home config file | `~/.wezterm.lua` | `~/.gameterm.lua` |
| XDG config file | `wezterm.lua` | `gameterm.lua` |
| Runtime/state data | `~/.local/share/wezterm/` | `~/.local/share/gameterm/` |

If you want to reuse an existing WezTerm config as a starting point, copy it to the GameTerm path and update Lua module imports from `wezterm` to `gameterm`.

For example:

```sh
mkdir -p ~/.config/gameterm
cp ~/.config/wezterm/wezterm.lua ~/.config/gameterm/gameterm.lua
```

Then update config imports:

```lua
local gameterm = require 'gameterm'
```

instead of:

```lua
local wezterm = require 'wezterm'
```

## Compatibility Policy

This rename pass does not preserve WezTerm config compatibility shims. Treat GameTerm as a fork with a separate config namespace.

Future compatibility aliases may be added deliberately, but they are not part of the initial hard rename.
