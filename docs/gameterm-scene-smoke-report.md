# GameTerm Scene Mode Smoke Report

This report records live and deterministic smoke results for the Scene Mode
next product layer. Live captures are intentionally separated from code fixes so
failures can be triaged without mixing report commits with behavior changes.

## Artifact Convention

Live smoke captures should use timestamped paths:

```text
~/Desktop/gameterm-scene-smoke-<scenario>-YYYYMMDD-HHMMSS.png
```

If Desktop capture is not appropriate, use a dedicated artifact directory and
record the absolute path in the scenario result.

## Environment

- Date recorded: 2026-05-30
- Host: macOS Darwin 25.5.0 arm64
- ffmpeg: `/opt/homebrew/bin/ffmpeg`, version 8.1.1
- GUI binary: `target/debug/gameterm-gui`

Updated live smoke automation pass:

- Date recorded: 2026-05-31
- Foreground automation: `osascript` activates the launched `gameterm-gui`
  process, sends `Ctrl+Shift+G`, and prints the frontmost process before
  capture.
- Sprite fixtures: copied smoke fixtures rewrite sprite paths to absolute repo
  asset paths before launch.

## Deterministic Checks

Command:

```sh
ci/gameterm-scene-smoke.sh --list-scenarios
ci/gameterm-scene-smoke.sh --describe-scenario guarded-input
ci/gameterm-scene-smoke.sh --describe-scenario process-state
ci/gameterm-scene-smoke.sh --check-assets
```

Result: PASS.

Validated named scenarios:

- `renderer-rows`
- `guarded-input`
- `run-command-targets`
- `overlay-cleanup`
- `patch-inbox`
- `mux-patch`
- `process-state`

## Live Results

### renderer-rows

Command:

```sh
ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario renderer-rows \
  --wait-before-capture 5 \
  --capture-timeout 12 \
  --output /Users/julianabeleda/Desktop/gameterm-scene-smoke-renderer-rows-20260530-223150.png
```

Result: PASS after foreground/open-scene automation.

Capture:

```text
/Users/julianabeleda/Desktop/gameterm-scene-smoke-renderer-rows-20260531-103019.png
```

Observation: capture shows the GameTerm Scene window with the renderer row
fixture. The smoke log reported `Frontmost process before capture:
gameterm-gui`.

### guarded-input

Command:

```sh
ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario guarded-input \
  --wait-before-capture 2 \
  --capture-timeout 12 \
  --output /Users/julianabeleda/Desktop/gameterm-scene-smoke-guarded-input-20260531-103430.png
```

Result: PASS for launch, foreground, Scene Mode open, sprite path resolution,
automated layer input, guarded transition, and capture.

Capture:

```text
/Users/julianabeleda/Desktop/gameterm-scene-smoke-guarded-input-20260531-111548.png
```

Observation: the default `space,enter` key sequence ran a layer-owned update
hook, then activated the guarded story transition. The capture shows
`Status: Layer story transitioned: dialogue -> choice`.

### run-command-targets

Command:

```sh
ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario run-command-targets \
  --wait-before-capture 2 \
  --capture-timeout 12 \
  --output /Users/julianabeleda/Desktop/gameterm-scene-smoke-run-command-targets-20260531-103151.png
```

Result: PASS for launch, foreground, Scene Mode open, automated RunCommand
input, route-pane split dispatch, and capture.

Capture:

```text
/Users/julianabeleda/Desktop/gameterm-scene-smoke-run-command-targets-20260531-112343.png
```

Observation: the default `enter,j,enter,j,enter` key sequence activated the
tab, split-right, and split-down choices. A crash discovered during the first
automated run was fixed by dispatching RunCommand work through the GUI window
event loop, then spawning the local future on the GUI thread. Split targets now
use the underlying route pane instead of the overlay pane.

### overlay-cleanup

Command:

```sh
ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario overlay-cleanup \
  --wait-before-capture 2 \
  --capture-timeout 12 \
  --output /Users/julianabeleda/Desktop/gameterm-scene-smoke-overlay-cleanup-20260531-111623.png
```

Result: PASS.

Capture:

```text
/Users/julianabeleda/Desktop/gameterm-scene-smoke-overlay-cleanup-20260531-111623.png
```

Observation: the default `escape` key sequence closed Scene Mode before
capture, returning to the underlying shell without crashing the GUI.

### patch-inbox

Command:

```sh
ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario patch-inbox \
  --wait-before-capture 2 \
  --capture-timeout 12 \
  --output /Users/julianabeleda/Desktop/gameterm-scene-smoke-patch-inbox-20260531-103207.png
```

Result: PASS.

Capture:

```text
/Users/julianabeleda/Desktop/gameterm-scene-smoke-patch-inbox-20260531-103207.png
```

Observation: the smoke script wrote `patch-status.json` into the temporary
inbox before capture and kept Scene Mode open.

### mux-patch

Command:

```sh
ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario mux-patch \
  --wait-before-capture 2 \
  --capture-timeout 12 \
  --output /Users/julianabeleda/Desktop/gameterm-scene-smoke-mux-patch-20260531-110753.png
```

Result: PASS.

Capture:

```text
/Users/julianabeleda/Desktop/gameterm-scene-smoke-mux-patch-20260531-110753.png
```

Observation: foreground/open-scene automation succeeded, the harness discovered
the route pane through the unique GUI class, submitted the mux patch with an
explicit `--target-pane-id`, and the capture shows
`flags=loaded, verified` plus `Status: Fixture patch applied`.

### process-state

Command:

```sh
ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario process-state \
  --wait-before-capture 2 \
  --capture-timeout 12 \
  --output /Users/julianabeleda/Desktop/gameterm-scene-smoke-process-state-20260531-103332.png
```

Result: PASS.

Capture:

```text
/Users/julianabeleda/Desktop/gameterm-scene-smoke-process-state-20260531-103332.png
```

Observation: the smoke script wrote a typed process-state patch through the
temporary inbox before capture and kept Scene Mode open.

## Follow-Up

1. Keep deterministic smoke registry and asset checks in
   `ci/gameterm-scene-verify.sh --all`.
