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

Updated dogfood workspace pass:

- Date recorded: 2026-06-02
- Foreground automation: `osascript` foregrounded the launched
  `gameterm-gui` process before capture.
- Dogfood fixture: generated from
  `/Users/julianabeleda/env/gameterm` through
  `ci/gameterm-scene-workspace.sh dogfood`.

## Deterministic Checks

Command:

```sh
ci/gameterm-scene-smoke.sh --list-scenarios
ci/gameterm-scene-smoke.sh --describe-scenario guarded-input
ci/gameterm-scene-smoke.sh --describe-scenario process-state
ci/gameterm-scene-smoke.sh --describe-scenario dogfood
ci/gameterm-scene-smoke.sh --check-assets
```

Result: PASS.

Validated named scenarios:

- `renderer-rows`
- `guarded-input`
- `run-command-targets`
- `overlay-cleanup`
- `vertical-slice`
- `workspace-agent`
- `workspace-discovery`
- `dogfood`
- `agent-lifecycle`
- `authoring-loop`
- `patch-inbox`
- `mux-patch`
- `process-state`

## Live Results

### vn-compose-codex-fake-pass-20260603-0055

Command:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-smoke.sh --launch \
  --scenario vn-compose-codex \
  --output /tmp/gameterm-scene-vn-compose-codex-fake.png \
  --wait-before-capture 3 \
  --post-action-wait 2
```

Result: PASS.

Capture:

```text
/tmp/gameterm-scene-vn-compose-codex-fake.png
```

Observation: the smoke helper launched the VN compose Codex scenario with the
deterministic fake-Codex command, submitted `look at roadmap`, and captured a
non-empty 1920x1080 PNG. Visual inspection confirmed the dialogue panel renders
the submitted user prompt as a highlighted `>` line and the fake Codex reply as
the assistant dialogue, with the composer dock cleared and ready for the next
prompt.

### vn-compose-real-codex-pass-20260603-0058

Command:

```sh
GAMETERM_SCENE_COMPOSE_BACKEND_KIND=codex \
GAMETERM_SCENE_COMPOSE_CODEX_BIN=/opt/homebrew/bin/codex \
GAMETERM_SCENE_COMPOSE_WORKSPACE=/Users/julianabeleda/env/gameterm \
GAMETERM_SCENE_COMPOSE_CODEX_SANDBOX=read-only \
GAMETERM_SCENE_COMPOSE_CODEX_APPROVAL=never \
GAMETERM_SCENE_COMPOSE_CODEX_TIMEOUT_SECONDS=90 \
ci/gameterm-scene-smoke.sh --launch \
  --scenario vn-compose \
  --key-sequence 'text:say hi,enter,delay:8' \
  --wait-before-capture 3 \
  --post-action-wait 2 \
  --capture-timeout 12 \
  --output /tmp/gameterm-scene-vn-real-codex-diagnostic.png
```

Result: PASS.

Capture:

```text
/tmp/gameterm-scene-vn-real-codex-diagnostic.png
```

Observation: the smoke helper launched Scene Mode with the real local Codex CLI
backend selected by environment, submitted `say hi`, and captured a non-empty
1920x1080 PNG. Visual inspection confirmed the dialogue panel shows `> say hi`
followed by the Codex reply `Hi!`, and the composer dock remains ready for the
next prompt. This differs from the deterministic built-in reply format
(`Built-in compose reply: ...` in current builds), so the capture validates the
real Codex backend path.

Earlier direct probe note: before this smoke, a standalone `codex exec
--output-last-message ... 'Reply exactly: hi'` probe from
`/Users/julianabeleda/env/gameterm` returned `429 Too Many Requests` and
websocket `403 Forbidden`. Treat that as transient account/session state unless
it reappears during live Scene use.

### vn-compose-shared-layout-pass-20260602-1822

Command:

```sh
cargo build -p gameterm-gui
GAMETERM_SCENE_COMPOSE_BACKEND_KIND=fake \
GAMETERM_SCENE_COMPOSE_FAKE_REPLY='Hi.' \
ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario vn-compose \
  --vn-asset-source-root .cache/gameterm-scene/vn-assets \
  --key-sequence 'text:say hi,enter,delay:4' \
  --wait-before-capture 3 \
  --post-action-wait 2 \
  --capture-timeout 12 \
  --output /Users/julianabeleda/Desktop/gameterm-scene-smoke-vn-shared-layout-20260602-182247.png
