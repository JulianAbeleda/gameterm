# GameTerm Scene Overlay `visual.rs` Refactor Scope

Date: 2026-06-06

This is the tracked repo-facing scope for the next behavior-preserving pass on
`gameterm-gui/src/overlay/visual.rs`.

The local working note is also saved at
`structure/Development/visual-overlay-refactor-scope.md`; `structure/` is
ignored by this checkout, so this document is the committed source for review.

## Principle Fit

This pass follows `structure/Development/coding-principles.md`:

- preserve upstream behavior unless a task explicitly changes it
- keep GameTerm-specific changes narrow and reviewable
- prefer existing module boundaries before new crates
- do not mix NFC refactors with behavior changes
- use small commits with subsystem prefixes

Every implementation commit in this pass should be behavior-preserving and use:

```text
[gui] NFC - ...
```

Use `[test] NFC` only for verification-only test reshaping.

## Current State

`gameterm-gui/src/overlay/visual.rs` is currently 2,456 lines.

Already extracted from this file:

- compose dock editing: `visual_compose_dock.rs`
- dialogue scrollback: `visual_dialogue_scroll.rs`
- scene file/source loading: `visual_scene_files.rs`
- scene patch plumbing: `visual_scene_patches.rs`
- mux command dispatch: `visual_command_dispatch.rs`
- compose result handling: `visual_compose_result.rs`
- Kiki idle animation helpers: `visual_kiki_idle.rs`
- voice debug menu: `visual_voice_debug.rs`

Remaining responsibilities:

- overlay boot and session setup
- main event loop
- command/compose/TTS/STT channel drains
- keyboard/mouse/resize dispatch
- TTS/STT key policy and result application
- runtime render orchestration
- frame-line utilities
- local integration tests for moved overlay behavior

The file is no longer the whole Scene overlay implementation, but it is still
doing too much coordination work.

## Goal

Make `visual.rs` the entrypoint and top-level coordinator only.

Target module shape:

```text
gameterm-gui/src/overlay/
  visual.rs                         # entrypoints and loop orchestration
  visual_input_keys.rs              # key mapping and modifier policies
  visual_frame.rs                   # terminal frame line utilities
  visual_render.rs                  # runtime render orchestration
  visual_voice_events.rs            # TTS/STT result and key handling
  visual_event_drain.rs             # channel drain helpers
  visual_overlay_session.rs         # session state struct, if it simplifies
```

## Non-Goals

- No Scene Mode behavior changes.
- No keybinding, config, voice, Codex, or render behavior changes.
- No VN UI redesign.
- No new crate.
- No mux removal.
- No TTS/STT backend rewrite.
- No broad workspace formatting.
- No test deletion to reduce line count.

## Lane 1: Input Key Policy

Commit prefix: `[gui] NFC`

Candidate module:

```text
gameterm-gui/src/overlay/visual_input_keys.rs
```

Move:

- `visual_input_from_key`
- `visual_input_resets_dialogue_scroll`
- `is_tts_toggle_key`
- `is_stt_hold_key`
- `is_stt_hold_release_key`

Definition of done:

- key mapping is unchanged
- TTS mute key behavior is unchanged
- STT hold/release behavior is unchanged
- dialogue scroll reset behavior is unchanged

Checks:

```sh
cargo test -p gameterm-gui scene_tts_toggle --bin gameterm-gui
cargo test -p gameterm-gui scene_stt_hold --bin gameterm-gui
cargo test -p gameterm-gui scene_dialogue_scrollback --bin gameterm-gui
```

## Lane 2: Frame Utilities

Commit prefix: `[gui] NFC`

Candidate module:

```text
gameterm-gui/src/overlay/visual_frame.rs
```

Move:

- `replace_last_screen_line`
- `replace_screen_line`
- `clip_text`
- possibly `apply_voice_debug_frame`, if coupling stays clean

Definition of done:

- frame height behavior is unchanged
- clipping behavior is unchanged
- voice debug frame remains bounded to top lines

Checks:

```sh
cargo test -p gameterm-gui replace_last_screen_line --bin gameterm-gui
cargo test -p gameterm-gui scene_voice_debug_frame --bin gameterm-gui
```

## Lane 3: Render Orchestration

Commit prefix: `[gui] NFC`

Candidate module:

```text
gameterm-gui/src/overlay/visual_render.rs
```

Move:

- `render_runtime`
- `render_runtime_with_compose`
- `render_runtime_with_compose_and_scroll`
- `render_error`

Keep explicit dependencies:

- `SceneRuntime`
- `SceneComposeDock`
- `SceneDialogueScrollback`
- `VisualSpriteManifestStatus`
- Kiki idle helper calls
- frame helper calls
- `TermWizTerminal`

Definition of done:

- visual metadata still attaches to the terminal
- Kiki idle animation still applies before metadata write
- VN compose dock still renders unchanged
- voice debug overlay still renders unchanged
- failed scene loads still render error text

Checks:

