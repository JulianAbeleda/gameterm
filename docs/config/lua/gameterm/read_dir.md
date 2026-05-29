---
title: gameterm.read_dir
tags:
 - utility
 - filesystem
---
# `gameterm.read_dir(path)`

{{since('20200503-171512-b13ef15f')}}

This function returns an array containing the absolute file names of the
directory specified.  Due to limitations in the lua bindings, all of the paths
must be able to be represented as UTF-8 or this function will generate an
error.

```lua
local gameterm = require 'gameterm'

-- logs the names of all of the entries under `/etc`
for _, v in ipairs(gameterm.read_dir '/etc') do
  gameterm.log_error('entry: ' .. v)
end
```