```

Result: PASS.

Capture:

```text
/Users/julianabeleda/Desktop/gameterm-scene-smoke-vn-shared-layout-20260602-182247.png
```

Observation: the smoke helper launched the VN compose scenario, sent a
deterministic `say hi` prompt through the fake compose backend, and captured a
non-empty 1920x1080 PNG. Visual inspection confirmed the shared VN layout fix:
`Codex` and `Composer` nameplates/text were inside their respective transparent
rounded panels. Caveat: macOS left GameTerm in a smaller foreground window
rather than true fullscreen, so repeat a maximized/fullscreen capture before
using this as final visual sign-off.

### dogfood-workspace-pass-20260602-1713

Command:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-smoke.sh --launch --scenario dogfood \
  --wait-before-capture 3 \
  --capture-timeout 12 \
  --output /Users/julianabeleda/Desktop/gameterm-scene-smoke-dogfood-20260602-171327.png
```

Result: PASS.

Capture:

```text
/Users/julianabeleda/Desktop/gameterm-scene-smoke-dogfood-20260602-171327.png
```

Observation: the smoke helper generated the dogfood workspace scene from
`/Users/julianabeleda/env/gameterm`, installed it into an isolated temporary
config, launched `gameterm-gui`, foregrounded it, opened Scene Mode through the
native smoke hook, and captured a non-empty 1920x1080 PNG. Visual inspection
confirmed the dogfood workspace, roadmap/onboarding/smoke/refactor docs, task
brief, `Run verification`, `Run git status`, and `Run dogfood smoke` choices are
visible.

### stabilization-refactor-lane4-lane5-smoke-pass-20260602-0132

Command:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-smoke.sh --launch --scenario workspace-discovery --output /tmp/gameterm-scene-smoke-workspace-discovery-lane5.png --allow-background-capture --no-fullscreen-window --wait-before-capture 2 --post-action-wait 1 --capture-timeout 2
```

Result: PASS.

Capture:

```text
/tmp/gameterm-scene-smoke-workspace-discovery-lane5.png
```

Observation: the smoke helper installed a `workspace-discovery` scene into
temporary config, launched `gameterm-gui`, foregrounded it, opened Scene Mode with
the configured shortcut, and captured a non-empty 1920x1080 PNG.

### active-pane-workflow-pass-20260531-2140

Command:

```sh
cargo build -p gameterm -p gameterm-gui
tmp_home="$(mktemp -d /tmp/gameterm-active-pane-live.XXXXXX)"
ci/gameterm-scene-mux-context.sh discover \
  --gameterm-bin target/debug/gameterm \
  --install \
  --config-home "${tmp_home}" \
  --force
ci/gameterm-scene-author.sh validate "${tmp_home}/gameterm/scenes/default.json"

ci/gameterm-scene-smoke.sh --launch --scenario live-mux-discovery \
  --wait-before-capture 2 \
  --capture-timeout 12 \
  --output /Users/julianabeleda/Desktop/gameterm-scene-smoke-active-pane-workflow-20260531-214051.png
```

Result: PASS.

Capture:

```text
/Users/julianabeleda/Desktop/gameterm-scene-smoke-active-pane-workflow-20260531-214051.png
```

Artifacts:

```text
/tmp/gameterm-active-pane-installed-default.json
/tmp/gameterm-active-pane-live-context.json
```

Observation: the active-pane install workflow wrote a validated default scene to
a temporary config home, leaving user config untouched. The live context came
from `target/debug/gameterm cli --no-auto-start list --format json` through the
mux-context helper: `source=gameterm-cli`, `pane_id=0`, `mux_window_id=0`, and
`pane_cwd=/Users/julianabeleda`. The installed scene recorded
`pane_context=provided`, `discovery_source=pane_cwd`, `active_pane_id=0`, and
`active_mux_window_id=0`. The named smoke scenario then launched and captured a
non-empty 1920x1080 PNG.

### live-mux-discovery-scenario-pass-20260531-2130

Command:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-smoke.sh --launch --scenario live-mux-discovery \
  --wait-before-capture 2 \
  --capture-timeout 12 \
  --output /Users/julianabeleda/Desktop/gameterm-scene-smoke-live-mux-discovery-20260531-213010.png
```

Result: PASS.

Capture:

```text
/Users/julianabeleda/Desktop/gameterm-scene-smoke-live-mux-discovery-20260531-213010.png
```

