# GameTerm Scene Mode Compose Stabilization Scope

Status: SCOPED.

This scope follows the June 1, 2026 Scene compose and Codex bridge pass. It is a
principle-driven stabilization pass for the compose surface, not a new product
feature pass.

The June 1 work is directionally aligned with GameTerm's purpose, but it left
too much process execution and compose UI behavior inside the GUI overlay
module. This pass tightens that code against
`structure/Development/coding-principles.md`: keep fork-specific changes
narrow, preserve upstream behavior, split behavior fixes from NFC refactors,
and verify GUI/process changes with focused and broadened checks.

## Goals

- Make Scene compose backend execution safe for quoted paths, long output, and
  timeout handling.
- Preserve the existing compose dock, VN dialogue patch behavior, Codex bridge
  environment variables, and smoke scenarios.
- Reduce `gameterm-gui/src/overlay/visual.rs` coupling around compose backend
  execution without broad overlay redesign.
- Keep the Scene JSON schema stable.
- Record a repeatable verification gate for compose, Codex bridge, and staged
  VN rendering.

## Non-Goals

- No persistent Codex session implementation.
- No streaming assistant output.
- No network or API default backend.
- No new scene schema fields.
- No visual redesign of the VN dock or panels.
- No broad `gameterm-gui` overlay reorganization outside compose-owned code.
- No cleanup of unrelated macOS Objective-C warning noise.

## Coding-Principle Constraints

Fork discipline:

- Keep changes isolated to GameTerm Scene Mode code and helper scripts.
- Do not change upstream terminal, mux, or renderer behavior except where Scene
  Mode already hooks into those surfaces.
- Do not mix installer, app identity, or unrelated release work into this pass.

Rust workspace discipline:

- Keep compose execution inside `gameterm-gui` unless a later pass proves that
  `gameterm-visual` needs a pure data contract.
- Keep dialogue patch and runtime state behavior inside `gameterm-visual`.
- Prefer small internal modules over new crates.
- Preserve existing public `gameterm_visual` exports.

Commit discipline:

- Use `[gui]` for compose overlay/backend execution changes.
- Use `[visual]` only for runtime or patch behavior changes.
- Use `[test]` for verification-only additions.
- Use `[docs]` for this scope and any follow-up notes.
- Mark mechanical moves with `NFC`; do not mix them with behavior fixes.

Verification discipline:

- Run focused unit tests after each lane.
- Broaden to GUI check and Scene verifier before claiming the pass is done.
- Run live smoke if input handling, rendering, or smoke automation changes.

## Current Risks

### Command String Parsing

`GAMETERM_SCENE_COMPOSE_BACKEND` is currently parsed with whitespace splitting.
That breaks quoted paths and quoted arguments.

Risk:

- a backend such as `/tmp/my tools/reply --mode "short reply"` cannot be
  represented reliably
- users may work around it with shell wrappers, which makes execution less
  auditable

### Piped Output Timeout

The backend process is polled with `try_wait()` while stdout and stderr are
piped but not read until process exit.

Risk:

- a verbose backend can block on a full pipe
- Scene Mode can report a timeout even though the backend is making progress
- stderr diagnostics may be truncated by failure mode rather than policy

### Overlay Coupling

`gameterm-gui/src/overlay/visual.rs` owns compose input, backend config, process
execution, Codex argv construction, output parsing, patch application, file
watching, and rendering glue.

Risk:

- future compose changes are harder to review narrowly
- behavior fixes and NFC moves are easy to mix accidentally
- tests must compile a large overlay module for small backend behavior

### Render-State Assumption

VN panel texture rendering unwraps the GUI render state after checking image
allowance.

Risk:

- likely acceptable in the normal renderer path, but it is an avoidable crash
  edge for alternate render-state paths or future tests

## Lanes

### Lane 1: Backend Command Contract

Type: behavior fix.

Target:

- replace whitespace-only backend command parsing with an explicit argv parser
  that supports quoted paths and arguments, or add a new argv-style environment
  contract while keeping the old string contract as a compatibility fallback

Preferred first pass:

- keep `GAMETERM_SCENE_COMPOSE_BACKEND` for compatibility
- add tests documenting quote behavior
- avoid shell expansion
- continue passing the prompt through environment and stdin

Acceptance:

```sh
cargo test -p gameterm-gui compose_backend_config --bin gameterm-gui
cargo test -p gameterm-gui run_configured_compose_backend --bin gameterm-gui
```

### Lane 2: Bounded Process Output

Type: behavior fix.

Target:

- replace the manual `try_wait()` loop with process collection that cannot
  deadlock on full stdout or stderr pipes
- keep the current timeout behavior
- keep stdout/stderr sanitization and clipping policy explicit
- preserve existing success/failure status strings unless a test proves they are
  ambiguous

Acceptance:

```sh
cargo test -p gameterm-gui compose_backend --bin gameterm-gui
```

Add focused tests for:

- backend emits large stdout
- backend emits large stderr
- timeout kills the backend
- nonzero exit renders failure dialogue

### Lane 3: Extract Compose Backend Module

Type: NFC refactor after lanes 1 and 2 are green.

Target shape:

```text
gameterm-gui/src/overlay/
  visual.rs
  visual_compose.rs
```

Candidate contents for `visual_compose.rs`:

- compose backend constants
- `ComposeBackendConfig`
- `CodexComposeConfig`
- `ComposeBackendRequest`
- `ComposeBackendResult`
- argv construction
- command/Codex process execution
- structured compose payload parsing, if it can move without exposing runtime
  internals

Keep in `visual.rs`:

- overlay event loop
- `SceneComposeDock`
- rendering calls
- runtime mutation orchestration

Acceptance:

```sh
cargo test -p gameterm-gui scene_compose --bin gameterm-gui
cargo test -p gameterm-gui codex_compose --bin gameterm-gui
```

Commit must be marked `NFC` and should not change behavior.

### Lane 4: Render-State Guard

Type: narrow behavior hardening.

Target:

- replace the VN panel texture render-state unwrap with an explicit fallback to
  rounded panel rendering when render state is unavailable
- do not change normal rendered output

Acceptance:

```sh
cargo test -p gameterm-gui visual_quad --bin gameterm-gui
cargo check -p gameterm-gui
```

### Lane 5: Verification And Smoke

Type: verification.

Run after lanes 1-4:

```sh
cargo test -p gameterm-visual
cargo test -p gameterm-gui scene_compose --bin gameterm-gui
cargo test -p gameterm-gui codex_compose --bin gameterm-gui
cargo test -p gameterm-gui visual_quad --bin gameterm-gui
cargo check -p gameterm-gui
ci/gameterm-scene-verify.sh --all
```

Run live smoke if any compose input, staged rendering, or smoke automation
changed:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario vn-compose \
  --output /tmp/gameterm-scene-compose-stabilization.png
```

## Done Means

- compose backend commands support quoted executable paths and arguments without
  invoking a shell
- verbose stdout/stderr cannot deadlock the backend runner
- timeout behavior remains deterministic and tested
- `visual.rs` no longer owns compose process execution details
- Scene schema and user-facing compose behavior remain compatible
- focused tests, GUI check, verifier, and required smoke are recorded

## First Implementation Slice

Do lanes 1 and 2 first in separate behavior commits:

1. `[gui] parse Scene compose backend argv safely`
2. `[gui] bound Scene compose backend output collection`

Only after those pass, do the behavior-preserving module split:

3. `[gui] NFC - move Scene compose backend helpers`

This order keeps real bug fixes reviewable before moving code.
