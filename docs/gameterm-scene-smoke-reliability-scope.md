# GameTerm Scene Smoke Reliability Scope

This document scopes the next Scene Mode verification lane: make live smoke
checks reliable enough to prove GUI/render changes without depending on fragile
macOS focus automation as the only path into Scene Mode.

It follows the compose stabilization pass where focused Rust tests and
`ci/gameterm-scene-verify.sh --all` passed, but the live VN compose smoke failed
while AppleScript tried to foreground GameTerm and send `Ctrl+Shift+G`.

## Problem

`ci/gameterm-scene-smoke.sh --launch` currently launches
`target/debug/gameterm-gui`, installs a temporary Scene Mode fixture, and then
uses macOS `System Events` automation to:

1. find the launched GUI process by pid,
2. make it frontmost,
3. optionally resize its window,
4. send the Scene Mode shortcut,
5. later assert the process is still frontmost before `ffmpeg` capture.

That path catches missed-focus captures, but it also means a valid code change
can remain visually unproved when Accessibility permission, focus timing, or
GUI process visibility fails. The failure mode is outside Scene Mode's runtime
and renderer, yet it blocks confidence in GUI/render work.

## Goal

Make Scene Mode live smoke deterministic enough that a developer can prove a
GUI/render change with one command on a properly configured macOS host, and get
clear, early diagnostics when host permissions prevent automation.

The preferred end state is:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-smoke.sh --launch --scenario vn-compose
```

opens Scene Mode, captures the launched GameTerm window, and fails only for
actionable reasons:

- stale binary,
- missing Screen Recording permission,
- missing Accessibility permission,
- GameTerm did not start or expose the expected GUI process,
- Scene Mode did not open,
- capture did not produce an image,
- captured image is too small or likely wrong.

## Non-Goals

- No renderer rewrite.
- No Scene Mode schema changes.
- No broad macOS app lifecycle refactor.
- No CI requirement for a GUI session.
- No replacement of `ffmpeg` AVFoundation capture in this lane.
- No attempt to bypass macOS privacy controls.
- No terminal scrollback parsing to infer visual correctness.

## Current Foundation

Already available in `ci/gameterm-scene-smoke.sh`:

- named scenarios, including VN and compose scenarios
- stale GUI binary guard
- temporary `XDG_CONFIG_HOME` launch
- fixture install/setup
- optional patch inbox and mux patch submission
- `--no-auto-open-scene`
- `--allow-background-capture`
- `--focus-timeout`
- `--key-sequence`
- frontmost process assertion
- capture size guard
- explicit Screen Recording failure text when `ffmpeg` times out

Existing docs already explain manual fallback and macOS permission needs in
[GameTerm Scene Mode](gameterm-scene-mode.md).

## Proposed Strategy

Keep the smoke harness additive and narrow. Do not turn it into a separate GUI
test framework.

### Lane 1: Scope

Deliverables:

- this scope document

Verification:

```sh
git diff --check -- docs/gameterm-scene-smoke-reliability-scope.md
```

Commit:

```text
[docs] scope Scene smoke reliability
```

### Lane 2: Automation Preflight

Add a deterministic preflight before launching GameTerm when `--launch` and
auto-open are enabled.

Deliverables:

- a small `check_macos_automation_preflight` helper
- verifies `osascript` is available on macOS
- verifies `System Events` can be queried before launching
- prints the current frontmost process when available
- fails with a dedicated exit code and the Accessibility fix text when
  `System Events` is unavailable or times out
- keeps non-macOS behavior unchanged: print that auto-open is unsupported and
  require manual opening unless a later lane provides a native open path

Verification:

```sh
bash -n ci/gameterm-scene-smoke.sh
ci/gameterm-scene-smoke.sh --list-scenarios
ci/gameterm-scene-smoke.sh --describe-scenario vn-compose
ci/gameterm-scene-smoke.sh --check-assets
```

Commit:

```text
[test] preflight Scene smoke automation
```

### Lane 3: Native Scene Open Path

Avoid using a keyboard shortcut as the only way to enter Scene Mode during
smoke launches.

Preferred approach:

- add an explicit smoke-only launch signal that the GUI can consume on startup,
  for example an environment variable such as
  `GAMETERM_SCENE_SMOKE_OPEN=scene` or `active-pane`
- after the first window/pane is available, the GUI opens the same overlay path
  used by `ShowGameTermScene` or `ShowGameTermActivePaneScene`
- keep normal user startup behavior unchanged when the variable is absent
- make the smoke script opt into this path by default for `--launch`
- retain AppleScript focus/resize only for frontmost capture and optional key
  sequences

Fallback approach if the native startup hook is too invasive:

- add `--open-scene-mode-on-launch` to the GUI binary or startup command path,
  scoped to the existing `start` flow
- use the same internal action handlers as the key assignments

Deliverables:

- one narrow GUI launch/open hook
- smoke script option and default behavior for launch scenarios
- docs update in `docs/gameterm-scene-mode.md`
- tests for parsing/configuring the new launch signal where practical

Verification:

```sh
cargo check -p gameterm-gui
cargo test -p gameterm-gui active_pane_scene --bin gameterm-gui
ci/gameterm-scene-verify.sh --all
```

Live verification:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-smoke.sh --launch --scenario vn-compose \
  --output /tmp/gameterm-scene-smoke-vn-compose.png
```