Observation: the named smoke scenario generated a Scene Mode scene through
`ci/gameterm-scene-mux-context.sh discover --allow-missing`, launched the
generated scene, foregrounded `gameterm-gui`, opened Scene Mode, and captured a
non-empty 1920x1080 PNG. The helper used the repo-local
`target/debug/gameterm` CLI and collected real live mux context:
`source=gameterm-cli`, `pane_id=0`, `mux_window_id=0`, and
`pane_cwd=/Users/julianabeleda`.

### live-mux-discovery-pass-20260601

Command:

```sh
ci/gameterm-scene-mux-context.sh collect --allow-missing \
  >/tmp/gameterm-live-mux-context.json
ci/gameterm-scene-mux-context.sh discover --allow-missing \
  --scene-output /tmp/gameterm-live-mux-workspace.json \
  --force
ci/gameterm-scene-author.sh validate /tmp/gameterm-live-mux-workspace.json
```

Result: PASS.

Artifacts:

```text
/tmp/gameterm-live-mux-context.json
/tmp/gameterm-live-mux-workspace.json
```

Observation: the helper queried the running mux through
`gameterm cli --no-auto-start list --format json`, selected active pane `0` in
mux window `0`, normalized hosted cwd URL `file://.../Users/julianabeleda` to
`/Users/julianabeleda`, generated a Scene Mode workspace from that active pane
cwd, and validated the scene. The generated scene recorded
`pane_context=provided`, `discovery_source=pane_cwd`, `active_pane_id=0`, and
`active_mux_window_id=0`.

### stabilization-refactor-workspace-discovery-20260531-1835

Command:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-smoke.sh --launch --scenario workspace-discovery \
  --output /Users/julianabeleda/Desktop/gameterm-scene-smoke-workspace-discovery-20260531-183532.png
```

Result: PASS after rebuilding the GUI binary.

Capture:

```text
/Users/julianabeleda/Desktop/gameterm-scene-smoke-workspace-discovery-20260531-183532.png
```

Observation: the stabilization refactor smoke used the documented onboarding
scenario after helper/runtime NFC cleanup. The smoke helper generated a scene
from `/Users/julianabeleda/env/gameterm`, launched an isolated
`gameterm-gui`, foregrounded the process, opened Scene Mode, and captured a
non-empty 1920x1080 PNG. The helper reported the launched `gameterm-gui`
process as frontmost before capture.

### workspace-agent-live-pass-20260531-1517

Command:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-smoke.sh --launch --scenario workspace-agent \
  --output /Users/julianabeleda/Desktop/gameterm-scene-smoke-workspace-agent-20260531-151646.png
```

Result: PASS after rebuilding the GUI binary.

Capture:

```text
/Users/julianabeleda/Desktop/gameterm-scene-smoke-workspace-agent-20260531-151646.png
```

Observation: the first live attempt used a stale `target/debug/gameterm-gui`
binary and showed a scene load error for the newer `SetEntityFlags` operation.
After rebuilding, Scene Mode loaded the `workspace-agent` fixture, the smoke
script emitted a real process patch plus planning/running/blocked/complete
agent lifecycle patches, and the capture showed the selected `Scene Agent`
entity with `agent_completed` state. The final status line is below the visible
area in the captured small window, but the entity state and unlocked/locked
choice state confirm that patch delivery reached the scene.

### workspace-discovery-live-pass-20260531-1652

Command:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario workspace-discovery \
  --output /Users/julianabeleda/Desktop/gameterm-scene-smoke-workspace-discovery-20260531-165200.png
```

Result: PASS.

Capture:

```text
/Users/julianabeleda/Desktop/gameterm-scene-smoke-workspace-discovery-20260531-165200.png
```

Observation: the smoke helper generated a Scene Mode scene from
`/Users/julianabeleda/env/gameterm`, launched GameTerm with that generated
scene, foregrounded `gameterm-gui`, and captured a non-empty 1920x1080 PNG. The
capture shows the generated `gameterm Workspace` project entity, repo path
state, and file choices for discovered docs such as `README.md` and the Scene
Mode roadmap/scope files.

### first-pass-live-pass-20260531-1259

Command:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario vertical-slice \
  --output /tmp/gameterm-scene-smoke-vertical-slice.png

for scenario in guarded-input run-command-targets overlay-cleanup authoring-loop \
  agent-lifecycle patch-inbox mux-patch process-state; do
  ci/gameterm-scene-smoke.sh --launch --scenario "${scenario}" \
    --output "/tmp/gameterm-scene-smoke-${scenario}.png"
done

ci/gameterm-scene-smoke.sh --launch --scenario renderer-rows \
  --output /tmp/gameterm-scene-smoke-renderer-rows.png
```

