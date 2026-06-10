# GameTerm Scene Mode Compose Slash Commands Scope

Status: IMPLEMENTED (lanes 1-4 landed; live dogfood pending).

Implementation record (2026-06-09):

- `[visual] resolve Scene Codex model from config and argv`
- `[visual] parse Scene compose slash commands`
- `[visual] route Scene compose slash commands and model override`
- `[visual] hint Scene compose slash commands in the dock`

Verification record:

- `cargo test -p gameterm-gui compose --bin gameterm-gui`: 61 passed
- `cargo test -p gameterm-visual`: 242 passed
- `cargo check -p gameterm-gui`: no errors
- `ci/gameterm-scene-verify.sh --all`: all functional checks print `ok`;
  the run exits 101 on a pre-existing broken-pipe panic in the verifier's
  trailing step, reproduced identically on the pre-change baseline commit
  `21f3c9ffe`, so it is unrelated to this work.

Scope trim: the dock footer hint shows a generic command list rather than the
active model name, to avoid threading session model state into the render
signatures. The active model is reported on demand by `/model` with no
argument.

Remaining: live dogfood (`/model spark`, confirm faster reply, `/model`
reports model, `/help`, `/clear`).

This scope adds local slash commands to the Scene Mode composer. The driving
need is runtime model switching: the operator is low on Codex tokens and needs
to pin compose turns to a cheaper model (`gpt-5.3-codex-spark`, "spark")
without quitting, editing `scene-compose.json`, and relaunching.

Slash commands are local composer actions. They never spawn the Codex backend
and never cost tokens. They are the right home for any composer action that
does not need an LLM round-trip.

## Audit Context

- Compose turns run `codex exec` per submit (`run_codex_compose_backend`).
- The model is currently whatever the global `~/.codex/config.toml` selects;
  the compose argv passes no `-m`, so there is no per-Scene model control.
- The recent reasoning-effort override (`codex_reasoning_effort`) proved the
  pattern for a config-resolved, argv-applied Codex knob.
- `SceneComposeDock::handle_key` already returns a `Submitted(String)` action
  the loop dispatches; that is the natural interception point.
- arkey-rs ships the same idea: a slash-prefixed draft shows command
  completion hints in the dock footer (`slash_command_footer_text`).

## Goals

- `/model <name>` switches the compose backend model for subsequent turns,
  with `spark` resolving to `gpt-5.3-codex-spark`.
- `/model` with no argument reports the active model.
- A small set of obvious local commands that need no backend: `/clear`
  (transcript), `/help` (command list).
- Slash commands are parsed by a pure, unit-tested function so the event loop
  only performs the side effect.
- Slash commands run as local actions: they bypass the backend busy guard and
  never enqueue TTS.
- The composer footer hints when the draft is a slash command, mirroring the
  idle composer affordance.

## Non-Goals

- No remote or backend-executed commands (those stay normal prompts).
- No Codex session reuse or streaming (separate scope).
- No model discovery/validation against a live model list; an unknown model
  string is passed through and Codex surfaces rejection through the existing
  failure path.
- No general scripting or macro system.
- No persistence of the runtime model override across overlay restarts in this
  pass (config file remains the durable default).

## Model Selection Model

Resolution order for the model used on a turn, highest priority first:

1. Runtime override set by `/model <name>` this session.
2. `scene-compose.json` `codex_model` or
   `GAMETERM_SCENE_COMPOSE_CODEX_MODEL`.
3. Unset: inherit the global Codex config model (current behavior).

Friendly aliases (case-insensitive), everything else passed through literally:

- `spark` -> `gpt-5.3-codex-spark`

The runtime override is session-local. It resets when the overlay is
reopened; durable defaults belong in `scene-compose.json`.

## Lanes

### Lane 1: Codex Model In Config And Argv

Type: behavior fix. Prerequisite for any model control.

Target: `gameterm-gui/src/overlay/visual_compose_backend.rs`.

- Add `model: Option<String>` to `CodexComposeConfig`, resolved from
  `GAMETERM_SCENE_COMPOSE_CODEX_MODEL` / `codex_model` like
  `reasoning_effort`.
