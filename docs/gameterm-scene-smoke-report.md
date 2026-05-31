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

Result: FAIL.

Capture:

```text
/Users/julianabeleda/Desktop/gameterm-scene-smoke-renderer-rows-20260530-223150.png
```

Failure class: launch/focus/input automation.

Observation: ffmpeg captured a valid 1920x1080 PNG, but the capture shows the
browser/workspace rather than the GameTerm Scene window. The automated
AppleScript shortcut path did not foreground GameTerm Scene for capture. This
does not indicate a Scene Mode runtime failure; it means the live smoke harness
still needs a reliable foreground/open-scene step or a manual run.

### guarded-input

Result: NOT RUN LIVE.

Reason: blocked by the same foreground/open-scene issue observed in
`renderer-rows`.

### run-command-targets

Result: NOT RUN LIVE.

Reason: blocked by the same foreground/open-scene issue observed in
`renderer-rows`.

### patch-inbox

Result: NOT RUN LIVE.

Reason: blocked by the same foreground/open-scene issue observed in
`renderer-rows`.

### mux-patch

Result: NOT RUN LIVE.

Reason: blocked by the same foreground/open-scene issue observed in
`renderer-rows`.

### process-state

Result: NOT RUN LIVE.

Reason: blocked by the same foreground/open-scene issue observed in
`renderer-rows`.

## Follow-Up

1. Add a reliable smoke foreground/open-scene mechanism for macOS.
2. Rerun all six named scenarios after the harness can reliably foreground the
   GameTerm Scene window.
3. Keep deterministic smoke registry and asset checks in
   `ci/gameterm-scene-verify.sh --all`.
