# Troubleshooting

## Review logs/error messages

If things aren't working out, there may be an issue printed in the logs.
Read on to learn more about how to see those logs.

### Debug Overlay

By default, pressing <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>L</kbd> will activate
the debug overlay and allow you to review the most recently logged issues.
It also gives you access to a Lua REPL for evaluating built-in lua functions.

See [ShowDebugOverlay](config/lua/keyassignment/ShowDebugOverlay.md) for more
information on this key assignment.

### Log Files

You can find log files in `$XDG_RUNTIME_DIR/gameterm` on unix systems,
or `$HOME/.local/share/gameterm` on macOS and Windows systems.

### Increasing Log Verbosity

The `GAMETERM_LOG` environment variable can be used to adjust the level
of logging for different modules within gameterm.

To see maximum verbosity, you can start gameterm like this:

```
GAMETERM_LOG=debug gameterm
```

to see debug level logs for everything on stdout.

On Windows systems you'll usually need to set the environment variable separately:

Using `cmd.exe`:

```
C:\> set GAMETERM_LOG=debug
C:\> gameterm
```

Using powershell:

```
PS C:\> $env:GAMETERM_LOG="debug"
PS C:\> gameterm
```

When using a flatpak you must first enter the flatpak container by running:

```
flatpak run --command=sh --devel org.wezfurlong.gameterm
```

Before then running `gameterm`.

Each log line will include the module name, which is a colon separated
namespace; in the output below the modules are `config`,
`gameterm_gui::frontend`, `gameterm_font::ftwrap` and `gameterm_gui::termwindow`:

```
10:29:24.451  DEBUG  config                    > Reloaded configuration! generation=2
10:29:24.452  DEBUG  gameterm_gui::frontend     > workspace is default, fixup windows
10:29:24.459  DEBUG  gameterm_font::ftwrap      > set_char_size computing 12 dpi=124 (pixel height=20.666666666666668)
10:29:24.461  DEBUG  gameterm_font::ftwrap      > set_char_size computing 12 dpi=124 (pixel height=20.666666666666668)
10:29:24.494  DEBUG  gameterm_gui::termwindow   > FocusChanged(true)
10:29:24.495  DEBUG  gameterm_gui::termwindow   > FocusChanged(false)
```

Those modules generally match up to directories and file names within the
gameterm source code, or to external modules that gameterm depends upon.

You can set a more restrictive filter to focus in on just the things you want.
For example, if you wanted to debug only configuration related things you might
set:

```
GAMETERM_LOG=config=debug,info
```

which says:

* log `config` at `debug` level
* everything else at `info` level

You can add more comma-separated items:

```
GAMETERM_LOG=config=debug,gameterm_font=debug,info
```

See Rust's [env_logger
documentation](https://docs.rs/env_logger/latest/env_logger/#enabling-logging)
for more details on the syntax/possibilities.

## Debugging Keyboard Related issues

Turn on [debug_key_events](config/lua/config/debug_key_events.md) to log
information about key presses.

Use [gameterm show-keys](cli/show-keys.md) or `gameterm show-keys --lua` to show
the effective set of key and mouse assignments defined by your config.

Consider changing [use_ime](config/lua/config/use_ime.md) to see that is
influencing your keyboard usage.

Double check to see if you have some system level utility/software that might
be intercepting or changing the behavior of a keyboard shortcut that you're
trying to use.

## Debugging Font Display

Use `gameterm ls-fonts` to explain which fonts will be used for different styles
of text.

Use `gameterm ls-fonts --list-system` to get a list of fonts available on your
system, in a form that you can use in your config file.

Use `gameterm ls-fonts --text foo` to explain how gameterm will render the text
`foo`, and `gameterm ls-fonts --text foo --rasterize-ascii` to show an ascii art
rendition of that text.

