# GameTerm Scene Maintainability Refactor Scope

Status: first implementation pass complete.

Date: 2026-06-07

Implementation pass:

- `a3f9d2981 [docs] scope Scene maintainability refactor`
- `709729bad [gui] NFC - split Scene TTS speech blocks`
- `26cfacf5d [gui] NFC - split Scene compose backend`
- `67b72c4e9 [gui] NFC - split Scene STT backend`
- `96e1cbae7 [gui] NFC - split Scene overlay input helpers`
- `b7ffb250d [gui] NFC - split Scene stage image helpers`
- `017ddf309 [gui] NFC - move SceneRuntime test import`

The implementation intentionally used smaller facade-first extractions instead
of every ideal target module in this scope. That kept the pass behavior-neutral
and avoided public API churn.

This document scopes the next maintainability refactor pass for Scene Mode
after the first product/dogfood pass and the first extraction pass.

It is based on:

- `structure/Development/coding-principles.md`
- `structure/Development/loc-complexity-refactor-analysis.md`
- `structure/Development/scene-quality-refactor-scope.md`
- `docs/gameterm-scene-refactor-plan.md`
- current source measurements from 2026-06-07

## Goal

Make Scene Mode easier to change without changing product behavior.

This pass should improve:

- troubleshooting clarity
- module ownership
- compose/TTS/STT reliability boundaries
- debug menu boundaries
- render primitive locality
- shell helper maintainability
- test locality

It should not redesign Scene Mode, change config, change keybindings, change
visual layout, change voice behavior, or remove mux.

## Coding Principles Applied

This pass follows the repo coding principles:

- preserve upstream behavior unless a task explicitly defines a fork-specific
  change
- keep GameTerm-specific changes narrow and reviewable
- prefer existing crate/module boundaries before adding new crates
- do not mix NFC refactors with behavior changes
- keep commits small and subsystem-prefixed
- use focused checks first, then broaden when touching shared behavior
- keep `structure/` and docs factual, compact, and free of transient logs

Commit prefixes for this pass:

- `[visual] NFC` for behavior-preserving Scene runtime/module moves
- `[gui] NFC` for behavior-preserving GUI overlay/module moves
- `[render] NFC` for behavior-preserving GPU/render primitive moves
- `[tools] NFC` for shell helper reorganization
- `[test]` for verification-only additions or test harness changes
- `[docs]` for scope/status updates

Functional fixes found during refactor must stop the NFC lane and land as a
separate behavior commit before continuing.

## Current Repo State

Scene Mode is no longer one unstructured file. The first refactor pass already
extracted meaningful modules:

```text
gameterm-visual/src/
  actions.rs
  compose_state.rs
  conditions.rs
  debug.rs
  patch.rs
  render.rs
  runtime_input.rs
  runtime_selection.rs
  runtime_status.rs
  runtime_status_methods.rs
  schema.rs
  story_state.rs
  validation.rs
  vn_asset_intake.rs
  vn_layout.rs
  vn_text.rs
  workspace_scene.rs

gameterm-gui/src/overlay/
  visual_command_dispatch.rs
  visual_compose_dock.rs
  visual_compose_result.rs
  visual_dialogue_scroll.rs
  visual_event_drain.rs
  visual_frame.rs
  visual_input_keys.rs
  visual_kiki_idle.rs
  visual_overlay_session.rs
  visual_render.rs
  visual_scene_files.rs
  visual_scene_patches.rs
  visual_voice_debug.rs
  visual_voice_events.rs
```

The current refactor should therefore focus on second-order coupling:

- oversized modules that now contain several sub-concerns
- files whose tests are protecting several behaviors at once
- modules where async/worker/event semantics are hard to debug
- shell helpers that still carry long conditional paths

## Current Hotspots

Measured line counts for Scene-owned files before this pass:

