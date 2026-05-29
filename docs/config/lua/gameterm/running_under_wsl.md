---
title: gameterm.running_under_wsl
tags:
 - utility
---
# `gameterm.running_under_wsl()`

This function returns a boolean indicating whether we believe that we are
running in a Windows Services for Linux (WSL) container.  In such an
environment the `gameterm.target_triple` will indicate that we are running in
Linux but there will be some slight differences in system behavior (such as
filesystem capabilities) that you may wish to probe for in the configuration.

```lua
local gameterm = require 'gameterm'
gameterm.log_error(
  'System '
    .. gameterm.target_triple
    .. ' '
    .. tostring(gameterm.running_under_wsl())
)
```


