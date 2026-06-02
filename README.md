# GameTerm

<img height="128" alt="GameTerm Icon" src="https://raw.githubusercontent.com/gameterm/gameterm/main/assets/icon/gameterm-icon.svg" align="left">

A GPU-accelerated terminal emulator with a built-in scene engine, written in
Rust.

GameTerm is a hard fork of WezTerm. It keeps the terminal foundation that makes
WezTerm exceptional, including GPU rendering, multiplexing, SSH, and Lua
scripting, then builds something new on top: Scene Mode, a visual novel and RPG
scene runtime that runs inside your terminal.

Your workspace becomes a scene. Your projects, agents, and tasks become
entities you can navigate, inspect, and interact with. Dialogue, choices,
inventory, quests, and relationships are rendered by the same GPU pipeline that
draws your text.

![Screenshot](docs/screenshots/two.png)

## What's Different From WezTerm

GameTerm is not a configuration fork. The differences are architectural:

- Scene Mode: a full scene runtime with entities, dialogue, state variables,
  RPG mechanics, and a GPU-rendered stage
- VN Script Import: parse Ren'Py `.rpy` scripts into GameTerm scenes
- Asset Pipeline: sprite manifests, asset catalogs, and attribution tracking
- Workspace Scene Generator: generate a scene from your current project and
  terminal pane context
- AI Compose: LLM-backed scene and dialogue composition through explicit,
  auditable backends
- Hard rename: config paths, crate names, binaries, and Lua modules use the
  `gameterm` namespace

WezTerm configs are not loaded automatically. See [MIGRATION.md](MIGRATION.md).

## Scene Mode

Scene Mode is the core of what makes GameTerm different.

A scene is a JSON file that describes a world: a background, a grid of
entities, dialogue, choices, and state. The terminal renders it through the same
WebGPU pipeline that renders text. Sprites are real PNGs. VN panels can use
9-sliced textures. Layouts adapt to terminal size.

Entities have kinds such as `Agent`, `Memory`, `Principle`, `Project`, and
`Task`. They carry state flags, metadata, and relationships to each other.
Choices can be locked behind conditions. Actions can open files, run commands,
advance dialogue, navigate between scenes, or trigger state operations that
adjust variables, inventory, stats, quests, and relationship values.

Story state is exportable and importable. Scenes can be hot-reloaded. A tile
debugger is built in.

Read more in [GameTerm Scene Mode](docs/gameterm-scene-mode.md).

## Ren'Py Import

GameTerm can parse a conservative subset of Ren'Py `.rpy` scripts and convert
them into native Scene Mode scenes. Asset bindings map Ren'Py character
definitions and image paths to GameTerm sprite IDs.

## Workspace Scene

GameTerm can scan the current working directory and generate a scene
automatically: project structure becomes entities, and terminal panes can be
reflected as live context.

## Installation

Installation packaging is still in progress. See [docs/installation.md](docs/installation.md)
for the current installation notes.

## Migrating From WezTerm

GameTerm uses its own config and runtime paths, such as `~/.config/gameterm/`
and `~/.gameterm.lua`. Existing WezTerm configs are not loaded automatically.

See [MIGRATION.md](MIGRATION.md) for path changes and a basic copy-forward
example.

## Getting Help

- [GitHub Issues](https://github.com/JulianAbeleda/gameterm/issues)
- [GitHub Discussions](https://github.com/JulianAbeleda/gameterm/discussions)
- [Matrix room via Element.io](https://app.element.io/#/room/#gameterm:matrix.org)

## License

See [LICENSE.md](LICENSE.md).
