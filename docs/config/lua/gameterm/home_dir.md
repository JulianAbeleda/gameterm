---
title: gameterm.home_dir
tags:
 - utility
 - filesystem
---

# `gameterm.home_dir`

This constant is set to the home directory of the user running `gameterm`.

```lua
local gameterm = require 'gameterm'
gameterm.log_error('Home ' .. gameterm.home_dir)
```