Result: PASS for all ten named live scenarios after the first-pass completion
work.

Captures:

```text
/tmp/gameterm-scene-smoke-vertical-slice.png
/tmp/gameterm-scene-smoke-guarded-input.png
/tmp/gameterm-scene-smoke-run-command-targets.png
/tmp/gameterm-scene-smoke-overlay-cleanup.png
/tmp/gameterm-scene-smoke-authoring-loop.png
/tmp/gameterm-scene-smoke-agent-lifecycle.png
/tmp/gameterm-scene-smoke-patch-inbox.png
/tmp/gameterm-scene-smoke-mux-patch.png
/tmp/gameterm-scene-smoke-process-state.png
/tmp/gameterm-scene-smoke-renderer-rows.png
```

Observation: every scenario reported `gameterm-gui` as the frontmost process
before capture and produced a non-empty 1920x1080 PNG. The mux-patch scenario
discovered target pane `0` and submitted through the CLI mux path. The
agent-lifecycle, patch-inbox, and process-state scenarios wrote structured
patches before capture. The smoke helper still logs AVFoundation device-listing
error code 251 before capture; capture itself succeeds.

### full-suite-pass-20260531

Command:

```sh
for scenario in renderer-rows guarded-input run-command-targets overlay-cleanup \
  vertical-slice agent-lifecycle authoring-loop patch-inbox mux-patch \
  process-state; do
  stamp=$(date +%Y%m%d-%H%M%S)
  out="/Users/julianabeleda/Desktop/gameterm-scene-smoke-${scenario}-${stamp}.png"
  ci/gameterm-scene-smoke.sh \
    --launch \
    --scenario "${scenario}" \
    --wait-before-capture 2 \
    --capture-timeout 12 \
    --output "${out}"
done
```

Result: PASS for all ten named live scenarios, including the authoring-loop
scenario added after the previous full-suite run.

Captures:

```text
/Users/julianabeleda/Desktop/gameterm-scene-smoke-renderer-rows-20260531-120344.png
/Users/julianabeleda/Desktop/gameterm-scene-smoke-guarded-input-20260531-120344.png
/Users/julianabeleda/Desktop/gameterm-scene-smoke-run-command-targets-20260531-120344.png
/Users/julianabeleda/Desktop/gameterm-scene-smoke-overlay-cleanup-20260531-120344.png
/Users/julianabeleda/Desktop/gameterm-scene-smoke-vertical-slice-20260531-120344.png
/Users/julianabeleda/Desktop/gameterm-scene-smoke-agent-lifecycle-20260531-120344.png
/Users/julianabeleda/Desktop/gameterm-scene-smoke-authoring-loop-20260531-120344.png
/Users/julianabeleda/Desktop/gameterm-scene-smoke-patch-inbox-20260531-120344.png
/Users/julianabeleda/Desktop/gameterm-scene-smoke-mux-patch-20260531-120344.png
/Users/julianabeleda/Desktop/gameterm-scene-smoke-process-state-20260531-120344.png
```

Observation: every scenario reported the launched `gameterm-gui` process as
frontmost before capture and produced a non-empty PNG. The mux-patch scenario
discovered target pane `0` through the unique GUI class and submitted
`patch-status.json` through the mux path. The process-state and
agent-lifecycle scenarios wrote structured patches through auto-created inbox
files before capture.

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

### vertical-slice

Command:

```sh
ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario vertical-slice \
  --wait-before-capture 2 \
  --capture-timeout 12 \
  --output /Users/julianabeleda/Desktop/gameterm-scene-smoke-vertical-slice-20260531-112852.png
```

Result: PASS.

Capture:

```text
/Users/julianabeleda/Desktop/gameterm-scene-smoke-vertical-slice-20260531-112852.png
```

Observation: the default `enter,j,enter,j,enter,j,enter` key sequence drove the
playable loop through brief acceptance, launch-kit preparation, loop
completion, and ending dialogue. The capture shows
`brief_accepted=true`, `launch_ready=true`, `agent_phase=complete`, and the
`Read ending` choice selected.