Commit:

```text
[gui] add Scene smoke launch hook
```

### Lane 4: Capture Diagnostics

Make wrong-host and wrong-window captures easier to diagnose.

Deliverables:

- print launched GameTerm pid and class, frontmost process, output path, and
  active scenario in a compact final report
- when frontmost assertion fails, include the exact command to rerun with
  `--no-auto-open-scene` for manual fallback
- when capture succeeds, report file size and `file` metadata
- preserve existing `--allow-background-capture` behavior

Verification:

```sh
bash -n ci/gameterm-scene-smoke.sh
ci/gameterm-scene-smoke.sh --check-assets
ci/gameterm-scene-verify.sh --all
```

Commit:

```text
[test] improve Scene smoke diagnostics
```

### Lane 5: Product Smoke Report Update

After the live command succeeds, record the result.

Deliverables:

- update `docs/gameterm-scene-smoke-report.md` with:
  - command
  - scenario
  - output path
  - GameTerm pid/class
  - result
  - any host permission caveat

Verification:

```sh
git diff --check -- docs/gameterm-scene-smoke-report.md
```

Commit:

```text
[docs] record Scene smoke reliability pass
```

## Design Constraints

- Preserve upstream terminal behavior by default.
- Keep the new launch/open hook GameTerm-specific and opt-in.
- Route through the same overlay functions used by existing key assignments.
- Do not make Scene Mode silently perform AI compose or external commands.
- Keep smoke script options explicit and auditable.
- Keep focused deterministic checks separate from live GUI smoke.
- Keep AppleScript use limited to host focus, window sizing, and optional user
  interaction once a native Scene open path exists.

## Acceptance Criteria

This lane is complete when:

1. `ci/gameterm-scene-smoke.sh --launch --scenario vn-compose` no longer
   depends on keyboard-shortcut automation to open Scene Mode.
2. macOS Accessibility failure is detected before launching GameTerm or before
   waiting through the full smoke path.
3. stale binary, missing permissions, missing GUI process, wrong frontmost
   process, and capture timeout failures have distinct messages.
4. `--no-auto-open-scene` remains available as a manual fallback.
5. deterministic verification passes:

```sh
bash -n ci/gameterm-scene-smoke.sh
ci/gameterm-scene-smoke.sh --check-assets
ci/gameterm-scene-verify.sh --all
cargo check -p gameterm-gui
```

6. at least one live smoke capture succeeds on a configured macOS host and is
   recorded in the smoke report.

## Risks

### Risk: Startup Hook Changes Normal Launch Behavior

Mitigation: require an explicit environment variable or CLI option and keep the
default path unchanged.

### Risk: Scene Opens Before The First Pane Exists

Mitigation: route through the same active-pane/default-scene logic already used
by `ShowGameTermScene`; if no pane/window is ready, report a smoke-only status
or retry briefly with a bounded timeout.

### Risk: Smoke Still Requires Accessibility For Capture Focus

Mitigation: native Scene open removes shortcut dependency, but macOS focus and
screen capture still need host permissions. Preflight and diagnostics should
make that explicit and early.

### Risk: Script Becomes Too Broad

Mitigation: keep shell changes in small helpers and do not duplicate GUI
runtime logic in Bash.

## Open Questions

1. Should the native open path be an environment variable, a `start` option, or
   both?
2. Should default `--launch` use the native open path automatically, with a
   separate `--open-scene-via-shortcut` option for testing key bindings?
3. Should active-pane Scene smoke be supported by the same launch signal in
   this lane or deferred until default Scene smoke is reliable?
4. Should successful captures include a lightweight image sanity check beyond
   byte size, such as dimensions or PNG metadata?
