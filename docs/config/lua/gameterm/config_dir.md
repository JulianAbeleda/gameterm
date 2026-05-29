---
title: gameterm.config_dir
tags:
 - filesystem
---

# `gameterm.config_dir`

This constant is set to the path to the directory in which your `gameterm.lua`
configuration file was found.

```lua
local gameterm = require 'gameterm'
gameterm.log_error('Config Dir ' .. gameterm.config_dir)
```