```text
7612 gameterm-visual/src/lib.rs
2119 gameterm-gui/src/overlay/visual.rs
1904 ci/gameterm-scene-author.sh
1719 ci/gameterm-scene-verify.sh
1702 gameterm-gui/src/overlay/visual_tts.rs
1441 ci/gameterm-scene-smoke.sh
1189 gameterm-gui/src/termwindow/render/visual_quad.rs
1131 ci/gameterm-scene-workspace.sh
1104 gameterm-gui/src/overlay/visual_compose.rs
1044 gameterm-gui/src/overlay/visual_stt.rs
 764 gameterm-visual/src/vn_asset_intake.rs
 751 gameterm-visual/src/vn_layout.rs
 699 gameterm-visual/src/actions.rs
 612 ci/gameterm-scene-mux-context.sh
 586 ci/gameterm-scene-vn-demo.sh
 544 gameterm-visual/src/workspace_scene.rs
 521 gameterm-visual/src/vn_text.rs
 503 gameterm-gui/src/overlay/visual_scene_files.rs
 473 gameterm-gui/src/termwindow/render/visual_vn_panel.rs
```

Raw LOC is not the only signal. Some of the line count is tests, schema, or
fixtures. The refactor should prioritize coupling and operational risk over
size alone.

Post-pass line counts for touched hotspots:

```text
1983 gameterm-gui/src/overlay/visual.rs
1156 gameterm-gui/src/overlay/visual_tts.rs
1061 gameterm-gui/src/termwindow/render/visual_quad.rs
1104 gameterm-gui/src/overlay/visual_compose_backend.rs
1044 gameterm-gui/src/overlay/visual_stt_backend.rs
 554 gameterm-gui/src/overlay/visual_speech_blocks.rs
 138 gameterm-gui/src/termwindow/render/visual_stage_image.rs
  97 gameterm-gui/src/overlay/visual_scene_debug_input.rs
  58 gameterm-gui/src/overlay/visual_voice_hold_flow.rs
  10 gameterm-gui/src/overlay/visual_compose.rs
   6 gameterm-gui/src/overlay/visual_stt.rs
```

Remaining large modules are now more explicit:

- `visual_compose_backend.rs` still owns the full compose backend internals.
- `visual_stt_backend.rs` still owns the full STT backend internals.
- `visual_tts.rs` still owns TTS worker/backend execution, while speech parsing
  lives in `visual_speech_blocks.rs`.
- `visual_quad.rs` still owns GPU quad allocation, while stage image placement
  lives in `visual_stage_image.rs` and panel primitives live in
  `visual_vn_panel.rs`.

## What Not To Refactor Yet

Do not remove mux.

Reasons:

- Scene overlays are tied to mux panes/windows.
- Patch targeting needs mux window/pane identity.
- Active pane and workspace context flow through mux.
- Removing mux would be a broad product architecture rewrite, not cleanup.

Do not move Scene Mode into a new crate yet.

Reasons:

- `gameterm-visual` and `gameterm-gui` already give a usable split between
  runtime/schema and GUI/platform behavior.
- A new crate would create public API churn before module ownership is stable.

Do not delete tests to reduce LOC.

Reasons:

- Scene Mode has grown through dogfood-driven fixes.
- Tests are the main protection against visual/TTS/compose regressions.

Do not refactor generated/table data, upstream terminal code, or macOS warning
noise in this pass.

Reasons:

- They are not the bottleneck for Scene Mode dogfood.
- They risk upstream divergence without improving the active product surface.

## Refactor Lanes

### Lane 0: Baseline And Guardrails

Owner prefix: `[docs]`, `[test]`

Purpose: make the current refactor measurable before moving code.

Actions:

- record current hotspot measurements
- list current verification commands
- keep the current TTS/compose future-turn regression tests
- add focused negative tests only where a lane exposes an actual blind spot

Verification:

```sh
git diff --check
cargo test -p gameterm-visual
cargo test -p gameterm-gui visual_tts_worker -- --nocapture
cargo check -p gameterm-gui
```