### agent-lifecycle

Command:

```sh
ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario agent-lifecycle \
  --wait-before-capture 2 \
  --capture-timeout 12 \
  --output /Users/julianabeleda/Desktop/gameterm-scene-smoke-agent-lifecycle-20260531-113121.png
```

Result: PASS.

Capture:

```text
/Users/julianabeleda/Desktop/gameterm-scene-smoke-agent-lifecycle-20260531-113121.png
```

Observation: the scenario emits `planning`, `blocked`, and `complete` patches
through the auto-created inbox. The capture shows `flags=agent,
agent_complete` on the selected entity and `Status: Agent complete: Finished
visual slice`.

### authoring-loop

Command:

```sh
ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario authoring-loop \
  --wait-before-capture 2 \
  --capture-timeout 12 \
  --output /Users/julianabeleda/Desktop/gameterm-scene-smoke-authoring-loop-20260531-115810.png
```

Result: PASS.

Capture:

```text
/Users/julianabeleda/Desktop/gameterm-scene-smoke-authoring-loop-20260531-115810.png
```

Observation: the default `enter,j,enter,j,enter` key sequence exported the
story state, mutated draft state, then imported the saved story state again.
The capture shows `draft_dirty=false`, selected `Reload saved story`, and
`Status: Story state imported: /tmp/gameterm-scene-authoring-loop.story.json`.

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

### active-pane-gui

Command:

```sh
ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario active-pane-gui \
  --wait-before-capture 3 \
  --capture-timeout 8 \
  --output /tmp/gameterm-scene-active-pane-gui.png
```

Result: PASS.

Capture:

```text
/tmp/gameterm-scene-active-pane-gui.png
```

Observation: macOS automation foregrounded the launched GameTerm process and
sent `CTRL|ALT|SHIFT+g`. The capture shows the native active-pane generated
Scene overlay open with source/status text for the generated workspace scene.
No configured `default.json` scene file was written by the transient GUI path.

### vn-demo

Command:

```sh
ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario vn-demo \
  --wait-before-capture 3 \
  --capture-timeout 8 \
  --output /tmp/gameterm-scene-vn-demo-fullscreen.png
```

Result: PASS.

Capture:

```text
/tmp/gameterm-scene-vn-demo-fullscreen.png
```

Observation: the smoke harness generated the VN demo into a temporary Scene
config, strict-validated sprite files as PNG image data, resized the launched
GameTerm window to fill the visible desktop, opened Scene Mode, sent the
scripted VN key sequence, and captured the imported VN scene. The capture shows
`VN Script Demo` with `sprite=vn.character.kiki.neutral`.

### vn-demo downloaded PSD

Command:

```sh
ci/gameterm-scene-vn-image-export.sh \
  --source .cache/gameterm-scene/vn-assets/raw/visual_novel_asset/'visual novel asset.psd' \
  --output-source-root .cache/gameterm-scene/vn-assets \
  --force

ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario vn-demo \
  --vn-asset-source-root .cache/gameterm-scene/vn-assets \
  --wait-before-capture 3 \
  --capture-timeout 8 \
  --output /tmp/gameterm-scene-vn-demo-downloaded-psd-fullscreen.png
```

Result: PASS.

Capture:

```text
/tmp/gameterm-scene-vn-demo-downloaded-psd-fullscreen.png
```

Observation: the local downloaded PSD was flattened through the VN image export
helper into the existing source-root layout, then the VN smoke path generated a
strict-validated demo with local school backgrounds and captured the fullscreen
terminal render. The downloaded source files remain under ignored `.cache`
paths and are not committed.

### vn-compose

Command:

```sh
ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario vn-compose \
  --vn-asset-source-root .cache/gameterm-scene/vn-assets \
  --wait-before-capture 3 \
  --capture-timeout 8 \
  --output /tmp/gameterm-scene-vn-compose-fullscreen-final.png
```

Result: PASS.

Capture:

```text
/tmp/gameterm-scene-vn-compose-fullscreen-final.png
```

Observation: the smoke harness generated the VN demo, opened Scene Mode,
typed `look at roadmap` into the bottom compose dock, submitted it, waited for
the deterministic local compose backend, and captured the reply rendered as
`Codex` dialogue. The capture also shows the compose dock still mounted with
the last prompt while the staged classroom background and character sprite
remain visible.

### vn-compose-codex

Command:

