# `ToggleFullScreen`

Toggles full screen mode for the current window.

```lua
local gameterm = require 'gameterm'

config.keys = {
  {
    key = 'n',
    mods = 'SHIFT|CTRL',
    action = gameterm.action.ToggleFullScreen,
  },
}
```

See also: [native_macos_fullscreen_mode](../config/native_macos_fullscreen_mode.md).

