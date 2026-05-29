# Lua Reference

GameTerm provides Lua 5.4 as a configuration language. This section documents
the various lua functions and types that are provided to the configuration
file. These are provided by the `gameterm` module that must be imported into
your configuration file:

```lua
local gameterm = require 'gameterm'
local config = {}
config.font = gameterm.font 'JetBrains Mono'
return config
```

## Full List of Configuration Options

[Config Options](config/index.md) has a list of the main configuration options.

