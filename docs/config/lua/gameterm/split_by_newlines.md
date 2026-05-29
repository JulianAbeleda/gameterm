---
title: gameterm.split_by_newlines
tags:
 - utility
 - string
---
# `gameterm.split_by_newlines(str)`

{{since('20200503-171512-b13ef15f')}}

This function takes the input string and splits it by newlines (both `\n` and `\r\n`
are recognized as newlines) and returns the result as an array of strings that
have the newlines removed.

```lua
local gameterm = require 'gameterm'

local example = 'hello\nthere\n'

for _, line in ipairs(gameterm.split_by_newlines(example)) do
  gameterm.log_error(line)
end
```


