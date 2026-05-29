---
title: gameterm.action_callback
tags:
 - keys
 - event
---

# `gameterm.action_callback(callback)`

{{since('20211204-082213-a66c61ee9')}}

This function is a helper to register a custom event and return an action triggering it.

It is helpful to write custom key bindings directly, without having to declare
the event and use it in a different place.

The implementation is essentially the same as:
```lua
function gameterm.action_callback(callback)
  local event_id = '...' -- the function generates a unique event id
  gameterm.on(event_id, callback)
  return gameterm.action.EmitEvent(event_id)
end
```

See [gameterm.on](./on.md) and [gameterm.action](./action.md) for more info on what you can do with these.


## Usage

```lua
local gameterm = require 'gameterm'

return {
  keys = {
    {
      mods = 'CTRL|SHIFT',
      key = 'i',
      action = gameterm.action_callback(function(win, pane)
        gameterm.log_info 'Hello from callback!'
        gameterm.log_info(
          'WindowID:',
          win:window_id(),
          'PaneID:',
          pane:pane_id()
        )
      end),
    },
  },
}
```
