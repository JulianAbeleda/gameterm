---
title: gameterm.has_action
tags:
 - utility
 - version
---

# gameterm.has_action(NAME)

{{since('20230408-112425-69ae8472')}}

Returns true if the string *NAME* is a valid key assignment action variant
that can be used with [gameterm.action](action.md).

This is useful when you want to use a gameterm configuration across multiple
different versions of gameterm.

```lua
if gameterm.has_action 'PromptInputLine' then
  table.insert(config.keys, {
    key = 'p',
    mods = 'LEADER',
    action = gameterm.action.PromptInputLine {
      -- other parameters here
    },
  })
end
```