Definition of done:

- scope is current
- no dirty behavior changes are mixed into the refactor plan
- first implementation lane has a clear target and focused checks

### Lane 1: Scene Runtime Schema Boundary

Owner prefix: `[visual] NFC`

Purpose: reduce `gameterm-visual/src/lib.rs` without forcing a broad schema
rewrite.

Current issue:

`lib.rs` still owns large public DTO blocks, runtime construction, render frame
assembly, and tests. Some extraction has already happened, but the remaining
schema/runtime boundary is still hard to scan.

Move candidates:

- stage/displayable DTOs to `schema.rs` if the move is mechanical
- entity/dialogue DTOs to `schema.rs` only if public exports stay simple
- render snapshot DTOs to `render.rs` or a narrow `render_snapshot.rs`
- remaining low-level render-frame assembly helpers to `vn_text.rs` only if
  they do not expose `SceneRuntime` internals

Do not move:

- `SceneRuntime` as a broad rewrite
- core public exports if the diff becomes mostly import churn
- tests unless locality improves

Verification:

```sh
cargo test -p gameterm-visual scene_rejects
cargo test -p gameterm-visual staged_scene
cargo test -p gameterm-visual vn_overlay
cargo test -p gameterm-visual
```

Definition of done:

- moved code is mechanically equivalent
- public `gameterm_visual` imports remain compatible
- fixture JSON does not change
- no behavior change is included

Pause criteria:

- serde defaults become harder to audit
- public export churn dominates the diff
- runtime methods need to become public only to satisfy the move

### Lane 2: Compose/TTS Speech Pipeline Boundary

Implementation status: partially complete in `709729bad`.

Completed:

- extracted `visual_speech_blocks.rs`
- moved `SpeakableSegment`, `SpeechBlockKind`, `SpeakableSource`, extraction,
  technical cleanup, and chunk splitting into that module
- kept `visual_tts.rs` as the worker/backend implementation

Deferred:

- separate `visual_tts_worker.rs`
- separate `visual_tts_backends.rs`

Reason: the first split removed the highest-risk parsing/backend coupling
without changing TTS behavior or import paths broadly.

Owner prefix: `[gui] NFC`, `[visual] NFC` only if runtime state moves

Purpose: make speech-block ordering, text visibility, TTS playback, and
compose result application easy to debug.

Current issue:

Recent dogfood found that TTS timing could hide text. The immediate fix landed,
but the speech path still crosses multiple modules:

```text
visual_compose.rs
visual_compose_result.rs
visual_tts.rs
visual_event_drain.rs
visual_voice_events.rs
gameterm-visual/src/compose_state.rs
gameterm-visual/src/vn_text.rs
```

Refactor target:

```text
visual_speech_blocks.rs
  SpeakableSegment
  SpeechBlockKind
  SpeakableSource
  extraction and cleaning helpers
  split_speakable_chunks

visual_tts_worker.rs
  SceneTtsRequest
  SceneTtsResult
  SceneTtsEvent
  SceneTtsWorker

visual_tts_backends.rs
  command backend
  voicevox backend
  translation command
  player command
```

Keep `visual_tts.rs` as a thin facade if that avoids changing import paths
across the overlay.

Do not change:

- TTS backend config names
- output cleanup behavior
- worker serialization behavior
- text visibility semantics

Verification:

```sh
cargo test -p gameterm-gui visual_tts_extracts --bin gameterm-gui
cargo test -p gameterm-gui visual_tts_worker --bin gameterm-gui -- --nocapture
cargo test -p gameterm-gui visual_tts_voicevox --bin gameterm-gui
cargo test -p gameterm-visual compose_blocks
cargo check -p gameterm-gui
```

Definition of done:

- speech-block extraction can be tested without backend execution
- worker sequencing can be tested without text parsing
- backend/player behavior can be tested without compose result application
- future-turn text visibility tests remain green

