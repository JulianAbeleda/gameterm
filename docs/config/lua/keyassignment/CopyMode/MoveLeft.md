# CopyMode `MoveLeft`

{{since('20220624-141144-bd1b7c5d')}}

Moves the CopyMode cursor position one cell to the left.

```lua
local gameterm = require 'gameterm'
local act = gameterm.action

return {
  key_tables = {
    copy_mode = {
      { key = 'h', mods = 'NONE', action = act.CopyMode 'MoveLeft' },
      { key = 'LeftArrow', mods = 'NONE', action = act.CopyMode 'MoveLeft' },
    },
  },
}
```
