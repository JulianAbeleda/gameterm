# `QuickSelect`

{{since('20210502-130208-bff6815d')}}

Activates [Quick Select Mode](../../../quickselect.md).

```lua
local gameterm = require 'gameterm'

config.keys = {
  { key = ' ', mods = 'SHIFT|CTRL', action = gameterm.action.QuickSelect },
}
```

See also [QuickSelectArgs](QuickSelectArgs.md)