### Lane 3: STT/Microphone Boundary

Implementation status: facade-first split complete in `67b72c4e9`.

Completed:

- added `visual_stt_backend.rs`
- kept `visual_stt.rs` as a thin facade/re-export module

Deferred:

- separate config, mic-device, mic-test, Whisper, and command modules

Reason: the current split gives `visual.rs` a stable STT import surface while
preserving behavior. The backend internals can be split later if STT becomes
the active debugging bottleneck.

Owner prefix: `[gui] NFC`

Purpose: split voice input configuration, mic capture/test, Whisper execution,
and command backend execution.

Current issue:

`visual_stt.rs` owns config, state, mic listing, mic test, local recording,
Whisper cache, Whisper transcription, command backend, and tests.

Refactor target:

```text
visual_stt.rs             facade/types
visual_stt_config.rs      env/config parsing
visual_mic_devices.rs     device enumeration and selection
visual_mic_test.rs        mic test capture/result
visual_whisper.rs         Whisper model/cache/transcription
visual_stt_command.rs     command backend
```

Do not change:

- push-to-talk key behavior
- selected mic config semantics
- Whisper model path env var
- auto-submit behavior

Verification:

```sh
cargo test -p gameterm-gui visual_stt_config --bin gameterm-gui
cargo test -p gameterm-gui visual_stt_whisper --bin gameterm-gui
cargo test -p gameterm-gui scene_voice_debug --bin gameterm-gui
cargo check -p gameterm-gui
```

Definition of done:

- mic selection can be tested separately from Whisper
- command backend can be tested separately from recording
- debug menu tests still cover voice diagnostics

### Lane 4: Compose Backend Boundary

Implementation status: facade-first split complete in `26cfacf5d`.

Completed:

- added `visual_compose_backend.rs`
- kept `visual_compose.rs` as a thin facade/re-export module

Deferred:

- separate config, Codex argv, process, and failure-classification modules

Reason: this keeps existing tests and imports stable while making the compose
backend separable from overlay orchestration.

Owner prefix: `[gui] NFC`

Purpose: separate compose backend config, Codex command construction, process
execution, and failure classification.

Current issue:

`visual_compose.rs` owns:

- env/file config loading
- backend kind selection
- Codex config validation
- command argv parsing
- process timeout handling
- Codex command construction
- failure classification
- fake process tests

Refactor target:

```text
visual_compose.rs             facade/types
visual_compose_config.rs      env/file config
visual_compose_codex.rs       codex argv/config
visual_compose_process.rs     command spawn/wait/output capture
visual_compose_failure.rs     failure classification/status/dialogue
```

Do not change:

- persistent app-launch config path
- env var names
- Codex sandbox/approval defaults
- timeout defaults
- lazy Codex-only validation semantics

Verification:

```sh
cargo test -p gameterm-gui compose_backend_config --bin gameterm-gui
cargo test -p gameterm-gui codex_compose_argv --bin gameterm-gui
cargo test -p gameterm-gui run_configured_compose_backend --bin gameterm-gui
cargo test -p gameterm-gui compose_failure --bin gameterm-gui
cargo check -p gameterm-gui
```

Definition of done:

- config tests do not need process helpers
- process tests do not need Codex config parsing
- failure classification is isolated and easier to extend

### Lane 5: Overlay Event Loop Orchestration

Implementation status: first helper extraction complete in `96e1cbae7`.

Completed:

- moved centralized Scene debug-menu side effects to
  `visual_scene_debug_input.rs`
- moved voice hold reconciliation to `visual_voice_hold_flow.rs`
- kept `visual.rs` as the event-loop owner

Deferred:

- full key/mouse dispatch module split
- moving the broad `visual.rs` test module

Reason: the extracted helpers remove feature-specific branches without changing
input behavior or render cadence.

Owner prefix: `[gui] NFC`

Purpose: keep `visual.rs` as orchestration only.

Current issue:

