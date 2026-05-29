---
title: gameterm.strftime_utc
tags:
 - utility
 - time
 - string
---
# `gameterm.strftime_utc(format)`

{{since('20220624-141144-bd1b7c5d')}}

Formats the current UTC date/time into a string using [the Rust chrono
strftime syntax](https://docs.rs/chrono/0.4.19/chrono/format/strftime/index.html).

```lua
local gameterm = require 'gameterm'

local date_and_time = gameterm.strftime_utc '%Y-%m-%d %H:%M:%S'
gameterm.log_info(date_and_time)
```

See also [strftime](strftime.md) and [gameterm.time](../gameterm.time/index.md).
