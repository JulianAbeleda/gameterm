# Command Line

This section documents the gameterm command line.

*Note that `gameterm --help` or `gameterm SUBCOMMAND --help` will show the precise
set of options that are applicable to your installed version of gameterm.*

gameterm is deployed with two major executables:

* `gameterm` (or `gameterm.exe` on Windows) - for interacting with gameterm from the terminal
* `gameterm-gui` (or `gameterm-gui.exe` on Windows) - for spawning gameterm from a desktop environment

You will typically use `gameterm` when scripting gameterm; it knows when to
delegate to `gameterm-gui` under the covers.

If you are setting up a launcher for gameterm to run in the Windows GUI
environment then you will want to explicitly target `gameterm-gui` so that
Windows itself doesn't pop up a console host for its logging output.

!!! note
    `gameterm-gui.exe --help` will not output anything to a console when
    run on Windows systems, because it runs in the Windows GUI subsystem and has no
    connection to the console.  You can use `gameterm.exe --help` to see information
    about the various commands; it will delegate to `gameterm-gui.exe` when
    appropriate.

## Synopsis

```console
{% include "../examples/cmd-synopsis-gameterm--help.txt" %}
```
