# GameTerm Scene Mode Real Codex Dogfood Scope

Status: SCOPED.

This document scopes the pass that turns the implemented fake/one-shot Codex
compose bridge into a dogfoodable Scene Mode feature. The existing bridge can
already run `codex exec` when `GAMETERM_SCENE_COMPOSE_BACKEND_KIND=codex` is
present. The gap is that normal app launches still default to the deterministic
backend, real Codex failures are not surfaced as a first-class health state,
and the live timeout/config path is too env-only for daily use.

## Current Baseline

Current local state:

- `4707ffed2 [visual] render compose prompts in VN dialogue`
- `main` is ahead of `origin/main` by 1 commit
- worktree is clean

Scene Mode compose currently supports:

- deterministic built-in compose replies
- explicit command backend through `GAMETERM_SCENE_COMPOSE_BACKEND`
- explicit Codex backend through `GAMETERM_SCENE_COMPOSE_BACKEND_KIND=codex`
- local Codex CLI command construction:
  - `codex exec`
  - `--output-last-message <tempfile>`
  - `--json`
  - `-C <workspace>`
  - `-s <sandbox>`
  - `-c approval_policy="<policy>"`
- fake-Codex smoke through `vn-compose-codex`
- VN dialogue rendering of submitted prompts and replies:
  - `> user prompt`
  - blank separator
  - Codex/backend reply

Recent real-Codex probe:

```sh
codex exec --output-last-message /tmp/gameterm-real-codex-test.txt \
  -C /Users/julianabeleda/env/gameterm \
  -s read-only \
  -c 'approval_policy="never"' \
  --json 'Reply exactly: hi'
```

Result: failed before producing a final message because the current local Codex
service/session state returned `429 Too Many Requests` and websocket `403
Forbidden` reconnect errors. This is not a Scene rendering failure, but Scene
Mode needs to make this kind of state obvious to the user.

## Product End Goal

When this pass is complete, the user can launch GameTerm from the macOS app,
choose Scene Mode from the boot menu, type into the Composer dock, press Enter,
and see one of two clear outcomes:

1. Real Codex succeeds:
   - status changes through `Codex running` to `Codex succeeded`
   - the dialogue box shows the user's highlighted prompt and Codex's reply
   - the Composer dock is ready for the next prompt

2. Real Codex is unavailable:
   - status changes to `Codex unavailable` or `Codex failed`
   - the dialogue box shows a short, readable diagnostic
   - the user can continue typing or switch back to deterministic mode
   - Scene Mode does not hang, silently fail, or hide the failure in logs

## Non-Goals

This pass should not implement:

- direct OpenAI API calls
- raw interactive Codex TUI embedding
- token streaming as the primary success path
- full approval UI inside Scene Mode
- dangerous sandbox bypass by default
- automatic network dispatch without an explicit Scene/App config choice
- persistent `codex exec resume` sessions
- multi-agent orchestration

Persistent session support is a follow-up after one-shot live Codex is
dogfoodable.

## Lane 1: Persistent Scene Compose Config

Purpose: make Finder/app launches use the intended compose backend without
requiring terminal environment variables.

Add a small Scene compose config model with env overrides preserved:

```text
backend_kind = built_in | command | codex
command = optional argv/string for command backend
codex_bin = optional path, default "codex"
codex_workspace = optional path, default active scene/workspace cwd
codex_sandbox = read-only | workspace-write | danger-full-access
codex_approval = on-request | never | untrusted
codex_timeout_seconds = positive integer
```

Recommended config source for first pass:

- user config file under the existing GameTerm config home, for example
  `~/.config/gameterm/scene-compose.json`
- env vars continue to override config for smoke and debugging
- deterministic built-in backend remains the default if no config is present

Acceptance:

- normal app launch can opt into real Codex without a shell wrapper
- env-only smoke scenarios continue to work
- invalid config values produce readable Scene diagnostics
- no secrets are written to config

## Lane 2: Real Codex Health And Diagnostics

Purpose: make Codex availability clear before and after submit.

Add a lightweight health path:

- detect whether the configured `codex` binary exists
- record `codex --version` when available
- classify common failure modes:
  - missing binary
  - timeout
  - nonzero exit
  - rate limit / `429`
  - websocket/auth / `403`
  - empty final message
- render a short user-facing diagnostic in the dialogue box
- keep detailed stderr available in debug/log output

Acceptance:

- a real `429` failure renders as a Codex availability issue, not a generic
  compose failure
- missing binary says which binary/path failed
- the Composer dock remains usable after failure
- tests use fake commands and do not require network or a real Codex account

## Lane 3: Timeout And Process Behavior

Purpose: make one-shot Codex usable without allowing stuck background jobs.

Current compose timeout is 15 seconds. That is fine for deterministic/local
helpers but tight for real Codex.

Scope:

- make timeout configurable
- keep deterministic/default backend timeout small
- use a larger real-Codex default, for example 90 seconds
- keep hard process kill on timeout
- preserve stdout/stderr collection without pipe deadlocks

Acceptance:

- deterministic backend behavior does not slow down
- real Codex has enough time for short prompts
- timeout message includes the configured timeout
- tests cover timeout selection per backend kind