```sh
ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario vn-compose-codex \
  --vn-asset-source-root .cache/gameterm-scene/vn-assets \
  --wait-before-capture 3 \
  --capture-timeout 8 \
  --output /tmp/gameterm-scene-vn-compose-codex-fullscreen.png
```

Result: PASS.

Capture:

```text
/tmp/gameterm-scene-vn-compose-codex-fullscreen.png
```

Observation: the smoke harness generated the VN demo, configured
`GAMETERM_SCENE_COMPOSE_BACKEND_KIND=codex`, pointed Scene Mode at a temporary
fake Codex CLI helper, typed `look at roadmap`, and captured the reply rendered
as `Codex` dialogue. The capture shows `Status: Codex succeeded`, the staged
classroom background, the character sprite, and the compose dock still mounted
with the last prompt.

### vn-compose-native-open

Command:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-smoke.sh --launch --scenario vn-compose \
  --output /tmp/gameterm-scene-smoke-vn-compose-native.png
```

Result: PASS.

Capture:

```text
/tmp/gameterm-scene-smoke-vn-compose-native.png
```

Smoke report:

```text
scenario: vn-compose
fixture: vn-demo
GameTerm class: org.gameterm.scene-smoke.22762
native Scene open: 1
output: /tmp/gameterm-scene-smoke-vn-compose-native.png
bytes: 282894
```

Observation: the smoke harness opened Scene Mode through the native smoke hook
instead of the keyboard shortcut path, foregrounded the launched GameTerm
window, typed `look at roadmap` into the compose dock, and captured a
`1920x1080` PNG. This validates the current GUI smoke path for Scene changes
without depending on shortcut automation to enter Scene Mode.

### vn-demo-stage

Command:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-smoke.sh --launch --scenario vn-demo \
  --wait-before-capture 3 \
  --capture-timeout 8 \
  --output /tmp/gameterm-scene-vn-demo-stage.png
```

Result: PASS.

Capture:

```text
/tmp/gameterm-scene-vn-demo-stage.png
```

Smoke report:

```text
scenario: vn-demo
fixture: vn-demo
GameTerm class: org.gameterm.scene-smoke.24842
native Scene open: 1
output: /tmp/gameterm-scene-vn-demo-stage.png
bytes: 284194
```

Observation: the smoke harness opened the staged VN demo through the native
Scene smoke hook, sent the deterministic `enter,j,enter` sequence, and captured
a `1920x1080` PNG. This records the first-pass VN presentation evidence for
large background/character staging with the transparent dialogue and compose
panels visible.

## Follow-Up

1. Keep deterministic smoke registry and asset checks in
   `ci/gameterm-scene-verify.sh --all`.

## Aspect-Safe Scene Images

### vn-compose-fullscreen-aspect

Command:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-smoke.sh --scenario vn-compose --launch \
  --wait-before-capture 3 \
  --post-action-wait 2 \
  --capture-timeout 12 \
  --output /tmp/gameterm-scene-aspect-vn-fullscreen.png
```

Result: PASS.

Capture:

```text
/tmp/gameterm-scene-aspect-vn-fullscreen.png
```

Smoke report:

```text
scenario: vn-compose
fixture: vn-demo
window size: fullscreen
native Scene open: 1
output: /tmp/gameterm-scene-aspect-vn-fullscreen.png
bytes: 268285
```

Observation: the smoke harness opened Scene Mode through the native smoke hook,
typed `look at roadmap` into the compose dock, submitted it, and captured a
`1920x1080` PNG. The staged character remains proportionally centered and the
background fills the stage without visible squashing.

### vn-compose-windowed-aspect

Command:

```sh
ci/gameterm-scene-smoke.sh --scenario vn-compose --launch \
  --window-size 1000x560 \
  --wait-before-capture 3 \
  --post-action-wait 2 \
  --capture-timeout 12 \
  --output /tmp/gameterm-scene-aspect-vn-windowed.png
```

Result: PASS.

Capture:

```text
/tmp/gameterm-scene-aspect-vn-windowed.png
```

Smoke report:

```text
scenario: vn-compose
fixture: vn-demo
window size: 1000x560
native Scene open: 1
output: /tmp/gameterm-scene-aspect-vn-windowed.png
bytes: 2008168
```

Observation: the compact windowed capture keeps the staged character's
proportions consistent with the fullscreen capture. The dialogue and Composer
panels remain in the VN overlay path and are not affected by image aspect
scaling.
