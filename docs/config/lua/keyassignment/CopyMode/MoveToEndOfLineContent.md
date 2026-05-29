# CopyMode `MoveToEndOfLineContent`

{{since('20220624-141144-bd1b7c5d')}}

Moves the CopyMode cursor position to the last non-space cell in the current
line.

```lua
local gameterm = require 'gameterm'
local act = gameterm.action

return {
  key_tables = {
    copy_mode = {
      {
        key = '$',
        mods = 'NONE',
        action = act.CopyMode 'MoveToEndOfLineContent',
      },
    },
  },
}
```