## Lane 4: App Launch Consistency

Purpose: ensure the clickable macOS app points at the current build and reads
the same Scene compose config every time.

Scope:

- keep `ci/install-macos-dev-app.sh` as the development install path
- rebuild and reinstall `/Users/julianabeleda/Applications/GameTerm.app`
- avoid app-bundle-only environment hacks as the primary product path
- optionally add a development helper profile only for smoke/debug launch

Acceptance:

- clicking GameTerm launches the current rebuilt binary
- app-launched Scene Mode can use the configured Codex backend
- terminal-launched and Finder-launched behavior match aside from env overrides

## Lane 5: Smoke Coverage

Purpose: prove both the deterministic contract and the live-Codex path.

Deterministic smoke remains:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario vn-compose-codex \
  --output /tmp/gameterm-scene-vn-compose-codex-fake.png
```

Add or document a live-Codex smoke:

```sh
GAMETERM_SCENE_COMPOSE_BACKEND_KIND=codex \
GAMETERM_SCENE_COMPOSE_CODEX_TIMEOUT_SECONDS=90 \
ci/gameterm-scene-smoke.sh --launch --scenario vn-compose \
  --key-sequence 'text:say hi,enter,delay:8' \
  --output ~/Desktop/gameterm-scene-smoke-vn-real-codex-YYYYMMDD-HHMMSS.png
```

The live smoke may fail because of account/rate/auth state. That is acceptable
only if the screenshot and report show a readable in-Scene Codex diagnostic.

Acceptance:

- fake-Codex smoke passes
- live-Codex smoke either succeeds or fails with a readable Scene diagnostic
- smoke report records command, result, screenshot path, and failure class

## Lane 6: Documentation And User Workflow

Purpose: make the feature usable without remembering env vars.

Update docs with:

- how to enable real Codex backend
- how to switch back to deterministic compose
- expected sandbox and approval defaults
- how to run fake-Codex smoke
- how to run live-Codex smoke
- what `429` and `403` mean operationally
- current limitation: no persistent Codex session yet

Acceptance:

- roadmap links this scope
- smoke report records latest fake/live evidence
- handoff records current commit, app install state, and Codex availability

## Proposed Commit Plan

Keep commits separate by concern:

```text
[docs] scope Scene real Codex dogfood pass
[gui] add Scene compose config
[gui] classify Scene Codex backend failures
[gui] tune Scene Codex backend timeout
[test] cover Scene real Codex config and diagnostics
[docs] record Scene real Codex smoke
```

If the app install script needs a narrow change:

```text
[test] update macOS dev app install smoke path
```

## Verification Checklist

Current pass results:

- `cargo test -p gameterm-gui compose_backend --bin gameterm-gui`: PASS
- `cargo test -p gameterm-gui codex_compose --bin gameterm-gui`: PASS
- `cargo test -p gameterm-gui visual_compose --bin gameterm-gui`: PASS
- `cargo check -p gameterm-gui`: PASS with pre-existing warning noise
- `cargo build -p gameterm-gui`: PASS with pre-existing warning noise
- `vn-compose-codex` fake-Codex smoke: PASS at
  `/tmp/gameterm-scene-vn-compose-codex-fake.png`
- `vn-compose` real-Codex smoke: PASS at
  `/tmp/gameterm-scene-vn-real-codex-diagnostic.png`
- Earlier direct live-Codex probe returned `429 Too Many Requests` plus
  websocket `403 Forbidden`; the later Scene smoke succeeded, so treat that as
  transient account/session state unless it reappears.

Targeted checks:

```sh
cargo test -p gameterm-gui compose_backend --bin gameterm-gui
cargo test -p gameterm-gui codex_compose --bin gameterm-gui
cargo test -p gameterm-gui scene_compose --bin gameterm-gui
cargo check -p gameterm-gui
```

Smoke checks:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-smoke.sh --launch --scenario vn-compose-codex \
  --output /tmp/gameterm-scene-vn-compose-codex-fake.png
ci/install-macos-dev-app.sh --no-build
```

Manual/live check when Codex account state allows:

```sh
GAMETERM_SCENE_COMPOSE_BACKEND_KIND=codex \
GAMETERM_SCENE_COMPOSE_CODEX_TIMEOUT_SECONDS=90 \
ci/gameterm-scene-smoke.sh --launch --scenario vn-compose \
  --key-sequence 'text:say hi,enter,delay:8' \
  --output ~/Desktop/gameterm-scene-smoke-vn-real-codex-YYYYMMDD-HHMMSS.png
```

## Stop Conditions

Stop and rescope if:

- current Codex CLI changes the `exec` command contract
- app-launched Codex cannot access the same auth state as terminal-launched
  Codex
- live Codex requires interactive approval UI before the one-shot path can run
- the solution requires storing tokens or account secrets in GameTerm config
- the implementation starts turning Scene Mode into a raw terminal emulator
  inside the overlay

## Follow-Up Scope

After this pass, the next Codex scope should be persistent sessions:

- parse `thread_id` or session metadata from Codex JSONL
- persist session identity per Scene overlay/session
- add reset/new-session action
- add `codex exec resume` support
- decide whether progress/streaming events should render into Scene Mode