- `codex_compose_argv`: when model is `Some`, push `-m <model>`.
- No alias logic here; this lane only plumbs a literal model string.

Acceptance:

```sh
cargo test -p gameterm-gui codex --bin gameterm-gui
```

Tests: argv includes `-m` when model set, omits it when unset.

### Lane 2: Slash Command Parser

Type: new pure module.

Target: new `gameterm-gui/src/overlay/visual_compose_commands.rs`.

- `SceneComposeCommand` enum: `Model { name: Option<String> }`, `Clear`,
  `Help`, `Unknown(String)`.
- `parse_scene_compose_command(input: &str) -> Option<SceneComposeCommand>`:
  returns `None` for non-slash input (a normal prompt), `Some(Unknown)` for an
  unrecognized `/word`.
- `resolve_model_alias(name: &str) -> String`: applies the alias table.
- A footer hint helper for slash-prefixed drafts.

Acceptance:

```sh
cargo test -p gameterm-gui compose_command --bin gameterm-gui
```

Tests: `/model spark` parses with resolved alias; `/model` parses with no
name; `/clear`, `/help` parse; `hello` returns `None`; `/bogus` returns
`Unknown`; alias resolution and pass-through.

### Lane 3: Session Override And Loop Routing

Type: behavior fix.

Targets: `visual_overlay_session.rs`, `visual_loop.rs`,
`visual_compose_backend.rs` (`ComposeBackendRequest`), `visual_voice_events.rs`.

- `ComposeBackendRequest` gains `model_override: Option<String>`.
- `run_codex_compose_backend` applies `model_override` over `config.model`
  before building argv.
- `VisualOverlaySession` gains `compose_model: Option<String>` and a helper to
  report the active model label.
- In the loop `Submitted(prompt)` branch, before the busy guard, call
  `parse_scene_compose_command`. If `Some`, handle locally:
  - `Model { Some(name) }`: set `session.compose_model` to the resolved alias,
    `mark_action_status("Compose model: <name>")`, record in dock history.
  - `Model { None }`: status reports the active model.
  - `Clear`: `runtime.clear_compose_history()`, reset scroll.
  - `Help`: status lists the commands.
  - `Unknown`: status reports the unknown command and points at `/help`.
  Then clear the buffer, render, and continue. No backend spawn, no TTS.
- Typed and voice submit both thread `session.compose_model` into the request.
  (Voice has no slash entry path; it only inherits the override.)

Acceptance:

```sh
cargo test -p gameterm-gui compose --bin gameterm-gui
```

Tests: a `/model spark` submit sets the override and spawns no backend; the
next real submit carries the override into the request; `/clear` empties
history; slash command while backend running is still handled locally.

### Lane 4: Footer Hint And Verification

Type: render polish + verification.

- The composer dock footer shows a slash hint when the draft starts with `/`
  (active model and `/help`), reusing the existing waiting/idle placeholder
  slot precedence: waiting indicator wins, then slash hint, then idle text.

```sh
cargo test -p gameterm-visual
cargo test -p gameterm-gui compose --bin gameterm-gui
cargo check -p gameterm-gui
ci/gameterm-scene-verify.sh --all
```

Live dogfood: `/model spark`, confirm status, send a turn, confirm faster
reply; `/model` reports spark; `/help` lists commands; `/clear` empties the
transcript.

## Done Means

- `/model spark` pins compose turns to `gpt-5.3-codex-spark` live, no restart.
- `/model` reports the active model; `/help` and `/clear` work locally.
- Slash commands cost no tokens and never spawn the backend.
- Model resolution order is runtime override, then config, then global default.
- Tests, GUI check, and Scene verifier recorded; dogfood checklist run.

## First Implementation Slice

1. `[visual] resolve Scene Codex model from config and argv` (Lane 1)
2. `[visual] parse Scene compose slash commands` (Lane 2)
3. `[visual] route Scene compose slash commands and model override` (Lane 3)
4. `[visual] hint Scene compose slash commands in the dock` (Lane 4)

Lane 1 is independent and lands first. Lanes 2-4 build the command surface on
top.