`visual.rs` is much smaller than before, but it still owns:

- main event loop
- key/mouse branch coordination
- STT hold transitions
- fake-Codex immediate path
- reload/close handling
- debug view coordination
- test module with many cross-feature assertions

Refactor target:

```text
visual_overlay_loop.rs       event loop helpers
visual_key_dispatch.rs       key branch coordination
visual_mouse_dispatch.rs     mouse branch coordination
visual_voice_hold_input.rs   command-key hold transitions
visual_debug_dispatch.rs     debug view routing
```

This lane should happen after lanes 2-4 so the event loop has smaller helpers
to call.

Do not change:

- keybindings
- mouse scroll behavior
- reload/close semantics
- fake-Codex debug behavior
- render cadence

Verification:

```sh
cargo test -p gameterm-gui overlay::visual --bin gameterm-gui
cargo test -p gameterm-gui scene_voice_debug --bin gameterm-gui
cargo test -p gameterm-gui fake_codex --bin gameterm-gui
cargo check -p gameterm-gui
```

Definition of done:

- `visual.rs` reads primarily as high-level event orchestration
- feature-specific behavior lives in named modules
- no behavior changes are included

### Lane 6: VN GPU Render Primitive Boundary

Implementation status: first render primitive split complete in `b7ffb250d`.

Completed:

- added `visual_stage_image.rs`
- moved image scale mode, aspect-fit/cover math, stage slot placement,
  placeholder fit, and sprite lookup into that module
- kept VN panel primitives in the existing `visual_vn_panel.rs`

Deferred:

- separate overlay quad population module
- separate debug suppression module

Reason: panel primitives were already extracted; the highest-value remaining
mechanical split was stage image math out of `visual_quad.rs`.

Owner prefix: `[render] NFC`

Purpose: separate stage image placement, VN panel drawing, scrollbars, voice
indicator, and debugger suppression rules.

Current issue:

`visual_quad.rs` still mixes:

- stage displayable placement
- image aspect-fit/cover policy
- VN panel population
- nameplates
- scrollbars
- voice indicator
- debug-view suppression
- placeholder colors

Refactor target:

```text
visual_quad.rs                 facade/entrypoints
visual_stage_quads.rs          background/character/entity stage quads
visual_vn_overlay_quads.rs     panels/nameplates/scrollbar/voice indicator
visual_image_scaling.rs        stretch/fill/fit/integer-fit rect math
visual_debug_suppression.rs    view/menu suppression rules
```

Do not change:

- pixel output
- panel opacity/radius constants
- aspect-safe image placement
- debug suppression behavior
- scrollbar behavior

Verification:

```sh
cargo test -p gameterm-gui visual_quad --bin gameterm-gui
cargo test -p gameterm-gui visual_vn_panel --bin gameterm-gui
cargo check -p gameterm-gui
ci/gameterm-scene-smoke.sh --launch --scenario vn-compose \
  --output /tmp/gameterm-scene-render-refactor.png
```

Definition of done:

- render math is testable without scanning panel draw code
- panel primitives remain in one named module
- smoke screenshot still shows background, Kiki, dialogue, composer, and
  nameplates

### Lane 7: Scene CI Helper Table-Driven Cleanup

Owner prefix: `[tools] NFC`, `[test]`

Purpose: reduce long shell helper branching without changing helper behavior.

Current issue:

The largest Scene shell helpers are:

```text
1904 ci/gameterm-scene-author.sh
1719 ci/gameterm-scene-verify.sh
1441 ci/gameterm-scene-smoke.sh
1131 ci/gameterm-scene-workspace.sh
```

Refactor target:

- table-drive smoke scenario metadata
- isolate common path/config helpers
- isolate screenshot/output path helpers
- isolate scenario command construction
- keep generated fixture behavior unchanged

Do not change:

- command names
- output file naming
- fixture paths
- smoke scenario semantics
- user-facing helper docs

Verification:

```sh
bash -n ci/gameterm-scene-author.sh
bash -n ci/gameterm-scene-verify.sh
bash -n ci/gameterm-scene-smoke.sh
bash -n ci/gameterm-scene-workspace.sh
ci/gameterm-scene-verify.sh --all
ci/gameterm-scene-smoke.sh --list
ci/gameterm-scene-smoke.sh --describe vn-compose
```

Definition of done:

- branching is easier to audit
- scenario metadata can be inspected without reading the full script
- helper behavior is unchanged

### Lane 8: Test Locality And Naming

Owner prefix: `[test]`

Purpose: make failures easier to diagnose.

Current issue:

Many tests are already valuable, but some test names are narrower than the
invariant they protect, and some modules still hold broad cross-feature test
blocks.

Actions:

- rename tests to describe invariants, not incidental turn numbers
- keep future-turn/failure-mode language where the bug is general
- move tests only when the module move improves locality
- add negative tests for missing/stale event cases when worker/event code moves

Examples of target wording:

```text
later_turns_render_while_previous_voice_blocks_are_unfinished
stale_voice_events_do_not_hide_text
disabled_capability_does_not_enter_prompt_manifest
unknown_app_tile_proposal_requires_policy_rejection
```

Verification:

```sh
cargo test -p gameterm-visual
cargo test -p gameterm-gui overlay::visual --bin gameterm-gui
```

Definition of done:

- test failures describe the product invariant
- no tests are deleted for LOC reduction
- broad tests remain only where cross-module behavior is intentional

### Lane 9: Capability Routing Substrate

Owner prefix: `[visual]`, `[gui]`, `[docs]`

Purpose: implement the already-scoped Arkey-style capability routing substrate
only after the core compose/voice boundaries are clearer.

This is not NFC. It is included here as a sequencing note because it will touch
compose prompt construction, debug diagnostics, and action policy.

Scope doc:

- [Arkey Capability Routing Scope](gameterm-scene-arkey-capability-routing-scope.md)

Recommended prerequisite lanes:

- Lane 2: Compose/TTS Speech Pipeline Boundary
- Lane 4: Compose Backend Boundary
- Lane 8: Test Locality And Naming

Reason:

Capability routing adds new prompt/action state. It will be easier to implement
safely after the existing compose pipeline is less coupled.

## Recommended Execution Order

1. Lane 0: Baseline And Guardrails
2. Lane 2: Compose/TTS Speech Pipeline Boundary
3. Lane 4: Compose Backend Boundary
4. Lane 3: STT/Microphone Boundary
5. Lane 5: Overlay Event Loop Orchestration
6. Lane 6: VN GPU Render Primitive Boundary
7. Lane 1: Scene Runtime Schema Boundary
8. Lane 7: Scene CI Helper Table-Driven Cleanup
9. Lane 8: Test Locality And Naming, as part of each lane and as a final pass
10. Lane 9: Capability Routing Substrate, as a product lane after refactor

Rationale:

- The active dogfood pain is compose/voice timing, so compose/TTS comes first.
- Compose backend config is the next highest operational surface.
- STT is adjacent but can follow after compose/TTS conventions settle.
- The event loop gets easier to split after its subfeatures are smaller.
- Render and schema refactors are valuable but less urgent than voice/compose
  dogfood reliability.
- Shell helper cleanup can wait unless smoke churn becomes the blocker.

## Verification Matrix

Minimum checks by lane:

| Lane | Focused checks | Broad check |
| --- | --- | --- |
| Runtime schema | `cargo test -p gameterm-visual staged_scene vn_overlay scene_rejects` | `cargo test -p gameterm-visual` |
| Compose/TTS | `cargo test -p gameterm-gui visual_tts_worker -- --nocapture` | `cargo check -p gameterm-gui` |
| STT/mic | `cargo test -p gameterm-gui visual_stt_whisper --bin gameterm-gui` | `cargo check -p gameterm-gui` |
| Compose backend | `cargo test -p gameterm-gui compose_backend_config --bin gameterm-gui` | `cargo check -p gameterm-gui` |
| Overlay loop | `cargo test -p gameterm-gui overlay::visual --bin gameterm-gui` | `cargo check -p gameterm-gui` |
| Render | `cargo test -p gameterm-gui visual_quad --bin gameterm-gui` | `cargo check -p gameterm-gui` + smoke |
| CI helpers | `bash -n ci/gameterm-scene-*.sh` | `ci/gameterm-scene-verify.sh --all` |

Run live smoke when a lane touches:

- GUI input
- overlay event loop
- render quads
- patch transport
- app launch/install behavior
- smoke script behavior

Default smoke:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario vn-compose \
  --output /tmp/gameterm-scene-refactor-vn-compose.png
```

## Definition Of Done

This refactor pass is complete when:

- compose/TTS/STT responsibilities are split enough to debug worker, backend,
  and text visibility failures independently
- `visual.rs` remains orchestration rather than feature implementation
- render primitives are grouped by stage images, VN panels, and scaling math
- CI helper branching is documented or table-driven enough for smoke scenarios
  to be maintained without rereading entire scripts
- tests protect current dogfood regressions with invariant-oriented names
- no public Scene JSON schema changes occurred unless separately scoped
- no upstream-wide code was reorganized
- no behavior fixes were hidden inside NFC commits

First-pass status:

- Compose/TTS, compose backend, STT, overlay helper, and render primitive
  boundaries are complete enough for current dogfood troubleshooting.
- Runtime schema, CI helper table cleanup, and deeper backend sub-splits remain
  explicit follow-ups.
- No Scene JSON schema changes were made in this NFC pass.
- No behavior fixes were included in the five NFC implementation commits.

Verification completed for this pass:

```sh
cargo test -p gameterm-gui visual_speech_blocks
cargo test -p gameterm-gui visual_tts_
cargo test -p gameterm-gui compose_backend
cargo test -p gameterm-gui codex_
cargo test -p gameterm-gui visual_stt_
cargo test -p gameterm-gui scene_debug_menu
cargo test -p gameterm-gui scene_voice_debug
cargo test -p gameterm-gui scene_voice
cargo test -p gameterm-gui visual_quad
cargo test -p gameterm-gui vn_panel
cargo test -p gameterm-gui stage_displayable
cargo test -p gameterm-visual
cargo test -p gameterm-gui --bin gameterm-gui
cargo check -p gameterm-gui
git diff --check
```

Results:

- `gameterm-visual`: 186 passed, 0 failed.
- `gameterm-gui --bin gameterm-gui`: 155 passed, 0 failed.
- `cargo check -p gameterm-gui`: passed with only known existing warnings.
- Focused GUI filters listed above passed.

## Stop Conditions

Pause and rescope if:

- a move requires changing user-visible behavior
- a module extraction creates wider public APIs than the code it removes
- tests need large rewrites just to follow moved private helpers
- the diff becomes mostly import churn
- a dogfood defect appears during refactor
- a lane crosses from GameTerm-specific Scene code into upstream terminal/mux
  architecture

## Immediate Next Step

The first pass is complete. The next refactor should only continue if it is
serving an active dogfood problem.

Recommended follow-ups:

1. Product lane: persistent Codex sessions / `codex exec resume`.
2. Refactor lane: split `visual_compose_backend.rs` into config, Codex argv,
   process, and failure-classification modules if compose diagnostics become
   hard to extend.
3. Refactor lane: split `visual_stt_backend.rs` into config, mic device/test,
   Whisper, and command modules if STT debugging becomes active again.
4. Tooling lane: table-drive Scene shell helper scenario metadata if smoke
   script maintenance becomes the bottleneck.

Do not continue refactoring only to reduce LOC. The next pass should remove
real coupling that is blocking product work or troubleshooting.