```sh
cargo test -p gameterm-gui kiki --bin gameterm-gui
cargo test -p gameterm-gui scene_voice_debug_frame --bin gameterm-gui
cargo test -p gameterm-gui visual_quad --bin gameterm-gui
cargo test -p gameterm-gui overlay::visual --bin gameterm-gui
```

## Lane 4: Voice Event Handling

Commit prefix: `[gui] NFC`

Candidate module:

```text
gameterm-gui/src/overlay/visual_voice_events.rs
```

Move:

- `apply_tts_result`
- `apply_stt_result`
- STT cancel/start/stop helpers only if control flow stays readable

Do not move:

- `SceneTtsWorker`
- `SceneTtsState`
- `SceneSttState`
- backend implementations in `visual_tts.rs` or `visual_stt.rs`

Definition of done:

- fake Codex and voice debug test mode still work
- STT hold-to-talk state transitions are unchanged
- TTS mute/result status behavior is unchanged

Checks:

```sh
cargo test -p gameterm-gui scene_voice_debug --bin gameterm-gui
cargo test -p gameterm-gui visual_tts --bin gameterm-gui
cargo test -p gameterm-gui visual_stt --bin gameterm-gui
cargo test -p gameterm-gui overlay::visual --bin gameterm-gui
```

## Lane 5: Channel Drain Helpers

Commit prefix: `[gui] NFC`

Candidate module:

```text
gameterm-gui/src/overlay/visual_event_drain.rs
```

Move:

- command result drain
- compose result drain
- TTS result drain
- STT result drain

Likely support type:

```text
VisualOverlaySession
```

Constraint:

Each drain helper should return whether rendering is needed. It should not call
rendering directly unless that is demonstrably simpler and still preserves the
event-loop structure.

Definition of done:

- first voice reveal behavior is unchanged
- STT test mode behavior is unchanged
- command spawn/failure status behavior is unchanged
- compose/fake-Codex result behavior is unchanged

Checks:

```sh
cargo test -p gameterm-gui compose_backend --bin gameterm-gui
cargo test -p gameterm-gui fake_codex --bin gameterm-gui
cargo test -p gameterm-gui scene_voice_debug --bin gameterm-gui
cargo test -p gameterm-gui dispatch_ --bin gameterm-gui
cargo test -p gameterm-gui overlay::visual --bin gameterm-gui
```

## Lane 6: Overlay Session State

Commit prefix: `[gui] NFC`

Candidate module:

```text
gameterm-gui/src/overlay/visual_overlay_session.rs
```

Purpose:

Group the large set of local variables in `show_visual_scene_overlay_with_source`
only after lanes 1-5 make the state shape clear.

Candidate groups:

- runtime/load state
- scene path and generated scene source
- sprite manifest state
- file watcher and patch inbox
- compose dock/backend state
- dialogue scroll state
- TTS/STT state
- idle animation cache

Constraint:

Do not introduce this struct if it hides coupling or makes the event loop harder
to read.

Checks:

```sh
cargo test -p gameterm-gui overlay::visual --bin gameterm-gui
cargo check -p gameterm-gui
```

## Lane 7: Test Locality

Commit prefix: `[test] NFC`

Current state:

`visual.rs` still owns a large integration-style test module. Some tests now
cover helpers moved into sibling modules.

Rules:

- keep integration-style overlay tests in `visual.rs`
- move pure helper tests into helper modules only when locality improves
- do not move tests only to reduce line count

Checks:

```sh
cargo test -p gameterm-gui overlay::visual --bin gameterm-gui
cargo test -p gameterm-gui visual_ --bin gameterm-gui
```

## Recommended Order

1. `[gui] NFC - move Scene input key helpers`
2. `[gui] NFC - move Scene frame helpers`
3. `[gui] NFC - move Scene render helpers`
4. `[gui] NFC - move Scene voice event helpers`
5. `[gui] NFC - move Scene event drain helpers`
6. `[gui] NFC - introduce Scene overlay session state`
7. `[test] NFC - localize Scene overlay helper tests`, only if useful

## Full Definition Of Done

This refactor pass is complete when:

- `visual.rs` is primarily entrypoints plus event-loop orchestration
- pure key, frame, render, voice, and channel-drain helpers live in focused
  modules
- no Scene Mode behavior, config, keybinding, voice, Codex, or render behavior
  changes
- every lane lands as a separate NFC commit
- focused tests pass for each lane
- broad checks pass:

```sh
cargo test -p gameterm-visual
cargo test -p gameterm-gui overlay::visual --bin gameterm-gui
cargo test -p gameterm-gui visual_quad --bin gameterm-gui
cargo check -p gameterm-gui
git diff --check
```

Optional live smoke if render/input event paths move materially:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario vertical-slice \
  --output /tmp/gameterm-scene-visual-overlay-refactor.png
```

## Stop Conditions

Pause and reassess if:

- a helper move requires behavior changes
- a helper move creates broad API churn outside Scene overlay code
- a session object makes dependencies less visible
- tests require assertion changes for an NFC lane
- the change starts touching non-Scene upstream behavior

## Current Recommendation

Do lanes 1-4 first. They have the best value-to-risk ratio.

Only do lanes 5-6 after the smaller extractions prove the event-loop state
shape.
